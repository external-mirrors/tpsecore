use std::fmt::{Display, Formatter};
use crate::import::{BackgroundType, OtherSkinType, SkinType};

/// A distilled copy of an import type that's been rendered
/// to a more specific form than the public API.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, serde_with::SerializeDisplay)]
pub enum SpecificImportType {
  Zip,
  TPSE,
  Skin(SkinType),
  OtherSkin(OtherSkinType),
  SoundEffects,
  Background(BackgroundType),
  Music
}
impl Display for SpecificImportType {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      SpecificImportType::Zip => write!(f, "zip"),
      SpecificImportType::TPSE => write!(f, "tpse"),
      SpecificImportType::Skin(subtype) => write!(f, "{} skin", subtype),
      SpecificImportType::OtherSkin(subtype) => write!(f, "{} skin", subtype),
      SpecificImportType::SoundEffects => write!(f, "sound effects"),
      SpecificImportType::Background(subtype) => write!(f, "{} background", subtype),
      SpecificImportType::Music => write!(f, "music")
    }
  }
}