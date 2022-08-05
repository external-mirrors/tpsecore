use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use lazy_static::lazy_static;
use mime::Mime;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;
use crate::import::{Asset, AssetProvider, DefaultAssetProvider, import, ImportErrorType, ImportOptions, RenderFailure};
use crate::import::decode_helper::{decode, TetrioAtlasDecoder};
use crate::import::tetriojs::custom_sound_atlas;
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

fn with_tpse<T>(tpse: u32, handler: impl FnOnce(&mut TPSE, &mut DefaultAssetProvider) -> Result<T, ImportErrorType>) -> Result<T, JsValue> {
  let mut state = GLOBAL_STATE.lock().unwrap();
  let state = state.deref_mut();
  state.active_tpse_files.get_mut(&tpse)
    .ok_or_else(|| JsValue::from("invalid TPSE handle"))
    .and_then(|tpse| (handler)(tpse, &mut state.provider).map_err(stringify_error))
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
  let import_type = import_type.into_serde().map_err(stringify_error)?;
  log::debug!("[TPSE {}] Importing file {} as {:?}", tpse, filename, import_type);
  with_tpse(tpse, |tpse, provider| {
    let options = ImportOptions {
      asset_source: provider,
      depth_limit: 5
    };
    let new_tpse = import(vec![(import_type, &filename, bytes)], options)?;
    tpse.merge(new_tpse);
    Ok(())
  })?;
  Ok(())
}

#[wasm_bindgen]
pub fn export_tpse(tpse: u32) -> Result<String, JsValue> {
  log::debug!("[TPSE {}] Exporting", tpse);
  with_tpse(tpse, |tpse, _| Ok(serde_json::to_string(tpse).unwrap()))
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
  let asset = asset.into_serde().map_err(stringify_error)?;
  log::debug!("Provided asset {}: {} bytes", asset, data.len());
  state.provider.preload(asset, Vec::from(data));
  Ok(())
}

#[wasm_bindgen]
pub fn get_atlas(tpse: u32) -> Result<JsValue, JsValue> {
  log::debug!("[TPSE {}] Get atlas", tpse);
  with_tpse(tpse, |tpse, _| Ok(JsValue::from_serde(&tpse.custom_sound_atlas).unwrap()))
}

#[wasm_bindgen]
pub fn get_default_atlas() -> Result<JsValue, JsValue> {
  log::debug!("[TPSE] Get default atlas");
  let state = GLOBAL_STATE.lock().unwrap();
  let tetriojs = state.provider.provide(Asset::TetrioJS).map_err(stringify_error)?;
  let atlas = custom_sound_atlas(tetriojs).map_err(stringify_error)?;
  Ok(JsValue::from_serde(&atlas).unwrap())
}

#[wasm_bindgen]
pub fn render_sound_effect(tpse: u32, sound: &str) -> Result<Option<Vec<f32>>, JsValue> {
  log::debug!("[TPSE {}] Render sound effect", tpse);
  with_tpse(tpse, |tpse, opts| {
    let atlas = match &tpse.custom_sound_atlas {
      None => return Err(RenderFailure::NoSoundEffectsConfiguration.into()),
      Some(atlas) => atlas
    };
    let ogg = match &tpse.custom_sounds {
      None => return Err(RenderFailure::NoSoundEffectsConfiguration.into()),
      Some(ogg) => ogg
    };
    let ext = mime_guess::get_mime_extensions_str(&ogg.mime)
      .and_then(|mime| mime.first())
      .map(Deref::deref);
    let decoder = TetrioAtlasDecoder::decode(&ogg.binary, ext)?;
    Ok(decoder.lookup(atlas, sound).map(|slice| slice.to_vec()))
  })
}

#[wasm_bindgen]
pub fn render_default_sound_effect(sound: &str) -> Result<Vec<f32>, JsValue> {
  log::debug!("[TPSE] Render default sound effect");
  let state = GLOBAL_STATE.lock().unwrap();
  let tetrio_js = state.provider.provide(Asset::TetrioJS)?;
  let tetrio_ogg = state.provider.provide(Asset::TetrioOGG)?;
  let atlas = custom_sound_atlas(tetrio_js).map_err(|err| ImportErrorType::AssetParseFailure(err))?;
  let decoder = TetrioAtlasDecoder::decode(tetrio_ogg, Some("ogg"))?;
  let samples = decoder.lookup(&atlas, sound)
    .ok_or_else(|| RenderFailure::NoSoundSoundEffect(sound.to_string()))
    .map_err(stringify_error)?
    .to_vec();
  Ok(samples)
}

impl From<ImportErrorType> for JsValue {
  fn from(err: ImportErrorType) -> Self {
    stringify_error(err)
  }
}

fn stringify_error(t: impl Display) -> JsValue {
  JsValue::from(t.to_string())
}