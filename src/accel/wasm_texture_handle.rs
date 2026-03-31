use std::mem::take;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use async_channel::Sender;

use crate::accel::traits::TextureHandle;
use crate::wasm::BUFFER_STATE;
use crate::wasm::wasm_wakeable::{WasmWakeable, WasmWakeableSize, provide_wakeable};

#[derive(Clone, Debug)]
pub struct WasmTextureHandle(Arc<WasmTextureHandleInner>);

#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub struct WasmAcceleratorError(String);

static STATE: LazyLock<Mutex<WasmAcceleratorState>> = LazyLock::new(Default::default);
#[derive(Default)]
struct WasmAcceleratorState {
  id_counter: u64,
  command_buffer: Vec<u8>,
  arcs_in_command_buffer: Vec<Arc<[u8]>>
}
impl WasmAcceleratorState {
  pub fn new_handle(&mut self, init_dimension: Option<(u32, u32)>) -> WasmTextureHandle {
    let dimensions = OnceLock::new();
    if let Some(init_dimension) = init_dimension {
      dimensions.set(init_dimension).expect("freshly made oncelock");
    }
    let handle = WasmTextureHandle(Arc::new(WasmTextureHandleInner {
      id: self.id_counter,
      dimensions
    }));
    self.id_counter += 1;
    handle
  }
  /// Flushes, clears, and if necessary shrinks the command buffer.
  /// The existing buffers are detached and moved to a background task for processing.
  /// The returned promise should be awaited only after dropping the lock on self.
  /// Returns None if the buffers are empty.
  fn flush_command_buffer(&mut self) -> Option<WasmWakeable> {
    if self.command_buffer.is_empty() { return None }
    let (wake_id, future) = WasmWakeable::new();
    let Ok(None) = COMMAND_FLUSH_TASK.force_send(FlushedCommands {
      command_buffer: take(&mut self.command_buffer),
      arcs_in_command_buffer: take(&mut self.arcs_in_command_buffer),
      notify: wake_id
    }) else {
      unreachable!("force_send shifted a value on an unbounded channel?");
    };
    Some(future)
  }
}

struct FlushedCommands {
  command_buffer: Vec<u8>,
  #[allow(dead_code)] // it's never "read" but it _is_ dropped for effect
  arcs_in_command_buffer: Vec<Arc<[u8]>>,
  notify: u64
}
static COMMAND_FLUSH_TASK: LazyLock<Sender<FlushedCommands>> = LazyLock::new(|| {
  let (tx, rx) = async_channel::unbounded::<FlushedCommands>();
  crate::wasm::asynch::spawn(async move {
    while let Ok(flush) = rx.recv().await {
      let (wake_id, future) = WasmWakeable::new();
      unsafe { flush_command_buffer(flush.command_buffer.as_ptr(), flush.command_buffer.len(), wake_id); }
      assert_eq!(
        Ok(WasmWakeableSize::Zero),
        future.await,
        "incorrect size provided or sender dropped for flush_command_buffer wakeup"
      );
      provide_wakeable(flush.notify, WasmWakeableSize::Zero);
      // buffer dropped and deallocated
    }
  });
  tx
});

#[link(wasm_import_module="wasm_accelerator_texture")]
unsafe extern "C" {
  /// Processes all commands in the command buffer
  /// This needs to be called before other methods to ensure the state they query is actually available
  unsafe fn flush_command_buffer(command_buffer: *const u8, len: usize, async_wake_id: u64);
  
  /// Fetches dimensions for the given handle, storing them in provided outvalues.
  ///
  /// If out_code is nonzero, indicates a pointer containing an error that should be retrieved from the TPSE buffer 
  /// infrastructure. The buffer will be deallocated internally, manual deallocation is not required.
  unsafe fn fetch_dimensions(id: u64, out_code: *mut u64, out_width: *mut u32, out_height: *mut u32);
  
  /// Calculates what fraction of the image is made up of pixels with opacity > 0
  ///
  /// If out_code is nonzero, indicates a pointer containing an error that should be retrieved from the TPSE buffer 
  /// infrastructure. The buffer will be deallocated internally, manual deallocation is not required.
  unsafe fn fraction_opaque(id: u64, out_code: *mut u64) -> f32;
  
  /// Encodes the given handle into a buffer managed by the TPSE buffer infastructure
  ///
  /// The waker takes two arguments: a status code (0=ok, 1=error) and a pointer to a buffer containing either the
  /// encoded png or the error message. The buffer will be deallocated internally, manual deallocation is not required.
  unsafe fn encode_png(id: u64, async_wake_id: u64) -> *const u8;
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

#[derive(Debug)]
struct WasmTextureHandleInner {
  id: u64,
  dimensions: OnceLock<(u32, u32)>
}
impl Drop for WasmTextureHandleInner {
  fn drop(&mut self) {
    // locking state shouldn't deadlock so long as we don't go dropping handles inside any of the impl
    // WasmTextureHandle methods
    let mut state = STATE.lock().unwrap();
    state.command_buffer.push(0);
    state.command_buffer.extend(self.id.to_be_bytes());
  }
}

impl WasmTextureHandle {
  pub fn id(&self) -> u64 {
    self.0.id
  }
}

impl WasmTextureHandle {
  pub async fn force_flush() {
    let flush_complete = STATE.lock().unwrap().flush_command_buffer();
    if let Some(f) = flush_complete { // STATE lock dropped by this point
      assert_eq!(f.await, Ok(WasmWakeableSize::Zero));
    }
  }
  
  async fn dimensions(&self) -> Result<(u32, u32), WasmAcceleratorError> {
    if self.0.dimensions.get().is_none() {
      let flush_complete = STATE.lock().unwrap().flush_command_buffer();
      if let Some(f) = flush_complete { // STATE lock dropped by this point
        assert_eq!(f.await, Ok(WasmWakeableSize::Zero));
      }
      
      // even though state has been dropped, the texture handle can't be invalidated
      // because we're still a reference counted copy of it and invalidation doesn't
      // occur until Drop of the inner value.
      // At worst, something might be drawn on top of the handle, which doesn't matter
      // since we're just retrieving the dimensions here. All size-changing operations
      // only operate by creating a texture with a new handle.
      
      let mut code: u64 = 0;
      let mut width: u32 = 0;
      let mut height: u32 = 0;
      unsafe { fetch_dimensions(self.0.id, &mut code, &mut width, &mut height) };
      
      if code != 0 {
        let mut state = BUFFER_STATE.lock().unwrap();
        let buf_id = state.lookup_buffer(code as *mut u8).unwrap();
        let data = state.buffers.remove(&buf_id).unwrap();
        let message = String::from_utf8_lossy(&*data).into_owned();
        return Err(WasmAcceleratorError(message));
      }
      
      let _ = self.0.dimensions.set((width, height));
    }
    Ok(*self.0.dimensions.get().unwrap())
  }
}
impl TextureHandle for WasmTextureHandle {
  type Error = WasmAcceleratorError;

  fn new_texture(width: u32, height: u32) -> Self {
    let mut state = STATE.lock().unwrap();
    let handle = state.new_handle(Some((width, height)));
    encode!(state, handle, 1, [width, height]);
    handle
  }

  fn decode_texture(buffer: Arc<[u8]>) -> Result<Self, Self::Error> {
    let mut state = STATE.lock().unwrap();
    let handle = state.new_handle(None);
    encode!(state, handle, 2, [buffer.as_ptr() as u64, buffer.len() as u64]);
    state.arcs_in_command_buffer.push(buffer);
    Ok(handle)
  }
  
  async fn width(&self) -> Result<u32, Self::Error> {
    Ok(self.dimensions().await?.0)
  }

  async fn height(&self) -> Result<u32, Self::Error> {
    Ok(self.dimensions().await?.1)
  }

  async fn encode_png(&self) -> Result<Arc<[u8]>, Self::Error> {
    let flush_complete = STATE.lock().unwrap().flush_command_buffer();
    if let Some(f) = flush_complete { f.await.expect("should never be dropped"); } // STATE lock dropped at this point
    
    let state = STATE.lock().unwrap();
    let (wake_id, future) = WasmWakeable::new();
    unsafe { encode_png(self.0.id, wake_id) };
    drop(state);
    
    let result = future.await;
    let Ok(WasmWakeableSize::Two(error, ptr)) = result else {
      panic!("incorrect size provided or sender dropped for encode_png wakeup, got: {:?}", result);
    };
    
    let mut state = BUFFER_STATE.lock().unwrap();
    let buf_id = state.lookup_buffer(ptr as *mut u8).unwrap();
    let buffer = state.buffers.remove(&buf_id).unwrap();
    match error {
      0 => Ok(buffer),
      _nonzero => {
        let message = String::from_utf8_lossy(&*buffer).into_owned();
        Err(WasmAcceleratorError(message))
      }
    }
  }
  
  async fn fraction_opaque(&self) -> Result<f32, Self::Error> {
    let flush_complete = STATE.lock().unwrap().flush_command_buffer();
    if let Some(f) = flush_complete { f.await.expect("should never be dropped"); } // STATE lock dropped at this point
    
    let mut code: u64 = 0;
    let fraction = unsafe { fraction_opaque(self.0.id, &mut code) };
    
    if code != 0 {
      let mut state = BUFFER_STATE.lock().unwrap();
      let buf_id = state.lookup_buffer(code as *mut u8).unwrap();
      let data = state.buffers.remove(&buf_id).unwrap();
      let message = String::from_utf8_lossy(&*data).into_owned();
      return Err(WasmAcceleratorError(message));
    }
    
    Ok(fraction)
  }

  fn create_copy(&self) -> Self {
    encode!(auto, self, 3, new(self.0.dimensions.get().copied()), [])
  }

  fn slice(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
    encode!(auto, self, 4, new(self.0.dimensions.get().copied()), [x, y, width, height])
  }

  fn resized(&self, width: u32, height: u32) -> Self {
    encode!(auto, self, 5, new(Some((width, height))), [width, height])
  }

  fn tinted(&self, [r, g, b, a]: [u8; 4]) -> Self {
    encode!(auto, self, 6, new(self.0.dimensions.get().copied()), [r, g, b, a])
  }

  fn overlay(&self, with_image: &Self, x: i64, y: i64) {
    encode!(auto, self, 7, [with_image.0.id, x, y])
  }

  fn draw_line(&self, start: (f32, f32), end: (f32, f32), [r, g, b, a]: [u8; 4]) {
    encode!(auto, self, 8, [start.0, start.1, end.0, end.1, r, g, b, a]);
  }

  fn draw_text(&self, [r, g, b, a]: [u8; 4], x: i32, y: i32, scale: f32, text: &str) {
    let mut state = STATE.lock().unwrap();
    encode!(state, self, 9, [r, g, b, a, x, y, scale, text.as_ptr() as u64, text.len() as u64]);
    state.command_buffer.extend_from_slice(text.as_bytes());
  }
}