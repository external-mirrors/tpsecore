use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::accel::traits::{AssetProvider, TPSEAccelerator, TextureHandle};
use crate::import::{Asset, BackgroundType, FileType, ImportContext, ImportContextEntry, ImportError, ImportErrorType, ImportResult, ImportType, MediaLoadError, SkinType, SpecificImportType as SIT};
use crate::import::radiance::parse_radiance_sound_definition;
use crate::log::LogLevel;
use crate::tpse::File;

/// Prepares a single file for import.
pub async fn decide_specific_type<'c, T: TPSEAccelerator>
  (import_type: ImportType, filename: &str, bytes: Arc<[u8]>, ctx: &mut ImportContext<'c, T>)
   -> Result<ImportResult, ImportError<T>>
{
  ctx.log(LogLevel::Debug, format_args!("Deciding import type for {:?} {}", import_type, filename));
  if ctx.is_too_deep() {
    return Err(ctx.wrap(ImportErrorType::TooMuchNesting))
  }

  let specific_import_type = match import_type {
    ImportType::Automatic => {
      if let Some(filekey) = ImportType::parse_filekey(filename) {
        let mut guard = ctx.enter_context(ImportContextEntry::WithFilekey { filekey });
        return Box::pin(decide_specific_type::<T>(filekey, filename, bytes, &mut *guard)).await;
      }
      let Some(guess) = FileType::from_extension(filename) else {
        return Err(ctx.wrap(ImportErrorType::UnknownFileType));
      };
      
      let guessed_type = match guess {
        FileType::Zip => SIT::Zip,
        FileType::TPSE => SIT::TPSE,
        FileType::Image => {
          let image = T::Texture::decode_texture(bytes.clone())
            .map_err(|err| ctx.wrap(MediaLoadError::TextureError(err).into()))?;
          let width = image.width().await
            .map_err(|err| ctx.wrap(MediaLoadError::TextureError(err).into()))?;
          let height = image.height().await
            .map_err(|err| ctx.wrap(MediaLoadError::TextureError(err).into()))?;
          if let Some(format) = SkinType::guess_format(filename, width, height, &ctx) {
            let format = ImportType::Skin { subtype: format };
            let mut guard = ctx.enter_context(ImportContextEntry::WithGuessedType { as_type: format });
            return Box::pin(decide_specific_type::<T>(format, filename, bytes, &mut *guard)).await
          } else {
            match Path::new(&filename).extension().map(|ext| ext.to_string_lossy()) {
              Some(str) if str == "gif" => SIT::Background(BackgroundType::Video),
              _ => SIT::Background(BackgroundType::Image)
            }
          }
        },
        FileType::Video => SIT::Background(BackgroundType::Video),
        FileType::Audio => {
          let asset = ctx.asset_source.provide(Asset::TetrioRSD).await
            .map_err(|err| ctx.wrap(ImportErrorType::AssetFetchFailed(err)))?;
          let rsd = parse_radiance_sound_definition(&asset).map_err(|err| ctx.wrap(err.into()))?;
          let atlas = rsd.to_old_style_atlas();
          let sfx = PathBuf::from(filename).file_stem().and_then(|ext| ext.to_str()).and_then(|ext| atlas.get(ext));
          match sfx {
            Some(_) => SIT::SoundEffects,
            None => SIT::Music
          }
        }
      };
      ctx.log(LogLevel::Info, format_args!("Guessed import type {guessed_type}"));
      ctx.flags.guessed_files.insert(filename.to_string(), guessed_type.clone());
      guessed_type
    },
    ImportType::Skin { subtype } => SIT::Skin(subtype),
    ImportType::OtherSkin { subtype } => SIT::OtherSkin(subtype),
    ImportType::SoundEffects => SIT::SoundEffects,
    ImportType::Background { subtype } => SIT::Background(subtype),
    ImportType::Music => SIT::Music
  };

  let mime = mime_guess::from_path(filename).first_or_octet_stream().to_string();
  Ok(ImportResult {
    filename: filename.to_string(),
     file: File { binary: bytes, mime },
     specific_import_type
  })
}