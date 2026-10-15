//! Window shell: lime sidebar navigation plus the selected page, the notice in the corner, and the
//! twice-a-second refresh of the live status and the settings file.

use super::assets;
use super::easing::Easing;
use super::images::ImageBin;
use super::jobs::{Job, JobOutcome};
use super::pages::home::HomeTab;
use super::pages::panorama::{PanoramaPage, PanoramaTab};
use super::pages::setting::SettingPage;
use super::setting_slider::{SettingSlider, SliderRange};
use super::theme::{BACKGROUND, BLACK, CARD_RAISED, DANGER, LIME, OK, TEXT, faded};
use crate::link::unix_millis;
use crate::settings::SettingsStore;
use crate::status::{LiveStatus, StatusReader};
use crate::ui::StartPage;
use crate::ui::editor::SettingsEditor;
use crate::ui::network_history::NetworkHistory;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const SIDEBAR_WIDTH: f32 = 272.0;
const REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const NOTICE_DURATION: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Home,
    Panorama,
    Setting,
}

struct Notice {
    text: SharedString,
    is_error: bool,
    shown_at: Instant,
}

/// The sliders on every page, each tied to one setting.
pub struct Sliders {
    pub brightness: SettingSlider,
    pub update_interval: SettingSlider,
    pub fan_speed: SettingSlider,
    /// The filter strength of whichever stats are being edited.
    pub filter_opacity: SettingSlider,
    pub crop_size: SettingSlider,
}

impl Sliders {
    fn new(cx: &mut Context<CanalisWindow>) -> Self {
        let percent = || SliderRange { min: 0.0, max: 100.0, step: 1.0 };
        Self {
            brightness: SettingSlider::new(percent(), |this, value| this.editor.settings.brightness = value as u8, cx),
            update_interval: SettingSlider::new(SliderRange { min: 0.5, max: 5.0, step: 0.5 }, |this, value| this.editor.settings.update_interval_seconds = value as f64, cx),
            fan_speed: SettingSlider::new(percent(), |this, value| this.editor.settings.fan.fixed_percent = value as u8, cx),
            filter_opacity: SettingSlider::new(SliderRange { min: 0.0, max: 100.0, step: 25.0 }, |this, value| this.shown_stats().1.filter.opacity = value as u8, cx),
            crop_size: SettingSlider::new(SliderRange { min: crate::crop::MIN_SCALE, max: 1.0, step: 0.01 }, |this, value| this.panorama.cropper.set_scale(value), cx),
        }
    }
}

pub struct CanalisWindow {
    pub page: Page,
    pub home_tab: HomeTab,
    pub panorama: PanoramaPage,
    pub setting_page: SettingPage,
    pub editor: SettingsEditor,
    pub status: StatusReader,
    pub network: NetworkHistory,
    pub sliders: Sliders,
    pub easing: Easing,
    /// Set while the user drags a slider or a point, so saving waits for release.
    pub dragging: bool,
    busy: Option<&'static str>,
    notice: Option<Notice>,
    /// Pictures no longer shown, dropped from the GPU before the next frame.
    bin: ImageBin,
    _ticker: Task<()>,
}

impl CanalisWindow {
    pub fn new(store: SettingsStore, status_path: PathBuf, start_page: StartPage, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (page, home_tab) = match start_page {
            StartPage::Home => (Page::Home, HomeTab::Dashboard),
            StartPage::Specifications => (Page::Home, HomeTab::Specifications),
            StartPage::Presets | StartPage::Customization | StartPage::Screen | StartPage::Fan => (Page::Panorama, HomeTab::Dashboard),
            StartPage::Setting => (Page::Setting, HomeTab::Dashboard),
        };
        let editor = SettingsEditor::new(store);
        let bin = ImageBin::default();
        let mut this = Self {
            page,
            home_tab,
            panorama: PanoramaPage::new(PanoramaTab::for_start_page(start_page), &editor.settings, &bin),
            setting_page: SettingPage::new(status_path.clone()),
            editor,
            status: StatusReader::new(status_path),
            network: NetworkHistory::default(),
            sliders: Sliders::new(cx),
            easing: Easing::default(),
            dragging: false,
            busy: None,
            notice: None,
            bin,
            _ticker: start_ticker(window, cx),
        };
        this.refresh();
        this.start_job(Job::start_companions(), cx);
        this
    }

    pub fn status(&self) -> &LiveStatus {
        &self.status.current
    }

    pub fn connected(&self) -> bool {
        self.status.current.is_connected(unix_millis())
    }

    pub fn service_running(&self) -> bool {
        self.status.current.is_service_running(unix_millis())
    }

    pub fn busy_label(&self) -> Option<&'static str> {
        self.busy
    }

    /// Call after changing the settings: saves them, unless a drag is still going on.
    pub fn settings_changed(&mut self, cx: &mut Context<Self>) {
        self.editor.sync(self.dragging);
        cx.notify();
    }

    /// Runs `job` in the background, one at a time, and reports how it went.
    pub fn start_job(&mut self, job: Job, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(job.label);
        let work = job.work;
        cx.spawn(async move |this, cx| {
            let outcome = cx.background_executor().spawn(async move { work() }).await;
            let _ = this.update(cx, |this, cx| this.finish_job(outcome, cx));
        })
        .detach();
        cx.notify();
    }

    fn finish_job(&mut self, outcome: JobOutcome, cx: &mut Context<Self>) {
        self.busy = None;
        let (text, is_error) = match outcome {
            JobOutcome::MediaAdded(names) => {
                let message = format!("Copied {} to the screen", names.join(", "));
                self.editor.settings.add_custom_media(names, true);
                (message, false)
            }
            JobOutcome::Done(message) => (message, false),
            JobOutcome::Failed(error) => (error, true),
            JobOutcome::NothingToReport => (String::new(), false),
        };
        self.setting_page.reload_login_start();
        if !text.is_empty() {
            self.notice = Some(Notice { text: text.into(), is_error, shown_at: Instant::now() });
        }
        self.settings_changed(cx);
    }

    fn refresh(&mut self) {
        self.status.refresh();
        self.network.record(&self.status.current);
        self.editor.sync(self.dragging);
        if self.notice.as_ref().is_some_and(|notice| notice.shown_at.elapsed() > NOTICE_DURATION) {
            self.notice = None;
        }
    }
}

fn start_ticker(window: &mut Window, cx: &mut Context<CanalisWindow>) -> Task<()> {
    cx.spawn_in(window, async move |this, cx| {
        loop {
            cx.background_executor().timer(REFRESH_INTERVAL).await;
            let refreshed = this.update(cx, |this, cx| {
                this.refresh();
                cx.notify();
            });
            if refreshed.is_err() {
                break;
            }
        }
    })
}

impl Render for CanalisWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.bin.empty(window);
        let page = match self.page {
            Page::Home => self.home_page(window, cx),
            Page::Panorama => self.panorama_page(window, cx),
            Page::Setting => self.setting_page(window, cx),
        };
        div()
            .relative()
            .flex()
            .size_full()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .text_size(px(15.0))
            .child(self.sidebar(cx))
            .child(div().flex().flex_col().flex_1().min_w_0().h_full().px(px(36.0)).py(px(20.0)).child(page))
            .children(self.toast())
            .children(self.crop_overlay(window, cx))
    }
}

impl CanalisWindow {
    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .bg(rgb(LIME))
            .px(px(22.0))
            .pt(px(28.0))
            .pb(px(24.0))
            .gap(px(6.0))
            .child(brand())
            .child(div().h(px(30.0)))
            .child(self.nav_item(assets::HOME, "HOMEPAGE", Page::Home, cx))
            .child(self.nav_item(assets::PANORAMA, "PANORAMA", Page::Panorama, cx))
            .child(div().flex_1())
            .child(self.nav_item(assets::SETTINGS, "SETTING", Page::Setting, cx))
    }

    fn nav_item(&self, icon: &'static str, label: &'static str, page: Page, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.page == page;
        let color = if selected { TEXT } else { BLACK };
        div()
            .id(label)
            .flex()
            .items_center()
            .gap(px(14.0))
            .h(px(54.0))
            .px(px(20.0))
            .rounded(px(12.0))
            .cursor_pointer()
            .text_size(px(16.0))
            .text_color(rgb(color))
            .when(selected, |item| item.bg(rgb(BLACK)))
            .when(!selected, |item| item.hover(|style| style.bg(faded(BLACK, 0.08))))
            .child(svg().path(icon).size(px(24.0)).text_color(rgb(color)))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.page = page;
                cx.notify();
            }))
    }

    fn toast(&self) -> Option<impl IntoElement> {
        let (text, is_error) = match (&self.editor.error, &self.notice) {
            (Some(error), _) => (SharedString::from(format!("Settings not saved: {error}")), true),
            (None, Some(notice)) => (notice.text.clone(), notice.is_error),
            _ => return None,
        };
        let accent = if is_error { DANGER } else { OK };
        let card = div().max_w(px(560.0)).px(px(18.0)).py(px(12.0)).bg(rgb(CARD_RAISED)).border_2().border_color(rgb(accent)).rounded(px(12.0)).child(text);
        Some(div().absolute().right(px(28.0)).bottom(px(28.0)).child(card))
    }
}

fn brand() -> impl IntoElement {
    let name = div().text_size(px(23.0)).font_weight(FontWeight::BOLD).text_color(rgb(BLACK)).child("CANALIS");
    let tagline = div().text_size(px(12.0)).text_color(faded(BLACK, 0.7)).child("Cooler screen control");
    div().flex().items_center().gap_3().child(svg().path(assets::LOGO).size(px(44.0)).text_color(rgb(BLACK))).child(div().flex().flex_col().child(name).child(tagline))
}
