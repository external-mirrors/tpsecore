use std::io::Cursor;
use image::DynamicImage;
use crate::import::{
  ImportErrorType, ImportResult, ImportType, SkinType, FileType, SpecificImportType as SIT,
  parse_filekey, ImportOptions
};
use crate::import::skin_splicer::{decode_image, SkinSplicer};


/// Prepares a single file for import.
pub fn decide_specific_type<'a, 'b, 'c>
  (import_type: ImportType, filename: &'a str, bytes: &'b [u8], options: ImportOptions<'c>)
  -> Result<ImportResult<'a, 'b, 'c>, ImportErrorType>
{
  if options.depth_limit == 0 { return Err(ImportErrorType::TooMuchNesting) }
  Ok(ImportResult::new(filename, bytes, options.clone(), match import_type {
    ImportType::Automatic => {
      if let Some(guess) = parse_filekey(filename) {
        return decide_specific_type(guess, filename, bytes, options.minus_one_depth())
      } else if let Some(guess) = FileType::from_extension(filename) {
        match guess {
          FileType::Zip => SIT::Zip,
          FileType::TPSE => SIT::TPSE,
          FileType::Image => {
            let image = decode_image(bytes)?;
            return if let Some(format) = SkinType::guess_format(filename, image.width(), image.height()) {
              let format = ImportType::Skin { subtype: format };
              decide_specific_type(format, filename, bytes, options.minus_one_depth())
            } else {
              Err(ImportErrorType::UnknownFileType)
            }
          },
          FileType::Video => todo!(),
          FileType::Audio => {
            todo!()
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
  }))
}