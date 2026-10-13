//! The background service's tray icon, like KANALI's: click it to open the window, or Quit from
//! its menu to stop the service. Windows uses the notification area; Linux uses the
//! freedesktop StatusNotifierItem protocol, which Waybar's tray shows on Omarchy.

#[cfg(not(windows))]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
use linux as platform;
#[cfg(windows)]
use windows as platform;

use crate::app_icon as icon;
use crate::hidden_command::hidden_command;

const TOOLTIP: &str = "Panorama (cooler screen)";
const ICON_SIZE: u32 = 32;

/// Shows the icon and handles its clicks. Returns only if the icon cannot be shown.
pub fn run() -> Result<(), String> {
    platform::run()
}

fn open_window() {
    let opened = std::env::current_exe().and_then(|executable| hidden_command(executable).spawn());
    if let Err(error) = opened {
        eprintln!("panorama: cannot open the window: {error}");
    }
}

/// Quit stops this service; it starts again at the next login, or from the window's SETTING page.
fn quit() -> ! {
    eprintln!("panorama: quit from the tray icon");
    std::process::exit(0)
}
