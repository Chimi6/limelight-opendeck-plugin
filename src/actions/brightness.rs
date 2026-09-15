use openaction::{Instance, OpenActionResult};

use crate::actions::{apply, clamp_brightness, define_action, ready_target};
use crate::client::UpdateRequest;

define_action!(BrightnessSet, "brightness.set", {
    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let Some(_) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let brightness = clamp_brightness(settings.value.unwrap_or(50.0));
        let turn_on = settings.turn_on.unwrap_or(true);
        let patch = UpdateRequest {
            brightness: Some(brightness),
            on: if turn_on { Some(1) } else { None },
            ..Default::default()
        };
        apply(&settings.target, patch).await;
        Ok(())
    }
});

define_action!(BrightnessAdjust, "brightness.adjust", {
    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let Some(agg) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let step = settings.step.unwrap_or(10.0);
        let brightness = clamp_brightness(agg.brightness as f64 + step);
        let patch = UpdateRequest {
            brightness: Some(brightness),
            ..Default::default()
        };
        apply(&settings.target, patch).await;
        Ok(())
    }
});
