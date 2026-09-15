use openaction::{Instance, OpenActionResult};

use crate::actions::{define_action, ready_target};
use crate::client;

define_action!(Identify, "identify", {
    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let Some(_) = ready_target(instance, settings).await else {
            return Ok(());
        };
        if settings.target.kind != "light" {
            return instance.show_alert().await;
        }
        let id = settings.target.id.clone();
        let client = client::current();
        let result = tokio::task::spawn_blocking(move || client.identify(&id)).await;
        match result {
            Ok(Ok(())) => instance.show_ok().await,
            Ok(Err(why)) => {
                log::warn!("identify failed: {why}");
                instance.show_alert().await
            }
            Err(_) => instance.show_alert().await,
        }
    }
});
