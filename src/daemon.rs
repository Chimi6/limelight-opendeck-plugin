use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use crate::{client, settings};

pub const BUNDLED_VERSION: &str = "0.2.0";

#[derive(Clone, Debug, PartialEq)]
pub enum DaemonStatus {
    Unknown,
    Up(String),
    Starting,
    Down(String),
}

static STATUS: LazyLock<RwLock<DaemonStatus>> =
    LazyLock::new(|| RwLock::new(DaemonStatus::Unknown));
static START_ATTEMPTED: AtomicBool = AtomicBool::new(false);

pub fn status() -> DaemonStatus {
    STATUS.read().unwrap().clone()
}

pub fn is_up() -> bool {
    matches!(status(), DaemonStatus::Up(_))
}

pub fn version() -> String {
    match status() {
        DaemonStatus::Up(version) => version,
        _ => String::new(),
    }
}

pub fn set_status(new_status: DaemonStatus) -> bool {
    let mut current = STATUS.write().unwrap();
    if *current == new_status {
        return false;
    }
    log::info!("daemon status: {:?} -> {:?}", *current, new_status);
    *current = new_status;
    true
}

pub fn mark_down(reason: String) -> bool {
    set_status(DaemonStatus::Down(reason))
}

pub fn allow_start_again() {
    START_ATTEMPTED.store(false, Ordering::SeqCst);
}

async fn probe() -> Option<String> {
    let client = client::current();
    let health = tokio::task::spawn_blocking(move || client.health()).await;
    match health {
        Ok(Ok(health)) => Some(health.version),
        _ => None,
    }
}

pub async fn ensure_running() -> DaemonStatus {
    if let Some(version) = probe().await {
        if version != BUNDLED_VERSION {
            log::warn!(
                "running keylightd is v{version}, bundled is v{BUNDLED_VERSION}; using the running one"
            );
        }
        allow_start_again();
        set_status(DaemonStatus::Up(version));
        return status();
    }

    let may_start =
        settings::global().start_daemon && !START_ATTEMPTED.swap(true, Ordering::SeqCst);
    if !may_start {
        set_status(DaemonStatus::Down("keylightd is not reachable".to_string()));
        return status();
    }

    set_status(DaemonStatus::Starting);
    if let Err(why) = spawn_daemon() {
        set_status(DaemonStatus::Down(format!(
            "could not start keylightd: {why}"
        )));
        return status();
    }

    for _ in 0..8 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if let Some(version) = probe().await {
            log::info!("started bundled keylightd v{version}");
            set_status(DaemonStatus::Up(version));
            return status();
        }
    }
    set_status(DaemonStatus::Down("keylightd did not start".to_string()));
    status()
}

fn daemon_path() -> PathBuf {
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.parent()
                .map(|dir| dir.join(format!("keylightd-{}", env!("TARGET_TRIPLE"))))
        })
        .filter(|path| path.exists());
    bundled.unwrap_or_else(|| PathBuf::from("keylightd"))
}

fn spawn_daemon() -> Result<(), String> {
    let path = daemon_path();
    log::info!("starting keylightd from {}", path.display());
    let mut command = Command::new(&path);
    command
        .arg("serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    command.spawn().map(|_| ()).map_err(|e| e.to_string())
}
