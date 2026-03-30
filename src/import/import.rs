use crate::accel::traits::{ImportDecisionMaker, TPSEAccelerator};
use crate::import::inter_stage_data::ImportFile;
use crate::import::{ImportContext, ImportContextEntry, ImportError, ImportErrorWrapHelper, ImportTaskContextEntry, ImportType, err};

use crate::import::stages::{execute_task, explore_files, partition_import_groups, reduce_types};
use crate::log::LogLevel;
use crate::tpse::tpse_key::{AllKnownKeys, merge};


pub async fn import<T: TPSEAccelerator>
  (context: &mut ImportContext<'_, T>, files: Vec<ImportFile<ImportType>>, target: &mut impl AllKnownKeys)
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
  (context: &mut ImportContext<'_, T>, files: Vec<ImportFile<ImportType>>, target: &mut impl AllKnownKeys)
  -> Result<(), ImportError<T>>
{
  let results = explore_files(files, &mut *context.enter_context(ImportContextEntry::ExploreFiles)).await?;
  
  let mut guard = context.enter_context(ImportContextEntry::PartitionGroups);
  let options = partition_import_groups(&results, &mut *guard)?;
  
  guard.log(LogLevel::Status, &"Decision needed");
  let decisions = guard.decider.decide(&options).await.wrap(err!(guard, decisionfail))?;
  guard.log(LogLevel::Info, &format_args!("Decision made: {decisions:?}"));
  drop(guard);
  
  let mut decided_files = vec![];
  for tree in &options {
    tree.visit(&mut |o| {
      if let Some(decision) = decisions.get(&o.id) {
        decided_files.extend(o.options[*decision].files.iter().map(|f| (*f).clone()));
      }
    });
  };
  
  let tasks = reduce_types(&decided_files, &mut *context.enter_context(ImportContextEntry::ReduceTypes))?;

  for task in tasks {
    let mut guard = context.enter_context(ImportTaskContextEntry::from(&task).into());
    let execute = execute_task::<T>(task, &mut *guard).await?;
    merge(target, &execute).await.wrap(err!(guard))?;
  }

  Ok(())
}

