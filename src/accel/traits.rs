use std::sync::Arc;


pub trait TPSEAccelerator {
  type Texture: TextureHandle;
  type DecodeError: std::error::Error + Send + Sync + 'static;
  
  /// Creates a new texture of the given size, filled with transparency
  fn new_texture(width: u32, height: u32) -> Self::Texture;
  fn decode_texture(buffer: Arc<[u8]>) -> Result<Self::Texture, Self::DecodeError>;
}

/// A handle to a texture.
/// Cloning the handle still points to the original texture. Use [create_copy] to create an independent copy.
/// Some methods mutate, and some create new versions.
pub trait TextureHandle: Clone {
  type Error: std::error::Error + Send + Sync + 'static;
  async fn width(&self) -> Result<u32, Self::Error>;
  async fn height(&self) -> Result<u32, Self::Error>;
  async fn encode_png(&self) -> Result<Arc<[u8]>, Self::Error>;
  /// Creates a standalone copy of the underlying texture
  fn create_copy(&self) -> Self;
  /// Creates a view of the texture. Modifying the view with in-place methods will modify the original texture.
  fn slice(&self, x: u32, y: u32, width: u32, height: u32) -> Self;
  /// Creates a resized copy of the texture
  fn resized(&self, width: u32, height: u32) -> Self;
  /// Creates a tinted copy of the texture
  fn tinted(&self, color: [u8; 4]) -> Self;
  /// Overlays another image on top of the texture in-place
  fn overlay(&self, with_image: &Self, x: i64, y: i64);
  /// Draws a line on the texture in-place
  fn draw_line(&self, start: (f32, f32), end: (f32, f32), color: [u8; 4]);
  /// Draws text on the texture in-place
  fn draw_text(&self, color: [u8; 4], x: i32, y: i32, scale: f32, text: &str);
}