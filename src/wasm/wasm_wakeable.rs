use std::pin::Pin;
use std::sync::{LazyLock, Mutex};
use std::task::{Context, Poll, Waker};

use slotmap::{DefaultKey, Key, KeyData, SlotMap};

#[derive(Debug)]
enum WasmWakeableState {
  Unpolled,
  Waiting(Waker),
  Finished(WasmWakeableSize)
}
#[derive(Debug, PartialEq)]
pub enum WasmWakeableSize {
  Zero,
  One(u64),
  Two(u64, u64)
}

static WAKEABLES: LazyLock<Mutex<SlotMap<DefaultKey, WasmWakeableState>>> = LazyLock::new(Default::default);


#[unsafe(no_mangle)]
pub extern "C" fn provide_wakeable_zero(key: u64) -> u32 {
  provide_wakeable(key, WasmWakeableSize::Zero)
}
#[unsafe(no_mangle)]
pub extern "C" fn provide_wakeable_one(key: u64, one: u64) -> u32 {
  provide_wakeable(key, WasmWakeableSize::One(one))
}
#[unsafe(no_mangle)]
pub extern "C" fn provide_wakeable_two(key: u64, one: u64, two: u64) -> u32 {
  provide_wakeable(key, WasmWakeableSize::Two(one, two))
}

/// Provides a [WASMWakeable] with a value and wakes it
/// Return codes 0=success 1=unknown wakeable 2=invalid state
pub fn provide_wakeable(key: u64, value: WasmWakeableSize) -> u32 {
  let key = DefaultKey::from(KeyData::from_ffi(key));
  let mut wakeables = WAKEABLES.lock().unwrap();
  let Some(slot) = wakeables.get_mut(key) else { return 1 };
  match slot {
    WasmWakeableState::Unpolled => {
      *slot = WasmWakeableState::Finished(value);
      0
    }
    WasmWakeableState::Waiting(waker) => {
      waker.wake_by_ref();
      *slot = WasmWakeableState::Finished(value);
      0
    }
    WasmWakeableState::Finished(_) => 2
  }
}

/// A future that resolves once [provide_wakeable] has been called from wasm
pub(in crate) struct WasmWakeable(DefaultKey);
impl WasmWakeable {
  /// Constructs a new WASMWakeable, returning the key ID required to wake it and the future
  pub fn new() -> (u64, Self) {
    let key = WAKEABLES.lock().unwrap().insert(WasmWakeableState::Unpolled);
    (key.data().as_ffi(), Self(key))
  }
}
impl Future for WasmWakeable {
  type Output = Result<WasmWakeableSize, ()>;
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut state = WAKEABLES.lock().unwrap();
    let mut slot = state.get_mut(self.0);
    match slot {
      None => Poll::Ready(Err(())),
      Some(ref mut slot@(WasmWakeableState::Unpolled | WasmWakeableState::Waiting(_))) => {
        **slot = WasmWakeableState::Waiting(cx.waker().clone());
        Poll::Pending
      }
      Some(WasmWakeableState::Finished(_)) => {
        let Some(WasmWakeableState::Finished(value)) = state.remove(self.0) else { unreachable!() };
        Poll::Ready(Ok(value))
      }
    }
  }
}
impl Drop for WasmWakeable {
  fn drop(&mut self) {
    WAKEABLES.lock().unwrap().remove(self.0);
  }
}