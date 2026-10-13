//! Drawn stand-in for the cooler's 2240×1080 panel showing the theme and live overlay.

use super::format;
use super::theme::{self, BORDER, LIME, MUTED, TEXT};
use crate::settings::{CUSTOMIZATION, Settings};
use crate::status::LiveStatus;
use eframe::egui::{self, Align2, Color32, FontId, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

const PREVIEW_SIZE: egui::Vec2 = egui::Vec2::new(392.0, 196.0);

pub fn screen_preview(ui: &mut Ui, settings: &Settings, status: &LiveStatus) {
    let width = ui.available_width().min(PREVIEW_SIZE.x);
    let (response, painter) = ui.allocate_painter(vec2(width, width * PREVIEW_SIZE.y / PREVIEW_SIZE.x), Sense::hover());
    let bezel = response.rect;
    painter.rect_filled(bezel, 18, Color32::from_rgb(0x0E, 0x0F, 0x11));
    painter.rect_stroke(bezel, 18, Stroke::new(1.5, BORDER), StrokeKind::Inside);
    let screen = bezel.shrink(10.0);
    if !settings.enable {
        painter.text(screen.center(), Align2::CENTER_CENTER, "Screen off", FontId::proportional(16.0), MUTED);
        return;
    }
    paint_theme_backdrop(&painter, screen, settings);
    paint_overlay(&painter, screen, settings, status);
    painter.text(screen.right_top() + vec2(-8.0, 8.0), Align2::RIGHT_TOP, format!("↻ {}°", settings.rotate), FontId::proportional(11.0), TEXT.gamma_multiply(0.7));
}

fn paint_theme_backdrop(painter: &egui::Painter, screen: Rect, settings: &Settings) {
    painter.rect_filled(screen, 10, Color32::from_rgb(0x15, 0x17, 0x1A));
    let art = Rect::from_min_max(pos2(screen.center().x + 20.0, screen.top() + 28.0), screen.right_bottom() - vec2(18.0, 30.0));
    let stripes = if settings.theme == CUSTOMIZATION { 4 } else { 7 };
    for index in 0..stripes {
        let top = art.top() + art.height() * index as f32 / stripes as f32;
        let indent = art.width() * 0.08 * index as f32;
        let band = Rect::from_min_max(pos2(art.left() + indent, top), pos2(art.right(), top + 7.0));
        painter.rect_filled(band, 3, LIME.gamma_multiply(0.15 + 0.06 * index as f32));
    }
    let caption = if settings.theme == CUSTOMIZATION { custom_caption(settings) } else { settings.theme.clone() };
    painter.text(screen.left_bottom() + vec2(10.0, -8.0), Align2::LEFT_BOTTOM, caption, FontId::proportional(11.5), TEXT.gamma_multiply(0.75));
}

fn custom_caption(settings: &Settings) -> String {
    match settings.custom.media.as_slice() {
        [] => "Customization (no media)".into(),
        [only] => only.clone(),
        many => format!("{} files, {}", many.len(), settings.custom.play_mode.to_lowercase()),
    }
}

fn paint_overlay(painter: &egui::Painter, screen: Rect, settings: &Settings, status: &LiveStatus) {
    let color = if settings.is_custom_theme() { parse_hex_color(&settings.custom.color).unwrap_or(TEXT) } else { TEXT };
    let (anchor, x) = match settings.custom.align.as_str() {
        "Center" => (Align2::CENTER_TOP, screen.center().x),
        "Right" => (Align2::RIGHT_TOP, screen.right() - 16.0),
        _ => (Align2::LEFT_TOP, screen.left() + 16.0),
    };
    for (row, field) in settings.active_overlay().iter().enumerate() {
        let y = screen.top() + 16.0 + row as f32 * 44.0;
        painter.text(pos2(x, y), anchor, field, FontId::proportional(11.0), color.gamma_multiply(0.8));
        let value = format::overlay_value(field, status, &settings.temperature_unit);
        painter.text(pos2(x, y + 13.0), anchor, value, FontId::new(22.0, theme::bold_family()), color);
    }
}

pub fn parse_hex_color(text: &str) -> Option<Color32> {
    let hex = text.strip_prefix('#')?;
    let value = u32::from_str_radix(hex, 16).ok().filter(|_| hex.len() == 6)?;
    Some(Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colours() {
        assert_eq!(parse_hex_color("#dcdcdc"), Some(Color32::from_rgb(0xDC, 0xDC, 0xDC)));
        assert_eq!(parse_hex_color("dcdcdc"), None);
        assert_eq!(parse_hex_color("#12"), None);
    }
}
