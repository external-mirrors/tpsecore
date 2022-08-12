use crate::import::{ImportContextEntry, ImportError, ImportErrorType, ImportTaskContextEntry, ImportType};

use crate::import::import_types::ImportContext;
use crate::import::stages::{decide_specific_type, execute_task, reduce_types};
use crate::tpse::TPSE;

pub fn import(files: Vec<(ImportType, &str, &[u8])>, context: ImportContext) -> Result<TPSE, ImportError> {
  let mut results = Vec::with_capacity(files.len());
  for (file_type, name, contents) in files {
    let context = context.with_context(ImportContextEntry::ImportFile(name.to_string(), file_type));
    results.push(decide_specific_type(file_type, name, contents, context)?);
  }

  let tasks = reduce_types(&results, context.with_context(ImportContextEntry::ReduceTypes))?;

  let mut tpse = TPSE::default();
  for task in tasks {
    let context = context.with_context(ImportTaskContextEntry::from(&task).into());
    tpse.merge(execute_task(task, context)?);
  }

  Ok(tpse)
}