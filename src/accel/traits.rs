use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Range;
use std::sync::Arc;

use crate::import::Asset;
use crate::import::inter_stage_data::DecisionTree;

/// A bundle of implementations of functionality used during the TPSE import and rendering process.
/// These are constructed piecemeal out of things like software vs externally-accelerated-in-wasm
/// image processing, for example.
pub trait TPSEAccelerator: Debug {
  type Decider: ImportDecisionMaker;
  type Asset: AssetProvider;
  type Texture: TextureHandle;
  type Audio: AudioHandle;
}

pub trait ImportDecisionMaker: Debug {
  type Error: std::error::Error + Send + Sync + 'static;
  fn decide(&self, options: &[DecisionTree]) -> impl Future<Output = Result<HashMap<u64, usize>, Self::Error>>;
}

pub trait AssetProvider: Debug {
  type Error: std::error::Error + Send + Sync + 'static;
  fn provide(&self, asset: Asset) -> impl Future<Output = Result<Arc<[u8]>, Self::Error>>;
}

/// A handle to a texture.
/// Cloning the handle still points to the original texture. Use [create_copy] to create an independent copy.
/// Some methods mutate, and some create new versions.
#[allow(async_fn_in_trait)] // maybe fix later
pub trait TextureHandle: Clone + Debug {
  type Error: std::error::Error + Send + Sync + 'static;
  
  /// Creates a new texture of the given size, filled with transparency
  fn new_texture(width: u32, height: u32) -> Self;
  fn decode_texture(buffer: Arc<[u8]>) -> Result<Self, Self::Error>;
  
  fn width(&self) -> impl Future<Output = Result<u32, Self::Error>>;
  fn height(&self) -> impl Future<Output = Result<u32, Self::Error>>;
  fn encode_png(&self) -> impl Future<Output = Result<Arc<[u8]>, Self::Error>>;
  fn fraction_opaque(&self) -> impl Future<Output = Result<f32, Self::Error>>;
  
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

#[allow(async_fn_in_trait)] // maybe fix later
pub trait AudioHandle: Clone + Debug {
  type Error: std::error::Error + Send + Sync + 'static;
  
  fn new_from_samples(samples: Arc<[f32]>) -> Self;
  fn decode_audio(buffer: Arc<[u8]>, mime_type: Option<&str>) -> impl Future<Output = Result<Self, Self::Error>>;
  
  fn slice(&self, slice: Range<usize>) -> Self;
  /// Returns the length of the buffer in samples.
  /// For multi-channel buffers, samples are interleaved and counted once per channel.
  fn length(&self) -> impl Future<Output = Result<usize, Self::Error>>;
  /// Reads sample data from the audio's internal buffer into the provided buffer
  fn read(&self, accept: impl FnMut(f32)) -> impl Future<Output = Result<(), Self::Error>>;
  
  fn encode_ogg(chunks: &[Self]) -> impl Future<Output = Result<Arc<[u8]>, Self::Error>>;
}