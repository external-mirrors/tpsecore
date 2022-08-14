mod board_element;

use std::ops::Deref;
use image::{DynamicImage, GenericImage, GenericImageView};
use image::imageops::FilterType;
use rusttype::{Font, Scale};
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
const P: Option<Piece> = Some(Piece::Ghost);
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
  &[(T, 0b00010), (Z, 0b11100), (Z, 0b00001), (E, 0b00000), (E, 0b00000), (P, 0b00010), (L, 0b00010), (O, 0b00110), (O, 0b00011), (J, 0b00010)], // 5
  &[(T, 0b11110), (T, 0b00001), (S, 0b10110), (S, 0b00001), (P, 0b00100), (P, 0b11011), (L, 0b01010), (O, 0b01100), (O, 0b01001), (J, 0b01010)], // 4
  &[(T, 0b01000), (S, 0b00100), (S, 0b11001), (E, 0b00000), (E, 0b00000), (P, 0b01000), (L, 0b11100), (L, 0b00001), (J, 0b00100), (J, 0b11001)], // 3
  &[(G, 0b00100), (G, 0b00101), (G, 0b00101), (G, 0b00001), (E, 0b00000), (G, 0b00100), (G, 0b00101), (G, 0b00101), (G, 0b00101), (G, 0b00001)], // 2
  &[(D, 0b00100), (D, 0b00101), (D, 0b00101), (D, 0b00101), (D, 0b00101), (D, 0b00101), (D, 0b00101), (D, 0b00101), (D, 0b00101), (D, 0b00001)], // 1
];


pub fn nine_slice(w: u32, h: u32, pad_top: u32, pad_right: u32, pad_bottom: u32, pad_left: u32) -> [(u32, u32, u32, u32); 9] {
  let center_width = w.saturating_sub(pad_left + pad_right);
  let center_height = h.saturating_sub(pad_top + pad_bottom);
  [
    (0, 0, pad_left, pad_top), // top left
    (pad_left, 0, center_width, pad_top), // top center
    (pad_left + center_width, 0, pad_right, pad_top), // top right
    (0, pad_top, pad_left, center_height), // middle left
    (pad_left, pad_top, center_width, center_height), // center
    (pad_left + center_width, pad_top, pad_right, center_height), // middle right
    (0, pad_top + center_height, pad_left, pad_bottom), // bottom left
    (pad_left, pad_top + center_height, center_width, pad_bottom), // bottom center
    (pad_left + center_width, pad_top + center_height, pad_right, pad_bottom), // bottom right
  ]
}

pub fn nine_slice_resize
  (tex: &DynamicImage, w: u32, h: u32, pad_top: u32, pad_right: u32, pad_bottom: u32, pad_left: u32)
  -> DynamicImage
{
  let sources = nine_slice(tex.width(), tex.height(), pad_top, pad_right, pad_bottom, pad_left);
  let dests = nine_slice(w, h, pad_top, pad_right, pad_bottom, pad_left);
  let mut dest = DynamicImage::new_rgba8(w, h);
  for (i, ((sx, sy, sw, sh), (dx, dy, dw, dh))) in sources.iter().copied().zip(dests.iter().copied()).enumerate() {
    println!("slice {} of {} {}: draw {} {} {} {} to {} {} {} {}", i+1, tex.width(), tex.height(), sx, sy, sw, sh, dx, dy, dw, dh);
    if sw == 0 || sh == 0 { continue; }
    let slice = tex.view(sx, sy, sw, sh);
    let resized = image::imageops::resize(slice.deref(), dw, dh, FilterType::CatmullRom);
    image::imageops::overlay(&mut dest, &resized, dx as i64, dy as i64);
  }
  dest
}

/// Clones a slice of the given DynamicImage, filling overflow regions with transparency
pub fn clone_slice(tex: &DynamicImage, x: u32, y: u32, w: u32, h: u32) -> DynamicImage {
  let mut target = DynamicImage::new_rgba8(w, h);
  image::imageops::overlay(&mut target, tex, -(x as i64), -(y as i64));
  target
}

pub struct RenderOptions<'a> {
  pub board_pieces: &'a [BoardElement],
  pub debug_grid: bool
}
impl Default for RenderOptions<'static> {
  fn default() -> Self {
    RenderOptions {
      board_pieces: BoardElement::get_draw_order(),
      debug_grid: false
    }
  }
}

pub fn render(tpse: &TPSE, opts: RenderOptions) -> Result<Option<DynamicImage>, LoadError> {
  /// A list of drawing tasks to perform. Units are in blocks.
  let mut tasks: Vec<(DynamicImage, f64, f64, f64, f64)> = vec![];
  /// The size of a block in pixels.
  let resolution = 48;

  if let Some(board) = &tpse.board {
    let texture = decode_image(&board.binary)?;
    for el in BoardElement::get_draw_order() {
      if !opts.board_pieces.contains(el) { continue }
      let ((x, y, w, h), (pad_top, pad_right, pad_bottom, pad_left)) = el.get_slice();
      let texture = clone_slice(&texture, x, y, w, h);
      let (x, y, w, h) = el.get_target();
      let texture = nine_slice_resize(
        &texture,
        (w * resolution as f64) as u32,
        (h * resolution as f64) as u32,
        pad_top,
        pad_right,
        pad_bottom,
        pad_left
      );
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
          tasks.push((tex.into(), col as f64, (row - 4) as f64, 1.0, 1.0));
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

  let min_x = tasks.iter().map(|(img, x, y, w, h)| *x).min().unwrap();
  let min_y = tasks.iter().map(|(img, x, y, w, h)| *y).min().unwrap();
  let max_x = tasks.iter().map(|(img, x, y, w, h)| x+w).max().unwrap();
  let max_y = tasks.iter().map(|(img, x, y, w, h)| y+h).max().unwrap();
  let mut canvas = DynamicImage::new_rgba8((max_x - min_x) as u32, (max_y - min_y) as u32);
  for (img, x, y, w, h) in tasks {
    let resized = image::imageops::resize(&img, w as u32, h as u32, FilterType::CatmullRom);
    image::imageops::overlay(&mut canvas, &resized, x - min_x, y - min_y);
  }

  if opts.debug_grid {
    let white = [255, 255, 255, 255].into();
    let font = Font::try_from_bytes(include_bytes!("../../assets/pfw.ttf")).unwrap();
    for x in (min_x..max_x).filter(|el| el % 48 == 0 /* "performance"? */) {
      let height = canvas.height();
      imageproc::drawing::draw_line_segment_mut(
        &mut canvas,
        ((x - min_x) as f32, 0.0),
        ((x - min_x) as f32, height as f32),
        white
      );
      imageproc::drawing::draw_text_mut(
        &mut canvas,
        white,
        (x - min_x) as i32 + 2, 2,
        Scale::uniform(16.0),
        &font,
        &format!("X{}", x)
      );
    }
    for y in (min_y..max_y).filter(|el| el % 48 == 0) {
      let width = canvas.width();
      imageproc::drawing::draw_line_segment_mut(
        &mut canvas,
        (0.0, (y - min_y) as f32),
        (width as f32, (y - min_y) as f32),
        white
      );
      imageproc::drawing::draw_text_mut(
        &mut canvas,
        white,
        2, (y - min_y) as i32 + if y == min_y { 16 } else { 2 },
        Scale::uniform(16.0),
        &font,
        &format!("Y{}", y)
      );
    }
  }

  Ok(Some(canvas))
}