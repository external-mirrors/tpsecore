use crate::import::SpecificImportType;
use crate::ImportOptions;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ImportResult<'a, 'b> {
  pub filename: &'a str,
  pub bytes: &'b [u8],
  pub options: ImportOptions,
  pub specific_import_type: SpecificImportType
}

impl<'a, 'b> ImportResult<'a, 'b> {
  pub fn new(filename: &'a str, bytes: &'b [u8], options: ImportOptions, specific_import_type: SpecificImportType) -> Self {
    Self { filename, bytes, options, specific_import_type }
  }
}