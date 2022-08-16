use std::ops::Deref;
use image::{DynamicImage, GenericImageView};
use image::imageops::FilterType;

pub fn nine_slice
  (w: u32, h: u32, pad_top: u32, pad_right: u32, pad_bottom: u32, pad_left: u32)
  -> [(u32, u32, u32, u32); 9]
{
  let center_width = w.saturating_sub(pad_left + pad_right);
  let center_height = h.saturating_sub(pad_top + pad_bottom);
  [
    (0, 0, pad_left, pad_top), // top left
    (pad_left, 0, center_width, pad_top), // top center
    (pad_left + center_width, 0, pad_right, pad_top), // top right
    (0, pad_top, pad_left, center_height), // middle left
    (pad_left, pad_top, center_width, center_height), // center
    (pad_left + center_width, pad_top, pad_right, center_height), // middle right
    (0, pad_top + center_height, pad_left, pad_bottom), // bottom left
    (pad_left, pad_top + center_height, center_width, pad_bottom), // bottom center
    (pad_left + center_width, pad_top + center_height, pad_right, pad_bottom), // bottom right
  ]
}

pub fn nine_slice_resize
  (tex: &DynamicImage, w: u32, h: u32, pad_top: u32, pad_right: u32, pad_bottom: u32, pad_left: u32)
  -> DynamicImage
{
  let sources = nine_slice(tex.width(), tex.height(), pad_top, pad_right, pad_bottom, pad_left);
  let dests = nine_slice(w, h, pad_top, pad_right, pad_bottom, pad_left);
  let mut dest = DynamicImage::new_rgba8(w, h);
  for ((sx, sy, sw, sh), (dx, dy, dw, dh)) in sources.iter().copied().zip(dests.iter().copied()) {
    if sw == 0 || sh == 0 { continue; }
    let slice = tex.view(sx, sy, sw, sh);
    let resized = image::imageops::resize(slice.deref(), dw, dh, FilterType::CatmullRom);
    image::imageops::overlay(&mut dest, &resized, dx as i64, dy as i64);
  }
  dest
}