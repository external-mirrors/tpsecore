// A short story: some dependencies depend on getrandom.
//
// getrandom has this nice big warning in its readme:
// > WARNING: We strongly recommend against enabling this feature in libraries (except for tests) since it is known to break non-Web WASM builds and further since the usage of wasm-bindgen causes significant bloat to Cargo.lock (on all targets).
//
// The dependency does not care. It enables it anyway.
// My wasm build is broken with `TypeError: import object field '__wbindgen_placeholder__' is not an Object`,
// as I do not like wasm-bindgen and no longer use it.
//
// I hate the dependency. Why does it even need random numbers?
// Soon I shall remove the dependency altogether, as it has ugly subdependency requirements and is also slow!
// I do however appreciate getrandom and its simple API surface.


#[derive(Debug)]
pub struct Error;
impl std::error::Error for Error {}
impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "No random numbers for you!")
  }
}

pub fn getrandom(_dest: &mut [u8]) -> Result<(), Error> {
  unimplemented!("No random numbers for you!")
}