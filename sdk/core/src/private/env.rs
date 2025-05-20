#[cfg(not(target_arch = "wasm32"))]
#[path = "env/off_chain.rs"]
pub mod off_chain;
#[cfg(target_arch = "wasm32")]
#[path = "env/on_chain.rs"]
pub mod on_chain;
