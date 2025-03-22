use std::path::PathBuf;

use anyhow::Result;
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_runtime::Runtime;
use borderless_sdk::ContractId;
use clap::Parser;

use log::{debug, error, info, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the database directory
    #[arg(short, long)]
    db: PathBuf,

    /// Path to the Wasm contract
    #[arg(short, long)]
    contract: PathBuf,
}

fn main() -> Result<()> {
    colog::init();
    let args = Cli::parse();
    info!("📁 Database path: {}", args.db.display());
    info!("📦 Contract path: {}", args.contract.display());

    let db = Lmdb::new(&args.db, 2)?;

    let mut rt = Runtime::new(db)?;

    let cid = ContractId::generate();
    info!("Using contract-id: {cid}");

    rt.instantiate_contract(cid, args.contract)?;

    rt.run_contract()?;

    Ok(())
}
