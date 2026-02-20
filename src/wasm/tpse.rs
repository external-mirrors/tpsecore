use std::ptr::null;

use crate::import::{import, Asset, AssetProvider, ImportContext, ImportErrorType, ImportType};
use crate::log::ImportLogger;
use crate::tpse::TPSE;
use crate::wasm::{import_log, StagedFile, State, TPSEContext, STATE};

#[unsafe(no_mangle)]
pub extern fn allocate_tpse() -> u32 {
  let mut state = STATE.lock().unwrap();
  let id = state.next_id();
  state.tpses.insert(id, TPSEContext::default());
  id
}

/// Return codes: 0=ok, 1=no such tpse
#[unsafe(no_mangle)]
pub extern fn deallocate_tpse(tpse_id: u32, deallocate_attached_buffers: bool) -> u32 {
  if deallocate_attached_buffers {
    clear_staged_files(tpse_id, true);
  }
  let mut state = STATE.lock().unwrap();
  match state.tpses.remove(&tpse_id) {
    Some(_) => 0,
    None => 1
  }
}

#[unsafe(no_mangle)]
pub extern fn allocate_buffer(length: usize) -> *mut u8 {
  let mut state = STATE.lock().unwrap();
  let id = state.next_id();
  state.buffers.insert(id, vec![0; length]);
  state.buffers.get_mut(&id).unwrap().as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern fn get_buffer_length(ptr: *mut u8) -> usize {
  let mut state = STATE.lock().unwrap();
  let Some(id) = state.lookup_buffer(ptr) else { return 0 };
  state.buffers.get(&id).unwrap().len()
}

/// Return codes: 0=ok, 1=no such buffer
#[unsafe(no_mangle)]
pub extern fn deallocate_buffer(ptr: *mut u8) -> u32 {
  let mut state = STATE.lock().unwrap();
  let Some(id) = state.lookup_buffer(ptr) else { return 1 };
  state.buffers.remove(&id);
  0
}

/// Stages a file for import into a given TPSE by handle  
/// Return codes: 0=ok, 1=no such tpse, 2=no such filename buffer, 3=no such content buffer
#[unsafe(no_mangle)]
pub extern fn stage_file(tpse_id: u32, filename: *mut u8, content: *mut u8) -> u32 {
  let mut state = STATE.lock().unwrap();
  let Some(filename) = state.lookup_buffer(filename) else { return 2 };
  let Some(content) = state.lookup_buffer(content) else { return 3 };
  let Some(tpse) = state.tpses.get_mut(&tpse_id) else { return 1 };
  
  tpse.staged_files.push(StagedFile {
    filename,
    content,
  });
  0
}

/// Return codes: 0=ok, 1=no such tpse
#[unsafe(no_mangle)]
pub extern fn clear_staged_files(tpse_id: u32, deallocate_buffers: bool) -> u32 {
  let mut state = STATE.lock().unwrap();
  let State { tpses, buffers, .. } = &mut *state;
  let Some(tpse) = tpses.get_mut(&tpse_id) else { return 1 };
  for entry in tpse.staged_files.drain(..) {
    if deallocate_buffers {
      buffers.remove(&entry.filename);
      buffers.remove(&entry.content);
    }
  }
  0
}

/// Runs import for a tpse using its staged files, merging the result with the tpse slot.
/// Files are _not_ unstaged after this process and [clear_staged_files] must be called manually
/// Return codes: 0=ok, 1=no such tpse, 2=general import error (see logs)
#[unsafe(no_mangle)]
pub extern fn run_import(tpse_id: u32) -> u32 {
  let mut state = STATE.lock().unwrap();
  let State { tpses, buffers, .. } = &mut *state;
  let Some(tpse) = tpses.get_mut(&tpse_id) else { return 1 };
  let files = tpse.staged_files.iter().map(|file| {
    
    log::info!(
      "file lookup {:?} {:?} {:?}",
      file.filename,
      buffers.get(&file.filename).unwrap(),
      str::from_utf8(buffers.get(&file.filename).unwrap()).unwrap(),
    );
    (
      ImportType::Automatic,
      str::from_utf8(buffers.get(&file.filename).unwrap()).unwrap(),
      &buffers.get(&file.content).unwrap()[..]
    )
  }).collect();
  log::info!("operating on files {files:?}");
  
  let source = WasmAssetProvider;
  let logger = WasmImportLogger { tpse_id: 0 };
  let options = ImportContext::new(&source, 5).with_logger(&logger);
  
  logger.log(log::Level::Error, format_args!("error"));
  logger.log(log::Level::Warn, format_args!("warn"));
  logger.log(log::Level::Info, format_args!("info"));
  logger.log(log::Level::Debug, format_args!("debug"));
  logger.log(log::Level::Trace, format_args!("trace"));
  
  let result = import(files, options);
  match result {
    Err(error) => {
      logger.log(log::Level::Error, format_args!("import failed: {error}"));
      2
    },
    Ok(new_tpse) => {
      tpse.tpse.merge(new_tpse);
      0
    }
  }
}

#[unsafe(no_mangle)]
pub extern fn export_tpse(tpse_id: u32) -> *const u8 {
  let mut state = STATE.lock().unwrap();
  let id = state.next_id();
  let State { tpses, buffers, .. } = &mut *state;
  let Some(tpse) = tpses.get_mut(&tpse_id) else { return null() };
  
  let encoded = serde_json::to_vec(&tpse.tpse).unwrap();
  state.buffers.insert(id, encoded);
  state.buffers.get_mut(&id).unwrap().as_mut_ptr()
}

struct WasmAssetProvider;
impl AssetProvider for WasmAssetProvider {
  fn provide(&self, asset: Asset) -> Result<&[u8], ImportErrorType> {
    Err(ImportErrorType::AssetNotPreloaded(asset))
  }
}
  
struct WasmImportLogger { tpse_id: u32 }
impl ImportLogger for WasmImportLogger {
  fn log(&self, level: log::Level, msg: std::fmt::Arguments) {
    let formatted = msg.to_string();
    let bytes = formatted.as_bytes();
    unsafe {
      import_log(level as u8, self.tpse_id, bytes.as_ptr(),   bytes.len());
    }
  }
}