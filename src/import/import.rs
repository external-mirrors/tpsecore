use crate::import::{ImportError, ImportType};
use crate::import::stages::decide_specific_type;
use crate::ImportError;
use crate::tpse::TPSE;

pub fn import(files: Vec<(ImportType, &str, &[u8])>) -> Result<TPSE, ImportError> {
  let results = Vec::with_capacity(files.len());
  for (file_type, name, contents) in files {
    results.push(decide_specific_type(file_type, name, contents)?);
  }
  todo!()
}