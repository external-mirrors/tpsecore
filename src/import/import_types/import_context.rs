use std::fmt::Arguments;
use std::sync::{Arc, Mutex};
use itertools::Itertools;
use log::Level;
use crate::import::asset_provider::AssetProvider;
use crate::import::{ImportContextEntry, ImportError, ImportErrorType};

/// Stores metadata and context associated with an import process, tracking the stack location
/// (e.g. nested zip files) and base game asset provider.
#[derive(Clone)]
pub struct ImportContext<'a> {
  /// The asset provider providing `Asset`s for the importer
  pub asset_source: &'a dyn AssetProvider + Send + Sync,
  /// The maximum depth the context stack is allowed to reach before bailing
  pub depth_limit: u8,
  /// A stack of context describing the current item the importer is working on
  pub context: Vec<ImportContextEntry>,
  /// An outlet for diagnostic/progress messages
  pub logger: Option<&'a dyn Fn(Level, Arguments) + Send + Sync>
}

impl<'a> ImportContext<'a> {
  pub fn new(asset_source: &'a dyn AssetProvider + Send + Sync, depth_limit: u8) -> ImportContext<'a> {
    Self {
      depth_limit,
      asset_source,
      context: Default::default(),
      logger: None
    }
  }

  pub fn with_logger(self, logger: &'a dyn Fn(Level, Arguments) + Send + Sync) -> Self {
    Self { logger: Some(logger), ..self }
  }

  pub fn log(&self, level: Level, message: Arguments) {
    if let Some(logger) = self.logger {
      (logger)(level, format_args!("[{:?}] {}", self.context.iter().format(" "), message));
    }
  }

  /// Wraps an ImportErrorType with this `ImportContext`'s context
  pub fn wrap(&self, error: ImportErrorType) -> ImportError {
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