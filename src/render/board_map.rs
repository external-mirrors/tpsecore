use std::str::FromStr;
use crate::import::skin_splicer::Piece;

#[derive(Debug, Clone)]
pub struct BoardMap {
  width: usize,
  contents: Vec<Option<(Piece, u8)>>
}

impl BoardMap {
  /// Iterates the contents of the map, yielding (row, col, Option<(piece, connections)>) tuples
  pub fn iter(&self) -> impl Iterator<Item = (usize, usize, Option<(Piece, u8)>)> + '_ {
    self.contents
      .chunks(self.width)
      .into_iter()
      .enumerate()
      .flat_map(|(row, chunk)| {
        chunk.into_iter()
          .enumerate()
          .map(move |(col, piece)| (row, col, piece.clone()))
      })
  }

  pub fn width(&self) -> usize {
    self.width
  }

  pub fn height(&self) -> usize {
    self.contents.len() / self.width
  }

  pub fn get(&self, col: usize, row: usize) -> Option<&(Piece, u8)> {
    if col >= self.width || row >= self.height() { return None }
    self.contents[col + row * self.width].as_ref()
  }

  pub fn get_mut(&mut self, col: usize, row: usize) -> Option<&mut (Piece, u8)> {
    if col >= self.width || row >= self.height() { return None }
    self.contents[col + row * self.width].as_mut()
  }
}

impl FromStr for BoardMap {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut rows = vec![];
    for row in s.split("\n") {
      let row = row.chars()
        .map(|char| Some((Piece::from_str(&char.to_string()).ok()?, 0)))
        .collect::<Vec<_>>();
      rows.push(row);
    }
    Ok(BoardMap::from(rows))
  }
}

impl From<&[&[Option<(Piece, u8)>]]> for BoardMap {
  fn from(array: &[&[Option<(Piece, u8)>]]) -> Self {
    match array.first() {
      None => BoardMap { width: 0, contents: vec![] },
      Some(first) => {
        let homogenous = array.iter().all(|el| el.len() == first.len());
        if !homogenous { panic!("Cannot create BoardMap from jagged array"); }
        BoardMap {
          width: first.len(),
          contents: array.iter().flat_map(|el| *el).copied().collect()
        }
      }
    }
  }
}

impl From<Vec<Vec<Option<(Piece, u8)>>>> for BoardMap {
  fn from(array: Vec<Vec<Option<(Piece, u8)>>>) -> Self {
    match array.first() {
      None => BoardMap { width: 0, contents: vec![] },
      Some(first) => {
        let homogenous = array.iter().all(|el| el.len() == first.len());
        if !homogenous { panic!("Cannot create BoardMap from jagged vec"); }
        BoardMap {
          width: first.len(),
          contents: array.iter().flatten().copied().collect()
        }
      }
    }
  }
}