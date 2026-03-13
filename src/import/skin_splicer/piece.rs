use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Copy, Clone, serde_with::DeserializeFromStr)]
pub enum Piece { Z, L, O, S, I, J, T, HoldDisabled, Garbage, DarkGarbage, Ghost, Topout }

impl Piece {
  pub fn values() -> &'static [Self] {
    use Piece::*;
    &[ Z, L, O, S, I, J, T, HoldDisabled, Garbage, DarkGarbage, Ghost, Topout ]
  }
}

pub struct PieceStringFailure(String);
impl Display for PieceStringFailure {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "unknown piece: {:?}", self.0)
  }
}
impl FromStr for Piece {
  type Err = PieceStringFailure;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "z" | "Z" => Ok(Self::Z),
      "l" | "L" => Ok(Self::L),
      "o" | "O" => Ok(Self::O),
      "s" | "S" => Ok(Self::S),
      "i" | "I" => Ok(Self::I),
      "j" | "J" => Ok(Self::J),
      "t" | "T" => Ok(Self::T),
      "hold" => Ok(Self::HoldDisabled),
      "garbage" | "gb" | "#" => Ok(Self::Garbage),
      "dark garbage" | "darkgarbage" | "dgb" | "@" => Ok(Self::DarkGarbage),
      "ghost" => Ok(Self::Ghost),
      "topout" => Ok(Self::Topout),
      other => Err(PieceStringFailure(other.to_string()))
    }
  }
}