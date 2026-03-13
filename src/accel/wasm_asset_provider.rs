use std::pin::Pin;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::task::Poll;

use crate::accel::traits::AssetProvider;
use crate::import::Asset;
use crate::wasm::STATE;

#[derive(Debug)]
pub struct WasmAssetProvider;

#[derive(Debug, thiserror::Error)]
#[error("provide_asset was called with a failure code for asset {0}")]
pub struct WasmAssetProviderError(Asset);

impl AssetProvider for WasmAssetProvider {
  type Error = WasmAssetProviderError;

  async fn provide(&self, asset: Asset) -> Result<Arc<[u8]>, Self::Error> {
    let (tx, rx) = sync_channel(1);
    match AwaitAssetFuture(asset, Some(tx), Mutex::new(rx)).await {
      Ok(ok) => Ok(ok),
      Err(()) => Err(WasmAssetProviderError(asset))
    }
  }
}

type AssetFetchResult = Result<Arc<[u8]>, ()>;
static ASSET_TETRIOJS_LISTENERS: Mutex<Vec<Box<dyn FnOnce(AssetFetchResult) + Send>>> = Mutex::new(vec![]);
static ASSET_TETRIORSD_LISTENERS: Mutex<Vec<Box<dyn FnOnce(AssetFetchResult) + Send>>> = Mutex::new(vec![]);

/// Provides an asset. If asset provision fails, a null pointer should be passed.
/// This method does not clean up the given buffer, [deallocate_buffer] should be used for that.
/// It does create a refcounted copy, so immediately deallocating the buffer is safe - though an
/// asset may be requested multiple times over the course of an import, so caching is advised.
///
/// Return codes: 0=ok, 1=no such buffer, 2=unknown asset type
#[unsafe(no_mangle)]
pub extern "C" fn provide_asset(asset_type: u8, ptr: *mut u8) -> u32 {
  let state = STATE.lock().unwrap();
  let Ok(asset) = Asset::try_from(asset_type) else { return 2 };
  let listeners = match asset {
    Asset::TetrioJS => &ASSET_TETRIOJS_LISTENERS,
    Asset::TetrioRSD => &ASSET_TETRIORSD_LISTENERS
  };
  let result = if ptr == null_mut() {
    Err(())
  } else {
    let Some(id) = state.lookup_buffer(ptr) else { return 1 };
    let buffer = state.buffers.get(&id).unwrap();
    Ok(buffer)
  };
  for listener in listeners.lock().unwrap().drain(..) {
    (listener)(result.cloned());
  }
  0
}

#[link(wasm_import_module="wasm_accelerator_asset")]
unsafe extern "C" {
  /// Requests an external asset be fetched and provided back asynchronously to `provide_asset`
  unsafe fn fetch_asset(asset_id: u32);
}

struct AwaitAssetFuture(Asset, Option<SyncSender<AssetFetchResult>>, Mutex<Receiver<AssetFetchResult>>);
impl Future for AwaitAssetFuture {
  type Output = AssetFetchResult;

  fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
    if let Some(sender) = self.1.take() {
      let listeners = match self.0 {
        Asset::TetrioJS => &ASSET_TETRIOJS_LISTENERS,
        Asset::TetrioRSD => &ASSET_TETRIORSD_LISTENERS
      };
      let waker = cx.waker().clone();
      listeners.lock().unwrap().push(Box::new(move |data| {
        let _ = sender.send(data.clone());
        waker.wake();
      }));
      unsafe { fetch_asset(self.0 as u32); }
    }
    if let Ok(recv) = self.2.lock().unwrap().try_recv() {
      return Poll::Ready(recv);
    }
    Poll::Pending
  }
}