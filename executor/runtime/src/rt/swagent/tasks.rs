use std::{error::Error, sync::Arc, time::Duration};

use borderless::{agents::Schedule, events::Events, AgentId};
use borderless_kv_store::Db;
use log::error;
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

pub async fn handle_schedules<S>(
    rt: Arc<Mutex<Runtime<S>>>,
    aid: AgentId,
    schedules: Vec<Schedule>,
    out_tx: mpsc::Sender<Events>,
) -> Result<(), Box<dyn Error>>
where
    S: Db + 'static,
{
    let mut join_set = JoinSet::new();
    for sched in schedules {
        let rt = rt.clone();
        let aid = aid.clone();
        let action = sched.get_action();
        let period = sched.period;
        let delay = sched.delay;
        let immediate = sched.immediate;

        join_set.spawn(async move {
            if immediate {
                // TODO: Dispatch output events
                let _ = rt.lock().await.process_action(&aid, action.clone()).await;
            }

            if !immediate && delay > 0 {
                sleep(Duration::from_secs(delay as u64)).await;
            }

            let mut interval = interval(Duration::from_secs(period as u64));
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                interval.tick().await;
                // TODO: Dispatch output events
                let _ = rt.lock().await.process_action(&aid, action.clone()).await;
            }
        });
    }

    // This loop will run forever unless the outer task is cancelled.
    // If the outer task is aborted, all spawned tasks inside JoinSet are also dropped.
    while let Some(res) = join_set.join_next().await {
        // Optional: log errors if any task fails (though they are infinite loops)
        if let Err(e) = res {
            error!("A schedule task failed: {e}");
        }
    }
    Ok(())
}
