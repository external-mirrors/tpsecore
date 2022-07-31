use std::borrow::Cow;
use std::ops::Sub;
use std::path::Path;
use std::str::FromStr;
use lazy_static::lazy_static;
use crate::import::ImportType::OtherSkin;
use regex::Regex;

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all="snake_case")]
pub enum ImportType {
  /// An import type will be decided automatically.
  /// This is the only way to import a zip or tpse file
  Automatic,
  Skin {
    #[serde(flatten)]
    subtype: SkinType
  },
  OtherSkin {
    #[serde(flatten)]
    subtype: OtherSkinType
  },
  SoundEffects,
  Background,
  Music
}

impl ImportType {
  pub fn from_filekey(filename: &str) -> Option<Self> {
    use ImportType::*;
    use SkinType::*;
    use OtherSkinType::*;

    let opts = AnimatedOptions::from(filename);

    if filename.contains("_unconnected_minos") {
      return Some(Skin { subtype: Tetrio61 });
    }
    if filename.contains("_unconnected_ghost") {
      return Some(Skin { subtype: Tetrio61Ghost });
    }
    if filename.contains("_connected_minos") {
      return Some(Skin { subtype: Tetrio61Connected });
    }
    if filename.contains("_connected_ghost") {
      return Some(Skin { subtype: Tetrio61ConnectedGhost });
    }
    if filename.contains("_animated_connected_minos") {
      return Some(Skin { subtype: Tetrio61ConnectedAnimated { opts }})
    }
    if filename.contains("_animated_connected_ghost") {
      return Some(Skin { subtype: Tetrio61ConnectedGhostAnimated { opts } })
    }
    if filename.contains("_old_tetrio") {
      return Some(Skin { subtype: TetrioRaster });
    }
    if filename.contains("_animated_old_tetrio") {
      return Some(Skin { subtype: TetrioAnimated { opts } });
    }
    if filename.contains("_jstris") {
      return Some(Skin { subtype: JstrisRaster });
    }
    if filename.contains("_animated_jstris") {
      return Some(Skin { subtype: JstrisAnimated { opts } });
    }
    if filename.contains("_board") {
      return Some(OtherSkin { subtype: Board });
    }
    if filename.contains("_queue") {
      return Some(OtherSkin { subtype: Queue });
    }
    if filename.contains("_grid") {
      return Some(OtherSkin { subtype: Grid });
    }
    if filename.contains("_particle_beam") {
      return Some(OtherSkin { subtype: ParticleBeam });
    }
    if filename.contains("_particle_beams_beam") {
      return Some(OtherSkin { subtype: ParticleBeamsBeam });
    }
    if filename.contains("_particle_bigbox") {
      return Some(OtherSkin { subtype: ParticleBigBox });
    }
    if filename.contains("_particle_box") {
      return Some(OtherSkin { subtype: ParticleBox });
    }
    if filename.contains("_particle_chip") {
      return Some(OtherSkin { subtype: ParticleChip });
    }
    if filename.contains("_particle_chirp") {
      return Some(OtherSkin { subtype: ParticleChirp });
    }
    if filename.contains("_particle_dust") {
      return Some(OtherSkin { subtype: ParticleDust });
    }
    if filename.contains("_particle_fbox") {
      return Some(OtherSkin { subtype: ParticleFBox });
    }
    if filename.contains("_particle_fire") {
      return Some(OtherSkin { subtype: ParticleFire });
    }
    if filename.contains("_particle_particle") {
      return Some(OtherSkin { subtype: ParticleParticle });
    }
    if filename.contains("_particle_smoke") {
      return Some(OtherSkin { subtype: ParticleSmoke });
    }
    return None;
  }
}


#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum SkinType {
  // The new tetrio formats used after TETR.IO v6.1.0
  Tetrio61,
  Tetrio61Ghost,
  Tetrio61Connected,
  Tetrio61ConnectedGhost,
  Tetrio61ConnectedAnimated { #[serde(flatten)] opts: AnimatedOptions },
  Tetrio61ConnectedGhostAnimated { #[serde(flatten)] opts: AnimatedOptions },

  // The old tetrio format used before TETR.IO v6.1.0
  TetrioSVG,
  TetrioRaster,
  TetrioAnimated { #[serde(flatten)] opts: AnimatedOptions },

  // The format used by jstris
  JstrisRaster,
  JstrisAnimated { #[serde(flatten)] opts: AnimatedOptions },

  // The connected format used by the jstris connected textures userscript
  // e.g. https://docs.google.com/document/d/1JCXhdDI7E1yvVaedr6b1gudXty8G7uRLWuyuZPQIGcs
  JstrisConnected
}

impl SkinType {
  pub fn guess_format(filename: &str, width: u32, height: u32) -> Option<SkinType> {
    let ext = Path::new(&filename).extension()
      .map(|ext| ext.to_string_lossy())
      .unwrap_or(Cow::from(filename));
    let opts = AnimatedOptions::from(filename);
    use SkinType::*;
    let likely_animated = ext.as_ref() == "gif" || opts.has_fields();
    let ratio = |target: f64| (width as f64 / height as f64).sub(target).abs() < 0.1;
    match (ext.as_ref(), width, height, likely_animated) {
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
      _ => todo!()
    }
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
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum OtherSkinType {
  Board,
  Queue,
  Grid,
  ParticleBeam,
  ParticleBeamsBeam,
  ParticleBigBox,
  ParticleBox,
  ParticleChip,
  ParticleChirp,
  ParticleDust,
  ParticleFBox,
  ParticleFire,
  ParticleParticle,
  ParticleSmoke,
  ParticleStar,
  ParticleFlake,
  RankD,
  RankDPlus,
  RankCMinus,
  RankC,
  RankCPlus,
  RankBMinus,
  RankB,
  RankBPlus,
  RankAMinus,
  RankA,
  RankAPlus,
  RankSMinus,
  RankS,
  RankSPlus,
  RankSS,
  RankU,
  RankX,
  RankZ
}

#[derive(Default, Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimatedOptions {
  /// A frame rate to override with. See `AnimMeta#delay`
  pub delay: Option<u32>,
  /// A combine frames setting to override with. Overrides any inferred gif combine setting.
  pub combine: Option<bool>
}
impl AnimatedOptions {
  pub fn has_fields(&self) -> bool {
    self.delay.is_some() || self.combine.is_some()
  }
}
impl From<&str> for AnimatedOptions {
  fn from(filename: &str) -> Self {
    lazy_static! {
      static ref DELAY_REGEX: Regex = Regex::new(r"_delay=(\d+)").unwrap();
      static ref COMBINE_REGEX: Regex = Regex::new(r"_combine=(true|false)").unwrap();
    }

    AnimatedOptions {
      delay: DELAY_REGEX.captures(filename).and_then(|matches| {
        matches.get(1).unwrap().as_str().parse().ok()
      }),
      combine: COMBINE_REGEX.captures(filename).map(|matches| {
        matches.get(1).unwrap().as_str().parse().unwrap()
      })
    }
  }
}

#[cfg(test)]
mod test {
  use crate::import::{AnimatedOptions, ImportType, SkinType};
  use crate::import::ImportType::Skin;

  #[test]
  fn from_filekey() {
    assert_eq!(ImportType::from_filekey("foo"), None);
    assert_eq!(
      ImportType::from_filekey("_animated_connected_minos_delay=20_combine=false"),
      Some(Skin {
        subtype: SkinType::Tetrio61ConnectedAnimated {
          opts: AnimatedOptions { delay: Some(20), combine: Some(false)}
        }
      })
    );
  }
}