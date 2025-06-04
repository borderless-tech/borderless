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

#[derive(Clone)]
pub struct App<D: Db = Lmdb> {
    pub db: D,
}

impl<D: Db> App<D> {
    pub fn new(db: D) -> Result<App<D>> {
        db.create_sub_db(PKG_SUB_DB)?;
        Ok(App { db })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Start Registry Server!");

    let args = Cli::parse();
    let db = Lmdb::new(&args.db, 2).context("failed to open database")?;
    let app_state = App::new(db)?;

    let app = Router::new()
        .route("/pkg", get(list_pkgs))
        .route("/pkg/add", post(add_pkg))
        .route("/pkg/{hash}", get(get_pkg))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

// GET /pkg
pub async fn list_pkgs(State(app): State<App>) -> Result<Json<Vec<String>>, Error> {
    let db_ptr = app.db.open_sub_db(PKG_SUB_DB)?;
    let txn = app.db.begin_ro_txn()?;
    let mut cursor = txn.ro_cursor(&db_ptr)?;
    let mut pkgs = Vec::new();

    for (_, buf) in cursor.iter() {
        let pkg: WasmPkg = bincode::deserialize(&buf)?;
        let identifier = [
            pkg.app_name.as_deref(),
            pkg.app_module.as_deref(),
            Some(pkg.name.as_str()),
        ]
        .iter()
        .filter_map(|&x| x)
        .collect::<Vec<_>>()
        .join("/");

        pkgs.push(identifier);
    }

    Ok(Json(pkgs))
}

// POST /pkg/add/
pub async fn add_pkg(State(app): State<App>, Json(pkg): Json<WasmPkg>) -> Result<(), Error> {
    let identifier = [
        pkg.app_name.as_deref(),
        pkg.app_module.as_deref(),
        Some(pkg.name.as_str()),
    ]
    .iter()
    .filter_map(|&x| x)
    .collect::<Vec<_>>()
    .join("/");

    let key = Hash256::digest(&identifier);

    let db_ptr = app.db.open_sub_db(PKG_SUB_DB)?;

    let txn = app.db.begin_rw_txn()?;
    let buf = txn.read(&db_ptr, &key)?;

    if buf.is_some() {
        return Err(Error::Dublicated(key));
    }

    let mut txn = app.db.begin_rw_txn()?;

    let buf = bincode::serialize(&pkg)?;
    txn.write(&db_ptr, &key, &buf)?;
    txn.commit()?;

    Ok(())
}

// GET /pkg/:hash
pub async fn get_pkg(
    Path(name): Path<String>,
    State(app): State<App>,
) -> Result<Json<WasmPkg>, Error> {
    let db_ptr = app.db.open_sub_db(PKG_SUB_DB)?;
    let txn = app.db.begin_ro_txn()?;

    let key = Hash256::digest(&name);
    let buf = txn.read(&db_ptr, &key)?.ok_or(Error::NoPkg(key))?;
    let pkg = bincode::deserialize(&buf)?;

    Ok(Json(pkg))
}
