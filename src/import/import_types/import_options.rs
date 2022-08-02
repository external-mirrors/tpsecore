#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ImportOptions {
  pub depth_limit: u8
}

impl Default for ImportOptions {
  fn default() -> Self {
    Self { depth_limit: 5 }
  }
}

impl ImportOptions {
  pub fn minus_one_depth(self) -> Self {
    ImportOptions { depth_limit: self.depth_limit - 1 }
  }
}