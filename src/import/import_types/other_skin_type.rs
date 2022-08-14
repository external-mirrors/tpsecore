use std::fmt::{Display, Formatter};
use crate::tpse::{File, TPSE};

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

impl OtherSkinType {
  pub fn tpse_field<'a>(&'_ self, tpse: &'a mut TPSE) -> &'a mut Option<File> {
    match self {
      Self::Board => &mut tpse.board,
      Self::Queue => &mut tpse.queue,
      Self::Grid => &mut tpse.grid,
      Self::ParticleBeam => &mut tpse.particle_beam,
      Self::ParticleBeamsBeam => &mut tpse.particle_beams_beam,
      Self::ParticleBigBox => &mut tpse.particle_bigbox,
      Self::ParticleBox => &mut tpse.particle_box,
      Self::ParticleChip => &mut tpse.particle_chip,
      Self::ParticleChirp => &mut tpse.particle_chirp,
      Self::ParticleDust => &mut tpse.particle_dust,
      Self::ParticleFBox => &mut tpse.particle_fbox,
      Self::ParticleFire => &mut tpse.particle_fire,
      Self::ParticleParticle => &mut tpse.particle_particle,
      Self::ParticleSmoke => &mut tpse.particle_smoke,
      Self::ParticleStar => &mut tpse.particle_star,
      Self::ParticleFlake => &mut tpse.particle_flake,
      Self::RankD => &mut tpse.rank_d,
      Self::RankDPlus => &mut tpse.rank_dplus,
      Self::RankCMinus => &mut tpse.rank_cminus,
      Self::RankC => &mut tpse.rank_c,
      Self::RankCPlus => &mut tpse.rank_cplus,
      Self::RankBMinus => &mut tpse.rank_bminus,
      Self::RankB => &mut tpse.rank_b,
      Self::RankBPlus => &mut tpse.rank_bplus,
      Self::RankAMinus => &mut tpse.rank_aminus,
      Self::RankA => &mut tpse.rank_a,
      Self::RankAPlus => &mut tpse.rank_aplus,
      Self::RankSMinus => &mut tpse.rank_sminus,
      Self::RankS => &mut tpse.rank_s,
      Self::RankSPlus => &mut tpse.rank_splus,
      Self::RankSS => &mut tpse.rank_ss,
      Self::RankU => &mut tpse.rank_u,
      Self::RankX => &mut tpse.rank_x,
      Self::RankZ => &mut tpse.rank_z
    }
  }
}

impl Display for OtherSkinType {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", match self {
      OtherSkinType::Board => "board",
      OtherSkinType::Queue => "queue",
      OtherSkinType::Grid => "grid",
      OtherSkinType::ParticleBeam => "particle_beam",
      OtherSkinType::ParticleBeamsBeam => "particle_beams_beam",
      OtherSkinType::ParticleBigBox => "particle_big_box",
      OtherSkinType::ParticleBox => "particle_box",
      OtherSkinType::ParticleChip => "particle_chip",
      OtherSkinType::ParticleChirp => "particle_chirp",
      OtherSkinType::ParticleDust => "particle_dust",
      OtherSkinType::ParticleFBox => "particle_f_box",
      OtherSkinType::ParticleFire => "particle_fire",
      OtherSkinType::ParticleParticle => "particle_particle",
      OtherSkinType::ParticleSmoke => "particle_smoke",
      OtherSkinType::ParticleStar => "particle_star",
      OtherSkinType::ParticleFlake => "particle_flake",
      OtherSkinType::RankD => "rank_d",
      OtherSkinType::RankDPlus => "rank_d_plus",
      OtherSkinType::RankCMinus => "rank_c_minus",
      OtherSkinType::RankC => "rank_c",
      OtherSkinType::RankCPlus => "rank_c_plus",
      OtherSkinType::RankBMinus => "rank_b_minus",
      OtherSkinType::RankB => "rank_b",
      OtherSkinType::RankBPlus => "rank_b_plus",
      OtherSkinType::RankAMinus => "rank_a_minus",
      OtherSkinType::RankA => "rank_a",
      OtherSkinType::RankAPlus => "rank_a_plus",
      OtherSkinType::RankSMinus => "rank_s_minus",
      OtherSkinType::RankS => "rank_s",
      OtherSkinType::RankSPlus => "rank_s_plus",
      OtherSkinType::RankSS => "rank_ss",
      OtherSkinType::RankU => "rank_u",
      OtherSkinType::RankX => "rank_x",
      OtherSkinType::RankZ => "rank_z"
    })
  }
}