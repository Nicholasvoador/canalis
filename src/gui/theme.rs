//! Colours and component theme, modelled on KANALI's lime-and-charcoal look.

use gpui_kit::component::{Theme, ThemeConfig, ThemeMode};
use gpui_kit::{App, Rgba, rgb, rgba};
use std::rc::Rc;

pub const LIME: u32 = 0xDEF750;
pub const BLACK: u32 = 0x0B0B0C;
pub const BACKGROUND: u32 = 0x1C1E22;
pub const CARD: u32 = 0x24272C;
pub const CARD_RAISED: u32 = 0x2C3036;
pub const BORDER: u32 = 0x34383F;
pub const TEXT: u32 = 0xF4F5F6;
pub const MUTED: u32 = 0x969BA3;
pub const DANGER: u32 = 0xFF6B6B;
pub const OK: u32 = 0x7BE08A;
pub const DOWNLOAD_GREEN: u32 = 0x72D56C;
pub const UPLOAD_BLUE: u32 = 0x3D64F2;
pub const ORANGE: u32 = 0xF59A57;

/// `color` with its opacity scaled by `alpha` (0..=1).
pub fn faded(color: u32, alpha: f32) -> Rgba {
    rgba((color << 8) | (alpha.clamp(0.0, 1.0) * 255.0).round() as u32)
}

/// The colour `amount` (0..=1) of the way from `from` to `to`.
pub fn mix(from: u32, to: u32, amount: f32) -> Rgba {
    let channel = |shift: u32| {
        let (a, b) = (((from >> shift) & 0xFF) as f32, ((to >> shift) & 0xFF) as f32);
        (a + (b - a) * amount.clamp(0.0, 1.0)).round() as u32
    };
    rgb((channel(16) << 16) | (channel(8) << 8) | channel(0))
}

/// GPUI Kit's components (buttons, switches, sliders) take their colours from this theme.
pub fn apply(cx: &mut App) {
    match serde_json::from_str::<ThemeConfig>(include_str!("../../assets/canalis-theme.json")) {
        Ok(config) => {
            Theme::global_mut(cx).apply_config(&Rc::new(config));
            Theme::change(ThemeMode::Dark, None, cx);
        }
        Err(error) => eprintln!("canalis: cannot read the theme: {error}"),
    }
}
