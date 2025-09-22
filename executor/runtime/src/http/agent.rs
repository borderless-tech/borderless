pub use super::*;
use crate::log_shim::*;
use crate::{db::controller::Controller, rt::agent::Runtime};
use borderless::events::{Message, Topic, TopicDto};
use borderless::{
    events::{CallAction, Events},
    http::queries::Pagination,
    AgentId, BorderlessId,
};
use borderless_kv_store::{backend::lmdb::Lmdb, Db};
use http::method::Method;
use serde_json::json;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::future::Future;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};
use tokio::sync::Mutex;

#[derive(Serialize)]
pub struct ActionResp {
    pub events: Events,
    pub action: CallAction,
}

pub trait EventHandler: Clone + Send + Sync {
    type Error: std::fmt::Display + Send + Sync;

    fn handle_events(&self, events: Events)
        -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// A dummy implementation of an event-handler, that does nothing with the events.
///
/// Useful for testing.
#[derive(Clone)]
pub struct NoEventHandler;

impl EventHandler for NoEventHandler {
    type Error = Infallible;

    async fn handle_events(&self, _events: Events) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// A dummy implementation of an event-handler, that immediately applies all agent events
///
/// Discards all contract events in the process.
///
/// Useful for testing.
#[derive(Clone)]
pub struct RecursiveEventHandler<S: Db> {
    pub rt: Arc<Mutex<Runtime<S>>>,
}

impl<S: Db> EventHandler for RecursiveEventHandler<S> {
    type Error = crate::Error;

    async fn handle_events(&self, events: Events) -> Result<(), Self::Error> {
        let mut rt = self.rt.lock().await;
        let db = rt.get_db();
        let sub_handler = Controller::new(&db).messages();

        let mut agent_events: VecDeque<_> = events.local.into();
        // Handle events one by one
        while let Some(Message {
            publisher,
            topic,
            value,
        }) = agent_events.pop_front()
        {
            let subscribers = sub_handler.get_topic_subscribers(publisher, topic)?;

            for (subscriber, method) in subscribers {
                let action = CallAction::by_method(method, value.clone());
                // Queue all process events and apply them again
                if let Some(events) = rt.process_action(&subscriber, action).await? {
                    agent_events.extend(events.local);
                }
            }
        }
        Ok(())
    }
}

/// Simple service around the runtime
#[derive(Clone)]
pub struct SwAgentService<E, S = Lmdb>
where
    S: Db + 'static,
    E: EventHandler,
{
    rt: Arc<Mutex<Runtime<S>>>,
    db: S,
    // TODO: This is not optimal. The runtime is not tied to a tx-writer,
    // and for our multi-tenant contract-node we require this to be flexible.
    writer: BorderlessId,
    event_handler: E,
}

impl<S, E> SwAgentService<E, S>
where
    S: Db + 'static,
    E: EventHandler,
{
    pub fn new(db: S, rt: Runtime<S>, writer: BorderlessId, event_handler: E) -> Self {
        Self {
            rt: Arc::new(Mutex::new(rt)),
            db,
            writer,
            event_handler,
        }
    }

    pub fn with_shared(
        db: S,
        rt: Arc<Mutex<Runtime<S>>>,
        event_handler: E,
        writer: BorderlessId,
    ) -> Self {
        Self {
            rt,
            db,
            writer,
            event_handler,
        }
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
        let query = req.uri().query();

        if path == "/" {
            let agents = self.rt.lock().await.available_agents()?;
            return Ok(json_response(&agents));
        }

        let mut pieces = path.split('/').skip(1);

        // Extract agent-id from first piece
        let agent_id: AgentId = match pieces.next().and_then(|first| first.parse().ok()) {
            Some(aid) => aid,
            None => return Ok(reject_404()),
        };
        let controller = Controller::new(&self.db);

        // Ensure, that the agent exists
        if !controller.agent_exists(&agent_id)? {
            return Ok(reject_404());
        }

        // Get top-level route
        let route = match pieces.next() {
            Some(r) => r,
            None => {
                // Get full agent info
                let full_info = controller.agent_full(&agent_id)?;
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
                // TODO: The agent should also parse query parameters !
                let mut rt = self.rt.lock().await;
                let (status, payload) = rt.http_get_state(&agent_id, trunc).await?;
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
                let log = controller.logs(agent_id).get_logs_paginated(pagination)?;

                Ok(json_response(&log))
            }
            "sinks" => {
                let sinks = controller.agent_sinks(&agent_id)?;
                Ok(json_response(&sinks))
            }
            "subs" => {
                let subs = controller.agent_subs(&agent_id)?;
                Ok(json_response(&subs))
            }
            "desc" => {
                let desc = controller.agent_desc(&agent_id)?;
                Ok(json_response_nested(desc, &trunc))
            }
            "meta" => {
                let meta = controller.agent_meta(&agent_id)?;
                Ok(json_response_nested(meta, &trunc))
            }
            "symbols" => {
                let mut rt = self.rt.lock().await;
                let symbols = rt.get_symbols(&agent_id).await?;
                Ok(json_response_nested(symbols, &trunc))
            }
            "pkg" => match trunc.as_str() {
                "/" => {
                    let result = controller.agent_pkg_full(&agent_id)?.map(|r| r.into_dto());
                    Ok(json_response(&result))
                }
                "/def" => {
                    let result = controller.agent_pkg_def(&agent_id)?.map(|r| r.into_dto());
                    Ok(json_response(&result))
                }
                "/source" => {
                    let result = controller.agent_pkg_source(&agent_id)?;
                    Ok(json_response(&result))
                }
                _ => Ok(reject_404()),
            },
            // Same as empty path
            "" => {
                let full_info = controller.agent_full(&agent_id)?;
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

        // Extract agent-id from first piece
        let aid_str = match pieces.next() {
            Some(s) => s,
            None => return Ok(method_not_allowed()),
        };
        let agent_id: AgentId = match aid_str.parse() {
            Ok(aid) => aid,
            Err(e) => return Ok(bad_request(format!("failed to parse agent-id - {e}"))),
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
                let (events, action) = {
                    let mut rt = self.rt.lock().await;
                    rt.set_executor(self.writer)?; // For agents the executor and the writer are actually the same
                    match rt
                        .http_post_action(&agent_id, trunc, payload.into(), &self.writer)
                        .await?
                    {
                        Ok(out) => out,
                        Err((status, err)) => {
                            return Ok(err_response(status.try_into().unwrap(), err))
                        }
                    }
                };
                // Forward the events
                match self.event_handler.handle_events(events.clone()).await {
                    Ok(_) => {
                        // Build action response
                        let resp = ActionResp { events, action };
                        Ok(json_response(&resp))
                    }
                    Err(e) => Ok(into_server_error(e)),
                }
            }
            "subscribe" => {
                // Check request header
                let (parts, payload) = req.into_parts();
                if !check_json_content(&parts) {
                    return Ok(unsupported_media_type());
                }
                // Extract Topic from request
                let payload: Vec<u8> = payload.into();
                let dto = match TopicDto::from_bytes(payload.as_slice()) {
                    Ok(dto) => dto,
                    Err(e) => return Ok(bad_request(format!("failed to parse topic - {e}"))),
                };
                let topic = Topic::from(dto);
                // Control that there are no newline characters in topic
                if !topic.validate() {
                    return Ok(bad_request("topic contains invalid characters".to_string()));
                }
                // Start subscription
                Controller::new(&self.db)
                    .messages()
                    .subscribe(agent_id, topic)
                    .expect("Handle error");
                Ok(json_response(&json!({"Success": true})))
            }
            "unsubscribe" => {
                // Check request header
                let (parts, payload) = req.into_parts();
                if !check_json_content(&parts) {
                    return Ok(unsupported_media_type());
                }
                // Extract Topic from request
                let payload: Vec<u8> = payload.into();
                let dto = match TopicDto::from_bytes(payload.as_slice()) {
                    Ok(dto) => dto,
                    Err(e) => return Ok(bad_request(format!("failed to parse topic - {e}"))),
                };
                let topic = Topic::from(dto);
                // Stop subscription
                Controller::new(&self.db)
                    .messages()
                    .unsubscribe(agent_id, topic.publisher, topic.topic)
                    .expect("Handle error");
                Ok(json_response(&json!({"Success": true})))
            }
            "" => Ok(method_not_allowed()),
            _ => Ok(reject_404()),
        }
    }
}

impl<E, S> Service<Request> for SwAgentService<E, S>
where
    S: Db + 'static,
    E: EventHandler + 'static,
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
