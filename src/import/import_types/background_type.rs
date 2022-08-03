/// The type of the background to import as.
/// Distinct from but similar to `crate::tpse::Background`, which is the actual background type
/// inside a TPSE file, whereas this struct is for import configuration.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum BackgroundType {
  Video,
  Image
}