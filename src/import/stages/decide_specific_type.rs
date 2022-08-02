use crate::import::{ImportError, ImportResult, ImportType, SkinType, FileType, SpecificImportType as SIT, parse_filekey};
use crate::import::import_types::ImportOptions;
use crate::ImportOptions;

/// Prepares a single file for import.
pub fn decide_specific_type<'a, 'b>
  (import_type: ImportType, filename: &'a str, bytes: &'b [u8], options: ImportOptions)
  -> Result<ImportResult<'a, 'b>, ImportError>
{
  if options.depth_limit == 0 { return Err(ImportError::TooMuchNesting) }
  Ok(ImportResult::new(filename, bytes, options, match import_type {
    ImportType::Automatic => {
      if let Some(guess) = parse_filekey(filename) {
        return decide_specific_type(guess, filename, bytes, options.minus_one_depth())
      } else if let Some(guess) = FileType::from_extension(filename) {
        match guess {
          FileType::Zip => SIT::Zip,
          //import_zip_file(bytes, options.minus_one_depth()),
          FileType::TPSE => SIT::TPSE,
          FileType::Image => {
            return if let Some(format) = SkinType::guess_format(filename, 0, 0) {
              let format = ImportType::Skin { subtype: format };
              decide_specific_type(format, filename, bytes, options.minus_one_depth())
            } else {
              Err(ImportError::UnknownFileType)
            }
          },
          FileType::Video => todo!(),
          FileType::Audio => todo!()
        }
      } else {
        return Err(ImportError::UnknownFileType);
      }
    },
    ImportType::Skin { subtype } => SIT::Skin(subtype),
    ImportType::OtherSkin { subtype } => SIT::OtherSkin(subtype),
    ImportType::SoundEffects => SIT::SoundEffects,
    ImportType::Background => SIT::Background,
    ImportType::Music => SIT::Music
  }))
}