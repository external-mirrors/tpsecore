use crate::import::{ImportOptions, SpecificImportType};

#[derive(Clone)]
pub struct ImportResult<'a, 'b, 'c> {
  pub filename: &'a str,
  pub bytes: &'b [u8],
  pub options: ImportOptions<'c>,
  pub specific_import_type: SpecificImportType
}

impl<'a, 'b, 'c> ImportResult<'a, 'b, 'c> {
  pub fn new(
    filename: &'a str,
    bytes: &'b [u8],
    options: ImportOptions<'c>,
    specific_import_type: SpecificImportType
  ) -> Self {
    Self { filename, bytes, options, specific_import_type }
  }
}