use std::fmt::{Debug, Display, Formatter};
use std::io::{BufWriter, Cursor};
use std::str::FromStr;
use data_url::{DataUrl, DataUrlError, forgiving_base64};
use image::DynamicImage;


/// A parsed data url stored inside a tpse
#[derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr)]
pub struct File {
  /// The raw contents of the file
  pub binary: Vec<u8>,
  /// The MIME type of the file
  pub mime: String
}

impl From<DynamicImage> for File {
  fn from(img: DynamicImage) -> Self {
    // allocate 25% of the uncompressed size upfront for performance
    // this value was chosen randomly and could use some empirical testing
    let mut binary = Vec::with_capacity(img.as_bytes().len() / 4);
    img.write_to(&mut Cursor::new(&mut binary), image::ImageOutputFormat::Png).unwrap();
    File { binary, mime: "image/png".to_string() }
  }
}

impl Debug for File {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "File: {} {:x?}", self.mime, self.binary)
  }
}

impl Display for File {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "data:{};base64,{}", self.mime, base64::encode(&self.binary))
  }
}

#[derive(Debug, thiserror::Error)]
pub enum FileParseError {
  #[error("invalid data URI: {0:?}")]
  InvalidDataURI(DataUrlError),
  #[error("unparsable base64: {0:?}")]
  UnparsableBase64(forgiving_base64::InvalidBase64)
}

impl FromStr for File {
  type Err = FileParseError;
  fn from_str(value: &str) -> Result<Self, Self::Err> {
    let url = DataUrl::process(value).map_err(|err| FileParseError::InvalidDataURI(err))?;
    let mime = url.mime_type().to_string();
    let (binary, _) = url.decode_to_vec().map_err(|err| FileParseError::UnparsableBase64(err))?;
    Ok(File { binary, mime })
  }
}