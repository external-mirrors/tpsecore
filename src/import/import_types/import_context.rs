use std::fmt::Arguments;
use itertools::Itertools;
use log::Level;
use crate::accel::traits::TPSEAccelerator;
use crate::import::{ImportContextEntry, ImportError, ImportErrorType};
use crate::log::ImportLogger;

/// Stores metadata and context associated with an import process, tracking the stack location
/// (e.g. nested zip files) and base game asset provider.
pub struct ImportContext<'a, T: TPSEAccelerator> {
  /// The asset provider providing `Asset`s for the importer
  pub asset_source: &'a T::Asset,
  /// The maximum depth the context stack is allowed to reach before bailing
  pub depth_limit: u8,
  /// A stack of context describing the current item the importer is working on
  pub context: Vec<ImportContextEntry>,
  /// An outlet for diagnostic/progress messages
  pub logger: Option<&'a (dyn ImportLogger + Send + Sync)>
}

impl<T: TPSEAccelerator> Clone for ImportContext<'_, T> {
  fn clone(&self) -> Self {
    Self {
      asset_source: self.asset_source,
      depth_limit: self.depth_limit.clone(),
      context: self.context.clone(),
      logger: self.logger.clone()
    }
  }
}

impl<'a, T: TPSEAccelerator> ImportContext<'a, T> {
  pub fn new(asset_source: &'a T::Asset, depth_limit: u8) -> ImportContext<'a, T> {
    Self {
      depth_limit,
      asset_source,
      context: Default::default(),
      logger: None
    }
  }

  pub fn with_logger(self, logger: &'a (dyn ImportLogger + Send + Sync)) -> Self {
    Self { logger: Some(logger), ..self }
  }

  pub fn log(&self, level: Level, message: Arguments) {
    if let Some(logger) = self.logger {
      logger.log(level, format_args!("[{:?}] {}", self.context.iter().format(" "), message));
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

  /// Creates a new `ImportContext` with extra context
  pub fn with_context(&self, context: ImportContextEntry) -> Self {
    let mut clone = self.clone();
    self.log(Level::Debug, format_args!("Entering context {:?}", context));
    clone.context.push(context.clone());
    clone
  }
}