use anyhow::Result;
use axum::{
    body::{self, Body},
    http::{Request, Response},
    Router,
};
use borderless_runtime::{logger::Logger, Runtime};
use borderless_sdk::{contract::CallAction, ContractId, Uuid};
use http::StatusCode;
use log::info;
use serde::Serialize;
use std::{
    convert::Infallible,
    sync::Mutex,
    task::{Context, Poll},
};
use std::{
    future::{ready, Ready},
    sync::Arc,
};
use tower::Service;

/// Simple service around the runtime
#[derive(Clone)]
struct RtService {
    rt: Arc<Mutex<Runtime>>,
}

fn reject_404() -> Response<Body> {
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = StatusCode::NOT_FOUND;
    resp
}

fn json_response<S: Serialize>(value: &S) -> Response<Body> {
    let bytes = serde_json::to_vec(value).unwrap();
    json_body(bytes)
}

fn json_body(bytes: Vec<u8>) -> Response<Body> {
    let body: body::Bytes = bytes.into();
    let mut resp = Response::new(body.into());
    resp.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::header::HeaderValue::from_static("application/json"),
    );
    resp
}

// TODO: Polish this, and put this into the http module of the runtime
//
// We can simply use this as the service in the contract node aswell, so we don't have to duplicate logic
impl Service<Request<Body>> for RtService {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let path = req.uri().path();

        info!("{path}");

        // TODO: Wrap the inner function, so that we don't have to always return "ready(Ok(...))"
        if path == "/" {
            let contracts = self.rt.lock().unwrap().available_contracts();
            return ready(Ok(json_response(&contracts)));
        }

        let mut pieces = path.split('/').skip(1);

        // Extract contract-id from first piece
        let contract_id: ContractId =
            if let Some(contract_id) = pieces.next().and_then(|first| first.parse().ok()) {
                contract_id
            } else {
                return ready(Ok(reject_404()));
            };

        if let Some(route) = pieces.next() {
            // Build truncated path
            let mut trunc = String::new();
            for piece in pieces {
                trunc.push('/');
                trunc.push_str(piece);
            }
            if trunc.is_empty() {
                trunc.push('/');
            }
            if let Some(query) = req.uri().query() {
                trunc.push('?');
                trunc.push_str(query);
            }

            let mut rt = self.rt.lock().unwrap();
            match route {
                "state" => {
                    let (status, payload) = rt.http_get_state(&contract_id, trunc).unwrap();
                    if status == 200 {
                        return ready(Ok(json_body(payload)));
                    } else {
                        return ready(Ok(reject_404()));
                    }
                }
                "logs" => {
                    let db = rt.get_db();
                    drop(rt);
                    let logger = Logger::new(&db, contract_id);

                    // TODO: Pagination and return type
                    let log = logger.get_full_log().unwrap();
                    return ready(Ok(json_response(&log)));
                }
                "txs" => {
                    let mut idx = 0;
                    // TODO: Generate a real output type for this
                    // TODO: Pagination
                    let mut out = Vec::new();
                    while let Some(record) = rt.read_action(&contract_id, idx).unwrap() {
                        let action = CallAction::from_bytes(&record.value).unwrap();
                        out.push(action);
                        idx += 1;
                    }
                    return ready(Ok(json_response(&out)));
                }
                _ => return ready(Ok(reject_404())),
            }
        }

        ready(Ok(reject_404()))
    }
}

pub async fn start_contract_server(rt: Runtime) -> Result<()> {
    let srv = RtService {
        rt: Arc::new(Mutex::new(rt)),
    };

    // Create a router and attach the custom service to a route
    let app = Router::new().nest_service("/v0/contract", srv);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
