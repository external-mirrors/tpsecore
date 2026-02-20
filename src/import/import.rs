use crate::import::{ImportContextEntry, ImportError, ImportErrorType, ImportTaskContextEntry, ImportType};

use crate::import::import_types::ImportContext;
use crate::import::stages::{decide_specific_type, execute_task, reduce_types};
use crate::tpse::TPSE;

pub async fn import(files: impl IntoIterator<Item = (ImportType, &str, &[u8])>, context: ImportContext<'_>) -> Result<TPSE, ImportError> {
  let files = files.into_iter();
  
  let mut results = Vec::with_capacity(files.size_hint().0);
  for (file_type, name, contents) in files {
    let context = context.with_context(ImportContextEntry::ImportFile(name.to_string(), file_type));
    results.push(decide_specific_type(file_type, name, contents, context).await?);
  }

  let tasks = reduce_types(&results, context.with_context(ImportContextEntry::ReduceTypes))?;

  let mut tpse = TPSE::default();
  for task in tasks {
    let context = context.with_context(ImportTaskContextEntry::from(&task).into());
    tpse.merge(execute_task(task, context).await?);
  }

  Ok(tpse)
}