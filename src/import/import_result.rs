use std::sync::Arc;

use crate::accel::traits::TPSEAccelerator;
use crate::import::{ImportContext, SpecificImportType};
use crate::tpse::File;

pub struct ImportResult<'c, T: TPSEAccelerator> {
  pub filename: String,
  pub file: File,
  pub specific_import_type: SpecificImportType,
  pub options: ImportContext<'c, T>
}

impl<T: TPSEAccelerator> Clone for ImportResult<'_, T> {
  fn clone(&self) -> Self {
    Self {
      filename: self.filename.clone(),
      file: self.file.clone(),
      specific_import_type: self.specific_import_type.clone(),
      options: self.options.clone()
    }
  }
}

impl<'c, T: TPSEAccelerator> ImportResult<'c, T> {
  pub fn new(
    filename: &str,
    bytes: Arc<[u8]>,
    mime_type: &str,
    options: ImportContext<'c, T>,
    specific_import_type: SpecificImportType
  ) -> Self {
    Self {
      filename: filename.to_string(),
      file: File { binary: bytes, mime: mime_type.to_string() },
      specific_import_type,
      options
    }
  }
}