pub mod tpse;
pub mod import;
pub mod render;
pub mod log;
#[cfg(target_arch = "wasm32")]
mod wasm;
pub mod accel;

// library cleanup todos:
// - Reintroduce lifetimes into the tpse management to reduce memory overhead

#[cfg(test)]
mod tests;
