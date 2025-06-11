use std::env;

fn main() {
    let log = env::var("CARGO_FEATURE_LOG").is_ok();
    let tracing = env::var("CARGO_FEATURE_TRACING").is_ok();

    if log && tracing {
        println!("cargo:warning=Both `log` and `tracing` features are enabled. Only one should be active, as one deactivates the other (so neither logging nor tracing is enabled).");
    }
}
