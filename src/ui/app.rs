//! Window shell: lime sidebar navigation plus the selected page.

use super::editor::SettingsEditor;
use super::jobs::{JobOutcome, Jobs};
use super::screen_preview::PreviewSources;
use super::network_history::NetworkHistory;
use super::pages::home::{self, HomeTab};
use super::pages::panorama::{self, PanoramaPage, PanoramaTab};
use super::pages::setting::{self, SettingPage};
use super::pages::PageContext;
use super::StartPage;
use super::icons;
use super::theme::{BACKGROUND, BLACK, DANGER, LIME, OK, TEXT};
use crate::link::unix_millis;
use crate::settings::SettingsStore;
use crate::status::StatusReader;
use eframe::egui::{self, Align, Align2, CornerRadius, FontId, ImageSource, Layout, Margin, Rect, Sense, Stroke, Ui, pos2, vec2};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const SIDEBAR_WIDTH: f32 = 272.0;
const NAV_ICON_SIZE: f32 = 24.0;
const LOGO_SIZE: f32 = 44.0;
const REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const NOTICE_DURATION: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Panorama,
    Setting,
}

struct Notice {
    text: String,
    is_error: bool,
    shown_at: Instant,
}

pub struct PanoramaApp {
    page: Page,
    home_tab: HomeTab,
    panorama_page: PanoramaPage,
    setting_page: SettingPage,
    editor: SettingsEditor,
    status: StatusReader,
    network: NetworkHistory,
    jobs: Jobs,
    preview: PreviewSources,
    notice: Option<Notice>,
}

impl PanoramaApp {
    pub fn new(store: SettingsStore, status_path: PathBuf, start_page: StartPage) -> Self {
        let (page, home_tab, panorama_tab) = match start_page {
            StartPage::Home => (Page::Home, HomeTab::Dashboard, PanoramaTab::Presets),
            StartPage::Specifications => (Page::Home, HomeTab::Specifications, PanoramaTab::Presets),
            StartPage::Presets => (Page::Panorama, HomeTab::Dashboard, PanoramaTab::Presets),
            StartPage::Customization => (Page::Panorama, HomeTab::Dashboard, PanoramaTab::Customization),
            StartPage::Screen => (Page::Panorama, HomeTab::Dashboard, PanoramaTab::Screen),
            StartPage::Fan => (Page::Panorama, HomeTab::Dashboard, PanoramaTab::Fan),
            StartPage::Setting => (Page::Setting, HomeTab::Dashboard, PanoramaTab::Presets),
        };
        let editor = SettingsEditor::new(store);
        Self {
            page,
            home_tab,
            panorama_page: PanoramaPage::new(panorama_tab, &editor.settings),
            setting_page: SettingPage::new(status_path.clone()),
            editor,
            status: StatusReader::new(status_path),
            network: NetworkHistory::default(),
            jobs: Jobs::default(),
            preview: PreviewSources::default(),
            notice: None,
        }
    }

    fn handle_finished_job(&mut self) {
        let Some(outcome) = self.jobs.poll() else { return };
        let (text, is_error) = match outcome {
            JobOutcome::MediaAdded(names) => {
                let message = format!("Copied {} to the screen", names.join(", "));
                self.editor.settings.add_custom_media(names, true);
                (message, false)
            }
            JobOutcome::Done(message) => (message, false),
            JobOutcome::Failed(error) => (error, true),
        };
        self.setting_page.reload_login_start();
        self.notice = Some(Notice { text, is_error, shown_at: Instant::now() });
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        let frame = egui::Frame::new().fill(LIME).inner_margin(Margin { left: 22, right: 22, top: 28, bottom: 24 });
        egui::Panel::left("sidebar").exact_size(SIDEBAR_WIDTH).resizable(false).show_separator_line(false).frame(frame).show(ui, |ui| {
            brand(ui);
            ui.add_space(36.0);
            self.nav_item(ui, icons::HOME, "HOMEPAGE", Page::Home);
            ui.add_space(6.0);
            self.nav_item(ui, icons::PANORAMA, "PANORAMA", Page::Panorama);
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| self.nav_item(ui, icons::SETTINGS, "SETTING", Page::Setting));
        });
    }

    fn nav_item(&mut self, ui: &mut Ui, icon: ImageSource<'static>, label: &str, page: Page) {
        let selected = self.page == page;
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 54.0), Sense::click());
        if selected {
            ui.painter().rect_filled(rect, 12, BLACK);
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 12, BLACK.gamma_multiply(0.08));
        }
        let color = if selected { TEXT } else { BLACK };
        let icon_rect = Rect::from_center_size(pos2(rect.left() + 32.0, rect.center().y), vec2(NAV_ICON_SIZE, NAV_ICON_SIZE));
        egui::Image::new(icon).tint(color).paint_at(ui, icon_rect);
        ui.painter().text(pos2(rect.left() + 58.0, rect.center().y), Align2::LEFT_CENTER, label, FontId::proportional(16.0), color);
        if response.clicked() {
            self.page = page;
        }
    }

    fn content(&mut self, ui: &mut Ui) {
        let frame = egui::Frame::new().fill(BACKGROUND).inner_margin(Margin { left: 36, right: 36, top: 20, bottom: 20 });
        let mut page = PageContext { editor: &mut self.editor, status: &self.status.current, network: &self.network, now_millis: unix_millis(), jobs: &mut self.jobs, preview: &mut self.preview, interacting: false };
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            match self.page {
                Page::Home => home::show(ui, &mut self.home_tab, &mut page),
                Page::Panorama => panorama::show(ui, &mut self.panorama_page, &mut page),
                Page::Setting => setting::show(ui, &mut self.setting_page, &mut page),
            }
        });
        let interacting = page.interacting || ui.ctx().input(|input| input.pointer.any_down());
        self.editor.sync(interacting);
    }

    fn notices(&mut self, ui: &mut Ui) {
        if self.notice.as_ref().is_some_and(|notice| notice.shown_at.elapsed() > NOTICE_DURATION) {
            self.notice = None;
        }
        let message = match (&self.editor.error, &self.notice) {
            (Some(error), _) => Some((format!("Settings not saved: {error}"), true)),
            (None, Some(notice)) => Some((notice.text.clone(), notice.is_error)),
            _ => None,
        };
        if let Some((text, is_error)) = message {
            show_toast(ui, &text, is_error);
        }
    }
}

impl eframe::App for PanoramaApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint_after(REFRESH_INTERVAL);
        self.status.refresh();
        self.network.record(&self.status.current);
        self.handle_finished_job();
        self.sidebar(ui);
        self.content(ui);
        self.notices(ui);
    }
}

fn brand(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.add(egui::Image::new(icons::LOGO).tint(BLACK).fit_to_exact_size(vec2(LOGO_SIZE, LOGO_SIZE)));
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(egui::RichText::new("PANORAMA").family(super::theme::bold_family()).size(23.0).color(BLACK));
            ui.label(egui::RichText::new("Cooler screen control").size(12.0).color(BLACK.gamma_multiply(0.7)));
        });
    });
}

fn show_toast(ui: &Ui, text: &str, is_error: bool) {
    let screen = ui.ctx().content_rect();
    let accent = if is_error { DANGER } else { OK };
    egui::Area::new(egui::Id::new("notice")).anchor(Align2::RIGHT_BOTTOM, vec2(-28.0, -28.0)).show(ui.ctx(), |ui| {
        ui.set_max_width((screen.width() * 0.5).max(320.0));
        egui::Frame::new()
            .fill(super::theme::CARD_RAISED)
            .stroke(Stroke::new(1.5, accent))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(18, 12))
            .show(ui, |ui| ui.label(egui::RichText::new(text).color(TEXT)));
    });
}
