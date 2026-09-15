use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::client::{Battery, Group, Light, TargetResult, UpdateRequest};
use crate::settings::Target;

const USER_GRACE: Duration = Duration::from_millis(1500);

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub lights: Vec<Light>,
    pub groups: Vec<Group>,
    pub fetched_at: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Aggregate {
    pub name: String,
    pub any_on: bool,
    pub brightness: u8,
    pub kelvin: u16,
    pub hue: Option<f32>,
    pub saturation: Option<f32>,
    pub reachable: bool,
    pub color_capable: bool,
}

static SNAPSHOT: LazyLock<RwLock<Snapshot>> = LazyLock::new(|| RwLock::new(Snapshot::default()));
static USER_TOUCHED: LazyLock<Mutex<HashMap<String, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static BATTERY: LazyLock<Mutex<HashMap<String, Option<Battery>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn read() -> Snapshot {
    SNAPSHOT.read().unwrap().clone()
}

pub fn is_stale(max_age: Duration) -> bool {
    match SNAPSHOT.read().unwrap().fetched_at {
        Some(at) => at.elapsed() > max_age,
        None => true,
    }
}

pub fn touch(target: &Target) {
    USER_TOUCHED
        .lock()
        .unwrap()
        .insert(target.key(), Instant::now());
}

fn recently_touched_targets() -> Vec<Target> {
    let mut touched = USER_TOUCHED.lock().unwrap();
    touched.retain(|_, at| at.elapsed() < USER_GRACE);
    touched
        .keys()
        .filter_map(|key| {
            let (kind, id) = key.split_once(':')?;
            Some(Target {
                kind: kind.to_string(),
                id: id.to_string(),
            })
        })
        .collect()
}

impl Snapshot {
    pub fn member_ids(&self, target: &Target) -> Vec<String> {
        match target.kind.as_str() {
            "light" => vec![target.id.clone()],
            "group" => self
                .groups
                .iter()
                .find(|group| group.name == target.id)
                .map(|group| group.members.clone())
                .unwrap_or_default(),
            "all" => self.lights.iter().map(|light| light.id.clone()).collect(),
            _ => Vec::new(),
        }
    }

    pub fn members(&self, target: &Target) -> Vec<&Light> {
        let ids = self.member_ids(target);
        self.lights
            .iter()
            .filter(|light| ids.contains(&light.id))
            .collect()
    }

    fn target_exists(&self, target: &Target) -> bool {
        match target.kind.as_str() {
            "light" => self.lights.iter().any(|light| light.id == target.id),
            "group" => self.groups.iter().any(|group| group.name == target.id),
            "all" => true,
            _ => false,
        }
    }

    pub fn aggregate(&self, target: &Target) -> Option<Aggregate> {
        if !self.target_exists(target) {
            return None;
        }
        let members = self.members(target);
        let name = match target.kind.as_str() {
            "light" => members
                .first()
                .map(|light| light.display_name().to_string())
                .unwrap_or_default(),
            "group" => target.id.clone(),
            _ => "All lights".to_string(),
        };
        let reachable: Vec<&&Light> = members.iter().filter(|light| light.reachable).collect();
        let basis: Vec<&&Light> = if reachable.is_empty() {
            members.iter().collect()
        } else {
            reachable.clone()
        };
        let count = basis.len().max(1) as u32;
        let brightness = basis
            .iter()
            .map(|light| light.brightness as u32)
            .sum::<u32>()
            / count;
        let kelvin = basis.iter().map(|light| light.kelvin as u32).sum::<u32>() / count;
        let first_color = basis.iter().find(|light| light.hue.is_some());
        Some(Aggregate {
            name,
            any_on: reachable.iter().any(|light| light.on),
            brightness: brightness as u8,
            kelvin: kelvin as u16,
            hue: first_color.and_then(|light| light.hue),
            saturation: first_color.and_then(|light| light.saturation),
            reachable: !reachable.is_empty(),
            color_capable: members.iter().any(|light| light.color_capable),
        })
    }
}

pub fn replace_from_poll(lights: Vec<Light>, groups: Vec<Group>) {
    let mut snapshot = SNAPSHOT.write().unwrap();
    let mut protected: HashSet<String> = HashSet::new();
    for target in recently_touched_targets() {
        protected.extend(snapshot.member_ids(&target));
    }
    let merged = lights
        .into_iter()
        .map(|fresh| {
            let keep_old = protected.contains(&fresh.id);
            let old = snapshot.lights.iter().find(|light| light.id == fresh.id);
            match (keep_old, old) {
                (true, Some(old)) => old.clone(),
                _ => fresh,
            }
        })
        .collect();
    snapshot.lights = merged;
    snapshot.groups = groups;
    snapshot.fetched_at = Some(Instant::now());
}

pub fn apply_results(results: &[TargetResult]) {
    let mut snapshot = SNAPSHOT.write().unwrap();
    for result in results {
        let Some(state) = &result.state else { continue };
        if let Some(light) = snapshot
            .lights
            .iter_mut()
            .find(|light| light.id == state.id)
        {
            *light = state.clone();
        }
    }
}

pub fn optimistic(target: &Target, patch: &UpdateRequest) {
    let mut snapshot = SNAPSHOT.write().unwrap();
    let ids = snapshot.member_ids(target);
    for light in snapshot
        .lights
        .iter_mut()
        .filter(|light| ids.contains(&light.id))
    {
        if let Some(on) = patch.on {
            light.on = on == 1;
        }
        if let Some(brightness) = patch.brightness {
            light.brightness = brightness;
        }
        if let Some(kelvin) = patch.kelvin {
            light.kelvin = kelvin;
        }
        if patch.hue.is_some() {
            light.hue = patch.hue;
        }
        if patch.saturation.is_some() {
            light.saturation = patch.saturation;
        }
    }
}

pub fn mark_unreachable(target: &Target) {
    let mut snapshot = SNAPSHOT.write().unwrap();
    let ids = snapshot.member_ids(target);
    for light in snapshot
        .lights
        .iter_mut()
        .filter(|light| ids.contains(&light.id))
    {
        light.reachable = false;
    }
}

pub fn set_battery(id: &str, battery: Option<Battery>) {
    BATTERY.lock().unwrap().insert(id.to_string(), battery);
}

pub fn battery(id: &str) -> Option<Option<Battery>> {
    BATTERY.lock().unwrap().get(id).cloned()
}
