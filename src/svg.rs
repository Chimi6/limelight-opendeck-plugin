use base64::{Engine as _, engine::general_purpose::STANDARD};

pub const LIME: &str = "#A8FF60";
pub const SLATE: &str = "#2B3443";
pub const AMBER: &str = "#B0552A";
pub const DARK_RED: &str = "#5A1F1F";
pub const DARK: &str = "#1E232B";
pub const MUTED: &str = "#6B7684";
const DIM_LIME: &str = "#1E2A1A";
const COOL: &str = "#6AA8FF";
const NEUTRAL: &str = "#EAF2FF";
const WARM: &str = "#F4B37A";
const GLYPH_DARK: &str = "#182010";
const GLYPH_LIGHT: &str = "#E6EAF0";
const SLASH: &str = "#FF5C5C";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Glyph {
    Panel,
    Sun,
    Thermometer,
    Droplet,
    Star,
    Battery,
    Eye,
    Radar,
    Sliders,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Badge {
    Up,
    Down,
    Bolt,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Tile {
    pub fill: String,
    pub dashed: bool,
    pub glyph: Glyph,
    pub slash: bool,
    pub badge: Option<Badge>,
    pub battery_level: Option<u8>,
}

impl Tile {
    pub fn new(fill: &str, glyph: Glyph) -> Tile {
        Tile {
            fill: fill.to_string(),
            dashed: false,
            glyph,
            slash: false,
            badge: None,
            battery_level: None,
        }
    }

    pub fn dashed(mut self) -> Tile {
        self.dashed = true;
        self
    }

    pub fn slashed(mut self) -> Tile {
        self.slash = true;
        self
    }

    pub fn badge(mut self, badge: Badge) -> Tile {
        self.badge = Some(badge);
        self
    }

    pub fn battery(mut self, level: u8) -> Tile {
        self.battery_level = Some(level);
        self
    }

    pub fn svg(&self) -> String {
        let glyph_color = contrast_color(&self.fill);
        let mut parts = vec![
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">"#.to_string(),
            format!(r#"<rect width="144" height="144" rx="28" fill="{}"/>"#, self.fill),
        ];
        if self.dashed {
            parts.push(format!(
                r#"<rect x="6" y="6" width="132" height="132" rx="24" fill="none" stroke="{MUTED}" stroke-width="4" stroke-dasharray="14 10"/>"#
            ));
        }
        parts.push(format!(
            r#"<g transform="translate(72 62) scale(3.2) translate(-12 -12)" fill="none" stroke="{glyph_color}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{}</g>"#,
            glyph_paths(self.glyph, self.battery_level, glyph_color)
        ));
        if let Some(badge) = self.badge {
            parts.push(badge_svg(badge, glyph_color, &self.fill));
        }
        if self.slash {
            parts.push(slash_svg(28.0, 116.0, 10.0));
        }
        parts.push("</svg>".to_string());
        parts.concat()
    }

    pub fn data_uri(&self) -> String {
        data_uri(&self.svg())
    }
}

pub fn dial_icon(glyph: Glyph, color: &str, slashed: bool) -> String {
    let slash = if slashed {
        slash_svg(10.0, 38.0, 4.0)
    } else {
        String::new()
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48"><g transform="translate(24 24) scale(1.7) translate(-12 -12)" fill="none" stroke="{color}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{}</g>{slash}</svg>"#,
        glyph_paths(glyph, None, color)
    )
}

pub fn data_uri(svg: &str) -> String {
    format!("data:image/svg+xml;base64,{}", STANDARD.encode(svg))
}

fn glyph_paths(glyph: Glyph, battery_level: Option<u8>, color: &str) -> String {
    match glyph {
        Glyph::Panel => r#"<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M12 16v5M8 21h8"/>"#.to_string(),
        Glyph::Sun => r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/>"#.to_string(),
        Glyph::Thermometer => r#"<path d="M14 14.76V3.5a2.5 2.5 0 0 0-5 0v11.26a4.5 4.5 0 1 0 5 0z"/>"#.to_string(),
        Glyph::Droplet => r#"<path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"/>"#.to_string(),
        Glyph::Star => r#"<path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01z"/>"#.to_string(),
        Glyph::Battery => {
            let level = battery_level.unwrap_or(0).min(100) as f32;
            let width = 14.0 * level / 100.0;
            format!(
                r#"<rect x="2" y="7" width="18" height="10" rx="2"/><path d="M22 11v2"/><rect x="4" y="9" width="{width:.1}" height="6" rx="1" fill="{color}" stroke="none"/>"#
            )
        }
        Glyph::Eye => r#"<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>"#.to_string(),
        Glyph::Radar => r#"<path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>"#.to_string(),
        Glyph::Sliders => r#"<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/>"#.to_string(),
    }
}

fn badge_svg(badge: Badge, circle: &str, stroke: &str) -> String {
    let path = match badge {
        Badge::Up => r#"<path d="M6 15l6-6 6 6"/>"#,
        Badge::Down => r#"<path d="M6 9l6 6 6-6"/>"#,
        Badge::Bolt => r#"<path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>"#,
    };
    format!(
        r#"<circle cx="118" cy="26" r="18" fill="{circle}"/><g transform="translate(118 26) scale(1.15) translate(-12 -12)" fill="none" stroke="{stroke}" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">{path}</g>"#
    )
}

fn slash_svg(from: f32, to: f32, width: f32) -> String {
    let outline = width * 1.6;
    format!(
        r##"<path d="M{from} {from}L{to} {to}" stroke="#000" stroke-opacity="0.35" stroke-width="{outline}" stroke-linecap="round"/><path d="M{from} {from}L{to} {to}" stroke="{SLASH}" stroke-width="{width}" stroke-linecap="round"/>"##
    )
}

fn parse_hex(hex: &str) -> (f32, f32, f32) {
    let digits = hex.trim_start_matches('#');
    let channel = |index: usize| {
        u8::from_str_radix(digits.get(index..index + 2).unwrap_or("00"), 16).unwrap_or(0) as f32
            / 255.0
    };
    (channel(0), channel(2), channel(4))
}

fn to_hex(r: f32, g: f32, b: f32) -> String {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", byte(r), byte(g), byte(b))
}

fn to_linear(channel: f32) -> f32 {
    channel.powf(2.2)
}

fn to_srgb(channel: f32) -> f32 {
    channel.max(0.0).powf(1.0 / 2.2)
}

pub fn mix(from: &str, to: &str, t: f32) -> String {
    let t = t.clamp(0.0, 1.0);
    let (r1, g1, b1) = parse_hex(from);
    let (r2, g2, b2) = parse_hex(to);
    let blend = |a: f32, b: f32| to_srgb(to_linear(a) + (to_linear(b) - to_linear(a)) * t);
    to_hex(blend(r1, r2), blend(g1, g2), blend(b1, b2))
}

pub fn dim(hex: &str, factor: f32) -> String {
    let (r, g, b) = parse_hex(hex);
    let scale = |channel: f32| to_srgb(to_linear(channel) * factor);
    to_hex(scale(r), scale(g), scale(b))
}

fn luminance(hex: &str) -> f32 {
    let (r, g, b) = parse_hex(hex);
    0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
}

fn contrast_color(fill: &str) -> &'static str {
    if luminance(fill) > 0.3 {
        GLYPH_DARK
    } else {
        GLYPH_LIGHT
    }
}

pub fn kelvin_color(kelvin: u16, on: bool) -> String {
    let kelvin = kelvin.clamp(2900, 7000) as f32;
    let color = if kelvin >= 5000.0 {
        mix(NEUTRAL, COOL, (kelvin - 5000.0) / 2000.0)
    } else {
        mix(WARM, NEUTRAL, (kelvin - 2900.0) / 2100.0)
    };
    if on { color } else { dim(&color, 0.35) }
}

pub fn brightness_color(brightness: u8, on: bool) -> String {
    if !on {
        return SLATE.to_string();
    }
    mix(DIM_LIME, LIME, brightness.min(100) as f32 / 100.0)
}

pub fn hue_color(hue: f32, saturation: f32) -> String {
    let h = hue.rem_euclid(360.0) / 60.0;
    let s = (saturation / 100.0).clamp(0.0, 1.0);
    let v = 0.85;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    to_hex(r + m, g + m, b + m)
}

pub fn battery_color(level: u8) -> &'static str {
    match level {
        50..=100 => LIME,
        15..=49 => "#D9822B",
        _ => "#C0392B",
    }
}

#[allow(dead_code)]
pub fn static_icons() -> Vec<(&'static str, String)> {
    let list = |glyph: Glyph| Tile::new("#333B48", glyph);
    vec![
        ("plugin/icon.svg", Tile::new(LIME, Glyph::Panel).svg()),
        ("category/icon.svg", Tile::new(LIME, Glyph::Panel).svg()),
        ("actions/power.svg", list(Glyph::Panel).svg()),
        ("actions/brightness-set.svg", list(Glyph::Sun).svg()),
        (
            "actions/brightness-adjust.svg",
            list(Glyph::Sun).badge(Badge::Up).svg(),
        ),
        (
            "actions/temperature-set.svg",
            list(Glyph::Thermometer).svg(),
        ),
        (
            "actions/temperature-adjust.svg",
            list(Glyph::Thermometer).badge(Badge::Up).svg(),
        ),
        ("actions/color.svg", list(Glyph::Droplet).svg()),
        ("actions/preset.svg", list(Glyph::Star).svg()),
        (
            "actions/battery.svg",
            list(Glyph::Battery).battery(75).svg(),
        ),
        ("actions/identify.svg", list(Glyph::Eye).svg()),
        ("actions/scan.svg", list(Glyph::Radar).svg()),
        ("actions/dial-brightness.svg", list(Glyph::Sun).svg()),
        (
            "actions/dial-temperature.svg",
            list(Glyph::Thermometer).svg(),
        ),
        ("keys/power-on.svg", Tile::new(LIME, Glyph::Panel).svg()),
        ("keys/power-off.svg", Tile::new(SLATE, Glyph::Panel).svg()),
        (
            "keys/brightness.svg",
            Tile::new(&brightness_color(60, true), Glyph::Sun).svg(),
        ),
        (
            "keys/brightness-up.svg",
            Tile::new(&brightness_color(60, true), Glyph::Sun)
                .badge(Badge::Up)
                .svg(),
        ),
        (
            "keys/brightness-down.svg",
            Tile::new(&brightness_color(60, true), Glyph::Sun)
                .badge(Badge::Down)
                .svg(),
        ),
        (
            "keys/temperature.svg",
            Tile::new(&kelvin_color(4500, true), Glyph::Thermometer).svg(),
        ),
        (
            "keys/temperature-cooler.svg",
            Tile::new(&kelvin_color(4500, true), Glyph::Thermometer)
                .badge(Badge::Up)
                .svg(),
        ),
        (
            "keys/temperature-warmer.svg",
            Tile::new(&kelvin_color(4500, true), Glyph::Thermometer)
                .badge(Badge::Down)
                .svg(),
        ),
        (
            "keys/color.svg",
            Tile::new(&hue_color(300.0, 80.0), Glyph::Droplet).svg(),
        ),
        (
            "keys/preset.svg",
            Tile::new(&kelvin_color(4200, true), Glyph::Star).svg(),
        ),
        (
            "keys/battery.svg",
            Tile::new(LIME, Glyph::Battery).battery(80).svg(),
        ),
        ("keys/identify.svg", Tile::new(DARK, Glyph::Eye).svg()),
        ("keys/scan.svg", Tile::new(DARK, Glyph::Radar).svg()),
        (
            "keys/offline.svg",
            Tile::new(AMBER, Glyph::Panel).slashed().svg(),
        ),
        (
            "keys/no-daemon.svg",
            Tile::new(DARK_RED, Glyph::Panel).slashed().svg(),
        ),
        (
            "keys/setup.svg",
            Tile::new(DARK, Glyph::Sliders).dashed().svg(),
        ),
        ("dial/brightness.svg", dial_icon(Glyph::Sun, LIME, false)),
        (
            "dial/temperature.svg",
            dial_icon(Glyph::Thermometer, NEUTRAL, false),
        ),
        ("dial/offline.svg", dial_icon(Glyph::Panel, AMBER, true)),
    ]
}
