use std::io::{Cursor, ErrorKind};
use std::ops::Range;
use std::sync::Arc;

use hound::{SampleFormat, WavSpec};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

use crate::accel::traits::AudioHandle;

/// This is actually fairly useless right now since it can't handle opus coded audio
/// which tetrio has been using for a while now
#[derive(Debug, Clone)]
pub struct SoftwareAudioHandle(Arc<[f32]>, Range<usize>);

#[derive(Debug, thiserror::Error)]
pub enum SoftwareAudioError {
  #[error("failed to decode audio: {0}")]
  SymphoniaError(#[from] symphonia::core::errors::Error),
  #[error("failed to decode audio: no supported audio track")]
  NoSupportedAudioTrack,
}

impl AudioHandle for SoftwareAudioHandle {
  type Error = SoftwareAudioError;
  
  fn new_from_samples(samples: Arc<[f32]>) -> Self {
    let range = 0..samples.len();
    Self(samples, range)
  }
  
  async fn decode_audio(buffer: Arc<[u8]>, extension: Option<&str>) -> Result<Self, Self::Error> {
    let mut hint = Hint::new();
    if let Some(extension) = extension {
      hint.with_extension(extension);
    }

    // requires a boxed byte source which is annoying since that needs a copy
    let stream = MediaSourceStream::new(Box::new(Cursor::new(buffer.to_vec())), Default::default());

    let fmt_opts = FormatOptions { enable_gapless: true, ..Default::default() };
    let mut probe = get_probe().format(&hint, stream, &fmt_opts, &Default::default())?;

    let track = probe.format.tracks().iter()
      .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
      .ok_or(SoftwareAudioError::NoSupportedAudioTrack)?;
    let track_id = track.id;

    let mut decoder = get_codecs().make(&track.codec_params, &Default::default())?;

    let mut buffers = vec![];
    loop {
      let packet = match probe.format.next_packet() {
        Err(symphonia::core::errors::Error::ResetRequired) => todo!("handle ResetRequired"),
        Err(symphonia::core::errors::Error::IoError(err)) if err.kind() == ErrorKind::UnexpectedEof => {
          log::warn!("[temporary debug] ignoring unexpected eof");
          break
        },
        Err(other) => {
          log::debug!("Rich symphonia error: {:?}", other);
          return Err(other.into())
        },
        Ok(packet) => packet
      };
      if packet.track_id() != track_id { continue; }
      let decoded = decoder.decode(&packet)?;
      let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
      sample_buf.copy_interleaved_ref(decoded);
      buffers.push(sample_buf);
    }
    
    let buffer: Arc<[_]> = buffers.iter().flat_map(|buf| buf.samples()).copied().collect::<Vec<_>>().into();
    let range = 0..buffer.len();
    Ok(Self(buffer, range))
  }
  
  fn slice(&self, slice: std::ops::Range<usize>) -> Self {
    let start = self.1.start + slice.start;
    Self(self.0.clone(), start..start+slice.end)
  }
  
  async fn length(&self) -> Result<usize, Self::Error> {
    Ok(self.1.end - self.1.start)
  }

  async fn read(&self, mut accept: impl FnMut(f32)) -> Result<(), Self::Error> {
    for byte in &self.0[self.1.clone()] {
      accept(*byte);
    }
    Ok(())
  }

  async fn encode_ogg(chunks: &[Self]) -> Result<Arc<[u8]>, Self::Error> {
    let mut encoded = vec![];
    let mut cursor = Cursor::new(&mut encoded);
    // "ogg"
    let mut encoder = hound::WavWriter::new(&mut cursor, WavSpec {
      channels: 2,
      sample_rate: 44100,
      bits_per_sample: 32,
      sample_format: SampleFormat::Float
    }).unwrap();
    for chunk in chunks {
      for sample in &chunk.0[..] {
        encoder.write_sample(*sample).unwrap();
      }
    }
    encoder.finalize().unwrap();
    Ok(encoded.into())
  }
}