use openaction::{Instance, OpenActionResult};

use crate::actions::{apply, clamp_brightness, clamp_kelvin, define_action, ready_target};
use crate::client::UpdateRequest;

define_action!(Preset, "preset", {
    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        let Some(_) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let on = settings.on.unwrap_or(true);
        let mut patch = UpdateRequest {
            on: Some(on as u8),
            ..Default::default()
        };
        if on {
            patch.brightness = Some(clamp_brightness(settings.brightness.unwrap_or(50.0)));
            let use_color = settings.use_color.unwrap_or(false) && settings.hue.is_some();
            if use_color {
                patch.hue = Some(settings.hue.unwrap_or(0.0).rem_euclid(360.0) as f32);
                patch.saturation =
                    Some(settings.saturation.unwrap_or(100.0).clamp(0.0, 100.0) as f32);
            } else {
                patch.kelvin = Some(clamp_kelvin(settings.kelvin.unwrap_or(4500.0)));
            }
        }
        apply(&settings.target, patch).await;
        Ok(())
    }
});
