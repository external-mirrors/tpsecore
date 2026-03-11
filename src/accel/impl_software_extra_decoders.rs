use std::error::Error;
use std::io::Cursor;
use std::time::Duration;

use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, ImageDecoder, ImageFormat};
use image::codecs::gif::GifDecoder;

use crate::accel::traits::TPSEAccelerator;
use crate::import::LoadError;

pub fn decode_gif<T: TPSEAccelerator>(bytes: &[u8]) -> Result<Vec<(T::Texture, Duration)>, Box<dyn Error>> {
  decode::<T>(GifDecoder::new(Cursor::new(bytes)))
}
pub fn decode_webp<T: TPSEAccelerator>(bytes: &[u8]) -> Result<Vec<(T::Texture, Duration)>, Box<dyn Error>> {
  decode::<T>(WebPDecoder::new(Cursor::new(bytes)))
}

fn decode<'a, T: TPSEAccelerator>
  (decoder: Result<impl AnimationDecoder<'a>, impl Error + 'static>)
  -> Result<Vec<(T::Texture, Duration)>, Box<dyn Error>>
{
  let frames = decoder.map_err(Box::new)?.into_frames();
  let mut textures = vec![];
  for frame in frames {
    let frame = frame.map_err(Box::new)?;
    let mut buffer = vec![];
    
    let (num, denom) = frame.delay().numer_denom_ms();
    let duration = Duration::from_secs_f64((num as f64 / denom as f64) / 1000.0);
    frame.buffer()
      .write_to(&mut Cursor::new(&mut buffer), ImageFormat::Bmp)
      .map_err(Box::new)?;
    let decoded = T::decode_texture(buffer.into()).map_err(Box::new)?;
    textures.push((decoded, duration));
  }
  
  Ok(textures)
}