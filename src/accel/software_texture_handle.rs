use std::error::Error;
use std::io::Cursor;
use std::panic::catch_unwind;
use std::sync::{Arc, LazyLock, Mutex};

use ab_glyph::FontRef;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, SubImage};
use image::imageops::{FilterType, overlay, resize};
use slotmap::{DefaultKey, SlotMap};
use tiny_skia::Pixmap;
use usvg::Transform;

use crate::accel::traits::TextureHandle;

#[derive(Debug, Clone)]
pub struct SoftwareTextureHandle(Arc<TexStoreKey>, [u32; 4]);

#[derive(Debug, thiserror::Error)]
pub enum SoftwareRenderingError {
  #[error("failed to load asset: {0}")]
  ErasedError(Box<dyn Error + Send + Sync + 'static>),
  #[error("failed to load image: image decoder implementation panicked")]
  ImageLoadPanic,
}

pub static TEX_STORE: LazyLock<Mutex<SlotMap<DefaultKey, DynamicImage>>> = LazyLock::new(Default::default);

#[derive(Debug)]
struct TexStoreKey(DefaultKey);
impl Drop for TexStoreKey {
  fn drop(&mut self) {
    let mut store = TEX_STORE.lock().unwrap();
    store.remove(self.0).unwrap();
  }
}

impl SoftwareTextureHandle {
  pub fn new(image: DynamicImage) -> Self {
    let width = image.width();
    let height = image.height();
    let key = TEX_STORE.lock().unwrap().insert(image);
    Self(Arc::new(TexStoreKey(key)), [0, 0, width, height])
  }
  pub fn get<T>(&self, handler: impl FnOnce(&mut SubImage<&mut DynamicImage>) -> T) -> T {
    let Self(image, [x, y, w, h]) = self;
    let mut map = TEX_STORE.lock().unwrap();
    let inner = map.get_mut(image.0).unwrap();
    let mut subimage = SubImage::new(inner, *x, *y, *w, *h);
    handler(&mut subimage)
  }
  pub fn get2<T>(&self, other: &Self, handler: impl FnOnce(&mut SubImage<&mut DynamicImage>, &mut SubImage<&mut DynamicImage>) -> T) -> T {
    let mut map = TEX_STORE.lock().unwrap();
    let Some([inner1, inner2]) = map.get_disjoint_mut([self.0.0, other.0.0]) else {
      panic!("attempted to use an image method with itself as an extra parameter");
    };
    let mut subimage1 = SubImage::new(inner1, self.1[0], self.1[1], self.1[2], self.1[3]);
    let mut subimage2 = SubImage::new(inner2, other.1[0], other.1[1], other.1[2], other.1[3]);
    handler(&mut subimage1, &mut subimage2)
  }
}
impl TextureHandle for SoftwareTextureHandle {
  type Error = SoftwareRenderingError;

  fn new_texture(width: u32, height: u32) -> Self {
    Self::new(DynamicImage::new_rgba8(width, height))
  }

  fn decode_texture(buffer: Arc<[u8]>) -> Result<Self, Self::Error> {
    fn decode_svg(bytes: &[u8]) -> Option<Vec<u8>> {
      let opt = usvg::Options::default();
      let rtree = usvg::Tree::from_data(bytes, &opt).ok()?;
      let pixmap_size = rtree.size().to_int_size();
      let mut pixmap = Pixmap::new(pixmap_size.width(), pixmap_size.height())?;
      resvg::render(&rtree, Transform::default(), &mut pixmap.as_mut());
      pixmap.encode_png().ok()
    }
    
    let transcoded = decode_svg(&buffer);
    let buffer = match transcoded {
      Some(transcoded) => transcoded.into(),
      None => buffer
    };

    let image = catch_unwind
      (|| {
        let reader = ImageReader::new(Cursor::new(buffer))
          .with_guessed_format()
          .expect("Cursor<&[u8]> shouldn't generate IO errors");
        reader.decode()
      })
      .map_err(|_err| SoftwareRenderingError::ImageLoadPanic)?
      .map_err(|err| SoftwareRenderingError::ErasedError(Box::new(err)))?;
    Ok(Self::new(image))
  }
  
  fn create_copy(&self) -> Self {
    let clone = self.get(|image| { image.to_image() });
    Self::new(clone.into())
  }
  fn draw_line(&self, start: (f32, f32), end: (f32, f32), color: [u8; 4]) {
    self.get(|image| {
      imageproc::drawing::draw_line_segment_mut(image.inner_mut(), start, end, color.into());
    });
  }
  fn draw_text(&self, color: [u8; 4], x: i32, y: i32, scale: f32, text: &str) {
    static FONT: LazyLock<FontRef> = LazyLock::new(|| FontRef::try_from_slice(include_bytes!("../../assets/pfw.ttf")).unwrap());
    self.get(|image| {
      imageproc::drawing::draw_text_mut(image.inner_mut(), color.into(), x, y, scale, &*FONT, text);
    });
  }
  async fn encode_png(&self) -> Result<Arc<[u8]>, Self::Error> {    
    self.get(|image| {
      let mut buffer = vec![];
      match image.inner().write_to(Cursor::new(&mut buffer), ImageFormat::Png) {
        Err(err) => {
          log::error!("failed to encode frame: {err}");
          Err(SoftwareRenderingError::ErasedError(Box::new(err)))
        }
        Ok(()) => Ok(buffer.into()),
      }
    })
  }
  async fn width(&self) -> Result<u32, Self::Error> {
    Ok(self.get(|x| x.width()))
  }
  async fn height(&self) -> Result<u32, Self::Error> {
    Ok(self.get(|x| x.height()))
  }
  fn overlay(&self, with_image: &Self, x: i64, y: i64) {
    self.get2(with_image, |a, b| {
      overlay(a.inner_mut(), b.inner(), x, y);
    });
  }
  fn resized(&self, width: u32, height: u32) -> Self {
    let resized = self.get(|image| {
      resize(image.inner_mut(), width, height, FilterType::CatmullRom)
    });
    Self::new(resized.into())
  }
  fn slice(&self, x: u32, y: u32, w: u32, h: u32) -> Self {
    let arc = self.0.clone();
    let [ix, iy, iw, ih] = self.1;
    self.get(|_image| {
      let nx = ix.saturating_add(x);
      let ny = iy.saturating_add(y);
      assert!(nx as u64 + w as u64 <= ix as u64 + iw as u64);
      assert!(ny as u64 + h as u64 <= iy as u64 + ih as u64);
      Self(arc, [nx, ny, w, h])
    })
  }
  fn tinted(&self, [r, g, b, a]: [u8; 4]) -> Self {
    Self::new(self.get(|image| {
      let mut texture = DynamicImage::from(image.to_image()).into_rgba8();
      for pixel in texture.pixels_mut() {
        pixel.0[0] = (r as f64 / 0xFF as f64 * pixel.0[0] as f64) as u8;
        pixel.0[1] = (g as f64 / 0xFF as f64 * pixel.0[1] as f64) as u8;
        pixel.0[2] = (b as f64 / 0xFF as f64 * pixel.0[2] as f64) as u8;
        pixel.0[3] = (a as f64 / 0xFF as f64 * pixel.0[3] as f64) as u8;
      }
      texture.into()
    }))
  }
}
