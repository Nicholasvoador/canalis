//! Native desktop window (egui), styled after KANALI.

mod app;
mod carousel;
mod charts;
mod crop_dialog;
mod editor;
mod fan_curve;
mod fonts;
mod format;
mod icons;
mod images;
mod jobs;
mod live_view;
mod media_preview;
mod network_history;
mod pages;
mod preset_art;
mod screen_preview;
mod theme;
mod theme_player;
mod widgets;

use crate::settings::SettingsStore;
use eframe::{egui, egui_wgpu};
use std::path::PathBuf;

/// Page the window opens on (`canalis ui --page fan`).
#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum StartPage {
    Home,
    Specifications,
    #[value(alias = "theme")]
    Presets,
    Customization,
    Screen,
    Fan,
    Setting,
}

pub fn run(store: SettingsStore, status_path: PathBuf, start_page: StartPage) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Canalis")
            .with_app_id("canalis")
            .with_inner_size([1240.0, 840.0])
            .with_min_inner_size([980.0, 640.0])
            .with_icon(window_icon()),
        wgpu_options: renderer(),
        ..Default::default()
    };
    eframe::run_native(
        "Canalis",
        options,
        Box::new(move |creation| {
            egui_extras::install_image_loaders(&creation.egui_ctx);
            fonts::install(&creation.egui_ctx);
            theme::apply(&creation.egui_ctx);
            let mut window = app::CanalisApp::new(store, status_path, start_page);
            window.start_companions(&creation.egui_ctx);
            Ok(Box::new(window))
        }),
    )
    .map_err(|error| error.to_string())
}

fn window_icon() -> egui::IconData {
    use crate::app_icon::{LARGEST_SIZE, rgba};
    egui::IconData { rgba: rgba(LARGEST_SIZE), width: LARGEST_SIZE, height: LARGEST_SIZE }
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
