use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;
use crate::accel::traits::{AssetProvider, TPSEAccelerator};
use crate::import::{Asset, ImportContextEntry, ImportError, ImportErrorType, ImportErrorWrapHelper, TypeStage1, err};
use crate::log::{ImportLogger, LogLevel};

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
  pub guessed_files: HashMap<PathBuf, TypeStage1>
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

  pub fn log(&self, level: LogLevel, message: impl Display) {
    if let Some(logger) = self.logger {
      logger.log(level, &self.context, &message);
    }
  }
  
  pub async fn provide_asset(&mut self, asset: Asset) -> Result<Arc<[u8]>, ImportError<T>> {
    if !self.asset_cache.contains_key(&asset) {
      let guard = self.enter_context(ImportContextEntry::ProvideAsset { asset });
      guard.log(LogLevel::Status, format_args!("Gathering asset {asset}"));
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
    self.log(LogLevel::Debug, format_args!("Entering context {:?}", context));
    self.context.push(context);
    ContextGuard { context: self }
  }
}

pub struct ContextGuard<'ctx, 'ctx_deps, T: TPSEAccelerator> {
  context: &'ctx mut ImportContext<'ctx_deps, T>
}
impl<T: TPSEAccelerator> Drop for ContextGuard<'_, '_, T> {
  fn drop(&mut self) {
    let entry = self.context.context.pop().expect("context guard lifecycle should be tied to context stack lifecycle");
    self.context.log(LogLevel::Debug, format_args!("Leaving context {:?}", entry));
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