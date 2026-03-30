use std::fmt::Display;
use std::path::Path;

use crate::import::{AnimatedOptions, BackgroundType, SkinType, OtherSkinType};
use crate::import::SkinType::*;
use crate::import::OtherSkinType::*;
use subenum::subenum;

/// An ImportType is metadata describing how a single file should be imported.
/// Several subvariants of ImportType exist which describe the possible types at different import stages.
/// - [ImportType]: Fed to explore_files/decide_specific_type
/// - [TypeStage1]: returned from decide_specific_type (-Automatic). This is also where filekey parsing happens.
/// - [TypeStage2]: returned from explore_files (-Zip)
/// - [TypeStage3]: returned from partition_import_groups, wrapped in DecisionTree (-PackJson)
/// - [TypeStage4]: returned from reduce_types, wrapped in ImportTask (-Unknown -SoundEffects)
#[subenum(TypeStage1, TypeStage2, TypeStage3, TypeStage4)]
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all="snake_case")]
pub enum ImportType {
  Automatic,
  #[subenum(TypeStage1)]
  Zip,
  #[subenum(TypeStage1, TypeStage2)]
  PackJson,
  #[subenum(TypeStage1, TypeStage2, TypeStage3)]
  Unknown,
  #[subenum(TypeStage1, TypeStage2, TypeStage3, TypeStage4)]
  TPSE,
  #[subenum(TypeStage1, TypeStage2, TypeStage3, TypeStage4)]
  Skin {
    #[serde(flatten)]
    subtype: SkinType
  },
  #[subenum(TypeStage1, TypeStage2, TypeStage3, TypeStage4)]
  OtherSkin {
    #[serde(flatten)]
    subtype: OtherSkinType
  },
  #[subenum(TypeStage1, TypeStage2, TypeStage3)]
  SoundEffects,
  #[subenum(TypeStage1, TypeStage2, TypeStage3, TypeStage4)]
  Background {
    #[serde(flatten)]
    subtype: BackgroundType
  },
  #[subenum(TypeStage1, TypeStage2, TypeStage3, TypeStage4)]
  Music
}

// this display impl is used for serializing the import type as part of ImportContextEntry
impl Display for ImportType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Automatic => write!(f, "automatic"),
      Self::PackJson => write!(f, "pack.json"),
      Self::Unknown => write!(f, "unknown"),
      Self::TPSE => write!(f, "tpse"),
      Self::Zip => write!(f, "zip"),
      Self::Skin { subtype } => write!(f, "{subtype} skin"),
      Self::OtherSkin { subtype } => write!(f, "{subtype} skin"),
      Self::SoundEffects => write!(f, "sound effect"),
      Self::Background { subtype: BackgroundType::Video } => write!(f, "video background"),
      Self::Background { subtype: BackgroundType::Image } => write!(f, "background"),
      Self::Music => write!(f, "music")
    }
  }
}
impl Display for TypeStage1 {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", ImportType::from(*self))
  }
}
impl Display for TypeStage2 {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", ImportType::from(*self))
  }
}
impl Display for TypeStage3 {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", ImportType::from(*self))
  }
}


// todo: maybe generate this with a macro from the below switch statement?
const POSSIBILITIES: [fn(AnimatedOptions) -> TypeStage1; 54] = {
  use TypeStage1::*;
  [
    |_opts| Unknown,
    |_opts| PackJson,
    |_opts| TPSE,
    |_opts| Zip,
    |_opts| Skin { subtype: Tetrio61 },
    |_opts| Skin { subtype: Tetrio61Ghost },
    |_opts| Skin { subtype: Tetrio61Connected },
    |_opts| Skin { subtype: Tetrio61ConnectedGhost },
    | opts| Skin { subtype: Tetrio61ConnectedAnimated { opts } },
    | opts| Skin { subtype: Tetrio61ConnectedGhostAnimated { opts } },
    |_opts| Skin { subtype: TetrioSVG },
    |_opts| Skin { subtype: TetrioRaster },
    | opts| Skin { subtype: TetrioAnimated { opts } },
    |_opts| Skin { subtype: JstrisRaster },
    | opts| Skin { subtype: JstrisAnimated { opts } },
    |_opts| Skin { subtype: JstrisConnected },
    |_opts| OtherSkin { subtype: Board },
    |_opts| OtherSkin { subtype: Queue },
    |_opts| OtherSkin { subtype: Grid },
    |_opts| OtherSkin { subtype: ParticleBeam },
    |_opts| OtherSkin { subtype: ParticleBeamsBeam },
    |_opts| OtherSkin { subtype: ParticleBigBox },
    |_opts| OtherSkin { subtype: ParticleBox },
    |_opts| OtherSkin { subtype: ParticleChip },
    |_opts| OtherSkin { subtype: ParticleChirp },
    |_opts| OtherSkin { subtype: ParticleDust },
    |_opts| OtherSkin { subtype: ParticleFBox },
    |_opts| OtherSkin { subtype: ParticleFire },
    |_opts| OtherSkin { subtype: ParticleParticle },
    |_opts| OtherSkin { subtype: ParticleSmoke },
    |_opts| OtherSkin { subtype: ParticleStar },
    |_opts| OtherSkin { subtype: ParticleFlake },
    |_opts| OtherSkin { subtype: RankD },
    |_opts| OtherSkin { subtype: RankDPlus },
    |_opts| OtherSkin { subtype: RankCMinus },
    |_opts| OtherSkin { subtype: RankC },
    |_opts| OtherSkin { subtype: RankCPlus },
    |_opts| OtherSkin { subtype: RankBMinus },
    |_opts| OtherSkin { subtype: RankB },
    |_opts| OtherSkin { subtype: RankBPlus },
    |_opts| OtherSkin { subtype: RankAMinus },
    |_opts| OtherSkin { subtype: RankA },
    |_opts| OtherSkin { subtype: RankAPlus },
    |_opts| OtherSkin { subtype: RankSMinus },
    |_opts| OtherSkin { subtype: RankS },
    |_opts| OtherSkin { subtype: RankSPlus },
    |_opts| OtherSkin { subtype: RankSS },
    |_opts| OtherSkin { subtype: RankU },
    |_opts| OtherSkin { subtype: RankX },
    |_opts| OtherSkin { subtype: RankZ },
    |_opts| SoundEffects,
    |_opts| Music,
    |_opts| Background { subtype: BackgroundType::Image },
    |_opts| Background { subtype: BackgroundType::Video }
  ]
};

impl TypeStage1 {
  /// Returns the file key for the given import type
  pub fn filekey(&self) -> &'static str {
    match self {
      // no point to using this one, but it's nice to have the function be complete
      Self::Unknown => "__unknown",
      // pack json files are primarily identified through being named exactly `pack.json`
      Self::PackJson => "__packjson",
      // tpse files are primarily identified through their file extension of `.tpse`
      Self::TPSE => "__tpse",
      // zip files are primarily identified through their extension of `.zip`
      Self::Zip => "__zip",
      Self::Skin { subtype: Tetrio61 } => "_unconnected_minos",
      Self::Skin { subtype: Tetrio61Ghost } => "_unconnected_ghost",
      Self::Skin { subtype: Tetrio61Connected } => "_connected_minos",
      Self::Skin { subtype: Tetrio61ConnectedGhost } => "_connected_ghost",
      // todo: Not sure exactly how animated skin filekeys will work out?
      // potential things that come up: this[0] is called `flow_connected_minos`, so it'll import
      // as a *non*-animated skin.
      // [0] https://you.have.fail/ed/at/tetrioplus/#skin-Haley_Halcyon-loop_connected_minos
      Self::Skin { subtype: Tetrio61ConnectedAnimated { .. } } => "_animated_connected_minos",
      Self::Skin { subtype: Tetrio61ConnectedGhostAnimated { .. } } => "_animated_connected_ghost",
      Self::Skin { subtype: TetrioSVG } => "_old_tetrio_svg",
      Self::Skin { subtype: TetrioRaster } => "_old_tetrio",
      Self::Skin { subtype: TetrioAnimated { .. } } => "_animated_old_tetrio",
      Self::Skin { subtype: JstrisRaster } => "_jstris",
      Self::Skin { subtype: JstrisAnimated { .. } } => "_animated_jstris",
      Self::Skin { subtype: JstrisConnected } => "_connected_jstris",
      Self::OtherSkin { subtype: Board } => "_board",
      Self::OtherSkin { subtype: Queue } => "_queue",
      Self::OtherSkin { subtype: Grid } => "_grid",
      Self::OtherSkin { subtype: ParticleBeam } => "_particle_beam",
      Self::OtherSkin { subtype: ParticleBeamsBeam } => "_particle_beams_beam",
      Self::OtherSkin { subtype: ParticleBigBox } => "_particle_bigbox",
      Self::OtherSkin { subtype: ParticleBox } => "_particle_box",
      Self::OtherSkin { subtype: ParticleChip } => "_particle_chip",
      Self::OtherSkin { subtype: ParticleChirp } => "_particle_chirp",
      Self::OtherSkin { subtype: ParticleDust } => "_particle_dust",
      Self::OtherSkin { subtype: ParticleFBox } => "_particle_fbox",
      Self::OtherSkin { subtype: ParticleFire } => "_particle_fire",
      Self::OtherSkin { subtype: ParticleParticle } => "_particle_particle",
      Self::OtherSkin { subtype: ParticleSmoke } => "_particle_smoke",
      Self::OtherSkin { subtype: ParticleStar } => "_particle_star",
      Self::OtherSkin { subtype: ParticleFlake } => "_particle_flake",
      Self::OtherSkin { subtype: RankD } => "_rank_d",
      Self::OtherSkin { subtype: RankDPlus } => "_rank_d_plus",
      Self::OtherSkin { subtype: RankCMinus } => "_rank_c_minus",
      Self::OtherSkin { subtype: RankC } => "_rank_c",
      Self::OtherSkin { subtype: RankCPlus } => "_rank_c_plus",
      Self::OtherSkin { subtype: RankBMinus } => "_rank_b_minus",
      Self::OtherSkin { subtype: RankB } => "_rank_b",
      Self::OtherSkin { subtype: RankBPlus } => "_rank_b_plus",
      Self::OtherSkin { subtype: RankAMinus } => "_rank_a_minus",
      Self::OtherSkin { subtype: RankA } => "_rank_a",
      Self::OtherSkin { subtype: RankAPlus } => "_rank_a_plus",
      Self::OtherSkin { subtype: RankSMinus } => "_rank_s_minus",
      Self::OtherSkin { subtype: RankS } => "_rank_s",
      Self::OtherSkin { subtype: RankSPlus } => "_rank_s_plus",
      Self::OtherSkin { subtype: RankSS } => "_rank_ss",
      Self::OtherSkin { subtype: RankU } => "_rank_u",
      Self::OtherSkin { subtype: RankX } => "_rank_x",
      Self::OtherSkin { subtype: RankZ } => "_rank_z",
      Self::SoundEffects => "_sfx",
      Self::Music => "_music",
      Self::Background { subtype: BackgroundType::Image } => "_background",
      Self::Background { subtype: BackgroundType::Video } => "_video_background"
    }
  }

  /// Creates an `TypeStage1` by parsing filekeys from the given filename
  /// Note that longer filekeys win - e.g. `_old_tetrio_svg` beats `_old_tetrio`.
  pub fn parse_filekey(filename: &Path) -> Option<Self> {
    let filename = filename.to_string_lossy(); // get that filekey no matter how mangled the filename is
    let opts = AnimatedOptions::from(filename.as_ref());
    POSSIBILITIES.iter()
      .map(|el| (el)(opts))
      .filter(|el| filename.contains(el.filekey()))
      .max_by_key(|el| el.filekey().len())
  }
}

#[cfg(test)]
mod test {
  use std::path::Path;
  use crate::import::{AnimatedOptions, SkinType, TypeStage1};

  #[test]
  fn test_parse_filekey() {
    assert_eq!(TypeStage1::parse_filekey(Path::new("foo")), None);
    assert_eq!(
      TypeStage1::parse_filekey(Path::new("_animated_connected_minos_delay=20_combine=false")),
      Some(TypeStage1::Skin {
        subtype: SkinType::Tetrio61ConnectedAnimated {
          opts: AnimatedOptions { delay: Some(20), combine: Some(false)}
        }
      })
    );
  }
}