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
use eframe::{egui, egui_wgpu};
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
        wgpu_options: renderer(),
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

/// On Windows, draws through Direct3D 12 and DirectComposition, the way browsers do. The default pick there is
/// Vulkan, which stuttered while the window had focus. `WGPU_BACKEND` and `WGPU_DX12_PRESENTATION_SYSTEM` still override it.
#[cfg(windows)]
fn renderer() -> egui_wgpu::WgpuConfiguration {
    use eframe::wgpu::{Backends, Dx12SwapchainKind};
    let mut setup = egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends = Backends::from_env().unwrap_or(Backends::DX12);
    setup.instance_descriptor.backend_options.dx12.presentation_system = Dx12SwapchainKind::DxgiFromVisual.with_env();
    egui_wgpu::WgpuConfiguration { wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(setup), ..Default::default() }
}

#[cfg(not(windows))]
fn renderer() -> egui_wgpu::WgpuConfiguration {
    egui_wgpu::WgpuConfiguration::default()
}
