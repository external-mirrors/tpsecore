use image::{DynamicImage, GenericImageView};
use image::imageops::FilterType;
use rusttype::{Font, Scale};
use crate::import::{LoadError, SkinType};
use crate::import::skin_splicer::{decode_image, SkinSplicer};
use crate::render::{BoardElement, clone_slice, nine_slice_resize, RenderOptions};
use crate::render::board_element::BoardTextureKind;
use crate::tpse::{AnimMeta, File, TPSE};

struct FrameRenderOptions<'a> {
  tpse: &'a TPSE,
  skin: Option<DynamicImage>,
  ghost: Option<DynamicImage>,
  skin_anim: Option<(DynamicImage, AnimMeta)>,
  ghost_anim: Option<(DynamicImage, AnimMeta)>,
  board: Option<DynamicImage>,
  queue: Option<DynamicImage>,
  grid: Option<DynamicImage>,
  render_options: RenderOptions<'a>,
  frame: usize
}
impl FrameRenderOptions<'_> {
  pub fn frames(&self) -> u32 {
    let skin = self.skin_anim.as_ref().map(|(_, meta)| meta.frames);
    let ghost = self.ghost_anim.as_ref().map(|(_, meta)| meta.frames);
    [skin, ghost].iter().filter_map(|el| *el).max().unwrap_or(1)
  }

  pub fn delay(&self) -> u32 {
    let skin = self.skin_anim.as_ref().map(|(_, meta)| meta.delay);
    let ghost = self.ghost_anim.as_ref().map(|(_, meta)| meta.delay);
    [skin, ghost].iter().filter_map(|el| *el).min().unwrap_or(1)
  }
}

pub fn render<'a>(tpse: &'a TPSE, opts: RenderOptions<'a>) -> Result<impl Iterator<Item = Option<DynamicImage>> + 'a, LoadError> {
  let load_transpose = |file: &Option<File>| file.as_ref().map(|file| decode_image(&file.binary)).transpose();
  let skin = load_transpose(&tpse.skin)?;
  let ghost = load_transpose(&tpse.ghost)?;
  let skin_anim = load_transpose(&tpse.skin_anim)?.and_then(|img| Some((img, tpse.skin_anim_meta?)));
  let ghost_anim = load_transpose(&tpse.ghost_anim)?.and_then(|img| Some((img, tpse.ghost_anim_meta?)));
  let board = load_transpose(&tpse.board)?;
  let queue = load_transpose(&tpse.queue)?;
  let grid = load_transpose(&tpse.grid)?;

  let mut fro = FrameRenderOptions {
    tpse, skin, ghost, skin_anim, ghost_anim, board, queue, grid,
    render_options: opts,
    frame: 0
  };

  println!("Frame count: {} (reg: {}, anim: {} ({} {:?}))", fro.frames(), fro.skin.is_some(), fro.skin_anim.is_some(), tpse.skin_anim.is_some(), tpse.skin_anim_meta);

  Ok((0..fro.frames()).map(move |frame| {
    fro.frame = frame as usize;
    render_frame(&fro)
  }))
}

fn render_frame(opts: &FrameRenderOptions) -> Option<DynamicImage> {
  /// A list of drawing tasks to perform. Units are in pixels.
  let mut tasks: Vec<(DynamicImage, i64, i64, i64, i64)> = vec![];

  for el in BoardElement::get_draw_order() {
    if !opts.render_options.board_pieces.contains(el) { continue }
    let (texture_source, (x, y, w, h), (pt, pr, pb, pl), scale) = el.get_slice();
    let texture = match texture_source {
      BoardTextureKind::Board => &opts.board,
      BoardTextureKind::Queue => &opts.queue,
      BoardTextureKind::Grid => &opts.grid
    };
    let texture = if let Some(texture) = texture { texture } else { continue };
    let texture = clone_slice(&texture, x, y, w, h);
    let (x, y, mut w, mut h) = el.get_target(opts.render_options);
    let texture = nine_slice_resize(&texture, w as u32 * scale, h as u32 * scale, pt, pr, pb, pl);
    let mut texture = texture.into_rgba8();
    for pixel in texture.pixels_mut() {
      pixel.0[0] = (((el.tint() >> 24) & 0xFF) as f64 / 0xFF as f64 * pixel.0[0] as f64) as u8;
      pixel.0[1] = (((el.tint() >> 16) & 0xFF) as f64 / 0xFF as f64 * pixel.0[1] as f64) as u8;
      pixel.0[2] = (((el.tint() >> 08) & 0xFF) as f64 / 0xFF as f64 * pixel.0[2] as f64) as u8;
      pixel.0[3] = (((el.tint() >> 00) & 0xFF) as f64 / 0xFF as f64 * pixel.0[3] as f64) as u8;
    }
    tasks.push((texture.into(), x, y, w, h))
  }

  let load_frame = |img: &DynamicImage, meta: &AnimMeta| -> DynamicImage {
    let x = (opts.frame % 16) * 1024;
    let y = (opts.frame / 16) * 1024;
    let tex = img.view(x as u32, y as u32, 1024, 1024);
    DynamicImage::from(tex.to_image())
  };

  let mut splicer = SkinSplicer::default();

  if let Some((skin, opts)) = &opts.skin_anim {
    splicer.load_decoded(SkinType::Tetrio61Connected, load_frame(skin, opts))
  } else if let Some(skin) = &opts.skin {
    splicer.load_decoded(SkinType::Tetrio61Connected, skin.clone())
  }

  if let Some((ghost, opts)) = &opts.ghost_anim {
    splicer.load_decoded(SkinType::Tetrio61ConnectedGhost, load_frame(ghost, opts))
  } else if let Some(ghost) = &opts.ghost {
    splicer.load_decoded(SkinType::Tetrio61ConnectedGhost, ghost.clone());
  }

  if splicer.len() > 0 {
    let skyline_size = opts.render_options.board_size().1 as i64 - opts.render_options.skyline as i64;
    for (row, row_data) in opts.render_options.board.iter().enumerate() {
      for (col, (piece, connection)) in row_data.iter().enumerate() {
        let tex = piece.and_then(|piece| {
          splicer.get(piece, *connection, None).or_else(|| splicer.get(piece, 0b00000, None))
        });
        if let Some(tex) = tex {
          tasks.push((
            tex.into(),
            col as i64 * opts.render_options.block_size,
            (row as i64 - skyline_size) * opts.render_options.block_size,
            opts.render_options.block_size, opts.render_options.block_size
          ));
        }
      }
    }
  }

  if tasks.is_empty() {
    return None
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

  if opts.render_options.debug_grid {
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

  Some(canvas)
}