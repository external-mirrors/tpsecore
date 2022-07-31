use wasm_bindgen::JsValue;
use crate::{GLOBAL_STATE, import_file as import_file_internal};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn create_tpse() -> u32 {
  let mut state = GLOBAL_STATE.lock().unwrap();
  let id = state.id_incr;
  state.id_incr += 1;
  state.active_tpse_files.insert(id, Default::default());
  log::debug!("[TPSE {}] Creating TPSE", id);
  id
}

#[wasm_bindgen]
pub fn import_file(tpse: u32, import_type: JsValue, filename: String, bytes: &[u8]) -> Result<(), JsValue> {
  log::debug!("[TPSE {}] Importing file {} as {:?}", tpse, filename, import_type);
  let import_type = import_type.into_serde().map_err(|err| JsValue::from(err.to_string()))?;
  import_file_internal(tpse, import_type, &filename, bytes)
    .map_err(|err| JsValue::from_serde(&err).unwrap())
}

#[wasm_bindgen]
pub fn export_tpse(tpse: u32) -> Option<String> {
  log::debug!("[TPSE {}] Exporting", tpse);
  let state = GLOBAL_STATE.lock().unwrap();
  match state.active_tpse_files.get(&tpse) {
    Some(tpse) => Some(serde_json::to_string(tpse).unwrap()),
    None => None
  }
}

#[wasm_bindgen]
pub fn drop_tpse(tpse: u32) -> bool {
  log::debug!("[TPSE {}] Dropped!", tpse);
  let mut state = GLOBAL_STATE.lock().unwrap();
  state.active_tpse_files.remove(&tpse).is_some()
}