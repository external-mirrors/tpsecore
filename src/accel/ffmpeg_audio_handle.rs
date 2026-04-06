use std::io::Write;
use std::ops::Range;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;

use crate::accel::traits::AudioHandle;

/// For industrial strength format support
#[derive(Clone, Debug)]
pub struct FFmpegAudioHandle(Arc<[f32]>, Range<usize>);

#[derive(Debug, thiserror::Error)]
pub enum FFmpegError {
  #[error("io error: {0}")]
  IOError(#[from] std::io::Error),
  #[error("ffmpeg exited unsuccessfully: {0}")]
  FFmpegExit(ExitStatus)
}

impl AudioHandle for FFmpegAudioHandle {
  type Error = FFmpegError;

  fn new_from_samples(samples: Arc<[f32]>) -> Self {
    let range = 0..samples.len();
    Self(samples, range)
  }
  
  async fn decode_audio(buffer: Arc<[u8]>, _extension: Option<&str>) -> Result<Self, Self::Error> {
    let mut ffmpeg = Command::new("ffmpeg")
      .args([
        "-hide_banner",
        "-loglevel", "error",
        "-i", "pipe:0",
        "-f", "f32le",
        "-acodec", "pcm_f32le",
        "-ac", "2",
        "-ar", "48000",
        "pipe:1",
      ])
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::inherit())
      .spawn()?;

    let mut stdin = ffmpeg.stdin.take().unwrap();
    let output = std::thread::scope(|scope| {
      scope.spawn(|| {
        let _ = stdin.write_all(&buffer);
        drop(stdin);
      });
      scope.spawn(|| {
        ffmpeg.wait_with_output()
      }).join().unwrap()
    })?;
    if !output.status.success() {
      return Err(FFmpegError::FFmpegExit(output.status));
    }
    
    let samples: Vec<f32> = output.stdout
      .chunks_exact(4)
      .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
      .collect();
    let range = 0..samples.len();
    Ok(Self(samples.into(), range))
  }
  
  fn slice(&self, slice: std::ops::Range<usize>) -> Self {
    let start = self.1.start + slice.start;
    Self(self.0.clone(), start..start+(slice.end - slice.start))
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
    let mut ffmpeg = Command::new("ffmpeg")
      .args([
        "-hide_banner",
        "-loglevel", "error",
        
        "-f", "f32le",
        "-ac", "2",
        "-ar", "48000",
        "-i", "pipe:0",
        
        "-f", "ogg",
        "-c:a", "libopus",
        "pipe:1",
      ])
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::inherit())
      .spawn()?;

    let mut stdin = ffmpeg.stdin.take().unwrap();
    let output = std::thread::scope(|scope| {
      scope.spawn(|| {
        for chunk in chunks {
          let _ = stdin.write_all(bytemuck::cast_slice(&chunk.0[chunk.1.clone()]));
        }
        drop(stdin);
      });
      scope.spawn(|| {
        ffmpeg.wait_with_output()
      }).join().unwrap()
    })?;
    if !output.status.success() {
      return Err(FFmpegError::FFmpegExit(output.status));
    }
    
    Ok(output.stdout.into())
  }
}