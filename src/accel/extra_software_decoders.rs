use std::io::Cursor;
use std::time::Duration;

use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, ImageError, ImageFormat};
use image::codecs::gif::GifDecoder;

use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::{ImportErrorType, MediaLoadError};

#[derive(Debug, thiserror::Error)]
pub enum ExtraSoftwareDecoderError<T: TPSEAccelerator> {
  #[error("{0}")]
  Tex(<T::Texture as TextureHandle>::Error),
  #[error("{0}")]
  ImageError(#[from] ImageError)
}

impl<T: TPSEAccelerator> From<ExtraSoftwareDecoderError<T>> for MediaLoadError<T> {
  fn from(value: ExtraSoftwareDecoderError<T>) -> Self {
    match value {
      ExtraSoftwareDecoderError::Tex(tex) => Self::TextureError(tex),
      ExtraSoftwareDecoderError::ImageError(image_error) => Self::Other(image_error.to_string())
    }
  }
}
impl<T: TPSEAccelerator> From<ExtraSoftwareDecoderError<T>> for ImportErrorType<T> {
  fn from(value: ExtraSoftwareDecoderError<T>) -> Self {
    ImportErrorType::LoadError(value.into())
  }
}

pub fn decode_gif<T: TPSEAccelerator>(bytes: &[u8])
  -> Result<Vec<(T::Texture, Duration)>, ExtraSoftwareDecoderError<T>>
{
  decode::<T>(GifDecoder::new(Cursor::new(bytes)))
}
pub fn decode_webp<T: TPSEAccelerator>(bytes: &[u8])
  -> Result<Vec<(T::Texture, Duration)>, ExtraSoftwareDecoderError<T>>
{
  decode::<T>(WebPDecoder::new(Cursor::new(bytes)))
}

fn decode<'a, T: TPSEAccelerator>
  (decoder: Result<impl AnimationDecoder<'a>, ImageError>)
  -> Result<Vec<(T::Texture, Duration)>, ExtraSoftwareDecoderError<T>>
{
  let frames = decoder?.into_frames();
  let mut textures = vec![];
  for frame in frames {
    let frame = frame?;
    let mut buffer = vec![];
    
    let (num, denom) = frame.delay().numer_denom_ms();
    let duration = Duration::from_secs_f64((num as f64 / denom as f64) / 1000.0);
    frame.buffer().write_to(&mut Cursor::new(&mut buffer), ImageFormat::Bmp)?;
    let decoded = T::Texture::decode_texture(buffer.into()).map_err(|err| ExtraSoftwareDecoderError::Tex(err))?;
    textures.push((decoded, duration));
  }
  
  Ok(textures)
}