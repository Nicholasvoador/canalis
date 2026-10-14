//! Stand-in for the cooler's 2:1 panel. With live view on it shows screenshots of the real screen;
//! otherwise it composes the theme or media with the live stats, laid out as the screen draws them
//! (measured from screenshots of firmware V1.0.11).

use super::format;
use super::live_view::{LiveFrame, LiveView};
use super::media_preview::{MediaFrame, MediaPreview};
use super::preset_art::PresetArt;
use super::theme::{self, BORDER, CARD_RAISED, LIME, MUTED, TEXT};
use crate::settings::{CPU_BADGE, CUSTOMIZATION, FILTERS, Filter, OverlayStyle, PRESETS, Settings};
use crate::status::LiveStatus;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2, pos2, vec2};
use std::time::{Duration, Instant};

const MAX_PREVIEW_WIDTH: f32 = 560.0;
const PANEL_RATIO: f32 = 2.0;
const SCREEN_RADIUS: u8 = 10;
const SCREEN_BLACK: Color32 = Color32::from_rgb(0x08, 0x09, 0x0A);
/// A screenshot taken this soon after a theme change may still show the previous theme.
const THEME_SETTLE_TIME: Duration = Duration::from_secs(4);
/// Gap from the screen's edge to the stats and badges, as a share of its height.
const EDGE_MARGIN: f32 = 0.074;

/// Everything the preview draws from: media files, theme pictures and live screenshots.
#[derive(Default)]
pub struct PreviewSources {
    /// The full-screen media, or the left and right halves in split screen.
    pub media: [MediaPreview; 2],
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
        return;
    }
    for (slot, pane) in panes(settings, screen).iter().enumerate() {
        paint_backdrop(ui, &painter, pane, &mut sources.media[slot], &mut sources.presets);
        paint_filter(ui, &painter, pane.area, &pane.overlay.style.filter);
        paint_overlay(&painter, pane, status);
    }
}

/// A picture with live stats over it: the whole screen, or one half of it in split screen.
struct Pane<'a> {
    area: Rect,
    corners: CornerRadius,
    backdrop: Backdrop<'a>,
    overlay: Overlay<'a>,
    /// Half of a split screen, which lays its stats out a little differently.
    is_half: bool,
}

enum Backdrop<'a> {
    Preset(usize),
    Media(&'a str),
    NoMedia,
    EmptyHalf,
}

struct Overlay<'a> {
    style: &'a OverlayStyle,
    fields: &'a [String],
    temperature_unit: &'a str,
}

fn panes<'a>(settings: &'a Settings, screen: Rect) -> Vec<Pane<'a>> {
    let unit = settings.temperature_unit.as_str();
    let whole = |backdrop, style, fields: &'a [String]| Pane {
        area: screen,
        corners: SCREEN_RADIUS.into(),
        backdrop,
        overlay: Overlay { style, fields, temperature_unit: unit },
        is_half: false,
    };
    let custom = &settings.custom;
    if let Some(index) = settings.preset_index() {
        return vec![whole(Backdrop::Preset(index), &settings.overlay_style, &settings.overlay)];
    }
    if !custom.is_split() {
        let backdrop = custom.media.first().map_or(Backdrop::NoMedia, |name| Backdrop::Media(name));
        return vec![whole(backdrop, &custom.style, &custom.overlay)];
    }
    let halves = split_areas(screen);
    custom
        .halves
        .iter()
        .zip(halves)
        .map(|(half, (area, corners))| Pane {
            area,
            corners,
            backdrop: half.media.as_deref().map_or(Backdrop::EmptyHalf, Backdrop::Media),
            overlay: Overlay { style: &half.style, fields: &half.overlay, temperature_unit: unit },
            is_half: true,
        })
        .collect()
}

/// The left and right halves, rounded only on the screen's outer corners.
fn split_areas(screen: Rect) -> [(Rect, CornerRadius); 2] {
    let (left, right) = screen.split_left_right_at_fraction(0.5);
    let radius = SCREEN_RADIUS;
    [(left, CornerRadius { nw: radius, sw: radius, ne: 0, se: 0 }), (right, CornerRadius { nw: 0, sw: 0, ne: radius, se: radius })]
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

fn paint_backdrop(ui: &Ui, painter: &Painter, pane: &Pane, media: &mut MediaPreview, presets: &mut PresetArt) {
    let area = pane.area;
    match pane.backdrop {
        Backdrop::Preset(index) => match presets.texture(ui.ctx(), index) {
            Some(texture) => paint_rounded_texture(ui, area, texture, pane.corners),
            None => paint_placeholder_art(painter, area, PRESETS[index]),
        },
        Backdrop::Media(name) => match media.frame(ui.ctx(), name) {
            MediaFrame::Image(texture) => paint_rounded_texture(ui, area, texture, pane.corners),
            MediaFrame::Loading => caption(painter, area, "Loading the screen's media…"),
            MediaFrame::Unavailable(reason) => caption(painter, area, reason),
        },
        Backdrop::NoMedia => paint_placeholder_art(painter, area, "Customization (no media yet)"),
        Backdrop::EmptyHalf => caption(painter, area, "No media on this half"),
    }
}

pub fn paint_texture(ui: &Ui, area: Rect, texture: &TextureHandle) {
    paint_rounded_texture(ui, area, texture, SCREEN_RADIUS.into());
}

fn paint_rounded_texture(ui: &Ui, area: Rect, texture: &TextureHandle, corners: CornerRadius) {
    egui::Image::from_texture(texture).uv(cover_uv(texture.size_vec2(), area.size())).corner_radius(corners).paint_at(ui, area);
}

/// The part of an image that fills `area` without stretching, cropped evenly from both sides.
fn cover_uv(image: Vec2, area: Vec2) -> Rect {
    let image_ratio = image.x / image.y;
    let area_ratio = area.x / area.y;
    let span = if image_ratio > area_ratio { vec2(area_ratio / image_ratio, 1.0) } else { vec2(1.0, image_ratio / area_ratio) };
    Rect::from_center_size(pos2(0.5, 0.5), span)
}

/// Wrapped to the screen's width so a long error stays readable.
fn caption(painter: &Painter, screen: Rect, text: &str) {
    let galley = painter.layout(text.to_string(), FontId::proportional(11.5), TEXT.gamma_multiply(0.75), screen.width() - 24.0);
    painter.galley(screen.left_bottom() + vec2(12.0, -10.0 - galley.size().y), galley, TEXT);
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
fn paint_filter(ui: &Ui, painter: &Painter, screen: Rect, filter: &Filter) {
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

fn paint_overlay(painter: &Painter, pane: &Pane, status: &LiveStatus) {
    let (area, overlay) = (pane.area, &pane.overlay);
    let color = parse_hex_color(&overlay.style.color).unwrap_or(TEXT);
    let height = area.height();
    let fonts = StatFonts {
        value: FontId::new(height * 0.165, theme::bold_family()),
        unit: FontId::proportional((height * 0.04).max(9.0)),
        label: FontId::proportional((height * 0.028).max(9.0)),
    };
    let row_height = height * 0.265;
    let fields = screen_order(overlay.fields, pane.is_half);
    let top = area.center().y - row_height * fields.len() as f32 / 2.0;
    for (row, field) in fields.iter().enumerate() {
        let (value, unit) = format::split_reading(&format::overlay_value(field, status, overlay.temperature_unit));
        let stat = Stat { value, unit, label: format::overlay_label(field) };
        let left = aligned_left(area, stat_width(painter, &stat, &fonts), &overlay.style.align);
        paint_stat(painter, pos2(left, top + row as f32 * row_height), &stat, &fonts, color);
    }
    paint_badges(painter, pane, status);
}

/// Full screen draws the second stat at the top, then the first, then the rest (seen with
/// [CPU, GPU] and with [CPU, GPU, Date & Time]); each half of a split screen keeps the chosen order.
fn screen_order(fields: &[String], is_half: bool) -> Vec<&String> {
    let mut ordered: Vec<&String> = fields.iter().collect();
    if ordered.len() >= 2 && !is_half {
        ordered.swap(0, 1);
    }
    ordered
}

fn aligned_left(screen: Rect, width: f32, align: &str) -> f32 {
    let margin = screen.height() * EDGE_MARGIN;
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
/// A right-aligned half of a split screen puts its badges in the top-right corner instead.
fn paint_badges(painter: &Painter, pane: &Pane, status: &LiveStatus) {
    let (area, overlay) = (pane.area, &pane.overlay);
    let font_size = (area.height() * 0.027).max(9.0);
    let names: Vec<String> = overlay.style.shown_badges(overlay.fields).iter().map(|shown| badge_name(shown, status)).collect();
    let from_right = pane.is_half && overlay.style.align == "Right";
    let margin = area.height() * EDGE_MARGIN;
    let mut x = if from_right { area.right() - margin - badges_width(painter, &names, font_size) } else { area.left() + margin };
    for name in &names {
        let rect = badge(painter, pos2(x, area.top() + area.height() * 0.07), name, vendor_color(name), font_size);
        x = rect.right() + BADGE_GAP;
    }
}

const BADGE_GAP: f32 = 8.0;

fn badge_name(shown: &str, status: &LiveStatus) -> String {
    let name = status.text(if shown == CPU_BADGE { "/spec/cpu" } else { "/spec/gpu" });
    if name.is_empty() { shown.replace(" Badge", "") } else { name }
}

fn badges_width(painter: &Painter, names: &[String], font_size: f32) -> f32 {
    let widths = names.iter().map(|name| badge_size(painter, name, font_size).x + BADGE_GAP);
    widths.sum::<f32>() - BADGE_GAP
}

fn vendor_color(name: &str) -> Color32 {
    match name.to_uppercase() {
        upper if upper.contains("NVIDIA") || upper.contains("GEFORCE") => Color32::from_rgb(0x5E, 0x9E, 0x10),
        upper if upper.contains("INTEL") => Color32::from_rgb(0x00, 0x68, 0xB5),
        _ => Color32::from_rgb(0xC9, 0x45, 0x34),
    }
}

fn badge_size(painter: &Painter, text: &str, font_size: f32) -> Vec2 {
    painter.layout_no_wrap(text.to_string(), FontId::proportional(font_size), Color32::WHITE).size() + vec2(font_size * 1.4, font_size * 0.7)
}

fn badge(painter: &Painter, left_top: Pos2, text: &str, fill: Color32, font_size: f32) -> Rect {
    let galley = painter.layout_no_wrap(text.to_string(), FontId::proportional(font_size), Color32::WHITE);
    let rect = Rect::from_min_size(left_top, badge_size(painter, text, font_size));
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
    fn split_screen_has_a_pane_per_half() {
        let mut settings = Settings::default();
        settings.split_screen("left.gif".into(), "right.gif".into());
        settings.custom.remove_media("right.gif");
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0));
        let panes = panes(&settings, screen);
        assert_eq!(panes.iter().map(|pane| pane.area.width()).collect::<Vec<_>>(), [100.0, 100.0]);
        assert!(matches!(panes[0].backdrop, Backdrop::Media("left.gif")));
        assert!(matches!(panes[1].backdrop, Backdrop::EmptyHalf));
    }

    #[test]
    fn full_screen_and_presets_have_one_pane() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0));
        let preset = Settings::default();
        assert!(matches!(panes(&preset, screen)[..], [Pane { backdrop: Backdrop::Preset(0), .. }]));
        let custom = Settings { theme: CUSTOMIZATION.into(), ..Settings::default() };
        assert!(matches!(panes(&custom, screen)[..], [Pane { backdrop: Backdrop::NoMedia, .. }]));
    }

    #[test]
    fn stats_follow_the_screens_order() {
        let fields: Vec<String> = ["CPU Temperature", "GPU Temperature", "Date&Time"].map(String::from).to_vec();
        assert_eq!(screen_order(&fields, false), ["GPU Temperature", "CPU Temperature", "Date&Time"]);
        assert_eq!(screen_order(&fields[..1], false), ["CPU Temperature"]);
        assert_eq!(screen_order(&fields, true), ["CPU Temperature", "GPU Temperature", "Date&Time"]);
    }
}
