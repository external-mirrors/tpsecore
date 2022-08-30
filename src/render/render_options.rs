use crate::import::skin_splicer::Piece;
use crate::render::{BoardElement, BoardMap, example_maps};


#[derive(Debug, Clone)]
pub struct RenderOptions<'a> {
  /// How long this render should be shown, in seconds.
  /// Note that animation frames are rounded up, and must last at least one video frame.
  pub duration: f64,
  /// What parts of the board to render and in what order
  pub board_elements: &'a [BoardElement],
  /// Whether to draw the coordinate debug grid overlay
  pub debug_grid: bool,
  /// The contents of the board
  pub board: BoardMap,
  /// The highest row that's rendered with a background.
  /// Typically four fewer (or half for a double-full-matrix-height board) of the board height.
  pub skyline: usize,
  /// The size to render each block as. Affects multiple other board elements that depend on it.
  pub block_size: i64
}

impl Default for RenderOptions<'static> {
  fn default() -> Self {
    RenderOptions {
      duration: 0.0,
      board_elements: BoardElement::get_draw_order(),
      debug_grid: false,
      board: BoardMap::from(example_maps::EMPTY_MAP),
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
    let height = self.board.height();
    let width = self.board.width();
    (width, height)
  }
}