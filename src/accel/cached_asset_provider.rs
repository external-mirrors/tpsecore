use std::collections::HashMap;
use std::sync::Arc;

use crate::accel::traits::AssetProvider;
use crate::import::Asset;

#[derive(Debug, Default)]
pub struct CachedAssetProvider {
  pub cache: HashMap<Asset, Arc<[u8]>>
}

#[derive(thiserror::Error, Debug)]
#[error("the {0} asset was not preloaded and the cached asset provider cannot fetch it")]
pub struct CachedAssetProviderError(Asset);

impl AssetProvider for CachedAssetProvider {
  type Error = CachedAssetProviderError;

  async fn provide(&self, asset: Asset) -> Result<Arc<[u8]>, Self::Error> {
    self.cache.get(&asset).cloned().ok_or(CachedAssetProviderError(asset))
  }
}