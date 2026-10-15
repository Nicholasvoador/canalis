//! KANALI's carousel of built-in themes: five cards, the centre one large and the outer ones
//! narrower and darker as if turned away. Click a side card to bring it to the centre, click the
//! centre card to put that theme on the screen, or use the arrows.

use super::geometry::Area;
use super::paint::{Anchor, Label, circle, polyline, rect_outline, ring, rounded_rect, text};
use super::preset_art::PresetArt;
use super::screen_preview::{paint_cover, paint_placeholder_art};
use super::theme::{BLACK, LIME, TEXT, faded};
use crate::settings::{PRESETS, preset_name};
use gpui_kit::*;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

const VISIBLE_SIDE_CARDS: i64 = 2;
/// Width and height of a card relative to the centre one, by distance from the centre.
const WIDTH_SCALE: [f32; 4] = [1.0, 0.42, 0.26, 0.0];
const HEIGHT_SCALE: [f32; 4] = [1.0, 0.86, 0.74, 0.6];
const DARKENING: [f32; 4] = [0.0, 0.45, 0.65, 0.8];
const SLIDE_TIME: Duration = Duration::from_millis(250);
const ARROW_RADIUS: f32 = 22.0;
const CARD_RADIUS: f32 = 12.0;
/// How close to the centre a card must slide before it counts as the one in front.
const CENTRE_CARD: f32 = 0.5;
const COMPETITION: &str = "TRYX 3D Visual Competition 2024";
/// Credits KANALI shows for the competition themes: (index into PRESETS, artist, award).
const CREDITS: [(usize, &str, &str); 6] = [
    (8, "Carson", ""),
    (9, "HuangQin", ""),
    (10, "shuiguo", ""),
    (11, "DayX", " - First Prize"),
    (12, "LiuChee", " - Second Prize"),
    (13, "JasonLee", " - Third Prize"),
];

pub struct Carousel {
    /// Index of the centre card, not wrapped, so sliding past the end stays smooth.
    position: i64,
    slide_from: f32,
    slide_started: Instant,
    /// Where the carousel was last drawn, for working out what a click hit.
    drawn_at: Rc<Cell<Area>>,
}

/// What a click on the carousel asks for.
pub enum CarouselClick {
    /// Put this theme on the screen.
    Choose(usize),
    Moved,
    Missed,
}

impl Carousel {
    pub fn new(centre: usize) -> Self {
        Self { position: centre as i64, slide_from: centre as f32, slide_started: Instant::now(), drawn_at: Rc::default() }
    }

    /// The carousel `width` points wide; `selected` is the theme on the screen.
    pub fn element(&self, width: f32, art: &mut PresetArt, selected: Option<usize>) -> impl IntoElement + use<> {
        let height = carousel_height(width);
        let animated = self.animated_position();
        let cards: Vec<CardScene> = cards_far_to_near(animated)
            .into_iter()
            .filter_map(|card| {
                let distance = card as f32 - animated;
                let index = card.rem_euclid(PRESETS.len() as i64) as usize;
                let steps = distance.abs();
                if steps >= 3.0 {
                    return None;
                }
                let playing = (steps < CENTRE_CARD).then(|| art.playing_frame(index)).flatten();
                Some(CardScene { distance, index, picture: playing.or_else(|| art.picture(index)), selected: selected == Some(index) })
            })
            .collect();
        let drawn_at = self.drawn_at.clone();
        canvas(
            move |bounds, _, _| drawn_at.set(Area::from_bounds(bounds)),
            move |bounds, (), window, cx| paint_carousel(Area::from_bounds(bounds), &cards, window, cx),
        )
        .w(px(width))
        .h(px(height))
    }

    /// Handles a click at `spot` (window coordinates).
    pub fn click(&mut self, spot: Point<f32>) -> CarouselClick {
        let area = self.drawn_at.get();
        for (direction, step) in [(-1.0, -1), (1.0, 1)] {
            if Area::centred(arrow_centre(area, direction), ARROW_RADIUS * 2.0, ARROW_RADIUS * 2.0).contains(spot) {
                self.slide_to(self.position + step);
                return CarouselClick::Moved;
            }
        }
        let animated = self.animated_position();
        let mut near_first = cards_far_to_near(animated);
        near_first.reverse();
        for card in near_first {
            let Some(rect) = card_rect(area, card as f32 - animated) else { continue };
            if !rect.contains(spot) {
                continue;
            }
            if card == self.position {
                return CarouselClick::Choose(card.rem_euclid(PRESETS.len() as i64) as usize);
            }
            self.slide_to(card);
            return CarouselClick::Moved;
        }
        CarouselClick::Missed
    }

    fn slide_to(&mut self, position: i64) {
        self.slide_from = self.animated_position();
        self.slide_started = Instant::now();
        self.position = position;
    }

    fn animated_position(&self) -> f32 {
        let progress = (self.slide_started.elapsed().as_secs_f32() / SLIDE_TIME.as_secs_f32()).min(1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        self.slide_from + (self.position as f32 - self.slide_from) * eased
    }
}

pub fn carousel_height(width: f32) -> f32 {
    (width * 0.27).clamp(170.0, 300.0)
}

struct CardScene {
    distance: f32,
    index: usize,
    picture: Option<Arc<RenderImage>>,
    selected: bool,
}

/// Cards around the centre, outermost first so nearer cards are drawn on top.
fn cards_far_to_near(animated: f32) -> Vec<i64> {
    let centre = animated.round() as i64;
    let mut cards: Vec<i64> = (centre - VISIBLE_SIDE_CARDS - 1..=centre + VISIBLE_SIDE_CARDS + 1).collect();
    cards.sort_by(|a, b| (*b as f32 - animated).abs().total_cmp(&(*a as f32 - animated).abs()));
    cards
}

fn centre_card_size(area: Area) -> (f32, f32) {
    let width = (area.height * 2.0).min(area.width * 0.52);
    (width, width / 2.0)
}

/// Where a card sits `distance` cards from the centre (fractional while sliding), or None off-stage.
fn card_rect(area: Area, distance: f32) -> Option<Area> {
    let steps = distance.abs();
    if steps >= 3.0 {
        return None;
    }
    let (centre_width, centre_height) = centre_card_size(area);
    let (width, height) = (centre_width * interpolate(&WIDTH_SCALE, steps), centre_height * interpolate(&HEIGHT_SCALE, steps));
    let offset = (0..steps.ceil() as usize).fold(0.0, |offset, step| {
        let share = (steps - step as f32).min(1.0);
        let (inner, outer) = (centre_width * WIDTH_SCALE[step], centre_width * WIDTH_SCALE[step + 1]);
        offset + share * (inner / 2.0 + outer / 2.0 - outer * 0.2)
    });
    let centre = area.centre();
    Some(Area::centred(point(centre.x + offset * distance.signum(), centre.y), width, height))
}

fn interpolate(table: &[f32; 4], steps: f32) -> f32 {
    let lower = (steps.floor() as usize).min(2);
    let amount = steps - lower as f32;
    table[lower] + (table[lower + 1] - table[lower]) * amount
}

fn arrow_centre(area: Area, direction: f32) -> Point<f32> {
    point(area.centre().x + direction * (area.width / 2.0 - ARROW_RADIUS - 4.0), area.centre().y)
}

fn paint_carousel(area: Area, cards: &[CardScene], window: &mut Window, cx: &mut App) {
    for card in cards {
        if let Some(rect) = card_rect(area, card.distance) {
            paint_card(window, cx, rect, card);
        }
    }
    for direction in [-1.0, 1.0] {
        paint_arrow(window, arrow_centre(area, direction), direction);
    }
}

fn paint_card(window: &mut Window, cx: &mut App, rect: Area, card: &CardScene) {
    let steps = card.distance.abs();
    rounded_rect(window, rect, CARD_RADIUS, rgb(BLACK));
    match &card.picture {
        Some(picture) => paint_cover(window, rect, Corners::all(px(CARD_RADIUS)), picture, 0),
        None => paint_placeholder_art(window, cx, rect, ""),
    }
    rounded_rect(window, rect, CARD_RADIUS, rgba(interpolate_darkening(steps)));
    let (border_width, border) = if card.selected { (2.0, rgb(LIME)) } else { (1.0, rgba(0xFFFFFF1E)) };
    rect_outline(window, rect, CARD_RADIUS, border_width, border);
    if steps < 1.5 {
        window.with_content_mask(Some(ContentMask { bounds: rect.shrink(4.0).bounds() }), |window| paint_title(window, cx, rect, card));
    }
    if steps < CENTRE_CARD {
        paint_credit(window, cx, rect, card.index);
    }
}

fn interpolate_darkening(steps: f32) -> u32 {
    (255.0 * interpolate(&DARKENING, steps)) as u32
}

fn paint_title(window: &mut Window, cx: &mut App, rect: Area, card: &CardScene) {
    let mark = point(rect.left + 22.0, rect.top + 22.0);
    ring(window, mark, 9.0, 1.5, rgb(0xFFFFFF));
    if card.selected {
        circle(window, mark, 5.0, rgb(LIME));
    }
    let title = format!("Pre-set {}:  {}", card.index + 1, preset_name(PRESETS[card.index]));
    text(window, cx, point(mark.x + 18.0, mark.y), Anchor::LeftCentre, &Label::new(&title, 14.0, rgb(TEXT)));
}

fn paint_credit(window: &mut Window, cx: &mut App, rect: Area, index: usize) {
    let Some((_, artist, award)) = CREDITS.iter().find(|(credited, _, _)| *credited == index) else { return };
    let bottom_left = point(rect.left + 16.0, rect.bottom() - 14.0);
    text(window, cx, bottom_left, Anchor::LeftBottom, &Label::new(&format!("{COMPETITION}{award}"), 12.0, faded(TEXT, 0.8)));
    text(window, cx, point(bottom_left.x, bottom_left.y - 18.0), Anchor::LeftBottom, &Label::new(&format!("Art by {artist}"), 12.5, rgb(TEXT)));
}

fn paint_arrow(window: &mut Window, centre: Point<f32>, direction: f32) {
    circle(window, centre, ARROW_RADIUS, rgb(0xEBEBEB));
    let tip = point(centre.x + direction * 5.0, centre.y);
    let back = centre.x - direction * 4.0;
    polyline(window, &[point(back, centre.y - 8.0), tip, point(back, centre.y + 8.0)], 2.2, rgb(BLACK));
}

#[cfg(test)]
mod tests {
    use super::*;
    // GPUI's own `test` attribute comes in with `gpui_kit::*`; these are plain tests.
    use core::prelude::v1::test;

    fn stage() -> Area {
        Area::new(-400.0, -100.0, 800.0, 200.0)
    }

    #[test]
    fn centre_card_is_full_size_and_outer_cards_shrink() {
        let middle = card_rect(stage(), 0.0).unwrap();
        let side = card_rect(stage(), 1.0).unwrap();
        let outer = card_rect(stage(), -2.0).unwrap();
        assert_eq!((middle.width, middle.height), (400.0, 200.0));
        assert!(side.width < middle.width && side.centre().x > 0.0);
        assert!(outer.width < side.width && outer.centre().x < 0.0);
        assert!(card_rect(stage(), 3.0).is_none());
    }

    #[test]
    fn side_cards_overlap_the_centre_card() {
        let middle = card_rect(stage(), 0.0).unwrap();
        let side = card_rect(stage(), 1.0).unwrap();
        assert!(side.left < middle.right());
    }
}
