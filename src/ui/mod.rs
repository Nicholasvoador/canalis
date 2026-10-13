//! Native desktop window (egui), styled after KANALI.

mod app;
mod charts;
mod editor;
mod fan_curve;
mod fonts;
mod format;
mod icons;
mod jobs;
mod network_history;
mod pages;
mod screen_preview;
mod theme;
mod widgets;

use crate::settings::SettingsStore;
use eframe::egui;
use std::path::PathBuf;

/// Page the window opens on (`panorama ui --page fan`).
#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum StartPage {
    Home,
    Specifications,
    Screen,
    Theme,
    Fan,
    Setting,
}

pub fn run(store: SettingsStore, status_path: PathBuf, start_page: StartPage) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Panorama")
            .with_app_id("panorama")
            .with_inner_size([1240.0, 840.0])
            .with_min_inner_size([980.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Panorama",
        options,
        Box::new(move |creation| {
            fonts::install(&creation.egui_ctx);
            theme::apply(&creation.egui_ctx);
            Ok(Box::new(app::PanoramaApp::new(store, status_path, start_page)))
        }),
    )
    .map_err(|error| error.to_string())
}
