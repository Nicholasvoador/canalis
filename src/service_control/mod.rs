//! Starting and stopping the background service, and whether it starts at login.
//! Linux drives the systemd user unit; Windows runs `canalis service` itself and uses the
//! Run registry key, the same mechanism KANALI uses for "Startup with Windows".

#[cfg(not(windows))]
mod systemd;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
use systemd as platform;
#[cfg(windows)]
use windows as platform;

pub use platform::{is_login_start_enabled, set_login_start};

/// Starts the service when it isn't running; returns a message when it was started.
pub fn ensure_running() -> Result<Option<String>, String> {
    if platform::is_running() {
        return Ok(None);
    }
    platform::start().map(|()| Some("Service started".into()))
}

#[derive(Clone, Copy)]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
}

impl ServiceAction {
    /// Performs the action and returns a message for the user.
    pub fn run(self) -> Result<String, String> {
        match self {
            ServiceAction::Start => platform::start().map(|()| "Service started".into()),
            ServiceAction::Stop => platform::stop().map(|()| "Service stopped".into()),
            ServiceAction::Restart => platform::restart().map(|()| "Service restarted".into()),
        }
    }
}
