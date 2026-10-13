//! Stand-in for the cooler's 2:1 panel: the media it plays, with the live stats laid out as the screen draws them.

use super::format;
use super::media_preview::{MediaFrame, MediaPreview};
use super::theme::{self, BORDER, LIME, MUTED, TEXT};
use crate::settings::{CUSTOMIZATION, Settings};
use crate::status::LiveStatus;
use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

const MAX_PREVIEW_WIDTH: f32 = 560.0;
const PANEL_RATIO: f32 = 2.0;
const SCREEN_RADIUS: u8 = 10;
const SCREEN_BLACK: Color32 = Color32::from_rgb(0x08, 0x09, 0x0A);

pub fn screen_preview(ui: &mut Ui, settings: &Settings, status: &LiveStatus, media: &mut MediaPreview) {
    let width = ui.available_width().min(MAX_PREVIEW_WIDTH);
    let (response, painter) = ui.allocate_painter(vec2(width, width / PANEL_RATIO + 20.0), Sense::hover());
    let bezel = response.rect;
    painter.rect_filled(bezel, 18, Color32::from_rgb(0x0E, 0x0F, 0x11));
    painter.rect_stroke(bezel, 18, Stroke::new(1.5, BORDER), StrokeKind::Inside);
    let screen = bezel.shrink(10.0);
    if !settings.enable {
        painter.text(screen.center(), Align2::CENTER_CENTER, "Screen off", FontId::proportional(16.0), MUTED);
        return;
    }
    paint_backdrop(ui, &painter, screen, settings, media);
    paint_overlay(&painter, screen, settings, status);
    painter.text(screen.right_top() + vec2(-8.0, 8.0), Align2::RIGHT_TOP, format!("↻ {}°", settings.rotate), FontId::proportional(11.0), TEXT.gamma_multiply(0.7));
}

fn paint_backdrop(ui: &Ui, painter: &Painter, screen: Rect, settings: &Settings, media: &mut MediaPreview) {
    painter.rect_filled(screen, SCREEN_RADIUS, SCREEN_BLACK);
    match settings.custom.media.first().filter(|_| settings.is_custom_theme()) {
        Some(name) => paint_media(ui, painter, screen, media.frame(ui.ctx(), name)),
        None => paint_placeholder_art(painter, screen, settings),
    }
}

fn paint_media(ui: &Ui, painter: &Painter, screen: Rect, frame: MediaFrame) {
    match frame {
        MediaFrame::Image(texture) => {
            egui::Image::from_texture(texture).uv(cover_uv(texture.size_vec2(), screen.size())).corner_radius(SCREEN_RADIUS).paint_at(ui, screen);
        }
        MediaFrame::Loading => caption(painter, screen, "Loading the screen's media…"),
        MediaFrame::Unavailable(reason) => caption(painter, screen, reason),
    }
}

/// The part of an image that fills `area` without stretching, cropped evenly from both sides.
fn cover_uv(image: Vec2, area: Vec2) -> Rect {
    let image_ratio = image.x / image.y;
    let area_ratio = area.x / area.y;
    let span = if image_ratio > area_ratio { vec2(area_ratio / image_ratio, 1.0) } else { vec2(1.0, image_ratio / area_ratio) };
    Rect::from_center_size(pos2(0.5, 0.5), span)
}

fn caption(painter: &Painter, screen: Rect, text: &str) {
    painter.text(screen.left_bottom() + vec2(12.0, -10.0), Align2::LEFT_BOTTOM, text, FontId::proportional(11.5), TEXT.gamma_multiply(0.75));
}

/// Built-in themes are animations inside the screen's firmware, so they are only suggested here.
fn paint_placeholder_art(painter: &Painter, screen: Rect, settings: &Settings) {
    let art = Rect::from_min_max(pos2(screen.center().x + 20.0, screen.top() + 28.0), screen.right_bottom() - vec2(18.0, 30.0));
    for index in 0..7 {
        let top = art.top() + art.height() * index as f32 / 7.0;
        let band = Rect::from_min_max(pos2(art.left() + art.width() * 0.08 * index as f32, top), pos2(art.right(), top + 7.0));
        painter.rect_filled(band, 3, LIME.gamma_multiply(0.15 + 0.06 * index as f32));
    }
    let name = if settings.theme == CUSTOMIZATION { "Customization (no media yet)" } else { settings.theme.as_str() };
    caption(painter, screen, name);
}

struct StatFonts {
    value: FontId,
    unit: FontId,
    label: FontId,
}

struct Stat {
    value: String,
    unit: String,
    label: String,
}

fn paint_overlay(painter: &Painter, screen: Rect, settings: &Settings, status: &LiveStatus) {
    let color = if settings.is_custom_theme() { parse_hex_color(&settings.custom.color).unwrap_or(TEXT) } else { TEXT };
    let height = screen.height();
    let fonts = StatFonts { value: FontId::new(height * 0.2, theme::bold_family()), unit: FontId::proportional(height * 0.06), label: FontId::proportional(height * 0.045) };
    let row_height = height * 0.29;
    let fields = settings.active_overlay();
    let top = screen.center().y - row_height * fields.len() as f32 / 2.0;
    // The screen stacks the stats from the bottom up, so the first one chosen is the lowest.
    for (row, field) in fields.iter().rev().enumerate() {
        let (value, unit) = format::split_reading(&format::overlay_value(field, status, &settings.temperature_unit));
        let stat = Stat { value, unit, label: format::overlay_label(field) };
        let width = stat_width(painter, &stat, &fonts);
        let left = aligned_left(screen, width, &settings.custom.align);
        paint_stat(painter, pos2(left, top + row as f32 * row_height), &stat, &fonts, color);
    }
}

fn aligned_left(screen: Rect, width: f32, align: &str) -> f32 {
    let margin = screen.width() * 0.06;
    match align {
        "Center" => screen.center().x - width / 2.0,
        "Right" => screen.right() - margin - width,
        _ => screen.left() + margin,
    }
}

fn stat_width(painter: &Painter, stat: &Stat, fonts: &StatFonts) -> f32 {
    let measure = |text: &str, font: &FontId| painter.layout_no_wrap(text.to_string(), font.clone(), TEXT).size().x;
    (measure(&stat.value, &fonts.value) + measure(&stat.unit, &fonts.unit)).max(measure(&stat.label, &fonts.label))
}

fn paint_stat(painter: &Painter, left_top: Pos2, stat: &Stat, fonts: &StatFonts, color: Color32) {
    let value = shadowed_text(painter, left_top, Align2::LEFT_TOP, &stat.value, &fonts.value, color);
    shadowed_text(painter, value.right_bottom() - vec2(-2.0, value.height() * 0.18), Align2::LEFT_BOTTOM, &stat.unit, &fonts.unit, color);
    shadowed_text(painter, pos2(value.left() + 2.0, value.bottom() - value.height() * 0.08), Align2::LEFT_TOP, &stat.label, &fonts.label, color);
}

/// Text with a soft dark shadow so it stays readable over bright media.
fn shadowed_text(painter: &Painter, position: Pos2, anchor: Align2, text: &str, font: &FontId, color: Color32) -> Rect {
    painter.text(position + vec2(1.0, 1.0), anchor, text, font.clone(), Color32::from_black_alpha(140));
    painter.text(position, anchor, text, font.clone(), color)
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

    #[test]
    fn wide_media_is_cropped_at_both_sides() {
        let uv = cover_uv(vec2(400.0, 100.0), vec2(200.0, 100.0));
        assert_eq!((uv.min.x, uv.max.x, uv.min.y, uv.max.y), (0.25, 0.75, 0.0, 1.0));
    }

    #[test]
    fn tall_media_is_cropped_at_top_and_bottom() {
        let uv = cover_uv(vec2(100.0, 100.0), vec2(200.0, 100.0));
        assert_eq!((uv.min.y, uv.max.y), (0.25, 0.75));
    }
}
