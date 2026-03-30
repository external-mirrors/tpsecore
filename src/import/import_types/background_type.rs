use std::fmt::{Display, Formatter};

/// The type of the background to import as.
/// Distinct from but similar to [`crate::tpse::Background`], which is the actual background type
/// inside a TPSE file, whereas this struct is for import configuration.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum BackgroundType {
  Video,
  Image
}
impl Display for BackgroundType {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      BackgroundType::Video => write!(f, "video"),
      BackgroundType::Image => write!(f, "image")
    }
  }
}
impl From<crate::tpse::BackgroundType> for BackgroundType {
  fn from(bg: crate::tpse::BackgroundType) -> Self {
    match bg {
      crate::tpse::BackgroundType::Image => Self::Image,
      crate::tpse::BackgroundType::Video => Self::Video
    }
  }
}