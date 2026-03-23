use std::borrow::Cow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::Utf8Error;
use itertools::Itertools;
use serde_json::Value;

use crate::accel::traits::{AssetProvider, AudioHandle, TPSEAccelerator, TextureHandle};
use crate::import::{ImportContextEntry, SkinType};
use crate::tpse::MigrationError;

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
  InvalidTPSE(TPSELoadError),
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
  #[error("failed to decode base game sound effects buffer: {0}")]
  AssetSoundEffectsDecode(<T::Audio as AudioHandle>::Error),
  #[error("rendering failure: {0}")]
  RenderFailure(#[from] RenderFailure),
  #[error("encoding image failed")]
  TextureEncodeFailed(<T::Texture as TextureHandle>::Error),
  #[error("encoding audio failed: {0}")]
  AudioEncodeFailed(<T::Audio as AudioHandle>::Error),
  #[error("storage error: {0}")]
  StorageError(#[from] StorageError)
}

#[derive(Debug, thiserror::Error)]
pub enum TPSELoadError {
  #[error("invalid json: {0}")]
  BadJson(serde_json::Error),
  #[error("error during migration: {0}")]
  MigrationFailed(MigrationError<Value>),
  #[error("parsing failure after migration: {0}")]
  ParseFailed(serde_json::Error)
}

#[derive(Debug, thiserror::Error)]
#[error("failed to {method} key {key} on {side}: {error}")]
pub struct StorageError {
  pub method: StorageMethod,
  pub side: StorageSide,
  pub key: String,
  pub error: TPSEProviderError
}

#[derive(Debug, thiserror::Error)]
pub enum TPSEProviderError {
  #[error("tpse key (de)serialization error: {0}")]
  SerializationError(serde_json::Error)
}

#[derive(Debug)]
pub enum StorageSide { Base, Source }
impl Display for StorageSide {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Base => write!(f, "base"),
      Self::Source => write!(f, "source")
    }
  }
}

#[derive(Debug)]
pub enum StorageMethod { Get, Set }
impl Display for StorageMethod {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Get => write!(f, "get"),
      Self::Set => write!(f, "set")
    }
  }
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

/// Accompanying macro for building [ImportErrorWrapHelper] shorthands.
/// Usage: `err!(context, ...transformations)`
macro_rules! err {
  ($ctx:expr $(, $wrapper:tt)*) => {
    (&$ctx, |ctx, value| {
      match value {
        Ok(value) => Ok(value),
        Err(error) => {
          $( let error = err!(@transform $wrapper error); )*
          Err(ctx.wrap_error(error.into()))
        }
      }
    })
  };
  (@transform (with $wrapper:expr) $expr:expr) => {
    ImportErrorType::InvalidTPSE($wrapper($expr.into()).into())
  };
  (@transform assetfetchfail $expr:expr) => {
    ImportErrorType::AssetFetchFailed($expr.into())
  };
  (@transform rsd_decode $expr:expr) => {
    ImportErrorType::AssetSoundEffectsDecode($expr.into())
  };
  (@transform tex_encode $expr:expr) => {
    ImportErrorType::TextureEncodeFailed($expr.into())
  };
  (@transform audio_encode $expr:expr) => {
    ImportErrorType::AudioEncodeFailed($expr.into())
  };
  (@transform tex $expr:expr) => {
    MediaLoadError::TextureError($expr.into())
  };
  (@transform audio $expr:expr) => {
    MediaLoadError::AudioError($expr.into())
  };
  (@transform zip $expr:expr) => {
    MediaLoadError::Zip($expr.into())
  };
}
pub(crate) use err;

/// A shorthand for wrapping errors in the appropriate type, assisted by [err]
pub trait ImportErrorWrapHelper {
  /// Expected usage: `.wrap(err!(ctx, ...transformations))`
  fn wrap<Wrapped, Ctx>(self, wrap: (Ctx, fn(Ctx, Self) -> Wrapped)) -> Wrapped;
}
impl<S> ImportErrorWrapHelper for S {
  fn wrap<Wrapped, Ctx>(self, (ctx, wrapper): (Ctx, fn(Ctx, Self) -> Wrapped)) -> Wrapped {
    wrapper(ctx, self)
  }
}