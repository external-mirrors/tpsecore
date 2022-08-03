use std::io::Cursor;
use image::DynamicImage;
use image::io::Reader;
use tiny_skia::{Pixmap, Transform};
use crate::import::ImportErrorType;

pub fn decode_image(bytes: &[u8]) -> Result<DynamicImage, ImportErrorType> {
  let transcoded = decode_svg(bytes);
  let bytes = match transcoded.as_ref() {
    Some(transcoded) => transcoded,
    None => bytes
  };

  Ok(Reader::new(Cursor::new(bytes)).with_guessed_format().unwrap().decode()?)
}

fn decode_svg(bytes: &[u8]) -> Option<Vec<u8>> {
  let mut opt = usvg::Options::default();
  let rtree = usvg::Tree::from_data(bytes, &opt.to_ref()).ok()?;
  let pixmap_size = rtree.svg_node().size.to_screen_size();
  let mut pixmap = Pixmap::new(pixmap_size.width(), pixmap_size.height())?;
  resvg::render(&rtree, usvg::FitTo::Original, Transform::default().into(), pixmap.as_mut())?;
  pixmap.encode_png().ok()
}