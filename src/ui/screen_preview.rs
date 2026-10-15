//! Stand-in for the cooler's 2:1 panel. With live view on it shows screenshots of the real screen;
//! otherwise it composes the theme or media with the live stats, laid out as the screen draws them
//! (measured from screenshots of firmware V1.0.11).

use super::geometry::Area;
use super::images::ImageBin;
use super::live_view::{LiveFrame, LiveView};
use super::media_preview::{MediaFrame, MediaPreview};
use super::paint::{Anchor, Label, line, rect_outline, rounded_rect, text};
use super::preset_art::PresetArt;
use super::theme::{BORDER, CARD_RAISED, LIME, MUTED, TEXT, faded};
use crate::settings::{CPU_BADGE, CUSTOMIZATION, FILTERS, Filter, OverlayStyle, PRESETS, Settings};
use crate::status::LiveStatus;
use crate::ui::format;
use gpui_kit::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const MAX_PREVIEW_WIDTH: f32 = 560.0;
const PANEL_RATIO: f32 = 2.0;
const BEZEL: f32 = 10.0;
const SCREEN_RADIUS: f32 = 10.0;
const SCREEN_BLACK: u32 = 0x08090A;
const BEZEL_COLOR: u32 = 0x0E0F11;
const LIVE_RED: u32 = 0xD93A3A;
/// A screenshot taken this soon after a theme change may still show the previous theme.
const THEME_SETTLE_TIME: Duration = Duration::from_secs(4);
/// Gap from the screen's edge to the stats and badges, as a share of its height.
const EDGE_MARGIN: f32 = 0.074;
const BADGE_GAP: f32 = 8.0;

/// Everything the preview draws from: media files, theme pictures and live screenshots.
pub struct PreviewSources {
    /// The full-screen media, or the left and right halves in split screen.
    pub media: [MediaPreview; 2],
    pub presets: PresetArt,
    pub live: LiveView,
    theme_shown_since: Option<(String, Instant)>,
    opened: Instant,
}

impl PreviewSources {
    pub fn new(bin: &ImageBin) -> Self {
        Self {
            media: [MediaPreview::new(bin.clone()), MediaPreview::new(bin.clone())],
            presets: PresetArt::new(bin.clone()),
            live: LiveView::new(bin.clone()),
            theme_shown_since: None,
            opened: Instant::now(),
        }
    }

    fn note_theme(&mut self, theme: &str) {
        if self.theme_shown_since.as_ref().is_none_or(|(shown, _)| shown != theme) {
            self.theme_shown_since = Some((theme.to_string(), Instant::now()));
        }
    }

    fn theme_settled_before(&self, moment: Instant) -> bool {
        self.theme_shown_since.as_ref().is_some_and(|(_, since)| moment.saturating_duration_since(*since) > THEME_SETTLE_TIME)
    }
}

/// The preview `width` points wide, drawn from a snapshot of what it shows right now.
pub fn screen_preview(width: f32, settings: &Settings, status: &LiveStatus, sources: &mut PreviewSources) -> impl IntoElement {
    let scene = Scene::compose(settings, status, sources);
    canvas(|_, _, _| {}, move |bounds, (), window, cx| scene.paint(Area::from_bounds(bounds), window, cx)).w(px(width)).h(px(width / PANEL_RATIO + 2.0 * BEZEL))
}

/// What the preview draws, gathered before painting so the paint step owns everything it needs.
enum Scene {
    Off,
    Live(LivePicture),
    Panes { panes: Vec<PaneScene>, seconds: f32 },
}

enum LivePicture {
    Image(Arc<RenderImage>),
    Caption(String),
}

struct PaneScene {
    slot: Slot,
    backdrop: BackdropScene,
    filter: Option<(bool, f32)>,
    stats: Vec<Stat>,
    color: u32,
    align: String,
    badges: Vec<String>,
    badges_from_right: bool,
}

enum BackdropScene {
    Picture(Arc<RenderImage>, usize),
    Placeholder(String),
    Caption(String),
}

struct Stat {
    value: String,
    unit: String,
    label: String,
}

impl Scene {
    fn compose(settings: &Settings, status: &LiveStatus, sources: &mut PreviewSources) -> Self {
        if !settings.enable {
            return Scene::Off;
        }
        sources.note_theme(&settings.theme);
        if sources.live.is_on() {
            return Scene::Live(live_picture(settings, sources));
        }
        let panes = panes(settings).into_iter().enumerate().map(|(slot, pane)| compose_pane(pane, &mut sources.media[slot], &mut sources.presets, status)).collect();
        Scene::Panes { panes, seconds: sources.opened.elapsed().as_secs_f32() }
    }

    fn paint(&self, bezel: Area, window: &mut Window, cx: &mut App) {
        rounded_rect(window, bezel, 18.0, rgb(BEZEL_COLOR));
        rect_outline(window, bezel, 18.0, 1.5, rgb(BORDER));
        let screen = bezel.shrink(BEZEL);
        rounded_rect(window, screen, SCREEN_RADIUS, rgb(SCREEN_BLACK));
        match self {
            Scene::Off => {
                text(window, cx, screen.centre(), Anchor::CentreCentre, &Label::new("Screen off", 16.0, rgb(MUTED)));
            }
            Scene::Live(picture) => paint_live(window, cx, screen, picture),
            Scene::Panes { panes, seconds } => {
                for pane in panes {
                    pane.paint(screen, *seconds, window, cx);
                }
            }
        }
    }
}

/// The newest screenshot, which also becomes the theme's carousel picture if it has none.
fn live_picture(settings: &Settings, sources: &mut PreviewSources) -> LivePicture {
    match sources.live.frame() {
        LiveFrame::Image(capture) => {
            if let Some(index) = settings.preset_index().filter(|_| sources.theme_settled_before(capture.taken)) {
                sources.presets.offer_capture(index, &capture.image);
            }
            LivePicture::Image(capture.picture)
        }
        LiveFrame::Waiting => LivePicture::Caption("Taking a screenshot of the cooler…".into()),
        LiveFrame::Failed(error) => LivePicture::Caption(format!("Live view unavailable: {error}")),
    }
}

fn paint_live(window: &mut Window, cx: &mut App, screen: Area, picture: &LivePicture) {
    match picture {
        LivePicture::Image(image) => {
            paint_cover(window, screen, Corners::all(px(SCREEN_RADIUS)), image, 0);
            badge(window, cx, point(screen.left + 10.0, screen.top + 10.0), "LIVE", LIVE_RED, 11.0);
        }
        LivePicture::Caption(message) => caption(window, cx, screen, message),
    }
}

/// Where a pane sits on the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Slot {
    Whole,
    Left,
    Right,
}

impl Slot {
    fn area(self, screen: Area) -> Area {
        let half = screen.width / 2.0;
        match self {
            Slot::Whole => screen,
            Slot::Left => Area { width: half, ..screen },
            Slot::Right => Area { left: screen.left + half, width: half, ..screen },
        }
    }

    /// Halves are rounded only on the screen's outer corners.
    fn corners(self) -> Corners<Pixels> {
        let (round, square) = (px(SCREEN_RADIUS), px(0.0));
        match self {
            Slot::Whole => Corners::all(round),
            Slot::Left => Corners { top_left: round, bottom_left: round, top_right: square, bottom_right: square },
            Slot::Right => Corners { top_left: square, bottom_left: square, top_right: round, bottom_right: round },
        }
    }
}

/// A picture with live stats over it: the whole screen, or one half of it in split screen.
struct Pane<'a> {
    slot: Slot,
    backdrop: Backdrop<'a>,
    overlay: Overlay<'a>,
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

fn panes(settings: &Settings) -> Vec<Pane<'_>> {
    let unit = settings.temperature_unit.as_str();
    let custom = &settings.custom;
    if let Some(index) = settings.preset_index() {
        return vec![Pane { slot: Slot::Whole, backdrop: Backdrop::Preset(index), overlay: Overlay { style: &settings.overlay_style, fields: &settings.overlay, temperature_unit: unit } }];
    }
    if !custom.is_split() {
        let backdrop = custom.media.first().map_or(Backdrop::NoMedia, |name| Backdrop::Media(name));
        return vec![Pane { slot: Slot::Whole, backdrop, overlay: Overlay { style: &custom.style, fields: &custom.overlay, temperature_unit: unit } }];
    }
    custom
        .halves
        .iter()
        .zip([Slot::Left, Slot::Right])
        .map(|(half, slot)| Pane {
            slot,
            backdrop: half.media.as_deref().map_or(Backdrop::EmptyHalf, Backdrop::Media),
            overlay: Overlay { style: &half.style, fields: &half.overlay, temperature_unit: unit },
        })
        .collect()
}

fn compose_pane(pane: Pane, media: &mut MediaPreview, presets: &mut PresetArt, status: &LiveStatus) -> PaneScene {
    let is_half = pane.slot != Slot::Whole;
    let overlay = &pane.overlay;
    let stats = screen_order(overlay.fields, is_half)
        .into_iter()
        .map(|field| {
            let (value, unit) = format::split_reading(&format::overlay_value(field, status, overlay.temperature_unit));
            Stat { value, unit, label: format::overlay_label(field) }
        })
        .collect();
    PaneScene {
        slot: pane.slot,
        backdrop: compose_backdrop(&pane.backdrop, media, presets),
        filter: filter_strength(&overlay.style.filter),
        stats,
        color: parse_hex_color(&overlay.style.color).unwrap_or(TEXT),
        align: overlay.style.align.clone(),
        badges: overlay.style.shown_badges(overlay.fields).iter().map(|shown| badge_name(shown, status)).collect(),
        badges_from_right: is_half && overlay.style.align == "Right",
    }
}

fn compose_backdrop(backdrop: &Backdrop, media: &mut MediaPreview, presets: &mut PresetArt) -> BackdropScene {
    match backdrop {
        Backdrop::Preset(index) => match presets.picture(*index) {
            Some(picture) => BackdropScene::Picture(picture, 0),
            None => BackdropScene::Placeholder(PRESETS[*index].to_string()),
        },
        Backdrop::Media(name) => match media.frame(name) {
            MediaFrame::Image(picture, frame) => BackdropScene::Picture(picture, frame),
            MediaFrame::Loading => BackdropScene::Caption("Loading the screen's media…".into()),
            MediaFrame::Unavailable(reason) => BackdropScene::Caption(reason),
        },
        Backdrop::NoMedia => BackdropScene::Placeholder(CUSTOMIZATION.into()),
        Backdrop::EmptyHalf => BackdropScene::Caption("No media on this half".into()),
    }
}

/// Rain (true) or smoke (false), and how strong.
fn filter_strength(filter: &Filter) -> Option<(bool, f32)> {
    filter.value.as_deref().map(|kind| (kind == FILTERS[0], filter.opacity as f32 / 100.0))
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

impl PaneScene {
    fn paint(&self, screen: Area, seconds: f32, window: &mut Window, cx: &mut App) {
        let area = self.slot.area(screen);
        match &self.backdrop {
            BackdropScene::Picture(picture, frame) => paint_cover(window, area, self.slot.corners(), picture, *frame),
            BackdropScene::Placeholder(name) => paint_placeholder_art(window, cx, area, name),
            BackdropScene::Caption(message) => caption(window, cx, area, message),
        }
        if let Some((is_rain, strength)) = self.filter {
            paint_filter(window, area, is_rain, strength, seconds);
        }
        self.paint_stats(area, window, cx);
        self.paint_badges(area, window, cx);
    }

    fn paint_stats(&self, area: Area, window: &mut Window, cx: &mut App) {
        let height = area.height;
        let fonts = StatFonts { value: height * 0.165, unit: (height * 0.04).max(9.0), label: (height * 0.028).max(9.0) };
        let row_height = height * 0.265;
        let top = area.centre().y - row_height * self.stats.len() as f32 / 2.0;
        for (row, stat) in self.stats.iter().enumerate() {
            let left = aligned_left(area, stat_width(window, stat, &fonts), &self.align);
            paint_stat(window, cx, point(left, top + row as f32 * row_height), stat, &fonts, self.color);
        }
    }

    /// Name badges in the top-left corner, coloured by vendor (the screen shows AMD in red).
    /// A right-aligned half of a split screen puts its badges in the top-right corner instead.
    fn paint_badges(&self, area: Area, window: &mut Window, cx: &mut App) {
        let font_size = (area.height * 0.027).max(9.0);
        let margin = area.height * EDGE_MARGIN;
        let total: f32 = self.badges.iter().map(|name| badge_size(window, name, font_size).0 + BADGE_GAP).sum::<f32>() - BADGE_GAP;
        let mut x = if self.badges_from_right { area.right() - margin - total } else { area.left + margin };
        for name in &self.badges {
            let drawn = badge(window, cx, point(x, area.top + area.height * 0.07), name, vendor_color(name), font_size);
            x = drawn.right() + BADGE_GAP;
        }
    }
}

/// `picture` filling `area` without stretching, cropped evenly from both sides.
pub fn paint_cover(window: &mut Window, area: Area, corners: Corners<Pixels>, picture: &Arc<RenderImage>, frame: usize) {
    let size = picture.size(frame);
    let image = cover_area(area, (size.width.0 as f32, size.height.0 as f32));
    let _ = window.paint_image(area.bounds(), image.bounds(), corners, picture.clone(), frame, false);
}

/// The area the whole image takes when scaled to cover `area`, centred on it.
fn cover_area(area: Area, (image_width, image_height): (f32, f32)) -> Area {
    let scale = (area.width / image_width.max(1.0)).max(area.height / image_height.max(1.0));
    Area::centred(area.centre(), image_width * scale, image_height * scale)
}

/// Wrapped onto lines that fit the area, so a long error stays readable.
pub fn caption(window: &mut Window, cx: &mut App, area: Area, message: &str) {
    let label_size = 11.5;
    let lines = wrap(window, message, label_size, area.width - 24.0);
    let mut bottom = area.bottom() - 10.0;
    for line_text in lines.iter().rev() {
        let drawn = text(window, cx, point(area.left + 12.0, bottom), Anchor::LeftBottom, &Label::new(line_text, label_size, faded(TEXT, 0.75)));
        bottom = drawn.top;
    }
}

/// Splits `message` into lines no wider than `width`, breaking between words.
fn wrap(window: &Window, message: &str, size: f32, width: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in message.split_whitespace() {
        let candidate = lines.last().map(|last| format!("{last} {word}"));
        match candidate {
            Some(longer) if super::paint::measure(window, &Label::new(&longer, size, rgb(TEXT))).0 <= width => *lines.last_mut().expect("a line") = longer,
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// Shown until a picture of the theme is available (from KANALI or the live view).
pub fn paint_placeholder_art(window: &mut Window, cx: &mut App, area: Area, name: &str) {
    rounded_rect(window, area, SCREEN_RADIUS, rgb(CARD_RAISED));
    let art = Area::from_corners(point(area.centre().x + 20.0, area.top + 28.0), point(area.right() - 18.0, area.bottom() - 30.0));
    for index in 0..7 {
        let top = art.top + art.height * index as f32 / 7.0;
        let band = Area::from_corners(point(art.left + art.width * 0.08 * index as f32, top), point(art.right(), top + 7.0));
        rounded_rect(window, band, 3.0, faded(LIME, 0.15 + 0.06 * index as f32));
    }
    let name = if name == CUSTOMIZATION { "Customization (no media yet)" } else { name };
    caption(window, cx, area, name);
}

/// Rain streaks or drifting smoke over the whole picture, like the screen's filters.
fn paint_filter(window: &mut Window, area: Area, is_rain: bool, strength: f32, seconds: f32) {
    window.with_content_mask(Some(ContentMask { bounds: area.bounds() }), |window| {
        for drop in 0..if is_rain { 70 } else { 9 } {
            let (x, speed) = (pseudo_random(drop * 2) * area.width, 0.15 + 0.25 * pseudo_random(drop * 2 + 1));
            let y = (pseudo_random(drop * 7) + seconds * speed).fract() * area.height;
            if is_rain {
                let top = point(area.left + x, area.top + y);
                line(window, top, point(top.x - 1.5, top.y + 7.0), 1.2, rgba(0xFFFFFF00 | (150.0 * strength) as u32));
            } else {
                let drift = (seconds * 0.05 + pseudo_random(drop)).sin() * 20.0;
                let centre = point(area.left + x + drift, area.bottom() - y * 0.6);
                super::paint::circle(window, centre, area.height * 0.25, rgba(0xFFFFFF00 | (14.0 * strength) as u32));
            }
        }
    });
}

fn pseudo_random(seed: usize) -> f32 {
    ((seed as f32 * 12.9898).sin() * 43_758.547).fract().abs()
}

/// Font sizes for a stat's value, unit and caption.
struct StatFonts {
    value: f32,
    unit: f32,
    label: f32,
}

fn aligned_left(area: Area, width: f32, align: &str) -> f32 {
    let margin = area.height * EDGE_MARGIN;
    match align {
        "Center" => area.centre().x - width / 2.0,
        "Right" => area.right() - margin - width,
        _ => area.left + margin,
    }
}

fn stat_width(window: &Window, stat: &Stat, fonts: &StatFonts) -> f32 {
    let measure = |text: &str, size: f32, bold: bool| {
        let label = Label::new(text, size, rgb(TEXT));
        super::paint::measure(window, &if bold { label.bold() } else { label }).0
    };
    (measure(&stat.value, fonts.value, true) + measure(&stat.unit, fonts.unit, false)).max(measure(&stat.label, fonts.label, false))
}

fn paint_stat(window: &mut Window, cx: &mut App, left_top: Point<f32>, stat: &Stat, fonts: &StatFonts, color: u32) {
    let value = shadowed_text(window, cx, left_top, Anchor::LeftTop, &Label::new(&stat.value, fonts.value, rgb(color)).bold());
    shadowed_text(window, cx, point(value.right() + 2.0, value.bottom() - value.height * 0.18), Anchor::LeftBottom, &Label::new(&stat.unit, fonts.unit, rgb(color)));
    shadowed_text(window, cx, point(value.left + 2.0, value.bottom() - value.height * 0.08), Anchor::LeftTop, &Label::new(&stat.label, fonts.label, rgb(color)));
}

/// Text with a soft dark shadow so it stays readable over bright media.
fn shadowed_text(window: &mut Window, cx: &mut App, at: Point<f32>, anchor: Anchor, label: &Label) -> Area {
    let shadow = Label { color: rgba(0x0000008C).into(), ..*label };
    text(window, cx, point(at.x + 1.0, at.y + 1.0), anchor, &shadow);
    text(window, cx, at, anchor, label)
}

fn badge_name(shown: &str, status: &LiveStatus) -> String {
    let name = status.text(if shown == CPU_BADGE { "/spec/cpu" } else { "/spec/gpu" });
    if name.is_empty() { shown.replace(" Badge", "") } else { name }
}

fn vendor_color(name: &str) -> u32 {
    match name.to_uppercase() {
        upper if upper.contains("NVIDIA") || upper.contains("GEFORCE") => 0x5E9E10,
        upper if upper.contains("INTEL") => 0x0068B5,
        _ => 0xC94534,
    }
}

fn badge_size(window: &Window, name: &str, font_size: f32) -> (f32, f32) {
    let (width, _) = super::paint::measure(window, &Label::new(name, font_size, rgb(TEXT)));
    (width + font_size * 1.4, font_size * 1.9)
}

fn badge(window: &mut Window, cx: &mut App, left_top: Point<f32>, name: &str, fill: u32, font_size: f32) -> Area {
    let (width, height) = badge_size(window, name, font_size);
    let area = Area::new(left_top.x, left_top.y, width, height);
    rounded_rect(window, area, 4.0, rgb(fill));
    text(window, cx, area.centre(), Anchor::CentreCentre, &Label::new(name, font_size, rgb(0xFFFFFF)));
    area
}

pub fn parse_hex_color(text: &str) -> Option<u32> {
    let hex = text.strip_prefix('#')?;
    u32::from_str_radix(hex, 16).ok().filter(|_| hex.len() == 6)
}

#[cfg(test)]
mod tests {
    use super::*;
    // GPUI's own `test` attribute comes in with `gpui_kit::*`; these are plain tests.
    use core::prelude::v1::test;

    #[test]
    fn parses_hex_colours() {
        assert_eq!(parse_hex_color("#dcdcdc"), Some(0xDCDCDC));
        assert_eq!(parse_hex_color("dcdcdc"), None);
        assert_eq!(parse_hex_color("#12"), None);
    }

    #[test]
    fn wide_media_is_cropped_at_both_sides() {
        let covered = cover_area(Area::new(0.0, 0.0, 200.0, 100.0), (400.0, 100.0));
        assert_eq!(covered, Area::new(-100.0, 0.0, 400.0, 100.0));
    }

    #[test]
    fn tall_media_is_cropped_at_top_and_bottom() {
        let covered = cover_area(Area::new(0.0, 0.0, 200.0, 100.0), (100.0, 100.0));
        assert_eq!(covered, Area::new(0.0, -50.0, 200.0, 200.0));
    }

    #[test]
    fn split_screen_has_a_pane_per_half() {
        let mut settings = Settings::default();
        settings.split_screen("left.gif".into(), "right.gif".into());
        settings.custom.remove_media("right.gif");
        let panes = panes(&settings);
        let screen = Area::new(0.0, 0.0, 200.0, 100.0);
        assert_eq!(panes.iter().map(|pane| pane.slot.area(screen).width).collect::<Vec<_>>(), [100.0, 100.0]);
        assert!(matches!(panes[0].backdrop, Backdrop::Media("left.gif")));
        assert!(matches!(panes[1].backdrop, Backdrop::EmptyHalf));
    }

    #[test]
    fn full_screen_and_presets_have_one_pane() {
        let preset = Settings::default();
        assert!(matches!(panes(&preset)[..], [Pane { backdrop: Backdrop::Preset(0), .. }]));
        let custom = Settings { theme: CUSTOMIZATION.into(), ..Settings::default() };
        assert!(matches!(panes(&custom)[..], [Pane { backdrop: Backdrop::NoMedia, .. }]));
    }

    #[test]
    fn stats_follow_the_screens_order() {
        let fields: Vec<String> = ["CPU Temperature", "GPU Temperature", "Date&Time"].map(String::from).to_vec();
        assert_eq!(screen_order(&fields, false), ["GPU Temperature", "CPU Temperature", "Date&Time"]);
        assert_eq!(screen_order(&fields[..1], false), ["CPU Temperature"]);
        assert_eq!(screen_order(&fields, true), ["CPU Temperature", "GPU Temperature", "Date&Time"]);
    }
}
