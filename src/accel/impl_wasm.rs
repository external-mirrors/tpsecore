use std::ptr::null;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::LoadError;
use crate::wasm::STATE as TPSE_STATE;

#[derive(Default)]
struct WasmAcceleratorState {
  id_counter: u64,
  command_buffer: Vec<u8>
}
impl WasmAcceleratorState {
  pub fn new_handle(&mut self, init_dimension: Option<(u32, u32)>) -> WasmTextureHandle {
    let dimensions = OnceLock::new();
    if let Some(init_dimension) = init_dimension {
      dimensions.set(init_dimension);
    }
    let handle = WasmTextureHandle(Arc::new(WasmTextureHandleInner {
      id: self.id_counter,
      dimensions
    }));
    self.id_counter += 1;
    handle
  }
}

macro_rules! encode {
  (auto, $($rest:tt)+) => {{
    let mut state = STATE.lock().unwrap();
    encode!(state, $($rest)+)
  }};
  ($state:expr, $handle:expr, $command_id:expr, $(new($dimensions:expr),)? [$($value:expr),*]) => {{
    // generate new handle, if requested
    $( let new_handle = $state.new_handle($dimensions); )?
    // push command ID
    $state.command_buffer.push($command_id);
    // push handle ID (for new_texture or decode_texture this is the ID to be allocated)
    $state.command_buffer.extend($handle.0.id.to_be_bytes());
    // if a new handle was requested, push that ID. Non-use of $dimensions to ensure group repeats correctly.
    $( $state.command_buffer.extend(new_handle.0.id.to_be_bytes()); let _ = $dimensions; )?
    // push arguments
    $( $state.command_buffer.extend($value.to_be_bytes()); )*
    // return handle, if requested
    $( let _ = $dimensions; new_handle )?
  }};
}

static STATE: LazyLock<Mutex<WasmAcceleratorState>> = LazyLock::new(Default::default);

pub struct WasmAccelerator;
impl TPSEAccelerator for WasmAccelerator {
  type Texture = WasmTextureHandle;
  type DecodeError = LoadError;

  fn new_texture(width: u32, height: u32) -> Self::Texture {
    let mut state = STATE.lock().unwrap();
    let handle = state.new_handle(Some((width, height)));
    encode!(state, handle, 0, [width, height]);
    handle
  }

  fn decode_texture(buffer: &[u8]) -> Result<Self::Texture, Self::DecodeError> {
    let mut state = STATE.lock().unwrap();
    let handle = state.new_handle(None);
    encode!(state, handle, 1, [buffer.len() as u64]);
    state.command_buffer.extend_from_slice(buffer);
    Ok(handle)
  }
}

#[link(wasm_import_module="wasm_accelerator")]
unsafe extern "C" {
  /// Returns dimensions for the given handle, as two u32 (width, height) packed into a u64
  /// Flushes the command buffer first, if it's not empty
  unsafe fn fetch_dimensions(command_buffer: *const u8, cmdbuflen: usize, id: u64) -> u64;
  /// Encodes the given handle into a buffer managed by the TPSE buffer infastructure
  /// Flushes the command buffer first, if it's not empty
  unsafe fn encode_png(command_buffer: *const u8, cmdbuflen: usize, id: u64) -> *const u8;
  /// Drops a handle by ID
  unsafe fn drop_handle(id: u64);
}

struct WasmTextureHandleInner {
  id: u64,
  dimensions: OnceLock<(u32, u32)>
}
impl Drop for WasmTextureHandleInner {
  fn drop(&mut self) {
    // locking state shouldn't deadlock so long as we don't go dropping handles inside any of the impl
    // WasmTextureHandle methods, but in this case freeing memory is more important than lazy evaluation
    unsafe { drop_handle(self.id); }
  }
}

#[derive(Clone)]
pub struct WasmTextureHandle(Arc<WasmTextureHandleInner>);
impl WasmTextureHandle {
  fn dimensions(&self) -> (u32, u32) {
    *self.0.dimensions.get_or_init(|| {
      let mut state = STATE.lock().unwrap();
      let bytes = unsafe { fetch_dimensions(state.command_buffer.as_ptr(), state.command_buffer.len(), self.0.id) };
      state.command_buffer.clear();
      state.command_buffer.shrink_to(8*1024*1024); // 8MiB
      let [a, b, c, d, e, f, g, h] = bytes.to_be_bytes();
      let width = u32::from_be_bytes([a, b, c, d]);
      let height = u32::from_be_bytes([e, f, g, h]);
      (width, height)
    })
  }
}
impl TextureHandle for WasmTextureHandle {
  fn width(&self) -> u32 {
    self.dimensions().0
  }

  fn height(&self) -> u32 {
    self.dimensions().1
  }

  fn encode_png(&self) -> Result<Arc<[u8]>, ()> {
    let mut state = STATE.lock().unwrap();
    let ptr = unsafe { encode_png(state.command_buffer.as_ptr(), state.command_buffer.len(), self.0.id) };
    state.command_buffer.clear();
    state.command_buffer.shrink_to(8*1024*1024); // 8MiB
    drop(state);
    
    // todo: error message routing
    // because all operations are deferred, this is the one place where we actually get back errors
    // and so having a good message here is quite important
    if ptr == null() {
      return Err(())
    }
    
    let mut state = TPSE_STATE.lock().unwrap();
    let buffer = state.lookup_buffer(ptr as *mut u8).unwrap();
    Ok(state.buffers.remove(&buffer).unwrap())
  }

  fn create_copy(&self) -> Self {
    encode!(auto, self, 2, new(self.0.dimensions.get().copied()), [])
  }

  fn slice(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
    encode!(auto, self, 3, new(self.0.dimensions.get().copied()), [])
  }

  fn resized(&self, width: u32, height: u32) -> Self {
    encode!(auto, self, 4, new(Some((width, height))), [width, height])
  }

  fn tinted(&self, [r, g, b, a]: [u8; 4]) -> Self {
    encode!(auto, self, 5, new(self.0.dimensions.get().copied()), [r, g, b, a])
  }

  fn overlay(&self, with_image: &Self, x: i64, y: i64) {
    encode!(auto, self, 6, [with_image.0.id, x, y])
  }

  fn draw_line(&self, start: (f32, f32), end: (f32, f32), [r, g, b, a]: [u8; 4]) {
    encode!(auto, self, 7, [start.0, start.1, end.0, end.1, r, g, b, a]);
  }

  fn draw_text(&self, [r, g, b, a]: [u8; 4], x: i32, y: i32, scale: f32, text: &str) {
    let mut state = STATE.lock().unwrap();
    encode!(state, self, 8, [r, g, b, a, x, y, scale, text.len() as u64]);
    state.command_buffer.extend_from_slice(text.as_bytes());
  }
}