mod actions;
mod client;
mod daemon;
mod pi_bridge;
mod poller;
mod queue;
mod render;
mod settings;
mod snapshot;
mod svg;

use openaction::global_events::*;
use openaction::*;
use simplelog::{Config, LevelFilter, SimpleLogger};

struct GlobalHandler;

#[async_trait]
impl GlobalEventHandler for GlobalHandler {
    async fn plugin_ready(&self) -> OpenActionResult<()> {
        get_global_settings().await
    }

    async fn did_receive_global_settings(
        &self,
        event: DidReceiveGlobalSettingsEvent,
    ) -> OpenActionResult<()> {
        settings::set_global(event.payload.settings);
        daemon::allow_start_again();
        poller::request_poll();
        Ok(())
    }

    async fn device_did_disconnect(&self, event: DeviceDidDisconnectEvent) -> OpenActionResult<()> {
        render::unregister_device(&event.device);
        Ok(())
    }

    async fn system_did_wake_up(&self, _event: SystemDidWakeUpEvent) -> OpenActionResult<()> {
        poller::request_poll();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let level = if std::env::var_os("LIMELIGHT_PLUGIN_DEBUG").is_some() {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    let _ = SimpleLogger::init(level, Config::default());
    log::info!("LimeLight plugin v{} starting", env!("CARGO_PKG_VERSION"));

    register_action(actions::power::Power).await;
    register_action(actions::brightness::BrightnessSet).await;
    register_action(actions::brightness::BrightnessAdjust).await;
    register_action(actions::temperature::TemperatureSet).await;
    register_action(actions::temperature::TemperatureAdjust).await;
    register_action(actions::color::Color).await;
    register_action(actions::preset::Preset).await;
    register_action(actions::battery::Battery).await;
    register_action(actions::identify::Identify).await;
    register_action(actions::scan::Scan).await;
    register_action(actions::dials::DialBrightness).await;
    register_action(actions::dials::DialTemperature).await;
    set_global_event_handler(&GlobalHandler);

    tokio::spawn(poller::run());
    run(std::env::args().collect()).await
}
