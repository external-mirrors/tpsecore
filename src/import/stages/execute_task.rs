use image::DynamicImage;
use crate::import::import_task::ImportTask;
use crate::import::{ImportErrorType, SkinType, SpecificImportType};
use crate::import::skin_splicer::SkinSplicer;
use crate::tpse::TPSE;

/// Executes an import task
pub fn execute_task(task: ImportTask) -> Result<TPSE, ImportErrorType> {
  let mut tpse = TPSE::default();
  match task {
    ImportTask::AnimatedSkinFrames(skin_type, frames) => todo!(),
    ImportTask::SoundEffects(sound_effects) => todo!(),
    ImportTask::Basic(specific_type, file) => {
      match specific_type {
        SpecificImportType::Zip => todo!(),
        SpecificImportType::TPSE => {
          tpse.merge(serde_json::from_slice(file).map_err(|err| {
            ImportErrorType::InvalidTPSE(err.to_string())
          })?);
        },
        SpecificImportType::Skin(skin_type) => {
          let (minos, ghost) = splice_to_t61(skin_type, file)?;
          if let Some(minos) = minos { tpse.skin = Some(minos.into()); }
          if let Some(ghost) = ghost { tpse.ghost = Some(ghost.into()); }
        },
        SpecificImportType::OtherSkin(skin_type) => todo!(),
        SpecificImportType::SoundEffects => todo!(),
        SpecificImportType::Background => todo!(),
        SpecificImportType::Music => todo!(),
      }
    }
  };
  Ok(tpse)
}

fn splice_to_t61(skin_type: SkinType, bytes: &[u8])
  -> Result<(Option<DynamicImage>, Option<DynamicImage>), ImportErrorType>
{
  let target_resolution = 96;
  let mut source = SkinSplicer::default();
  source.load(skin_type, bytes)?;
  let minos = source.convert(SkinType::Tetrio61Connected, Some(target_resolution));
  let ghost = source.convert(SkinType::Tetrio61ConnectedGhost, Some(target_resolution));
  Ok((minos, ghost))
}