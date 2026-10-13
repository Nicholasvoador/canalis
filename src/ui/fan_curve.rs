//! Interactive fan curve: drag points to set fan % for each CPU temperature.

use super::theme::{BORDER, LIME, MUTED, TEXT};
use crate::settings::FanCurve;
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};

const POINT_RADIUS: f32 = 6.0;
const GRAB_SIZE: f32 = 20.0;

/// Returns true while a point is being dragged (so saving can wait for release).
pub fn fan_curve_editor(ui: &mut Ui, curve: &mut FanCurve, current_temperature: Option<f32>) -> bool {
    let size = vec2(ui.available_width().min(760.0), 300.0);
    let (response, painter) = ui.allocate_painter(size, Sense::hover());
    let plot = Rect::from_min_max(response.rect.min + vec2(44.0, 12.0), response.rect.max - vec2(16.0, 32.0));
    draw_grid(&painter, plot);
    if let Some(temperature) = current_temperature {
        draw_current_temperature(&painter, plot, temperature);
    }
    let dragging = drag_points(ui, response.id, plot, curve);
    draw_curve(&painter, plot, curve);
    dragging
}

fn to_screen(plot: Rect, [temperature, percent]: [u8; 2]) -> Pos2 {
    pos2(plot.left() + plot.width() * temperature as f32 / 100.0, plot.bottom() - plot.height() * percent as f32 / 100.0)
}

fn draw_grid(painter: &egui::Painter, plot: Rect) {
    let label_font = FontId::proportional(12.0);
    for step in (0..=100).step_by(20) {
        let fraction = step as f32 / 100.0;
        let x = plot.left() + plot.width() * fraction;
        let y = plot.bottom() - plot.height() * fraction;
        painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], Stroke::new(1.0, BORDER));
        painter.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], Stroke::new(1.0, BORDER));
        painter.text(pos2(x, plot.bottom() + 8.0), Align2::CENTER_TOP, format!("{step}°C"), label_font.clone(), MUTED);
        painter.text(pos2(plot.left() - 8.0, y), Align2::RIGHT_CENTER, format!("{step}%"), label_font.clone(), MUTED);
    }
}

fn draw_current_temperature(painter: &egui::Painter, plot: Rect, temperature: f32) {
    let x = plot.left() + plot.width() * temperature.clamp(0.0, 100.0) / 100.0;
    painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], Stroke::new(1.5, TEXT.gamma_multiply(0.5)));
    painter.text(pos2(x + 6.0, plot.top() + 4.0), Align2::LEFT_TOP, format!("CPU {temperature:.0}°C"), FontId::proportional(12.0), TEXT);
}

fn draw_curve(painter: &egui::Painter, plot: Rect, curve: &FanCurve) {
    let points: Vec<Pos2> = curve.iter().map(|point| to_screen(plot, *point)).collect();
    for segment in points.windows(2) {
        let quad = vec![segment[0], segment[1], pos2(segment[1].x, plot.bottom()), pos2(segment[0].x, plot.bottom())];
        painter.add(egui::Shape::convex_polygon(quad, LIME.gamma_multiply(0.08), Stroke::NONE));
    }
    painter.line(points.clone(), Stroke::new(2.5, LIME));
    for point in points {
        painter.circle(point, POINT_RADIUS, super::theme::BLACK, Stroke::new(2.5, LIME));
    }
}

fn drag_points(ui: &mut Ui, id: egui::Id, plot: Rect, curve: &mut FanCurve) -> bool {
    let mut dragging = false;
    for index in 0..curve.len() {
        let center = to_screen(plot, curve[index]);
        let handle = ui.interact(Rect::from_center_size(center, vec2(GRAB_SIZE, GRAB_SIZE)), id.with(index), Sense::drag());
        if handle.hovered() || handle.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            show_point_label(ui, center, curve[index]);
        }
        if let (true, Some(pointer)) = (handle.dragged(), handle.interact_pointer_pos()) {
            move_point(curve, index, plot, pointer);
            dragging = true;
        }
    }
    dragging
}

/// First and last points keep their temperature; others stay between their neighbours.
fn move_point(curve: &mut FanCurve, index: usize, plot: Rect, pointer: Pos2) {
    let percent = ((plot.bottom() - pointer.y) / plot.height() * 100.0).round().clamp(0.0, 100.0) as u8;
    curve[index][1] = percent;
    let is_end = index == 0 || index + 1 == curve.len();
    if !is_end {
        let lowest = curve[index - 1][0] + 1;
        let highest = curve[index + 1][0].saturating_sub(1).max(lowest);
        let temperature = ((pointer.x - plot.left()) / plot.width() * 100.0).round().clamp(0.0, 100.0) as u8;
        curve[index][0] = temperature.clamp(lowest, highest);
    }
}

fn show_point_label(ui: &Ui, center: Pos2, [temperature, percent]: [u8; 2]) {
    ui.painter().text(center + vec2(0.0, -16.0), Align2::CENTER_BOTTOM, format!("{temperature}°C → {percent}%"), FontId::proportional(13.0), TEXT);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plot() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0))
    }

    #[test]
    fn middle_points_stay_between_neighbours() {
        let mut curve: FanCurve = vec![[0, 10], [30, 30], [50, 40], [100, 100]];
        move_point(&mut curve, 1, plot(), pos2(80.0, 25.0));
        assert_eq!(curve[1], [49, 75]);
    }

    #[test]
    fn end_points_keep_their_temperature() {
        let mut curve: FanCurve = vec![[0, 10], [50, 40], [100, 100]];
        move_point(&mut curve, 2, plot(), pos2(40.0, 60.0));
        assert_eq!(curve[2], [100, 40]);
    }
}
