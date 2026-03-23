use std::collections::HashMap;

/// A map of sprite name to (offset_milliseconds, duration_milliseconds)
pub type CustomSoundAtlas = HashMap<String, (f64, f64)>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Song {
  pub id: String,
  pub filename: String,
  #[serde(rename = "override")]
  pub song_override: Option<String>,
  pub metadata: SongMetadata
}

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SongMetadata {
  pub name: String,
  pub jpname: String,
  pub artist: String,
  pub jpartist: String,
  pub genre: SongGenre,
  pub source: String,
  #[serde(rename = "loop")]
  pub song_loop: bool,
  #[serde(rename = "loopStart")]
  pub loop_start: u32,
  #[serde(rename = "loopLength")]
  pub loop_length: u32,
  pub hidden: bool,
  #[serde(rename = "normalizeDb")]
  pub normalize_db: f64
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SongGenre {
  Interface,
  Disabled,
  Override,
  Calm,
  Battle
}
impl Default for SongGenre {
  fn default() -> Self {
    Self::Calm
  }
}

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

