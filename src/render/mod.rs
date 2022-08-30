mod board_element;
mod nine_slice;
pub mod example_maps;
mod render_options;
mod render;
mod board_map;

pub use board_element::{BoardElement, BoardTextureKind};
pub use nine_slice::{nine_slice, nine_slice_resize};
pub use render_options::RenderOptions;
pub use render::*;
pub use board_map::BoardMap;

use image::DynamicImage;

/// Clones a slice of the given DynamicImage, filling overflow regions with transparency
pub fn clone_slice(tex: &DynamicImage, x: u32, y: u32, w: u32, h: u32) -> DynamicImage {
  let mut target = DynamicImage::new_rgba8(w, h);
  image::imageops::overlay(&mut target, tex, -(x as i64), -(y as i64));
  target
}

