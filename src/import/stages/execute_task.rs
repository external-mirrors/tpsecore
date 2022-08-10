use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use hound::{SampleFormat, WavSpec};
use image::DynamicImage;
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
use crate::import::skin_splicer::SkinSplicer;
use crate::import::tetriojs::custom_sound_atlas;
use crate::tpse::{Background, File, MiscTPSEValue, Song, SongMetadata, TPSE};


/// Executes an import task
pub fn execute_task(task: ImportTask, ctx: ImportContext<'_>) -> Result<TPSE, ImportError> {
  log::debug!("Executing import task {:?}", task);
  let mut tpse = TPSE::default();
  match task {
    ImportTask::AnimatedSkinFrames(skin_type, frames) => todo!(),
    ImportTask::SoundEffects(sound_effects) => {
      let tetrio_js = ctx.asset_source.provide(Asset::TetrioJS).map_err(|err| ctx.wrap(err))?;
      let tetrio_ogg = ctx.asset_source.provide(Asset::TetrioOGG).map_err(|err| ctx.wrap(err))?;
      let mut atlas = custom_sound_atlas(tetrio_js).map_err(|err| ctx.wrap(err.into()))?;

      let mut encoded = vec![];
      // todo: probably not safe to assume 2 channel 44.1KHz audio
      let channels = 2;
      let sample_rate = 44100;
      let bits_per_sample = 32;
      let mut cursor = Cursor::new(&mut encoded);
      // todo: this is a wav encoder, but I can't find a wasm-compatible rust ogg encoder.
      // hopefully tetrio won't care?
      let mut encoder = hound::WavWriter::new(&mut cursor, WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float
      }).unwrap();
      let mut encoder_position = 0;

      let tetrio = Hint::new().with_extension("ogg");
      let mut stream = MediaSourceStream::new(Box::new(Cursor::new(tetrio_ogg.to_vec())), Default::default());
      let mut unvisited = atlas.keys().cloned().collect::<HashSet<_>>();
      let start = std::time::Instant::now();

      for sfx in sound_effects {
        let with_filekey_removed = sfx.name.replace(ImportType::SoundEffects.filekey(), "");
        let entry = match atlas.get_mut(&with_filekey_removed) {
          Some(entry) => entry,
          None => {
            log::warn!("Skipping unknown sound effect {}", sfx.name);
            continue;
          }
        };
        unvisited.remove(&sfx.name);
        let mut new_duration: usize = 0;
        log::trace!("Decoding {}: {} bytes (@ {:?})", sfx.filename, sfx.file.binary.len(), start.elapsed());
        decode(&sfx.file.binary, sfx.extension().as_ref().map(|el| el.as_str()), |samples| {
          for sample in samples { encoder.write_sample(*sample).unwrap(); }
          new_duration += samples.len() / channels;
        }).map_err(|err| ctx.wrap(err))?;
        entry.0 = encoder_position as f64 / 44100.0;
        entry.1 = new_duration as f64 / 44100.0;
        encoder_position += new_duration;
      }

      log::trace!("Decoding tetrio.ogg {} bytes (@ {:?})", tetrio_ogg.len(), start.elapsed());
      let mut decoded = Vec::with_capacity(546 * 44100 * 2);
      decode(tetrio_ogg, Some("ogg"), |samples| {
        decoded.extend_from_slice(samples);
      }).map_err(|err| ctx.wrap(err))?;

      log::trace!("Encoding... (@ {:?}", start.elapsed());
      for sfx_name in unvisited {
        let (offset, duration) = atlas.get_mut(&sfx_name).unwrap();
        let offset_samples = (*offset / 1000.0 * 44100.0) as usize;
        let duration_samples = (*duration / 1000.0 * 44100.0) as usize;
        let samples = &decoded[offset_samples..offset_samples + duration_samples];
        for sample in samples { encoder.write_sample(*sample).unwrap(); }
        *offset = encoder_position as f64 / 44100.0 * 1000.0;
        *duration = samples.len() as f64 / 44100.0 * 1000.0;
        encoder_position += samples.len() / channels;
      }

      encoder.finalize().unwrap();
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
          log::trace!("Splicing {:?} to t61", skin_type);
          let (minos, ghost) = splice_to_t61(skin_type, &file.binary).map_err(|err| ctx.wrap(err))?;
          log::trace!("Done splicing!");
          if let Some(minos) = minos { tpse.skin = Some(minos.into()); }
          if let Some(ghost) = ghost { tpse.ghost = Some(ghost.into()); }
          log::trace!("Done converting!");
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