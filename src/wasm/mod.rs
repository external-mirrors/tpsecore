use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use crate::accel::traits::TPSEAccelerator;
use crate::accel::wasm_asset_provider::WasmAssetProvider;
use crate::accel::wasm_audio_handle::WasmAudioHandle;
use crate::accel::wasm_decision_maker::WasmDecisionMaker;
use crate::render::RenderContext;
use crate::tpse::TPSE;
use crate::wasm::wasm_tpse_provider::WasmTPSEProvider;

mod tpse;
mod render;
pub(in crate) mod asynch;
pub(in crate) mod wasm_wakeable;
pub(in crate) mod wasm_tpse_provider;

#[link(wasm_import_module="tpsecore")]
unsafe extern "C" {
  /// Reports that an import has completed and that the results are now visible to `export_tpse`.  
  /// Code values: 0=success 1=failure 2=tpse disappeared before completion
  unsafe fn report_import_done(tpse: u32, code: u32);
  /// Reports that a migration has completed and that the results are now visible.
  /// Failures will be logged to the standard `log` function.
  /// Code values: 0=success 1=failure 2=tpse disappeared before completion (but operation completed)
  unsafe fn report_migration_done(tpse: u32, code: u32);
  /// Reports that rendering of a frame has been finished.
  /// Gives back the nonce used to identify the frame and the location of the buffer containing
  /// either the rendered frame (when status=0) or an error string (when status=1)
  unsafe fn report_frame_render_done(tpse: u32, nonce: u64, status: u8, ptr: *const u8, len: usize);
  /// Controls whether `tick_async` is called
  unsafe fn set_runtime_sleeping(sleep: bool);
  /// Called when a panic occurrs. Logs with additional details will be printed to accompany
  unsafe fn report_panic();
  /// Prints a log not associated with any specific tpse instance  
  /// Level is 1=error 2=warn 3=info 4=debug 5=trace
  unsafe fn log(level: u8, ptr: *const u8, len: usize);
  /// Prints a log associated with a specific tpse instance, with all log related metadata in the provided buffer
  unsafe fn import_log(tpse: u32, ptr: *const u8, len: usize);
  /// Obtains a key from browser storage. A status code (0=ok 1=novalue 2=fail) and buffer (result or error) should be
  /// provided asynchronously. The buffer will be deallocated internally.
  unsafe fn tpse_get(extern_tpse_id: u32, key_ptr: *const u8, key_len: usize, wake_id: u64);
  /// Writes a key into browser storage. A status code (0=ok other=pointer to error buffer) should be
  /// provided asynchronously. The buffer will be deallocated internally.
  unsafe fn tpse_set(extern_tpse_id: u32, key_ptr: *const u8, key_len: usize, data_ptr: *const u8, data_len: usize, wake_id: u64);
  /// Writes a key into browser storage. A status code (0=ok other=pointer to error buffer) should be
  /// provided asynchronously. The buffer will be deallocated internally.
  unsafe fn tpse_delete(extern_tpse_id: u32, key_ptr: *const u8, key_len: usize, wake_id: u64);
}

#[derive(Debug, Clone)]
pub struct WasmGlobalAccelerator;
impl TPSEAccelerator for WasmGlobalAccelerator {
  type Decider = WasmDecisionMaker;
  
  type Asset = WasmAssetProvider;
  
  #[cfg(all(not(feature = "software_texture"), not(feature = "wasm_rendering")))]
  type Texture = crate::accel::null_texture_handle::NullTextureHandle;
  #[cfg(all(feature = "software_texture", not(feature = "wasm_rendering")))]
  type Texture = crate::accel::software_texture_handle::SoftwareTextureHandle;
  #[cfg(feature = "wasm_rendering")]
  type Texture = crate::accel::wasm_texture_handle::WasmTextureHandle;
  
  type Audio = WasmAudioHandle;
}

pub(in crate) static BUFFER_STATE: LazyLock<Mutex<BufferState>> = LazyLock::new(|| {
  let _ = *TPSE_STATE; // ensure other_initialization is called
  Default::default()
});
pub(in crate) static TPSE_STATE: LazyLock<Mutex<TPSEState>> = LazyLock::new(|| {
  other_initialization();
  Default::default()
});

#[derive(Default)]
pub(in crate) struct BufferState {
  id_counter: u32,
  pub(in crate) buffers: HashMap<u32, Arc<[u8]>>
}

#[derive(Default)]
pub(in crate) struct TPSEState {
  id_counter: u32,
  tpses: HashMap<u32, TPSEContext>,
}

#[derive(Default)]
struct TPSEContext {
  status: TPSEStatus,
  render_data: Option<RenderContext<WasmGlobalAccelerator>>,
  staged_files: Vec<StagedFile>
}
#[derive(Debug)]
enum TPSEStatus {
  IdleInternal(TPSE),
  IdleExternal(WasmTPSEProvider),
  /// The TPSE is busy and unavailable because an import is running
  Busy
}
impl Default for TPSEStatus {
  fn default() -> Self {
    Self::IdleInternal(Default::default())
  }
}

pub enum ActiveTPSEStatus {
  IdleInternal(TPSE),
  IdleExternal(WasmTPSEProvider)
}
macro_rules! over_tpse_status {
  ($type:ident, $expr:expr, $bind:ident, $body:block$(, $default:block)?) => {
    match $expr {
      $type::IdleInternal($bind) => $body,
      $type::IdleExternal($bind) => $body,
      $(_ => $default)?
    }
  }
}
pub(in crate) use over_tpse_status;


#[derive(Debug)]
struct StagedFile {
  filename: Arc<[u8]>,
  content: Arc<[u8]>
}

impl BufferState {
  pub(in crate) fn next_id(&mut self) -> u32 {
    // realistically we should never reach this; tpsecore is initialized,
    // user manually drops files, then closes the window. If you actually
    // manage to drag-and-drop 2 billion files in one session, lol.
    // also note that right now, we slightly rely on IDs not being reused:
    // `queue_import` expects the same TPSE to be in the same place and
    // if IDs are reused will blindly use the new object. More of the caller's
    // fault for deallocating it before the import finishes, though.
    let Some(new_id) = self.id_counter.checked_add(1)
      else { panic!("out of IDs") };
    std::mem::replace(&mut self.id_counter, new_id)
  }
  pub(in crate) fn lookup_buffer(&self, ptr: *const u8) -> Option<u32> {
    self.buffers.iter()
      .find(|(_, v)| v.as_ptr() == ptr)
      .map(|(k, _)| *k)
  }
}
impl TPSEState {
  pub(in crate) fn next_id(&mut self) -> u32 {
    let Some(new_id) = self.id_counter.checked_add(1)
      else { panic!("out of IDs") };
    std::mem::replace(&mut self.id_counter, new_id)
  }
}

fn other_initialization() {
  log::set_logger(&WasmLogger).map(|()| log::set_max_level(log::LevelFilter::Debug)).unwrap();
  std::panic::set_hook(Box::new(|info| {
    log::error!("panic: {info}");
    unsafe { report_panic(); }
  }));
}

struct WasmLogger;
impl log::Log for WasmLogger {
  fn enabled(&self, _metadata: &log::Metadata) -> bool {
    true
  }

  fn log(&self, record: &log::Record) {
    use std::fmt::Write;
    let mut cursor = String::with_capacity(256);
    
    if let Some(file) = record.file() {
      write!(cursor, "{file}").unwrap();
    }
    if let Some(line) = record.line() {
      write!(cursor, ":{line} ").unwrap();
    }
    write!(cursor, "{}", record.args()).unwrap();
    
    unsafe { log(record.level() as u8, cursor.as_ptr(), cursor.len()); }
  }

  fn flush(&self) {
  }
}


