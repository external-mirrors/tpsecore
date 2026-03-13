use crate::import::{ImportType, SkinType, SpecificImportType};
use crate::import::import_task::ImportTask;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ImportContextEntry {
  #[error("file `{0}` (as {1:?})")]
  ImportFile(String, ImportType),
  #[error("frame source {0} from file `{1}`")]
  FrameSource(usize, String),
  #[error("zip folder {0}")]
  ZipFolder(String),
  #[error("with filekey {0:?}")]
  WithFilekey(ImportType),
  #[error("with guessed type {0:?}")]
  WithGuessedType(ImportType),
  #[error("task {0}")]
  Task(#[from] ImportTaskContextEntry),
  #[error("reducing types")]
  ReduceTypes
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ImportTaskContextEntry {
  #[error("{0:?} animated skin from frames: {1:?}")]
  AnimatedSkinFrames(SkinType, Vec<String>),
  #[error("sound effects from files: {0:?}")]
  SoundEffects(Vec<String>),
  #[error("`{1}` (as {0:?})")]
  Basic(SpecificImportType, String)
}

impl ImportTaskContextEntry {
  pub fn from(task: &ImportTask) -> Self {
    match task {
      ImportTask::AnimatedSkinFrames(skin_type, files) => {
        let files = files.iter().map(|file| file.filename.clone()).collect();
        Self::AnimatedSkinFrames(*skin_type, files)
      }
      ImportTask::SoundEffects(effects) => {
        Self::SoundEffects(effects.iter().map(|sfx| sfx.filename.clone()).collect())
      }
      ImportTask::Basic { import_type, filename, .. } => {
        Self::Basic(*import_type, filename.clone())
      }
    }
  }
}