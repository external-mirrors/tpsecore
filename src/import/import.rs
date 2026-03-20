use crate::accel::traits::TPSEAccelerator;
use crate::import::inter_stage_data::QueuedFile;
use crate::import::{ImportContext, ImportContextEntry, ImportError, ImportErrorWrapHelper, ImportTaskContextEntry, err};

use crate::import::stages::{execute_task, explore_files, reduce_types};
use crate::tpse::TPSE;
use crate::tpse::tpse_key::merge;


#[allow(unused)]
pub async fn import<T: TPSEAccelerator>
  (context: &mut ImportContext<'_, T>, files: Vec<QueuedFile>)
  -> Result<TPSE, ImportError<T>>
{
  let results = explore_files(files, &mut *context.enter_context(ImportContextEntry::ExploreFiles)).await?;
  let tasks = reduce_types(&results, &mut *context.enter_context(ImportContextEntry::ReduceTypes))?;

  let mut tpse = TPSE::default();
  for task in tasks {
    let mut guard = context.enter_context(ImportTaskContextEntry::from(&task).into());
    let execute = execute_task::<T>(task, &mut *guard).await?;
    merge(&mut tpse, &execute).await.wrap(err!(guard))?;
  }

  Ok(tpse)
}


