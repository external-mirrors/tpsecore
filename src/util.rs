use std::fmt::Formatter;
use std::ops::Deref;
use std::sync::Arc;


/// A thin wrapper for an Arc<[u8]> with a concise Debug representation
pub struct Buffer(Arc<[u8]>);
impl std::fmt::Debug for Buffer {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "<{} bytes>", self.0.len())
  }
}
impl From<Arc<[u8]>> for Buffer {
  fn from(value: Arc<[u8]>) -> Self {
    Self(value)
  }
}
impl Into<Arc<[u8]>> for Buffer {
  fn into(self) -> Arc<[u8]> {
    self.0
  }
}
impl Deref for Buffer {
  type Target = Arc<[u8]>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}