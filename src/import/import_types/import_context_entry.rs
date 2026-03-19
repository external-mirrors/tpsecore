use serde_with::{serde_as, DisplayFromStr};
use crate::import::{ImportType, SkinType, SpecificImportType};
use crate::import::import_task::ImportTask;

// ImportContextEntry serializes into a format meant mainly for interpolating into logs,
// and thus has no deserialize impl
#[serde_as]
#[derive(Debug, Clone, thiserror::Error, serde::Serialize)]
#[serde(tag = "kind", rename_all="kebab-case")]
pub enum ImportContextEntry {
  #[error("file `{file}` (as {as_type:?})")]
  ImportFile {
    file: String,
    #[serde(rename="type")]
    #[serde_as(as = "DisplayFromStr")]
    as_type: ImportType
  },
  #[error("frame source {frame} from file `{file}`")]
  FrameSource {
    frame: usize,
    file: String
  },
  #[error("zip folder {folder}")]
  ZipFolder {
    folder: String
  },
  #[error("with filekey {filekey:?}")]
  WithFilekey {
    #[serde_as(as = "DisplayFromStr")]
    filekey: ImportType
  },
  #[error("with guessed type {as_type:?}")]
  WithGuessedType {
    #[serde(rename="type")]
    #[serde_as(as = "DisplayFromStr")]
    as_type: ImportType
  },
  #[error("task {task}")]
  Task {
    #[from]
    #[serde(flatten)]
    task: ImportTaskContextEntry
  },
  #[error("reducing types")]
  ReduceTypes
}

#[derive(Debug, Clone, thiserror::Error, serde::Serialize)]
pub enum ImportTaskContextEntry {
  #[error("{skin_type:?} animated skin from frames: {frame_files:?}")]
  AnimatedSkinFrames {
    skin_type: SkinType,
    frame_files: Vec<String>
  },
  #[error("sound effects from files: {files:?}")]
  SoundEffects {
    files: Vec<String>
  },
  #[error("`{file}` (as {as_type:?})")]
  Basic {
    #[serde(rename="type")]
    as_type: SpecificImportType,
    file: String
  }
}

impl ImportTaskContextEntry {
  pub fn from(task: &ImportTask) -> Self {
    match task {
      ImportTask::AnimatedSkinFrames(skin_type, files) => {
        let files = files.iter().map(|file| file.filename.clone()).collect();
        Self::AnimatedSkinFrames { skin_type: *skin_type, frame_files: files }
      }
      ImportTask::SoundEffects(effects) => {
        let files = effects.iter().map(|sfx| sfx.filename.clone()).collect();
        Self::SoundEffects { files }
      }
      ImportTask::Basic { import_type, filename, .. } => {
        Self::Basic { as_type: *import_type, file: filename.clone() }
      }
    }
  }
}