use std::sync::{LazyLock, RwLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_PORT: u16 = 9124;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Target {
    pub kind: String,
    pub id: String,
}

impl Target {
    pub fn is_configured(&self) -> bool {
        match self.kind.as_str() {
            "all" => true,
            "light" | "group" => !self.id.is_empty(),
            _ => false,
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.kind, self.id)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub v: u8,
    pub target: Target,
    pub mode: String,
    pub value: Option<f64>,
    pub turn_on: Option<bool>,
    pub step: Option<f64>,
    pub kelvin: Option<f64>,
    pub hue: Option<f64>,
    pub saturation: Option<f64>,
    pub brightness: Option<f64>,
    pub use_color: Option<bool>,
    pub label: String,
    pub on: Option<bool>,
    pub step_per_tick: Option<f64>,
    pub tap_preset: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct GlobalSettings {
    pub port: u16,
    pub poll_seconds: u64,
    pub start_daemon: bool,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        let env_port = std::env::var("LIMELIGHT_PORT")
            .ok()
            .and_then(|value| value.trim().parse::<u16>().ok())
            .filter(|port| *port != 0);
        GlobalSettings {
            port: env_port.unwrap_or(DEFAULT_PORT),
            poll_seconds: 3,
            start_daemon: true,
        }
    }
}

static GLOBAL: LazyLock<RwLock<GlobalSettings>> =
    LazyLock::new(|| RwLock::new(GlobalSettings::default()));

pub fn global() -> GlobalSettings {
    GLOBAL.read().unwrap().clone()
}

pub fn set_global(value: Value) {
    let mut parsed: GlobalSettings = serde_json::from_value(value).unwrap_or_default();
    if parsed.port == 0 {
        parsed.port = GlobalSettings::default().port;
    }
    if parsed.poll_seconds == 0 {
        parsed.poll_seconds = 3;
    }
    log::info!(
        "global settings: port={} poll={}s startDaemon={}",
        parsed.port,
        parsed.poll_seconds,
        parsed.start_daemon
    );
    *GLOBAL.write().unwrap() = parsed;
}
