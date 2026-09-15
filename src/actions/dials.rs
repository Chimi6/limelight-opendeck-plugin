use openaction::{Instance, OpenActionResult};

use crate::actions::{
    apply, clamp_brightness, clamp_kelvin, define_action, ready_target, toggle_power,
};
use crate::client::UpdateRequest;

define_action!(DialBrightness, "dial.brightness", {
    async fn dial_rotate(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        ticks: i16,
        _pressed: bool,
    ) -> OpenActionResult<()> {
        let Some(agg) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let step = settings.step_per_tick.unwrap_or(2.0);
        let brightness = clamp_brightness(agg.brightness as f64 + ticks as f64 * step);
        let patch = UpdateRequest {
            brightness: Some(brightness),
            ..Default::default()
        };
        apply(&settings.target, patch).await;
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        toggle_power(instance, settings).await
    }

    async fn touch_tap(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        _position: (u16, u16),
        hold: bool,
    ) -> OpenActionResult<()> {
        if !hold {
            return toggle_power(instance, settings).await;
        }
        let Some(_) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let brightness = clamp_brightness(settings.tap_preset.unwrap_or(50.0));
        let patch = UpdateRequest {
            on: Some(1),
            brightness: Some(brightness),
            ..Default::default()
        };
        apply(&settings.target, patch).await;
        Ok(())
    }
});

define_action!(DialTemperature, "dial.temperature", {
    async fn dial_rotate(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        ticks: i16,
        _pressed: bool,
    ) -> OpenActionResult<()> {
        let Some(agg) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let step = settings.step_per_tick.unwrap_or(100.0);
        let kelvin = clamp_kelvin(agg.kelvin as f64 + ticks as f64 * step);
        let patch = UpdateRequest {
            kelvin: Some(kelvin),
            ..Default::default()
        };
        apply(&settings.target, patch).await;
        Ok(())
    }

    async fn dial_down(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        toggle_power(instance, settings).await
    }

    async fn touch_tap(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        _position: (u16, u16),
        hold: bool,
    ) -> OpenActionResult<()> {
        if !hold {
            return toggle_power(instance, settings).await;
        }
        let Some(_) = ready_target(instance, settings).await else {
            return Ok(());
        };
        let kelvin = clamp_kelvin(settings.tap_preset.unwrap_or(4500.0));
        let patch = UpdateRequest {
            on: Some(1),
            kelvin: Some(kelvin),
            ..Default::default()
        };
        apply(&settings.target, patch).await;
        Ok(())
    }
});
