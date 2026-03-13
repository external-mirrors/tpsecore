//! This module contains a very small global singleton async runtime for embedding in wasm.

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Wake};

use crate::wasm::set_runtime_sleeping;

static STATE: Mutex<VecDeque<Arc<Task>>> = Mutex::new(VecDeque::new());

struct Task(Mutex<TaskInner>);
impl Wake for Task {
  fn wake(self: std::sync::Arc<Self>) {
    let mut guard = STATE.lock().unwrap();
    guard.push_back(self);
    unsafe { set_runtime_sleeping(false); }
  }
}

struct TaskInner {
  future: Pin<Box<dyn Future<Output = ()> + Sync + Send>>,
  finished: bool
}

/// Spawns a task on the runtime
pub fn spawn(task: impl Future<Output = ()> + Sync + Send + 'static) {
  let mut guard = STATE.lock().unwrap();
  guard.push_back(Arc::new(Task(Mutex::new(TaskInner {
    future: Box::pin(task),
    finished: false
  }))));
  unsafe { set_runtime_sleeping(false); }
}

/// Processes one task on the runtime
#[unsafe(no_mangle)]
pub extern "C" fn tick_async() {
  let task = match STATE.lock().unwrap().pop_front() {
    Some(task) => task,
    None => {
      unsafe { set_runtime_sleeping(true); }
      return;
    }
  };
  let waker = task.clone().into();
  let mut ctx = Context::from_waker(&waker);
  let mut task_guard = task.0.lock().unwrap();
  if task_guard.finished { return }
  let result = task_guard.future.as_mut().poll(&mut ctx);
  if result.is_ready() { task_guard.finished = true; }
}