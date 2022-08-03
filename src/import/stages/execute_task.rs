use std::io::{Cursor, Seek};
use image::DynamicImage;
use ogg::PacketWriter;
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};
use crate::import::import_task::ImportTask;
use crate::import::{Asset, ImportErrorType, ImportOptions, OtherSkinType, SkinType, SpecificImportType};
use crate::import::LoadError::{NoSupportedAudioTrack, SymphoniaError};
use crate::import::skin_splicer::SkinSplicer;
use crate::import::tetriojs::custom_sound_atlas;
use crate::tpse::{Background, File, MiscTPSEValue, Song, SongMetadata, TPSE};

/// Executes an import task
pub fn execute_task(task: ImportTask, options: ImportOptions<'_>) -> Result<TPSE, ImportErrorType> {
  let mut tpse = TPSE::default();
  match task {
    ImportTask::AnimatedSkinFrames(skin_type, frames) => todo!(),
    ImportTask::SoundEffects(sound_effects) => {
      let js = options.asset_source.provide(Asset::TetrioJS)?;
      let ogg = options.asset_source.provide(Asset::TetrioOGG)?;
      let mut atlas = custom_sound_atlas(js)?;
      let mut encoded = vec![];
      let mut encoder = PacketWriter::new(&mut encoded);
      let mut position = 0;

      // todo: add default base game sound effects
      for sfx in sound_effects {
        let entry = match atlas.get_mut(&sfx.name) {
          Some(entry) => entry,
          None => {
            log::warn!("Skipping unknown sound effect {}", sfx.name);
            continue;
          }
        };

        let mut hint = Hint::new();
        if let Some(ext) = sfx.extension() {
          hint.with_extension(&ext);
        }
        let mut stream = MediaSourceStream::new(Box::new(Cursor::new(sfx.file.binary)), Default::default());
        let fmt_opts = FormatOptions { enable_gapless: true, ..Default::default() };
        let mut probe = get_probe()
          .format(&hint, stream, &fmt_opts, &Default::default())
          .map_err(|err| SymphoniaError(err))?;

        let track = probe.format.tracks().iter()
          .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
          .ok_or(NoSupportedAudioTrack)?;

        let track_id = track.id;

        let mut decoder = get_codecs()
          .make(&track.codec_params, &Default::default())
          .map_err(|err| SymphoniaError(err))?;

        loop {
          let packet = probe.format.next_packet().map_err(|err| SymphoniaError(err))?;
          if packet.track_id() != track_id { continue; }
          let decoded = decoder.decode(&packet).map_err(|err| SymphoniaError(err))?;
          let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
          sample_buf.copy_interleaved_ref(decoded);
          let samples = sample_buf.samples();

          // todo: figure out how to use this library
          // encoder.

          // todo: probably not safe to assume 2 channel 44.1KHz audio
          let length = samples.len() / 2;
          entry.0 = position as f64 / 44100.0;
          entry.1 = length as f64 / 44100.0;
          position += length;
        }
      }
    },
    ImportTask::Basic(specific_type, filename, file) => {
      match specific_type {
        SpecificImportType::Zip => todo!(),
        SpecificImportType::TPSE => {
          tpse.merge(serde_json::from_slice(&file.binary).map_err(|err| {
            ImportErrorType::InvalidTPSE(err.to_string())
          })?);
        },
        SpecificImportType::Skin(skin_type) => {
          let (minos, ghost) = splice_to_t61(skin_type, &file.binary)?;
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