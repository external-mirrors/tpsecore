use std::borrow::Cow;
use std::ptr::null;

use crate::import::TPSEProviderError;
use crate::tpse::tpse_key::{TPSEProvider, TPSEKey};
use crate::wasm::{BUFFER_STATE, tpse_get, tpse_set};

#[derive(Debug)]
pub struct WasmTPSEProvider(pub u32);

impl<T: TPSEKey> TPSEProvider<T> for WasmTPSEProvider {
  async fn get(&self, key: &T) -> Result<Option<Cow<'_, T::Data>>, TPSEProviderError> {
    let key_str = key.key();
    let ptr = unsafe { tpse_get(self.0, key_str.as_ptr(), key_str.len()) };
    if ptr == null() { return Ok(None) };
    
    let mut state = BUFFER_STATE.lock().unwrap();
    let lookup = state.lookup_buffer(ptr as *mut u8).expect("provided ptr to be put in tpse buffer storage");
    let buf = state.buffers.remove(&lookup).expect("lookup_buffer ptr to exist").clone();
    let result = match serde_json::from_slice(&buf) {
      Ok(result) => result,
      Err(err) => return Err(TPSEProviderError::SerializationError(err)),
    };
    Ok(Some(Cow::Owned(result)))
  }

  async fn set(&mut self, key: &T, value: Option<T::Data>) -> Result<(), TPSEProviderError> {
    let key_str = key.key();
    let result = match serde_json::to_string(&value) {
      Ok(result) => result,
      Err(err) => return Err(TPSEProviderError::SerializationError(err)),
    };
    unsafe { tpse_set(self.0, key_str.as_ptr(), key_str.len(), result.as_ptr(), result.len()); }
    Ok(())
  }
}