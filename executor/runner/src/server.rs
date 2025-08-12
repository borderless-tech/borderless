use std::{convert::Infallible, future::Future};

use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Request, Response},
    routing::method_routing,
    Router,
};
use borderless::{events::CallAction, hash::Hash256, BorderlessId, ContractId};
use borderless_kv_store::Db;
use borderless_runtime::{
    agent::SharedRuntime as SharedAgentRuntime,
    http::{
        agent::{EventHandler, RecursiveEventHandler, SwAgentService},
        contract::{ActionWriter, ContractService},
        ledger::LedgerService,
        Service,
    },
    SharedContractRuntime,
};
use log::info;

use crate::generate_tx_ctx;

/// Generalized wrapper - this is how you can bake the tower-service into any specific web-framework
async fn wrap_service<S>(State(mut srv): State<S>, req: Request<Body>) -> Response<Body>
where
    S: Service<
        borderless_runtime::http::Request,
        Error = Infallible,
        Response = borderless_runtime::http::Response,
    >,
{
    // We simply have to transform the request and response types here
    let (parts, body) = req.into_parts();

    // 10MB upper limit
    let bytes = to_bytes(body, 10_000_000).await.unwrap_or_default();

    let req = Request::from_parts(parts, bytes);
    let res = srv.call(req).await.expect("infallible");
    res.map(|bytes| bytes.into())
}

/// Wraps the contract service
async fn contract_handler(
    state: State<ContractService<impl ActionWriter, impl Db + 'static>>,
    req: Request<Body>,
) -> Response<Body> {
    wrap_service(state, req).await
}

/// Wraps the ledger service
async fn ledger_handler(
    state: State<LedgerService<impl Db + 'static>>,
    req: Request<Body>,
) -> Response<Body> {
    wrap_service(state, req).await
}

/// Wraps the agent service
async fn agent_handler<E, S>(
    state: State<SwAgentService<E, S>>,
    req: Request<Body>,
) -> Response<Body>
where
    E: EventHandler + 'static,
    S: Db + 'static,
{
    wrap_service(state, req).await
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
            Ok(events) => {
                let events = events.unwrap_or_default();
                // TODO: Recursively apply output events

                // Print messages ( since there are no agents )
                for msg in events.local {
                    info!(
                        "publish message to topic {}/{}: {}",
                        msg.publisher,
                        msg.topic.trim_matches('/'),
                        serde_json::to_string_pretty(&msg.value).unwrap_or_default()
                    );
                }
                Ok(hash)
            }
            Err(e) => Err(e),
        };

        let fut = async move { result };
        Box::pin(fut)
    }
}

pub async fn start_contract_server<DB: Db + 'static>(
    db: DB,
    rt: SharedContractRuntime<DB>,
    writer: BorderlessId,
) -> Result<()> {
    rt.lock().set_executor(writer)?;
    let action_writer = ActionApplier {
        rt: rt.clone(),
        writer,
    };
    let contract_srv = ContractService::with_shared(db.clone(), rt, action_writer, writer);
    let ledger_srv = LedgerService::new(db);

    // Create a router and attach the custom service to a route
    let contract = Router::new()
        .route("/", method_routing::any(contract_handler))
        .route("/{*any}", method_routing::any(contract_handler))
        .with_state(contract_srv);

    let ledger = Router::new()
        .route("/", method_routing::any(ledger_handler))
        .route("/{*any}", method_routing::any(ledger_handler))
        .with_state(ledger_srv);

    let app = Router::new()
        .nest("/v0/contract", contract)
        .nest("/v0/ledger", ledger);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn start_agent_server<DB: Db + 'static>(
    db: DB,
    rt: SharedAgentRuntime<DB>,
    writer: BorderlessId,
) -> Result<()> {
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
