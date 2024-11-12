use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::io::ErrorKind::UnexpectedEof;
use std::ops::Deref;
use std::os::raw::c_char;
use std::ptr::{null, null_mut};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};
use crate::import::import_task::SoundEffect;
use crate::import::{ImportErrorType, RenderFailure};
use crate::import::LoadError::{NoSupportedAudioTrack, SymphoniaError};
use crate::tpse::{CustomSoundAtlas, TPSE};

pub struct TetrioAtlasDecoder {
  pub atlas: CustomSoundAtlas,
  pub decoded: Vec<f32>
}

const DEFAULT_TETRIO_SFX_LENGTH_SAMPLES: usize = (9*60 + 6) * 44100 * 2;

impl TetrioAtlasDecoder {
  pub fn decode(atlas: CustomSoundAtlas, bytes: &[u8], extension: Option<&str>) -> Result<Self, ImportErrorType> {
    let mut decoded = Vec::with_capacity(DEFAULT_TETRIO_SFX_LENGTH_SAMPLES);
    decode(bytes, extension, |samples| {
      decoded.extend_from_slice(samples);
    })?;
    Ok(TetrioAtlasDecoder { atlas, decoded })
  }

  pub fn decode_from_tpse(tpse: &TPSE) -> Result<Self, ImportErrorType> {
    let atlas = tpse.custom_sound_atlas.as_ref().ok_or_else(|| {
      RenderFailure::NoSoundEffectsConfiguration
    })?.clone();
    let ogg = tpse.custom_sounds.as_ref().ok_or_else(|| {
      RenderFailure::NoSoundEffectsConfiguration
    })?;
    let ext = mime_guess::get_mime_extensions_str(&ogg.mime)
      .and_then(|mime| mime.first())
      .map(Deref::deref);
    let mut decoded = Vec::with_capacity(DEFAULT_TETRIO_SFX_LENGTH_SAMPLES);
    decode(&ogg.binary, ext, |samples| {
      decoded.extend_from_slice(samples);
    })?;
    Ok(TetrioAtlasDecoder { atlas, decoded })
  }

  /// Looks up an atlas entry by name and returns the associated samples
  pub fn lookup(&self, sfx_name: &str) -> Option<&[f32]> {
    let (offset, duration) = self.atlas.get(sfx_name)?;
    // todo throw on overflow (and all the other places we do unchecked casts)
    let offset_samples = (offset/1000.0 * 44100.0 * 2.0) as usize;
    let offset_duration = (duration/1000.0 * 44100.0 * 2.0) as usize;
    log::trace!("lookup {}: {} {} -> {} {}", sfx_name, offset, duration, offset_samples, offset_duration);
    Some(&self.decoded[offset_samples..offset_samples + offset_duration])
  }
}

pub fn decode(bytes: &[u8], extension: Option<&str>, mut caller: impl FnMut(&[f32])) -> Result<(), ImportErrorType> {
  let mut hint = Hint::new();
  if let Some(extension) = extension {
    hint.with_extension(extension);
  }

  let mut stream = MediaSourceStream::new(Box::new(Cursor::new(Vec::from(bytes))), Default::default());

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
    let packet = match probe.format.next_packet() {
      Err(symphonia::core::errors::Error::ResetRequired) => break,
      Err(symphonia::core::errors::Error::IoError(err)) if err.kind() == UnexpectedEof => {
        log::warn!("[temporary debug] ignoring unexpected eof");
        break
      },
      Err(other) => {
        log::debug!("Rich symphonia error: {:?}", other);
        return Err(SymphoniaError(other).into())
      },
      Ok(packet) => packet
    };
    if packet.track_id() != track_id { continue; }
    let decoded = decoder.decode(&packet).map_err(|err| SymphoniaError(err))?;
    let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
    sample_buf.copy_interleaved_ref(decoded);
    caller(&sample_buf.samples());
  }
  Ok(())
}