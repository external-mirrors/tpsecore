use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use log::Level;
use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::{Asset, BackgroundType, FileType, ImportContext, ImportContextEntry, ImportError, ImportErrorType, ImportResult, ImportType, LoadError, SkinType, SpecificImportType as SIT};
use crate::import::radiance::parse_radiance_sound_definition;

/// Prepares a single file for import.
pub async fn decide_specific_type<'c, T: TPSEAccelerator>
  (import_type: ImportType, filename: &str, bytes: Arc<[u8]>, ctx: ImportContext<'c>)
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
        return Box::pin(decide_specific_type::<T>(filekey, filename, bytes, context)).await;
      } else if let Some(guess) = FileType::from_extension(filename) {
        match guess {
          FileType::Zip => SIT::Zip,
          FileType::TPSE => SIT::TPSE,
          FileType::Image => {
            let image = T::decode_texture(bytes.clone())
              .map_err(|err| ctx.wrap(LoadError::ErasedError(Box::new(err)).into()))?;
            if let Some(format) = SkinType::guess_format(filename, image.width(), image.height(), &ctx) {
              let format = ImportType::Skin { subtype: format };
              let context = ctx.with_context(ImportContextEntry::WithGuessedType(format));
              return Box::pin(decide_specific_type::<T>(format, filename, bytes, context)).await
            } else {
              match Path::new(&filename).extension().map(|ext| ext.to_string_lossy()) {
                Some(str) if str == "gif" => SIT::Background(BackgroundType::Video),
                _ => SIT::Background(BackgroundType::Image)
              }
            }
          },
          FileType::Video => SIT::Background(BackgroundType::Video),
          FileType::Audio => {
            let asset = ctx.asset_source.provide(Asset::TetrioRSD).await.map_err(|err| ctx.wrap(err))?;
            let rsd = parse_radiance_sound_definition(&asset).map_err(|err| ctx.wrap(err.into()))?;
            let atlas = rsd.to_old_style_atlas();
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