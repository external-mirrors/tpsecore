use std::ops::Deref;
use crate::accel::traits::{AudioHandle, TPSEAccelerator};
use crate::tpse::{CustomSoundAtlas, TPSE};

pub struct TetrioAtlasDecoder {
  pub atlas: CustomSoundAtlas,
  pub buffer: Vec<f32>
}

impl TetrioAtlasDecoder {
  pub async fn decode<T: TPSEAccelerator>(atlas: CustomSoundAtlas, bytes: &[u8], extension: Option<&str>)
    -> Result<Self, <T::Audio as AudioHandle>::Error>
  {
    Ok(TetrioAtlasDecoder { atlas, buffer: decode::<T>(bytes, extension).await? })
  }

  pub async fn decode_from_tpse<T: TPSEAccelerator>(tpse: &TPSE) -> Result<Option<Self>, <T::Audio as AudioHandle>::Error> {
    let Some(atlas) = tpse.custom_sound_atlas.clone() else { return Ok(None) };
    let Some(file) = tpse.custom_sounds.clone() else { return Ok(None) };
    let ext = mime_guess::get_mime_extensions_str(&file.mime)
      .and_then(|mime| mime.first())
      .map(Deref::deref);
    Ok(Some(Self::decode::<T>(atlas, &file.binary, ext).await?))
  }

  /// Looks up an atlas entry by name and returns the associated samples
  pub fn lookup(&self, sfx_name: &str) -> Option<&[f32]> {
    let (offset, duration) = self.atlas.get(sfx_name)?;
    let offset_samples = (offset/1000.0 * 44100.0 * 2.0) as usize;
    let offset_duration = (duration/1000.0 * 44100.0 * 2.0) as usize;
    // log::trace!("lookup {}: {} {} -> {} {}", sfx_name, offset, duration, offset_samples, offset_duration);
    if offset_samples + offset_duration > self.buffer.len() {
      log::error!("sound effect atlas entry {sfx_name} does not fit in buffer: {offset_samples}+{offset_duration}>{}", self.buffer.len());
      return None;
    }
    Some(&self.buffer[offset_samples .. offset_samples + offset_duration])
  }
}

pub async fn decode<T: TPSEAccelerator>
  (bytes: &[u8], extension: Option<&str>)
  -> Result<Vec<f32>, <T::Audio as AudioHandle>::Error>
{
  debug_assert!(bytes.get(0..4) != Some(&[0x74, 0x52, 0x25, 0x44]), "decode was passed a tRSD file");
  let decoded = T::Audio::decode_audio(bytes, extension).await?;
  let length = decoded.length().await?;
  let mut buffer = vec![0.0; length];
  decoded.read(&mut buffer, 0).await?;
  Ok(buffer)
}

