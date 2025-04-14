use std::num::NonZeroUsize;

use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Request, Response},
    Router,
};
use borderless_kv_store::Db;
use borderless_runtime::{
    http::{ActionWriter, ContractService, NoActionWriter, Service},
    Runtime,
};
use log::info;

/// Wraps the contract service
async fn contract_handler(
    State(mut srv): State<ContractService<impl ActionWriter, impl Db + 'static>>,
    req: Request<Body>,
) -> Response<Body> {
    let (parts, body) = req.into_parts();

    // 10MB upper limit
    let bytes = to_bytes(body, 10_000_000).await.unwrap_or_default();

    let req = Request::from_parts(parts, bytes);
    let res = srv.call(req).await.expect("infallible");
    res.map(|bytes| bytes.into())
}

pub async fn start_contract_server(db: impl Db + 'static) -> Result<()> {
    let rt = Runtime::new(&db, NonZeroUsize::new(10).unwrap())?;
    let srv = ContractService::new(db, rt, NoActionWriter)?;

    // Create a router and attach the custom service to a route
    let contract = Router::new().fallback(contract_handler).with_state(srv);

    let app = Router::new().nest("/v0/contract", contract);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
