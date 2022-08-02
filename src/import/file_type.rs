use std::borrow::Cow;
use std::path::Path;

/// A broad category of content based on a file extension
pub enum FileType {
  Zip,
  TPSE,
  Image,
  Video,
  Audio
}

impl FileType {
  pub fn from_mime(string: &str) -> Option<FileType> {
    todo!()
  }

  pub fn from_extension(filename: &str) -> Option<FileType> {
    let ext = Path::new(&filename).extension()
      .map(|ext| ext.to_string_lossy())
      .unwrap_or(Cow::from(filename));
    match ext.as_ref() {
      "zip" => Some(FileType::Zip),
      "tpse" => Some(FileType::TPSE),
      "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" => Some(FileType::Image),
      "mp4" | "webm" => Some(FileType::Video),
      "ogg" | "mp3" | "flac" => Some(FileType::Audio),
      _ => return None
    }
  }
}