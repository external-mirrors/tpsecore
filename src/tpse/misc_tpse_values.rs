use std::collections::HashMap;

use crate::tpse::File;

/// A map of sprite name to (offset_milliseconds, duration_milliseconds)
pub type CustomSoundAtlas = HashMap<String, (f64, f64)>;

/// Metadata for animated mino skins
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct AnimMeta {
  /// The number of frames the animation lasts for
  pub frames: u32,
  /// The delay between frames, in game frames (e.g. 30 = 2fps)
  pub delay: u32
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimatedBackground {
  pub id: String,
  pub filename: String
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MiscTPSEValue {
  File(File),
  Other(serde_json::Value)
}