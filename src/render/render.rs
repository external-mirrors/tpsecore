use image::DynamicImage;
use image::imageops::FilterType;
use rusttype::{Font, Scale};
use crate::import::{LoadError, SkinType};
use crate::import::skin_splicer::{decode_image, SkinSplicer};
use crate::render::{BoardElement, clone_slice, nine_slice_resize, RenderOptions};
use crate::render::board_element::BoardTextureKind;
use crate::tpse::TPSE;

pub fn render(tpse: &TPSE, opts: RenderOptions) -> Result<Option<DynamicImage>, LoadError> {
  /// A list of drawing tasks to perform. Units are in pixels.
  let mut tasks: Vec<(DynamicImage, i64, i64, i64, i64)> = vec![];

  let board = tpse.board.as_ref().map(|file| decode_image(&file.binary)).transpose()?;
  let queue = tpse.queue.as_ref().map(|file| decode_image(&file.binary)).transpose()?;
  let grid = tpse.grid.as_ref().map(|file| decode_image(&file.binary)).transpose()?;

  for el in BoardElement::get_draw_order() {
    if !opts.board_pieces.contains(el) { continue }
    let (texture_source, (x, y, w, h), (p_top, p_right, p_bottom, p_left)) = el.get_slice();
    let texture = match texture_source {
      BoardTextureKind::Board => &board,
      BoardTextureKind::Queue => &queue,
      BoardTextureKind::Grid => &grid
    };
    let texture = if let Some(texture) = texture { texture } else { continue };
    let texture = clone_slice(&texture, x, y, w, h);
    let (x, y, w, h) = el.get_target(opts);
    let texture = nine_slice_resize(&texture, w as u32, h as u32, p_top, p_right, p_bottom, p_left);
    let mut texture = texture.into_rgba8();
    for pixel in texture.pixels_mut() {
      pixel.0[0] = (((el.tint() >> 24) & 0xFF) as f64 / 0xFF as f64 * pixel.0[0] as f64) as u8;
      pixel.0[1] = (((el.tint() >> 16) & 0xFF) as f64 / 0xFF as f64 * pixel.0[1] as f64) as u8;
      pixel.0[2] = (((el.tint() >> 08) & 0xFF) as f64 / 0xFF as f64 * pixel.0[2] as f64) as u8;
      pixel.0[3] = (((el.tint() >> 00) & 0xFF) as f64 / 0xFF as f64 * pixel.0[3] as f64) as u8;
    }
    tasks.push((texture.into(), x, y, w, h))
  }

  if tpse.skin.is_some() || tpse.ghost.is_some() {
    let mut splicer = SkinSplicer::default();
    if let Some(skin) = &tpse.skin {
      splicer.load(SkinType::Tetrio61Connected, &skin.binary)?;
    }
    if let Some(ghost) = &tpse.ghost {
      splicer.load(SkinType::Tetrio61ConnectedGhost, &ghost.binary)?;
    }
    let skyline_size = opts.board_size().1 as i64 - opts.skyline as i64;
    for (row, row_data) in opts.board.iter().enumerate() {
      for (col, (piece, connection)) in row_data.iter().enumerate() {
        let tex = piece.and_then(|piece| {
          splicer.get(piece, *connection, None).or_else(|| splicer.get(piece, 0b00000, None))
        });
        if let Some(tex) = tex {
          tasks.push((
            tex.into(),
            col as i64 * opts.block_size,
            (row as i64 - skyline_size) * opts.block_size,
            opts.block_size, opts.block_size
          ));
        }
      }
    }
  }

  if tasks.is_empty() {
    return Ok(None)
  }

  let min_x = tasks.iter().map(|(img, x, y, w, h)| *x).min().unwrap();
  let min_y = tasks.iter().map(|(img, x, y, w, h)| *y).min().unwrap();
  let max_x = tasks.iter().map(|(img, x, y, w, h)| x+w).max().unwrap();
  let max_y = tasks.iter().map(|(img, x, y, w, h)| y+h).max().unwrap();
  let mut canvas = DynamicImage::new_rgba8((max_x - min_x) as u32, (max_y - min_y) as u32);
  for (img, x, y, w, h) in tasks {
    let mut resized = image::imageops::resize(&img, w as u32, h as u32, FilterType::CatmullRom);
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