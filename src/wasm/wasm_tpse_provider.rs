use std::borrow::Cow;
use std::ptr::null;

use crate::tpse::tpse_key::{TPSEProvider, TPSEKey};
use crate::wasm::{State, tpse_get, tpse_set};

pub struct WasmTPSEProvider<'a>(&'a mut State);

impl<T: TPSEKey> TPSEProvider<T> for WasmTPSEProvider<'_> {
  fn get(&self, key: &T) -> Option<std::borrow::Cow<'_, <T as TPSEKey>::Data>> {
    let key_str = key.key();
    let ptr = unsafe { tpse_get(key_str.as_ptr(), key_str.len()) };
    if ptr == null() { return None };
    let lookup = self.0.lookup_buffer(ptr as *mut u8).expect("provided ptr to be put in tpse buffer storage");
    let buf = self.0.buffers.get(&lookup).expect("lookup_buffer ptr to exist").clone();
    let Ok(result) = serde_json::from_slice(&buf) else { todo!("error handling") };
    Some(Cow::Owned(result))
  }

  fn set(&mut self, key: &T, value: Option<<T as TPSEKey>::Data>) {
    let key_str = key.key();
    let Ok(result) = serde_json::to_string(&value) else { todo!("error handling") };
    unsafe { tpse_set(key_str.as_ptr(), key_str.len(), result.as_ptr(), result.len()); }
  }
}