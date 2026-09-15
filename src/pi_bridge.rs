use openaction::{Instance, OpenActionResult};
use serde_json::{Value, json};

use crate::settings::Settings;
use crate::{client, daemon, poller, snapshot};

pub async fn handle(
    instance: &Instance,
    settings: &Settings,
    payload: &Value,
) -> OpenActionResult<()> {
    match payload.get("event").and_then(Value::as_str) {
        Some("getTargets") => send_targets(instance).await,
        Some("startDaemon") => {
            daemon::allow_start_again();
            daemon::ensure_running().await;
            poller::request_poll();
            send_targets(instance).await
        }
        Some("identify") => {
            let id = payload
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| settings.target.id.clone());
            if !id.is_empty() {
                let client = client::current();
                tokio::task::spawn_blocking(move || {
                    if let Err(why) = client.identify(&id) {
                        log::warn!("identify failed: {why}");
                    }
                });
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub async fn send_targets(instance: &Instance) -> OpenActionResult<()> {
    let snapshot = snapshot::read();
    let lights: Vec<Value> = snapshot
        .lights
        .iter()
        .map(|light| {
            json!({
                "id": light.id,
                "name": light.display_name(),
                "reachable": light.reachable,
                "color_capable": light.color_capable,
                "product": light.product,
            })
        })
        .collect();
    let groups: Vec<Value> = snapshot
        .groups
        .iter()
        .map(|group| json!({ "name": group.name, "members": group.members }))
        .collect();
    let payload = json!({
        "event": "targets",
        "lights": lights,
        "groups": groups,
        "daemon": {
            "online": daemon::is_up(),
            "version": daemon::version(),
            "status": format!("{:?}", daemon::status()),
        },
    });
    instance.send_to_property_inspector(payload).await
}
