use std::pin::Pin;
use std::ptr::null_mut;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::task::Poll;

use crate::import::{Asset, AssetProvider, ImportErrorType};
use crate::wasm::{fetch_asset, STATE};

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

pub(in super) struct WasmAssetProvider;
impl AssetProvider for WasmAssetProvider {
  fn provide(&self, asset: Asset) -> Pin<Box<dyn Future<Output = Result<Arc<[u8]>, ImportErrorType>> + Send + Sync + '_>> {
    Box::pin(async move {
      let (tx, rx) = sync_channel(1);
      match AwaitAssetFuture(asset, Some(tx), Mutex::new(rx)).await {
        Ok(ok) => Ok(ok),
        Err(()) => Err(ImportErrorType::AssetFetchFailed(asset, "provide_asset was called with a failure code".to_string()))
      }
    })
  }
}

type AssetFetchResult = Result<Arc<[u8]>, ()>;
static ASSET_TETRIOJS_LISTENERS: Mutex<Vec<Box<dyn FnOnce(AssetFetchResult) + Send>>> = Mutex::new(vec![]);
static ASSET_TETRIORSD_LISTENERS: Mutex<Vec<Box<dyn FnOnce(AssetFetchResult) + Send>>> = Mutex::new(vec![]);

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