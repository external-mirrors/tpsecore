use std::ops::Deref;

use crate::accel::traits::{TPSEAccelerator, TextureHandle};

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

pub async fn nine_slice_resize<T: TPSEAccelerator>
  (tex: &T::Texture, w: u32, h: u32, pad_top: u32, pad_right: u32, pad_bottom: u32, pad_left: u32)
  -> T::Texture
{
  if w > 10_000 || h > 10_000 || w*h > 10_000_000 {
    log::warn!("nine_slice_resize: creating huge texture of {w}*{h}");
    #[cfg(test)]
    panic!("excessive texture size requested");
  }
  let sources = nine_slice(tex.width().await, tex.height().await, pad_top, pad_right, pad_bottom, pad_left);
  let dests = nine_slice(w, h, pad_top, pad_right, pad_bottom, pad_left);
  let mut dest = T::new_texture(w, h);
  for ((sx, sy, sw, sh), (dx, dy, dw, dh)) in sources.iter().copied().zip(dests.iter().copied()) {
    if sw == 0 || sh == 0 { continue; }
    let slice = tex.slice(sx, sy, sw, sh);
    let resized = slice.resized(dw, dh);
    dest.overlay(&resized, dx as i64, dy as i64);
  }
  dest
}