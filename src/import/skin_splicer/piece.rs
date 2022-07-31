use std::str::FromStr;

#[derive(Debug, Copy, Clone)]
pub enum Piece { Z, L, O, S, I, J, T, HoldDisabled, Garbage, DarkGarbage, Ghost, Topout }

impl Piece {
  pub fn values() -> &'static [Self] {
    use Piece::*;
    &[ Z, L, O, S, I, J, T, HoldDisabled, Garbage, DarkGarbage, Ghost, Topout ]
  }
}

impl FromStr for Piece {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "z" => Ok(Self::Z),
      "l" => Ok(Self::L),
      "o" => Ok(Self::O),
      "s" => Ok(Self::S),
      "i" => Ok(Self::I),
      "j" => Ok(Self::J),
      "t" => Ok(Self::T),
      "hold" => Ok(Self::HoldDisabled),
      "gb" => Ok(Self::Garbage),
      "dgb" => Ok(Self::DarkGarbage),
      "ghost" => Ok(Self::Ghost),
      "topout" => Ok(Self::Topout),
      _ => Err(())
    }
  }
}