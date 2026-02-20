use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use log::Level;
use crate::import::{ImportErrorType, ImportResult, ImportType, SkinType, FileType, SpecificImportType as SIT, ImportContext, Asset, BackgroundType, ImportContextEntry, ImportError};
use crate::import::skin_splicer::{decode_image};
use crate::import::tetriojs::custom_sound_atlas;


/// Prepares a single file for import.
pub async fn decide_specific_type<'c>
  (import_type: ImportType, filename: &str, bytes: &[u8], ctx: ImportContext<'c>)
   -> Result<ImportResult<'c>, ImportError>
{
  ctx.log(Level::Debug, format_args!("Deciding import type for {:?} {}", import_type, filename));
  if ctx.is_too_deep() {
    return Err(ctx.wrap(ImportErrorType::TooMuchNesting))
  }

  let specific_import_type = match import_type {
    ImportType::Automatic => {
      if let Some(filekey) = ImportType::parse_filekey(filename) {
        let context = ctx.with_context(ImportContextEntry::WithFilekey(filekey));
        return Box::pin(decide_specific_type(filekey, filename, bytes, context)).await;
      } else if let Some(guess) = FileType::from_extension(filename) {
        match guess {
          FileType::Zip => SIT::Zip,
          FileType::TPSE => SIT::TPSE,
          FileType::Image => {
            let image = decode_image(bytes).map_err(|err| ctx.wrap(err.into()))?;
            if let Some(format) = SkinType::guess_format(filename, image.width(), image.height(), &ctx) {
              let format = ImportType::Skin { subtype: format };
              let context = ctx.with_context(ImportContextEntry::WithGuessedType(format));
              return Box::pin(decide_specific_type(format, filename, bytes, context)).await
            } else {
              match Path::new(&filename).extension().map(|ext| ext.to_string_lossy()) {
                Some(str) if str == "gif" => SIT::Background(BackgroundType::Video),
                _ => SIT::Background(BackgroundType::Image)
              }
            }
          },
          FileType::Video => SIT::Background(BackgroundType::Video),
          FileType::Audio => {
            let asset = ctx.asset_source.provide(Asset::TetrioJS).await.map_err(|err| ctx.wrap(err))?;
            let atlas = custom_sound_atlas(&asset).map_err(|err| ctx.wrap(err.into()))?;
            let sfx = PathBuf::from(filename).file_stem().and_then(|ext| atlas.get(filename));
            match sfx {
              Some(_) => SIT::SoundEffects,
              None => SIT::Music
            }
          }
        }
      } else {
        return Err(ctx.wrap(ImportErrorType::UnknownFileType));
      }
    },
    ImportType::Skin { subtype } => SIT::Skin(subtype),
    ImportType::OtherSkin { subtype } => SIT::OtherSkin(subtype),
    ImportType::SoundEffects => SIT::SoundEffects,
    ImportType::Background { subtype } => SIT::Background(subtype),
    ImportType::Music => SIT::Music
  };

  let mime = mime_guess::from_path(filename).first_or_octet_stream().to_string();
  Ok(ImportResult::new(filename, bytes, &mime, ctx.clone(), specific_import_type))
}