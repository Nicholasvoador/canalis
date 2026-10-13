//! Windows: the service is this same executable run as `panorama.exe service` without a window.
//! It is found again through the process ID it writes into the status file, and started at
//! login by a value under HKCU\...\CurrentVersion\Run.

use crate::hidden_command::hidden_command;
use crate::paths;
use std::process::Stdio;
use sysinfo::{Pid, ProcessesToUpdate, System};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "Panorama";
const EXECUTABLE_PREFIX: &str = "panorama";

pub fn start() -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| format!("cannot find panorama.exe: {error}"))?;
    hidden_command(executable)
        .arg("service")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|error| format!("cannot start the service: {error}"))
}

/// Ends the service process and waits for it, so the screen's port is free again.
pub fn stop() -> Result<(), String> {
    let pid = Pid::from_u32(service_process_id().ok_or("the service is not running")?);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let process = system.process(pid).filter(|process| is_panorama(&process.name().to_string_lossy())).ok_or("the service is not running")?;
    if !process.kill() {
        return Err("cannot stop the service".into());
    }
    process.wait();
    Ok(())
}

/// Starts the service whether or not it was running before.
pub fn restart() -> Result<(), String> {
    let _was_not_running = stop();
    start()
}

pub fn is_login_start_enabled() -> Result<bool, String> {
    Ok(run_key()?.get_value::<String, _>(RUN_VALUE).is_ok())
}

pub fn set_login_start(enabled: bool) -> Result<(), String> {
    let key = run_key()?;
    if enabled {
        key.set_value(RUN_VALUE, &login_command()?).map_err(|error| format!("cannot turn on start at login: {error}"))
    } else {
        key.delete_value(RUN_VALUE).or_else(|error| if error.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(error) }).map_err(|error| format!("cannot turn off start at login: {error}"))
    }
}

fn run_key() -> Result<RegKey, String> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE).map_err(|error| format!("cannot open the Run registry key: {error}"))
}

fn login_command() -> Result<String, String> {
    let executable = std::env::current_exe().map_err(|error| format!("cannot find panorama.exe: {error}"))?;
    Ok(format!("\"{}\" service", executable.display()))
}

fn service_process_id() -> Option<u32> {
    let status: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(paths::status_path()).ok()?).ok()?;
    status["pid"].as_u64().map(|pid| pid as u32)
}

fn is_panorama(process_name: &str) -> bool {
    process_name.to_lowercase().starts_with(EXECUTABLE_PREFIX)
}
