use std::{fs::read_to_string, path::PathBuf, time::Instant};

use anyhow::Result;
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_runtime::Runtime;
use borderless_sdk::ContractId;
use clap::Parser;

use log::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the database directory
    #[arg(short, long)]
    db: PathBuf,

    /// Path to the Wasm contract
    #[arg(short, long)]
    contract: PathBuf,

    /// Path to the json data of the action that we want to execute
    #[arg(short, long)]
    action: PathBuf,
}

fn main() -> Result<()> {
    colog::init();
    let args = Cli::parse();
    info!("📁 Database path: {}", args.db.display());
    info!("📦 Contract path: {}", args.contract.display());
    info!("📦 Action path: {}", args.action.display());

    let db = Lmdb::new(&args.db, 2)?;
    let action_data = read_to_string(args.action)?;
    let action = action_data.parse()?;

    let mut rt = Runtime::new(db)?;

    let cid = ContractId::generate();
    info!("Using contract-id: {cid}");

    info!("Instantiate contract {cid}");
    rt.instantiate_contract(cid, args.contract)?;

    info!("Run contract {cid}");
    let start = Instant::now();
    rt.run_contract(&action)?;
    let elapsed = start.elapsed();
    info!("Outer time elapsed: {elapsed:?}");

    Ok(())
}
