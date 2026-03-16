use std::mem::replace;
use std::ptr::null;

use log::Level;

use crate::accel::wasm_asset_provider::WasmAssetProvider;
use crate::import::{import, ImportContext, ImportType};
use crate::log::ImportLogger;
use crate::tpse::tpse_key::merge;
use crate::wasm::{ImportStatus, STATE, StagedFile, State, TPSEContext, WasmGlobalAccelerator, import_log, report_import_done};


#[unsafe(no_mangle)]
pub extern "C" fn dump_loaded_asset_debug() {
  let state = STATE.lock().unwrap();
  log::info!("dump_loaded_asset_debug()");
  for (id, tpse) in &state.tpses {
    log::info!("tpse {id} status={:?} render_data={} staged_files={:?}", tpse.import_status, tpse.render_data.is_some(), tpse.staged_files);
  }
  for (id, buf) in &state.buffers {
    log::info!("buffer {id} len={} head={:?}", buf.len(), &buf[0..16.min(buf.len())]);
  }
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_tpse() -> u32 {
  let mut state = STATE.lock().unwrap();
  let id = state.next_id();
  state.tpses.insert(id, TPSEContext::default());
  id
}

/// Return codes: 0=ok, 1=no such tpse
#[unsafe(no_mangle)]
pub extern "C" fn deallocate_tpse(tpse_id: u32, deallocate_attached_buffers: bool) -> u32 {
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
pub extern "C" fn allocate_buffer(length: usize) -> *const u8 {
  let mut state = STATE.lock().unwrap();
  let id = state.next_id();
  state.buffers.insert(id, vec![0; length].into());
  state.buffers.get_mut(&id).unwrap().as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn get_buffer_length(ptr: *mut u8) -> usize {
  let state = STATE.lock().unwrap();
  let Some(id) = state.lookup_buffer(ptr) else { return 0 };
  state.buffers.get(&id).unwrap().len()
}

/// Return codes: 0=ok, 1=no such buffer
#[unsafe(no_mangle)]
pub extern "C" fn deallocate_buffer(ptr: *mut u8) -> u32 {
  let mut state = STATE.lock().unwrap();
  let Some(id) = state.lookup_buffer(ptr) else { return 1 };
  state.buffers.remove(&id);
  0
}

/// Stages a file for import into a given TPSE by handle  
/// Return codes: 0=ok, 1=no such tpse, 2=no such filename buffer, 3=no such content buffer
#[unsafe(no_mangle)]
pub extern "C" fn stage_file(tpse_id: u32, filename: *mut u8, content: *mut u8) -> u32 {
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
pub extern "C" fn clear_staged_files(tpse_id: u32, deallocate_buffers: bool) -> u32 {
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
/// Files are _not_ unstaged after this process and [clear_staged_files] must be called manually.
/// It is safe to call [clear_staged_files] before the import finishes; reference-counted copies are made.
///
/// Return codes: 0=import queued, 1=no such tpse, 2=invalid file staged to tpse, 3=import already running
#[unsafe(no_mangle)]
pub extern "C" fn queue_import(tpse_id: u32) -> usize {
  let mut state = STATE.lock().unwrap();
  let State { tpses, buffers, .. } = &mut *state;
  let Some(tpse) = tpses.get_mut(&tpse_id) else { return 1 };
  let ImportStatus::Idle(mut tpse_data) = replace(&mut tpse.import_status, ImportStatus::Running) else {
    return 3; // tpse already running, can't get a handle to it
  };
  let Ok(cloned) = tpse.staged_files.iter()
    .map(|file| Ok((
      buffers.get(&file.filename).ok_or(())?.clone(),
      buffers.get(&file.content).ok_or(())?.clone(),
    )))
    .collect::<Result<Vec<_>, ()>>()
    else { return 2 };
  
  crate::wasm::asynch::spawn(async move {
    let files = cloned.iter().map(|(filename, content)| (
      ImportType::Automatic,
      str::from_utf8(&filename).unwrap(),
      content.clone()
    )).collect::<Vec<_>>();
  
    let source = WasmAssetProvider;
    let logger = WasmImportLogger { tpse_id };
    let options = ImportContext::new(&source, 5).with_logger(&logger);
  
    let result = import::<WasmGlobalAccelerator>(files, options).await;
    match &result {
      Err(error) => logger.log(log::Level::Error, format_args!("import failed: {error}")),
      Ok(_) => logger.log(log::Level::Info, format_args!("import finished"))
    };
    
    let merge_result = match result {
      Err(err) => {
        logger.log(Level::Error, format_args!("import failed: {err}"));
        Err(1) // import failed
      }
      Ok(new_tpse) => {
        match merge(&mut tpse_data, &new_tpse).await {
          Err(err) => {
            logger.log(Level::Error, format_args!("failed to merge final import result upon TPSE: {err}"));
            Err(1) // import failed
          }
          Ok(()) => Ok(())
        }
        
      },
    };
    
    let code = match (STATE.lock().unwrap().tpses.get_mut(&tpse_id), merge_result) {
      (None, _) => 2, // tpse disappeared
      (Some(tpse), Err(code)) => {
        tpse.import_status = ImportStatus::Idle(tpse_data);
        code
      }
      (Some(tpse), Ok(())) => {
        tpse.import_status = ImportStatus::Idle(tpse_data);
        0 // success!
      }
    };
    
    unsafe { report_import_done(tpse_id, code); }
  });
  0
}

/// Serializes a TPSE and returns a pointer to the buffer holding the serialized data.
/// Returns 0 if the TPSE handle is invalid or the TPSE is busy importing
#[unsafe(no_mangle)]
pub extern "C" fn export_tpse(tpse_id: u32) -> *const u8 {
  let mut state = STATE.lock().unwrap();
  let id = state.next_id();
  let State { tpses, .. } = &mut *state;
  let Some(tpse) = tpses.get_mut(&tpse_id) else { return null() };
  let ImportStatus::Idle(tpse) = &mut tpse.import_status else { return null() };
  
  let encoded = serde_json::to_vec(&tpse).unwrap();
  state.buffers.insert(id, encoded.into());
  state.buffers.get_mut(&id).unwrap().as_ptr()
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


