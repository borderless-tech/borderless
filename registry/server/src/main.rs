mod error;
mod registry;

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::{
        header::{CONTENT_LENGTH, CONTENT_TYPE},
        HeaderMap, HeaderValue,
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use borderless_format::{
    pkg::{InsertPkg, Pkg},
    registry::ContractService,
};
use borderless_hash::Hash256;
use clap::Parser;
use std::path::PathBuf;

use borderless_kv_store::backend::lmdb::Lmdb;

use crate::error::Error;
use crate::registry::ContractRegistry;

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

    let app_state = App {
        registry: ContractRegistry::new(db)?,
    };

    let app = Router::new()
        .route("/registry/contract/{hash}", get(download_contract))
        .route("/registry/pkg", get(list_pkg))
        .route("/registry/pkg/create", post(create_pkg))
        .route("/registry/pkg/{pkg_name}", get(read_pkg))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

#[derive(Clone)]
pub struct App {
    pub registry: ContractRegistry<Lmdb>,
}

// GET /registry/contract/:hash
pub async fn download_contract(
    Path(hash): Path<Hash256>,
    State(app): State<App>,
) -> Result<Response, Error> {
    let buf = app.registry.get_contract(hash).await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );

    headers.insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&buf.len().to_string()).unwrap(),
    );

    Ok((headers, buf).into_response())
}

// GET  /registry/pkg
pub async fn list_pkg(State(app): State<App>) -> Result<Json<Vec<String>>, Error> {
    let pkgs = app.registry.list_pkg().await?;
    Ok(Json(pkgs))
}

// POST /registry/pkg/create
pub async fn create_pkg(State(app): State<App>, Json(pkg): Json<InsertPkg>) -> Result<(), Error> {
    app.registry.create_pkg(pkg).await?;
    Ok(())
}

// GET  /registry/pkg/:name
pub async fn read_pkg(
    Path(pkg_name): Path<String>,
    State(app): State<App>,
) -> Result<Json<Pkg>, Error> {
    let pkg = app.registry.read_pkg(pkg_name).await?;
    Ok(Json(pkg))
}
