use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::pin::Pin;
use std::sync::Arc;

use crate::import::ImportErrorType;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, serde::Deserialize)]
pub enum Asset {
  /// The main TETR.IO source code file, located at https://tetr.io/js/tetrio.js.
  #[serde(alias = "tetrio.js")]
  TetrioJS = 0,
  /// The TETR.IO sound effects file, located at https://tetr.io/sfx/tetrio.ogg.
  #[serde(alias = "tetrio.ogg")]
  TetrioOGG = 1
}
impl TryFrom<u8> for Asset {
  type Error = ();

  fn try_from(value: u8) -> Result<Self, Self::Error> {
    match value {
      0 => Ok(Self::TetrioJS),
      1 => Ok(Self::TetrioOGG),
      _ => Err(())
    }
  }
}
impl Display for Asset {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Asset::TetrioJS => write!(f, "tetrio.js"),
      Asset::TetrioOGG => write!(f, "tetrio.ogg")
    }
  }
}

pub trait AssetProvider {
  fn provide(&self, asset: Asset) -> Pin<Box<dyn Future<Output = Result<Arc<[u8]>, ImportErrorType>> + Send + Sync + '_>>;
}

#[derive(Default, Clone)]
pub struct DefaultAssetProvider {
  cache: HashMap<Asset, Arc<[u8]>>
}

impl DefaultAssetProvider {
  pub fn preload(&mut self, asset: Asset, data: impl Into<Arc<[u8]>>) {
    self.cache.insert(asset, data.into());
  }
}

impl AssetProvider for DefaultAssetProvider {
  #[cfg(target_arch = "wasm32")]
  fn provide(&self, asset: Asset) -> Pin<Box<dyn Future<Output = Result<Arc<[u8]>, ImportErrorType>> + Send + Sync + '_>> {
    Box::pin(async move {
      self.cache.get(&asset).cloned().ok_or(ImportErrorType::AssetNotPreloaded(asset))
    })
  }

  #[cfg(not(target_arch = "wasm32"))]
  fn provide(&self, asset: Asset) -> Pin<Box<dyn Future<Output = Result<Arc<[u8]>, ImportErrorType>> + Send + Sync + '_>> {
    Box::pin(async {
      match self.cache.get(&asset) {
        Some(cached) => Ok(cached.as_slice()),
        None => todo!()
      }
    })
  }
}