mod db;
mod migrator;
mod error;

use crate::error::Error;
use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use borderless_hash::Hash256;
use borderless_kv_store::backend::lmdb::Lmdb;
use borderless_kv_store::{Db, RawRead, RawWrite, RoCursor, RoTx, Tx};
use borderless_pkg::WasmPkg;
use clap::Parser;
use std::path::PathBuf;

const PKG_SUB_DB: &str = "pkg-sub-db";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the database directory (global)
    #[arg(short, long)]
    db: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Start Registry Server!");

    let args = Cli::parse();
    let db = Lmdb::new(&args.db, 2).context("failed to open database")?;

    let app = Router::new()
        .route("/pkg", get(list_pkgs))
        .route("/pkg/add", post(add_pkg))
        .route("/pkg/{hash}", get(get_pkg));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

// GET /pkg
pub async fn list_pkgs() -> Result<Json<Vec<String>>, Error> {
    todo!()
}

// POST /pkg/add/
pub async fn add_pkg(Json(pkg): Json<WasmPkg>) -> Result<(), Error> {
    todo!()
}

// GET /pkg/:hash
pub async fn get_pkg(Path(name): Path<String>) -> Result<Json<WasmPkg>, Error> {
    todo!()
}
