use crate::accel::traits::TPSEAccelerator;
use crate::import::inter_stage_data::QueuedFile;
use crate::import::{ImportContext, ImportContextEntry, ImportError, ImportErrorWrapHelper, ImportTaskContextEntry, err};

use crate::import::stages::{execute_task, explore_files, reduce_types};
use crate::log::LogLevel;
use crate::tpse::tpse_key::{AllKnownKeys, merge};


pub async fn import<T: TPSEAccelerator>
  (context: &mut ImportContext<'_, T>, files: Vec<QueuedFile>, target: &mut impl AllKnownKeys)
  -> Result<(), ImportError<T>>
{
  match import_inner(context, files, target).await {
    Err(err) => {
      context.log(LogLevel::Error, &format_args!("Import failed: {err}"));
      Err(err)
    }
    Ok(()) => {
      context.log(LogLevel::Status, &"Import finished");
      Ok(())
    }
  }
}

async fn import_inner<T: TPSEAccelerator>
  (context: &mut ImportContext<'_, T>, files: Vec<QueuedFile>, target: &mut impl AllKnownKeys)
  -> Result<(), ImportError<T>>
{
  let results = explore_files(files, &mut *context.enter_context(ImportContextEntry::ExploreFiles)).await?;
  let tasks = reduce_types(&results, &mut *context.enter_context(ImportContextEntry::ReduceTypes))?;

  for task in tasks {
    let mut guard = context.enter_context(ImportTaskContextEntry::from(&task).into());
    let execute = execute_task::<T>(task, &mut *guard).await?;
    merge(target, &execute).await.wrap(err!(guard))?;
  }

  Ok(())
}

