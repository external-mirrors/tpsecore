use std::borrow::Cow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::Utf8Error;
use itertools::Itertools;

use crate::import::asset_provider::Asset;
use crate::import::{ImportContextEntry, SkinType};

/// An error tracking both the actual error and the import context in which it occurred
#[derive(Debug)]
pub struct ImportError {
  pub context: Vec<ImportContextEntry>,
  pub error: ImportErrorType
}
impl Error for ImportError {}
impl Display for ImportError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "import error at {}: {}", self.context.iter().format(" "), self.error)
  }
}

impl ImportError {
  /// Creates an import error with no import context
  pub fn with_no_context(error: ImportErrorType) -> Self {
    Self { error, context: vec![] }
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
  RenderFailure(#[from] RenderFailure),
  #[error("encoding image failed")]
  EncodeFailed
}

/// An error indicating failure to parse base game assets
#[derive(Debug, thiserror::Error)]
pub enum AssetParseFailure {
  #[error("Tried to parse non-UTF8 data as UTF8")]
  UTF8Error,
  #[error("regex failed to extract sound effects atlas")]
  SoundEffectsAtlasRegex,
  #[error("failed to parse sound effects atlas")]
  SoundEffectsAtlasParse,
  #[error("unexpected EOF while parsing sound effects atlas")]
  SoundEffectsAtlasEOF,
  #[error("expected EOF while parsing sound effects atlas at position {position}, but buffer length is {length}")]
  SoundEffectsAtlasExpectedEOF { position: usize, length: usize },
  #[error("found invalid UTF-8 in name of sprite {sprite}: {error}")]
  SoundEffectsAtlasSpriteNameUTF8Error { sprite: usize, error: Utf8Error },
  #[error("name of sprite {sprite} beyond sane limits: {length} bytes")]
  SoundEffectsAtlasNameTooLong { sprite: usize, length: u32 }
}

/// An error indicating failure to parse a media file
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
  /// For errors that occured in javascript
  #[error("failed to load asset: {0}")]
  WasmAcceleratorError(String),
  /// For errors that occured in other accelerators
  #[error("failed to load asset: {0}")]
  ErasedAcceleratorError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
  #[error("failed to load image: image decoder implementation panicked")]
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