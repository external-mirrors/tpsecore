use std::borrow::Cow;
use std::io::Cursor;
use hound::{SampleFormat, WavSpec};
use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::{ImportErrorType, SkinType};
use crate::import::decode_helper::TetrioAtlasDecoder;
use crate::import::skin_splicer::SkinSplicer;
use crate::render::{BoardElement, clone_slice, nine_slice_resize, RenderOptions};
use crate::render::board_element::BoardTextureKind;
use crate::tpse::{AnimMeta, File, TPSE};

pub struct RenderContext<T: TPSEAccelerator> {
  skin: Option<T::Texture>,
  ghost: Option<T::Texture>,
  skin_anim: Option<(T::Texture, AnimMeta)>,
  ghost_anim: Option<(T::Texture, AnimMeta)>,
  board: Option<T::Texture>,
  queue: Option<T::Texture>,
  grid: Option<T::Texture>
}
impl<T: TPSEAccelerator> Clone for RenderContext<T> {
  fn clone(&self) -> Self {
    // texture handles are lightweight to clone, so this should be quite fast
    Self {
      skin: self.skin.clone(),
      ghost: self.ghost.clone(),
      skin_anim: self.skin_anim.clone(),
      ghost_anim: self.ghost_anim.clone(),
      board: self.board.clone(),
      queue: self.queue.clone(),
      grid: self.grid.clone()
    }
  }
}

/// A rendered frame
#[derive(Debug, serde::Serialize)]
pub struct RenderedFrame<T> {
  /// The rendered image
  pub image: T,
  /// The logical render coordinate of the left border of the image
  pub min_x: i64,
  /// The logical render coordinate of the top border of the image
  pub min_y: i64,
  /// The logical render coordinate of the right border of the image
  pub max_x: i64,
  /// The logical render coordinate of the bottom border of the image
  pub max_y: i64
}

#[derive(Debug, thiserror::Error)]
pub enum SoundEffectRenderError<'a> {
  #[error("no such sound effect: {0}")]
  NoSuchSoundEffect(Cow<'a, str>),
  #[error(transparent)]
  ImportError(#[from] ImportErrorType)
}

/// Renders a sequence of sound effects to a continuous buffer
#[allow(unused)] // remove when done reimplementing
pub fn render_sound_effects<'a>
  (tpse: &'a TPSE, opts: &'a [SoundEffectInfo])
  -> Result<File, SoundEffectRenderError<'a>>
{
  if opts.is_empty() {
    return Ok(File {
      binary: include_bytes!("../../assets/empty_2c.wav").to_vec().into(),
      mime: "audio/wav".to_string()
    });
  }
  use SoundEffectRenderError::*;

  let atlas = TetrioAtlasDecoder::decode_from_tpse(tpse)?;
  let sample_rate = 44100;
  let channels = 2;
  // let samples_per_frame = (1.0/ctx.frame_rate * sample_rate as f64) as usize * channels;
  let samples_per_frame: usize = todo!();

  let length: usize = opts.iter()
    .filter_map(|el| Some(el.time * samples_per_frame + atlas.lookup(&el.name)?.len()))
    .max_by(|a, b| a.partial_cmp(b).unwrap())
    .unwrap_or(0);

  let mut decoded = vec![0f32; length];
  for SoundEffectInfo { name, time } in opts {
    let sfx = atlas.lookup(name.as_ref()).ok_or_else(|| NoSuchSoundEffect(name.clone()))?;
    let samples = time * samples_per_frame;
    let slice = &mut decoded[samples..samples+sfx.len()];
    for (a, b) in slice.iter_mut().zip(sfx.iter()) {
      *a += b;
    }
  }

  let mut encoded = Vec::with_capacity(length);
  let mut cursor = Cursor::new(&mut encoded);
  let mut encoder = hound::WavWriter::new(&mut cursor, WavSpec {
    channels: 2,
    sample_rate,
    bits_per_sample: 32,
    sample_format: SampleFormat::Float
  }).unwrap();
  for sample in decoded {
    encoder.write_sample(sample).unwrap();
  }
  encoder.finalize().unwrap();

  Ok(File {
    binary: encoded.into(),
    mime: "audio/wav".to_string()
  })
}

#[derive(Debug, serde::Deserialize)]
pub struct SoundEffectInfo<'a> {
  /// The name of the sound effect
  pub name: Cow<'a, str>,
  /// The time the sound effect occurs, in frames
  pub time: usize
}
impl<'a> SoundEffectInfo<'a> {
  pub fn new(name: &'a str, time: usize) -> SoundEffectInfo<'a> {
    Self { name: Cow::from(name), time }
  }
}

#[derive(Debug)]
pub struct FrameInfo<'a> {
  /// The time the frame is being rendered for.
  /// Used for picking animated skin frame.
  pub real_time: f64,
  pub render_options: &'a RenderOptions<'a>,
}
impl<T: TPSEAccelerator> RenderContext<T> {
  pub fn try_from_tpse(tpse: &TPSE) -> Result<Self, T::DecodeError> {
    let load_transpose = |file: &Option<File>| {
      file.as_ref().map(|file| T::decode_texture(file.binary.clone())).transpose()
    };
    let skin = load_transpose(&tpse.skin)?;
    let ghost = load_transpose(&tpse.ghost)?;
    let skin_anim = load_transpose(&tpse.skin_anim)?
      .and_then(|img| Some((img, tpse.skin_anim_meta?)));
    let ghost_anim = load_transpose(&tpse.ghost_anim)?
      .and_then(|img| Some((img, tpse.ghost_anim_meta?)));
    let board = load_transpose(&tpse.board)?;
    let queue = load_transpose(&tpse.queue)?;
    let grid = load_transpose(&tpse.grid)?;
    Ok(Self { skin, ghost, skin_anim, ghost_anim, board, queue, grid })
  }

  pub fn max_skin_frames(&self) -> u32 {
    let skin = self.skin_anim.as_ref().map(|(_, meta)| meta.frames);
    let ghost = self.ghost_anim.as_ref().map(|(_, meta)| meta.frames);
    [skin, ghost].iter().filter_map(|el| *el).max().unwrap_or(1)
  }

  pub fn min_skin_delay(&self) -> u32 {
    let skin = self.skin_anim.as_ref().map(|(_, meta)| meta.delay);
    let ghost = self.ghost_anim.as_ref().map(|(_, meta)| meta.delay);
    [skin, ghost].iter().filter_map(|el| *el).min().unwrap_or(1)
  }

  pub async fn render_frame(&self, frame: &FrameInfo<'_>)
    -> Result<RenderedFrame<T::Texture>, <T::Texture as TextureHandle>::Error>
  {
    // A list of drawing tasks to perform. Units are in pixels.
    let mut tasks: Vec<(T::Texture, i64, i64, i64, i64)> = vec![];

    for el in BoardElement::get_draw_order() {
      if !frame.render_options.board_elements.contains(el) { continue }

      let (texture_source, (x, y, w, h), (pt, pr, pb, pl), scale) = el.get_slice();
      let Some(texture) = (match texture_source {
        BoardTextureKind::Board => &self.board,
        BoardTextureKind::Queue => &self.queue,
        BoardTextureKind::Grid => &self.grid
      }) else { continue };
      let texture = clone_slice::<T>(&texture, x, y, w, h);
      let (x, y, w, h) = el.get_target(&frame.render_options);
      let texture = nine_slice_resize::<T>(&texture, w as u32 * scale, h as u32 * scale, pt, pr, pb, pl).await?;
      let texture = texture.tinted(el.tint());
      tasks.push((texture, x, y, w, h))
    }

    let load_frame = |img: &T::Texture, meta: &AnimMeta| -> T::Texture {
      let frame = (frame.real_time * 60.0 / meta.delay as f64) as u32 % meta.frames;
      let x = (frame % 16) * 1024;
      let y = (frame / 16) * 1024;
      img.slice(x, y, 1024, 1024)
    };

    let mut splicer: SkinSplicer<T> = Default::default();

    if let Some((skin, opts)) = &self.skin_anim {
      splicer.load_decoded(SkinType::Tetrio61Connected, load_frame(skin, opts))
    } else if let Some(skin) = &self.skin {
      splicer.load_decoded(SkinType::Tetrio61Connected, skin.create_copy())
    }

    if let Some((ghost, opts)) = &self.ghost_anim {
      splicer.load_decoded(SkinType::Tetrio61ConnectedGhost, load_frame(ghost, opts))
    } else if let Some(ghost) = &self.ghost {
      splicer.load_decoded(SkinType::Tetrio61ConnectedGhost, ghost.create_copy());
    }

    if splicer.len() > 0 {
      let skyline_size = frame.render_options.board_size().1 as i64 - frame.render_options.skyline as i64;
      for (row, col, piece) in frame.render_options.board.iter() {
        let tex = match piece {
          Some((piece, connection)) => {
            let piece_a = splicer.get(piece, connection, None).await?;
            let piece_fallback = splicer.get(piece, 0b00000, None).await?;
            piece_a.or(piece_fallback)
          },
          None => None
        };
        if let Some(tex) = tex {
          tasks.push((
            tex,
            col as i64 * frame.render_options.block_size,
            (row as i64 - skyline_size) * frame.render_options.block_size,
            frame.render_options.block_size, frame.render_options.block_size
          ));
        }
      }
    }

    if tasks.is_empty() {
      log::trace!("No render tasks!");
      Ok(RenderedFrame {
        image: T::new_texture(0, 0),
        min_x: 0,
        min_y: 0,
        max_x: 0,
        max_y: 0
      })
    } else {
      let min_x = tasks.iter().map(|(_, x, _, _, _)| *x).min().unwrap();
      let min_y = tasks.iter().map(|(_, _, y, _, _)| *y).min().unwrap();
      let max_x = tasks.iter().map(|(_, x, _, w, _)| x + w).max().unwrap();
      let max_y = tasks.iter().map(|(_, _, y, _, h)| y + h).max().unwrap();

      let canvas_w: u32 = (max_x - min_x).try_into().expect("min_x > max_x or max_x - min_x overflow");
      let canvas_h: u32 = (max_y - min_y).try_into().expect("min_y > max_y or max_y - min_y overflow");
      if canvas_w > 10_000 || canvas_h > 10_000 || canvas_w*canvas_h > 10_000_000 {
        log::warn!("render_frame: creating huge texture of {canvas_w}*{canvas_h} (extents {min_x} - {max_x} by {min_y} - {max_y})");
        #[cfg(test)]
        panic!("excessive texture size requested");
      }
      let canvas = T::new_texture(canvas_w, canvas_h);

      for (img, x, y, w, h) in tasks {
        canvas.overlay(&img.resized(w as u32, h as u32), x - min_x, y - min_y);
      }

      if frame.render_options.debug_grid {
        let white = [255, 255, 255, 255];
        for x in (min_x..max_x).filter(|el| el % 48 == 0) {
          let height = canvas.height().await?;
          canvas.draw_line(
            ((x - min_x) as f32, 0.0),
            ((x - min_x) as f32, height as f32),
            white
          );

          canvas.draw_text(
            white,
            (x - min_x) as i32 + 2, 2,
            16.0,
            &format!("X{}", x)
          );
        }
        for y in (min_y..max_y).filter(|el| el % 48 == 0) {
          let width = canvas.width().await?;
          canvas.draw_line(
            (0.0, (y - min_y) as f32),
            (width as f32, (y - min_y) as f32),
            white
          );
          canvas.draw_text(
            white,
            2, (y - min_y) as i32 + if y == min_y { 16 } else { 2 },
            16.0,
            &format!("Y{}", y)
          );
        }
      }

      Ok(RenderedFrame { image: canvas, min_x, min_y, max_x, max_y })
    }
  }
}