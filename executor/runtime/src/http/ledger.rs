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
        // strip leading “/” and split, collecting all non-empty segments
        let segs: Vec<&str> = req
            .uri()
            .path()
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let query = req.uri().query();

        let controller = Controller::new(&self.db);
        let pagination = Pagination::from_query(query).unwrap_or_default();
        match segs.as_slice() {
            // GET /
            [] => {
                let res = controller.ledger().all_paginated(pagination)?;
                Ok(json_response(&res))
            }
            // GET /ids
            ["ids"] => {
                let ids = controller.ledger().all_ids_paginated(pagination)?;
                Ok(json_response(&ids))
            }
            [id_str] => {
                let ledger_id = match id_str.parse::<u64>() {
                    Ok(id) => id,
                    Err(e) => return Ok(bad_request(e.to_string())),
                };
                let ledger = controller.ledger().select(ledger_id);
                Ok(json_response(&ledger.meta()?.map(|m| m.into_dto())))
            }
            [id_str, "entries"] => {
                let ledger_id = match id_str.parse::<u64>() {
                    Ok(id) => id,
                    Err(e) => return Ok(bad_request(e.to_string())),
                };
                let ledger = controller.ledger().select(ledger_id);
                let entries = ledger.get_entries_paginated(pagination)?;
                Ok(json_response(&entries))
            }
            _ => Ok(reject_404()),
        }
    }

    async fn process_post_rq(&self, req: Request) -> crate::Result<Response> {
        // Check request header
        let (parts, payload) = req.into_parts();
        if !check_json_content(&parts) {
            return Ok(unsupported_media_type());
        }

        // NOTE: We don't have any nested routes here, so we can get away with matching the path
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

        // NOTE: We haven't split the path, so the trailing '/' might be important depending on the
        // web-framework that embeds this service !
        match path {
            "/" | "" => match payload.contract_id {
                Some(cid) => Ok(json_response(&ledger.meta_for_contract(cid)?)),
                None => Ok(json_response(&ledger.meta()?.map(|m| m.into_dto()))),
            },
            "/entries" | "entries" => match payload.contract_id {
                Some(cid) => Ok(json_response(
                    &ledger.get_contract_paginated(cid, pagination)?,
                )),
                None => Ok(json_response(&ledger.get_entries_paginated(pagination)?)),
            },
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
