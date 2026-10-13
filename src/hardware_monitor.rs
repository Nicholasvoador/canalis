//! LibreHardwareMonitor provides temperatures, fan speeds and CPU power on Windows. The installer
//! sets it up as a scheduled task that runs with administrator rights at login, and running that
//! task again starts it without a UAC prompt. Linux reads its sensors directly: nothing to start.

/// Starts LibreHardwareMonitor when it isn't running; returns a message when it was started.
#[cfg(not(windows))]
pub fn ensure_running() -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(windows)]
pub use windows::ensure_running;

#[cfg(windows)]
mod windows {
    use crate::hidden_command::hidden_command;
    use std::ffi::OsStr;
    use std::process::Stdio;
    use sysinfo::{ProcessesToUpdate, System};

    const PROCESS_NAME: &str = "LibreHardwareMonitor.exe";
    /// The login task, under its name from before the app was renamed from Panorama too.
    const TASK_NAMES: [&str; 2] = ["Canalis - LibreHardwareMonitor", "Panorama - LibreHardwareMonitor"];

    pub fn ensure_running() -> Result<Option<String>, String> {
        if is_running() {
            return Ok(None);
        }
        if TASK_NAMES.iter().any(|task| run_task(task)) {
            Ok(Some("LibreHardwareMonitor started".into()))
        } else {
            Err("LibreHardwareMonitor isn't running and its login task is missing; run install-windows.ps1 again".into())
        }
    }

    fn is_running() -> bool {
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, true);
        system.processes_by_exact_name(OsStr::new(PROCESS_NAME)).next().is_some()
    }

    fn run_task(name: &str) -> bool {
        hidden_command("schtasks").args(["/Run", "/TN", name]).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|status| status.success())
    }
}
