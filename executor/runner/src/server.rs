use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Request, Response},
    routing::any,
    Router,
};
use borderless_kv_store::Db;
use borderless_runtime::http::{RtService, Service};
use log::info;

/// Wraps the contract service
async fn contract_handler(
    State(mut srv): State<RtService<impl Db + 'static>>,
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
    let srv = RtService::new(db)?;

    // Create a router and attach the custom service to a route
    let app = Router::new()
        .route("/v0/contract/{*path}", any(contract_handler))
        .with_state(srv);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
