use borderless_kv_store::{backend::lmdb::Lmdb, Db};
use borderless_sdk::contract::CallAction;
use borderless_sdk::hash::Hash256;
use borderless_sdk::BorderlessId;
use borderless_sdk::{http::queries::Pagination, ContractId};
use bytes::Bytes;
use http::method::Method;
use http::request::Parts;
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use log::info;
use mime::{APPLICATION_JSON, TEXT_PLAIN_UTF_8};
use parking_lot::Mutex;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::{
    convert::Infallible,
    task::{Context, Poll},
    time::Instant,
};
pub use tower::Service;

use crate::{controller::Controller, Runtime};

pub type Request<T = Bytes> = http::Request<T>;
pub type Response<T = Bytes> = http::Response<T>;

fn reject_404() -> Response {
    let mut resp = Response::new(Bytes::new());
    *resp.status_mut() = StatusCode::NOT_FOUND;
    resp
}

fn method_not_allowed() -> Response {
    let mut resp = Response::new(Bytes::new());
    *resp.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
    resp
}

fn unsupported_media_type() -> Response {
    let mut resp = Response::new(Bytes::new());
    *resp.status_mut() = StatusCode::UNSUPPORTED_MEDIA_TYPE;
    resp
}

// fn bad_request(err: String) -> Response {
//     let mut resp = Response::new(err.into_bytes().into());
//     *resp.status_mut() = StatusCode::BAD_REQUEST;
//     resp
// }

fn err_response(status: StatusCode, err_msg: String) -> Response {
    let mut resp = Response::new(err_msg.into_bytes().into());
    *resp.status_mut() = status;
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

fn check_json_content(parts: &Parts) -> bool {
    if let Some(content_type) = parts.headers.get(CONTENT_TYPE) {
        content_type.as_bytes() == APPLICATION_JSON.as_ref().as_bytes()
    } else {
        false
    }
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

pub trait ActionWriter: Clone + Send + Sync {
    type Error: std::error::Error + Send + Sync;

    fn write_action(
        &self,
        cid: ContractId,
        action: CallAction,
    ) -> impl Future<Output = Result<Hash256, Self::Error>> + Send;
}

/// A dummy implementation of an action-writer, that does nothing with the action.
#[derive(Clone)]
pub struct NoActionWriter;

impl ActionWriter for NoActionWriter {
    type Error = Infallible;

    async fn write_action(
        &self,
        _cid: ContractId,
        _action: CallAction,
    ) -> Result<Hash256, Self::Error> {
        Ok(Hash256::zero())
    }
}

#[derive(Serialize)]
pub struct ActionResp {
    pub success: bool,
    pub action: CallAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<Hash256>,
}

/// Simple service around the runtime
#[derive(Clone)]
pub struct ContractService<A, S = Lmdb>
where
    A: ActionWriter + 'static,
    S: Db + 'static,
{
    rt: Arc<Mutex<Runtime<S>>>,
    db: S,
    writer: BorderlessId,
    // TODO: This is not optimal. The runtime is not tied to a tx-writer,
    // and for our multi-tenant contract-node we require this to be flexible.
    action_writer: A,
}

impl<A, S> ContractService<A, S>
where
    A: ActionWriter + 'static,
    S: Db + 'static,
{
    pub fn new(db: S, rt: Runtime<S>, action_writer: A, writer: BorderlessId) -> Self {
        Self {
            rt: Arc::new(Mutex::new(rt)),
            db,
            writer,
            action_writer,
        }
    }

    pub fn with_shared(
        db: S,
        rt: Arc<Mutex<Runtime<S>>>,
        action_writer: A,
        writer: BorderlessId,
    ) -> Self {
        Self {
            rt,
            db,
            writer,
            action_writer,
        }
    }

    async fn process_rq(&self, req: Request) -> crate::Result<Response> {
        match *req.method() {
            Method::GET => self.process_get_rq(req),
            Method::POST => self.process_post_rq(req).await,
            _ => Ok(method_not_allowed()),
        }
    }

    fn process_get_rq(&self, req: Request) -> crate::Result<Response> {
        let path = req.uri().path();
        let query = req.uri().query();

        info!("{path}");

        if path == "/" {
            let contracts = self.rt.lock().available_contracts()?;
            return Ok(json_response(&contracts));
        }

        let mut pieces = path.split('/').skip(1);

        // Extract contract-id from first piece
        let contract_id: ContractId = match pieces.next().and_then(|first| first.parse().ok()) {
            Some(cid) => cid,
            None => return Ok(reject_404()),
        };

        // Get top-level route
        let route = match pieces.next() {
            Some(r) => r,
            None => {
                // Get full contract info
                let full_info = Controller::new(&self.db).contract_full(&contract_id)?;
                return Ok(json_response(&full_info));
            }
        };

        // Build truncated path
        let mut trunc = String::new();
        for piece in pieces {
            trunc.push('/');
            trunc.push_str(piece);
        }
        if trunc.is_empty() {
            trunc.push('/');
        }
        if let Some(query) = query {
            trunc.push('?');
            trunc.push_str(query);
        }

        let mut rt = self.rt.lock();
        match route {
            "state" => {
                // TODO: The contract should also parse query parameters !
                let (status, payload) = rt.http_get_state(&contract_id, trunc)?;
                if status == 200 {
                    Ok(json_body(payload))
                } else {
                    Ok(reject_404())
                }
            }
            "logs" => {
                // Extract pagination
                let pagination = Pagination::from_query(query).unwrap_or_default();

                // Get logs
                let log = Controller::new(&self.db)
                    .logs(contract_id)
                    .get_logs_paginated(pagination)?;

                Ok(json_response(&log))
            }
            "txs" => {
                // Extract pagination
                let pagination = Pagination::from_query(query).unwrap_or_default();

                // Get actions
                let paginated = Controller::new(&self.db)
                    .actions(contract_id)
                    .get_tx_action_paginated(pagination)?;

                Ok(json_response(&paginated))
            }
            "info" => {
                let info = Controller::new(&self.db).contract_info(&contract_id)?;
                Ok(json_response(&info))
            }
            "desc" => {
                let desc = Controller::new(&self.db).contract_desc(&contract_id)?;
                Ok(json_response(&desc))
            }
            "meta" => {
                let meta = Controller::new(&self.db).contract_meta(&contract_id)?;
                Ok(json_response(&meta))
            }
            // Same as empty path
            "" => {
                let full_info = Controller::new(&self.db).contract_full(&contract_id)?;
                Ok(json_response(&full_info))
            }
            _ => Ok(reject_404()),
        }
    }

    async fn process_post_rq(&self, req: Request) -> crate::Result<Response> {
        let path = req.uri().path();

        if path == "/" {
            return Ok(method_not_allowed());
        }

        let mut pieces = path.split('/').skip(1);

        // Extract contract-id from first piece
        let contract_id: ContractId = match pieces.next().and_then(|first| first.parse().ok()) {
            Some(cid) => cid,
            None => return Ok(method_not_allowed()),
        };

        // Get top-level route
        let route = match pieces.next() {
            Some(r) => r,
            None => return Ok(method_not_allowed()),
        };

        // Build truncated path
        let mut trunc = String::new();
        let mut cnt = 0;
        for piece in pieces {
            trunc.push('/');
            trunc.push_str(piece);
            cnt += 1;
        }
        // NOTE: The action route only has one additional path parameter
        if cnt > 1 {
            return Ok(reject_404());
        }
        if trunc.is_empty() {
            trunc.push('/');
        }
        if let Some(query) = req.uri().query() {
            trunc.push('?');
            trunc.push_str(query);
        }
        match route {
            "action" => {
                // Check request header
                let (parts, payload) = req.into_parts();
                if !check_json_content(&parts) {
                    return Ok(unsupported_media_type());
                }

                let action = {
                    let mut rt = self.rt.lock();
                    match rt.http_post_action(&contract_id, trunc, payload.into())? {
                        Ok(action) => {
                            // Perform dry-run of action ( and return action resp in case of error )
                            if let Err(e) = rt.perform_dry_run(&contract_id, &action, &self.writer)
                            {
                                let resp = ActionResp {
                                    success: false,
                                    action,
                                    error: Some(e.to_string()),
                                    tx_hash: None,
                                };
                                return Ok(json_response(&resp));
                            }

                            action
                        }
                        Err((status, err)) => {
                            return Ok(err_response(status.try_into().unwrap(), err))
                        }
                    }
                };
                let tx_hash = self
                    .action_writer
                    .write_action(contract_id, action.clone())
                    .await
                    .map_err(|e| crate::Error::msg(format!("failed to write action: {e}")))?;

                // Build action response
                let resp = ActionResp {
                    success: true,
                    error: None,
                    action,
                    tx_hash: Some(tx_hash),
                };
                Ok(json_response(&resp))
            }
            "" => Ok(method_not_allowed()),
            _ => Ok(reject_404()),
        }
    }
}

impl<A, S> Service<Request> for ContractService<A, S>
where
    A: ActionWriter + 'static,
    S: Db + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let start = Instant::now();
        let this = self.clone();
        let fut = async move {
            let result: Response = match this.process_rq(req).await {
                Ok(r) => r,
                Err(e) => into_server_error(e),
            };
            Ok(result)
        };
        let elapsed = start.elapsed();
        info!("Time elapsed: {elapsed:?}");
        Box::pin(fut)
    }
}
