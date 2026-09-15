use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use crate::client::{self, ClientError, UpdateRequest};
use crate::settings::Target;
use crate::{daemon, poller, render, snapshot};

const COALESCE_WINDOW: Duration = Duration::from_millis(50);

static PENDING: LazyLock<Mutex<HashMap<String, (Target, UpdateRequest)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ACTIVE: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn enqueue(target: &Target, patch: UpdateRequest) {
    snapshot::optimistic(target, &patch);
    snapshot::touch(target);

    let key = target.key();
    {
        let mut pending = PENDING.lock().unwrap();
        pending
            .entry(key.clone())
            .and_modify(|(_, existing)| existing.merge(&patch))
            .or_insert((target.clone(), patch));
    }

    let newly_active = ACTIVE.lock().unwrap().insert(key.clone());
    if newly_active {
        tokio::spawn(worker(key));
    }
}

async fn worker(key: String) {
    loop {
        tokio::time::sleep(COALESCE_WINDOW).await;
        let job = PENDING.lock().unwrap().remove(&key);
        let Some((target, patch)) = job else {
            ACTIVE.lock().unwrap().remove(&key);
            return;
        };
        send(&target, &patch, &key).await;
        render::render_all().await;
    }
}

async fn send(target: &Target, patch: &UpdateRequest, key: &str) {
    let client = client::current();
    let (owned_target, owned_patch) = (target.clone(), patch.clone());
    let result =
        tokio::task::spawn_blocking(move || client.update(&owned_target, &owned_patch)).await;

    match result {
        Ok(Ok(response)) => {
            for item in &response.results {
                if !item.ok {
                    log::warn!(
                        "update failed for {}: {}",
                        item.id,
                        item.error.as_deref().unwrap_or("unknown")
                    );
                }
            }
            let superseded = PENDING.lock().unwrap().contains_key(key);
            if !superseded {
                snapshot::apply_results(&response.results);
            }
            if !response.ok {
                snapshot::mark_unreachable(target);
            }
        }
        Ok(Err(ClientError::Unreachable(why))) => {
            log::warn!("update for {key} failed: daemon unreachable ({why})");
            daemon::mark_down(why);
            poller::request_poll_after(Duration::from_secs(1));
        }
        Ok(Err(other)) => {
            log::warn!("update for {key} failed: {other}");
            snapshot::mark_unreachable(target);
            poller::request_poll_after(Duration::from_secs(1));
        }
        Err(join_error) => log::error!("update task panicked: {join_error}"),
    }
}
