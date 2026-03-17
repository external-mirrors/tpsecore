use std::ptr::null;
use std::sync::{Arc, Mutex};

use crate::accel::traits::AudioHandle;
use crate::wasm::BUFFER_STATE;
use crate::wasm::wasm_wakeable::{WasmWakeable, WasmWakeableSize};

#[link(wasm_import_module="wasm_accelerator_audio")]
unsafe extern "C" {
  unsafe fn new_from_samples(new_id: u32, ptr: *const u8, len: usize);
  unsafe fn decode_audio(new_id: u32, data_ptr: *const u8, mime_len: usize, mime_ptr: *const u8, ext_len: usize, wake_id: u64);
  unsafe fn slice(id: u32, new_id: u32, start: usize, len: usize);
  unsafe fn length(id: u32) -> usize;
  unsafe fn read(id: u32, ptr: *mut u8, len: usize) -> u32;
  unsafe fn encode_ogg(id_buf_ptr: *const u8, id_buf_ptr_len: usize, wake_id: u64);
}

static AUDIO_HANDLE_ID_COUNTER: Mutex<u32> = Mutex::new(0);
fn next_id() -> u32 {
  let mut guard = AUDIO_HANDLE_ID_COUNTER.lock().unwrap();
  let Some(new_id) = guard.checked_add(1) else { panic!("out of IDs") };
  std::mem::replace(&mut *guard, new_id)
}

#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Copy, Clone)]
#[repr(C)]
pub struct WasmAudioHandle(u32);

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct WasmAudioError(String);

impl AudioHandle for WasmAudioHandle {
  type Error = WasmAudioError;

  fn new_from_samples(samples: Arc<[f32]>) -> Self {
    let id = next_id();
    let samples = bytemuck::cast_slice::<f32, u8>(&samples);
    unsafe { new_from_samples(id, samples.as_ptr(), samples.len()); }
    Self(id)
  }

  async fn decode_audio(buffer: Arc<[u8]>, mime_type: Option<&str>) -> Result<Self, Self::Error> {
    let id = next_id();
    let (wake_id, future) = WasmWakeable::new();
    let (ext_ptr, ext_len) = match mime_type {
      Some(ext) => (ext.as_ptr(), ext.len()),
      None => (null(), 0)
    };
    unsafe { decode_audio(id, buffer.as_ptr(), buffer.len(), ext_ptr, ext_len, wake_id); }
    
    let Ok(WasmWakeableSize::One(error_ptr)) = future.await else {
      panic!("incorrect size provided or sender dropped for decode_audio wakeup");
    };
    
    match error_ptr {
      0 => Ok(Self(id)),
      error_ptr => {
        let mut buffers = BUFFER_STATE.lock().unwrap();
        let id = buffers.lookup_buffer(error_ptr as *const u8).expect("wasm provided ptr to be for a valid buffer");
        let buffer = buffers.buffers.remove(&id).expect("wasm provided ptr to be for a valid buffer");
        Err(WasmAudioError(String::from_utf8_lossy(&buffer[..]).into_owned()))
      }
    }
  }

  fn slice(&self, slicearg: std::ops::Range<usize>) -> Self {
    let id = next_id();
    unsafe { slice(self.0, id, slicearg.start, slicearg.end - slicearg.start); }
    Self(id)
  }

  async fn length(&self) -> Result<usize, Self::Error> {
    Ok(unsafe { length(self.0) })
  }

  async fn read(&self, mut accept: impl FnMut(f32)) -> Result<(), Self::Error> {
    let len = unsafe { length(self.0) };
    let mut buf = vec![0; len];
    let code = unsafe { read(self.0, buf.as_mut_ptr(), buf.len()) };
    if code != 0 { return Err(WasmAudioError(format!("todo: route error message (read returned {code})"))) }
    for sample in bytemuck::cast_slice::<u8, f32>(&buf) {
      accept(*sample);
    }
    Ok(())
  }

  async fn encode_ogg(chunks: &[Self]) -> Result<Arc<[u8]>, Self::Error> {
    let buf = bytemuck::cast_slice::<Self, u8>(chunks);
    let (wake_id, future) = WasmWakeable::new();
    unsafe { encode_ogg(buf.as_ptr(), buf.len(), wake_id); }
    
    let Ok(WasmWakeableSize::Two(status, ptr)) = future.await else {
      panic!("incorrect size provided or sender dropped for encode_ogg wakeup");
    };
    let mut buffers = BUFFER_STATE.lock().unwrap();
    let id = buffers.lookup_buffer(ptr as *const u8).expect("wasm provided ptr to be for a valid buffer");
    let buffer = buffers.buffers.remove(&id).expect("wasm provided ptr to be for a valid buffer");
    
    match status {
      0 => Ok(buffer),
      _ => Err(WasmAudioError(String::from_utf8_lossy(&buffer[..]).into_owned()))
    }
  }
}