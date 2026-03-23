use std::borrow::Cow;

use crate::import::TPSEProviderError;
use crate::tpse::tpse_key::{TPSEProvider, TPSEKey};
use crate::wasm::wasm_wakeable::{WasmWakeable, WasmWakeableSize};
use crate::wasm::{BUFFER_STATE, tpse_delete, tpse_get, tpse_set};

#[derive(Debug)]
pub struct WasmTPSEProvider(pub u32);

impl<T: TPSEKey> TPSEProvider<T> for WasmTPSEProvider {
  async fn get(&self, key: &T) -> Result<Option<Cow<'_, T::Data>>, TPSEProviderError> {
    let key_str = key.key();
    let (wake_id, future) = WasmWakeable::new();
    unsafe { tpse_get(self.0, key_str.as_ptr(), key_str.len(), wake_id) };
    
    let Ok(WasmWakeableSize::Two(status, ptr)) = future.await else {
      panic!("incorrect size provided or sender dropped for WasmTPSEProvider::get wakeup");
    };
    
    if status == 1 {
      return Ok(None);
    }
    
    let mut state = BUFFER_STATE.lock().unwrap();
    let lookup = state.lookup_buffer(ptr as *mut u8).expect("provided ptr to be put in tpse buffer storage");
    let buf = state.buffers.remove(&lookup).expect("lookup_buffer ptr to exist").clone();
    
    if status != 0 {
      return Err(TPSEProviderError::GeneralFailure(String::from_utf8_lossy(&buf).into_owned()));
    }
    
    let result = match serde_json::from_slice(&buf) {
      Ok(result) => result,
      Err(err) => return Err(TPSEProviderError::SerializationError(err)),
    };
    Ok(Some(Cow::Owned(result)))
  }

  async fn set(&mut self, key: &T, value: Option<T::Data>) -> Result<(), TPSEProviderError> {
    let key_str = key.key();
    let (wake_id, future) = WasmWakeable::new();
    
    match value {
      Some(value) => {
        let result = match serde_json::to_string(&value) {
          Ok(result) => result,
          Err(err) => return Err(TPSEProviderError::SerializationError(err)),
        };
        unsafe { tpse_set(self.0, key_str.as_ptr(), key_str.len(), result.as_ptr(), result.len(), wake_id); }
      },
      None => {
        unsafe { tpse_delete(self.0, key_str.as_ptr(), key_str.len(), wake_id); }
      }
    }
    
    let Ok(WasmWakeableSize::One(status_ptr)) = future.await else {
      panic!("incorrect size provided or sender dropped for WasmTPSEProvider::set wakeup");
    };
    
    if status_ptr != 0 {
      let mut state = BUFFER_STATE.lock().unwrap();
      let lookup = state.lookup_buffer(status_ptr as *mut u8).expect("provided ptr to be put in tpse buffer storage");
      let buf = state.buffers.remove(&lookup).expect("lookup_buffer ptr to exist").clone();
      return Err(TPSEProviderError::GeneralFailure(String::from_utf8_lossy(&buf).into_owned()));
    }
    Ok(())
  }
}