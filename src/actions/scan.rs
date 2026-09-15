use openaction::{Instance, OpenActionResult};

use crate::actions::define_action;
use crate::{client, daemon, poller};

define_action!(Scan, "scan", {
    async fn key_up(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        if !daemon::is_up() {
            return instance.show_alert().await;
        }
        let client = client::current();
        let result = tokio::task::spawn_blocking(move || client.refresh(1)).await;
        poller::request_poll();
        match result {
            Ok(Ok(())) => instance.show_ok().await,
            Ok(Err(why)) => {
                log::warn!("scan failed: {why}");
                instance.show_alert().await
            }
            Err(_) => instance.show_alert().await,
        }
    }
});
