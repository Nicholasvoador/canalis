//! Stand-in for the cooler's 2:1 panel. With live view on it shows screenshots of the real screen;
//! otherwise it composes the theme or media with the live stats, laid out as the screen draws them
//! (measured from screenshots of firmware V1.0.11).

use super::format;
use super::live_view::{LiveFrame, LiveView};
use super::media_preview::{MediaFrame, MediaPreview};
use super::preset_art::PresetArt;
use super::theme::{self, BORDER, CARD_RAISED, LIME, MUTED, TEXT};
use crate::settings::{CPU_BADGE, CUSTOMIZATION, FILTERS, OverlayStyle, Settings};
use crate::status::LiveStatus;
use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2, pos2, vec2};
use std::time::{Duration, Instant};

const MAX_PREVIEW_WIDTH: f32 = 560.0;
const PANEL_RATIO: f32 = 2.0;
const SCREEN_RADIUS: u8 = 10;
const SCREEN_BLACK: Color32 = Color32::from_rgb(0x08, 0x09, 0x0A);
/// A screenshot taken this soon after a theme change may still show the previous theme.
const THEME_SETTLE_TIME: Duration = Duration::from_secs(4);

/// Everything the preview draws from: media files, theme pictures and live screenshots.
#[derive(Default)]
pub struct PreviewSources {
    pub media: MediaPreview,
    pub presets: PresetArt,
    pub live: LiveView,
    theme_shown_since: Option<(String, Instant)>,
}

impl PreviewSources {
    fn note_theme(&mut self, theme: &str) {
        if self.theme_shown_since.as_ref().is_none_or(|(shown, _)| shown != theme) {
            self.theme_shown_since = Some((theme.to_string(), Instant::now()));
        }
    }

    fn theme_settled_before(&self, moment: Instant) -> bool {
        self.theme_shown_since.as_ref().is_some_and(|(_, since)| moment.saturating_duration_since(*since) > THEME_SETTLE_TIME)
    }
}

pub fn screen_preview(ui: &mut Ui, settings: &Settings, status: &LiveStatus, sources: &mut PreviewSources) {
    let width = ui.available_width().min(MAX_PREVIEW_WIDTH);
    let (response, painter) = ui.allocate_painter(vec2(width, width / PANEL_RATIO + 20.0), Sense::hover());
    let bezel = response.rect;
    painter.rect_filled(bezel, 18, Color32::from_rgb(0x0E, 0x0F, 0x11));
    painter.rect_stroke(bezel, 18, Stroke::new(1.5, BORDER), StrokeKind::Inside);
    let screen = bezel.shrink(10.0);
    painter.rect_filled(screen, SCREEN_RADIUS, SCREEN_BLACK);
    if !settings.enable {
        painter.text(screen.center(), Align2::CENTER_CENTER, "Screen off", FontId::proportional(16.0), MUTED);
        return;
    }
    sources.note_theme(&settings.theme);
    if sources.live.is_on() {
        paint_live(ui, &painter, screen, settings, sources);
    } else {
        paint_backdrop(ui, &painter, screen, settings, sources);
        paint_filter(ui, &painter, screen, &settings.active_style().filter);
        paint_overlay(&painter, screen, settings, status);
    }
}

/// Draws the newest screenshot, and keeps it as the theme's carousel picture if it has none.
fn paint_live(ui: &Ui, painter: &Painter, screen: Rect, settings: &Settings, sources: &mut PreviewSources) {
    let preset = settings.preset_index();
    let capture = match sources.live.frame() {
        LiveFrame::Image(capture) => {
            paint_texture(ui, screen, &capture.texture);
            badge(painter, screen.left_top() + vec2(10.0, 10.0), "LIVE", Color32::from_rgb(0xD9, 0x3A, 0x3A), 11.0);
            preset.map(|_| (capture.taken, capture.image.clone()))
        }
        LiveFrame::Waiting => {
            caption(painter, screen, "Taking a screenshot of the cooler…");
            None
        }
        LiveFrame::Failed(error) => {
            caption(painter, screen, &format!("Live view unavailable: {error}"));
            None
        }
    };
    if let (Some((taken, image)), Some(index)) = (capture, preset) {
        if sources.theme_settled_before(taken) {
            sources.presets.offer_capture(ui.ctx(), index, &image);
        }
    }
}

fn paint_backdrop(ui: &Ui, painter: &Painter, screen: Rect, settings: &Settings, sources: &mut PreviewSources) {
    if let Some(index) = settings.preset_index() {
        match sources.presets.texture(ui.ctx(), index) {
            Some(texture) => paint_texture(ui, screen, texture),
            None => paint_placeholder_art(painter, screen, &settings.theme),
        }
        return;
    }
    match settings.custom.media.first() {
        Some(name) => match sources.media.frame(ui.ctx(), name) {
            MediaFrame::Image(texture) => paint_texture(ui, screen, texture),
            MediaFrame::Loading => caption(painter, screen, "Loading the screen's media…"),
            MediaFrame::Unavailable(reason) => caption(painter, screen, reason),
        },
        None => paint_placeholder_art(painter, screen, "Customization (no media yet)"),
    }
}

pub fn paint_texture(ui: &Ui, area: Rect, texture: &TextureHandle) {
    egui::Image::from_texture(texture).uv(cover_uv(texture.size_vec2(), area.size())).corner_radius(SCREEN_RADIUS).paint_at(ui, area);
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

/// Shown until a picture of the theme is available (from KANALI or the live view).
pub fn paint_placeholder_art(painter: &Painter, area: Rect, name: &str) {
    painter.rect_filled(area, SCREEN_RADIUS, CARD_RAISED);
    let art = Rect::from_min_max(pos2(area.center().x + 20.0, area.top() + 28.0), area.right_bottom() - vec2(18.0, 30.0));
    for index in 0..7 {
        let top = art.top() + art.height() * index as f32 / 7.0;
        let band = Rect::from_min_max(pos2(art.left() + art.width() * 0.08 * index as f32, top), pos2(art.right(), top + 7.0));
        painter.rect_filled(band, 3, LIME.gamma_multiply(0.15 + 0.06 * index as f32));
    }
    let name = if name == CUSTOMIZATION { "Customization (no media yet)" } else { name };
    caption(painter, area, name);
}

/// Rain streaks or drifting smoke over the whole picture, like the screen's filters.
fn paint_filter(ui: &Ui, painter: &Painter, screen: Rect, filter: &crate::settings::Filter) {
    let Some(kind) = filter.value.as_deref() else { return };
    let strength = filter.opacity as f32 / 100.0;
    let seconds = ui.input(|input| input.time) as f32;
    ui.ctx().request_repaint_after(Duration::from_millis(50));
    let painter = painter.with_clip_rect(screen);
    for drop in 0..if kind == FILTERS[0] { 70 } else { 9 } {
        let (x, speed) = (pseudo_random(drop * 2) * screen.width(), 0.15 + 0.25 * pseudo_random(drop * 2 + 1));
        let y = ((pseudo_random(drop * 7) + seconds * speed).fract()) * screen.height();
        let position = screen.left_top() + vec2(x, y);
        if kind == FILTERS[0] {
            painter.line_segment([position, position + vec2(-1.5, 7.0)], Stroke::new(1.2, Color32::from_white_alpha((150.0 * strength) as u8)));
        } else {
            let drift = vec2((seconds * 0.05 + pseudo_random(drop)).sin() * 20.0, 0.0);
            painter.circle_filled(pos2(position.x, screen.bottom() - y * 0.6) + drift, screen.height() * 0.25, Color32::from_white_alpha((14.0 * strength) as u8));
        }
    }
}

fn pseudo_random(seed: usize) -> f32 {
    ((seed as f32 * 12.9898).sin() * 43_758.547).fract().abs()
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
    let style = settings.active_style();
    let color = parse_hex_color(&style.color).unwrap_or(TEXT);
    let height = screen.height();
    let fonts = StatFonts {
        value: FontId::new(height * 0.165, theme::bold_family()),
        unit: FontId::proportional((height * 0.04).max(9.0)),
        label: FontId::proportional((height * 0.028).max(9.0)),
    };
    let row_height = height * 0.265;
    let fields = screen_order(settings.active_overlay());
    let top = screen.center().y - row_height * fields.len() as f32 / 2.0;
    for (row, field) in fields.iter().enumerate() {
        let (value, unit) = format::split_reading(&format::overlay_value(field, status, &settings.temperature_unit));
        let stat = Stat { value, unit, label: format::overlay_label(field) };
        let left = aligned_left(screen, stat_width(painter, &stat, &fonts), &style.align);
        paint_stat(painter, pos2(left, top + row as f32 * row_height), &stat, &fonts, color);
    }
    paint_badges(painter, screen, style, settings.active_overlay(), status);
}

/// The screen draws the second stat at the top, then the first, then the rest
/// (seen with [CPU, GPU] and with [CPU, GPU, Date & Time]).
fn screen_order(fields: &[String]) -> Vec<&String> {
    let mut ordered: Vec<&String> = fields.iter().collect();
    if ordered.len() >= 2 {
        ordered.swap(0, 1);
    }
    ordered
}

fn aligned_left(screen: Rect, width: f32, align: &str) -> f32 {
    let margin = screen.width() * 0.037;
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

/// Name badges in the top-left corner, coloured by vendor (the screen shows AMD in red).
fn paint_badges(painter: &Painter, screen: Rect, style: &OverlayStyle, overlay: &[String], status: &LiveStatus) {
    let mut left_top = screen.left_top() + vec2(screen.width() * 0.034, screen.height() * 0.07);
    for shown in style.shown_badges(overlay) {
        let name = status.text(if shown == CPU_BADGE { "/spec/cpu" } else { "/spec/gpu" });
        let name = if name.is_empty() { shown.replace(" Badge", "") } else { name };
        let rect = badge(painter, left_top, &name, vendor_color(&name), (screen.height() * 0.027).max(9.0));
        left_top.x = rect.right() + 8.0;
    }
}

fn vendor_color(name: &str) -> Color32 {
    match name.to_uppercase() {
        upper if upper.contains("NVIDIA") || upper.contains("GEFORCE") => Color32::from_rgb(0x5E, 0x9E, 0x10),
        upper if upper.contains("INTEL") => Color32::from_rgb(0x00, 0x68, 0xB5),
        _ => Color32::from_rgb(0xC9, 0x45, 0x34),
    }
}

fn badge(painter: &Painter, left_top: Pos2, text: &str, fill: Color32, font_size: f32) -> Rect {
    let galley = painter.layout_no_wrap(text.to_string(), FontId::proportional(font_size), Color32::WHITE);
    let rect = Rect::from_min_size(left_top, galley.size() + vec2(font_size * 1.4, font_size * 0.7));
    painter.rect_filled(rect, 4, fill);
    painter.galley(rect.center() - galley.size() / 2.0, galley, Color32::WHITE);
    rect
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

    #[test]
    fn stats_follow_the_screens_order() {
        let fields: Vec<String> = ["CPU Temperature", "GPU Temperature", "Date&Time"].map(String::from).to_vec();
        assert_eq!(screen_order(&fields), ["GPU Temperature", "CPU Temperature", "Date&Time"]);
        assert_eq!(screen_order(&fields[..1]), ["CPU Temperature"]);
    }
}
