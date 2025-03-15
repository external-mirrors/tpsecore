use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::time::Duration;
use hound::{SampleFormat, WavSpec};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, Delay, DynamicImage, Frame};
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::imageops::FilterType;
use log::Level;
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};
use zip::read::ZipFile;
use zip::ZipArchive;
use crate::import::import_task::ImportTask;
use crate::import::{Asset, import, ImportErrorType, ImportContext, ImportType, LoadError, OtherSkinType, SkinType, SpecificImportType, ImportContextEntry, ImportError};
use crate::import::decode_helper::decode;
use crate::import::LoadError::{NoSupportedAudioTrack, SymphoniaError};
use crate::import::skin_splicer::{decode_image, SkinSplicer};
use crate::import::tetriojs::custom_sound_atlas;
use crate::tpse::{AnimMeta, Background, File, MiscTPSEValue, Song, SongMetadata, TPSE};

/// Executes an import task
pub fn execute_task(task: ImportTask, ctx: ImportContext<'_>) -> Result<TPSE, ImportError> {
  ctx.log(Level::Info, format_args!("Executing import task {:?}", task));
  let mut tpse = TPSE::default();
  match task {
    ImportTask::AnimatedSkinFrames(skin_type, frames) => {
      let frames = frames
        .into_iter()
        .enumerate()
        .flat_map(|(i, frame)| {
          let ctx = ctx.with_context(ImportContextEntry::FrameSource(i, frame.filename));
          let ctx2 = ctx.clone();
          let img = match decode_image(&frame.file.binary) {
            Err(err) => {
              let res = Err(ctx.wrap(err.into()));
              return Box::new(std::iter::once(res)) as Box<dyn Iterator<Item = _>>
            },
            Ok(img) => img
          };
          let iter = match frame.file.mime.as_str() {
            "image/gif" => {
              GifDecoder::new(Cursor::new(frame.file.binary))
                .map_err(Into::into)
                .map(move |decoder| decoder.into_frames().map(move |r| {
                  r.map_err(|e| ctx.wrap(LoadError::ImageError(e).into()))
                }))
                .map(|iter| Box::new(iter) as Box<dyn Iterator<Item = _>>)
            }
            "image/webp" => {
              WebPDecoder::new(Cursor::new(frame.file.binary))
                .map_err(Into::into)
                .map(move |decoder| decoder.into_frames().map(move |r| {
                  r.map_err(|e| ctx.wrap(LoadError::ImageError(e).into()))
                }))
                .map(|iter| Box::new(iter) as Box<dyn Iterator<Item = _>>)
            },
            _ => { // other single frame image
              let res = decode_image(&frame.file.binary)
                .map(|res| {
                  let delay = Delay::from_saturating_duration(Duration::from_millis(1000/30));
                  Frame::from_parts(res.into_rgba8(), 0, 0, delay)
                })
                .map_err(|err| ctx2.wrap(err.into()));
              Ok(Box::new(std::iter::once(res)) as Box<dyn Iterator<Item = _>>)
            }
          };
          match iter {
            Err(err) => Box::new(std::iter::once(Err(ctx2.wrap(err)))),
            Ok(iter) => iter
          }
        })
        .collect::<Result<Vec<Frame>, ImportError>>()?;

      // Yay, no more arbitrary canvas size restrictions!
      // Note: tetrio plus forces non-UHD HD texture res when using animated skins just because
      // they get absurdly huge and the 4x savings is worth it. Each frame is always 1024x1024.
      // tetrio plus also assumes animated textures sprite sheets wrap every 16 frames.
      let frame_count = frames.len();
      let width = 1024 * (frame_count as u32).min(16);
      let height = 1024 * ((frame_count + 15) / 16) as u32; // ceiling division
      let block_size = 48; // at HD/1024x1024 resolution
      let mut mino_canvas: Option<DynamicImage> = None;
      let mut ghost_canvas: Option<DynamicImage> = None;
      for (i, frame) in frames.iter().enumerate() {
        let mut source = SkinSplicer::default();
        source.load_decoded(skin_type, frame.buffer().clone().into());
        let groups = [
          (source.convert(SkinType::Tetrio61Connected, Some(block_size)), &mut mino_canvas),
          (source.convert(SkinType::Tetrio61ConnectedGhost, Some(block_size)), &mut ghost_canvas)
        ];
        for (frame, canvas) in groups {
          if let Some(frame) = frame {
            let canvas = canvas.get_or_insert_with(|| DynamicImage::new_rgb8(width, height));
            let x = (i % 16) as i64 * 1024;
            let y = (i / 16) as i64 * 1024;
            image::imageops::overlay(canvas, &frame, x, y);
          }
        }
      }
      let delay = skin_type.get_anim_options().delay.unwrap_or_else(|| {
        let milliseconds_per_image_frame = frames.iter()
          .map(|frame| {
            let (num, denom) = frame.delay().numer_denom_ms();
            return num as f64 / denom as f64;
          })
          .min_by(|left, right| left.partial_cmp(right).expect("float comparison failed"))
          .expect("There should be at least one frame");
        let milliseconds_per_game_frame = 1000.0 / 60.0;
        let game_frame_per_image_frame = milliseconds_per_image_frame / milliseconds_per_game_frame;
        game_frame_per_image_frame as u32
      });

      // Back to normal 96px blocks for non-animated skin fallback

      let uhd_block_size = 96;
      if let Some(mino_canvas) = mino_canvas {
        tpse.skin_anim = Some(mino_canvas.into());
        tpse.skin_anim_meta = Some(AnimMeta { frames: frames.len() as u32, delay });

        let mut source = SkinSplicer::default();
        let first_frame = frames.first().expect("There should be at least one frame");
        source.load_decoded(skin_type, first_frame.buffer().clone().into());
        let skin = source.convert(SkinType::Tetrio61Connected, Some(uhd_block_size));
        tpse.skin = Some(skin.expect("Skin should exist if animated buffer was created").into());
      }

      if let Some(ghost_canvas) = ghost_canvas {
        tpse.ghost_anim = Some(ghost_canvas.into());
        tpse.ghost_anim_meta = Some(AnimMeta { frames: frames.len() as u32, delay });

        let mut source = SkinSplicer::default();
        let first_frame = frames.first().expect("There should be at least one frame");
        source.load_decoded(skin_type, first_frame.buffer().clone().into());
        let ghost = source.convert(SkinType::Tetrio61ConnectedGhost, Some(uhd_block_size));
        tpse.ghost = Some(ghost.expect("Skin should exist if animated buffer was created").into());
      }
    },
    ImportTask::SoundEffects(sound_effects) => {
      // todo: probably not safe to assume 2 channel 44.1KHz audio
      let channels: usize = 2;
      let sample_rate: usize = 44100;
      let bits_per_sample: usize = 32;
      // Atlas entries are in floating point milliseconds, but we primarily work in samples here.
      // Multiply by this constant to get from atlas timings to sample timings.
      let atlas_entry_to_sample_ratio: f64 = 1.0/1000.0 * sample_rate as f64 * channels as f64;

      let tetrio_js = ctx.asset_source.provide(Asset::TetrioJS).map_err(|err| ctx.wrap(err))?;
      let mut atlas = custom_sound_atlas(tetrio_js).map_err(|err| ctx.wrap(err.into()))?;
      let mut unvisited = atlas.keys().cloned().collect::<HashSet<_>>();
      let mut encoded = vec![];
      let mut cursor = Cursor::new(&mut encoded);
      // todo: this is a wav encoder, but I can't find a wasm-compatible rust ogg encoder.
      // hopefully tetrio won't care?
      let mut encoder = hound::WavWriter::new(&mut cursor, WavSpec {
        channels: channels as u16,
        sample_rate: sample_rate as u32,
        bits_per_sample: bits_per_sample as u16,
        sample_format: SampleFormat::Float
      }).unwrap();
      let mut encoder_position = 0;

      for sfx in sound_effects {
        let with_filekey_removed = sfx.name.replace(ImportType::SoundEffects.filekey(), "");
        let Some((offset, duration)) = atlas.get_mut(&with_filekey_removed) else {
          ctx.log(Level::Warn, format_args!("Skipping unknown sound effect {}", sfx.name));
          continue;
        };
        unvisited.remove(&with_filekey_removed);

        ctx.log(Level::Trace, format_args!("Decoding {}: {} bytes", sfx.filename, sfx.file.binary.len()));

        let mut sample_duration: usize = 0;
        decode(&sfx.file.binary, sfx.extension().as_ref().map(|el| el.as_str()), |samples| {
          assert!(samples.len() % 2 == 0);
          for sample in samples { encoder.write_sample(*sample).unwrap(); }
          sample_duration += samples.len();
        }).map_err(|err| ctx.wrap(err))?;
        *offset = encoder_position as f64 / atlas_entry_to_sample_ratio;
        *duration = sample_duration as f64 / atlas_entry_to_sample_ratio;
        encoder_position += sample_duration;
      }

      if !unvisited.is_empty() {
        let tetrio_ogg = ctx.asset_source.provide(Asset::TetrioRSD).map_err(|err| ctx.wrap(err))?;

        ctx.log(Level::Trace, format_args!("Decoding tetrio.ogg: {} bytes", tetrio_ogg.len()));
        let mut decoded = Vec::with_capacity(546 * sample_rate * channels);
        decode(tetrio_ogg, Some("ogg"), |samples| decoded.extend_from_slice(samples)).map_err(|err| ctx.wrap(err))?;


        ctx.log(Level::Trace, format_args!("Encoding..."));
        for sfx_name in unvisited {
          let (offset, duration) = atlas.get_mut(&sfx_name).unwrap();
          let offset_samples = (*offset * atlas_entry_to_sample_ratio) as usize;
          let duration_samples = (*duration * atlas_entry_to_sample_ratio) as usize;
          let samples = &decoded[offset_samples..offset_samples + duration_samples];
          for sample in samples { encoder.write_sample(*sample).unwrap(); }
          *offset = encoder_position as f64 / atlas_entry_to_sample_ratio;
          *duration = samples.len() as f64 / atlas_entry_to_sample_ratio;
          encoder_position += samples.len();
        }
      }

      if let Err(err) = encoder.finalize() {
        log::error!("non-fatal encoding error: {err}")
      }
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

          let mut groups: HashMap<String, Vec<(ImportType, String, Vec<u8>)>> = HashMap::new();
          let zip = ZipArchive::new(Cursor::new(&file.binary))
            .map_err(LoadError::from)
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
                file.read_to_end(&mut bytes);
                bytes
              }
            ));
          }
          for (folder, files) in groups {
            let files = files.iter()
              .map(|(it, name, bytes)| (*it, name.as_ref(), bytes.as_ref()))
              .collect::<Vec<_>>();
            let context = ctx.with_context(ImportContextEntry::ZipFolder(folder));
            let new_tpse = import(files, context)?;
            tpse.merge(new_tpse);
          }
        },
        SpecificImportType::TPSE => {
          tpse.merge(serde_json::from_slice(&file.binary).map_err(|err| {
            ImportErrorType::InvalidTPSE(err.to_string())
          }).map_err(|err| ctx.wrap(err))?);
        },
        SpecificImportType::Skin(skin_type) => {
          let (minos, ghost) = splice_to_t61(skin_type, &file.binary).map_err(|err| ctx.wrap(err))?;
          if let Some(minos) = minos { tpse.skin = Some(minos.into()); }
          if let Some(ghost) = ghost { tpse.ghost = Some(ghost.into()); }
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

fn splice_to_t61(skin_type: SkinType, bytes: &[u8])
  -> Result<(Option<DynamicImage>, Option<DynamicImage>), ImportErrorType>
{
  let target_resolution = 96;
  let mut source = SkinSplicer::default();
  source.load(skin_type, bytes)?;
  let minos = source.convert(SkinType::Tetrio61Connected, Some(target_resolution));
  let ghost = source.convert(SkinType::Tetrio61ConnectedGhost, Some(target_resolution));
  Ok((minos, ghost))
}