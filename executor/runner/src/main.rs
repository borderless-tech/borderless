use std::{
    fs::read_to_string,
    ops::DerefMut,
    path::PathBuf,
    str::FromStr,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use borderless::{
    common::{Introduction, Revocation},
    contracts::TxCtx,
    events::CallAction,
    hash::Hash256,
    pkg::{SourceType, WasmPkg},
    AgentId, BlockIdentifier, ContractId, TxIdentifier,
};
use borderless_kv_store::{backend::lmdb::Lmdb, Db};
use borderless_runtime::{
    agent::{
        tasks::{handle_schedules, handle_ws_connection},
        MutLock as AgentLock, Runtime as AgentRuntime,
    },
    contract::{MutLock as ContractLock, Runtime as ContractRuntime},
    db::{
        controller::Controller,
        logger::{print_log_line, Logger},
    },
    CodeStore,
};
use clap::{Parser, Subcommand};
use reqwest::blocking::Client;

use log::info;
use server::{start_agent_server, start_contract_server};

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
    /// Execute the given action on the agent
    Process {
        /// Input file containing action data
        action: PathBuf,
    },
    /// Revoke the contract using the provided revocation data
    Revoke {
        /// Input file containing revocation data
        revocation: PathBuf,
    },

    /// Executes the agent in the background while also providing api access
    Run,

    /// Prints out all logs for this agent
    Logs,

    // TODO: Make this also a top-level command maybe ?
    /// Only provides API access but does not spin up the agent
    Api,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Initialize logging
    colog::init();

    let args = Cli::parse();
    // Setup the DB connection, etc.
    let db = Lmdb::new(&args.db, 16).context("failed to open database")?;

    match args.command {
        Commands::Contract(cmd) => contract(cmd, db).await?,
        Commands::Agent(cmd) => sw_agent(cmd, db).await?,
    }
    Ok(())
}

/// Generates a new dummy tx-ctx
pub fn generate_tx_ctx(
    mut rt: impl DerefMut<Target = ContractRuntime<impl Db>>,
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

    let lock = ContractLock::default();
    let mut rt = ContractRuntime::new(&db, code_store, lock)?;

    let cid: ContractId = if let Some(cid) = command.contract_id {
        cid
    } else {
        // Otherwise: Read from env
        "cc8ca79c-3bbb-89d2-bb28-29636c170387".parse()?
    };
    // let module_bytes = std::fs::read(command.contract)?;
    // rt.instantiate_contract(cid, &module_bytes)?;

    let writer = "bbcd81bb-b90c-8806-8341-fe95b8ede45a".parse()?;

    // The writer is also the executor
    rt.set_executor(writer)?;

    // Parse command
    match command.action {
        ContractAction::Introduce { introduction } => {
            // Parse introduction
            let data = read_to_string(introduction)?;
            let introduction = Introduction::from_str(&data)?;

            let cid = introduction.id.as_cid().unwrap();

            match &introduction.package.source.code {
                SourceType::Registry { registry } => {
                    info!("fetching from registry");
                    let client = Client::new();
                    let response = client
                        // for now write the full
                        // path in the registry hostname field
                        .get(&registry.registry_hostname)
                        .header("Content-Type", "application/json")
                        .send()?;

                    let text = response.text()?;
                    let pkg: WasmPkg = serde_json::from_str(&text)?;
                }
                SourceType::Wasm { wasm } => {
                    if !wasm.is_empty() {
                        info!("try to instantiate the contract");
                        rt.instantiate_contract(cid, &wasm)?;
                    } else {
                        info!("Introduction had empty code bytes - using filesystem instead");
                    }
                }
            }

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
            assert_eq!(revocation.id, cid);

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
            start_contract_server(db, rt.into_shared()).await?;
        }
    }
    Ok(())
}

#[allow(unused)]
async fn sw_agent(command: AgentCommand, db: Lmdb) -> Result<()> {
    // Create runtime
    let code_store = CodeStore::new(&db)?;
    let lock = AgentLock::default();
    let mut rt = AgentRuntime::new(&db, code_store, lock)?;

    let aid: AgentId = if let Some(aid) = command.agent_id {
        aid
    } else {
        // Otherwise: Read from env
        "a265e6fd-7f7a-85b5-aa24-a79305daf2a5".parse()?
    };
    // let module_bytes = std::fs::read(command.code)?;
    // rt.instantiate_sw_agent(aid, &module_bytes)?;

    let writer = "bbcd81bb-b90c-8806-8341-fe95b8ede45a".parse()?;

    // The writer is also the executor
    rt.set_executor(writer)?;

    // Parse command
    match command.action {
        AgentAction::Introduce { introduction } => {
            // Parse introduction
            let data = read_to_string(introduction)?;
            let introduction = Introduction::from_str(&data)?;

            let aid = introduction.id.as_aid().unwrap();

            match &introduction.package.source.code {
                SourceType::Registry { registry: _ } => {
                    todo!("implement fetching from registry")
                }
                SourceType::Wasm { wasm } => {
                    if !wasm.is_empty() {
                        rt.instantiate_sw_agent(aid, &wasm)?;
                    } else {
                        info!("Introduction had empty code bytes - using filesystem instead");
                    }
                }
            }

            info!("Introduce agent {aid}");
            let start = Instant::now();
            let _events = rt.process_introduction(introduction).await?;
            let elapsed = start.elapsed();
            info!("Outer time elapsed: {elapsed:?}");
        }
        AgentAction::Process { action } => {
            // Parse action
            let data = read_to_string(action)?;
            let action = CallAction::from_str(&data)?;
            rt.process_action(&aid, action).await?;

            info!("--- Agent-Log:");
            let log = Logger::new(&db, aid).get_last_log()?;
            log.into_iter().for_each(print_log_line);
        }
        AgentAction::Revoke { revocation } => todo!(),
        AgentAction::Run => {
            let init = rt.initialize(&aid).await?;
            dbg!(&init);
            let rt = rt.into_shared();

            // Spin up a dedicated task to handle the schedules
            let (tx, _rx) = tokio::sync::mpsc::channel(1);
            let handle = tokio::spawn(handle_schedules(
                rt.clone(),
                aid,
                init.schedules,
                tx.clone(),
            ));

            if let Some(ws_config) = init.ws_config {
                let _ws_handle = tokio::spawn(handle_ws_connection(rt.clone(), aid, ws_config, tx));
                // ws_handle.await;
            }

            start_agent_server(db, rt).await?;
            handle.await;
        }
        AgentAction::Logs => {
            let log = Logger::new(&db, aid).get_full_log()?;
            log.into_iter().for_each(print_log_line);
        }
        AgentAction::Api => {
            start_agent_server(db, rt.into_shared()).await?;
        }
    }
    Ok(())
}
