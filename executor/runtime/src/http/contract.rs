use borderless::events::CallAction;
use borderless::hash::Hash256;
use borderless::http::queries::Pagination;
use borderless::BorderlessId;
use borderless::ContractId;
use borderless_kv_store::{backend::lmdb::Lmdb, Db};
use http::method::Method;
use parking_lot::Mutex;
use std::convert::Infallible;
use std::future::Future;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};

pub use super::*;
use crate::log_shim::*;
use crate::{db::controller::Controller, rt::contract::Runtime};

pub trait ActionWriter: Clone + Send + Sync {
    type Error: std::fmt::Display + Send + Sync;

    fn write_action(
        &self,
        cid: ContractId,
        action: CallAction,
    ) -> impl Future<Output = Result<Hash256, Self::Error>> + Send;
}

/// A dummy implementation of an action-writer, that does nothing with the action.
///
/// Useful for testing.
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
    // TODO: This is not optimal. The runtime is not tied to a tx-writer,
    // and for our multi-tenant contract-node we require this to be flexible.
    writer: BorderlessId,
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
        let start = Instant::now();
        let path = req.uri().path().to_string();
        let result = match *req.method() {
            Method::GET => self.process_get_rq(req),
            Method::POST => self.process_post_rq(req).await,
            _ => Ok(method_not_allowed()),
        };
        let elapsed = start.elapsed();
        match &result {
            Ok(res) => info!(
                "Request success. path={path}. Time elapsed: {elapsed:?}, status={}",
                res.status()
            ),
            Err(e) => warn!("Request failed. path={path}. Time elapsed: {elapsed:?}, error={e}"),
        }
        result
    }

    fn process_get_rq(&self, req: Request) -> crate::Result<Response> {
        let path = req.uri().path();
        let query = req.uri().query();

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
        let controller = Controller::new(&self.db);

        // Ensure, that the contract exists
        if !controller.contract_exists(&contract_id)? {
            return Ok(reject_404());
        }

        // Get top-level route
        let route = match pieces.next() {
            Some(r) => r,
            None => {
                // Get full contract info
                let full_info = controller.contract_full(&contract_id)?;
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
        match route {
            "state" => {
                // TODO: The contract should also parse query parameters !
                // TODO: URL-Decode !
                let mut rt = self.rt.lock();
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
                let log = controller
                    .logs(contract_id)
                    .get_logs_paginated(pagination)?;

                Ok(json_response(&log))
            }
            "txs" => {
                // Extract pagination
                let pagination = Pagination::from_query(query).unwrap_or_default();

                // Get actions
                let paginated = controller
                    .actions(contract_id)
                    .get_tx_action_paginated(pagination)?;

                Ok(json_response(&paginated))
            }
            "info" => {
                let info = controller.contract_info(&contract_id)?;
                Ok(json_response_nested(info, &trunc))
            }
            "desc" => {
                let desc = controller.contract_desc(&contract_id)?;
                Ok(json_response_nested(desc, &trunc))
            }
            "meta" => {
                let meta = controller.contract_meta(&contract_id)?;
                Ok(json_response_nested(meta, &trunc))
            }
            "symbols" => {
                let mut rt = self.rt.lock();
                let symbols = rt.get_symbols(&contract_id)?;
                Ok(json_response(&symbols))
            }
            "pkg" => match trunc.as_str() {
                "/" => {
                    let result = controller
                        .contract_pkg_full(&contract_id)?
                        .map(|r| r.into_dto());
                    Ok(json_response(&result))
                }
                "/def" => {
                    let result = controller
                        .contract_pkg_def(&contract_id)?
                        .map(|r| r.into_dto());
                    Ok(json_response(&result))
                }
                "/source" => {
                    let result = controller.contract_pkg_source(&contract_id)?;
                    Ok(json_response(&result))
                }
                _ => Ok(reject_404()),
            },
            // Same as empty path
            "" => {
                // TODO: Maybe we also add the package definition to this
                let full_info = controller.contract_full(&contract_id)?;
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
        let cid_str = match pieces.next() {
            Some(s) => s,
            None => return Ok(method_not_allowed()),
        };
        let contract_id: ContractId = match cid_str.parse() {
            Ok(cid) => cid,
            Err(e) => return Ok(bad_request(format!("failed to parse contract-id - {e}"))),
        };

        // Get top-level route
        let route = match pieces.next() {
            Some(r) => r,
            None => return Ok(reject_404()),
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
                    rt.set_executor(self.writer)?; // NOTE: In this case writer and executor are identical
                    match rt.http_post_action(&contract_id, trunc, payload.into(), &self.writer)? {
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
        let this = self.clone();
        let fut = async move {
            let result: Response = match this.process_rq(req).await {
                Ok(r) => r,
                Err(e) => into_server_error(e),
            };
            Ok(result)
        };
        Box::pin(fut)
    }
}
