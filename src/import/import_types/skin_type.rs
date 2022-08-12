use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::ops::Sub;
use std::path::Path;
use log::Level;
use crate::import::{AnimatedOptions, ImportContext};
use crate::tpse::AnimMeta;

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize, thiserror::Error)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum SkinType {
  // The new tetrio formats used after TETR.IO v6.1.0
  #[error("tetrio v6.1 unconnected minos")]
  Tetrio61,
  #[error("tetrio v6.1 unconnected ghost")]
  Tetrio61Ghost,
  #[error("tetrio v6.1 connected minos")]
  Tetrio61Connected,
  #[error("tetrio v6.1 connected ghost")]
  Tetrio61ConnectedGhost,
  #[error("tetrio v6.1 connected animated {opts}")]
  Tetrio61ConnectedAnimated { #[serde(flatten)] opts: AnimatedOptions },
  #[error("tetrio v6.1 connected ghost animated {opts}")]
  Tetrio61ConnectedGhostAnimated { #[serde(flatten)] opts: AnimatedOptions },

  // The old tetrio format used before TETR.IO v6.1.0
  #[error("old tetrio svg")]
  TetrioSVG,
  #[error("old tetrio raster")]
  TetrioRaster,
  #[error("old tetrio animated {opts}")]
  TetrioAnimated { #[serde(flatten)] opts: AnimatedOptions },

  // The format used by jstris
  #[error("jstris")]
  JstrisRaster,
  #[error("jstris animated")]
  JstrisAnimated { #[serde(flatten)] opts: AnimatedOptions },

  // The connected format used by the jstris connected textures userscript
  // e.g. https://docs.google.com/document/d/1JCXhdDI7E1yvVaedr6b1gudXty8G7uRLWuyuZPQIGcs
  #[error("jstris connected")]
  JstrisConnected
}

impl SkinType {
  pub fn guess_format(filename: &str, width: u32, height: u32, ctx: &ImportContext) -> Option<SkinType> {
    let ext = Path::new(&filename).extension()
      .map(|ext| ext.to_string_lossy())
      .unwrap_or(Cow::from(filename));
    let opts = AnimatedOptions::from(filename);
    use SkinType::*;
    let likely_animated = ext.as_ref() == "gif" || opts.has_fields();
    let ratio = |target: f64| (width as f64 / height as f64).sub(target).abs() < 0.1;
    let result = match (ext.as_ref(), width, height, likely_animated) {
      (    _, 256, 256, true) => Some(Tetrio61ConnectedAnimated { opts }),
      (    _, 256, 256,    _) => Some(Tetrio61Connected),
      (    _, 128, 128, true) => Some(Tetrio61ConnectedGhostAnimated { opts }),
      (    _, 128, 128,    _) => Some(Tetrio61ConnectedGhost),
      (    _, 372,  30, true) => Some(TetrioAnimated { opts }),
      ("svg", 372,  30,    _) => Some(TetrioSVG),
      (    _, 372,  30,    _) => Some(TetrioRaster),
      (    _, 288, 640,    _) => Some(JstrisConnected), // 32px size
      (    _, 216, 480,    _) => Some(JstrisConnected), // 24px size
      (    _,   _,   _, true) if ratio(12.4) => Some(TetrioAnimated { opts }),
      ("svg",   _,   _,    _) if ratio(12.4) => Some(TetrioSVG),
      (    _,   _,   _,    _) if ratio(12.4) => Some(TetrioRaster),
      (    _,   _,   _, true) if ratio(9.0) => Some(JstrisAnimated { opts }),
      (    _,   _,   _,    _) if ratio(9.0) => Some(JstrisConnected),
      (    _,   _,   _,    _) if ratio(9.0/20.0) => Some(JstrisConnected),
      _ => None
    };
    ctx.log(Level::Debug, format_args!(
      "Guessing format for ext={} w={} h={} anim={}: {:?}",
      ext, width, height, likely_animated, result
    ));
    result
  }

  /// Returns the native size of the texture as a combination of the default size of the individual
  /// blocks (u32) and the size ratio of the canvas relative to those blocks (f64, f64). To get the
  /// actual size, just multiply them.
  pub fn get_native_texture_size(&self) -> (f64, f64, u32) {
    match self {
      SkinType::Tetrio61                              => ( 256.0/48.0,  256.0/48.0, 48),
      SkinType::Tetrio61Ghost                         => ( 128.0/48.0,  256.0/48.0, 48),
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

  /// Returns the animated options for this skin format, returning an object with all `None`s if
  /// not an animated format.
  pub fn get_anim_options(&self) -> AnimatedOptions {
    let opts = match self {
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
    };
    AnimatedOptions {
      delay: opts.and_then(|opt| opt.delay),
      combine: opts.and_then(|opt| opt.combine)
    }
  }
}