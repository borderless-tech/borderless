use anyhow::Result;
use borderless_kv_store::RawRead;
use borderless_kv_store::{backend::lmdb::Lmdb, Db};
use borderless_sdk::__private::{from_postcard_bytes, storage_keys::*};
use borderless_sdk::http::ContractInfo;
use borderless_sdk::{
    contract::{Description, Info, Metadata},
    http::{queries::Pagination, PaginatedElements, TxAction},
    ContractId,
};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use log::info;
use mime::{APPLICATION_JSON, TEXT_PLAIN_UTF_8};
use parking_lot::Mutex;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    convert::Infallible,
    task::{Context, Poll},
    time::Instant,
};
use std::{
    future::{ready, Ready},
    sync::Arc,
};
pub use tower::Service;

use crate::CONTRACT_SUB_DB;
use crate::{logger::Logger, vm::len_actions, Runtime};

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

fn read_key<S, D>(db: &S, key: StorageKey) -> anyhow::Result<Option<D>>
where
    S: Db,
    D: DeserializeOwned,
{
    let db_ptr = db.open_sub_db(CONTRACT_SUB_DB)?;
    let txn = db.begin_ro_txn()?;
    let bytes = match txn.read(&db_ptr, &key)? {
        Some(bytes) => bytes,
        None => return Ok(None),
    };
    let value = from_postcard_bytes(bytes)?;
    Ok(Some(value))
}

fn read_contract_info(db: &impl Db, cid: &ContractId) -> anyhow::Result<Option<Info>> {
    let participants = read_key(
        db,
        StorageKey::system_key(cid, BASE_KEY_METADATA, META_SUB_KEY_PARTICIPANTS),
    )?;
    let roles = read_key(
        db,
        StorageKey::system_key(cid, BASE_KEY_METADATA, META_SUB_KEY_ROLES),
    )?;
    let sinks = read_key(
        db,
        StorageKey::system_key(cid, BASE_KEY_METADATA, META_SUB_KEY_SINKS),
    )?;
    info!("{participants:?}, {roles:?}, {sinks:?}");
    match (participants, roles, sinks) {
        (Some(participants), Some(roles), Some(sinks)) => Ok(Some(Info {
            contract_id: *cid,
            participants,
            roles,
            sinks,
        })),
        _ => Ok(None),
    }
}

fn read_contract_desc(db: &impl Db, cid: &ContractId) -> anyhow::Result<Option<Description>> {
    read_key(
        db,
        StorageKey::system_key(cid, BASE_KEY_METADATA, META_SUB_KEY_DESC),
    )
}

fn read_contract_meta(db: &impl Db, cid: &ContractId) -> anyhow::Result<Option<Metadata>> {
    read_key(
        db,
        StorageKey::system_key(cid, BASE_KEY_METADATA, META_SUB_KEY_META),
    )
}

// TODO: Query params
fn read_contract_full(db: &impl Db, cid: &ContractId) -> anyhow::Result<Option<ContractInfo>> {
    let info = read_contract_info(db, cid)?;
    info!("- {info:?}");
    let desc = read_contract_desc(db, cid)?;
    info!("- {desc:?}");
    let meta = read_contract_meta(db, cid)?;
    info!("- {meta:?}");
    Ok(Some(ContractInfo { info, desc, meta }))
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
                let full_info = read_contract_full(&self.db, &contract_id)?;
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
        if let Some(query) = req.uri().query() {
            trunc.push('?');
            trunc.push_str(query);
        }

        let mut rt = self.rt.lock();
        match route {
            "state" => {
                // TODO: The contract should also parse query parameters !
                let (status, payload) = rt.http_get_state(&contract_id, trunc)?;
                if status == 200 {
                    return Ok(json_body(payload));
                } else {
                    return Ok(reject_404());
                }
            }
            "log" => {
                let logger = Logger::new(&self.db, contract_id);

                // Extract pagination
                let pagination = Pagination::from_query(query).unwrap_or_default();

                // Get logs
                let log = logger.get_logs_paginated(pagination)?;
                return Ok(json_response(&log));
            }
            "txs" => {
                // Extract pagination
                let pagination = Pagination::from_query(query).unwrap_or_default();

                // Get actions
                let n_actions = len_actions(&self.db, &contract_id)?.unwrap_or_default() as usize;

                let mut elements = Vec::new();
                for idx in pagination.to_range() {
                    match rt.read_action(&contract_id, idx)? {
                        Some(record) => {
                            let action = TxAction::try_from(record)?;
                            elements.push(action);
                        }
                        None => break,
                    }
                }
                let paginated = PaginatedElements {
                    elements,
                    total_elements: n_actions,
                    pagination,
                };
                return Ok(json_response(&paginated));
            }
            "info" => {
                let info = read_contract_info(&self.db, &contract_id)?;
                return Ok(json_response(&info));
            }
            "desc" => {
                let desc = read_contract_desc(&self.db, &contract_id)?;
                return Ok(json_response(&desc));
            }
            "meta" => {
                let meta = read_contract_meta(&self.db, &contract_id)?;
                return Ok(json_response(&meta));
            }
            _ => return Ok(reject_404()),
        }
    }
}

impl<S: Db> Service<Request> for RtService<S> {
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let start = Instant::now();
        let result: Response = match self.process_rq(req) {
            Ok(r) => r,
            Err(e) => into_server_error(e),
        };
        let elapsed = start.elapsed();
        info!("Time elapsed: {elapsed:?}");
        ready(Ok(result))
    }
}
