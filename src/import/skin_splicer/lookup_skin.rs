use crate::import::skin_splicer::{Piece, SkinSlice};
use crate::import::skin_splicer::maps::*;
use crate::import::SkinType;

pub fn lookup_skin(skin_type: SkinType, piece: Piece) -> Option<SkinSlice> {
  match skin_type {
    SkinType::Tetrio61 => tetrio_61_map(piece),
    SkinType::Tetrio61Ghost => tetrio_61_ghost_map(piece),
    SkinType::Tetrio61Connected => tetrio_61_conn_map(piece),
    SkinType::Tetrio61ConnectedGhost => tetrio_61_conn_ghost_map(piece),
    SkinType::Tetrio61ConnectedAnimated { .. } => tetrio_61_conn_map(piece),
    SkinType::Tetrio61ConnectedGhostAnimated { .. } => tetrio_61_conn_ghost_map(piece),
    SkinType::TetrioAnimated { .. } => tetrio_map(piece),
    SkinType::TetrioRaster => tetrio_map(piece),
    SkinType::TetrioSVG => tetrio_map(piece),
    SkinType::JstrisRaster => jstris_map(piece),
    SkinType::JstrisAnimated { .. } => jstris_map(piece),
    SkinType::JstrisConnected => jstris_conn_map(piece)
  }
}