use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use crate::accel::traits::TPSEAccelerator;
use crate::import::{ImportContextEntry, ImportError, ImportErrorType, SpecificImportType};
use crate::log::{ImportLogger, LogLevel};

/// Stores metadata and context associated with an import process, tracking the stack location
/// (e.g. nested zip files) and base game asset provider.
pub struct ImportContext<'ctx_deps, T: TPSEAccelerator> {
  /// The asset provider providing `Asset`s for the importer
  pub asset_source: &'ctx_deps T::Asset,
  /// The maximum depth the context stack is allowed to reach before bailing
  pub depth_limit: u8,
  /// A stack of context describing the current item the importer is working on
  pub context: Vec<ImportContextEntry>,
  pub flags: ImportFlags,
  /// An outlet for diagnostic/progress messages
  pub logger: Option<&'ctx_deps (dyn ImportLogger + Send + Sync)>
}

/// Stores 'flags' raised during import which are used to supplement logs
#[derive(Default, serde::Serialize)]
pub struct ImportFlags {
  pub guessed_files: HashMap<String, SpecificImportType>
}

impl<'ctx_deps, T: TPSEAccelerator> ImportContext<'ctx_deps, T> {
  pub fn new(asset_source: &'ctx_deps T::Asset, depth_limit: u8) -> ImportContext<'ctx_deps, T> {
    Self {
      depth_limit,
      asset_source,
      context: Default::default(),
      flags: Default::default(),
      logger: None
    }
  }

  pub fn with_logger(self, logger: &'ctx_deps (dyn ImportLogger + Send + Sync)) -> Self {
    Self { logger: Some(logger), ..self }
  }

  pub fn log(&self, level: LogLevel, message: impl Display) {
    if let Some(logger) = self.logger {
      logger.log(level, &self.context, &message);
    }
  }

  /// Wraps an ImportErrorType with this `ImportContext`'s context
  pub fn wrap(&self, error: ImportErrorType<T>) -> ImportError<T> {
    ImportError {
      context: self.context.clone(),
      error
    }
  }

  /// Checks if the context stack is at or beyond its depth limit
  pub fn is_too_deep(&self) -> bool {
    self.context.len() >= self.depth_limit as usize
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