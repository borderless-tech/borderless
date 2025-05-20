use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use borderless::{
    agents::{Schedule, WsConfig},
    events::Events,
    AgentId,
};
use borderless_kv_store::Db;
use log::{error, info, warn};
use thiserror::Error;
use tokio::{
    sync::{broadcast, mpsc, Mutex},
    task::JoinSet,
    time::{interval, sleep, MissedTickBehavior},
};

use super::Runtime;

// TODO: There is one inconsistency in our design:
// In general, there should never be more than one mutable execution for an agent,
// or we might end up with changing the state, while something else is still running !
// This is also true for the contracts, but since there is only one thread feeding the transactions, this is always true.
//
// For the agents though, the schedules and the websocket messages and the actions can occur completely at random.
// We should therefore implement some mechanism, that prevent multiple runtimes from mutating the state at the same time !
// -> Semaphore over the agent-id

#[derive(Debug, Error)]
#[error("Critical error in schedule task - forced to shutdown")]
pub struct ScheduleError;

/// Function to handle all schedules of a single sw-agent
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
        let aid = aid.clone();
        let action = sched.get_action();
        let out_tx = out_tx.clone();

        join_set.spawn(async move {
            if sched.delay > 0 {
                sleep(Duration::from_millis(sched.delay)).await;
            }

            let mut interval = interval(Duration::from_millis(sched.period));
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                interval.tick().await;
                // Dispatch output events
                let now = SystemTime::now();
                match rt.lock().await.process_action(&aid, action.clone()).await {
                    Ok(Some(events)) => {
                        // NOTE: We panic here to shutdown the entire task in case the receiver is closed
                        out_tx
                            .send(events)
                            .await
                            .expect("receiver dropped or closed");
                    }
                    Ok(None) => (),
                    Err(e) => error!("failure while executing schedule: {e}"),
                }
                info!("-- Outer time elapsed: {:?}", now.elapsed().unwrap());
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

use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::{
    http::{Method, Request},
    Bytes, Message,
};
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};

pub async fn handle_ws_connection<S>(
    rt: Arc<Mutex<Runtime<S>>>,
    aid: AgentId,
    ws_config: WsConfig,
    mut msg_rx: mpsc::Receiver<Vec<u8>>,
    out_tx: mpsc::Sender<Events>,
) where
    S: Db + 'static,
{
    // Open websocket connection to given url
    // TODO: Retry ? (add outer loop)
    info!("opening websocket connection to '{}'", ws_config.url);
    let result = connect_async(ws_config.url)
        .await
        .expect("failed to establish websocket connection");

    let (stream, response) = result;
    info!("{response:#?}");

    // Set heartbeat timer
    let mut heartbeat_timer = interval(Duration::from_secs(ws_config.ping_interval));

    // Now start receiving messages from the websocket
    let (mut tx, mut rx) = stream.split();

    // main loop
    loop {
        tokio::select! {
            biased;
            // Check heartbeat timer
            _ = heartbeat_timer.tick() => {
                let msg = Message::Ping(Vec::new().into());
                if let Err(e) = tx.send(msg).await {
                    warn!("Error while sending heartbeat: {e}");
                    break;
                }
            }
            result = msg_rx.recv() => {
                if result.is_none() {
                    warn!("Websocket message receiver closed.");
                    break;
                }
                let payload = result.unwrap();
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
                    warn!("Websocket-msg failure: {}", msg.unwrap_err());
                    break;
                }
                let data = match msg.unwrap() {
                    Message::Text(text) => {
                        let bytes: Bytes = text.into();
                        bytes.into()
                    }
                    Message::Binary(b) => b.into(),
                    Message::Pong(_) => continue,
                    Message::Close(frame) => {
                        info!("Received closing frame: {frame:#?}");
                        break;
                    }
                    other => {
                        info!("receive other websocket msg: {other:#?}");
                        continue
                    }
                };

                // Apply message and dispatch output events
                match rt.lock().await.process_ws_msg(&aid, data).await {
                    Ok(Some(events)) => {
                        // NOTE: We panic here to shutdown the entire task in case the receiver is closed
                        out_tx
                            .send(events)
                            .await
                            .expect("receiver dropped or closed");
                    }
                    Ok(None) => (),
                    Err(e) => error!("failure while executing schedule: {e}"),
                }
            }
        }
    }
}
