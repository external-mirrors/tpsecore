use std::borrow::Cow;
use std::ptr::null;

use crate::import::TPSEProviderError;
use crate::tpse::tpse_key::{TPSEProvider, TPSEKey};
use crate::wasm::{BUFFER_STATE, tpse_get, tpse_set};

pub struct WasmTPSEProvider(u32);

impl<T: TPSEKey> TPSEProvider<T> for WasmTPSEProvider {
  async fn get(&self, key: &T) -> Result<Option<Cow<'_, T::Data>>, TPSEProviderError> {
    let key_str = key.key();
    let ptr = unsafe { tpse_get(key_str.as_ptr(), key_str.len()) };
    if ptr == null() { return Ok(None) };
    
    let mut state = BUFFER_STATE.lock().unwrap();
    let lookup = state.lookup_buffer(ptr as *mut u8).expect("provided ptr to be put in tpse buffer storage");
    let buf = state.buffers.remove(&lookup).expect("lookup_buffer ptr to exist").clone();
    let Ok(result) = serde_json::from_slice(&buf) else { return Err(TPSEProviderError::Failed) };
    Ok(Some(Cow::Owned(result)))
  }

  async fn set(&mut self, key: &T, value: Option<T::Data>) -> Result<(), TPSEProviderError> {
    let key_str = key.key();
    let Ok(result) = serde_json::to_string(&value) else { return Err(TPSEProviderError::Failed) };
    unsafe { tpse_set(key_str.as_ptr(), key_str.len(), result.as_ptr(), result.len()); }
    Ok(())
  }
}