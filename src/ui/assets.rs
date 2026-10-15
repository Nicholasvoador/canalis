//! Canalis's own icons, with GPUI Kit's Lucide icons for everything else.

use gpui_kit::{AssetSource, Result, SharedString};
use std::borrow::Cow;

pub const LOGO: &str = "icons/logo.svg";
pub const HOME: &str = "icons/home.svg";
pub const PANORAMA: &str = "icons/panorama.svg";
pub const SETTINGS: &str = "icons/settings.svg";
pub const THERMOMETER: &str = "icons/thermometer.svg";
pub const MEMORY: &str = "icons/memory.svg";
pub const DISK: &str = "icons/disk.svg";
pub const ARROW_DOWN: &str = "icons/arrow-down.svg";
pub const ARROW_UP: &str = "icons/arrow-up.svg";

const ICONS: [(&str, &[u8]); 9] = [
    (LOGO, include_bytes!("../../assets/icons/logo.svg")),
    (HOME, include_bytes!("../../assets/icons/home.svg")),
    (PANORAMA, include_bytes!("../../assets/icons/panorama.svg")),
    (SETTINGS, include_bytes!("../../assets/icons/settings.svg")),
    (THERMOMETER, include_bytes!("../../assets/icons/thermometer.svg")),
    (MEMORY, include_bytes!("../../assets/icons/memory.svg")),
    (DISK, include_bytes!("../../assets/icons/disk.svg")),
    (ARROW_DOWN, include_bytes!("../../assets/icons/arrow-down.svg")),
    (ARROW_UP, include_bytes!("../../assets/icons/arrow-up.svg")),
];

pub struct CanalisAssets;

impl AssetSource for CanalisAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match ICONS.iter().find(|(name, _)| *name == path) {
            Some((_, bytes)) => Ok(Some(Cow::Borrowed(bytes))),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui_kit::assets::Assets.list(path)
    }
}
