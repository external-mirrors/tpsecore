use std::convert::Infallible;
use std::sync::Arc;

use crate::accel::traits::TextureHandle;

#[derive(Clone, Debug)]
pub struct NullTextureHandle;

impl TextureHandle for NullTextureHandle {
  type Error = Infallible;

  fn new_texture(_width: u32, _height: u32) -> Self {
    NullTextureHandle
  }

  fn decode_texture(_buffer: Arc<[u8]>) -> Result<Self, Self::Error> {
    Ok(NullTextureHandle)
  }

  async fn width(&self) -> Result<u32, Self::Error> {
    Ok(1)
  }

  async fn height(&self) -> Result<u32, Self::Error> {
    Ok(1)
  }

  async fn encode_png(&self) -> Result<Arc<[u8]>, Self::Error> {
    Ok(include_bytes!("../../assets/empty.png").to_vec().into())
  }

  fn create_copy(&self) -> Self {
    Self
  }

  fn slice(&self, _x: u32, _y: u32, _width: u32, _height: u32) -> Self {
    Self
  }

  fn resized(&self, _width: u32, _height: u32) -> Self {
    Self
  }

  fn tinted(&self, _color: [u8; 4]) -> Self {
    Self
  }

  fn overlay(&self, _with_image: &Self, _x: i64, _y: i64) {
  }

  fn draw_line(&self, _start: (f32, f32), _end: (f32, f32), _color: [u8; 4]) {
  }

  fn draw_text(&self, _color: [u8; 4], _x: i32, _y: i32, _scale: f32, _text: &str) {
  }
}