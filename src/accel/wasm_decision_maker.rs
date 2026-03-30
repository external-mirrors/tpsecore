use std::collections::HashMap;
use std::path::PathBuf;

use crate::accel::traits::ImportDecisionMaker;
use crate::import::inter_stage_data::{DecisionTree, DecisionTreeOption};
use crate::wasm::BUFFER_STATE;
use crate::wasm::wasm_wakeable::{WasmWakeable, WasmWakeableSize};

#[link(wasm_import_module="wasm_decision_maker")]
unsafe extern "C" {
  unsafe fn decide(tpse_id: u32, data: *const u8, len: usize, async_wake_id: u64);
}

#[derive(Debug)]
pub struct WasmDecisionMaker { pub tpse_id: u32 }

#[derive(Debug, thiserror::Error)]
pub enum WasmDecisionMakerError {
  #[error("de/serialization error: {0}")]
  SerializationError(#[from] serde_json::Error),
  #[error("error from javascript: {0}")]
  JavascriptError(String)
}

impl ImportDecisionMaker for WasmDecisionMaker {
  type Error = WasmDecisionMakerError;
  async fn decide(&self, options: &[DecisionTree]) -> Result<HashMap<u64, usize>, Self::Error> {
    let boundary = options.iter().map(WasmDecisionTree::from_isd).collect::<Vec<_>>();
    let serialized = serde_json::to_string(&boundary)?;
    let (wake_id, future) = WasmWakeable::new();
    
    unsafe { decide(self.tpse_id, serialized.as_ptr(), serialized.len(), wake_id); }
    
    let Ok(WasmWakeableSize::Two(status, ptr)) = future.await else {
      panic!("incorrect size provided or sender dropped for ImportDecisionMaker::decide wakeup");
    };
    
    let mut state = BUFFER_STATE.lock().unwrap();
    let lookup = state.lookup_buffer(ptr as *mut u8).expect("provided ptr to be put in tpse buffer storage");
    let buf = state.buffers.remove(&lookup).expect("lookup_buffer ptr to exist").clone();
    
    if status != 0 {
      return Err(WasmDecisionMakerError::JavascriptError(String::from_utf8_lossy(&buf).into_owned()));
    }
    
    Ok(serde_json::from_slice(&buf)?)
  }
}

#[derive(serde::Serialize)]
struct WasmDecisionTree {
  id: u64,
  description: String,
  options: Vec<WasmDecisionTreeOption>
}
impl WasmDecisionTree {
  fn from_isd(tree: &DecisionTree) -> Self {
    Self {
      id: tree.id,
      description: tree.description.clone(),
      options: tree.options.iter().map(WasmDecisionTreeOption::from_isd).collect()
    }
  }
}

#[derive(serde::Serialize)]
struct WasmDecisionTreeOption {
  description: String,
  files: Vec<PathBuf>,
  subtrees: Vec<WasmDecisionTree>
}
impl WasmDecisionTreeOption {
  fn from_isd(option: &DecisionTreeOption) -> Self {
    Self {
      description: option.description.clone(),
      files: option.files.iter().map(|f| f.path.clone()).collect(),
      subtrees: option.subtrees.iter().map(WasmDecisionTree::from_isd).collect()
    }
  }
}