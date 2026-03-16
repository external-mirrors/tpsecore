use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use crate::accel::software_audio_handle::SoftwareAudioHandle;
use crate::accel::traits::TPSEAccelerator;
use crate::accel::wasm_asset_provider::WasmAssetProvider;
use crate::render::RenderContext;
use crate::tpse::TPSE;

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
  /// Prints a log associated with a specific tpse instance  
  /// Level is 1=error 2=warn 3=info 4=debug 5=trace
  unsafe fn import_log(level: u8, tpse: u32, ptr: *const u8, len: usize);
  /// Obtains a key from browser storage, or a null pointer for null.
  /// The buffer should be deallocated manually with [deallocate_buffer].
  unsafe fn tpse_get(key_ptr: *const u8, key_len: usize) -> *const u8;
  /// Writes a key into browser storage
  unsafe fn tpse_set(key_ptr: *const u8, key_len: usize, data_ptr: *const u8, data_len: usize);
}

#[derive(Debug, Clone)]
pub struct WasmGlobalAccelerator;
impl TPSEAccelerator for WasmGlobalAccelerator {
  type Asset = WasmAssetProvider;
  
  #[cfg(all(not(feature = "software_texture"), not(feature = "wasm_rendering")))]
  type Texture = crate::accel::null_texture_handle::NullTextureHandle;
  #[cfg(all(feature = "software_texture", not(feature = "wasm_rendering")))]
  type Texture = crate::accel::software_texture_handle::SoftwareTextureHandle;
  #[cfg(feature = "wasm_rendering")]
  type Texture = crate::accel::wasm_texture_handle::WasmTextureHandle;
  
  type Audio = SoftwareAudioHandle;
}

pub(in crate) static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
  other_initialization();
  Default::default()
});

#[derive(Default)]
pub(in crate) struct State {
  id_counter: u32,
  tpses: HashMap<u32, TPSEContext>,
  pub(in crate) buffers: HashMap<u32, Arc<[u8]>>
}

#[derive(Default)]
struct TPSEContext {
  render_data: Option<RenderContext<WasmGlobalAccelerator>>,
  import_status: ImportStatus,
  staged_files: Vec<StagedFile>
}
/// The import status of the TPSE. While the import is running, the import task temporarily
/// takes ownership of the actual TPSE in order to do async writes against it. Thus, other
/// operations that affect the TPSE are not valid during import.
#[derive(Debug)]
enum ImportStatus {
  Idle(TPSE),
  Running
}
impl Default for ImportStatus {
  fn default() -> Self {
    Self::Idle(Default::default())
  }
}

#[derive(Default, Debug)]
struct StagedFile {
  filename: u32,
  content: u32
}

impl State {
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
  pub(in crate) fn lookup_buffer(&self, ptr: *mut u8) -> Option<u32> {
    self.buffers.iter()
      .find(|(_, v)| v.as_ptr() == ptr)
      .map(|(k, _)| *k)
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


