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
    bytes: &[u8],
    mime_type: &str,
    options: ImportContext<'c>,
    specific_import_type: SpecificImportType
  ) -> Self {
    Self {
      filename: filename.to_string(),
      file: File { binary: bytes.to_owned(), mime: mime_type.to_string() },
      specific_import_type,
      options
    }
  }
}