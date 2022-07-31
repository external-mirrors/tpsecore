use std::io::Cursor;
use std::ops::{Deref, DerefMut};
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, Rgba, SubImage};
use image::imageops::FilterType;
use image::io::Reader;
use crate::import::skin_splicer::{LoadError, lookup_skin, Piece};
use crate::import::skin_splicer::maps::*;
use crate::import::SkinType;

#[derive(Default)]
pub struct SkinSplicer {
  images: Vec<(SkinType, DynamicImage)>
}
impl SkinSplicer {
  /// Loads an image into the SkinSplicer, putting it at the end of the queue
  pub fn load(&mut self, format: SkinType, file: &[u8]) -> Result<(), LoadError> {
    let image = Reader::new(Cursor::new(file)).with_guessed_format().unwrap().decode()?;
    self.images.push((format, image));
    Ok(())
  }

  /// Creates an empty canvas sized for the given format
  pub fn create_empty(&mut self, format: SkinType, block_size_override: Option<u32>) {
    let (image_width_ratio, image_height_ratio, mut block_size) = format.get_native_texture_size();
    if let Some(block_size_override) = block_size_override {
      block_size = block_size_override;
    }
    let width = (image_width_ratio * block_size as f64) as u32;
    let height = (image_height_ratio * block_size as f64) as u32;
    self.images.push((format, DynamicImage::new_rgb8(width, height)))
  }

  /// Draws a block, combining all available pieces. If no resolution is provided, the
  /// first available image size is used instead.
  pub fn get(&self, piece: Piece, connection: u8, resolution: Option<u32>)
    -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>>
  {
    let mut iter = self.lookup(piece, connection)
      .filter_map(|(skin_type, slices)| slices.map(|slices| (skin_type, slices)))
      .flat_map(|(skin_type, slices)| slices.map(move |slice| (skin_type, slice)));
    let (_, canvas) = iter.next()?;
    let mut buffer = canvas.to_image();
    if let Some(resolution) = resolution {
      buffer = image::imageops::resize(canvas.deref(), resolution, resolution, FilterType::CatmullRom);
    }
    for (_, canvas) in iter {
      let resized = image::imageops::resize(canvas.deref(), buffer.width(), buffer.height(), FilterType::CatmullRom);
      image::imageops::overlay(&mut buffer, &resized, 0, 0);
    }
    Some(buffer)
  }

  /// Draws a buffer to the first available slice for a block
  /// Returns `Some(())` if a slice was found and written to
  pub fn set(&mut self, piece: Piece, connection: u8, buffer: &DynamicImage) -> Option<()> {
    for mut slicer in self.lookup_mut(piece, connection) {
      let mut canvas = slicer.next_mut()?;
      let buffer = image::imageops::resize(buffer, canvas.width(), canvas.height(), FilterType::CatmullRom);
      image::imageops::overlay(canvas.deref_mut(), &buffer, 0, 0);
      return Some(());
    }
    None
  }

  /// Looks up slices matching the given piece and connection in all images
  pub fn lookup(&self, piece: Piece, connection: u8)
    -> impl Iterator<Item = (SkinType, Option<impl Iterator<Item = SubImage<&DynamicImage>>>)>
  {
    self.images.iter().map(move |(skin_type, image)| {
      let slices = lookup_skin(*skin_type, piece)
        .and_then(|skin_slice| skin_slice.slices(connection, image.width(), image.height()))
        .map(|iter| iter.map(|(x, y, w, h)| image.view(x, y, w, h)));
      (*skin_type,  slices)
    })
  }

  /// Looks up mutable slices matching the given piece and connections in all images
  pub fn lookup_mut(&mut self, piece: Piece, connection: u8)
    -> impl Iterator<Item = SliceLookup<&mut DynamicImage, impl Iterator<Item = (u32, u32, u32, u32)>>>
  {
    self.images.iter_mut().map(move |(skin_type, image)| {
      let skin_type = *skin_type;
      let w = image.width();
      let h = image.height();
      let iter = lookup_skin(skin_type, piece).and_then(|el| el.slices(connection, w, h));
      SliceLookup { skin_type, image, iter }
    })
  }
}

/// exists because lifetime limitations, need lending iterators :/
struct SliceLookup<T, IT> {
  pub skin_type: SkinType,
  pub image: T,
  pub iter: Option<IT>
}
impl<T, IT> SliceLookup<T, IT> where T: Deref<Target = DynamicImage>, IT: Iterator<Item = (u32, u32, u32, u32)> {
  fn next(&mut self) -> Option<SubImage<&DynamicImage>> {
    let (x, y, w, h) = self.iter.as_mut()?.next()?;
    Some(self.image.view(x, y, w, h))
  }
}
impl<T, IT> SliceLookup<T, IT> where T: DerefMut<Target = DynamicImage>, IT: Iterator<Item = (u32, u32, u32, u32)> {
  fn next_mut(&mut self) -> Option<SubImage<&mut DynamicImage>> {
    let (x, y, w, h) = self.iter.as_mut()?.next()?;
    Some(self.image.sub_image(x, y, w, h))
  }
}