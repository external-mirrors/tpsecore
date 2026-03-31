use std::path::PathBuf;

use serde_with::{serde_as, DisplayFromStr};
use crate::import::inter_stage_data::{ImportTask};
use crate::import::{Asset, ImportType, SkinType, TypeStage4};

// ImportContextEntry serializes into a format meant mainly for interpolating into logs,
// and thus has no deserialize impl
#[serde_as]
#[derive(Debug, Clone, thiserror::Error, serde::Serialize)]
#[serde(tag = "kind", rename_all="kebab-case")]
pub enum ImportContextEntry {
  #[error("file `{file}` (as {as_type:?})")]
  ImportFile {
    file: PathBuf,
    #[serde(rename="type")]
    #[serde_as(as = "DisplayFromStr")]
    as_type: ImportType
  },
  #[error("with files `{files:?}`")]
  WithFiles {
    files: Vec<PathBuf>
  },
  #[error("frame source {frame} from file `{file}`")]
  FrameSource {
    frame: usize,
    file: PathBuf
  },
  #[error("zip folder {folder}")]
  ZipFolder {
    folder: PathBuf
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
  #[error("provide asset {asset}")]
  ProvideAsset {
    asset: Asset
  },
  #[error("from nearest pack.json file at {pack_json_file:?}")]
  PackJson {
    pack_json_file: PathBuf
  },
  #[error("exploring files")]
  ExploreFiles,
  #[error("partitioning import groups")]
  PartitionGroups,
  #[error("reducing types")]
  ReduceTypes
}

#[serde_as]
#[derive(Debug, Clone, thiserror::Error, serde::Serialize)]
#[serde(tag = "task", rename_all="snake_case")]
pub enum ImportTaskContextEntry {
  #[error("{skin_type:?} animated skin from frames: {frame_files:?}")]
  AnimatedSkinFrames {
    skin_type: SkinType,
    frame_files: Vec<PathBuf>
  },
  #[error("sound effects from files: {files:?}")]
  SoundEffects {
    files: Vec<PathBuf>
  },
  #[error("`{file}` (as {as_type:?})")]
  Basic {
    #[serde(rename="type")]
    #[serde_as(as = "DisplayFromStr")]
    as_type: TypeStage4,
    file: PathBuf
  }
}

impl ImportTaskContextEntry {
  pub fn from(task: &ImportTask) -> Self {
    match task {
      ImportTask::AnimatedSkinFrames(skin_type, files) => {
        let files = files.iter().map(|file| file.path.clone()).collect();
        Self::AnimatedSkinFrames { skin_type: *skin_type, frame_files: files }
      }
      ImportTask::SoundEffects(effects) => {
        let files = effects.iter().map(|sfx| sfx.path.clone()).collect();
        Self::SoundEffects { files }
      }
      ImportTask::Basic { import_type, path, .. } => {
        Self::Basic { as_type: *import_type, file: path.clone() }
      }
    }
  }
}