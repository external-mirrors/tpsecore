use std::io::Cursor;
use std::panic::catch_unwind;
use image::{AnimationDecoder, DynamicImage, Frame, ImageResult};
use image::codecs::gif::GifDecoder;
use image::codecs::webp::WebPDecoder;
use image::io::Reader;
use tiny_skia::{Pixmap, Transform};
use crate::import::{ImportErrorType, LoadError};

pub fn decode_image(bytes: &[u8]) -> Result<DynamicImage, LoadError> {
  let transcoded = decode_svg(bytes);
  let bytes = match transcoded.as_ref() {
    Some(transcoded) => transcoded,
    None => bytes
  };

  Ok(catch_unwind(|| {
    let reader = Reader::new(Cursor::new(bytes))
      .with_guessed_format()
      .expect("Cursor<&[u8]> shouldn't generate IO errors");
    reader.decode()
  }).map_err(|err| {
    LoadError::ImageLoadPanic
  })??)
}

fn decode_svg(bytes: &[u8]) -> Option<Vec<u8>> {
  let opt = usvg::Options::default();
  let rtree = usvg::Tree::from_data(bytes, &opt.to_ref()).ok()?;
  let pixmap_size = rtree.svg_node().size.to_screen_size();
  let mut pixmap = Pixmap::new(pixmap_size.width(), pixmap_size.height())?;
  resvg::render(&rtree, usvg::FitTo::Original, Transform::default().into(), pixmap.as_mut())?;
  pixmap.encode_png().ok()
}