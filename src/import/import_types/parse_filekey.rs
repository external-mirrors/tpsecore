use crate::import::{AnimatedOptions, ImportType};
use crate::import::ImportType::*;
use crate::import::SkinType::*;
use crate::import::OtherSkinType::*;

pub fn parse_filekey(filename: &str) -> Option<ImportType> {
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

#[cfg(test)]
mod test {
  use crate::import::{AnimatedOptions, SkinType, parse_filekey};
  use crate::import::ImportType::Skin;

  #[test]
  fn test_parse_filekey() {
    assert_eq!(parse_filekey("foo"), None);
    assert_eq!(
      parse_filekey("_animated_connected_minos_delay=20_combine=false"),
      Some(Skin {
        subtype: SkinType::Tetrio61ConnectedAnimated {
          opts: AnimatedOptions { delay: Some(20), combine: Some(false)}
        }
      })
    );
  }
}