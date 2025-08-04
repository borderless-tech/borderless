pub use super::*;
use crate::db::controller::Controller;
use crate::log_shim::*;
use borderless::http::queries::Pagination;
use borderless::{BorderlessId, ContractId};
use borderless_kv_store::{backend::lmdb::Lmdb, Db};
use http::method::Method;
use serde::Deserialize;
use std::convert::Infallible;
use std::future::Future;
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

/// Simple service around the runtime
#[derive(Clone)]
pub struct LedgerService<S = Lmdb>
where
    S: Db + 'static,
{
    db: S,
}

impl<S> LedgerService<S>
where
    S: Db + 'static,
{
    pub fn new(db: S) -> Self {
        Self { db }
    }

    async fn process_rq(&self, req: Request) -> crate::Result<Response> {
        let start = Instant::now();
        let path = req.uri().path().to_string();
        let result = match *req.method() {
            Method::GET => self.process_get_rq(req).await,
            Method::POST => self.process_post_rq(req).await,
            _ => Ok(method_not_allowed()),
        };
        let elapsed = start.elapsed();
        // TODO: I don't know if this should be logged every time
        match &result {
            Ok(res) => info!(
                "Request success. path={path}. Time elapsed: {elapsed:?}, status={}",
                res.status()
            ),
            Err(e) => warn!("Request failed. path={path}. Time elapsed: {elapsed:?}, error={e}"),
        }
        result
    }

    async fn process_get_rq(&self, req: Request) -> crate::Result<Response> {
        let path = req.uri().path();

        // TODO: Add pagination to ledger routes
        let query = req.uri().query();

        let controller = Controller::new(&self.db);
        if path == "/" {
            let ids = controller.ledger().all_ids()?;
            return Ok(json_response(&ids));
        }
        let mut pieces = path.split('/').skip(1);

        // Get top-level route
        let route = match pieces.next() {
            Some(r) => r,
            None => return Ok(reject_404()),
        };

        match route {
            "" => {
                let ids = controller.ledger().all_ids()?;
                Ok(json_response(&ids))
            }
            "balances" => {
                let balances = controller.ledger().all_balances()?;
                Ok(json_response(&balances))
            }
            "meta" => {
                let meta: Vec<_> = controller
                    .ledger()
                    .all()?
                    .into_iter()
                    .map(|m| m.into_dto())
                    .collect();
                Ok(json_response(&meta))
            }
            other => {
                // Try parse the ledger-id and then match against the next piece
                let ledger_id = match other.parse::<u64>() {
                    Ok(id) => id,
                    Err(e) => return Ok(bad_request(e.to_string())),
                };
                let ledger = controller.ledger().select(ledger_id);
                let route = pieces.next().unwrap_or_default();
                match route {
                    "" => {
                        let pagination = Pagination::from_query(query).unwrap_or_default();
                        let entries = ledger.get_entries_paginated(pagination)?;
                        Ok(json_response(&entries))
                    }
                    "balances" => Ok(json_response(&ledger.balances()?)),
                    "meta" => Ok(json_response(&ledger.meta()?)),
                    _ => Ok(reject_404()),
                }
            }
        }
    }

    async fn process_post_rq(&self, req: Request) -> crate::Result<Response> {
        // Check request header
        let (parts, payload) = req.into_parts();
        if !check_json_content(&parts) {
            return Ok(unsupported_media_type());
        }
        let path = parts.uri.path();

        // Parse pagination
        let query = parts.uri.query();
        let pagination = Pagination::from_query(query).unwrap_or_default();

        let payload = match serde_json::from_slice::<LedgerQuery>(&payload) {
            Ok(p) => p,
            Err(e) => return Ok(bad_request(e.to_string())),
        };

        // Select ledger
        let controller = Controller::new(&self.db);
        let ledger_id = match payload.to_ledger_id() {
            Some(id) => id,
            None => {
                return Ok(bad_request(
                    "must specify either ledger-id or creditor / debitor pair".to_string(),
                ))
            }
        };
        let ledger = controller.ledger().select(ledger_id);

        match path {
            "/" | "" => match payload.contract_id {
                Some(cid) => Ok(json_response(
                    &ledger.get_contract_paginated(cid, pagination)?,
                )),
                None => Ok(json_response(&ledger.get_entries_paginated(pagination)?)),
            },
            // TODO: Incorporate contract-id !
            "/balances" | "balances" => Ok(json_response(&ledger.balances()?)),
            // TODO: Incorporate contract-id
            "/meta" | "meta" => Ok(json_response(&ledger.meta()?)),
            // TODO: Add query for ledger by ledger-id (maybe post request ?)
            _ => Ok(reject_404()),
        }
    }
}

/// Selects a ledger either by its id or creditor / debitor tuple
#[derive(Deserialize)]
pub struct LedgerQuery {
    creditor: Option<BorderlessId>,
    debitor: Option<BorderlessId>,
    ledger_id: Option<u64>,
    contract_id: Option<ContractId>,
}

impl LedgerQuery {
    fn to_ledger_id(&self) -> Option<u64> {
        if self.ledger_id.is_some() {
            return self.ledger_id;
        }
        match (self.creditor, self.debitor) {
            (Some(c), Some(d)) => Some(c.merge_compact(&d)),
            _ => None,
        }
    }
}

impl<S> Service<Request> for LedgerService<S>
where
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
