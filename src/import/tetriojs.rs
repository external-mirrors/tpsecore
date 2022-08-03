use regex::Regex;
use crate::import::import_error::AssetParseFailure;
use crate::import::import_error::AssetParseFailure::*;
use crate::import::ImportErrorType;
use crate::tpse::CustomSoundAtlas;

pub fn custom_sound_atlas(tetriojs: &[u8]) -> Result<CustomSoundAtlas, AssetParseFailure> {
  let tetriojs = std::str::from_utf8(tetriojs).map_err(|_| UTF8Error)?;
  let regex = Regex::new(r"TETRIO_SE_SHEET\s*=\s*(\{[^}]+})").unwrap();
  let captures = regex.captures(tetriojs).ok_or(SoundEffectsAtlasRegex)?;
  Ok(serde_json::from_str(captures.get(1).unwrap().as_str()).map_err(|_| SoundEffectsAtlasParse)?)
}