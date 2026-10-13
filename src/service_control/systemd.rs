//! Linux: the `canalis.service` systemd user unit, enabled to start at login.

use crate::hidden_command::hidden_command;

const UNIT: &str = "canalis.service";

pub fn start() -> Result<(), String> {
    systemctl(&["start", UNIT]).map(drop)
}

pub fn stop() -> Result<(), String> {
    systemctl(&["stop", UNIT]).map(drop)
}

pub fn restart() -> Result<(), String> {
    systemctl(&["restart", UNIT]).map(drop)
}

pub fn is_login_start_enabled() -> Result<bool, String> {
    let output = hidden_command("systemctl").args(["--user", "is-enabled", UNIT]).output().map_err(cannot_run)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim() == "enabled")
}

pub fn set_login_start(enabled: bool) -> Result<(), String> {
    systemctl(&[if enabled { "enable" } else { "disable" }, UNIT]).map(drop)
}

fn systemctl(arguments: &[&str]) -> Result<String, String> {
    let output = hidden_command("systemctl").arg("--user").args(arguments).output().map_err(cannot_run)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn cannot_run(error: std::io::Error) -> String {
    format!("cannot run systemctl: {error}")
}
