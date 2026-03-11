use crate::render::RenderOptions;

/// An element contained on the `board.png` texture
#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardElement {
  /// Board: Board background.
  Background,
  /// Board: Mini-grid borders (used for small boards in multiplayer).
  MiniGridBorder,
  /// Board: Name tag background.
  NameTagBackground,
  /// Board: Name tag background when on fire.
  NameTagBackgroundOnFire,
  /// Board: Top board border when in danger.
  DangerLine,
  /// Board: Gradient appearing below DangerLine on the top part of the board.
  DangerGlow,
  /// Board: Inner board/grid borders.
  BoardGridBordersInnerBottom,
  /// Board: Far left board border. Used only when garbage bar is visible.
  GarbageBar,
  /// Board: Far right board border. Used only when progress bar is visible.
  ProgressBar,
  /// Board: Stock indicators (lives).
  Stock,
  /// Board: Garbage meter; incoming garbage stored in the garbage bar. Tinted red.
  Garbage,
  /// Board: Progress meter; current objective progress stored in the progress bar.
  /// Tinted orange. Lower half stretched to get the full bar.
  Progress,
  /// Board: Garbage cap. Small horizontal line that appears on the garbage bar.
  GarbageCap,
  /// Board: Warning marker, appears on your board when about to top out due to garbage.
  Warning,
  /// Board: Targeting marker, appears on other boards.
  Target,
  /// Board: Pending garbage stored in the garbage bar above [BoardElement::Garbage]. Semitransparent. Tinted red.
  PendingGarbage,
  /// Board: Winter event compat: a texture that appears behind the grid, relatively high resolution
  MegaBackground,
  /// Board: Winter event compat: a texture that appears in front of the grid, relatively high resolution
  MegaForeground,

  /// Queue: the hold texture
  Hold,
  /// Queue: the next queue
  Next,
  /// Queue: the replay overlay
  Replay
}

/// Adjusts the scale of `BoardGridBordersInnerBottom`, `GarbageBar`, `ProgressBar`, `Hold`, `Next`
/// and some layout changes to make it all fit.
const BORDER_SCALE: u32 = 2;

pub enum BoardTextureKind {
  Board,
  Queue,
  Grid
}

impl BoardElement {
  pub fn get_draw_order() -> &'static [BoardElement] {
    &[
      Self::Background,
      Self::MegaBackground,

      Self::Hold,
      Self::Next,

      Self::BoardGridBordersInnerBottom,
      Self::GarbageBar,
      Self::ProgressBar,

      Self::DangerLine,
      Self::DangerGlow,

      Self::Stock,
      Self::Garbage,
      Self::PendingGarbage,
      Self::Progress,

      Self::MegaForeground,

      Self::GarbageCap,

      Self::Replay,

      Self::Warning,
      Self::Target,
    ]
  }

  pub fn tint(&self) -> [u8; 4] {
    match self {
      BoardElement::Garbage => 0xF71700FF,
      BoardElement::Progress => 0xB84E07FF,
      _ => 0xFFFFFFFFu32
    }.to_be_bytes()
  }

  /// Gets the relative location the texture should be drawn to in a rendered board.
  /// Values in pixels, where 0,0 is the top left corner of the inner board where the blocks start.
  /// All values determined using only screenshots, the tetrio board texture, and manual alignment.
  pub fn get_target(&self, opts: &RenderOptions) -> (i64, i64, i64, i64) {
    let (width, _) = opts.board_size();
    let width = width as i64;
    let height = opts.skyline as i64;
    /// How many pixels each block is
    let block = opts.block_size;
    /// How wide a border on the board is
    let border = 9 / BORDER_SCALE as i64;
    /// The space inside the board where the blocks and grid are located.
    /// Does not include the default borders.
    let board_internal = (0, 0, block*width, block*height);
    /// The whole board including its borders but not including any bars
    let board_with_border = (-border, 0, block*width + border*2, block*height + border);
    /// How wide each bar is
    let bar_width = 32;
    /// The garbage bar, located to the left of the board
    let mut garbage_bar = (border*-2 + bar_width*-1, 0, border*2 + bar_width, block*height + border);
    let has_garbage_bar = [BoardElement::GarbageBar, BoardElement::Garbage, BoardElement::PendingGarbage].iter()
      .any(|x| opts.board_elements.contains(x));
    if !has_garbage_bar { garbage_bar.2 = 0; }
    /// The progress bar, located to the right of the board
    let mut progress_bar = (block*width, 0, border*2 + bar_width, block*height + border);
    let has_progress_bar = [BoardElement::ProgressBar, BoardElement::Progress].iter()
      .any(|x| opts.board_elements.contains(x));
    if !has_progress_bar { progress_bar.2 = 0; }
    /// How many pixels to bad the bar contents by
    let bar_pad = 4;

    match self {
      Self::Background => board_internal,
      Self::MiniGridBorder => todo!(),
      Self::NameTagBackground => todo!(),
      Self::NameTagBackgroundOnFire => todo!(),
      Self::DangerLine => (0, 0, block*width, border),
      Self::DangerGlow => (0, 0, block*width, 150),
      Self::BoardGridBordersInnerBottom => (-border, 0, block*width + border*2, block*height + border),
      Self::GarbageBar => garbage_bar,
      Self::ProgressBar => progress_bar,
      Self::Stock => ((block as f64 * width as f64 / 2.0) as i64 - 24, block*height+border*2, 48, 48),
      Self::Garbage => {
        let (x, y, w, h) = garbage_bar;
        let garbage_height = block*4; // arbitrary
        (x + border + bar_pad, y + (h - border - garbage_height) + bar_pad, w - border*2 - bar_pad*2, garbage_height - bar_pad*2)
      },
      Self::Progress => {
        let (x, y, w, h) = progress_bar;
        let progress_height = block*3; // arbitrary
        (x + border + bar_pad, y + (h - border - progress_height) + bar_pad, w - border*2 - bar_pad*2, progress_height - bar_pad*2)
      },
      Self::GarbageCap => {
        let (x, y, w, h) = garbage_bar;
        let ymod = block*6;
        let height = border;
        (x, y + (h - border - ymod - height/2), w, height)
      },
      Self::Warning => {
        // completely arbitrary
        let size = 48*3;
        (-size/2, block*height/2 - size/2, size, size)
      },
      Self::Target => (block * width + border + bar_width + border + block * 6, 0, 100, 100), // made up off-board value
      Self::PendingGarbage => {
        let (x, y, w, h) = garbage_bar;
        let garbage_height = block*2; // arbitrary
        let existing_garbage_height = block*4;
        let ymod = garbage_height + existing_garbage_height;
        (x + border + bar_pad, y + (h - border - ymod) + bar_pad, w - border*2 - bar_pad*2, garbage_height - bar_pad*2)
      },
      Self::MegaBackground => board_internal,
      Self::MegaForeground => (-border, 0, block*width+ border*2, block*height + border),
      Self::Hold => {
        let height = block*4; // Arbitrary, but it lines up almost perfectly
        let width = (height as f64 * 183.0/137.0) as i64; // approximate aspect ratio
        let (_, _, gw, _) = garbage_bar;
        (-(gw + width) + border /* overlapping border */, 0, width, height)
      },
      Self::Next => {
        let height = block*16; // Arbitrary, but it lines up almost perfectly
        let width = (height as f64 * 183.0/556.0) as i64; // approximate aspect ratio
        let (px, _, pw, _) = progress_bar;
        (px + pw - border /* overlapping border */, 0, width, height)
      },
      Self::Replay => (0, 0, 0, 0) // TODO
    }
  }

  /// Gets the coordinates of the subtexture from the main board texture in pixels
  /// The first value is the board texture type to pull from
  /// The second four values are the x,y,w,h of the texture
  /// The third four values make up the padding of a 9-slice grid on the sides: 🡱🡲🡳🡰
  /// The fourth value is a scale to multiply the nine-slice resize size by.
  pub fn get_slice(&self) -> (BoardTextureKind, (u32, u32, u32, u32), (u32, u32, u32, u32), u32) {
    use BoardTextureKind::*;
    match self {
      Self::Background => (Board, (0, 0, 20, 20), (0, 0, 0, 0), 1),
      Self::MiniGridBorder => (Board, (22, 0, 26, 18), (0, 10, 10, 10), 1),
      Self::NameTagBackground => (Board, (50, 0, 20, 20), (0, 0, 0, 0), 1),
      Self::NameTagBackgroundOnFire => (Board, (72, 0, 20, 20), (0, 0, 0, 0), 1),
      Self::DangerLine => (Board, (96, 0, 13, 11), (0, 0, 0, 0), 1),
      Self::DangerGlow => (Board, (96, 11, 13, 64), (0, 0, 0, 0), 1),
      Self::BoardGridBordersInnerBottom => (Board, (111, 0, 27, 20), (0, 9, 9, 9), BORDER_SCALE),
      Self::GarbageBar => (Board, (142, 0, 27, 20), (0, 9, 9, 9), BORDER_SCALE),
      Self::ProgressBar => (Board, (173, 0, 27, 20), (0, 9, 9, 9), BORDER_SCALE),
      Self::Stock => (Board, (10, 30, 76, 76), (0, 0, 0, 0), 1),
      Self::Garbage => (Board, (109, 30, 64, 56), (6, 0, 8, 0), 1),
      Self::Progress => (Board, (173, 24, 64, 62), (32, 0, 0, 0), 1),
      Self::GarbageCap => (Board, (111, 88, 60, 8), (0, 0, 0, 0), 1),
      Self::Warning => (Board, (2, 118, 92, 92), (0, 0, 0, 0), 1),
      Self::Target => (Board, (98, 100, 69, 69), (0, 0, 0, 0), 1),
      Self::PendingGarbage => (Board, (173, 94, 64, 56), (6, 0, 8, 0), 1),
      Self::MegaBackground => (Board, (256, 0, 256, 256), (0, 0, 0, 0), BORDER_SCALE),
      Self::MegaForeground => (Board, (258, 258, 252, 252), (0, 9, 9, 9), BORDER_SCALE),
      Self::Hold => (Queue, (2, 148, 474, 142), (77, 9, 49, 9), BORDER_SCALE),
      Self::Next => (Queue, (2, 2, 474, 142), (77, 9, 49, 9), BORDER_SCALE),
      Self::Replay => (Queue, (2, 294, 398, 77), (0, 0, 0, 0), 1)
    }
  }
}