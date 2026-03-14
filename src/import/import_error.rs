use std::borrow::Cow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::Utf8Error;
use itertools::Itertools;

use crate::accel::traits::{AssetProvider, AudioHandle, TPSEAccelerator, TextureHandle};
use crate::import::{ImportContextEntry, SkinType};

/// An error tracking both the actual error and the import context in which it occurred
#[derive(Debug)]
pub struct ImportError<T: TPSEAccelerator> {
  pub context: Vec<ImportContextEntry>,
  pub error: ImportErrorType<T>
}
impl<T: TPSEAccelerator> Error for ImportError<T> {}
impl<T: TPSEAccelerator> Display for ImportError<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "import error at {}: {}", self.context.iter().format(" "), self.error)
  }
}

impl<T: TPSEAccelerator> ImportError<T> {
  /// Creates an import error with no import context
  pub fn with_no_context(error: ImportErrorType<T>) -> Self {
    Self { error, context: vec![] }
  }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportErrorType<T: TPSEAccelerator> {
  #[error("unknown file type")]
  UnknownFileType,
  #[error("invalid TPSE: {0}")]
  InvalidTPSE(String),
  #[error("files were nested too deeply")]
  TooMuchNesting,
  #[error("failed to load files: {0}")]
  LoadError(#[from] MediaLoadError<T>),
  #[error("animated {0} skin results were ambiguous: found multiple possible formats: {1:?}")]
  AmbiguousAnimatedSkinResults(Cow<'static, str>, HashSet<SkinType>),
  #[error("failed to load TETR.IO asset: {0}")]
  AssetFetchFailed(<T::Asset as AssetProvider>::Error),
  #[error("base game asset metadata parse failure: {0}")]
  AssetParseFailure(#[from] TetrioAssetMetadataParseFailure),
  #[error("failed to decode base game sound effects buffer")]
  AssetSoundEffectsDecode(<T::Audio as AudioHandle>::Error),
  #[error("rendering failure: {0}")]
  RenderFailure(#[from] RenderFailure),
  #[error("encoding image failed")]
  EncodeFailed
}

/// An error indicating failure to parse base game asset metadata
#[derive(Debug, thiserror::Error)]
pub enum TetrioAssetMetadataParseFailure {
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
  SoundEffectsAtlasNameTooLong { sprite: usize, length: u32 },
}

/// An error indicating failure to parse a media file
#[derive(Debug, thiserror::Error)]
pub enum MediaLoadError<T: TPSEAccelerator> {
  #[error(transparent)]
  TextureError(<T::Texture as TextureHandle>::Error),
  #[error(transparent)]
  AudioError(<T::Audio as AudioHandle>::Error),
  #[error("failed to read zip file: {0}")]
  Zip(#[from] zip::result::ZipError),
  /// Currently only used by extra software decoders, which needs to be cleaned up so this can be removed
  #[error("{0}")]
  Other(String)
}

#[derive(Debug, thiserror::Error)]
pub enum RenderFailure {
  #[error("tpse has no sound effect configuration")]
  NoSoundEffectsConfiguration,
  #[error("tpse has no such sound effect {0}")]
  NoSoundSoundEffect(String)
}