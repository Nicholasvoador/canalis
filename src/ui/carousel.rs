//! KANALI's carousel of built-in themes: five cards, the centre one large and the outer ones
//! narrower and darker as if turned away. Click a side card to bring it to the centre, click the
//! centre card to put that theme on the screen, or use the arrows.

use super::preset_art::PresetArt;
use super::screen_preview::{paint_placeholder_art, paint_texture};
use super::theme::{BLACK, LIME, TEXT};
use crate::settings::{PRESETS, preset_name};
use eframe::egui::{self, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

const VISIBLE_SIDE_CARDS: i64 = 2;
/// Width and height of a card relative to the centre one, by distance from the centre.
const WIDTH_SCALE: [f32; 4] = [1.0, 0.42, 0.26, 0.0];
const HEIGHT_SCALE: [f32; 4] = [1.0, 0.86, 0.74, 0.6];
const DARKENING: [f32; 4] = [0.0, 0.45, 0.65, 0.8];
const SLIDE_SECONDS: f32 = 0.25;
const ARROW_RADIUS: f32 = 22.0;
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
}

impl Carousel {
    pub fn new(centre: usize) -> Self {
        Self { position: centre as i64 }
    }

    /// Draws the carousel and returns the index of a theme the user chose.
    pub fn show(&mut self, ui: &mut Ui, art: &mut PresetArt, selected: Option<usize>) -> Option<usize> {
        let width = ui.available_width();
        let (area, _) = ui.allocate_exact_size(vec2(width, (width * 0.27).clamp(170.0, 300.0)), Sense::hover());
        let card_width = (area.height() * 2.0).min(width * 0.52);
        let centre_card = vec2(card_width, card_width / 2.0);
        let animated = ui.ctx().animate_value_with_time(Id::new("theme-carousel"), self.position as f32, SLIDE_SECONDS);
        let mut chosen = None;
        for card in self.cards_far_to_near(animated) {
            let distance = card as f32 - animated;
            let Some(rect) = card_rect(area.center(), centre_card, distance) else { continue };
            let index = card.rem_euclid(PRESETS.len() as i64) as usize;
            paint_card(ui, rect, index, distance.abs(), art, selected == Some(index));
            let response = ui.interact(rect, Id::new(("theme-card", card)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                if card == self.position { chosen = Some(index) } else { self.position = card }
            }
        }
        self.arrows(ui, area);
        chosen
    }

    /// Cards around the centre, outermost first so nearer cards are drawn (and clicked) on top.
    fn cards_far_to_near(&self, animated: f32) -> Vec<i64> {
        let centre = animated.round() as i64;
        let mut cards: Vec<i64> = (centre - VISIBLE_SIDE_CARDS - 1..=centre + VISIBLE_SIDE_CARDS + 1).collect();
        cards.sort_by(|a, b| (*b as f32 - animated).abs().total_cmp(&(*a as f32 - animated).abs()));
        cards
    }

    fn arrows(&mut self, ui: &mut Ui, area: Rect) {
        for (direction, step) in [(-1.0, -1), (1.0, 1)] {
            let centre = pos2(area.center().x + direction * (area.width() / 2.0 - ARROW_RADIUS - 4.0), area.center().y);
            let rect = Rect::from_center_size(centre, vec2(ARROW_RADIUS * 2.0, ARROW_RADIUS * 2.0));
            let response = ui.interact(rect, Id::new(("theme-arrow", step)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            let painter = ui.painter();
            painter.circle_filled(centre, ARROW_RADIUS, if response.hovered() { Color32::WHITE } else { Color32::from_gray(235) });
            let tip = centre + vec2(direction * 5.0, 0.0);
            let back = centre - vec2(direction * 4.0, 0.0);
            painter.line(vec![back + vec2(0.0, -8.0), tip, back + vec2(0.0, 8.0)], Stroke::new(2.2, BLACK));
            if response.clicked() {
                self.position += step;
            }
        }
    }
}

/// Where a card sits `distance` cards from the centre (fractional while sliding), or None off-stage.
fn card_rect(centre: Pos2, centre_card: egui::Vec2, distance: f32) -> Option<Rect> {
    let steps = distance.abs();
    if steps >= 3.0 {
        return None;
    }
    let size = vec2(centre_card.x * interpolate(&WIDTH_SCALE, steps), centre_card.y * interpolate(&HEIGHT_SCALE, steps));
    let offset = (0..steps.ceil() as usize).fold(0.0, |offset, step| {
        let share = (steps - step as f32).min(1.0);
        let (inner, outer) = (centre_card.x * WIDTH_SCALE[step], centre_card.x * WIDTH_SCALE[step + 1]);
        offset + share * (inner / 2.0 + outer / 2.0 - outer * 0.2)
    });
    Some(Rect::from_center_size(centre + vec2(offset * distance.signum(), 0.0), size))
}

fn interpolate(table: &[f32; 4], steps: f32) -> f32 {
    let lower = (steps.floor() as usize).min(2);
    egui::lerp(table[lower]..=table[lower + 1], steps - lower as f32)
}

fn paint_card(ui: &Ui, rect: Rect, index: usize, steps: f32, art: &mut PresetArt, selected: bool) {
    let painter = ui.painter();
    painter.rect_filled(rect, 12, BLACK);
    // Only the card in the centre plays its theme's video, and the still stands in until it starts.
    let playing = (steps < CENTRE_CARD).then(|| art.playing_frame(ui.ctx(), index).cloned()).flatten();
    match playing.or_else(|| art.texture(ui.ctx(), index).cloned()) {
        Some(texture) => paint_texture(ui, rect, &texture),
        None => paint_placeholder_art(painter, rect, ""),
    }
    painter.rect_filled(rect, 12, Color32::from_black_alpha((255.0 * interpolate(&DARKENING, steps)) as u8));
    let border = if selected { Stroke::new(2.0, LIME) } else { Stroke::new(1.0, Color32::from_white_alpha(30)) };
    painter.rect_stroke(rect, 12, border, StrokeKind::Inside);
    if steps < 1.5 {
        paint_title(painter, rect, index, selected);
    }
    if steps < CENTRE_CARD {
        paint_credit(painter, rect, index);
    }
}

fn paint_title(painter: &egui::Painter, rect: Rect, index: usize, selected: bool) {
    let mark = rect.left_top() + vec2(22.0, 22.0);
    painter.circle_stroke(mark, 9.0, Stroke::new(1.5, Color32::WHITE));
    if selected {
        painter.circle_filled(mark, 5.0, LIME);
    }
    let title = format!("Pre-set {}:  {}", index + 1, preset_name(PRESETS[index]));
    let clipped = painter.with_clip_rect(rect.shrink(4.0));
    clipped.text(mark + vec2(18.0, 0.0), Align2::LEFT_CENTER, title, FontId::proportional(14.0), TEXT);
}

fn paint_credit(painter: &egui::Painter, rect: Rect, index: usize) {
    let Some((_, artist, award)) = CREDITS.iter().find(|(credited, _, _)| *credited == index) else { return };
    let bottom_left = rect.left_bottom() + vec2(16.0, -14.0);
    painter.text(bottom_left, Align2::LEFT_BOTTOM, format!("{COMPETITION}{award}"), FontId::proportional(12.0), TEXT.gamma_multiply(0.8));
    painter.text(bottom_left - vec2(0.0, 18.0), Align2::LEFT_BOTTOM, format!("Art by {artist}"), FontId::proportional(12.5), TEXT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centre_card_is_full_size_and_outer_cards_shrink() {
        let centre = vec2(400.0, 200.0);
        let middle = card_rect(Pos2::ZERO, centre, 0.0).unwrap();
        let side = card_rect(Pos2::ZERO, centre, 1.0).unwrap();
        let outer = card_rect(Pos2::ZERO, centre, -2.0).unwrap();
        assert_eq!(middle.size(), centre);
        assert!(side.width() < middle.width() && side.center().x > 0.0);
        assert!(outer.width() < side.width() && outer.center().x < 0.0);
        assert!(card_rect(Pos2::ZERO, centre, 3.0).is_none());
    }

    #[test]
    fn side_cards_overlap_the_centre_card() {
        let centre = vec2(400.0, 200.0);
        let middle = card_rect(Pos2::ZERO, centre, 0.0).unwrap();
        let side = card_rect(Pos2::ZERO, centre, 1.0).unwrap();
        assert!(side.left() < middle.right());
    }
}
