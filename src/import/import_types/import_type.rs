use crate::import::{BackgroundType, OtherSkinType, SkinType};

/// An ImportType is metadata describing how a single file should be imported
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all="snake_case")]
pub enum ImportType {
  /// An import type will be decided automatically.
  /// This is the only way to import a zip or tpse file
  Automatic,
  Skin {
    #[serde(flatten)]
    subtype: SkinType
  },
  OtherSkin {
    #[serde(flatten)]
    subtype: OtherSkinType
  },
  SoundEffects,
  Background {
    #[serde(flatten)]
    subtype: BackgroundType
  },
  Music
}
