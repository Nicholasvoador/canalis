//! SETTING: background service, start at login, update rate, files and about.

use super::page_scroll;
use crate::gui::jobs::Job;
use crate::gui::shell::CanalisWindow;
use crate::gui::theme::{DANGER, LIME, MUTED, OK};
use crate::gui::widgets::{CARD_GAP, bold, busy_note, card, info_line, muted, page_header, primary_button, secondary_button, setting_row, status_dot};
use crate::service_control::{self, ServiceAction};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::switch::Switch;
use gpui_kit::*;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const LOGS_HINT: &str = "The service log is service.log, next to the live status file below.";
#[cfg(not(windows))]
const LOGS_HINT: &str = "Service logs: journalctl --user -u canalis -f";

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

impl CanalisWindow {
    pub fn setting_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let cards = div()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(self.service_card(cx))
            .children(self.sensors_card())
            .child(self.files_card(window, cx))
            .child(about_card());
        div().flex().flex_col().size_full().child(page_header("SETTING")).child(page_scroll("setting-scroll", cards)).into_any_element()
    }

    fn service_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        card()
            .child(bold("Background service", 19.0))
            .child(muted("Keeps the screen configured and sends it live stats, even when this window is closed."))
            .child(div().pt_2().child(self.service_status()))
            .child(div().flex().items_center().gap_3().py_2().child(self.service_buttons(cx)))
            .child(self.login_start_row(cx))
            .child(muted(LOGS_HINT))
    }

    fn service_status(&self) -> impl IntoElement {
        if self.connected() {
            status_dot(OK, "Running and connected to the screen")
        } else if self.service_running() {
            status_dot(LIME, "Running, waiting for the screen")
        } else {
            status_dot(DANGER, "Not running")
        }
    }

    fn service_buttons(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(label) = self.busy_label() {
            return busy_note(label).into_any_element();
        }
        let command = |action: ServiceAction| cx.listener(move |this: &mut Self, _: &ClickEvent, _, cx| this.start_job(Job::service_command(action), cx));
        if !self.service_running() {
            return primary_button("service-start", "Start").on_click(command(ServiceAction::Start)).into_any_element();
        }
        div()
            .flex()
            .gap_3()
            .child(secondary_button("service-restart", "Restart").on_click(command(ServiceAction::Restart)))
            .child(secondary_button("service-stop", "Stop").on_click(command(ServiceAction::Stop)))
            .into_any_element()
    }

    fn login_start_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(enabled) = self.setting_page.login_start else {
            return muted("Start at login is unavailable: run the installer first.").into_any_element();
        };
        let switch = Switch::new("login-start").checked(enabled).on_click(cx.listener(|this, checked: &bool, _, cx| {
            this.setting_page.login_start = Some(*checked);
            this.start_job(Job::set_login_start(*checked), cx);
        }));
        setting_row("Start at login", "Run the service automatically when you log in, like KANALI's \"Startup with Windows\".", switch).into_any_element()
    }

    /// Windows only lets kernel drivers read temperatures and fans; LibreHardwareMonitor provides them.
    #[cfg(windows)]
    fn sensors_card(&self) -> Option<impl IntoElement> {
        let card = card().child(bold("Temperatures and fans", 19.0));
        if self.status().number("/snapshot/cpu/temperature") > 0.0 {
            return Some(card.child(status_dot(OK, "Reading them from LibreHardwareMonitor")));
        }
        Some(card.child(status_dot(LIME, "Not available: LibreHardwareMonitor is not running")).child(muted(
            "install-windows.ps1 installs LibreHardwareMonitor and starts it at login; run the installer again to repair it. \
             Until it runs, the CPU temperature reads 0, so the smart fan curve stays at its lowest speed.",
        )))
    }

    #[cfg(not(windows))]
    fn sensors_card(&self) -> Option<Div> {
        None
    }

    fn files_card(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let interval = self.editor.settings.update_interval_seconds as f32;
        let slider = self.sliders.update_interval.show(interval, self.dragging, window, cx);
        let control = div().flex().items_center().gap_3().child(div().w(px(260.0)).child(slider)).child(format!("{interval:.1} s"));
        card()
            .child(setting_row("Update interval", "How often live stats are sent to the screen.", control))
            .child(file_location("Settings file", self.editor.path(), cx))
            .child(file_location("Live status file", &self.setting_page.status_path, cx))
    }
}

fn file_location(title: &'static str, path: &Path, cx: &App) -> impl IntoElement {
    let path = div().font_family(cx.theme().mono_font_family.clone()).text_size(px(12.5)).child(path.display().to_string());
    div().flex().flex_col().gap_1().pt_1().child(muted(title)).child(path)
}

fn about_card() -> impl IntoElement {
    card()
        .child(bold("About", 19.0))
        .child(info_line("Version", env!("CARGO_PKG_VERSION")))
        .child(muted("Unofficial replacement for KANALI on Linux and Windows. Not affiliated with TRYX. The USB protocol was worked out from KANALI and the screen's own logs.").text_color(rgb(MUTED)))
        .mb(px(CARD_GAP))
}
