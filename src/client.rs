use std::fmt;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use ureq::Agent;

use crate::settings::{self, Target};

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Health {
    #[serde(default)]
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Light {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub alias: Option<String>,
    pub enabled: bool,
    pub reachable: bool,
    pub on: bool,
    pub brightness: u8,
    pub kelvin: u16,
    #[serde(default)]
    pub hue: Option<f32>,
    #[serde(default)]
    pub saturation: Option<f32>,
    #[serde(default)]
    pub color_capable: bool,
    #[serde(default)]
    pub product: Option<String>,
}

impl Light {
    pub fn display_name(&self) -> &str {
        let alias = self.alias.as_deref().map(str::trim).unwrap_or("");
        if alias.is_empty() { &self.name } else { alias }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Group {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct UpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kelvin: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saturation: Option<f32>,
}

impl UpdateRequest {
    pub fn merge(&mut self, newer: &UpdateRequest) {
        self.on = newer.on.or(self.on);
        self.brightness = newer.brightness.or(self.brightness);
        self.kelvin = newer.kelvin.or(self.kelvin);
        self.hue = newer.hue.or(self.hue);
        self.saturation = newer.saturation.or(self.saturation);
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct TargetResult {
    pub id: String,
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub state: Option<Light>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct UpdateResponse {
    pub ok: bool,
    #[serde(default)]
    pub results: Vec<TargetResult>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Battery {
    #[serde(default)]
    pub level: f32,
    #[serde(default)]
    pub status: u8,
}

impl Battery {
    pub fn is_charging(&self) -> bool {
        self.status == 2 || self.status == 3
    }
}

#[derive(Serialize)]
struct RefreshRequest {
    timeout: u64,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: String,
}

#[derive(Debug, Clone)]
pub enum ClientError {
    Unreachable(String),
    Status(u16, String),
    Decode(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::Unreachable(why) => write!(f, "daemon unreachable: {why}"),
            ClientError::Status(code, why) => write!(f, "HTTP {code}: {why}"),
            ClientError::Decode(why) => write!(f, "bad response: {why}"),
        }
    }
}

type Response = ureq::http::Response<ureq::Body>;

pub struct Client {
    agent: Agent,
    base: String,
}

pub fn current() -> Client {
    Client::new(settings::global().port)
}

pub fn encode_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

impl Client {
    pub fn new(port: u16) -> Client {
        let config = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(2)))
            .http_status_as_error(false)
            .build();
        Client {
            agent: Agent::new_with_config(config),
            base: format!("http://127.0.0.1:{port}/v1"),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn target_path(target: &Target) -> String {
        match target.kind.as_str() {
            "light" => format!("/lights/{}", encode_segment(&target.id)),
            "group" => format!("/groups/{}", encode_segment(&target.id)),
            _ => "/all".to_string(),
        }
    }

    pub fn health(&self) -> Result<Health, ClientError> {
        parse(self.agent.get(self.url("/health")).call())
    }

    pub fn states(&self) -> Result<Vec<Light>, ClientError> {
        parse(self.agent.get(self.url("/lights/states")).call())
    }

    pub fn groups(&self) -> Result<Vec<Group>, ClientError> {
        parse(self.agent.get(self.url("/groups")).call())
    }

    pub fn update(
        &self,
        target: &Target,
        body: &UpdateRequest,
    ) -> Result<UpdateResponse, ClientError> {
        let url = self.url(&Self::target_path(target));
        parse(self.agent.put(url).send_json(body))
    }

    pub fn identify(&self, id: &str) -> Result<(), ClientError> {
        let url = self.url(&format!("/lights/{}/identify", encode_segment(id)));
        parse::<serde_json::Value>(self.agent.post(url).send_empty()).map(|_| ())
    }

    pub fn refresh(&self, timeout_secs: u64) -> Result<(), ClientError> {
        let body = RefreshRequest {
            timeout: timeout_secs,
        };
        let result = self
            .agent
            .post(self.url("/lights/refresh"))
            .send_json(&body);
        parse::<serde_json::Value>(result).map(|_| ())
    }

    pub fn battery(&self, id: &str) -> Result<Option<Battery>, ClientError> {
        let url = self.url(&format!("/lights/{}/battery", encode_segment(id)));
        match parse::<Battery>(self.agent.get(url).call()) {
            Ok(battery) => Ok(Some(battery)),
            Err(ClientError::Status(404, _)) => Ok(None),
            Err(other) => Err(other),
        }
    }
}

fn parse<T: DeserializeOwned>(result: Result<Response, ureq::Error>) -> Result<T, ClientError> {
    let mut response = result.map_err(|e| ClientError::Unreachable(e.to_string()))?;
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| ClientError::Decode(e.to_string()))?;
    let is_error = status >= 400 && status != 502;
    if is_error {
        let message = serde_json::from_str::<ErrorBody>(&text)
            .map(|body| body.error)
            .unwrap_or(text);
        return Err(ClientError::Status(status, message));
    }
    serde_json::from_str(&text).map_err(|e| ClientError::Decode(e.to_string()))
}
