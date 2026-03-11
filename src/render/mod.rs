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

use crate::accel::traits::{TPSEAccelerator, TextureHandle};

/// Clones a slice of the given T::Texture, filling overflow regions with transparency
pub fn clone_slice<T: TPSEAccelerator>(tex: &T::Texture, x: u32, y: u32, w: u32, h: u32) -> T::Texture {
  let target = T::new_texture(w, h);
  target.overlay(tex, -(x as i64), -(y as i64));
  target
}

