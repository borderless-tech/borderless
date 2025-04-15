use std::{convert::Infallible, num::NonZeroUsize};

use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Request, Response},
    Router,
};
use borderless_kv_store::Db;
use borderless_runtime::{
    http::{ActionWriter, ContractService, Service},
    Runtime, SharedRuntime,
};
use borderless_sdk::{BorderlessId, ContractId};
use log::info;

use crate::generate_tx_ctx;

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

/// A dummy action-writer, that instantly applies the actions to the runtime
#[derive(Clone)]
struct ActionApplier<S: Db> {
    rt: SharedRuntime<S>,
    writer: BorderlessId,
}

impl<S: Db> ActionWriter for ActionApplier<S> {
    type Error = Infallible;

    fn write_action(
        &self,
        cid: ContractId,
        action: borderless_sdk::contract::CallAction,
    ) -> impl std::future::Future<Output = Result<borderless_sdk::hash::Hash256, Self::Error>> + Send
    {
        let rt = self.rt.lock();
        let tx_ctx = generate_tx_ctx(rt, &cid).unwrap();
        let hash = tx_ctx.tx_id.hash;

        let mut rt = self.rt.lock();
        rt.process_transaction(&cid, action, &self.writer, tx_ctx)
            .unwrap();

        let fut = async move { Ok(hash) };
        Box::pin(fut)
    }
}

pub async fn start_contract_server(db: impl Db + 'static) -> Result<()> {
    let writer = "bbcd81bb-b90c-8806-8341-fe95b8ede45a".parse()?;
    let rt = Runtime::new(&db, NonZeroUsize::new(10).unwrap())?.into_shared();
    let action_writer = ActionApplier {
        rt: rt.clone(),
        writer,
    };
    let srv = ContractService::with_shared(db, rt, action_writer, writer);

    // Create a router and attach the custom service to a route
    let contract = Router::new().fallback(contract_handler).with_state(srv);

    let app = Router::new().nest("/v0/contract", contract);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
