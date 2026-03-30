use crate::import::AnimatedOptions;

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize, strum::Display)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum SkinType {
  // The new tetrio formats used after TETR.IO v6.1.0
  // all of these also have double resolution formats available at `{tetrio,connected}.2x.png`
  /// https://tetr.io/res/skins/minos/tetrio.png
  #[strum(to_string = "tetrio v6.1 unconnected minos")]
  Tetrio61,
  /// https://tetr.io/res/skins/ghost/tetrio.png
  #[strum(to_string = "tetrio v6.1 unconnected ghost")]
  Tetrio61Ghost,
  // https://tetr.io/res/skins/minos/connected.png
  #[strum(to_string = "tetrio v6.1 connected minos")]
  Tetrio61Connected,
  /// https://tetr.io/res/skins/ghost/connected.png
  #[strum(to_string = "tetrio v6.1 connected ghost")]
  Tetrio61ConnectedGhost,
  #[strum(to_string = "tetrio v6.1 connected animated {opts}")]
  Tetrio61ConnectedAnimated { #[serde(flatten)] opts: AnimatedOptions },
  #[strum(to_string = "tetrio v6.1 connected ghost animated {opts}")]
  Tetrio61ConnectedGhostAnimated { #[serde(flatten)] opts: AnimatedOptions },

  // The old tetrio format used before TETR.IO v6.1.0
  #[strum(to_string = "old tetrio svg")]
  TetrioSVG,
  #[strum(to_string = "old tetrio raster")]
  TetrioRaster,
  #[strum(to_string = "old tetrio animated {opts}")]
  TetrioAnimated { #[serde(flatten)] opts: AnimatedOptions },

  // The format used by jstris
  #[strum(to_string = "jstris")]
  JstrisRaster,
  #[strum(to_string = "jstris animated")]
  JstrisAnimated { #[serde(flatten)] opts: AnimatedOptions },

  // The connected format used by the jstris connected textures userscript
  // e.g. https://docs.google.com/document/d/1JCXhdDI7E1yvVaedr6b1gudXty8G7uRLWuyuZPQIGcs
  #[strum(to_string = "jstris connected")]
  JstrisConnected
}

impl SkinType {
  /// Returns the native size of the texture as a combination of the default size of the individual
  /// blocks (u32) and the size ratio of the canvas relative to those blocks (f64, f64). To get the
  /// actual size, just multiply them.
  pub fn get_native_texture_size(&self) -> (f64, f64, u32) {
    match self {
      SkinType::Tetrio61                              => ( 256.0/48.0,  256.0/48.0, 48),
      SkinType::Tetrio61Ghost                         => ( 128.0/48.0,  128.0/48.0, 48),
      SkinType::Tetrio61Connected                     => (1024.0/48.0, 1024.0/48.0, 48),
      SkinType::Tetrio61ConnectedGhost                => ( 512.0/48.0,  512.0/48.0, 48),
      SkinType::Tetrio61ConnectedAnimated { .. }      => (1024.0/48.0, 1024.0/48.0, 48),
      SkinType::Tetrio61ConnectedGhostAnimated { .. } => ( 512.0/48.0,  512.0/48.0, 48),
      SkinType::TetrioSVG                             => (12.4, 1.0, 30),
      SkinType::TetrioRaster                          => (12.4, 1.0, 30),
      SkinType::TetrioAnimated { .. }                 => (12.4, 1.0, 30),
      SkinType::JstrisRaster                          => ( 9.0, 1.0, 30),
      SkinType::JstrisAnimated { .. }                 => ( 9.0, 1.0, 30),
      SkinType::JstrisConnected                       => ( 9.0, 20.0, 32)
    }
  }
  
  pub fn canonical_tex_size(&self) -> Option<[u32; 2]> {
    match self {
      SkinType::Tetrio61                              => Some([ 256,  256]),
      SkinType::Tetrio61Ghost                         => Some([ 128,  256]),
      SkinType::Tetrio61Connected                     => Some([1024, 1024]),
      SkinType::Tetrio61ConnectedGhost                => Some([ 512,  512]),
      SkinType::Tetrio61ConnectedAnimated { .. }      => Some([1024, 1024]),
      SkinType::Tetrio61ConnectedGhostAnimated { .. } => Some([ 512,  512]),
      SkinType::TetrioSVG                             => Some([372, 30]),
      SkinType::TetrioRaster                          => Some([372, 30]),
      SkinType::TetrioAnimated { .. }                 => Some([372, 30]),
      SkinType::JstrisRaster                          => None,
      SkinType::JstrisAnimated { .. }                 => None,
      SkinType::JstrisConnected                       => None
    }
  }

  /// Returns the animated options for this skin format
  pub fn get_anim_options(&self) -> Option<AnimatedOptions> {
    match self {
      SkinType::Tetrio61 => None,
      SkinType::Tetrio61Ghost => None,
      SkinType::Tetrio61Connected => None,
      SkinType::Tetrio61ConnectedGhost => None,
      SkinType::Tetrio61ConnectedAnimated { opts } => Some(*opts),
      SkinType::Tetrio61ConnectedGhostAnimated { opts } => Some(*opts),
      SkinType::TetrioSVG => None,
      SkinType::TetrioRaster => None,
      SkinType::TetrioAnimated { opts } => Some(*opts),
      SkinType::JstrisRaster => None,
      SkinType::JstrisAnimated { opts } => Some(*opts),
      SkinType::JstrisConnected => None,
    }
  }
  
  pub fn has_minos_and_ghost(&self) -> (bool, bool) {
    match &self {
      SkinType::TetrioAnimated { .. }                 => ( true,  true),
      SkinType::Tetrio61ConnectedAnimated { .. }      => ( true, false),
      SkinType::Tetrio61ConnectedGhostAnimated { .. } => (false,  true),
      SkinType::JstrisAnimated { .. }                 => ( true,  true),
      SkinType::TetrioSVG                             => ( true,  true),
      SkinType::TetrioRaster                          => ( true,  true),
      SkinType::Tetrio61                              => ( true, false),
      SkinType::Tetrio61Ghost                         => (false,  true),
      SkinType::Tetrio61Connected                     => ( true, false),
      SkinType::Tetrio61ConnectedGhost                => (false,  true),
      SkinType::JstrisRaster                          => ( true,  true),
      SkinType::JstrisConnected                       => ( true,  true)
    }
  }
}


