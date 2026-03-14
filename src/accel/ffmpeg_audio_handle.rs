use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;

use crate::accel::traits::AudioHandle;

/// For industrial strength format support
#[derive(Clone, Debug)]
pub struct FFmpegAudioHandle(Arc<[f32]>);

impl AudioHandle for FFmpegAudioHandle {
  type Error = std::io::Error;

  async fn decode_audio(buffer: &[u8], _extension: Option<&str>) -> Result<Self, Self::Error> {
    let mut ffmpeg = Command::new("ffmpeg")
      .args([
        "-nostdin",
        "-i", "pipe:0",
        "-f", "f32le",
        "-acodec", "pcm_f32le",
        "-ac", "2",
        "-ar", "44100",
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
    
    let samples: Vec<f32> = output.stdout
      .chunks_exact(4)
      .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
      .collect();
    Ok(Self(samples.into()))
  }
  
  async fn length(&self) -> Result<usize, Self::Error> {
    Ok(self.0.len())
  }
  
  async fn read(&self, out: &mut [f32], offset: usize) -> Result<(), Self::Error> {
    out.copy_from_slice(&self.0[offset..offset+out.len()]);
    Ok(())
  }
}