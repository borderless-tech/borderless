use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use borderless::{
    agents::{Schedule, WsConfig},
    events::Events,
    AgentId,
};
use borderless_kv_store::Db;
use futures_util::{SinkExt, StreamExt};
use thiserror::Error;
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinSet,
    time::{interval, sleep, MissedTickBehavior},
};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::{Bytes, Message};

use crate::log_shim::*;

use super::Runtime;

#[derive(Debug, Error)]
#[error("Critical error in schedule task - forced to shutdown")]
pub struct ScheduleError;

/// Function to handle all schedules of a single sw-agent
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(agent_id = %aid), err))]
pub async fn handle_schedules<S>(
    rt: Arc<Mutex<Runtime<S>>>,
    aid: AgentId,
    schedules: Vec<Schedule>,
    out_tx: mpsc::Sender<Events>,
) -> Result<(), ScheduleError>
where
    S: Db + 'static,
{
    let mut join_set = JoinSet::new();
    for sched in schedules {
        let rt = rt.clone();
        let action = sched.get_action();
        let out_tx = out_tx.clone();
        let action_name = action.print_method();

        join_set.spawn(async move {
            if sched.delay > 0 {
                sleep(Duration::from_millis(sched.delay)).await;
            }

            let mut interval = interval(Duration::from_millis(sched.interval));
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                interval.tick().await;
                // Dispatch output events
                let now = Instant::now();
                let result = rt.lock().await.process_action(&aid, action.clone()).await;
                match result {
                    Ok(Some(events)) => {
                        // NOTE: We panic here to shutdown the entire task in case the receiver is closed
                        out_tx
                            .send(events)
                            .await
                            .expect("receiver dropped or closed");
                    }
                    Ok(None) => (),
                    Err(e) => error!("failure while executing schedule {action_name}: {e}"),
                }
                info!(
                    "executed schedule {action_name}, time elapsed: {:?}",
                    now.elapsed()
                );
            }
        });
    }

    // This loop will run forever unless the outer task is cancelled.
    // If the outer task is aborted, all spawned tasks inside JoinSet are also dropped.
    while let Some(res) = join_set.join_next().await {
        // Catch panics here and shutdown the entire task
        if let Err(e) = res {
            error!("A schedule task failed: {e}");
            // Gracefully shut down all other tasks
            join_set.abort_all();
            // Return error
            return Err(ScheduleError);
        }
    }
    Ok(())
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(agent_id = %aid), err))]
pub async fn handle_ws_connection<S>(
    rt: Arc<Mutex<Runtime<S>>>,
    aid: AgentId,
    ws_config: WsConfig,
    out_tx: mpsc::Sender<Events>,
) -> crate::Result<()>
where
    S: Db + 'static,
{
    // Register the websocket at the runtime
    let mut msg_rx = rt.lock().await.register_ws(aid)?;

    let mut failure_cnt = 1;
    #[allow(clippy::while_immutable_condition)]
    while ws_config.reconnect {
        match handle_ws_inner(
            rt.clone(),
            aid,
            ws_config.clone(),
            out_tx.clone(),
            &mut msg_rx,
        )
        .await
        {
            Ok(()) => failure_cnt = 1,
            Err(e) => {
                warn!("cnt={failure_cnt}, agent-id={aid}, {e}");
                failure_cnt = (failure_cnt * 2).min(60);
                sleep(Duration::from_secs(failure_cnt)).await;
            }
        }
    }
    Ok(())
}

async fn handle_ws_inner<S>(
    rt: Arc<Mutex<Runtime<S>>>,
    aid: AgentId,
    ws_config: WsConfig,
    out_tx: mpsc::Sender<Events>,
    msg_rx: &mut mpsc::Receiver<Vec<u8>>,
) -> Result<(), String>
where
    S: Db + 'static,
{
    info!("opening websocket connection to '{}'", ws_config.url);
    let result = connect_async(&ws_config.url)
        .await
        .map_err(|e| format!("failed to open ws-connection - {e}"))?;

    let (stream, response) = result;
    if response.status().is_client_error() || response.status().is_server_error() {
        return Err(format!(
            "failed to open ws-connection - status={}",
            response.status()
        ));
    }

    // Call "on-open"
    handle_events(rt.lock().await.on_ws_open(&aid).await, &out_tx).await;

    // Set heartbeat timer
    let mut heartbeat_timer = interval(Duration::from_secs(ws_config.ping_interval.max(10)));

    // Now start receiving messages from the websocket
    let (mut tx, mut rx) = stream.split();

    info!("successfully opened ws-connection to '{}'", ws_config.url);

    // main loop
    loop {
        tokio::select! {
            biased;
            // Check heartbeat timer
            _ = heartbeat_timer.tick() => {
                let msg = Message::Ping(Vec::new().into());
                tx.send(msg).await.map_err(|e| format!("failed to send heartbeat: {e}"))?;
            }
            result = msg_rx.recv() => {
                let payload = result.ok_or("Websocket message receiver closed.")?;
                let msg = if ws_config.binary {
                    Message::Binary(payload.into())
                } else {
                    Message::Text(payload.try_into().unwrap())
                };
                // Send message
                if let Err(e) = tx.send(msg).await {
                    warn!("failed to send ws-msg: {e}");
                }
            }
            // Check incoming messages
            result = rx.next() => {
                if result.is_none() {
                    warn!("Websocket receiver closed.");
                    break;
                }
                let msg = result.unwrap();
                if msg.is_err() {
                    // TODO: Forward error message to wasm ?
                    warn!("Websocket-msg failure: {}", msg.unwrap_err());
                    // Call "on-error"
                    handle_events(rt.lock().await.on_ws_error(&aid).await, &out_tx).await;
                    break;
                }
                let data = match msg.unwrap() {
                    Message::Text(text) => {
                        // TODO: Remove this log line, once everything is up and running
                        info!("incoming text ws msg");
                        let bytes: Bytes = text.into();
                        bytes.into()
                    }
                    Message::Binary(b) => {
                        // TODO: Remove this log line, once everything is up and running
                        info!("incoming binary ws msg");
                        b.into()
                    }
                    Message::Pong(_) => continue,
                    Message::Close(frame) => {
                        // TODO: Forward closing frame to wasm ?
                        info!("Received closing frame: {frame:#?}");
                        // Call "on-close"
                        handle_events(rt.lock().await.on_ws_close(&aid).await, &out_tx).await;
                        break;
                    }
                    other => {
                        info!("receive other websocket msg: {other:#?}");
                        continue
                    }
                };

                // Apply message and dispatch output events
                handle_events(rt.lock().await.process_ws_msg(&aid, data).await, &out_tx).await;
            }
        }
    }
    Ok(())
}

async fn handle_events(result: crate::Result<Option<Events>>, out_tx: &mpsc::Sender<Events>) {
    match result {
        Ok(Some(events)) => {
            // NOTE: We panic here to shutdown the entire task in case the receiver is closed
            out_tx
                .send(events)
                .await
                .expect("receiver dropped or closed");
        }
        Ok(None) => (),
        Err(e) => error!("failure while executing on-ws-msg: {e}"),
    }
}
