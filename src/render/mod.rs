mod board_element;

use image::{DynamicImage, GenericImage, GenericImageView};
use image::imageops::FilterType;
pub use board_element::BoardElement;
use crate::import::{LoadError, SkinType};
use crate::import::skin_splicer::{decode_image, Piece, SkinSplicer};
use crate::tpse::TPSE;

const E: Option<Piece> = None;
const Z: Option<Piece> = Some(Piece::Z);
const L: Option<Piece> = Some(Piece::L);
const O: Option<Piece> = Some(Piece::O);
const S: Option<Piece> = Some(Piece::S);
const I: Option<Piece> = Some(Piece::I);
const J: Option<Piece> = Some(Piece::J);
const T: Option<Piece> = Some(Piece::T);
const G: Option<Piece> = Some(Piece::Garbage);
const D: Option<Piece> = Some(Piece::DarkGarbage);
const W: Option<Piece> = Some(Piece::Topout);
const PCO_MAP: &[&[(Option<Piece>, u8)]] = &[
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (W, 0b00010), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 24 (skyline)
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (W, 0b01010), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 23 (skyline)
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (W, 0b01010), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 22 (skyline)
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (W, 0b01000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 21 (skyline)
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 20
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (T, 0b00010), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 19
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (T, 0b00100), (T, 0b11011), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 18
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (T, 0b01000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 17
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 16
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 15
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 14
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 13
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 12
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 11
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 10
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 9
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 8
  &[(E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000)], // 7
  &[(Z, 0b00100), (Z, 0b10011), (E, 0b00000), (E, 0b00000), (E, 0b00000), (E, 0b00000), (I, 0b00100), (I, 0b00101), (I, 0b00101), (I, 0b00001)], // 6
  &[(T, 0b00010), (Z, 0b11100), (Z, 0b00001), (E, 0b00000), (E, 0b00000), (G, 0b00010), (L, 0b00010), (O, 0b00110), (O, 0b00011), (J, 0b00010)], // 5
  &[(T, 0b11110), (T, 0b00001), (S, 0b10110), (S, 0b00001), (G, 0b00100), (G, 0b11011), (L, 0b01010), (O, 0b01100), (O, 0b01001), (J, 0b01010)], // 4
  &[(T, 0b01000), (S, 0b00100), (S, 0b11001), (E, 0b00000), (E, 0b00000), (G, 0b01000), (L, 0b11100), (L, 0b00001), (J, 0b00100), (J, 0b11001)], // 3
  &[(G, 0b00100), (G, 0b00101), (G, 0b00101), (G, 0b00001), (E, 0b00000), (G, 0b00100), (G, 0b00101), (G, 0b00101), (G, 0b00101), (G, 0b00001)], // 2
  &[(D, 0b00100), (D, 0b00101), (D, 0b00101), (D, 0b00001), (E, 0b00000), (D, 0b00100), (D, 0b00101), (D, 0b00101), (D, 0b00101), (D, 0b00001)], // 1
];


pub fn nine_slice_resize() {

}

pub fn clone_slice(tex: &DynamicImage, x: u32, y: u32, w: u32, h: u32) -> DynamicImage {
  let mut target = DynamicImage::new_rgba8(w, h);
  image::imageops::overlay(&mut target, tex, -(x as i64), -(y as i64));
  target
}

pub fn render(tpse: &TPSE) -> Result<Option<DynamicImage>, LoadError> {
  /// A list of drawing tasks to perform. Units are in blocks.
  let mut tasks: Vec<(DynamicImage, f64, f64, f64, f64)> = vec![];
  let resolution = 48;

  if let Some(board) = &tpse.board {
    let texture = decode_image(&board.binary)?;
    for el in BoardElement::get_draw_order() {
      let (x, y, w, h, pad_top, pad_right, pad_bottom, pad_left) = el.get_slice();
      println!("{} {} {} {}   {} {}", x, y, w, h, texture.width(), texture.height());
      let texture = clone_slice(&texture, x, y, w, h);
      let (x, y, w, h) = el.get_target();
      // todo: figure out nine slicing
      tasks.push((texture, x, y, w, h))
    }
  }

  if tpse.skin.is_some() || tpse.ghost.is_some() {
    let mut splicer = SkinSplicer::default();
    if let Some(skin) = &tpse.skin {
      splicer.load(SkinType::Tetrio61Connected, &skin.binary)?;
    }
    if let Some(ghost) = &tpse.ghost {
      splicer.load(SkinType::Tetrio61ConnectedGhost, &ghost.binary)?;
    }
    for (row, row_data) in PCO_MAP.iter().enumerate() {
      for (col, (piece, connection)) in row_data.iter().enumerate() {
        let tex = piece.and_then(|piece| {
          splicer.get(piece, *connection, None).or_else(|| splicer.get(piece, 0b00000, None))
        });
        if let Some(tex) = tex {
          // to line up blocks with board, determined using same image as `BoardElement::get_slice`
          let brx = 209.0 / 34.0;
          // for skyline
          let bry = -4.0;
          tasks.push((tex.into(), col as f64 + brx, row as f64 + bry, 1.0, 1.0));
        }
      }
    }
  }

  if tasks.is_empty() {
    return Ok(None)
  }

  // convert task block coordinates into pixel coordinates
  let tasks = tasks.into_iter().map(|(img, x, y, w, h)| {
    let res = resolution as f64;
    (img, (x * res) as i64, (y * res) as i64, (w * res) as i64, (h * res) as i64)
  }).collect::<Vec<_>>();

  let min_x = tasks.iter().map(|(img, x, y, w, h)| x).min().unwrap();
  let min_y = tasks.iter().map(|(img, x, y, w, h)| y).min().unwrap();
  let max_x = tasks.iter().map(|(img, x, y, w, h)| x+w).max().unwrap();
  let max_y = tasks.iter().map(|(img, x, y, w, h)| y+h).max().unwrap();
  let mut canvas = DynamicImage::new_rgb8((max_x - min_x) as u32, (max_y - min_y) as u32);
  for (img, x, y, w, h) in tasks {
    let resized = image::imageops::resize(&img, w as u32, h as u32, FilterType::CatmullRom);
    image::imageops::overlay(&mut canvas, &resized, x as i64, y as i64);
  }

  Ok(Some(canvas))
}