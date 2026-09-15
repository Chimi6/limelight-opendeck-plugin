use openaction::{Instance, OpenActionResult};

use crate::actions::{apply, clamp_brightness, define_action, ready_target};
use crate::client::UpdateRequest;

define_action!(Color, "color", {
    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let Some(_) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let patch = UpdateRequest {
            on: Some(1),
            hue: Some(settings.hue.unwrap_or(0.0).rem_euclid(360.0) as f32),
            saturation: Some(settings.saturation.unwrap_or(100.0).clamp(0.0, 100.0) as f32),
            brightness: Some(clamp_brightness(settings.brightness.unwrap_or(100.0))),
            kelvin: None,
        };
        apply(&settings.target, patch).await;
        Ok(())
    }
});
