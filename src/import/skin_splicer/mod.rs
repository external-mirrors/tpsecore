// This module translated from SkinSplicer v2.2.0 by UniQMG

mod lookup_skin;
pub mod maps;
mod piece;
mod skin_splicer;
mod decode_image;

pub use lookup_skin::lookup_skin;
pub use maps::SkinSlice;
pub use piece::Piece;
pub use skin_splicer::SkinSplicer;
pub use decode_image::decode_image;

pub type Connection = &'static [(u8, u8)];

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
  #[error("failed to load image: {0}")]
  ImageError(#[from] image::ImageError)
}