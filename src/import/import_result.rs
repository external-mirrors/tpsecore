use std::sync::Arc;

use crate::import::{ImportContext, SpecificImportType};
use crate::tpse::File;

#[derive(Clone)]
pub struct ImportResult<'c> {
  pub filename: String,
  pub file: File,
  pub specific_import_type: SpecificImportType,
  pub options: ImportContext<'c>
}

impl<'c> ImportResult<'c> {
  pub fn new(
    filename: &str,
    bytes: Arc<[u8]>,
    mime_type: &str,
    options: ImportContext<'c>,
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