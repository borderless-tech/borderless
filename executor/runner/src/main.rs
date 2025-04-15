use std::{
    fs::read_to_string,
    num::NonZeroUsize,
    ops::DerefMut,
    path::PathBuf,
    str::FromStr,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use borderless_kv_store::{backend::lmdb::Lmdb, Db};
use borderless_runtime::{
    logger::{print_log_line, Logger},
    Runtime,
};
use borderless_sdk::{
    contract::{CallAction, Introduction, TxCtx},
    hash::Hash256,
    BlockIdentifier, ContractId, TxIdentifier,
};
use clap::{Parser, Subcommand};

use log::info;
use server::start_contract_server;

mod server;

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
    /// Lists all actions that were executed by this contract
    ListActions,

    /// Prints out all logs for this contract
    Logs,

    /// Start a webserver which exposes the contract-api
    Api,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Initialize logging
    colog::init();

    let args = Cli::parse();
    // Setup the DB connection, etc.
    let db = Lmdb::new(&args.db, 2).context("failed to open database")?;

    match args.command {
        Commands::Contract(cmd) => contract(cmd, db).await?,
    }
    Ok(())
}

/// Generates a new dummy tx-ctx
pub fn generate_tx_ctx(
    mut rt: impl DerefMut<Target = Runtime<impl Db>>,
    cid: &ContractId,
) -> Result<TxCtx> {
    // We now have to provide additional context when executing the contract
    let n_actions = rt.len_actions(cid)?.unwrap_or_default();
    let tx_hash = Hash256::digest(&n_actions.to_be_bytes());
    let tx_ctx = TxCtx {
        tx_id: TxIdentifier::new(0, n_actions, tx_hash),
        index: 0,
    };
    // Set block
    (*rt).set_block(
        BlockIdentifier::new(0, n_actions, Hash256::empty()),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )?;
    Ok(tx_ctx)
}

async fn contract(command: ContractCommand, db: Lmdb) -> Result<()> {
    // Create runtime
    let mut rt = Runtime::new(&db, NonZeroUsize::new(10).unwrap())?;

    let cid: ContractId = if let Some(cid) = command.contract_id {
        cid
    } else {
        // Otherwise: Read from env
        "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse()?
    };
    rt.instantiate_contract(cid, command.contract)?;

    let writer = "bbcd81bb-b90c-8806-8341-fe95b8ede45a".parse()?;

    // Parse command
    match command.action {
        ContractAction::Introduce { introduction } => {
            // Parse introduction
            let data = read_to_string(introduction)?;
            let introduction = Introduction::from_str(&data)?;

            let cid = introduction.contract_id;
            let tx_ctx = generate_tx_ctx(&mut rt, &cid)?;
            info!("Introduce contract {cid}");
            let start = Instant::now();
            rt.process_introduction(introduction, &writer, tx_ctx)?;
            let elapsed = start.elapsed();
            info!("Outer time elapsed: {elapsed:?}");
            info!("--- Contract-Log:");
            let log = Logger::new(&db, cid).get_last_log()?;
            log.into_iter().for_each(print_log_line);
        }
        ContractAction::Process { action } => {
            // Parse action
            let data = read_to_string(action)?;
            let action = CallAction::from_str(&data)?;
            let tx_ctx = generate_tx_ctx(&mut rt, &cid)?;

            info!("Run contract {cid}");
            let start = Instant::now();
            rt.process_transaction(&cid, action, &writer, tx_ctx.clone())?;
            let elapsed = start.elapsed();
            info!("Time elapsed: {elapsed:?}");

            // Print log
            info!("--- Contract-Log:");
            let log = Logger::new(&db, cid).get_last_log()?;
            log.into_iter().for_each(print_log_line);
        }
        ContractAction::Revoke { revocation } => todo!(),
        ContractAction::ListActions => {
            let mut idx = 0;
            while let Some(record) = rt.read_action(&cid, idx)? {
                let action = CallAction::from_bytes(&record.value)?;
                println!("{}, commited: {}", record.tx_ctx, record.commited);
                println!("{}", action.pretty_print()?);
                idx += 1;
            }
        }
        ContractAction::Logs => {
            let log = Logger::new(&db, cid).get_full_log()?;
            log.into_iter().for_each(print_log_line);
        }
        ContractAction::Api => {
            start_contract_server(db).await?;
            // let mut buf = String::new();
            // std::io::stdin().read_line(&mut buf)?;
            // let input = buf.trim().to_lowercase();
            // if input.is_empty() {
            //     break;
            // }
            // if input.starts_with('/') {
            //     let now = Instant::now();
            //     // TODO: Query
            //     let rs = rt.http_get_state(&cid, input)?;
            //     let elapsed = now.elapsed();
            //     let value = String::from_utf8(rs.payload)?;
            //     info!("{}: {}, time elapsed: {elapsed:?}", rs.status, value);
            //     continue;
            // }
        }
    }
    Ok(())
}
