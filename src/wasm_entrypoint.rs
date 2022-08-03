use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::Mutex;
use lazy_static::lazy_static;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;
use crate::import::{DefaultAssetProvider, import, ImportOptions, ImportType};
use crate::tpse::TPSE;

#[derive(Default)]
struct State {
  active_tpse_files: HashMap<u32, TPSE>,
  provider: DefaultAssetProvider,
  id_incr: u32
}

lazy_static! {
    static ref GLOBAL_STATE: Mutex<State> = {
        #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Trace);
        }
        #[cfg(not(target_arch = "wasm32"))] {
            simple_logger::SimpleLogger::new().env().init().unwrap();
        }
        Default::default()
    };
}

fn with_tpse<T>(tpse: u32, handler: impl FnOnce(&mut TPSE, &mut DefaultAssetProvider) -> T) -> Result<T, JsValue> {
  let mut state = GLOBAL_STATE.lock().unwrap();
  let mut state = state.deref_mut();
  match &mut state.active_tpse_files.get_mut(&tpse) {
    None => Err(JsValue::from("invalid TPSE handle")),
    Some(tpse) => Ok((handler)(tpse, &mut state.provider))
  }
}

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
  let import_type = import_type.into_serde().map_err(|err| JsValue::from(err.to_string()))?;
  log::debug!("[TPSE {}] Importing file {} as {:?}", tpse, filename, import_type);
  with_tpse(tpse, |tpse, provider| -> Result<(), JsValue> {
    let options = ImportOptions {
      asset_source: provider,
      depth_limit: 5
    };
    let new_tpse = import(vec![(import_type, &filename, bytes)], options).map_err(|err| {
      JsValue::from(err.to_string())
    })?;
    tpse.merge(new_tpse);
    Ok(())
  })??;
  Ok(())
}

#[wasm_bindgen]
pub fn export_tpse(tpse: u32) -> Result<String, JsValue> {
  log::debug!("[TPSE {}] Exporting", tpse);
  with_tpse(tpse, |tpse, _| serde_json::to_string(tpse).unwrap())
}

#[wasm_bindgen]
pub fn drop_tpse(tpse: u32) -> Result<(), JsValue> {
  log::debug!("[TPSE {}] Dropping", tpse);
  let mut state = GLOBAL_STATE.lock().unwrap();
  if !state.active_tpse_files.remove(&tpse).is_some() {
    Err(JsValue::from("invalid TPSE handle"))
  } else {
    Ok(())
  }
}

#[wasm_bindgen]
pub fn provide_asset(asset: JsValue, data: &[u8]) -> Result<(), JsValue> {
  let mut state = GLOBAL_STATE.lock().unwrap();
  let asset = asset.into_serde().map_err(|err| JsValue::from(err.to_string()))?;
  log::debug!("Provided asset {}: {} bytes", asset, data.len());
  state.provider.preload(asset, Vec::from(data));
  Ok(())
}