mod db;
mod error;
mod migrator;
mod models;

use std::future::{ready, Future};

use crate::error::Error;
use anyhow::Result;
use axum::{
    extract::{FromRequestParts, Path, State},
    http::request::Parts,
    routing::put,
    Json, Router,
};
use borderless_pkg::WasmPkg;
use clap::Parser;
use db::entities::package::ActivePackage;
use models::OciIdentifier;
use sea_orm::{DatabaseConnection, TransactionTrait};
use std::str::FromStr;
use tracing::{info, instrument};

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

    // add pkg to database
    let pkg_model = ActivePackage::from_model(&txn, pkg).await?;
    // let pkg_result = ActivePackage::insert(pkg_model, txn).await?;

    // add entry to regsistry index
    txn.commit().await?;
    Ok(())
}

// GET search a package
#[instrument]
pub async fn search(
    State(state): State<AppState>,
    OciExtractor(oci): OciExtractor,
) -> Result<(), Error> {
    todo!()
}

// GET download a pkg by hash
#[instrument]
pub async fn download(State(state): State<AppState>) -> Result<(), Error> {
    todo!()
}

#[derive(Debug, Clone)]
pub struct OciExtractor(pub OciIdentifier);

impl<S> FromRequestParts<S> for OciIdentifier
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let uri = parts.uri.path().to_string();

        if let Some(oci_part) = uri.strip_prefix("/images/") {
            let decoded = urlencoding::decode(oci_part).map_err(|_| Error::InvalidSource)?;
            let oci = OciIdentifier::from_str(&decoded).map_err(|_| Error::InvalidSource)?;

            Ok(oci)
        } else {
            Err(Error::InvalidSource)
        }
    }
}
