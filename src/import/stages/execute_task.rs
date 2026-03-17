use std::io::Read;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use log::Level;
use zip::ZipArchive;
use crate::accel::traits::{AssetProvider, TPSEAccelerator, TextureHandle, AudioHandle};
use crate::import::import_task::ImportTask;
use crate::import::{Asset, ImportContext, ImportContextEntry, ImportError, ImportErrorType, ImportType, MediaLoadError, SkinType, SpecificImportType, import};
use crate::import::radiance::parse_radiance_sound_definition;
use crate::import::skin_splicer::{SkinSplicer};
use crate::tpse::tpse_key::merge;
use crate::tpse::{AnimMeta, Background, File, MiscTPSEValue, Song, SongMetadata, TPSE};

/// Executes an import task
pub async fn execute_task<T: TPSEAccelerator>(task: ImportTask, ctx: &ImportContext<'_, T>) -> Result<TPSE, ImportError<T>> {
  ctx.log(Level::Info, format_args!("Executing import task {:?}", task));
  let mut tpse = TPSE::default();
  match task {
    ImportTask::AnimatedSkinFrames(skin_type, frames) => {
      let mut decoded_frames = vec![];
      for (i, frame) in frames.into_iter().enumerate() {
        let ctx = ctx.with_context(ImportContextEntry::FrameSource(i, frame.filename));
        let decoded = match frame.file.mime.as_str() {
          #[cfg(feature = "extra_software_decoders")]
          "image/gif" => {
            crate::accel::extra_software_decoders::decode_gif::<T>(&frame.file.binary)
              .map_err(|err| ctx.wrap(ImportErrorType::LoadError(err.into()).into()))?
          }
          #[cfg(feature = "extra_software_decoders")]
          "image/webp" => {
            crate::accel::extra_software_decoders::decode_webp::<T>(&frame.file.binary)
              .map_err(|err| ctx.wrap(ImportErrorType::LoadError(err.into()).into()))?
          }
          // other single frame image
          _ => T::Texture::decode_texture(frame.file.binary)
            .map(|frame| vec![(frame, Duration::from_secs(1))])
            .map_err(|err| ctx.wrap(MediaLoadError::TextureError(err).into()))?
        };
        decoded_frames.extend(decoded)
      };
      let frames = decoded_frames;

      // Yay, no more arbitrary canvas size restrictions!
      // Note: tetrio plus forces non-UHD HD texture res when using animated skins just because
      // they get absurdly huge and the 4x savings is worth it. Each frame is always 1024x1024.
      // tetrio plus also assumes animated textures sprite sheets wrap every 16 frames.
      let frame_count = frames.len();
      let width = 1024 * (frame_count as u32).min(16);
      let height = 1024 * ((frame_count + 15) / 16) as u32; // ceiling division
      let block_size = 48; // at HD/1024x1024 resolution
      let mut mino_canvas: Option<T::Texture> = None;
      let mut ghost_canvas: Option<T::Texture> = None;
      for (i, (texture, _)) in frames.iter().enumerate() {
        let mut source = SkinSplicer::<T>::default();
        source.load_decoded(skin_type, texture.create_copy());
        let groups = [
          (
            source.convert(SkinType::Tetrio61Connected, Some(block_size)).await
              .map_err(|err| ctx.wrap(MediaLoadError::TextureError(err).into()))?,
           &mut mino_canvas
          ),
          (
            source.convert(SkinType::Tetrio61ConnectedGhost, Some(block_size)).await
              .map_err(|err| ctx.wrap(MediaLoadError::TextureError(err).into()))?, 
            &mut ghost_canvas
          )
        ];
        for (frame, canvas) in groups {
          if let Some(frame) = frame {
            let canvas = canvas.get_or_insert_with(|| T::Texture::new_texture(width, height));
            let x = (i % 16) as i64 * 1024;
            let y = (i / 16) as i64 * 1024;
            canvas.overlay(&frame, x, y);
          }
        }
      }
      let delay = skin_type.get_anim_options().delay.unwrap_or_else(|| {
        frames.iter()
          .map(|(_, delay)| {
            let milliseconds_per_image_frame = delay.as_secs_f64() * 1000.0;
            let milliseconds_per_game_frame = 1000.0 / 60.0;
            let game_frame_per_image_frame = milliseconds_per_image_frame / milliseconds_per_game_frame;
            game_frame_per_image_frame as u32
          })
          .min()
          .expect("There should be at least one frame")
      });

      // Back to normal 96px blocks for non-animated skin fallback

      let uhd_block_size = 96;
      let instances = [
        (mino_canvas, &mut tpse.skin_anim, &mut tpse.skin_anim_meta, &mut tpse.skin, SkinType::Tetrio61Connected),
        (ghost_canvas, &mut tpse.ghost_anim, &mut tpse.ghost_anim_meta, &mut tpse.ghost, SkinType::Tetrio61Ghost),
      ];
      
      for (canvas, skin_anim, skin_anim_meta, skin, skin_type) in instances {
        if let Some(canvas) = canvas {
          *skin_anim = Some(File {
            binary: canvas.encode_png().await.map_err(|err| ctx.wrap(ImportErrorType::TextureEncodeFailed(err)))?,
            mime: "image/png".to_string()
          });
          *skin_anim_meta = Some(AnimMeta { frames: frames.len() as u32, delay });

          let mut source = SkinSplicer::<T>::default();
          let (first_frame, _) = frames.first().expect("There should be at least one frame");
          source.load_decoded(skin_type, first_frame.clone());
          let first_frame_skin = source
            .convert(skin_type, Some(uhd_block_size))
            .await
            .map_err(|err| ctx.wrap(MediaLoadError::TextureError(err).into()))?
            .expect("Skin should exist if animated buffer was created");
          *skin = Some(File {
            binary: first_frame_skin.encode_png().await
              .map_err(|err| ctx.wrap(ImportErrorType::TextureEncodeFailed(err)))?,
            mime: "image/png".to_string()
          });
        }
      }
    },
    ImportTask::SoundEffects(sound_effects) => {
      let channels: usize = 2;
      let sample_rate: usize = 44100;
      // Atlas entries are in floating point milliseconds, but we primarily work in samples here.
      // Multiply by this constant to get from atlas timings to sample timings.
      let atlas_entry_to_sample_ratio: f64 = 1.0/1000.0 * sample_rate as f64 * channels as f64;

      let asset = ctx.asset_source.provide(Asset::TetrioRSD).await
        .map_err(|err| ctx.wrap(ImportErrorType::AssetFetchFailed(err)))?;
      let rsd = parse_radiance_sound_definition(&asset).map_err(|err| ctx.wrap(err.into()))?;
      
      let mut atlas = rsd.to_old_style_atlas();
      let mut unvisited = atlas.keys().cloned().collect::<HashSet<_>>();
      let mut encoding_queue = Vec::with_capacity(sound_effects.len());
      let mut encoder_position = 0;
      
      for sfx in &sound_effects {
        // todo: ensure we've stripped file extension by this point
        let with_filekey_removed = sfx.name.replace(ImportType::SoundEffects.filekey(), "");
        let Some((offset, duration)) = atlas.get_mut(&with_filekey_removed) else {
          ctx.log(Level::Warn, format_args!("Skipping unknown sound effect {}", sfx.name));
          continue;
        };
        unvisited.remove(&with_filekey_removed);

        ctx.log(Level::Trace, format_args!("Decoding {}: {} bytes", sfx.filename, sfx.file.binary.len()));

        let handle = T::Audio::decode_audio(sfx.file.binary.clone(), Some(&sfx.file.mime)).await
          .map_err(|err| ctx.wrap(MediaLoadError::AudioError(err).into()))?;
        let samples = handle.length().await
          .map_err(|err| ctx.wrap(MediaLoadError::AudioError(err).into()))?;
        encoding_queue.push(handle);
        
        assert!(samples % 2 == 0);
        
        *offset = encoder_position as f64 / atlas_entry_to_sample_ratio;
        *duration = samples as f64 / atlas_entry_to_sample_ratio;
        encoder_position += samples;
      }
      
      if !unvisited.is_empty() {
        let rsd_asset = ctx.asset_source.provide(Asset::TetrioRSD).await
          .map_err(|err| ctx.wrap(ImportErrorType::AssetFetchFailed(err)))?;
          
        let rsd = parse_radiance_sound_definition(&rsd_asset)
          .map_err(|err| ctx.wrap(err.into()))?;

        ctx.log(Level::Trace, format_args!("Decoding {}: {} bytes", Asset::TetrioRSD, rsd_asset.len()));

        let handle = T::Audio::decode_audio(rsd.audio_buffer.into(), Some("audio/ogg")).await
          .map_err(|err| ctx.wrap(ImportErrorType::AssetSoundEffectsDecode(err).into()))?;
          
        for sfx_name in unvisited {
          let (offset, duration) = atlas.get_mut(&sfx_name).unwrap();
          let offset_samples = (*offset * atlas_entry_to_sample_ratio) as usize;
          let duration_samples = (*duration * atlas_entry_to_sample_ratio) as usize;
          let subhandle = handle.slice(offset_samples..offset_samples + duration_samples);
          encoding_queue.push(subhandle);
          *offset = encoder_position as f64 / atlas_entry_to_sample_ratio;
          *duration = duration_samples as f64 / atlas_entry_to_sample_ratio;
          encoder_position += duration_samples;
        }
      }
      
      ctx.log(Level::Trace, format_args!("Encoding..."));
      let encoded = T::Audio::encode_ogg(&encoding_queue).await
        .map_err(|err| ctx.wrap(ImportErrorType::AudioEncodeFailed(err).into()))?;

      tpse.custom_sounds = Some(File {
        binary: encoded,
        mime: "audio/wav".to_string()
      });
      tpse.custom_sound_atlas = Some(atlas);
    },
    ImportTask::Basic { import_type, filename, file } => {
      match import_type {
        SpecificImportType::Zip => {
          // todo: optimeiz

          let mut groups: HashMap<String, Vec<(ImportType, String, Arc<[u8]>)>> = HashMap::new();
          let zip = ZipArchive::new(Cursor::new(&file.binary))
            .map_err(MediaLoadError::from)
            .map_err(|err| ctx.wrap(err.into()))?;
          for i in 0..zip.len() {
            let mut zip = zip.clone();
            let mut file = zip.by_index(i).unwrap();
            if !file.is_file() {
              continue;
            }
            let (folder, filename) = file.name().rsplit_once("/").unwrap_or(("", file.name()));
            groups.entry(folder.to_string()).or_default().push((
              ImportType::Automatic,
              filename.to_string(),
              {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).map_err(|err| ctx.wrap(MediaLoadError::Zip(err.into()).into()))?;
                bytes.into()
              }
            ));
          }
          for (folder, files) in groups {
            let files = files.iter()
              .map(|(it, name, bytes)| (*it, name.as_ref(), bytes.clone()))
              .collect::<Vec<_>>();
            let context = ctx.with_context(ImportContextEntry::ZipFolder(folder));
            let new_tpse = Box::pin(import::<T>(files, context)).await?;
            merge(&mut tpse, &new_tpse).await.map_err(|err| ctx.wrap(err.into()))?;
          }
        },
        SpecificImportType::TPSE => {
          let new_tpse: TPSE = serde_json::from_slice(&file.binary)
            .map_err(|err| ctx.wrap(ImportErrorType::InvalidTPSE(err.to_string())))?;
          merge(&mut tpse, &new_tpse).await.map_err(|err| ctx.wrap(err.into()))?;
        },
        SpecificImportType::Skin(skin_type) => {
          let (minos, ghost) = splice_to_t61::<T>(skin_type, file.binary.clone())
            .await.map_err(|err| ctx.wrap(err.into()))?;
          if let Some(minos) = minos {
            tpse.skin = Some(File {
              binary: minos.encode_png().await.unwrap(),
              mime: "image/png".to_string()
            });
          }
          if let Some(ghost) = ghost {
            tpse.ghost = Some(File {
              binary: ghost.encode_png().await.unwrap(),
              mime: "image/png".to_string()
            });
          }
        },
        SpecificImportType::OtherSkin(skin_type) => {
          skin_type.tpse_field(&mut tpse).replace(file);
        },
        SpecificImportType::SoundEffects => {
          unreachable!()
        },
        SpecificImportType::Background(bg_type) => {
          let id = format!("background-{}", file.sha256_hex());
          tpse.other.insert(id.clone(), MiscTPSEValue::File(file));
          let bg = Background { id, filename, background_type: bg_type.into() };
          tpse.backgrounds.get_or_insert(Default::default()).push(bg);
        },
        SpecificImportType::Music => {
          let id = format!("song-{}", file.sha256_hex());
          tpse.other.insert(id.clone(), MiscTPSEValue::File(file));
          let song = Song {
            id,
            filename: filename.clone(),
            song_override: None,
            metadata: SongMetadata {
              name: filename,
              ..Default::default()
            }
          };
          tpse.music.get_or_insert(Default::default()).push(song);
        },
      }
    }
  };
  log::trace!("Done executing import task");
  Ok(tpse)
}

async fn splice_to_t61<T: TPSEAccelerator>(skin_type: SkinType, bytes: Arc<[u8]>)
  -> Result<(Option<T::Texture>, Option<T::Texture>), MediaLoadError<T>>
{
  let target_resolution = 96;
  let mut source = SkinSplicer::<T>::default();
  source.load(skin_type, bytes)
    .map_err(|err| MediaLoadError::TextureError(err))?;
  let minos = source.convert(SkinType::Tetrio61Connected, Some(target_resolution)).await
    .map_err(|err| MediaLoadError::TextureError(err))?;
  let ghost = source.convert(SkinType::Tetrio61ConnectedGhost, Some(target_resolution)).await
    .map_err(|err| MediaLoadError::TextureError(err))?;
  Ok((minos, ghost))
}