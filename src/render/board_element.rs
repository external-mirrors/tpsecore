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

  /// Gets the relative location the texture should be drawn to in a rendered board.
  /// Units are in blocks.
  pub fn get_target(&self) -> (i64, i64, i64, i64) {
    match self {
      Self::Background => (-13, -20, 501, 990),
      Self::MiniGridBorder => todo!(),
      Self::NameTagBackground => todo!(),
      Self::NameTagBackgroundOnFire => todo!(),
      Self::DangerLine => (-7, -20, 491, 4),
      Self::DangerGlow => (-7, -16, 491, 143),
      Self::BoardGridBordersInnerBottom => (-13, -20, 501, 990),
      Self::GarbageBar => (-51, -20, 44, 990),
      Self::ProgressBar => (484, -20, 44, 990),
      Self::Stock => (210, 1020, 54, 44),
      Self::Garbage => (-45, 865, 34, 99),
      Self::Progress => (490, -20, 32, 984),
      Self::GarbageCap => (-45, 569, 32, 4),
      Self::Warning => (-75, 406, 131, 131),
      Self::Target => (761, 15, 141, 140), // made up off-board value
      Self::PendingGarbage => (-45, 766, 34, 99),
      Self::MegaBackground => (-13, -20, 501, 990),
      Self::MegaForeground => (-13, -20, 501, 990)
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