use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use crate::accel::impl_wasm::WasmAccelerator;
use crate::render::RenderContext;
use crate::tpse::TPSE;

mod tpse;
mod render;
pub(in crate) mod asynch;
pub(in crate) mod wasm_wakeable;
mod provide_asset;

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
  /// Requests an external asset be fetched and provided back asynchronously to `provide_asset`
  unsafe fn fetch_asset(asset_id: u32);
  /// Called when a panic occurrs. Logs with additional details will be printed to accompany
  unsafe fn report_panic();
  /// Prints a log not associated with any specific tpse instance  
  /// Level is 1=error 2=warn 3=info 4=debug 5=trace
  unsafe fn log(level: u8, ptr: *const u8, len: usize);
  /// Prints a log associated with a specific tpse instance  
  /// Level is 1=error 2=warn 3=info 4=debug 5=trace
  unsafe fn import_log(level: u8, tpse: u32, ptr: *const u8, len: usize);
}

// pub(in crate) type WasmGlobalAccelerator = crate::accel::impl_software::SoftwareRendering;
pub(in crate) type WasmGlobalAccelerator = WasmAccelerator;

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
  tpse: TPSE,
  render_data: Option<RenderContext<WasmGlobalAccelerator>>,
  import_status: ImportStatus,
  staged_files: Vec<StagedFile>
}
#[derive(Default, Debug)]
enum ImportStatus {
  #[default]
  Idle,
  Running
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


