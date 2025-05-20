use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use borderless::{agents::Schedule, events::Events, AgentId};
use borderless_kv_store::Db;
use log::{error, info};
use thiserror::Error;
use tokio::{
    sync::{mpsc, Mutex},
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
                sleep(Duration::from_secs(sched.delay as u64)).await;
            }

            let mut interval = interval(Duration::from_secs(sched.period as u64));
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
