use std::io::Cursor;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use crate::accel::traits::{TPSEAccelerator, TextureHandle};
use crate::import::skin_splicer::{lookup_skin, Piece};
use crate::import::skin_splicer::maps::*;
use crate::import::{LoadError, SkinType};

pub struct SkinSplicer<T: TPSEAccelerator> {
  images: Vec<(SkinType, T::Texture)>
}
impl<T: TPSEAccelerator> Default for SkinSplicer<T> {
  fn default() -> Self {
    Self { images: vec![] }
  }
}
impl<T: TPSEAccelerator> SkinSplicer<T> {
  /// Loads an image into the SkinSplicer, putting it at the end of the queue
  pub fn load(&mut self, format: SkinType, file: Arc<[u8]>) -> Result<(), T::DecodeError> {
    let image = T::decode_texture(file)?;
    self.images.push((format, image));
    Ok(())
  }

  /// Loads a pre-decoded image into the SkinSplicer, putting it at the end of the queue
  pub fn load_decoded(&mut self, format: SkinType, image: T::Texture) {
    self.images.push((format, image))
  }

  /// Creates an empty canvas sized for the given format
  pub fn create_empty(&mut self, format: SkinType, block_size_override: Option<u32>) {
    let (image_width_ratio, image_height_ratio, mut block_size) = format.get_native_texture_size();
    if let Some(block_size_override) = block_size_override {
      block_size = block_size_override;
    }
    let width = (image_width_ratio * block_size as f64) as u32;
    let height = (image_height_ratio * block_size as f64) as u32;
    self.images.push((format, T::new_texture(width, height)))
  }

  /// Draws a block, combining all available pieces. If no resolution is provided, the
  /// first available image size is used instead. If the connection or piece isn't supported
  /// by the loaded skins, None will be returned.
  pub async fn get(&self, piece: Piece, connection: u8, resolution: Option<u32>)
    -> Option<<T as TPSEAccelerator>::Texture>
  {
    let (texture, skin_slice) = self.images.iter()
      .filter_map(|(skin_type, tex)| Some((tex, lookup_skin(*skin_type, piece)?)))
      .next()?;
    let mut slice_iter = skin_slice.slices(connection, texture.width().await, texture.height().await)?;
    let (x, y, w, h) = slice_iter.next()?;
    let mut canvas = texture.slice(x, y, w, h).create_copy();
    
    if let Some(resolution) = resolution {
      canvas = canvas.resized(resolution, resolution);
    }
    for (x, y, w, h) in slice_iter {
      let next_canvas = texture.slice(x, y, w, h).resized(canvas.width().await, canvas.height().await);
      canvas.overlay(&next_canvas, 0, 0);
    }
    Some(canvas)
  }

  /// Draws a buffer to the first available slice for a block
  /// Returns `Some(())` if a slice was found and written to
  pub async fn set(&mut self, piece: Piece, connection: u8, buffer: &<T as TPSEAccelerator>::Texture) -> Option<()> {
    let (texture, skin_slice) = self.images.iter()
      .filter_map(|(skin_type, tex)| Some((tex, lookup_skin(*skin_type, piece)?)))
      .next()?;
    let mut slices = skin_slice.slices(connection, texture.width().await, texture.height().await)?;
    let (x, y, w, h) = slices.next()?;
    let sliced = texture.slice(x, y, w, h);
    log::trace!("Resizing image: {} {} -> {} {}", buffer.width().await, buffer.height().await, w, h);
    sliced.overlay(&buffer.resized(w, h), 0, 0);
    Some(())
  }

  /// Creates a skin of the given output type, returning None if no blocks were
  /// available to draw to it.
  pub async fn convert(&self, target_type: SkinType, block_size_override: Option<u32>)
    -> Option<<T as TPSEAccelerator>::Texture>
  {
    log::trace!(
      "Converting splicer of {:?} to {:?}",
      self.images.iter().map(|el| el.0).collect::<Vec<_>>(),
      target_type
    );
    let mut target = Self::default();
    target.create_empty(target_type, block_size_override);
    let mut valid = false;

    for piece in Piece::values() {
      for conn in tetrio_connections_submap.connections.keys() {
        let default_conn = tetrio_connections_submap.default;
        let texture = match self.get(*piece, *conn, block_size_override).await {
          None => self.get(*piece, default_conn, block_size_override).await,
          Some(x) => Some(x)
        };

        if let Some(texture) = texture {
          if let Some(()) = target.set(*piece, *conn, &texture).await {
            valid = true;
          }
        }
      }
    }

    log::trace!("Conversion finished!");
    if valid { Some(target.images.remove(0).1) } else { None }
  }

  /// Returns the number of loaded iamges
  pub fn len(&self) -> usize {
    self.images.len()
  }
}