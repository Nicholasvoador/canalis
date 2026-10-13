//! Linux tray icon over D-Bus (StatusNotifierItem), shown by Waybar's tray module on Omarchy.

use super::{ICON_SIZE, TOOLTIP, icon, open_window, quit};
use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;

struct PanoramaTray;

impl ksni::Tray for PanoramaTray {
    fn id(&self) -> String {
        "panorama".into()
    }

    fn title(&self) -> String {
        TOOLTIP.into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon { width: ICON_SIZE as i32, height: ICON_SIZE as i32, data: icon::argb(ICON_SIZE) }]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        open_window();
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem { label: "Open Panorama".into(), activate: Box::new(|_| open_window()), ..Default::default() }.into(),
            StandardItem { label: "Quit".into(), activate: Box::new(|_| quit()), ..Default::default() }.into(),
        ]
    }
}

pub fn run() -> Result<(), String> {
    let _tray = PanoramaTray.spawn().map_err(|error| format!("cannot show the tray icon: {error}"))?;
    loop {
        std::thread::park();
    }
}
