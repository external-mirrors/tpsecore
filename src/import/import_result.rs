use crate::import::SpecificImportType;
use crate::tpse::File;

#[derive(Clone)]
pub struct ImportResult {
  pub filename: String,
  pub file: File,
  pub specific_import_type: SpecificImportType
}