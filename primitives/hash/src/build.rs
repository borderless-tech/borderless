use std::fs::{self, File};
use std::io::prelude::*;
use std::path::Path;

fn main() -> Result<(), std::io::Error> {
    let inputs = &[Path::new("src/flatbuffer/hash.fbs")];
    let out_dir = Path::new("src/");
    flatc_rust::run(flatc_rust::Args {
        inputs,
        out_dir,
        ..Default::default()
    })
    .expect("flatc installed");
    Ok(())
}
