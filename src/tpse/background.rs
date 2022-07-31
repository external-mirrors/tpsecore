/// A background metadata object. Does not contain the background itself.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Background {
  /// The unique ID of the background, used to name the actual file
  /// accessible at the `background-${id}` key in storage.
  pub id: String,
  /// The background type, changed by using different importers
  #[serde(rename = "type")]
  pub background_type: BackgroundType,
  /// The name of the file, only used to show it to the user.
  pub filename: String
}

/// The background type, changed by using different importers
/// Note: the animated background type is stored separately, at the top level of the TPSE.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundType {
  /// Regular image backgrounds injected directly into the game
  Image,
  /// Special music-graph-only backgrounds that can use video files
  Video
}