use crate::import::skin_splicer::Piece;
use crate::render::{BoardElement, PCO_MAP};

#[derive(Copy, Clone)]
pub struct RenderOptions<'a> {
  /// What parts of the board to render and in what order
  pub board_pieces: &'a [BoardElement],
  /// Whether to draw the coordinate debug grid overlay
  pub debug_grid: bool,
  /// The contents of the board.
  pub board: &'a [&'a [(Option<Piece>, u8)]],
  /// The highest row that's rendered with a background.
  /// Typically four fewer (or half for a double-full-matrix-height board) of the board height.
  pub skyline: usize,
  /// The size to render each block as. Affects multiple other board elements that depend on it.
  pub block_size: i64
}

impl Default for RenderOptions<'static> {
  fn default() -> Self {
    RenderOptions {
      board_pieces: BoardElement::get_draw_order(),
      debug_grid: false,
      board: PCO_MAP,
      skyline: 20,
      // This is the size present in the modern tetrio format, so it'll look best when used with
      // most skins.
      block_size: 48
    }
  }
}

impl RenderOptions<'_> {
  /// Returns the width and height of the board. The height is calculated as the max row length,
  /// but there's no guarantee all rows are the same length.
  pub fn board_size(&self) -> (usize, usize) {
    let height = self.board.len();
    let width = self.board.iter().map(|row| row.len()).max().unwrap_or(0);
    (width, height)
  }
}