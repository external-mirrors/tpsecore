use std::ops::Sub;
use std::path::Path;

use arrayvec::ArrayVec;

use crate::accel::traits::TPSEAccelerator;
use crate::import::{AnimatedOptions, ImportContext, OtherSkinType, SkinType};
use crate::log::LogLevel;

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct TextureGuess {
  pub width: u32,
  pub height: u32,
  pub kind: TextureGuessKind,
}

impl TextureGuess {
  pub(in crate) fn dim(&self) -> [u32; 2] {
    [self.width, self.height]
  }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize, strum::Display)]
pub enum TextureGuessKind {
  #[strum(to_string = "{0}")]
  Skin(SkinType),
  #[strum(to_string = "{0}")]
  Other(OtherSkinType)
}

pub const MAX_POSSIBLE_TEXTURE_GUESSES: usize = 3;

pub fn guess_texture_format<T: TPSEAccelerator>
  (filename: &Path, width: u32, height: u32, ctx: &ImportContext<T>)
  -> ArrayVec<TextureGuess, MAX_POSSIBLE_TEXTURE_GUESSES>
{
  let ext = Path::new(&filename).extension().and_then(|x| x.to_str());
  let opts = AnimatedOptions::from(filename);
  let likely_animated = ext == Some("gif") || opts.has_fields();
  
  let mut guesses: ArrayVec<_, MAX_POSSIBLE_TEXTURE_GUESSES> = Default::default();
  guesses.extend(guess_skin(ext, width, height, likely_animated, false, opts).map(|x| TextureGuessKind::Skin(x)));
  guesses.extend(guess_skin(ext, width*2, height*2, likely_animated, true, opts).map(|x| TextureGuessKind::Skin(x)));
  guesses.extend(guess_other(width, height).map(|x| TextureGuessKind::Other(x)));
  
  ctx.log(LogLevel::Debug, format_args!(
    "Guessing format for ext={:?} w={} h={} anim={}: {:?}",
    ext, width, height, likely_animated, guesses
  ));
  ArrayVec::from_iter(guesses.into_iter().map(|kind| TextureGuess { width, height, kind }))
}

fn guess_skin
  (ext: Option<&str>, width: u32, height: u32, likely_animated: bool, test_2x: bool, opts: AnimatedOptions)
  -> Option<SkinType>
{
  let ratio = |target: f64| (width as f64 / height as f64).sub(target).abs() < 0.1;
  
  use SkinType::*;
  match (ext, width, height, likely_animated, test_2x) {
    (          _,1024,1024, true,     _) => Some(Tetrio61ConnectedAnimated { opts }),
    (          _,1024,1024,    _,     _) => Some(Tetrio61Connected),
    (          _, 512, 512, true,     _) => Some(Tetrio61ConnectedGhostAnimated { opts }),
    (          _, 512, 512,    _,     _) => Some(Tetrio61ConnectedGhost),
    (          _, 256, 256,    _, false) => Some(Tetrio61),
    (          _, 128, 128,    _, false) => Some(Tetrio61Ghost),
    (          _, 372,  30, true, false) => Some(TetrioAnimated { opts }),
    (Some("svg"), 372,  30,    _, false) => Some(TetrioSVG),
    (          _, 372,  30,    _, false) => Some(TetrioRaster),
    (          _, 288, 640,    _, false) => Some(JstrisConnected), // 32px size
    (          _, 216, 480,    _, false) => Some(JstrisConnected), // 24px size
    (          _,   _,   _, true, false) if ratio(12.4) => Some(TetrioAnimated { opts }),
    (Some("svg"),   _,   _,    _, false) if ratio(12.4) => Some(TetrioSVG),
    (          _,   _,   _,    _, false) if ratio(12.4) => Some(TetrioRaster),
    (          _,   _,   _, true, false) if ratio(9.0) => Some(JstrisAnimated { opts }),
    (          _,   _,   _,    _, false) if ratio(9.0) => Some(JstrisConnected),
    (          _,   _,   _,    _, false) if ratio(9.0/20.0) => Some(JstrisConnected),
    _ => None
  }
}

fn guess_other(width: u32, height: u32) -> Option<OtherSkinType> {
  match (width, height) {
    (512, 512) => Some(OtherSkinType::Board),
    // (512, 512) => Some(OtherSkinType::Queue), // ambiguous :(
    (1024, 1024) => Some(OtherSkinType::Grid),
    
    (12, 2) => Some(OtherSkinType::ParticleBeam),
    (300, 3) => Some(OtherSkinType::ParticleBigBox),
    // (128, 1) => Some(OtherSkinType::boardstar), // needs to be added
    // (44, 4) => Some(OtherSkinType::bokeh),
    (30, 3) => Some(OtherSkinType::ParticleBox),
    // (64, 6) => Some(OtherSkinType::chain-a),
    // (64, 6) => Some(OtherSkinType::chain-b),
    // (64, 6) => Some(OtherSkinType::chain-c),
    // (64, 6) => Some(OtherSkinType::wound-spark-1),
    // (64, 6) => Some(OtherSkinType::wound-spark-2),
    // (64, 6) => Some(OtherSkinType::wound-spark-3),
    // (64, 6) => Some(OtherSkinType::wound-particle-1),
    // (64, 6) => Some(OtherSkinType::wound-particle-2),
    // (64, 6) => Some(OtherSkinType::wound-particle-3),
    // (64, 6) => Some(OtherSkinType::wound-particle-4),
    (32, 3) => Some(OtherSkinType::ParticleChip),
    // (32, 3) => Some(OtherSkinType::ParticleChirp), // ambiguous :(
    (128, 1) => Some(OtherSkinType::ParticleDust),
    // (128, 1) => Some(OtherSkinType::exhaust),
    (64, 6) => Some(OtherSkinType::ParticleFire),
    // (110, 1) => Some(OtherSkinType::flare),
    (92, 9) => Some(OtherSkinType::ParticleFBox),
    (44, 4) => Some(OtherSkinType::ParticleParticle),
    // (128, 1) => Some(OtherSkinType::ParticleSmoke), // ambiguous :(
    // (32, 3) => Some(OtherSkinType::ParticleStar), // ambiguous :(
    // (110, 1) => Some(OtherSkinType::spark),
    (133, 4) => Some(OtherSkinType::ParticleBeamsBeam),
    // (1280, 3) => Some(OtherSkinType::beams/spark),
    // (133, 4) => Some(OtherSkinType::beams/sparkoff),
    _ => None
  }
}