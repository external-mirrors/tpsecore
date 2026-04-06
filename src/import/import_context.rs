use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use crate::accel::traits::{AssetProvider, TPSEAccelerator};
use crate::import::packjson::PackMetadata;
use crate::import::{Asset, ImportContextEntry, ImportError, ImportErrorType, ImportErrorWrapHelper, TypeStage1, err};
use crate::log::{ImportLogger, LogLevel};
use crate::util::sound_effects_sort_key;

/// Stores metadata and context associated with an import process, tracking the stack location
/// (e.g. nested zip files) and base game asset provider.
pub struct ImportContext<'ctx_deps, T: TPSEAccelerator> {
  pub(in crate) decider: &'ctx_deps T::Decider,
  /// The asset provider providing `Asset`s for the importer
  asset_source: &'ctx_deps T::Asset,
  asset_cache: HashMap<Asset, Arc<[u8]>>,
  /// The maximum depth the context stack is allowed to reach before bailing
  depth_limit: usize,
  /// A stack of context describing the current item the importer is working on
  context: Vec<ImportContextEntry>,
  /// An outlet for diagnostic/progress messages
  logger: Option<&'ctx_deps (dyn ImportLogger + Send + Sync)>,
  pub flags: ImportFlags,
}

/// Stores 'flags' raised during import which are used to supplement logs
#[derive(Default, serde::Serialize)]
pub struct ImportFlags {
  pub metadata: Option<PackMetadata>,
  #[serde(flatten)]
  pub modified_sound_effects: ModifiedSoundEffects,
  /// The initial filetype guesses made during stage 1 of the import process
  pub guessed_files: HashMap<PathBuf, TypeStage1>
}

#[derive(Default, serde::Serialize)]
#[serde(tag = "modified_sound_effect_class", rename_all = "lowercase")]
pub enum ModifiedSoundEffects {
  #[default]
  None,
  Some { modified_sound_effects: Vec<String> },
  All,
}
impl ModifiedSoundEffects {
  pub fn add(&mut self, sound: impl Into<String>) {
    match self {
      Self::None => {
        *self = Self::Some {
          modified_sound_effects: vec![sound.into()]
        };
      },
      Self::Some { modified_sound_effects } => {
        modified_sound_effects.push(sound.into());
      },
      Self::All => {}
    }
  }
  pub fn sort_and_dedup(&mut self) {
    if let Self::Some { modified_sound_effects } = self {
      modified_sound_effects.sort_by(|a, b| {
        sound_effects_sort_key(a).cmp(&sound_effects_sort_key(b))
      });
      modified_sound_effects.dedup();
    }
  }
}

impl<'ctx_deps, T: TPSEAccelerator> ImportContext<'ctx_deps, T> {
  pub fn new(asset_source: &'ctx_deps T::Asset, decider: &'ctx_deps T::Decider) -> ImportContext<'ctx_deps, T> {
    Self {
      depth_limit: 15,
      asset_cache: Default::default(),
      asset_source,
      decider,
      context: Default::default(),
      flags: Default::default(),
      logger: None
    }
  }
  
  pub fn with_depth_limit(mut self, limit: usize) -> Self {
    self.depth_limit = limit;
    self
  }

  pub fn with_logger(self, logger: &'ctx_deps (dyn ImportLogger + Send + Sync)) -> Self {
    Self { logger: Some(logger), ..self }
  }
  
  pub fn log_in_context
    (&self, level: LogLevel, context: &[ImportContextEntry], message: impl Display)
    -> impl Future<Output = ()> + Send + Sync + 'static
  {
    if let Some(logger) = self.logger {
      logger.log(level, context, &message);
    }
    LogFuture::default()
  }

  pub fn log(&self, level: LogLevel, message: impl Display) -> impl Future<Output = ()> + Send + Sync + 'static {
    if let Some(logger) = self.logger {
      logger.log(level, &self.context, &message);
    }
    LogFuture::default()
  }
  
  pub async fn provide_asset(&mut self, asset: Asset) -> Result<Arc<[u8]>, ImportError<T>> {
    if !self.asset_cache.contains_key(&asset) {
      let guard = self.enter_context(ImportContextEntry::ProvideAsset { asset });
      {guard.log(LogLevel::Status, format_args!("Gathering asset {asset}"))}.await;
      let fetched = guard.asset_source.provide(asset).await.wrap(err!(guard, assetfetchfail))?;
      drop(guard);
      self.asset_cache.insert(asset, fetched);
    }
    Ok(self.asset_cache.get(&asset).unwrap().clone())
  }

  /// Wraps an ImportErrorType with this `ImportContext`'s context
  pub fn wrap_error(&self, error: ImportErrorType<T>) -> ImportError<T> {
    ImportError {
      context: self.context.clone(),
      error
    }
  }

  /// Checks if the context stack is at or beyond its depth limit
  pub fn is_too_deep(&self) -> bool {
    self.context.len() >= self.depth_limit
  }

  /// Enters a new context, keeping it on the context stack until the returned guard is dropped.
  /// The context can be accessed mutably again through the guard.
  pub fn enter_context<'ctx>(&'ctx mut self, context: ImportContextEntry) -> ContextGuard<'ctx, 'ctx_deps, T> {
    // self.log(LogLevel::Debug, format_args!("Entering context {:?}", context));
    self.context.push(context);
    ContextGuard { context: self }
  }
}

pub struct ContextGuard<'ctx, 'ctx_deps, T: TPSEAccelerator> {
  context: &'ctx mut ImportContext<'ctx_deps, T>
}
impl<T: TPSEAccelerator> Drop for ContextGuard<'_, '_, T> {
  fn drop(&mut self) {
    let _entry = self.context.context.pop().expect("context guard lifecycle should be tied to context stack lifecycle");
    // self.context.log(LogLevel::Debug, format_args!("Leaving context {:?}", entry));
  }
}
impl<'ctx_deps, T: TPSEAccelerator> Deref for ContextGuard<'_, 'ctx_deps, T> {
  type Target = ImportContext<'ctx_deps, T>;
  fn deref(&self) -> &Self::Target {
    self.context
  }
}
impl<T: TPSEAccelerator> DerefMut for ContextGuard<'_, '_, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.context
  }
}


/// A future that, on wasm, sets the task to pending once, wakes itself immediately, then resolves on the next poll.
/// This gives control back to the javascript event loop - for logging, this ensures the DOM gets a chance to rerender.
/// On other targets, resolves immediately.
#[derive(Default)]
struct LogFuture { done: bool }

impl Future for LogFuture {
  type Output = ();
  #[allow(unused)] // silence cfg-induced warnings on non-wasm targets
  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    #[cfg(not(target_arch = "wasm32"))] { return Poll::Ready(()) }
    if self.done { return Poll::Ready(()) }
    cx.waker().wake_by_ref();
    self.done = true;
    Poll::Pending
  }
}