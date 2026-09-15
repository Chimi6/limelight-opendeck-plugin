use openaction::{Instance, OpenActionResult};

use crate::actions::{apply, define_action, ready_target};
use crate::client::UpdateRequest;

define_action!(Power, "power", {
    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let Some(agg) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let desired = match settings.mode.as_str() {
            "on" => true,
            "off" => false,
            _ => !agg.any_on,
        };
        let patch = UpdateRequest {
            on: Some(desired as u8),
            ..Default::default()
        };
        apply(&settings.target, patch).await;
        Ok(())
    }
});
