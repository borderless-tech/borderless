use std::{
    fs::read_to_string,
    ops::DerefMut,
    path::PathBuf,
    str::FromStr,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use borderless::{
    contracts::{Introduction, Revocation, TxCtx},
    events::CallAction,
    hash::Hash256,
    AgentId, BlockIdentifier, ContractId, TxIdentifier,
};
use borderless_kv_store::{backend::lmdb::Lmdb, Db};
use borderless_runtime::{
    controller::Controller,
    logger::{print_log_line, Logger},
    swagent::Runtime as AgentRuntime,
    CodeStore, Runtime,
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
    Agent(AgentCommand),
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

#[derive(Parser, Debug)]
struct AgentCommand {
    /// Path to the contract file (positional)
    code: PathBuf,

    /// Contract-ID of the contract
    #[arg(short, long)]
    agent_id: Option<AgentId>,

    #[command(subcommand)]
    action: AgentAction,
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

    // TODO: Make this also a top-level command maybe ?
    /// Start a webserver which exposes the contract-api
    Api,
}

#[derive(Subcommand, Debug)]
enum AgentAction {
    /// Introduce a new agent using the provided introduction
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

    /// Prints out all logs for this agent
    Logs,

    // TODO: Make this also a top-level command maybe ?
    /// Start a webserver which exposes the agent-api
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
        Commands::Agent(cmd) => sw_agent(cmd, db).await?,
    }
    Ok(())
}

/// Generates a new dummy tx-ctx
pub fn generate_tx_ctx(
    mut rt: impl DerefMut<Target = Runtime<impl Db>>,
    cid: &ContractId,
) -> Result<TxCtx> {
    // We now have to provide additional context when executing the contract
    let n_actions = rt.len_actions(cid)?;
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
    let code_store = CodeStore::new(&db)?;
    let mut rt = Runtime::new(&db, code_store)?;

    let cid: ContractId = if let Some(cid) = command.contract_id {
        cid
    } else {
        // Otherwise: Read from env
        "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse()?
    };
    rt.instantiate_contract(cid, command.contract)?;

    let writer = "bbcd81bb-b90c-8806-8341-fe95b8ede45a".parse()?;

    // The writer is also the executor
    rt.set_executor(writer)?;

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
            let events = rt.process_transaction(&cid, action, &writer, tx_ctx.clone())?;
            let elapsed = start.elapsed();
            info!("Time elapsed: {elapsed:?}");
            dbg!(events);

            // Print log
            info!("--- Contract-Log:");
            let log = Logger::new(&db, cid).get_last_log()?;
            log.into_iter().for_each(print_log_line);
        }
        ContractAction::Revoke { revocation } => {
            let data = read_to_string(revocation)?;
            let revocation = Revocation::from_str(&data)?;
            let tx_ctx = generate_tx_ctx(&mut rt, &cid)?;
            assert_eq!(revocation.contract_id, cid);

            info!("Revoke contract {cid}");
            let start = Instant::now();
            rt.process_revocation(revocation, &writer, tx_ctx.clone())?;
            let elapsed = start.elapsed();
            info!("Time elapsed: {elapsed:?}");
        }
        ContractAction::ListActions => {
            let actions = Controller::new(&db).actions(cid);
            for record in actions.iter().flatten() {
                let action = CallAction::from_bytes(&record.value)?;
                println!("{}, commited: {}", record.tx_ctx, record.commited);
                println!("{}", action.pretty_print()?);
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

#[allow(unused)]
async fn sw_agent(command: AgentCommand, db: Lmdb) -> Result<()> {
    // Create runtime
    let code_store = CodeStore::new(&db)?;
    let mut rt = AgentRuntime::new(&db, code_store)?;

    let aid: AgentId = if let Some(aid) = command.agent_id {
        aid
    } else {
        // Otherwise: Read from env
        "a265e6fd-7f7a-85b5-aa24-a79305daf2a5".parse()?
    };
    rt.instantiate_sw_agent(aid, command.code)?;

    let writer = "bbcd81bb-b90c-8806-8341-fe95b8ede45a".parse()?;

    // The writer is also the executor
    rt.set_executor(writer)?;

    // Parse command
    match command.action {
        AgentAction::Introduce { introduction } => todo!(),
        AgentAction::Process { action } => {
            // Parse action
            let data = read_to_string(action)?;
            let action = CallAction::from_str(&data)?;
            rt.process_action(&aid, action)?;

            info!("--- Agent-Log:");
            let log = Logger::new(&db, aid).get_last_log()?;
            log.into_iter().for_each(print_log_line);
        }
        AgentAction::Revoke { revocation } => todo!(),
        AgentAction::Logs => todo!(),
        AgentAction::Api => todo!(),
    }
    Ok(())
}
