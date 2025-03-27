use std::{
    fs::{read_to_string, File},
    io::{stdin, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    process::exit,
};

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::{Map, Value};

use json_proof::*;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Document which should be converted
    document: Option<PathBuf>,

    /// Save output to file instead of writing to stdout
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Use Given mask to generate obfuscated document
    #[arg(short, long, value_name = "FILE")]
    mask: Option<PathBuf>,

    /// Generates a mask for the document, that can then later be used with --mask
    #[arg(long = "gen-mask")]
    gen_mask: bool,

    /// Generates the proof (root-hash) for the given document
    #[arg(long = "gen-proof")]
    gen_proof: bool,
}

/// Reads the json either from a file or via pipe
fn get_json(document: Option<&Path>) -> Result<Value> {
    let mut buf = String::with_capacity(128);
    match document {
        Some(path) => {
            let mut file = File::open(path)?;
            file.read_to_string(&mut buf)?;
        }
        None => {
            if stdin().is_terminal() {
                eprintln!("Error: Please provide an input file or pipe data into the program.");
                exit(1);
            }
            stdin().read_to_string(&mut buf)?;
        }
    }
    let value = serde_json::from_str(&buf)?;
    Ok(value)
}

fn generate_output(value: &Map<String, Value>, output: Option<PathBuf>) -> Result<()> {
    let pretty = serde_json::to_string_pretty(value)?;
    match output {
        Some(path) => {
            let mut file = File::create(&path)?;
            file.write_all(pretty.as_bytes())?;
            println!("Wrote content to: {}", path.display());
        }
        None => println!("{pretty}"),
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let json_input = get_json(args.document.as_deref())?;

    // Sorts all keys of all nested objects in the json
    let mut canonicalized = canonicalize_json(json_input)
        .context("Please provide a json object and not just a simple json field.")?;

    // Generate a new mask for the document
    if args.gen_mask {
        let mask = generate_mask(&canonicalized);
        generate_output(&mask, args.output)?;
        return Ok(());
    }

    // Print out the proof for the given document
    if args.gen_proof {
        let proof = gen_proof(&mut canonicalized)?;
        println!("{proof}");
        return Ok(());
    }

    if let Some(path) = args.mask {
        // Load mask from given path
        let s = read_to_string(path)?;
        let mask: Value = serde_json::from_str(&s)?;
        let mask = canonicalize_json(mask)?;
        check_mask(&mask, &canonicalized)?;
        prepare_document(&mut canonicalized);
        let proof = split_out_proof(&mut canonicalized);

        let obfuscated = apply_mask(&mask, &canonicalized, &proof);
        generate_output(&obfuscated, args.output)?;

        return Ok(());
    }

    prepare_document(&mut canonicalized);
    let proof = split_out_proof(&mut canonicalized);

    generate_output(&proof, args.output)?;

    Ok(())
}
