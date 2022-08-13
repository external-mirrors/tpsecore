use std::fmt::{Display, Formatter};
use crate::tpse::{MiscTPSEValue, TPSE};

impl TPSE {
  pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
    let mut errors = vec![];
    use ValidationError::*;
    use AssetValidationError::*;
    use AssetType::*;

    // todo: check image sizes

    if let Some(music) = &self.music {
      for song in music {
        self.ensure_exists(&song.id, Song).map_err(|err| errors.push(err));
      }
    }

    if let Some(backgrounds) = &self.backgrounds {
      for bg in backgrounds {
        self.ensure_exists(&bg.id, Background).map_err(|err| errors.push(err));
      }
    }

    if let Some(graph) = &self.music_graph {
      for node in graph {
        for (key, asset_type) in [(&node.audio, Song), (&node.background, Background)] {
          key.as_ref().map(|id| self.ensure_exists(id, asset_type).map_err(|err| errors.push(err)));
        }
        for trigger in &node.triggers {
          if !graph.iter().filter(|node| node.id == trigger.target).next().is_some() {
            errors.push(MusicGraphPointer { source: node.id, target: trigger.target });
          }
        }
      }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
  }

  fn ensure_exists(&self, id: &str, asset_type: AssetType) -> Result<(), ValidationError> {
    use ValidationError::*;
    use AssetValidationError::*;
    match self.other.get(&format!("{}-{}", asset_type.tpse_key_prefix(), id)) {
      Some(MiscTPSEValue::File(file)) if file.mime.starts_with(asset_type.mime_prefix()) => {
        Ok(())
      }
      Some(MiscTPSEValue::File(file)) => {
        Err(AssetError(id.to_string(), asset_type, UnexpectedMIME(file.mime.clone())))
      }
      Some(MiscTPSEValue::Other(_)) => {
        Err(AssetError(id.to_string(), asset_type, InvalidFile))
      }
      None => {
        Err(AssetError(id.to_string(), asset_type, Missing))
      }
    }
  }
}

#[derive(Debug, Clone)]
pub enum ValidationError {
  MusicGraphPointer { source: u64, target: u64 },
  AssetError(String, AssetType, AssetValidationError)
}
impl Display for ValidationError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      // thiserror does not like the non-error-source source field
      ValidationError::MusicGraphPointer { source, target } => {
        write!(f, "invalid music graph pointer from node {} to {}", source, target)
      },
      ValidationError::AssetError(name, asset_type, error) => {
        write!(f, "{} asset {} failed to validate: {}", asset_type, name, error)
      }
    }
  }
}

#[derive(Debug, thiserror::Error, Copy, Clone)]
pub enum AssetType {
  #[error("song")]
  Song,
  #[error("background")]
  Background
}

impl AssetType {
  pub fn tpse_key_prefix(&self) -> &'static str {
    match self {
      AssetType::Song => "song",
      AssetType::Background => "background"
    }
  }
  pub fn mime_prefix(&self) -> &'static str {
    match self {
      AssetType::Song => "audio",
      AssetType::Background => "image"
    }
  }
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum AssetValidationError {
  #[error("not found")]
  Missing,
  #[error("invalid file")]
  InvalidFile,
  #[error("unexpected mime type: {0}")]
  UnexpectedMIME(String),
}