use std::sync::Arc;

use crate::accel::traits::TPSEAccelerator;
use crate::import::{ImportContextEntry, ImportError, ImportTaskContextEntry, ImportType};

use crate::import::import_types::ImportContext;
use crate::import::stages::{decide_specific_type, execute_task, reduce_types};
use crate::tpse::TPSE;
use crate::tpse::tpse_key::merge;

pub async fn import<T: TPSEAccelerator>
  (files: impl IntoIterator<Item = (ImportType, &str, Arc<[u8]>)>, context: &mut ImportContext<'_, T>)
  -> Result<TPSE, ImportError<T>>
{
  let files = files.into_iter();
  
  let mut results = Vec::with_capacity(files.size_hint().0);
  for (file_type, name, contents) in files {
    let mut context = context.enter_context(ImportContextEntry::ImportFile { file: name.to_string(), as_type: file_type});
    results.push(decide_specific_type::<T>(file_type, name, contents, &mut *context).await?);
  }

  let tasks = reduce_types(&results, &mut *context.enter_context(ImportContextEntry::ReduceTypes))?;

  let mut tpse = TPSE::default();
  for task in tasks {
    let mut guard = context.enter_context(ImportTaskContextEntry::from(&task).into());
    merge(&mut tpse, &execute_task::<T>(task, &mut *guard).await?)
      .await.map_err(|err| guard.wrap(err.into()))?;
  }

  Ok(tpse)
}