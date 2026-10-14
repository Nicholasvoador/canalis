//! Slow work (adb pushes, service control) runs on a worker thread so the window stays responsive.

use crate::crop::MediaUpload;
use crate::hardware_monitor;
use crate::media::AdbScreen;
use crate::service_control::{self, ServiceAction};
use std::sync::mpsc::{Receiver, channel};

pub enum JobOutcome {
    MediaAdded(Vec<String>),
    Done(String),
    Failed(String),
    NothingToReport,
}

#[derive(Default)]
pub struct Jobs {
    running: Option<(String, Receiver<JobOutcome>)>,
}

impl Jobs {
    pub fn busy_label(&self) -> Option<&str> {
        self.running.as_ref().map(|(label, _)| label.as_str())
    }

    pub fn add_media(&mut self, uploads: Vec<MediaUpload>, ctx: &eframe::egui::Context) {
        let label = if uploads.iter().any(|upload| upload.crop.is_some()) { "Cropping and copying media to the screen…" } else { "Copying media to the screen…" };
        self.spawn(label, ctx, move || {
            let screen = AdbScreen::new();
            match uploads.iter().map(|upload| upload.copy_to_screen(&screen)).collect::<Result<Vec<_>, _>>() {
                Ok(names) => JobOutcome::MediaAdded(names),
                Err(error) => JobOutcome::Failed(error),
            }
        });
    }

    /// Starts the service and LibreHardwareMonitor if either isn't running.
    pub fn start_companions(&mut self, ctx: &eframe::egui::Context) {
        self.spawn("Starting the service…", ctx, || {
            let started: Result<Vec<String>, String> = [service_control::ensure_running(), hardware_monitor::ensure_running()].into_iter().filter_map(Result::transpose).collect();
            match started {
                Ok(messages) if messages.is_empty() => JobOutcome::NothingToReport,
                Ok(messages) => JobOutcome::Done(messages.join(", ")),
                Err(error) => JobOutcome::Failed(error),
            }
        });
    }

    pub fn service_command(&mut self, action: ServiceAction, ctx: &eframe::egui::Context) {
        self.spawn("Talking to the service…", ctx, move || finished(action.run()));
    }

    pub fn set_login_start(&mut self, enabled: bool, ctx: &eframe::egui::Context) {
        let message = if enabled { "The service will start when you log in" } else { "The service will no longer start when you log in" };
        self.spawn("Updating start at login…", ctx, move || finished(service_control::set_login_start(enabled).map(|()| message.to_string())));
    }

    pub fn poll(&mut self) -> Option<JobOutcome> {
        let outcome = self.running.as_ref()?.1.try_recv().ok()?;
        self.running = None;
        Some(outcome)
    }

    fn spawn(&mut self, label: &str, ctx: &eframe::egui::Context, work: impl FnOnce() -> JobOutcome + Send + 'static) {
        if self.running.is_some() {
            return;
        }
        let (sender, receiver) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = sender.send(work());
            ctx.request_repaint();
        });
        self.running = Some((label.to_string(), receiver));
    }
}

fn finished(result: Result<String, String>) -> JobOutcome {
    match result {
        Ok(message) => JobOutcome::Done(message),
        Err(error) => JobOutcome::Failed(error),
    }
}
