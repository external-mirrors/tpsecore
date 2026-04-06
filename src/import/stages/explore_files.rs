use std::array::from_fn;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use itertools::Itertools;
use zip::ZipArchive;

use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::inter_stage_data::{FileType, ImportFile};
use crate::import::{Asset, BackgroundType, ImportContext, ImportContextEntry, ImportError, ImportErrorType, ImportErrorWrapHelper, ImportType, TextureGuess, TextureGuessKind, TypeStage1, TypeStage2, err, guess_texture_format};
use crate::import::radiance::parse_radiance_sound_definition;
use crate::log::LogLevel;


pub async fn explore_files<T: TPSEAccelerator>
  (queue: Vec<ImportFile<ImportType>>, ctx: &mut ImportContext<'_, T>)
   -> Result<Vec<ImportFile<TypeStage2>>, ImportError<T>>
{
  if ctx.is_too_deep() {
    return Err(ctx.wrap_error(ImportErrorType::TooMuchNesting))
  }
  
  let mut results = vec![];
  for file in queue {
    let mut guard = ctx.enter_context(ImportContextEntry::ImportFile {
      file: file.path.clone(),
      as_type: file.import_type.clone()
    });
    let kind = decide_specific_type::<T>(&file.import_type, &file.path, &file.binary, &mut *guard).await?;
    match kind {
      TypeStage1::Zip => {
        let mut zip = ZipArchive::new(Cursor::new(&file.binary)).wrap(err!(guard, zip))?;
        let mut subqueue = vec![];
        for i in 0..zip.len() {
          let mut entry = zip.by_index(i).wrap(err!(guard, zip))?;
          if !entry.is_file() { continue }
          let mut bytes = Vec::with_capacity(entry.size() as usize);
          entry.read_to_end(&mut bytes).wrap(err!(guard, zip))?;
          subqueue.push(ImportFile {
            import_type: ImportType::Automatic,
            path: file.path.join(entry.mangled_name()),
            binary: bytes.into()
          });
        }
        results.extend(Box::pin(explore_files(subqueue, &mut *guard)).await?);
      }
      other => {
        results.push(ImportFile {
          import_type: ImportType::from(other).try_into().expect("all stage 1 types should be handled"),
          path: file.path,
          binary: file.binary
        });
      }
    }
  }
  // for later heuristic stability, sort files by path now
  results.sort_by(|a, b| a.path.cmp(&b.path));
  Ok(results)
}

async fn decide_specific_type<'c, T: TPSEAccelerator>
  (import_type: &ImportType, path: &Path, bytes: &Arc<[u8]>, ctx: &mut ImportContext<'c, T>)
   -> Result<TypeStage1, ImportError<T>>
{
  {ctx.log(LogLevel::Debug, format_args!("Deciding import type for {:?} {:?}", import_type, path))}.await;
  
  match import_type {
    ImportType::Automatic => {
      if let Some(filekey) = TypeStage1::parse_filekey(path) {
        {ctx.log(LogLevel::Debug, format_args!("Filename has filekey for {filekey}"))}.await;
        return Ok(filekey);
      }
      
      let Some(file_type) = FileType::from_path(path) else {
        ctx.log(LogLevel::Info, "No known general media type indication from filename, import type unknown at this point.").await;
        return Ok(TypeStage1::Unknown);
      };
      
      let mut ctx = ctx.enter_context(ImportContextEntry::WithGeneralMediaType { file_type });
      let guessed_type = match file_type {
        FileType::PackJson => TypeStage1::PackJson,
        FileType::Zip => TypeStage1::Zip,
        FileType::TPSE => TypeStage1::TPSE,
        FileType::Image => {
          let image = T::Texture::decode_texture(bytes.clone()).wrap(err!(ctx, tex))?;
          let width = image.width().await.wrap(err!(ctx, tex))?;
          let height: u32 = image.height().await.wrap(err!(ctx, tex))?;
          let ctx = ctx.enter_context(ImportContextEntry::WithImageDimensions { width, height });
          match &guess_texture_format(path, width, height, &ctx).await[..] {
            [] => {
              ctx.log(LogLevel::Info, "No known texture determineable from image dimensions and extension, assuming texture is a custom background").await;
              match path.extension().and_then(|ext| ext.to_str()) {
                Some(str) if str == "gif" => TypeStage1::Background { subtype: BackgroundType::Video },
                _ => TypeStage1::Background { subtype: BackgroundType::Image }
              }
            }
            [TextureGuess { kind: TextureGuessKind::Skin(single_format), .. }] => {
              {ctx.log(LogLevel::Info, format_args!("Guessed import type {single_format}"))}.await;
              TypeStage1::Skin { subtype: *single_format }
            },
            [TextureGuess { kind: TextureGuessKind::Other(single_format), .. }] => {
              {ctx.log(LogLevel::Info, format_args!("Guessed import type {single_format}"))}.await;
              TypeStage1::OtherSkin { subtype: *single_format }
            },
            [first_guess, other_guesses@..] => {
              // Messages later in the pipeline will alert about the actual inferred type or a failure to infer
              // No info level log needed here
              {ctx.log(LogLevel::Debug, format_args!(
                "Multiple possible formats based on image dimensions and extension; specific type will be inferred later during type reduction from possibilities: {}",
                [*first_guess].iter().chain(other_guesses.iter()).map(|x| &x.kind).format(", ")
              ))}.await;
              TypeStage1::WeakTexture {
                first_guess: *first_guess,
                other_guesses: from_fn(|i| other_guesses.get(i).copied())
              }
            },
          }
        },
        FileType::Video => {
          ctx.log(LogLevel::Info, "Guessed import type video background").await;
          TypeStage1::Background { subtype: BackgroundType::Video }
        },
        FileType::Audio => {
          let asset = ctx.provide_asset(Asset::TetrioRSD).await?;
          let rsd = parse_radiance_sound_definition(&asset).wrap(err!(ctx))?;
          let atlas = rsd.to_old_style_atlas();
          let sfx = PathBuf::from(path).file_stem().and_then(|ext| ext.to_str()).and_then(|ext| atlas.get(ext));
          match sfx {
            Some(_) => {
              // No info level log needed here, this is a strong indicator of type
              ctx.log(LogLevel::Debug, "Audio filename corresponds to a known TETR.IO sound effect. Assuming audio file is a custom sound effect.").await;
              TypeStage1::SoundEffects
            },
            None => {
              // Same as above, messages later in the pipeline will notify specifics
              ctx.log(LogLevel::Debug, "Audio filename corresponds to no known TETR.IO sound effect. Specific type will be inferred later during type reduction.").await;
              TypeStage1::WeakAudio
            }
          }
        }
      };
      ctx.flags.guessed_files.insert(path.to_path_buf(), guessed_type.clone());
      Ok(guessed_type)
    },
    rest => Ok(TypeStage1::try_from(rest.clone()).expect("all stage 0 types should be handled"))
  }
}