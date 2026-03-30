use std::array::from_fn;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use itertools::Itertools;
use zip::ZipArchive;

use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::inter_stage_data::ImportFile;
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
      as_type: file.import_type
    });
    let kind = decide_specific_type::<T>(file.import_type, &file.path, &file.binary, &mut *guard).await?;
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
  (import_type: ImportType, path: &Path, bytes: &Arc<[u8]>, ctx: &mut ImportContext<'c, T>)
   -> Result<TypeStage1, ImportError<T>>
{
  ctx.log(LogLevel::Debug, format_args!("Deciding import type for {:?} {:?}", import_type, path));
  
  match import_type {
    ImportType::Automatic => {
      if let Some(filekey) = TypeStage1::parse_filekey(path) {
        ctx.log(LogLevel::Debug, format_args!("Filename has filekey for {filekey}"));
        return Ok(filekey);
      }
      
      let Some(file_type) = FileType::from_path(path) else {
        ctx.log(LogLevel::Info, "No known general media type indication from filename, import type unknown at this point.");
        return Ok(TypeStage1::Unknown);
      };
      
      ctx.log(LogLevel::Info, format_args!("Filename indicates general media type of {file_type}"));
      let guessed_type = match file_type {
        FileType::PackJson => TypeStage1::PackJson,
        FileType::Zip => TypeStage1::Zip,
        FileType::TPSE => TypeStage1::TPSE,
        FileType::Image => {
          let image = T::Texture::decode_texture(bytes.clone()).wrap(err!(ctx, tex))?;
          let width = image.width().await.wrap(err!(ctx, tex))?;
          let height: u32 = image.height().await.wrap(err!(ctx, tex))?;
          ctx.log(LogLevel::Info, format_args!("Image file is {width}x{height}"));
          match &guess_texture_format(path, width, height, &ctx)[..] {
            [] => {
              ctx.log(LogLevel::Info, "No known texture determineable from image dimensions and extension, assuming texture is a custom background");
              match path.extension().and_then(|ext| ext.to_str()) {
                Some(str) if str == "gif" => TypeStage1::Background { subtype: BackgroundType::Video },
                _ => TypeStage1::Background { subtype: BackgroundType::Image }
              }
            }
            [TextureGuess { kind: TextureGuessKind::Skin(single_format), .. }] => {
              ctx.log(LogLevel::Info, format_args!("Based on image dimensions and extension, uniquely inferred texture of type {single_format}"));
              TypeStage1::Skin { subtype: *single_format }
            },
            [TextureGuess { kind: TextureGuessKind::Other(single_format), .. }] => {
              ctx.log(LogLevel::Info, format_args!("Based on image dimensions and extension, uniquely inferred texture of type {single_format}"));
              TypeStage1::OtherSkin { subtype: *single_format }
            },
            [first_guess, other_guesses@..] => {
              ctx.log(LogLevel::Info, format_args!(
                "Multiple possible formats based on image dimensions and extension; specific type will be inferred later during type reduction from possibilities: {}",
                [*first_guess].iter().chain(other_guesses.iter()).map(|x| &x.kind).format(", ")
              ));
              TypeStage1::WeakTexture {
                first_guess: *first_guess,
                other_guesses: from_fn(|i| other_guesses.get(i).copied())
              }
            },
          }
        },
        FileType::Video => TypeStage1::Background { subtype: BackgroundType::Video },
        FileType::Audio => {
          let asset = ctx.provide_asset(Asset::TetrioRSD).await?;
          let rsd = parse_radiance_sound_definition(&asset).wrap(err!(ctx))?;
          let atlas = rsd.to_old_style_atlas();
          let sfx = PathBuf::from(path).file_stem().and_then(|ext| ext.to_str()).and_then(|ext| atlas.get(ext));
          match sfx {
            Some(_) => {
              ctx.log(LogLevel::Info, "Audio filename corresponds to a known TETR.IO sound effect. Assuming audio file is a custom sound effect.");
              TypeStage1::SoundEffects
            },
            None => {
              ctx.log(LogLevel::Info, "Audio filename corresponds to no known TETR.IO sound effect. Specific type will be inferred later during type reduction.");
              TypeStage1::WeakAudio
            }
          }
        }
      };
      ctx.log(LogLevel::Info, format_args!("Guessed import type {guessed_type}"));
      ctx.flags.guessed_files.insert(path.to_path_buf(), guessed_type.clone());
      Ok(guessed_type)
    },
    rest => Ok(TypeStage1::try_from(rest).expect("all stage 0 types should be handled"))
  }
}

/// A broad category of content based on a file extension
#[derive(strum::Display)]
enum FileType {
  #[strum(to_string = "pack.json")]
  PackJson,
  #[strum(to_string = "zip")]
  Zip,
  #[strum(to_string = "tpse")]
  TPSE,
  #[strum(to_string = "image")]
  Image,
  #[strum(to_string = "video")]
  Video,
  #[strum(to_string = "audio")]
  Audio
}
impl FileType {
  pub fn from_path(filename: &Path) -> Option<FileType> {
    if filename.file_name()?.to_str()? == "pack.json" {
      return Some(Self::PackJson);
    }
    let ext = Path::new(&filename).extension()?.to_str()?;
    match ext {
      "zip" => Some(FileType::Zip),
      "tpse" => Some(FileType::TPSE),
      "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" => Some(FileType::Image),
      "mp4" | "webm" => Some(FileType::Video),
      "ogg" | "mp3" | "flac" => Some(FileType::Audio),
      _ => return None
    }
  }
}