use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zip::ZipArchive;

use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::inter_stage_data::{FileType, ProcessedQueuedFile, QueuedFile, SpecificImportType, SpecificImportTypeWithZip};
use crate::import::{Asset, BackgroundType, ImportContext, ImportContextEntry, ImportError, ImportErrorType, ImportErrorWrapHelper, ImportType, SkinType, err, MediaLoadError};
use crate::import::radiance::parse_radiance_sound_definition;
use crate::log::LogLevel;


pub async fn explore_files<T: TPSEAccelerator>
  (queue: Vec<QueuedFile>, ctx: &mut ImportContext<'_, T>)
   -> Result<Vec<ProcessedQueuedFile>, ImportError<T>>
{
  if ctx.is_too_deep() {
    return Err(ctx.wrap_error(ImportErrorType::TooMuchNesting))
  }
  
  let mut results = vec![];
  for file in queue {
    let mut guard = ctx.enter_context(ImportContextEntry::ImportFile { file: file.path.clone(), as_type: file.kind });
    let kind = decide_specific_type::<T>(file.kind, &file.path, &file.binary, &mut *guard).await?;
    match kind {
      SpecificImportTypeWithZip::Zip => {
        let mut zip = ZipArchive::new(Cursor::new(&file.binary)).wrap(err!(guard, zip))?;
        let mut subqueue = vec![];
        for i in 0..zip.len() {
          let mut entry = zip.by_index(i).wrap(err!(guard, zip))?;
          if !entry.is_file() { continue }
          let mut bytes = Vec::with_capacity(entry.size() as usize);
          entry.read_to_end(&mut bytes).wrap(err!(guard, zip))?;
          subqueue.push(QueuedFile {
            kind: ImportType::Automatic,
            path: file.path.join(entry.mangled_name()),
            binary: bytes.into()
          });
        }
        results.extend(Box::pin(explore_files(subqueue, &mut *guard)).await?);
      }
      SpecificImportTypeWithZip::Other(other_kind) => {
        results.push(ProcessedQueuedFile {
          specific_kind: other_kind,
          kind: file.kind,
          path: file.path,
          binary: file.binary
        });
      }
    }
  }
  Ok(results)
}

pub async fn decide_specific_type<'c, T: TPSEAccelerator>
  (import_type: ImportType, path: &Path, bytes: &Arc<[u8]>, ctx: &mut ImportContext<'c, T>)
   -> Result<SpecificImportTypeWithZip, ImportError<T>>
{
  use SpecificImportTypeWithZip as SITZ;
  use SpecificImportType as SIT;
  
  ctx.log(LogLevel::Debug, format_args!("Deciding import type for {:?} {:?}", import_type, path));

  let specific_import_type = match import_type {
    ImportType::Automatic => {
      if let Some(filekey) = ImportType::parse_filekey(path) {
        let mut guard = ctx.enter_context(ImportContextEntry::WithFilekey { filekey });
        return Box::pin(decide_specific_type::<T>(filekey, path, bytes, &mut *guard)).await;
      }
      let Some(guess) = FileType::from_extension(path) else {
        return Err(ctx.wrap_error(ImportErrorType::UnknownFileType));
      };
      
      let guessed_type = match guess {
        FileType::Zip => SITZ::Zip,
        FileType::TPSE => SITZ::Other(SIT::TPSE),
        FileType::Image => {
          let image = T::Texture::decode_texture(bytes.clone()).wrap(err!(ctx, tex))?;
          let width = image.width().await.wrap(err!(ctx, tex))?;
          let height: u32 = image.height().await.wrap(err!(ctx, tex))?;
          if let Some(format) = SkinType::guess_format(path, width, height, &ctx) {
            let format = ImportType::Skin { subtype: format };
            let mut guard = ctx.enter_context(ImportContextEntry::WithGuessedType { as_type: format });
            return Box::pin(decide_specific_type::<T>(format, path, bytes, &mut *guard)).await
          } else {
            match path.extension().and_then(|ext| ext.to_str()) {
              Some(str) if str == "gif" => SITZ::Other(SIT::Background(BackgroundType::Video)),
              _ => SITZ::Other(SIT::Background(BackgroundType::Image))
            }
          }
        },
        FileType::Video => SITZ::Other(SIT::Background(BackgroundType::Video)),
        FileType::Audio => {
          let asset = ctx.provide_asset(Asset::TetrioRSD).await?;
          let rsd = parse_radiance_sound_definition(&asset).wrap(err!(ctx))?;
          let atlas = rsd.to_old_style_atlas();
          let sfx = PathBuf::from(path).file_stem().and_then(|ext| ext.to_str()).and_then(|ext| atlas.get(ext));
          match sfx {
            Some(_) => SITZ::Other(SIT::SoundEffects),
            None => SITZ::Other(SIT::Music)
          }
        }
      };
      ctx.log(LogLevel::Info, format_args!("Guessed import type {guessed_type}"));
      ctx.flags.guessed_files.insert(path.to_path_buf(), guessed_type.clone());
      guessed_type
    },
    ImportType::Skin { subtype } => SITZ::Other(SIT::Skin(subtype)),
    ImportType::OtherSkin { subtype } => SITZ::Other(SIT::OtherSkin(subtype)),
    ImportType::SoundEffects => SITZ::Other(SIT::SoundEffects),
    ImportType::Background { subtype } => SITZ::Other(SIT::Background(subtype)),
    ImportType::Music => SITZ::Other(SIT::Music),
  };

  Ok(specific_import_type)
}