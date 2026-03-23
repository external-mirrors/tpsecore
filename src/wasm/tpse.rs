use std::fmt::Display;
use std::mem::replace;
use std::path::PathBuf;
use std::ptr::null;

use serde_json::json;

use crate::accel::wasm_asset_provider::WasmAssetProvider;
use crate::import::inter_stage_data::QueuedFile;
use crate::import::{ImportContext, ImportContextEntry, ImportType, TPSEProviderError, import};
use crate::log::{ImportLogger, LogLevel};
use crate::tpse::{DynamicTPSE, migrate};
use crate::tpse::tpse_key::{TPSEProvider, RawTPSEKey, merge};
use crate::wasm::wasm_tpse_provider::WasmTPSEProvider;
use crate::wasm::{ActiveTPSEStatus, BUFFER_STATE, StagedFile, TPSE_STATE, TPSEContext, TPSEStatus, WasmGlobalAccelerator, import_log, over_tpse_status, report_import_done, report_migration_done};

#[unsafe(no_mangle)]
pub extern "C" fn dump_loaded_asset_debug() {
  log::info!("dump_loaded_asset_debug()");
  
  let tpse_state = TPSE_STATE.lock().unwrap();
  for (id, tpse) in &tpse_state.tpses {
    log::info!("tpse {id} status={:?} render_data={} staged_files={:?}", tpse.status, tpse.render_data.is_some(), tpse.staged_files);
  }
  drop(tpse_state);
  
  let buffer_state = BUFFER_STATE.lock().unwrap();
  for (id, buf) in &buffer_state.buffers {
    log::info!("buffer {id} len={} head={:?}", buf.len(), &buf[0..16.min(buf.len())]);
  }
}

/// Allocates an _external_ tpse which is read and updated via [tpse_get] and [tpse_set]
/// and which has some usage restrictions on other methods.
#[unsafe(no_mangle)]
pub extern "C" fn allocate_extern_tpse() -> u32 {
  let mut state = TPSE_STATE.lock().unwrap();
  let id = state.next_id();
  state.tpses.insert(id, TPSEContext {
    status: TPSEStatus::IdleExternal(WasmTPSEProvider(id)),
    ..Default::default()
  });
  id
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_tpse() -> u32 {
  let mut state = TPSE_STATE.lock().unwrap();
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
  
  let mut state = TPSE_STATE.lock().unwrap();
  match state.tpses.remove(&tpse_id) {
    Some(_) => 0,
    None => 1
  }
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_buffer(length: usize) -> *const u8 {
  let mut state = BUFFER_STATE.lock().unwrap();
  let id = state.next_id();
  state.buffers.insert(id, vec![0; length].into());
  state.buffers.get_mut(&id).unwrap().as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn get_buffer_length(ptr: *mut u8) -> usize {
  let state = BUFFER_STATE.lock().unwrap();
  let Some(id) = state.lookup_buffer(ptr) else { return 0 };
  state.buffers.get(&id).unwrap().len()
}

/// Return codes: 0=ok, 1=no such buffer
#[unsafe(no_mangle)]
pub extern "C" fn deallocate_buffer(ptr: *mut u8) -> u32 {
  let mut state = BUFFER_STATE.lock().unwrap();
  let Some(id) = state.lookup_buffer(ptr) else { return 1 };
  state.buffers.remove(&id);
  0
}

/// Stages a file for import into a given TPSE by handle  
/// Return codes: 0=ok, 1=no such tpse, 2=no such filename buffer, 3=no such content buffer
#[unsafe(no_mangle)]
pub extern "C" fn stage_file(tpse_id: u32, filename: *mut u8, content: *mut u8) -> u32 {
  let state = BUFFER_STATE.lock().unwrap();
  let Some(filename) = state.lookup_buffer(filename) else { return 2 };
  let Some(content) = state.lookup_buffer(content) else { return 3 };
  drop(state);
  
  let mut state = TPSE_STATE.lock().unwrap();
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
  let mut tpse_state = TPSE_STATE.lock().unwrap();
  let mut buffer_state = BUFFER_STATE.lock().unwrap();
  let Some(tpse) = tpse_state.tpses.get_mut(&tpse_id) else { return 1 };
  for entry in tpse.staged_files.drain(..) {
    if deallocate_buffers {
      buffer_state.buffers.remove(&entry.filename);
      buffer_state.buffers.remove(&entry.content);
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
  let mut tpse_state = TPSE_STATE.lock().unwrap();
  let buffer_state = BUFFER_STATE.lock().unwrap();
  let Some(tpse) = tpse_state.tpses.get_mut(&tpse_id) else { return 1 };
  
  let status = replace(&mut tpse.status, TPSEStatus::Busy);
  let mut tpse_data = match status {
    TPSEStatus::IdleInternal(tpse) => ActiveTPSEStatus::IdleInternal(tpse),
    TPSEStatus::IdleExternal(wasm) => ActiveTPSEStatus::IdleExternal(wasm),
    TPSEStatus::Busy => return 3 // tpse already running, can't get a handle to it
  };
  let Ok(cloned) = tpse.staged_files.iter()
    .map(|file| Ok((
      buffer_state.buffers.get(&file.filename).ok_or(())?.clone(),
      buffer_state.buffers.get(&file.content).ok_or(())?.clone(),
    )))
    .collect::<Result<Vec<_>, ()>>()
    else { return 2 };
  
  crate::wasm::asynch::spawn(async move {
    let files = cloned.iter().map(|(filename, content)| QueuedFile {
      kind: ImportType::Automatic,
      path: PathBuf::from(str::from_utf8(&filename).unwrap()),
      binary: content.clone()
    }).collect::<Vec<_>>();
  
    let source = WasmAssetProvider;
    let logger = WasmImportLogger { tpse_id };
    let mut context = ImportContext::new(&source).with_logger(&logger);
  
    let result = import::<WasmGlobalAccelerator>(&mut context, files).await;
    let merge_result = match result {
      Err(err) => {
        logger.log(LogLevel::Error, &[], &format_args!("import failed: {err}"));
        Some(1) // import failed
      }
      Ok(new_tpse) => {
        logger.log(LogLevel::Info, &[], &"import finished");
        over_tpse_status!(ActiveTPSEStatus, &mut tpse_data, tpse, {
          match merge(tpse, &new_tpse).await {
            Err(err) => {
              logger.log(LogLevel::Error, &[], &format_args!("failed to merge final import result upon TPSE: {err}"));
              Some(1) // import failed
            }
            Ok(_) => None
          }
        })
      },
    };
    
    let code = match TPSE_STATE.lock().unwrap().tpses.get_mut(&tpse_id) {
      None => 2, // tpse disappeared
      Some(tpse) => {
        tpse.status = match tpse_data {
          ActiveTPSEStatus::IdleInternal(tpse) => TPSEStatus::IdleInternal(tpse),
          ActiveTPSEStatus::IdleExternal(wasm) => TPSEStatus::IdleExternal(wasm)
        };
        merge_result.unwrap_or(0 /* succcess! */)
      }
    };
    
    unsafe { report_import_done(tpse_id, code); }
  });
  0
}

/// Return codes: 0=migration queued, 1=no such tpse, 2=tpse not extern, 3=import or migration already running
#[unsafe(no_mangle)]
pub extern "C" fn migrate_extern_tpse(tpse_id: u32) -> usize {
  let mut tpse_state = TPSE_STATE.lock().unwrap();
  let Some(tpse) = tpse_state.tpses.get_mut(&tpse_id) else { return 1 };
  let tpse_data = match replace(&mut tpse.status, TPSEStatus::Busy) {
    TPSEStatus::IdleInternal(_) => return 2,
    TPSEStatus::IdleExternal(wasm) => wasm,
    TPSEStatus::Busy => return 3 // tpse already running, can't get a handle to it
  };
  
  #[derive(Debug)]
  struct WasmTPSEProviderDynamicTPSEWrapper(WasmTPSEProvider);
  impl DynamicTPSE for WasmTPSEProviderDynamicTPSEWrapper {
    type Error = TPSEProviderError;
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>, Self::Error> {
      self.0.get(&RawTPSEKey(key.to_string())).await.map(|res_ok| res_ok.map(|opt_some| opt_some.into_owned()))
    }
    async fn set(&mut self, key: &str, value: Option<serde_json::Value>) -> Result<(), Self::Error> {
      self.0.set(&RawTPSEKey(key.to_string()), value).await
    }
  }
  
  crate::wasm::asynch::spawn(async move {
    let mut source = WasmTPSEProviderDynamicTPSEWrapper(tpse_data);
    let result = migrate(&mut source).await;
    
    let code = match TPSE_STATE.lock().unwrap().tpses.get_mut(&tpse_id) {
      None => { 2 } // tpse disappeared
      Some(tpse) => {
        tpse.status = TPSEStatus::IdleExternal(source.0);
        match result {
          Ok(versions) => {
            log::info!("migrate_extern_tpse success, performed migrations: {versions:?}");
            0
          }
          Err(err) => {
            log::error!("migrate_extern_tpse failed: {:?}", err);
            1
          },
        }
      }
    };
    unsafe { report_migration_done(tpse_id, code); }
  });
  0
}

/// Serializes a TPSE and returns a pointer to the buffer holding the serialized data.
/// Returns 0 if the TPSE handle is invalid, busy importing, or external
#[unsafe(no_mangle)]
pub extern "C" fn export_tpse(tpse_id: u32) -> *const u8 {
  let tpse_state = TPSE_STATE.lock().unwrap();
  let Some(tpse) = tpse_state.tpses.get(&tpse_id) else { return null() };
  let TPSEStatus::IdleInternal(tpse) = &tpse.status else { return null() };
  let encoded = serde_json::to_vec(&tpse).unwrap();
  drop(tpse_state);
  
  let mut buffer_state = BUFFER_STATE.lock().unwrap();
  let id = buffer_state.next_id();
  buffer_state.buffers.insert(id, encoded.into());
  buffer_state.buffers.get_mut(&id).unwrap().as_ptr()
}
  
struct WasmImportLogger { tpse_id: u32 }
impl ImportLogger for WasmImportLogger {
  fn log(&self, level: LogLevel, context: &[ImportContextEntry], msg: &dyn Display) {
    let info = serde_json::to_string(&json!({
      "level": level,
      "context": context,
      "message": msg.to_string()
    })).unwrap();
    let bytes = info.as_bytes();
    unsafe {
      import_log(self.tpse_id, bytes.as_ptr(), bytes.len());
    }
  }
}


