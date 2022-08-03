use crate::import::{ImportErrorType, ImportType};

use crate::import::import_types::ImportOptions;
use crate::import::stages::{decide_specific_type, execute_task, reduce_types};
use crate::tpse::TPSE;

pub fn import(files: Vec<(ImportType, &str, &[u8])>, options: ImportOptions) -> Result<TPSE, ImportErrorType> {
  let mut results = Vec::with_capacity(files.len());
  for (file_type, name, contents) in files {
    results.push(decide_specific_type(file_type, name, contents, options.clone())?);
  }

  let tasks = reduce_types(&results)?;

  let mut tpse = TPSE::default();
  for task in tasks {
    tpse.merge(execute_task(task)?);
  }

  Ok(tpse)
}