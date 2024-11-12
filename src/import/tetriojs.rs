use regex::Regex;
use crate::import::import_error::AssetParseFailure;
use crate::import::import_error::AssetParseFailure::*;
use crate::import::ImportErrorType;
use crate::tpse::CustomSoundAtlas;

pub fn custom_sound_atlas(tetriojs: &[u8]) -> Result<CustomSoundAtlas, AssetParseFailure> {
  let tetriojs = std::str::from_utf8(tetriojs).map_err(|_| UTF8Error)?;

  let extract_atlas = Regex::new(r"(\{[^{}]*boardappear:\[[\d.e+]+,[\d.e+]+\][^{}]*})").unwrap();
  let atlas = extract_atlas.captures(tetriojs).ok_or(SoundEffectsAtlasRegex)?.get(1).unwrap().as_str();

  let quote_keys = Regex::new(r#"(\s*?\{\s*?|\s*?,\s*?)(?:['"])?([a-zA-Z0-9_]+)(?:['"])?:"#).unwrap();
  let fixed_up_atlas = quote_keys.replace_all(atlas, r#"${1}"${2}":"#);

  Ok(serde_json::from_str(fixed_up_atlas.as_ref()).map_err(|_| SoundEffectsAtlasParse)?)
}