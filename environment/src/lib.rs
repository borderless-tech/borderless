pub mod api;
// Re-export symbols to flatten the API
pub use api::*;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        // Use on_chain environment
        mod on_chain;
    } else {
        // Use off_chain environment
        mod off_chain;
    }
}
