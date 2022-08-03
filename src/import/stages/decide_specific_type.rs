use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use crate::import::{ImportErrorType, ImportResult, ImportType, SkinType, FileType, SpecificImportType as SIT, parse_filekey, ImportOptions, Asset, BackgroundType};
use crate::import::skin_splicer::{decode_image};
use crate::import::tetriojs::custom_sound_atlas;


/// Prepares a single file for import.
pub fn decide_specific_type<'a, 'b, 'c>
  (import_type: ImportType, filename: &'a str, bytes: &'b [u8], options: ImportOptions<'c>)
  -> Result<ImportResult<'a, 'b, 'c>, ImportErrorType>
{
  if options.depth_limit == 0 {
    return Err(ImportErrorType::TooMuchNesting)
  }

  let specific_import_type = match import_type {
    ImportType::Automatic => {
      if let Some(filekey) = parse_filekey(filename) {
        return decide_specific_type(filekey, filename, bytes, options.minus_one_depth())
      } else if let Some(guess) = FileType::from_extension(filename) {
        match guess {
          FileType::Zip => SIT::Zip,
          FileType::TPSE => SIT::TPSE,
          FileType::Image => {
            let image = decode_image(bytes)?;
            if let Some(format) = SkinType::guess_format(filename, image.width(), image.height()) {
              let format = ImportType::Skin { subtype: format };
              return decide_specific_type(format, filename, bytes, options.minus_one_depth())
            } else {
              match Path::new(&filename).extension().map(|ext| ext.to_string_lossy()) {
                Some(str) if str == "gif" => SIT::Background(BackgroundType::Video),
                _ => SIT::Background(BackgroundType::Image)
              }
            }
          },
          FileType::Video => SIT::Background(BackgroundType::Video),
          FileType::Audio => {
            let atlas = custom_sound_atlas(options.asset_source.provide(Asset::TetrioJS)?)?;
            let sfx = PathBuf::from(filename).file_stem().and_then(|ext| atlas.get(filename));
            match sfx {
              Some(_) => SIT::SoundEffects,
              None => SIT::Music
            }
          }
        }
      } else {
        return Err(ImportErrorType::UnknownFileType);
      }
    },
    ImportType::Skin { subtype } => SIT::Skin(subtype),
    ImportType::OtherSkin { subtype } => SIT::OtherSkin(subtype),
    ImportType::SoundEffects => SIT::SoundEffects,
    ImportType::Background => SIT::Background,
    ImportType::Music => SIT::Music
  };

  Ok(ImportResult::new(filename, bytes, options.clone(), specific_import_type))
}