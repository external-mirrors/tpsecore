use std::collections::HashMap;

use crate::accel::traits::ImportDecisionMaker;
use crate::import::inter_stage_data::DecisionTree;

#[derive(Debug)]
pub struct DefaultDecisionMaker;

#[derive(Debug, thiserror::Error)]
#[error("encountered a decision with two or more options")]
pub struct DefaultDecisionMakerError;

impl ImportDecisionMaker for DefaultDecisionMaker {
  type Error = DefaultDecisionMakerError;

  async fn decide(&self, options: &[DecisionTree]) -> Result<HashMap<u64, usize>, Self::Error> {
    let mut decisions = HashMap::new();
    let mut queue = options.iter().collect::<Vec<_>>();
    while let Some(next) = queue.pop() {
      if next.options.is_empty() { continue }
      let [option] = &next.options[..] else { return Err(DefaultDecisionMakerError) };
      decisions.insert(next.id, 0);
      queue.extend(option.subtrees.iter());
    }
    Ok(decisions)
  }
}