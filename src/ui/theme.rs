//! Colours and global style, modelled on KANALI's lime-and-charcoal look.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Vec2};

pub const LIME: Color32 = Color32::from_rgb(0xDE, 0xF7, 0x50);
pub const LIME_DIM: Color32 = Color32::from_rgb(0x8E, 0x9E, 0x35);
pub const BLACK: Color32 = Color32::from_rgb(0x0B, 0x0B, 0x0C);
pub const BACKGROUND: Color32 = Color32::from_rgb(0x1C, 0x1E, 0x22);
pub const CARD: Color32 = Color32::from_rgb(0x24, 0x27, 0x2C);
pub const CARD_RAISED: Color32 = Color32::from_rgb(0x2C, 0x30, 0x36);
pub const BORDER: Color32 = Color32::from_rgb(0x34, 0x38, 0x3F);
pub const TEXT: Color32 = Color32::from_rgb(0xF4, 0xF5, 0xF6);
pub const MUTED: Color32 = Color32::from_rgb(0x96, 0x9B, 0xA3);
pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x6B);
pub const OK: Color32 = Color32::from_rgb(0x7B, 0xE0, 0x8A);
pub const DOWNLOAD_GREEN: Color32 = Color32::from_rgb(0x72, 0xD5, 0x6C);
pub const UPLOAD_BLUE: Color32 = Color32::from_rgb(0x3D, 0x64, 0xF2);
pub const ORANGE: Color32 = Color32::from_rgb(0xF5, 0x9A, 0x57);

pub const CARD_RADIUS: u8 = 16;
pub const BOLD: &str = "bold";

pub fn bold_family() -> FontFamily {
    FontFamily::Name(BOLD.into())
}

pub fn apply(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::new(28.0, bold_family())),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(12.5, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.5, FontFamily::Monospace)),
        ]
        .into();
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 8.0);
        style.spacing.slider_width = 260.0;
        style.spacing.interact_size.y = 28.0;
        apply_visuals(&mut style.visuals);
    });
}

fn apply_visuals(visuals: &mut egui::Visuals) {
    *visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = CARD;
    visuals.extreme_bg_color = BLACK;
    visuals.faint_bg_color = CARD_RAISED;
    visuals.override_text_color = Some(TEXT);
    visuals.hyperlink_color = LIME;
    visuals.selection.bg_fill = LIME_DIM;
    visuals.selection.stroke = Stroke::new(1.5, LIME);
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    let radius = CornerRadius::same(10);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = radius;
    }
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.weak_bg_fill = CARD_RAISED;
    visuals.widgets.inactive.bg_fill = CARD_RAISED;
    visuals.widgets.hovered.weak_bg_fill = BORDER;
    visuals.widgets.hovered.bg_fill = BORDER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, LIME_DIM);
    visuals.widgets.active.weak_bg_fill = LIME_DIM;
    visuals.widgets.active.bg_fill = LIME;
}
