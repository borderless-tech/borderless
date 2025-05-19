#[cfg(not(target_arch = "wasm32"))]
pub mod off_chain;
#[cfg(target_arch = "wasm32")]
pub mod on_chain;
