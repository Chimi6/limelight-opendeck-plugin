use std::sync::LazyLock;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use openaction::Instance;
use serde_json::{Value, json};

use crate::daemon;
use crate::settings::Settings;
use crate::snapshot::{self, Aggregate};
use crate::svg::{self, Badge, Glyph, Tile};

pub const UUID_PREFIX: &str = "io.github.chimi6.limelight.";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Power,
    BrightnessSet,
    BrightnessAdjust,
    TemperatureSet,
    TemperatureAdjust,
    Color,
    Preset,
    Battery,
    Identify,
    Scan,
    DialBrightness,
    DialTemperature,
}

impl Kind {
    pub fn from_uuid(uuid: &str) -> Option<Kind> {
        let kind = match uuid.strip_prefix(UUID_PREFIX)? {
            "power" => Kind::Power,
            "brightness.set" => Kind::BrightnessSet,
            "brightness.adjust" => Kind::BrightnessAdjust,
            "temperature.set" => Kind::TemperatureSet,
            "temperature.adjust" => Kind::TemperatureAdjust,
            "color" => Kind::Color,
            "preset" => Kind::Preset,
            "battery" => Kind::Battery,
            "identify" => Kind::Identify,
            "scan" => Kind::Scan,
            "dial.brightness" => Kind::DialBrightness,
            "dial.temperature" => Kind::DialTemperature,
            _ => return None,
        };
        Some(kind)
    }

    fn is_dial(self) -> bool {
        matches!(self, Kind::DialBrightness | Kind::DialTemperature)
    }

    fn glyph(self) -> Glyph {
        match self {
            Kind::Power => Glyph::Panel,
            Kind::BrightnessSet | Kind::BrightnessAdjust | Kind::DialBrightness => Glyph::Sun,
            Kind::TemperatureSet | Kind::TemperatureAdjust | Kind::DialTemperature => {
                Glyph::Thermometer
            }
            Kind::Color => Glyph::Droplet,
            Kind::Preset => Glyph::Star,
            Kind::Battery => Glyph::Battery,
            Kind::Identify => Glyph::Eye,
            Kind::Scan => Glyph::Radar,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
struct View {
    image: Option<String>,
    title: Option<String>,
    state: Option<u16>,
    feedback: Option<Value>,
}

struct Registered {
    kind: Kind,
    settings: Settings,
    device_id: String,
    titles_enabled: bool,
    registered_at: Instant,
    last: Option<View>,
}

static REGISTRY: LazyLock<DashMap<String, Registered>> = LazyLock::new(DashMap::new);

pub fn register(instance: &Instance, settings: &Settings) {
    let Some(kind) = Kind::from_uuid(&instance.action_uuid) else {
        return;
    };
    REGISTRY
        .entry(instance.instance_id.clone())
        .and_modify(|entry| entry.settings = settings.clone())
        .or_insert_with(|| Registered {
            kind,
            settings: settings.clone(),
            device_id: instance.device_id.clone(),
            titles_enabled: true,
            registered_at: Instant::now(),
            last: None,
        });
}

pub fn update_settings(id: &str, settings: &Settings) {
    if let Some(mut entry) = REGISTRY.get_mut(id) {
        entry.settings = settings.clone();
    }
}

pub fn unregister(id: &str) {
    REGISTRY.remove(id);
}

pub fn unregister_device(device_id: &str) {
    REGISTRY.retain(|_, entry| entry.device_id != device_id);
}

pub fn has_instances() -> bool {
    !REGISTRY.is_empty()
}

pub fn battery_targets() -> Vec<String> {
    REGISTRY
        .iter()
        .filter(|entry| entry.kind == Kind::Battery && entry.settings.target.kind == "light")
        .map(|entry| entry.settings.target.id.clone())
        .collect()
}

pub fn note_title_event(id: &str, title: &str) {
    let Some(mut entry) = REGISTRY.get_mut(id) else {
        return;
    };
    let ours = entry
        .last
        .as_ref()
        .and_then(|view| view.title.clone())
        .unwrap_or_default();
    let just_registered = entry.registered_at.elapsed() < Duration::from_secs(1);

    if just_registered {
        let stale_own_title =
            !title.is_empty() && title != ours && looks_generated(title, &entry.settings);
        if stale_own_title && let Some(last) = entry.last.as_mut() {
            last.title = Some(title.to_string());
        }
        return;
    }

    if title.is_empty() {
        entry.titles_enabled = true;
        if let Some(last) = entry.last.as_mut() {
            last.title = None;
        }
    } else if title != ours {
        entry.titles_enabled = false;
    }
}

fn looks_generated(title: &str, settings: &Settings) -> bool {
    let fixed = ["Set up", "Missing", "Offline", "No daemon", "n/a", "..."];
    if fixed.contains(&title) || title == settings.label.trim() {
        return true;
    }
    let number = title.trim_end_matches(['%', 'K']);
    number.len() < title.len() && number.parse::<u32>().is_ok()
}

pub async fn render_all() {
    let ids: Vec<String> = REGISTRY.iter().map(|entry| entry.key().clone()).collect();
    for id in ids {
        render_one(&id).await;
    }
}

pub async fn render_one(id: &str) {
    let Some(instance) = openaction::get_instance(id.to_string()).await else {
        return;
    };
    let prepared = REGISTRY.get_mut(id).map(|mut entry| {
        let view = build_view(entry.kind, &entry.settings);
        let previous = entry.last.replace(view.clone()).unwrap_or_default();
        (view, previous, entry.titles_enabled)
    });
    let Some((view, previous, titles_enabled)) = prepared else {
        return;
    };

    if view.image != previous.image {
        let _ = instance.set_image(view.image.clone(), None).await;
    }
    if view.state != previous.state
        && let Some(state) = view.state
    {
        let _ = instance.set_state(state).await;
    }
    if titles_enabled && view.title != previous.title {
        let _ = instance.set_title(view.title.clone(), None).await;
    }
    if view.feedback != previous.feedback
        && let Some(feedback) = &view.feedback
    {
        let _ = instance.set_feedback(feedback).await;
    }
}

enum Status {
    NoDaemon,
    Unconfigured,
    Missing,
    Offline(Aggregate),
    Ready(Aggregate),
}

fn resolve_status(settings: &Settings) -> Status {
    if !daemon::is_up() {
        return Status::NoDaemon;
    }
    if !settings.target.is_configured() {
        return Status::Unconfigured;
    }
    match snapshot::read().aggregate(&settings.target) {
        None => Status::Missing,
        Some(agg) if !agg.reachable => Status::Offline(agg),
        Some(agg) => Status::Ready(agg),
    }
}

fn build_view(kind: Kind, settings: &Settings) -> View {
    if kind == Kind::Scan {
        return scan_view();
    }
    let status = resolve_status(settings);
    if kind.is_dial() {
        return dial_view(kind, &status);
    }
    match status {
        Status::NoDaemon => key(
            Tile::new(svg::DARK_RED, Glyph::Panel).slashed(),
            Some("No daemon"),
            Some(1),
        ),
        Status::Unconfigured => key(
            Tile::new(svg::DARK, Glyph::Sliders).dashed(),
            Some("Set up"),
            Some(1),
        ),
        Status::Missing => key(
            Tile::new(svg::DARK, Glyph::Sliders).dashed().slashed(),
            Some("Missing"),
            Some(1),
        ),
        Status::Offline(_) => key(
            Tile::new(svg::AMBER, kind.glyph()).slashed(),
            Some("Offline"),
            Some(1),
        ),
        Status::Ready(agg) => ready_key_view(kind, settings, &agg),
    }
}

fn scan_view() -> View {
    if daemon::is_up() {
        key(Tile::new(svg::DARK, Glyph::Radar), None, None)
    } else {
        key(
            Tile::new(svg::DARK_RED, Glyph::Panel).slashed(),
            Some("No daemon"),
            None,
        )
    }
}

fn key(tile: Tile, title: Option<&str>, state: Option<u16>) -> View {
    View {
        image: Some(tile.data_uri()),
        title: title.map(str::to_string),
        state,
        feedback: None,
    }
}

fn step_badge(step: f64) -> Badge {
    if step < 0.0 { Badge::Down } else { Badge::Up }
}

fn ready_key_view(kind: Kind, settings: &Settings, agg: &Aggregate) -> View {
    let brightness_fill =
        svg::brightness_color(quantize(agg.brightness as f64, 5.0) as u8, agg.any_on);
    let kelvin_fill = svg::kelvin_color(quantize(agg.kelvin as f64, 100.0) as u16, agg.any_on);

    match kind {
        Kind::Power => {
            let fill = if agg.any_on { svg::LIME } else { svg::SLATE };
            let state = if agg.any_on { 0 } else { 1 };
            key(Tile::new(fill, Glyph::Panel), None, Some(state))
        }
        Kind::BrightnessSet => {
            let title = format!("{}%", settings.value.unwrap_or(50.0).round() as u8);
            key(Tile::new(&brightness_fill, Glyph::Sun), Some(&title), None)
        }
        Kind::BrightnessAdjust => {
            let badge = step_badge(settings.step.unwrap_or(10.0));
            let title = format!("{}%", agg.brightness);
            key(
                Tile::new(&brightness_fill, Glyph::Sun).badge(badge),
                Some(&title),
                None,
            )
        }
        Kind::TemperatureSet => {
            let title = format!("{}K", settings.kelvin.unwrap_or(4500.0).round() as u16);
            key(
                Tile::new(&kelvin_fill, Glyph::Thermometer),
                Some(&title),
                None,
            )
        }
        Kind::TemperatureAdjust => {
            let badge = step_badge(settings.step.unwrap_or(250.0));
            let title = format!("{}K", agg.kelvin);
            key(
                Tile::new(&kelvin_fill, Glyph::Thermometer).badge(badge),
                Some(&title),
                None,
            )
        }
        Kind::Color => {
            let fill = match (agg.hue, agg.saturation) {
                (Some(hue), Some(saturation)) if agg.any_on => {
                    svg::hue_color(quantize(hue as f64, 15.0) as f32, saturation)
                }
                _ => svg::DARK.to_string(),
            };
            key(Tile::new(&fill, Glyph::Droplet), None, None)
        }
        Kind::Preset => preset_view(settings, agg),
        Kind::Battery => battery_view(settings),
        Kind::Identify => key(Tile::new(svg::DARK, Glyph::Eye), None, None),
        Kind::Scan => key(Tile::new(svg::DARK, Glyph::Radar), None, None),
        Kind::DialBrightness | Kind::DialTemperature => View::default(),
    }
}

fn preset_view(settings: &Settings, agg: &Aggregate) -> View {
    let preset_on = settings.on.unwrap_or(true);
    let brightness = settings.brightness.unwrap_or(50.0);
    let kelvin = settings.kelvin.unwrap_or(4500.0);
    let use_color = settings.use_color.unwrap_or(false) && settings.hue.is_some();

    let own_color = if use_color {
        svg::hue_color(
            settings.hue.unwrap_or(0.0) as f32,
            settings.saturation.unwrap_or(100.0) as f32,
        )
    } else {
        svg::kelvin_color(kelvin.round() as u16, true)
    };

    let brightness_close = (agg.brightness as f64 - brightness).abs() <= 3.0;
    let kelvin_close = use_color || (agg.kelvin as f64 - kelvin).abs() <= 100.0;
    let matches = agg.any_on == preset_on && (!preset_on || (brightness_close && kelvin_close));
    let fill = if matches {
        own_color
    } else {
        svg::dim(&own_color, 0.3)
    };

    let title = settings.label.trim();
    let title = if title.is_empty() { None } else { Some(title) };
    key(Tile::new(&fill, Glyph::Star), title, None)
}

fn battery_view(settings: &Settings) -> View {
    match snapshot::battery(&settings.target.id) {
        None => key(Tile::new(svg::DARK, Glyph::Battery), Some("..."), None),
        Some(None) => key(Tile::new(svg::DARK, Glyph::Battery), Some("n/a"), None),
        Some(Some(battery)) => {
            let level = battery.level.round().clamp(0.0, 100.0) as u8;
            let mut tile = Tile::new(svg::battery_color(level), Glyph::Battery).battery(level);
            if battery.is_charging() {
                tile = tile.badge(Badge::Bolt);
            }
            let title = format!("{level}%");
            key(tile, Some(&title), None)
        }
    }
}

fn dial_view(kind: Kind, status: &Status) -> View {
    let glyph = kind.glyph();
    let (title, value, indicator, icon) = match status {
        Status::NoDaemon => (
            "No daemon".to_string(),
            "--".to_string(),
            0.0,
            svg::dial_icon(Glyph::Panel, svg::AMBER, true),
        ),
        Status::Unconfigured => (
            "Set up".to_string(),
            "--".to_string(),
            0.0,
            svg::dial_icon(Glyph::Sliders, svg::MUTED, false),
        ),
        Status::Missing => (
            "Missing".to_string(),
            "--".to_string(),
            0.0,
            svg::dial_icon(Glyph::Sliders, svg::MUTED, true),
        ),
        Status::Offline(agg) => (
            format!("{} (offline)", agg.name),
            "--".to_string(),
            0.0,
            svg::dial_icon(glyph, svg::AMBER, true),
        ),
        Status::Ready(agg) => {
            let title = if agg.any_on {
                agg.name.clone()
            } else {
                format!("{} · off", agg.name)
            };
            match kind {
                Kind::DialTemperature => {
                    let color =
                        svg::kelvin_color(quantize(agg.kelvin as f64, 100.0) as u16, agg.any_on);
                    (
                        title,
                        format!("{}K", agg.kelvin),
                        agg.kelvin as f64,
                        svg::dial_icon(glyph, &color, false),
                    )
                }
                _ => {
                    let color = svg::brightness_color(
                        quantize(agg.brightness as f64, 5.0) as u8,
                        agg.any_on,
                    );
                    let icon_color = if agg.any_on {
                        color
                    } else {
                        svg::MUTED.to_string()
                    };
                    (
                        title,
                        format!("{}%", agg.brightness),
                        agg.brightness as f64,
                        svg::dial_icon(glyph, &icon_color, false),
                    )
                }
            }
        }
    };
    View {
        image: None,
        title: None,
        state: None,
        feedback: Some(json!({
            "title": title,
            "value": value,
            "indicator": indicator,
            "icon": svg::data_uri(&icon),
        })),
    }
}

fn quantize(value: f64, step: f64) -> f64 {
    (value / step).round() * step
}
