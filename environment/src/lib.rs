use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        // Use on_chain environment
        mod on_chain;
        pub use on_chain::EnvInstance;
    } else {
        // Use off_chain environment
        mod off_chain;
        pub use off_chain::EnvInstance;
    }
}
