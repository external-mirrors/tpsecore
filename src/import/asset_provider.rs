use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use crate::import::ImportErrorType;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, serde::Deserialize)]
pub enum Asset {
  /// The main TETR.IO source code file, located at https://tetr.io/js/tetrio.js
  #[serde(alias = "tetrio.js")]
  TetrioJS,
  /// The TETR.IO sound effects file, located at https://tetr.io/sfx/tetrio.opus.rsd
  #[serde(alias = "tetrio.opus.rsd")]
  TetrioRSD
}
impl Display for Asset {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Asset::TetrioJS => write!(f, "tetrio.js"),
      Asset::TetrioRSD => write!(f, "tetrio.opus.rsd")
    }
  }
}

pub trait AssetProvider {
  fn provide(&self, asset: Asset) -> Result<&[u8], ImportErrorType>;
}

#[derive(Default, Clone)]
pub struct DefaultAssetProvider {
  cache: HashMap<Asset, Arc<Vec<u8>>>
}

impl DefaultAssetProvider {
  pub fn preload(&mut self, asset: Asset, data: Vec<u8>) {
    self.cache.insert(asset, Arc::new(data));
  }
}

impl AssetProvider for DefaultAssetProvider {
  #[cfg(target_arch = "wasm32")]
  fn provide(&self, asset: Asset) -> Result<&[u8], ImportErrorType> {
    self.cache.get(&asset).map(|el| el.as_slice()).ok_or(ImportErrorType::AssetNotPreloaded(asset))
  }

  #[cfg(not(target_arch = "wasm32"))]
  fn provide(&self, asset: Asset) -> Result<&[u8], ImportErrorType> {
    match self.cache.get(&asset) {
      Some(cached) => Ok(cached.as_slice()),
      None => todo!()
    }
  }
}