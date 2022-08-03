use std::sync::Arc;
use crate::import::asset_provider::{AssetProvider, DefaultAssetProvider};

#[derive(Copy, Clone)]
pub struct ImportOptions<'a> {
  pub depth_limit: u8,
  pub asset_source: &'a dyn AssetProvider
}

impl ImportOptions<'_> {
  pub fn minus_one_depth(&self) -> Self {
    ImportOptions {
      depth_limit: self.depth_limit - 1,
      asset_source: self.asset_source
    }
  }
}