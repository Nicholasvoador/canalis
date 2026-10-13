//! Small line icons for the dashboard cards, drawn around a centre point.

use eframe::egui::{Color32, Painter, Pos2, Rect, Stroke, StrokeKind, vec2};

const LINE_WIDTH: f32 = 1.6;

pub type Icon = fn(&Painter, Pos2, Color32);

pub fn thermometer(painter: &Painter, centre: Pos2, color: Color32) {
    let stroke = Stroke::new(LINE_WIDTH, color);
    let bulb = centre + vec2(0.0, 5.0);
    painter.rect_stroke(Rect::from_min_max(centre + vec2(-2.5, -9.0), centre + vec2(2.5, 2.5)), 3, stroke, StrokeKind::Middle);
    painter.circle_stroke(bulb, 4.0, stroke);
    painter.circle_filled(bulb, 1.8, color);
}

pub fn memory(painter: &Painter, centre: Pos2, color: Color32) {
    let stroke = Stroke::new(LINE_WIDTH, color);
    painter.rect_stroke(Rect::from_center_size(centre + vec2(0.0, -1.5), vec2(18.0, 11.0)), 2, stroke, StrokeKind::Middle);
    for offset in [-5.0, 0.0, 5.0] {
        painter.rect_filled(Rect::from_center_size(centre + vec2(offset, -1.5), vec2(3.0, 5.0)), 0, color);
        painter.line_segment([centre + vec2(offset, 4.0), centre + vec2(offset, 7.0)], stroke);
    }
}

pub fn disk(painter: &Painter, centre: Pos2, color: Color32) {
    let stroke = Stroke::new(LINE_WIDTH, color);
    painter.rect_stroke(Rect::from_center_size(centre, vec2(18.0, 14.0)), 3, stroke, StrokeKind::Middle);
    painter.line_segment([centre + vec2(-9.0, 2.0), centre + vec2(9.0, 2.0)], stroke);
    painter.circle_filled(centre + vec2(5.0, 4.5), 1.2, color);
}

pub fn arrow_down(painter: &Painter, centre: Pos2, color: Color32) {
    arrow(painter, centre, color, 1.0);
}

pub fn arrow_up(painter: &Painter, centre: Pos2, color: Color32) {
    arrow(painter, centre, color, -1.0);
}

/// Vertical arrow; `direction` is 1 for down, -1 for up.
fn arrow(painter: &Painter, centre: Pos2, color: Color32, direction: f32) {
    let stroke = Stroke::new(LINE_WIDTH, color);
    let tip = centre + vec2(0.0, 8.0 * direction);
    painter.line_segment([centre - vec2(0.0, 8.0 * direction), tip], stroke);
    painter.line_segment([tip, tip + vec2(-6.0, -6.0 * direction)], stroke);
    painter.line_segment([tip, tip + vec2(6.0, -6.0 * direction)], stroke);
}
