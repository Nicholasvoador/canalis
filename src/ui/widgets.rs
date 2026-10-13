//! Reusable pieces of the KANALI-style layout.

use super::theme::{self, BORDER, CARD, CARD_RADIUS, LIME, MUTED, TEXT};
use eframe::egui::{self, Align, Color32, CornerRadius, Layout, Margin, Response, RichText, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

pub fn bold(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).family(theme::bold_family()).size(size).color(TEXT)
}

pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text).color(MUTED)
}

const CARD_MARGIN: i8 = 24;
const CARD_STROKE_WIDTH: f32 = 1.0;

/// The content height that makes a card exactly `outer_height` tall.
pub fn card_content_height(outer_height: f32) -> f32 {
    outer_height - 2.0 * (CARD_MARGIN as f32 + CARD_STROKE_WIDTH)
}

pub fn card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(CARD_STROKE_WIDTH, BORDER))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::same(CARD_MARGIN))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(add_contents).inner
        })
        .inner
}

pub fn page_header(ui: &mut Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(bold(title, 28.0));
    ui.add_space(14.0);
    divider(ui);
    ui.add_space(14.0);
}

pub fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0, BORDER);
}

/// KANALI-style text tabs with a short lime underline under the selected one.
pub fn tabs<T: PartialEq + Copy>(ui: &mut Ui, current: &mut T, options: &[(T, &str)]) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 32.0;
        for (value, label) in options {
            let selected = *current == *value;
            let text = if selected { bold(*label, 15.0) } else { RichText::new(*label).size(15.0).color(MUTED) };
            let response = ui.add(egui::Label::new(text).selectable(false).sense(Sense::click()));
            if selected {
                let underline = egui::Rect::from_min_size(pos2(response.rect.left(), response.rect.bottom() + 5.0), vec2(22.0, 3.0));
                ui.painter().rect_filled(underline, 2, LIME);
            }
            if response.clicked() {
                *current = *value;
            }
        }
    });
    ui.add_space(18.0);
}

/// iOS-style switch. Returns true when flipped.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(46.0, 26.0), Sense::click());
    if response.clicked() {
        *on = !*on;
    }
    let progress = ui.ctx().animate_bool_responsive(response.id, *on);
    let track = if *on { LIME } else { theme::BORDER };
    ui.painter().rect_filled(rect, 13, track);
    let knob_x = egui::lerp((rect.left() + 13.0)..=(rect.right() - 13.0), progress);
    let knob = if *on { theme::BLACK } else { TEXT };
    ui.painter().circle_filled(pos2(knob_x, rect.center().y), 9.5, knob);
    response.clicked()
}

/// Row of mutually exclusive pill buttons. Returns true when the value changed.
pub fn segmented<T: PartialEq + Clone>(ui: &mut Ui, current: &mut T, options: &[(T, String)]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (value, label) in options {
            let selected = *current == *value;
            if pill(ui, label, selected).clicked() && !selected {
                *current = value.clone();
                changed = true;
            }
        }
    });
    changed
}

pub fn pill(ui: &mut Ui, label: &str, selected: bool) -> Response {
    ui.add(pill_button(label, selected))
}

/// Pills keep their label on one line, so a wrapped row moves a whole pill to the next line.
pub fn pill_button(label: &str, selected: bool) -> egui::Button<'_> {
    let (fill, text_color) = if selected { (LIME, theme::BLACK) } else { (theme::CARD_RAISED, TEXT) };
    egui::Button::new(RichText::new(label).color(text_color))
        .wrap_mode(egui::TextWrapMode::Extend)
        .fill(fill)
        .stroke(Stroke::new(1.0, if selected { LIME } else { BORDER }))
        .corner_radius(CornerRadius::same(18))
        .min_size(vec2(56.0, 32.0))
}

pub fn primary_button(ui: &mut Ui, label: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(theme::BLACK).family(theme::bold_family()))
            .fill(LIME)
            .corner_radius(CornerRadius::same(10))
            .min_size(vec2(120.0, 36.0)),
    )
}

pub fn secondary_button(ui: &mut Ui, label: &str) -> Response {
    ui.add(egui::Button::new(label).stroke(Stroke::new(1.0, BORDER)).corner_radius(CornerRadius::same(10)).min_size(vec2(100.0, 36.0)))
}

/// Title + description on the left, control on the right.
pub fn setting_row(ui: &mut Ui, title: &str, description: &str, add_control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_max_width((ui.available_width() - 380.0).max(240.0));
            ui.label(bold(title, 15.5));
            if !description.is_empty() {
                ui.label(muted(description).size(13.0));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), add_control);
    });
    ui.add_space(6.0);
    divider(ui);
    ui.add_space(6.0);
}

/// A card filling exactly `rect` (already allocated by the caller); overflowing content is clipped.
pub fn card_at(ui: &mut Ui, rect: Rect, add_contents: impl FnOnce(&mut Ui)) {
    let mut card_ui = ui.new_child(UiBuilder::new().max_rect(rect));
    card_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
    card(&mut card_ui, |ui| {
        ui.set_min_height(ui.available_height());
        add_contents(ui);
    });
}

/// A small coloured dot followed by text, used for connection state.
pub fn status_dot(ui: &mut Ui, color: Color32, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, color);
        ui.label(RichText::new(text).color(TEXT));
    });
}

/// Key/value line inside info cards.
pub fn info_line(ui: &mut Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(muted(key));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.label(RichText::new(value).color(TEXT)));
    });
}
