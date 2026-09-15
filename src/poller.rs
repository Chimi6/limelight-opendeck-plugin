use std::sync::LazyLock;
use std::time::{Duration, Instant};

use tokio::sync::Notify;

use crate::client::{self, ClientError};
use crate::{daemon, render, settings, snapshot};

const BATTERY_INTERVAL: Duration = Duration::from_secs(30);

static POLL_NOW: LazyLock<Notify> = LazyLock::new(Notify::new);

pub fn request_poll() {
    POLL_NOW.notify_one();
}

pub fn request_poll_after(delay: Duration) {
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        request_poll();
    });
}

pub async fn run() {
    let mut last_battery = Instant::now() - BATTERY_INTERVAL;
    loop {
        poll_once(&mut last_battery).await;
        let delay = next_delay();
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = POLL_NOW.notified() => {}
        }
    }
}

fn next_delay() -> Duration {
    if !daemon::is_up() {
        return Duration::from_secs(5);
    }
    if render::has_instances() {
        return Duration::from_secs(settings::global().poll_seconds.max(1));
    }
    Duration::from_secs(15)
}

async fn poll_once(last_battery: &mut Instant) {
    if !daemon::is_up() {
        daemon::ensure_running().await;
        if !daemon::is_up() {
            render::render_all().await;
            return;
        }
    }

    let client = client::current();
    let fetched = tokio::task::spawn_blocking(move || {
        let lights = client.states()?;
        let groups = client.groups()?;
        Ok::<_, ClientError>((lights, groups))
    })
    .await;

    match fetched {
        Ok(Ok((lights, groups))) => snapshot::replace_from_poll(lights, groups),
        Ok(Err(ClientError::Unreachable(why))) => {
            daemon::mark_down(why);
        }
        Ok(Err(other)) => log::warn!("poll failed: {other}"),
        Err(join_error) => log::error!("poll task panicked: {join_error}"),
    }

    if daemon::is_up() {
        refresh_batteries(last_battery).await;
    }
    render::render_all().await;
}

async fn refresh_batteries(last_battery: &mut Instant) {
    let ids = render::battery_targets();
    if ids.is_empty() {
        return;
    }
    let due = last_battery.elapsed() >= BATTERY_INTERVAL;
    let missing: Vec<String> = ids
        .iter()
        .filter(|id| snapshot::battery(id).is_none())
        .cloned()
        .collect();
    if !due && missing.is_empty() {
        return;
    }
    let wanted = if due { ids } else { missing };
    if due {
        *last_battery = Instant::now();
    }

    let client = client::current();
    let fetched = tokio::task::spawn_blocking(move || {
        wanted
            .into_iter()
            .map(|id| {
                let result = client.battery(&id);
                (id, result)
            })
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();

    for (id, result) in fetched {
        match result {
            Ok(battery) => snapshot::set_battery(&id, battery),
            Err(why) => log::debug!("battery for {id}: {why}"),
        }
    }
}
