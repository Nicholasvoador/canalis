//! Slow work (adb pushes, service control) runs on a background thread so the window stays responsive.

use crate::crop::MediaUpload;
use crate::hardware_monitor;
use crate::media::AdbScreen;
use crate::service_control::{self, ServiceAction};

pub enum JobOutcome {
    MediaAdded(Vec<String>),
    Done(String),
    Failed(String),
    NothingToReport,
}

/// A piece of slow work and the label shown while it runs.
pub struct Job {
    pub label: &'static str,
    pub work: Box<dyn FnOnce() -> JobOutcome + Send>,
}

impl Job {
    pub fn add_media(uploads: Vec<MediaUpload>) -> Self {
        let label = if uploads.iter().any(|upload| upload.crop.is_some()) { "Cropping and copying media to the screen…" } else { "Copying media to the screen…" };
        Self::new(label, move || {
            let screen = AdbScreen::new();
            match uploads.iter().map(|upload| upload.copy_to_screen(&screen)).collect::<Result<Vec<_>, _>>() {
                Ok(names) => JobOutcome::MediaAdded(names),
                Err(error) => JobOutcome::Failed(error),
            }
        })
    }

    /// Starts the service and LibreHardwareMonitor if either isn't running.
    pub fn start_companions() -> Self {
        Self::new("Starting the service…", || {
            let started: Result<Vec<String>, String> = [service_control::ensure_running(), hardware_monitor::ensure_running()].into_iter().filter_map(Result::transpose).collect();
            match started {
                Ok(messages) if messages.is_empty() => JobOutcome::NothingToReport,
                Ok(messages) => JobOutcome::Done(messages.join(", ")),
                Err(error) => JobOutcome::Failed(error),
            }
        })
    }

    pub fn service_command(action: ServiceAction) -> Self {
        Self::new("Talking to the service…", move || finished(action.run()))
    }

    pub fn set_login_start(enabled: bool) -> Self {
        let message = if enabled { "The service will start when you log in" } else { "The service will no longer start when you log in" };
        Self::new("Updating start at login…", move || finished(service_control::set_login_start(enabled).map(|()| message.to_string())))
    }

    fn new(label: &'static str, work: impl FnOnce() -> JobOutcome + Send + 'static) -> Self {
        Self { label, work: Box::new(work) }
    }
}

fn finished(result: Result<String, String>) -> JobOutcome {
    match result {
        Ok(message) => JobOutcome::Done(message),
        Err(error) => JobOutcome::Failed(error),
    }
}
