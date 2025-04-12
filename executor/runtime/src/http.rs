use anyhow::Result;
use borderless_kv_store::{backend::lmdb::Lmdb, Db};
use borderless_sdk::{contract::CallAction, ContractId};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use log::info;
use mime::{APPLICATION_JSON, TEXT_PLAIN_UTF_8};
use parking_lot::Mutex;
use serde::Serialize;
use std::{
    convert::Infallible,
    task::{Context, Poll},
};
use std::{
    future::{ready, Ready},
    sync::Arc,
};
pub use tower::Service;

use crate::{logger::Logger, Runtime};

pub type Request<T = Bytes> = http::Request<T>;
pub type Response<T = Bytes> = http::Response<T>;

fn reject_404() -> Response {
    let mut resp = Response::new(Bytes::new());
    *resp.status_mut() = StatusCode::NOT_FOUND;
    resp
}

fn json_response<S: Serialize>(value: &S) -> Response<Bytes> {
    let bytes = serde_json::to_vec(value).unwrap();
    json_body(bytes)
}

fn json_body(bytes: Vec<u8>) -> Response<Bytes> {
    let mut resp = Response::new(bytes.into());
    resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_JSON.as_ref()),
    );
    resp
}

/// Converts any error-type into a http-response with status-code 500
///
/// The response has content-type set to `text/plain; charset=utf-8`
/// and the body contains the error message as string.
pub fn into_server_error<E: ToString>(error: E) -> Response {
    let mut resp = Response::new(error.to_string().into_bytes().into());
    resp.headers_mut().append(
        CONTENT_TYPE,
        HeaderValue::from_static(TEXT_PLAIN_UTF_8.as_ref()),
    );
    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    resp
}

/// Simple service around the runtime
#[derive(Clone)]
pub struct RtService<S = Lmdb>
where
    S: Db,
{
    rt: Arc<Mutex<Runtime<S>>>,
    db: S,
}

impl<S: Db> RtService<S> {
    pub fn new(db: S) -> anyhow::Result<Self> {
        let rt = Runtime::new(&db)?;
        Ok(Self {
            rt: Arc::new(Mutex::new(rt)),
            db,
        })
    }

    fn process_rq(&self, req: Request) -> anyhow::Result<Response> {
        let path = req.uri().path();

        info!("{path}");

        if path == "/" {
            let contracts = self.rt.lock().available_contracts();
            return Ok(json_response(&contracts));
        }

        let mut pieces = path.split('/').skip(1);

        // Extract contract-id from first piece
        let contract_id: ContractId =
            if let Some(contract_id) = pieces.next().and_then(|first| first.parse().ok()) {
                contract_id
            } else {
                return Ok(reject_404());
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

            let mut rt = self.rt.lock();
            match route {
                "state" => {
                    let (status, payload) = rt.http_get_state(&contract_id, trunc)?;
                    if status == 200 {
                        return Ok(json_body(payload));
                    } else {
                        return Ok(reject_404());
                    }
                }
                "logs" => {
                    let logger = Logger::new(&self.db, contract_id);

                    // TODO: Pagination and return type
                    let log = logger.get_full_log().unwrap();
                    return Ok(json_response(&log));
                }
                "txs" => {
                    let mut idx = 0;
                    // TODO: Generate a real output type for this
                    // TODO: Pagination
                    let mut out = Vec::new();
                    while let Some(record) = rt.read_action(&contract_id, idx)? {
                        let action = CallAction::from_bytes(&record.value)?;
                        out.push(action);
                        idx += 1;
                    }
                    return Ok(json_response(&out));
                }
                _ => return Ok(reject_404()),
            }
        }

        Ok(reject_404())
    }
}

// TODO: Polish this, and put this into the http module of the runtime
//
// We can simply use this as the service in the contract node aswell, so we don't have to duplicate logic
impl<S: Db> Service<Request> for RtService<S> {
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let result: Response = match self.process_rq(req) {
            Ok(r) => r,
            Err(e) => into_server_error(e),
        };
        ready(Ok(result))
    }
}
