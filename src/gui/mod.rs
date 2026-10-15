//! Native desktop window drawn with GPUI (Zed's GPU UI framework) through GPUI Kit, styled after KANALI.

mod assets;
mod carousel;
mod charts;
mod crop_dialog;
mod easing;
mod fan_curve;
mod geometry;
mod images;
mod jobs;
mod live_view;
mod media_preview;
mod pages;
mod paint;
mod pointer;
mod preset_art;
mod screen_preview;
mod setting_slider;
mod shell;
mod theme;
mod theme_player;
mod widgets;

use crate::settings::SettingsStore;
use crate::ui::StartPage;
use gpui_kit::*;
use std::path::PathBuf;

const WINDOW_SIZE: Size<Pixels> = size(px(1240.0), px(840.0));
const MIN_WINDOW_SIZE: Size<Pixels> = size(px(980.0), px(640.0));

pub fn run(store: SettingsStore, status_path: PathBuf, start_page: StartPage) {
    gpui_kit::application().with_assets(assets::CanalisAssets).run(move |cx| {
        gpui_kit::init(cx);
        theme::apply(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(WINDOW_SIZE, cx)),
            window_min_size: Some(MIN_WINDOW_SIZE),
            titlebar: Some(TitlebarOptions { title: Some("Canalis".into()), ..TitlebarOptions::default() }),
            app_id: Some("canalis".into()),
            ..WindowOptions::default()
        };
        let opened = gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| shell::CanalisWindow::new(store, status_path, start_page, window, cx)));
        if let Err(error) = opened {
            eprintln!("canalis: cannot open the window: {error}");
            cx.quit();
        }
    });
}
