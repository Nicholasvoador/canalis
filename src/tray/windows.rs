//! Windows notification-area icon. Its events arrive as window messages, so this thread pumps them.

use super::{ICON_SIZE, TOOLTIP, icon, open_window, quit};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, TranslateMessage};

pub fn run() -> Result<(), String> {
    let open_item = MenuItem::new("Open Canalis", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let menu = Menu::new();
    menu.append_items(&[&open_item, &quit_item]).map_err(|error| format!("cannot build the tray menu: {error}"))?;
    let icon = Icon::from_rgba(icon::rgba(ICON_SIZE), ICON_SIZE, ICON_SIZE).map_err(|error| format!("cannot load the tray icon: {error}"))?;
    let _tray = TrayIconBuilder::new()
        .with_tooltip(TOOLTIP)
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()
        .map_err(|error| format!("cannot show the tray icon: {error}"))?;
    let (open_id, quit_id) = (open_item.id().clone(), quit_item.id().clone());
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == open_id {
            open_window();
        } else if event.id == quit_id {
            quit();
        }
    }));
    TrayIconEvent::set_event_handler(Some(|event: TrayIconEvent| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
            open_window();
        }
    }));
    pump_messages();
    Ok(())
}

fn pump_messages() {
    let mut message: MSG = unsafe { std::mem::zeroed() };
    while unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
