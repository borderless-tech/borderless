cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        // Use on_chain environment
        pub mod on_chain;
    } else {
        // Use off_chain environment
        pub mod off_chain;
    }
}
