use std::future::Future;

use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Request, Response},
    Router,
};
use borderless::{events::CallAction, hash::Hash256, BorderlessId, ContractId};
use borderless_kv_store::Db;
use borderless_runtime::{
    agent::SharedRuntime as SharedAgentRuntime,
    http::{
        agent::{EventHandler, RecursiveEventHandler, SwAgentService},
        contract::{ActionWriter, ContractService},
        Service,
    },
    SharedContractRuntime,
};
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

/// Wraps the agent service
async fn agent_handler<E, S>(
    State(mut srv): State<SwAgentService<E, S>>,
    req: Request<Body>,
) -> Response<Body>
where
    E: EventHandler + 'static,
    S: Db + 'static,
{
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
    rt: SharedContractRuntime<S>,
    writer: BorderlessId,
}

impl<S: Db> ActionWriter for ActionApplier<S> {
    type Error = borderless_runtime::Error;

    fn write_action(
        &self,
        cid: ContractId,
        action: CallAction,
    ) -> impl Future<Output = Result<Hash256, Self::Error>> + Send {
        let rt = self.rt.lock();
        let tx_ctx = generate_tx_ctx(rt, &cid).unwrap();
        let hash = tx_ctx.tx_id.hash;

        let mut rt = self.rt.lock();
        let result = match rt.process_transaction(&cid, action, &self.writer, tx_ctx) {
            Ok(_events) => Ok(hash),
            Err(e) => Err(e),
        };

        let fut = async move { result };
        Box::pin(fut)
    }
}

pub async fn start_contract_server<DB: Db + 'static>(
    db: DB,
    rt: SharedContractRuntime<DB>,
) -> Result<()> {
    let writer = "bbcd81bb-b90c-8806-8341-fe95b8ede45a".parse()?;
    rt.lock().set_executor(writer)?;
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

pub async fn start_agent_server<DB: Db + 'static>(
    db: DB,
    rt: SharedAgentRuntime<DB>,
) -> Result<()> {
    let writer = "bbcd81bb-b90c-8806-8341-fe95b8ede45a".parse()?;
    let event_handler = RecursiveEventHandler { rt: rt.clone() };
    rt.lock().await.set_executor(writer)?;
    let srv = SwAgentService::with_shared(db, rt, event_handler, writer);

    // Create a router and attach the custom service to a route
    let contract = Router::new().fallback(agent_handler).with_state(srv);

    let app = Router::new().nest("/v0/agent", contract);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
