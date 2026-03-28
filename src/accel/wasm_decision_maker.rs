use std::collections::HashMap;

use crate::accel::traits::ImportDecisionMaker;
use crate::import::inter_stage_data::DecisionTree;

#[derive(Debug)]
pub struct WasmDecisionMaker;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct WasmDecisionMakerError(String);

impl ImportDecisionMaker for WasmDecisionMaker {
  type Error = WasmDecisionMakerError;
  async fn decide(&self, _options: &[DecisionTree<'_>]) -> Result<HashMap<u64, usize>, Self::Error> {
    todo!()
  }
}