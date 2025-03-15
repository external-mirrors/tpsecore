use regex::Regex;
use crate::import::import_error::AssetParseFailure;
use crate::import::import_error::AssetParseFailure::*;
use crate::import::ImportErrorType;
use crate::tpse::CustomSoundAtlas;

/// A parsed RSD file
/// See [`parse_radiance_sound_definition`]
pub struct RadianceSoundDefinition<'a> {
  /// The version of the RSD spec that was parsed, as (major, minor)
  pub version: (u32, u32),
  /// Sound effect sprites, stored as (name, offset_seconds, duration_seconds) tuples
  pub sprites: Vec<RadianceSprite<'a>>,
  /// Encoded audio data, currently ogg opus for the base game
  pub audio_buffer: &'a [u8]
}
impl RadianceSoundDefinition<'_> {
  /// Converts to the old-style atlas, as used in TETR.IO before β1.6.2 and TPSE files.
  /// This converts seconds to milliseconds.
  pub fn to_old_style_atlas(&self) -> CustomSoundAtlas {
    self.sprites.iter().map(|sprite| {
      (
        sprite.name.to_string(),
        (sprite.offset as f64 / 1000.0, sprite.duration as f64 / 1000.0)
      )
    }).collect()
  }
}

pub struct RadianceSprite<'a> {
  /// The name of the sprite
  pub name: &'a str,
  /// The offset of the audio sprite in the parsed audio buffer, in seconds
  pub offset: f32,
  /// The duration of the audio sprite, in seconds
  pub duration: f32
}

pub fn parse_radiance_sound_definition(rsd: &[u8]) -> Result<RadianceSoundDefinition, AssetParseFailure> {
  let mut buf = AtlasReadHelper { buffer: rsd, position: 0 };

  let header = u32::from_be_bytes(buf.read()?);
  if header != 0x74525344 {} // "tRSD" todo: emit header mismatch warning

  let major = u32::from_le_bytes(buf.read()?);
  let minor = u32::from_le_bytes(buf.read()?);
  if major != 1 && minor != 0 {} // todo: emit version mismatch warning

  let mut sprites = vec![];
  let mut last_audio_offset = 0.0;
  loop {
    let audio_offset = f32::from_le_bytes(buf.read()?);
    let name_length = u32::from_le_bytes(buf.read()?);
    if name_length == 0 {
      last_audio_offset = audio_offset;
      break
    }
    if name_length > 1000 { return Err(SoundEffectsAtlasNameTooLong { sprite: sprites.len(), length: name_length }) }
    let name = str::from_utf8(buf.read_n(name_length)?).map_err(|error| {
      SoundEffectsAtlasSpriteNameUTF8Error { sprite: sprites.len(), error }
    })?;
    sprites.push((name, audio_offset));
  }
  let sprites = sprites.iter().enumerate().map(|(i, (name, offset))| {
    let next_audio_offset = sprites.get(i+1).map(|x| x.1).unwrap_or(last_audio_offset);
    RadianceSprite { name, offset: *offset, duration: next_audio_offset - offset }
  }).collect::<Vec<_>>();

  let audio_len = u32::from_le_bytes(buf.read()?);
  let audio_buffer = buf.read_n(audio_len)?;
  buf.assert_eof()?;

  let version = (major, minor);
  Ok(RadianceSoundDefinition { version, sprites, audio_buffer })
}

struct AtlasReadHelper<'data> {
  buffer: &'data [u8],
  position: usize
}
impl<'data> AtlasReadHelper<'data> {
  pub fn read<const N: usize>(&mut self) -> Result<[u8; N], AssetParseFailure> {
    let start = self.position;
    self.position += N;
    let slice = self.buffer.get(start..self.position).ok_or(SoundEffectsAtlasEOF)?;
    Ok(slice.try_into().unwrap())
  }
  pub fn read_n<'reader>(&'reader mut self, n: u32) -> Result<&'data [u8], AssetParseFailure> {
    let n: usize = n.try_into().expect("unsupported arch width: u32 doesn't fit into usize");
    let start = self.position;
    self.position += n;
    self.buffer.get(start..self.position).ok_or(SoundEffectsAtlasEOF)
  }
  pub fn assert_eof(&self) -> Result<(), AssetParseFailure> {
    if self.position < self.buffer.len() {
      Err(SoundEffectsAtlasExpectedEOF { position: self.position, length: self.buffer.len() })
    } else {
      Ok(())
    }
  }
}