//! SETTING: background service, start at login, update rate, files and about.

use super::PageContext;
use crate::service_control::{self, ServiceAction};
use crate::ui::theme::{DANGER, LIME, OK};
use crate::ui::widgets::{bold, card, info_line, muted, page_header, primary_button, secondary_button, setting_row, status_dot, toggle};
use eframe::egui::{self, RichText, Ui};
use std::path::{Path, PathBuf};

#[cfg(windows)]
const LOGS_HINT: &str = "The service log is service.log, next to the live status file below.";
#[cfg(not(windows))]
const LOGS_HINT: &str = "Service logs: journalctl --user -u panorama -f";

/// What the SETTING page remembers between frames.
pub struct SettingPage {
    pub status_path: PathBuf,
    /// None when it could not be read (for example, the service was never installed).
    pub login_start: Option<bool>,
}

impl SettingPage {
    pub fn new(status_path: PathBuf) -> Self {
        let mut page = Self { status_path, login_start: None };
        page.reload_login_start();
        page
    }

    pub fn reload_login_start(&mut self) {
        self.login_start = service_control::is_login_start_enabled().ok();
    }
}

pub fn show(ui: &mut Ui, state: &mut SettingPage, page: &mut PageContext) {
    page_header(ui, "SETTING");
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        service_card(ui, state, page);
        ui.add_space(14.0);
        #[cfg(windows)]
        {
            sensors_card(ui, page);
            ui.add_space(14.0);
        }
        card(ui, |ui| {
            setting_row(ui, "Update interval", "How often live stats are sent to the screen.", |ui| {
                ui.add(egui::Slider::new(&mut page.editor.settings.update_interval_seconds, 0.5..=5.0).suffix(" s").step_by(0.5));
            });
            file_location(ui, "Settings file", page.editor.path());
            file_location(ui, "Live status file", &state.status_path);
        });
        ui.add_space(14.0);
        about_card(ui);
    });
}

fn file_location(ui: &mut Ui, title: &str, path: &Path) {
    ui.label(muted(title));
    ui.add(egui::Label::new(RichText::new(path.display().to_string()).monospace().size(12.5)).wrap());
    ui.add_space(4.0);
}

fn service_card(ui: &mut Ui, state: &mut SettingPage, page: &mut PageContext) {
    card(ui, |ui| {
        ui.label(bold("Background service", 19.0));
        ui.label(muted("Keeps the screen configured and sends it live stats, even when this window is closed."));
        ui.add_space(12.0);
        service_status(ui, page);
        ui.add_space(12.0);
        ui.horizontal(|ui| service_buttons(ui, page));
        ui.add_space(12.0);
        login_start_row(ui, state, page);
        ui.label(muted(LOGS_HINT));
    });
}

fn service_status(ui: &mut Ui, page: &PageContext) {
    if page.connected() {
        status_dot(ui, OK, "Running and connected to the screen");
    } else if page.service_running() {
        status_dot(ui, LIME, "Running, waiting for the screen");
    } else {
        status_dot(ui, DANGER, "Not running");
    }
}

fn service_buttons(ui: &mut Ui, page: &mut PageContext) {
    if let Some(label) = page.jobs.busy_label() {
        ui.spinner();
        ui.label(muted(label));
        return;
    }
    let ctx = ui.ctx().clone();
    if !page.service_running() && primary_button(ui, "Start").clicked() {
        page.jobs.service_command(ServiceAction::Start, &ctx);
    }
    if page.service_running() && secondary_button(ui, "Restart").clicked() {
        page.jobs.service_command(ServiceAction::Restart, &ctx);
    }
    if page.service_running() && secondary_button(ui, "Stop").clicked() {
        page.jobs.service_command(ServiceAction::Stop, &ctx);
    }
}

fn login_start_row(ui: &mut Ui, state: &mut SettingPage, page: &mut PageContext) {
    let Some(enabled) = state.login_start.as_mut() else {
        ui.label(muted("Start at login is unavailable: run the installer first."));
        return;
    };
    setting_row(ui, "Start at login", "Run the service automatically when you log in, like KANALI's \"Startup with Windows\".", |ui| {
        if toggle(ui, enabled) {
            page.jobs.set_login_start(*enabled, ui.ctx());
        }
    });
}

/// Windows only lets kernel drivers read temperatures and fans; LibreHardwareMonitor provides them.
#[cfg(windows)]
fn sensors_card(ui: &mut Ui, page: &PageContext) {
    card(ui, |ui| {
        ui.label(bold("Temperatures and fans", 19.0));
        ui.add_space(8.0);
        if page.status.number("/snapshot/cpu/temperature") > 0.0 {
            status_dot(ui, OK, "Reading them from LibreHardwareMonitor");
            return;
        }
        status_dot(ui, LIME, "Not available: LibreHardwareMonitor is not running");
        ui.label(muted(
            "install-windows.ps1 installs LibreHardwareMonitor and starts it at login; run the installer again to repair it. \
             Until it runs, the CPU temperature reads 0, so the smart fan curve stays at its lowest speed.",
        ));
    });
}

fn about_card(ui: &mut Ui) {
    card(ui, |ui| {
        ui.label(bold("About", 19.0));
        ui.add_space(6.0);
        info_line(ui, "Version", env!("CARGO_PKG_VERSION"));
        ui.label(muted(
            "Unofficial replacement for KANALI on Linux and Windows. Not affiliated with TRYX. The USB protocol was worked out from KANALI and the screen's own logs.",
        ));
    });
}
