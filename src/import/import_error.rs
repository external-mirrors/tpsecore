use std::borrow::Cow;
use std::collections::HashSet;
use crate::import::skin_splicer::LoadError;
use crate::import::SkinType;

#[derive(Debug, serde_with::SerializeDisplay, thiserror::Error)]
// #[serde(tag = "error")]
pub enum ImportError {
  #[error("invalid TPSE handle")]
  InvalidTPSEHandle,
  #[error("unknown file type")]
  UnknownFileType,
  #[error("invalid TPSE: {0}")]
  InvalidTPSE(String),
  #[error("files were nested too deeply")]
  TooMuchNesting,
  #[error("failed to load image")]
  ImageError(#[from] LoadError),
  #[error("animated {0} skin results were ambiguous: found multiple possible formats: {1:?}")]
  AmbiguousAnimatedSkinResults(Cow<'static, str>, HashSet<SkinType>)
}