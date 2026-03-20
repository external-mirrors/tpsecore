use std::path::Path;

use crate::import::{AnimatedOptions, BackgroundType, SkinType, OtherSkinType};
use crate::import::SkinType::*;
use crate::import::OtherSkinType::*;
use ImportType::*;

/// An ImportType is metadata describing how a single file should be imported
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
  Background {
    #[serde(flatten)]
    subtype: BackgroundType
  },
  Music
}

// this display impl is used for serializing the import type as part of ImportContextEntry
impl std::fmt::Display for ImportType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Automatic => write!(f, "automatic"),
      Self::Skin { subtype } => write!(f, "{subtype} skin"),
      Self::OtherSkin { subtype } => write!(f, "{subtype} skin"),
      Self::SoundEffects => write!(f, "sound effect"),
      Self::Background { subtype: BackgroundType::Video } => write!(f, "video background"),
      Self::Background { subtype: BackgroundType::Image } => write!(f, "background"),
      Self::Music => write!(f, "music"),
    }
  }
}

// todo: maybe generate this with a macro from the below switch statement?
const POSSIBILITIES: [fn(AnimatedOptions) -> ImportType; 50] = [
  //|_opts| Automatic,
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
];

impl ImportType {
  /// Returns the file key for the given import type
  pub fn filekey(&self) -> &'static str {
    use ImportType::*;
    match self {
      // no real point to using this one, but it's nice to have the function be complete
      Automatic => "__automatic",
      Skin { subtype: Tetrio61 } => "_unconnected_minos",
      Skin { subtype: Tetrio61Ghost } => "_unconnected_ghost",
      Skin { subtype: Tetrio61Connected } => "_connected_minos",
      Skin { subtype: Tetrio61ConnectedGhost } => "_connected_ghost",
      // todo: Not sure exactly how animated skin filekeys will work out?
      // potential things that come up: this[0] is called `flow_connected_minos`, so it'll import
      // as a *non*-animated skin.
      // [0] https://you.have.fail/ed/at/tetrioplus/#skin-Haley_Halcyon-loop_connected_minos
      Skin { subtype: Tetrio61ConnectedAnimated { .. } } => "_animated_connected_minos",
      Skin { subtype: Tetrio61ConnectedGhostAnimated { .. } } => "_animated_connected_ghost",
      Skin { subtype: TetrioSVG } => "_old_tetrio_svg",
      Skin { subtype: TetrioRaster } => "_old_tetrio",
      Skin { subtype: TetrioAnimated { .. } } => "_animated_old_tetrio",
      Skin { subtype: JstrisRaster } => "_jstris",
      Skin { subtype: JstrisAnimated { .. } } => "_animated_jstris",
      Skin { subtype: JstrisConnected } => "_connected_jstris",
      OtherSkin { subtype: Board } => "_board",
      OtherSkin { subtype: Queue } => "_queue",
      OtherSkin { subtype: Grid } => "_grid",
      OtherSkin { subtype: ParticleBeam } => "_particle_beam",
      OtherSkin { subtype: ParticleBeamsBeam } => "_particle_beams_beam",
      OtherSkin { subtype: ParticleBigBox } => "_particle_bigbox",
      OtherSkin { subtype: ParticleBox } => "_particle_box",
      OtherSkin { subtype: ParticleChip } => "_particle_chip",
      OtherSkin { subtype: ParticleChirp } => "_particle_chirp",
      OtherSkin { subtype: ParticleDust } => "_particle_dust",
      OtherSkin { subtype: ParticleFBox } => "_particle_fbox",
      OtherSkin { subtype: ParticleFire } => "_particle_fire",
      OtherSkin { subtype: ParticleParticle } => "_particle_particle",
      OtherSkin { subtype: ParticleSmoke } => "_particle_smoke",
      OtherSkin { subtype: ParticleStar } => "_particle_star",
      OtherSkin { subtype: ParticleFlake } => "_particle_flake",
      OtherSkin { subtype: RankD } => "_rank_d",
      OtherSkin { subtype: RankDPlus } => "_rank_d_plus",
      OtherSkin { subtype: RankCMinus } => "_rank_c_minus",
      OtherSkin { subtype: RankC } => "_rank_c",
      OtherSkin { subtype: RankCPlus } => "_rank_c_plus",
      OtherSkin { subtype: RankBMinus } => "_rank_b_minus",
      OtherSkin { subtype: RankB } => "_rank_b",
      OtherSkin { subtype: RankBPlus } => "_rank_b_plus",
      OtherSkin { subtype: RankAMinus } => "_rank_a_minus",
      OtherSkin { subtype: RankA } => "_rank_a",
      OtherSkin { subtype: RankAPlus } => "_rank_a_plus",
      OtherSkin { subtype: RankSMinus } => "_rank_s_minus",
      OtherSkin { subtype: RankS } => "_rank_s",
      OtherSkin { subtype: RankSPlus } => "_rank_s_plus",
      OtherSkin { subtype: RankSS } => "_rank_ss",
      OtherSkin { subtype: RankU } => "_rank_u",
      OtherSkin { subtype: RankX } => "_rank_x",
      OtherSkin { subtype: RankZ } => "_rank_z",
      SoundEffects => "_sfx",
      Music => "_music",
      Background { subtype: BackgroundType::Image } => "_background",
      Background { subtype: BackgroundType::Video } => "_video_background",
    }
  }

  /// Creates an `ImportType` by parsing filekeys from the given filename
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

  use crate::import::{AnimatedOptions, SkinType, ImportType};
  // use crate::import::import_types::import_type::POSSIBILITIES;
  use crate::import::ImportType::Skin;

  #[test]
  fn test_parse_filekey() {
    assert_eq!(ImportType::parse_filekey(Path::new("foo")), None);
    assert_eq!(
      ImportType::parse_filekey(Path::new("_animated_connected_minos_delay=20_combine=false")),
      Some(Skin {
        subtype: SkinType::Tetrio61ConnectedAnimated {
          opts: AnimatedOptions { delay: Some(20), combine: Some(false)}
        }
      })
    );
  }

  // #[test]
  // fn test_filekey_ambiguity() {
  //   let opts = AnimatedOptions::default();
  //   for possibility in POSSIBILITIES {
  //     for next_possibility in POSSIBILITIES {
  //       if possibility == next_possibility {
  //         continue;
  //       }
  //       let left = possibility(opts).filekey();
  //       let right = next_possibility(opts).filekey();
  //       if left.contains(right) {
  //         panic!(
  //           "File key collision: \"{}\" contains \"{}\"! File keys should be unique and should not \
  //           contain substrings of other filekeys to avoid ambiguity while parsing.",
  //           left, right
  //         );
  //       }
  //     }
  //   }
  // }
}