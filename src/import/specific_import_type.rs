use crate::import::{OtherSkinType, SkinType};

/// A distilled copy of an import type that's been rendered
/// to a more specific form than the public API.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum SpecificImportType {
  Zip,
  TPSE,
  Skin(SkinType),
  OtherSkin(OtherSkinType),
  SoundEffects,
  Background,
  Music
}