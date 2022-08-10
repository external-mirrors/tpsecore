use std::borrow::Cow;
use std::collections::HashSet;
use image::ImageError;

use crate::import::asset_provider::Asset;
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
  AssetNotPreloaded(Asset),
  #[error("asset parse failure: {0}")]
  AssetParseFailure(#[from] AssetParseFailure),
  #[error("rendering failure: {0}")]
  RenderFailure(#[from] RenderFailure)
}

/// An error indicating failure to parse base game assets
#[derive(Debug, thiserror::Error)]
pub enum AssetParseFailure {
  #[error("Tried to parse non-UTF8 data as UTF8")]
  UTF8Error,
  #[error("failed to extract sound effects regex")]
  SoundEffectsAtlasRegex,
  #[error("failed to parse sound effects atlas")]
  SoundEffectsAtlasParse,
}

/// An error indicating failure to parse a media file
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
  #[error("failed to load image: {0}")]
  ImageError(#[from] image::ImageError),
  #[error("the image decoder we're using is broken as hell and panicked")]
  ImageLoadPanic,
  #[error("failed to decode audio: {0}")]
  SymphoniaError(#[from] symphonia::core::errors::Error),
  #[error("failed to decode audio: no supported audio track")]
  NoSupportedAudioTrack,
  #[error("failed to read zip file: {0}")]
  Zip(#[from] zip::result::ZipError)
}

#[derive(Debug, thiserror::Error)]
pub enum RenderFailure {
  #[error("tpse has no sound effect configuration")]
  NoSoundEffectsConfiguration,
  #[error("tpse has no such sound effect {0}")]
  NoSoundSoundEffect(String)
}

impl From<ImageError> for ImportErrorType {
  fn from(err: ImageError) -> Self {
    Self::LoadError(LoadError::ImageError(err))
  }
}