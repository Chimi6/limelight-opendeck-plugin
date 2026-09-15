use openaction::{Instance, OpenActionResult};

use crate::actions::{apply, clamp_kelvin, define_action, ready_target};
use crate::client::UpdateRequest;

define_action!(TemperatureSet, "temperature.set", {
    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let Some(_) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let kelvin = clamp_kelvin(settings.kelvin.unwrap_or(4500.0));
        let patch = UpdateRequest {
            kelvin: Some(kelvin),
            ..Default::default()
        };
        apply(&settings.target, patch).await;
        Ok(())
    }
});

define_action!(TemperatureAdjust, "temperature.adjust", {
    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let Some(agg) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let step = settings.step.unwrap_or(250.0);
        let kelvin = clamp_kelvin(agg.kelvin as f64 + step);
        let patch = UpdateRequest {
            kelvin: Some(kelvin),
            ..Default::default()
        };
        apply(&settings.target, patch).await;
        Ok(())
    }
});
