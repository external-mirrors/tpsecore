// This module translated from SkinSplicer v2.2.0 by UniQMG

mod lookup_skin;
pub mod maps;
mod piece;
mod skin_splicer;

pub use lookup_skin::lookup_skin;
pub use maps::SkinSlice;
pub use piece::Piece;
pub use skin_splicer::SkinSplicer;

pub type Connection = &'static [(u8, u8)];