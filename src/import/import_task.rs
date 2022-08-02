use crate::import::{SkinType, SpecificImportType};

/// A collated form of a SpecificImportType suitable for performing the actual import step.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum ImportTask<'a> {
  AnimatedSkinFrames(SkinType, Vec<&'a [u8]>),
  SoundEffects(Vec<(String, &'a [u8])>),
  Basic(SpecificImportType, &'a [u8])
}