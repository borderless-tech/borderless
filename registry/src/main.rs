mod db;
mod error;
mod migrator;

use crate::error::Error;
use anyhow::Result;
use axum::{extract::State, routing::put, Json, Router};
use borderless_pkg::WasmPkg;
use clap::Parser;
use db::entities::package::ActivePackage;
use sea_orm::{Database, DatabaseConnection, TransactionTrait};
use std::path::PathBuf;
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the database directory (global)
    #[arg(short, long)]
    db: String,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: DatabaseConnection,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Start Registry Server!");

    let args = Cli::parse();
    let db = db::setup_database(&args.db).await?;

    let app = AppState { db };

    let app = Router::new()
        .route("/api/v0/publish", put(publish))
        .with_state(app);

    info!("Start API Service");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

// PUT publish wasm package in registry
#[instrument]
pub async fn publish(State(state): State<AppState>, Json(pkg): Json<WasmPkg>) -> Result<(), Error> {
    let txn = state.db.begin().await?;
    ActivePackage::from_model(&txn, pkg).await?;
    txn.commit().await?;
    Ok(())
}
