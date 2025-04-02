use std::{fs::read_to_string, path::PathBuf, str::FromStr, time::Instant};

use anyhow::Result;
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_runtime::Runtime;
use borderless_sdk::{contract::CallAction, ContractId};
use clap::{Parser, Subcommand};

use log::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the database directory (global)
    #[arg(short, long)]
    db: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Contract related commands
    Contract(ContractCommand),
}

#[derive(Parser, Debug)]
struct ContractCommand {
    /// Path to the contract file (positional)
    contract: PathBuf,

    /// Contract-ID of the contract
    #[arg(short, long)]
    contract_id: Option<ContractId>,

    #[command(subcommand)]
    action: ContractAction,
}

#[derive(Subcommand, Debug)]
enum ContractAction {
    /// Introduce a new contract using the provided introduction
    Introduce {
        /// Input file containing introduction data
        introduction: PathBuf,
    },
    /// Execute the given action on the contract
    Process {
        /// Input file containing action data
        action: PathBuf,
    },
    /// Revoke the contract using the provided revocation data
    Revoke {
        /// Input file containing revocation data
        revocation: PathBuf,
    },
}

fn main() -> Result<()> {
    // Initialize logging
    colog::init();

    let args = Cli::parse();
    info!("📁 Database path: {}", args.db.display());

    // Setup the DB connection, etc.
    let db = Lmdb::new(&args.db, 2)?;

    match args.command {
        Commands::Contract(cmd) => contract(cmd, db)?,
    }
    Ok(())
}

fn contract(command: ContractCommand, db: Lmdb) -> Result<()> {
    // Create runtime
    let mut rt = Runtime::new(&db)?;

    let cid: ContractId = if let Some(cid) = command.contract_id {
        cid
    } else {
        // Otherwise: Read from env
        "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse()?
    };
    info!("Using contract-id: {cid}");

    info!("Instantiate contract {cid}");
    rt.instantiate_contract(cid, command.contract)?;

    // Parse command
    match command.action {
        ContractAction::Introduce { introduction } => todo!(),
        ContractAction::Process { action } => {
            // Parse action
            let data = read_to_string(action)?;
            let action = CallAction::from_str(&data)?;

            info!("Run contract {cid}");
            let start = Instant::now();
            rt.process_transaction(&action)?;
            let elapsed = start.elapsed();
            info!("Outer time elapsed: {elapsed:?}");
        }
        ContractAction::Revoke { revocation } => todo!(),
    }
    Ok(())
}
