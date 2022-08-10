use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::io::{BufWriter, Cursor};
use std::str::FromStr;
use data_url::{DataUrl, DataUrlError, forgiving_base64};
use image::DynamicImage;
use sha2::{Digest, Sha256};


/// A parsed data url stored inside a tpse
#[derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr, Clone, Eq, PartialEq)]
pub struct File {
  /// The raw contents of the file
  pub binary: Vec<u8>,
  /// The MIME type of the file
  pub mime: String
}

#[derive(Debug, thiserror::Error)]
pub enum FileParseError {
  #[error("invalid data URI: {0:?}")]
  InvalidDataURI(DataUrlError),
  #[error("unparsable base64: {0:?}")]
  UnparsableBase64(forgiving_base64::InvalidBase64)
}

impl File {
  pub fn sha256(&self) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&self.binary);
    let mut hash: [u8; 32] = Default::default();
    hash.copy_from_slice(hasher.finalize().as_slice());
    hash
  }

  pub fn sha256_hex(&self) -> String {
    let string = hex::encode(self.sha256());
    string.to_ascii_uppercase();
    return string
  }
}

impl Debug for File {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    // write!(f, "File: {} {:x?}", self.mime, self.binary)
    write!(f, "File {{ mime: \"{}\", length: {} }}", self.mime, self.binary.len())
  }
}

impl Display for File {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "data:{};base64,{}", self.mime, base64::encode(&self.binary))
  }
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

impl From<DynamicImage> for File {
  fn from(img: DynamicImage) -> Self {
    // allocate 100% of the uncompressed size upfront for performance
    // this value was chosen randomly and could use some empirical testing
    log::trace!("Encoding image...");
    let mut binary = Vec::with_capacity(img.as_bytes().len());
    img.write_to(&mut Cursor::new(&mut binary), image::ImageOutputFormat::Png).unwrap();
    log::trace!("Done encoding image!");
    File { binary, mime: "image/png".to_string() }
  }
}