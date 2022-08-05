use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::os::raw::c_char;
use std::ptr::{null, null_mut};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};
use crate::import::import_task::SoundEffect;
use crate::import::ImportErrorType;
use crate::import::LoadError::{NoSupportedAudioTrack, SymphoniaError};
use crate::tpse::CustomSoundAtlas;

pub struct TetrioAtlasDecoder {
  pub decoded: Vec<f32>,
}

impl TetrioAtlasDecoder {
  pub fn decode(bytes: &[u8], extension: Option<&str>) -> Result<Self, ImportErrorType> {
    let mut decoded = Vec::with_capacity(546 * 44100 * 2);
    decode(bytes, Some("ogg"), |samples| {
      decoded.extend_from_slice(samples);
    })?;
    Ok(TetrioAtlasDecoder { decoded })
  }

  pub fn lookup(&self, atlas: &CustomSoundAtlas, sfx_name: &str) -> Option<&[f32]> {
    let (offset, duration) = atlas.get(sfx_name)?;
    let offset_samples = (offset * 44100.0 * 2.0) as usize;
    let offset_duration = (duration * 44100.0 * 2.0) as usize;
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
      Err(other) => {
        log::warn!("{:?}", other);
        return Err(SymphoniaError(other).into())
      },
      Ok(packet) => packet,
    };
    if packet.track_id() != track_id { continue; }
    let decoded = decoder.decode(&packet).map_err(|err| SymphoniaError(err))?;
    let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
    sample_buf.copy_interleaved_ref(decoded);
    caller(&sample_buf.samples());
  }
  Ok(())
}