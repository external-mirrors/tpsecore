use std::sync::{Arc, Mutex};
use crate::import::asset_provider::AssetProvider;
use crate::import::{ImportContextEntry, ImportError, ImportErrorType};

/// Stores metadata and context associated with an import process, tracking the stack location
/// (e.g. nested zip files) and base game asset provider.
#[derive(Clone)]
pub struct ImportContext<'a> {
  /// The asset provider providing `Asset`s for the importer
  pub asset_source: &'a dyn AssetProvider,
  /// The maximum depth the context stack is allowed to reach before bailing
  pub depth_limit: u8,
  /// A stack of context describing the current item the importer is working on
  pub context: Vec<ImportContextEntry>,
  /// A list of all context the importer has ever encountered
  /// May end up looking achronological due to lack of context exit markers
  pub context_history: Arc<Mutex<Vec<ImportContextEntry>>>
}

impl<'a> ImportContext<'a> {
  pub fn new(asset_source: &'a dyn AssetProvider, depth_limit: u8) -> ImportContext<'a> {
    Self {
      depth_limit,
      asset_source,
      context: Default::default(),
      context_history: Default::default()
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
    clone.context.push(context.clone());
    clone.context_history.lock().unwrap().push(context);
    clone
  }
}