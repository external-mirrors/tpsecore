use std::fmt::{Debug, Display, Formatter};

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Asset {
  /// The main TETR.IO source code file, located at https://tetr.io/js/tetrio.js
  #[serde(alias = "tetrio.js")]
  TetrioJS = 0,
  /// The TETR.IO sound effects file, located at https://tetr.io/sfx/tetrio.opus.rsd
  #[serde(alias = "tetrio.opus.rsd")]
  TetrioRSD = 1
}
impl TryFrom<u8> for Asset {
  type Error = ();

  fn try_from(value: u8) -> Result<Self, Self::Error> {
    match value {
      0 => Ok(Self::TetrioJS),
      1 => Ok(Self::TetrioRSD),
      _ => Err(())
    }
  }
}
impl Display for Asset {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Asset::TetrioJS => write!(f, "tetrio.js"),
      Asset::TetrioRSD => write!(f, "tetrio.opus.rsd")
    }
  }
}