use std::path::Path;
use crate::import::{SkinType, SpecificImportType};
use crate::tpse::File;

/// A collated form of a SpecificImportType suitable for performing the actual import step.
#[derive(Debug, Clone)]
pub enum ImportTask {
  AnimatedSkinFrames(SkinType, Vec<AnimatedSkinFrame>),
  SoundEffects(Vec<SoundEffect>),
  Basic {
    import_type: SpecificImportType,
    filename: String,
    file: File
  }
}

#[derive(Debug, Clone)]
pub struct AnimatedSkinFrame {
  pub filename: String,
  pub file: File
}

#[derive(Debug, Clone)]
pub struct SoundEffect {
  /// The name of the sound effect, usually the file name sans extension
  pub name: String,
  pub filename: String,
  pub file: File,
}

impl SoundEffect {
  pub fn extension(&self) -> Option<String> {
    Path::new(&self.filename).extension().map(|el| el.to_string_lossy().to_string())
  }
}