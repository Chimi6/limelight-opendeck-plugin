pub mod battery;
pub mod brightness;
pub mod color;
pub mod dials;
pub mod identify;
pub mod power;
pub mod preset;
pub mod scan;
pub mod temperature;

use std::time::Duration;

use openaction::{Instance, OpenActionResult, TitleParametersDidChangePayload};

use crate::client::UpdateRequest;
use crate::settings::{Settings, Target};
use crate::snapshot::Aggregate;
use crate::{daemon, poller, queue, render, snapshot};

macro_rules! define_action {
    ($name:ident, $suffix:literal, { $($handlers:tt)* }) => {
        pub struct $name;

        #[openaction::async_trait]
        impl openaction::Action for $name {
            const UUID: &'static str = concat!("io.github.chimi6.limelight.", $suffix);
            type Settings = crate::settings::Settings;

            async fn will_appear(&self, instance: &openaction::Instance, settings: &Self::Settings) -> openaction::OpenActionResult<()> {
                crate::actions::on_appear(instance, settings).await
            }

            async fn will_disappear(&self, instance: &openaction::Instance, _settings: &Self::Settings) -> openaction::OpenActionResult<()> {
                crate::actions::on_disappear(instance).await
            }

            async fn did_receive_settings(&self, instance: &openaction::Instance, settings: &Self::Settings) -> openaction::OpenActionResult<()> {
                crate::actions::on_settings(instance, settings).await
            }

            async fn title_parameters_did_change(
                &self,
                instance: &openaction::Instance,
                _settings: &Self::Settings,
                event: &openaction::TitleParametersDidChangePayload,
            ) -> openaction::OpenActionResult<()> {
                crate::actions::on_title_parameters(instance, event).await
            }

            async fn property_inspector_did_appear(&self, instance: &openaction::Instance, _settings: &Self::Settings) -> openaction::OpenActionResult<()> {
                crate::pi_bridge::send_targets(instance).await
            }

            async fn send_to_plugin(
                &self,
                instance: &openaction::Instance,
                settings: &Self::Settings,
                payload: &serde_json::Value,
            ) -> openaction::OpenActionResult<()> {
                crate::pi_bridge::handle(instance, settings, payload).await
            }

            $($handlers)*
        }
    };
}
pub(crate) use define_action;

pub async fn on_appear(instance: &Instance, settings: &Settings) -> OpenActionResult<()> {
    if instance.is_in_multi_action {
        return Ok(());
    }
    render::register(instance, settings);
    render::render_one(&instance.instance_id).await;
    if snapshot::is_stale(Duration::from_secs(3)) {
        poller::request_poll();
    }
    Ok(())
}

pub async fn on_disappear(instance: &Instance) -> OpenActionResult<()> {
    render::unregister(&instance.instance_id);
    Ok(())
}

pub async fn on_settings(instance: &Instance, settings: &Settings) -> OpenActionResult<()> {
    if instance.is_in_multi_action {
        return Ok(());
    }
    render::update_settings(&instance.instance_id, settings);
    render::render_one(&instance.instance_id).await;
    poller::request_poll();
    Ok(())
}

pub async fn on_title_parameters(
    instance: &Instance,
    event: &TitleParametersDidChangePayload,
) -> OpenActionResult<()> {
    render::note_title_event(&instance.instance_id, event.title.trim());
    render::render_one(&instance.instance_id).await;
    Ok(())
}

pub async fn ready_target(instance: &Instance, settings: &Settings) -> Option<Aggregate> {
    let ready = daemon::is_up() && settings.target.is_configured();
    let aggregate = if ready {
        snapshot::read().aggregate(&settings.target)
    } else {
        None
    };
    match aggregate {
        Some(agg) if agg.reachable => Some(agg),
        _ => {
            let _ = instance.show_alert().await;
            None
        }
    }
}

pub async fn apply(target: &Target, patch: UpdateRequest) {
    queue::enqueue(target, patch);
    render::render_all().await;
}

pub async fn toggle_power(instance: &Instance, settings: &Settings) -> OpenActionResult<()> {
    let Some(agg) = ready_target(instance, settings).await else {
        return Ok(());
    };
    let on = if agg.any_on { 0 } else { 1 };
    let patch = UpdateRequest {
        on: Some(on),
        ..Default::default()
    };
    apply(&settings.target, patch).await;
    Ok(())
}

pub fn clamp_brightness(value: f64) -> u8 {
    value.round().clamp(0.0, 100.0) as u8
}

pub fn clamp_kelvin(value: f64) -> u16 {
    value.round().clamp(2900.0, 7000.0) as u16
}
