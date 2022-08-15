/// An element contained on the `board.png` texture
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BoardElement {
  /// Board background.
  Background,
  /// Mini-grid borders (used for small boards in multiplayer).
  MiniGridBorder,
  /// Name tag background.
  NameTagBackground,
  /// Name tag background when on fire.
  NameTagBackgroundOnFire,
  /// Top board border when in danger.
  DangerLine,
  /// Gradient appearing below DangerLine on the top part of the board.
  DangerGlow,
  /// Inner board/grid borders.
  BoardGridBordersInnerBottom,
  /// Far left board border. Used only when garbage bar is visible.
  GarbageBar,
  /// Far right board border. Used only when progress bar is visible.
  ProgressBar,
  /// Stock indicators (lives).
  Stock,
  /// Incoming garbage. Tinted red.
  Garbage,
  /// Progress meter. Tinted orange. Lower half stretched to get the full bar.
  Progress,
  /// Garbage cap. Small horizontal line that appears on the garbage bar.
  GarbageCap,
  /// Warning marker, appears on your board when about to top out due to garbage.
  Warning,
  /// Targeting marker, appears on other boards.
  Target,
  /// Pending garbage. Semitransparent. Tinted red.
  PendingGarbage,
  /// Winter event compat: a texture that appears behind the grid, relatively high resolution
  MegaBackground,
  /// Winter event compat: a texture that appears in front of the grid, relatively high resolution
  MegaForeground
}

mod board_size_units {
  /// The size of a single mino
  pub const BLOCK: i64 = 48;
  /// How wide a border on the board is
  pub const BORDER: i64 = 9; // todo: these show up as thinner in game for some reason?
  /// The space inside the board where the blocks and grid are located.
  /// Does not include the default borders.
  pub const BOARD_INTERNAL: (i64, i64, i64, i64) = (0, 0, BLOCK*10, BLOCK*20);
  /// The whole board including its borders but not including any bars
  pub const BOARD_WITH_BORDER: (i64, i64, i64, i64) = (-BORDER, 0, BLOCK*10 + BORDER*2, BLOCK*20 + BORDER);
  /// How wide each bar is
  pub const BAR_WIDTH: i64 = 32;
  /// The garbage bar, located to the left of the board
  pub const GARBAGE_BAR: (i64, i64, i64, i64) = (BORDER*-2 + BAR_WIDTH*-1, 0, BORDER*2 + BAR_WIDTH, BLOCK*20 + BORDER);
  /// The progress bar, located to the right of the board
  pub const PROGRESS_BAR: (i64, i64, i64, i64) = (BLOCK*10, 0, BORDER*2 + BAR_WIDTH, BLOCK*20 + BORDER);
  /// How many pixels to bad the bar contents by
  pub const BAR_PAD: i64 = 4;
}

impl BoardElement {
  pub fn get_draw_order() -> &'static [BoardElement] {
    &[
      Self::Background,
      Self::MegaBackground,

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

      Self::Warning,
      Self::Target,
    ]
  }

  pub fn tint(&self) -> u32 {
    match self {
      BoardElement::Garbage => 0xF71700FF,
      BoardElement::Progress => 0xB84E07FF,
      BoardElement::GarbageCap => 0xF717007F,
      _ => 0xFFFFFFFF
    }
  }

  /// Gets the relative location the texture should be drawn to in a rendered board.
  /// Values in pixels, where 0,0 is the top left corner of the inner board where the blocks start.
  /// All values determined using only screenshots, the tetrio board texture, and manual alignment.
  pub fn get_target(&self) -> (i64, i64, i64, i64) {
    use board_size_units::*;
    use BAR_PAD as p;
    match self {
      Self::Background => BOARD_INTERNAL,
      Self::MiniGridBorder => todo!(),
      Self::NameTagBackground => todo!(),
      Self::NameTagBackgroundOnFire => todo!(),
      Self::DangerLine => (0, 0, BLOCK*10, BORDER),
      Self::DangerGlow => (0, 0, BLOCK*10, 150),
      Self::BoardGridBordersInnerBottom => (-BORDER, 0, BLOCK*10 + BORDER*2, BLOCK*20 + BORDER),
      Self::GarbageBar => GARBAGE_BAR,
      Self::ProgressBar => PROGRESS_BAR,
      Self::Stock => (BLOCK*5 - 24, BLOCK*20+BORDER*2, 48, 48),
      Self::Garbage => {
        let (x, y, w, h) = GARBAGE_BAR;
        (x + BORDER + p, y + BLOCK*16 + p, w - BORDER*2 - p*2, BLOCK*4 - p*2)
      },
      Self::Progress => {
        let (x, y, w, h) = PROGRESS_BAR;
        (x + BORDER + p, y + BLOCK*10 + p, w - BORDER*2 - p*2, BLOCK*10 - p*2)
      },
      Self::GarbageCap => {
        let (x, y, w, h) = GARBAGE_BAR;
        (x + BORDER + p, y + BLOCK*14 + p, w - BORDER*2 - p*2, BORDER - p*2)
      },
      Self::Warning => (-75, 406, 130, 130),
      Self::Target => (BLOCK*20+BORDER+BAR_WIDTH+BORDER+10, 0, 100, 100), // made up off-board value
      Self::PendingGarbage => {
        let (x, y, w, h) = GARBAGE_BAR;
        (x + BORDER + p, y + BLOCK*14 + p, w - BORDER*2 - p*2, BLOCK*2 - p*2)
      },
      Self::MegaBackground => BOARD_INTERNAL,
      Self::MegaForeground => (-BORDER, 0, BLOCK*10+BORDER*2, BLOCK*20+BORDER)
    }
  }

  /// Gets the coordinates of the subtexture from the main board texture in pixels
  /// The first four values are the x,y,w,h of the texture
  /// The second four values make up the padding of a 9-slice grid on the sides: 🡱🡲🡳🡰
  pub fn get_slice(&self) -> ((u32, u32, u32, u32), (u32, u32, u32, u32)) {
    match self {
      Self::Background => ((0, 0, 20, 20), (0, 0, 0, 0)),
      Self::MiniGridBorder => ((22, 0, 26, 18), (0, 10, 10, 10)),
      Self::NameTagBackground => ((50, 0, 20, 20), (0, 0, 0, 0)),
      Self::NameTagBackgroundOnFire => ((72, 0, 20, 20), (0, 0, 0, 0)),
      Self::DangerLine => ((96, 0, 13, 11), (0, 0, 0, 0)),
      Self::DangerGlow => ((96, 11, 13, 64), (0, 0, 0, 0)),
      Self::BoardGridBordersInnerBottom => ((111, 0, 27, 20), (0, 9, 9, 9)),
      Self::GarbageBar => ((142, 0, 27, 20), (0, 9, 9, 9)),
      Self::ProgressBar => ((173, 0, 27, 20), (0, 9, 9, 9)),
      Self::Stock => ((10, 30, 76, 76), (0, 0, 0, 0)),
      Self::Garbage => ((109, 30, 64, 56), (6, 0, 8, 0)),
      Self::Progress => ((173, 24, 64, 62), (32, 0, 0, 0)),
      Self::GarbageCap => ((111, 88, 60, 8), (0, 0, 0, 0)),
      Self::Warning => ((2, 118, 92, 92), (0, 0, 0, 0)),
      Self::Target => ((98, 100, 69, 69), (0, 0, 0, 0)),
      Self::PendingGarbage => ((173, 94, 64, 56), (6, 0, 8, 0)),
      Self::MegaBackground => ((256, 0, 256, 256), (0, 0, 0, 0)),
      Self::MegaForeground => ((258, 258, 252, 252), (0, 9, 9, 9))
    }
  }
}