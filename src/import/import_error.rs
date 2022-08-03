use std::borrow::Cow;
use std::collections::HashSet;
use image::ImageError;
use wasm_bindgen::JsValue;
use crate::import::asset_provider::Asset;
use crate::import::skin_splicer::LoadError;
use crate::import::SkinType;

pub struct ImportError {
  pub context: Vec<String>,
  pub error: ImportErrorType
}

impl ImportError {
  pub fn with_context(mut self, ctx: String) -> Self {
    self.context.push(ctx);
    self
  }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportErrorType {
  #[error("unknown file type")]
  UnknownFileType,
  #[error("invalid TPSE: {0}")]
  InvalidTPSE(String),
  #[error("files were nested too deeply")]
  TooMuchNesting,
  #[error("failed to load files: {0}")]
  LoadError(#[from] LoadError),
  #[error("animated {0} skin results were ambiguous: found multiple possible formats: {1:?}")]
  AmbiguousAnimatedSkinResults(Cow<'static, str>, HashSet<SkinType>),
  #[error("failed to fetch asset for {0:?}: {1:?}")]
  AssetFetchFailed(Asset, String),
  #[error("the {0} asset was not preloaded and the given AssetProvider cannot fetch it")]
  AssetNotPreloaded(Asset)
}
impl From<ImageError> for ImportErrorType {
  fn from(err: ImageError) -> Self {
    Self::LoadError(LoadError::ImageError(err))
  }
}