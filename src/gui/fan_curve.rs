//! Interactive fan curve: drag points to set fan % for each CPU temperature.

use super::geometry::{Area, from_pixels};
use super::paint::{Anchor, Label, circle, line, polygon, polyline, ring, text};
use super::pointer::follow_drag;
use super::theme::{BLACK, BORDER, LIME, MUTED, TEXT, faded};
use crate::settings::FanCurve;
use gpui_kit::*;
use std::cell::Cell;
use std::rc::Rc;

const MAX_WIDTH: f32 = 760.0;
const HEIGHT: f32 = 300.0;
const POINT_RADIUS: f32 = 6.0;
const GRAB_SIZE: f32 = 20.0;

#[derive(Default)]
pub struct FanCurveEditor {
    /// The point being dragged.
    pub dragging: Option<usize>,
    /// Where the plot was last drawn, for working out which point a press grabbed.
    plot: Rc<Cell<Area>>,
}

/// What happens while a point is dragged: `moved` gets the pointer's position, `released` the end.
pub struct DragHandlers<Moved, Released> {
    pub moved: Moved,
    pub released: Released,
}

impl FanCurveEditor {
    pub fn element<Moved, Released>(&self, width: f32, curve: &FanCurve, current_temperature: Option<f32>, handlers: DragHandlers<Moved, Released>) -> impl IntoElement + use<Moved, Released>
    where
        Moved: Fn(Point<f32>, &mut Window, &mut App) + 'static,
        Released: Fn(&mut Window, &mut App) + 'static,
    {
        let (curve, plot_cell, dragging) = (curve.clone(), self.plot.clone(), self.dragging);
        canvas(
            move |bounds, _, _| plot_cell.set(plot_area(Area::from_bounds(bounds))),
            move |bounds, (), window, cx| {
                let plot = plot_area(Area::from_bounds(bounds));
                draw_grid(window, cx, plot);
                if let Some(temperature) = current_temperature {
                    draw_current_temperature(window, cx, plot, temperature);
                }
                draw_curve(window, plot, &curve);
                let pointer = from_pixels(window.mouse_position());
                if let Some(index) = dragging.or_else(|| grabbed_point(plot, &curve, pointer)) {
                    show_point_label(window, cx, plot, curve[index]);
                }
                if dragging.is_some() {
                    follow_drag(window, handlers.moved, handlers.released);
                }
            },
        )
        .w(px(width.min(MAX_WIDTH)))
        .h(px(HEIGHT))
    }

    /// Starts dragging the point under `pointer`, if there is one.
    pub fn press(&mut self, curve: &FanCurve, pointer: Point<f32>) -> bool {
        self.dragging = grabbed_point(self.plot.get(), curve, pointer);
        self.dragging.is_some()
    }

    pub fn drag(&self, curve: &mut FanCurve, pointer: Point<f32>) {
        if let Some(index) = self.dragging {
            move_point(curve, index, self.plot.get(), pointer);
        }
    }
}

fn plot_area(bounds: Area) -> Area {
    Area::from_corners(point(bounds.left + 44.0, bounds.top + 12.0), point(bounds.right() - 16.0, bounds.bottom() - 32.0))
}

fn to_screen(plot: Area, [temperature, percent]: [u8; 2]) -> Point<f32> {
    point(plot.left + plot.width * temperature as f32 / 100.0, plot.bottom() - plot.height * percent as f32 / 100.0)
}

fn grabbed_point(plot: Area, curve: &FanCurve, pointer: Point<f32>) -> Option<usize> {
    (0..curve.len()).find(|&index| Area::centred(to_screen(plot, curve[index]), GRAB_SIZE, GRAB_SIZE).contains(pointer))
}

fn draw_grid(window: &mut Window, cx: &mut App, plot: Area) {
    for step in (0..=100).step_by(20) {
        let fraction = step as f32 / 100.0;
        let x = plot.left + plot.width * fraction;
        let y = plot.bottom() - plot.height * fraction;
        line(window, point(x, plot.top), point(x, plot.bottom()), 1.0, rgb(BORDER));
        line(window, point(plot.left, y), point(plot.right(), y), 1.0, rgb(BORDER));
        text(window, cx, point(x, plot.bottom() + 8.0), Anchor::CentreTop, &Label::new(&format!("{step}°C"), 12.0, rgb(MUTED)));
        text(window, cx, point(plot.left - 8.0, y), Anchor::RightCentre, &Label::new(&format!("{step}%"), 12.0, rgb(MUTED)));
    }
}

fn draw_current_temperature(window: &mut Window, cx: &mut App, plot: Area, temperature: f32) {
    let x = plot.left + plot.width * temperature.clamp(0.0, 100.0) / 100.0;
    line(window, point(x, plot.top), point(x, plot.bottom()), 1.5, faded(TEXT, 0.5));
    text(window, cx, point(x + 6.0, plot.top + 4.0), Anchor::LeftTop, &Label::new(&format!("CPU {temperature:.0}°C"), 12.0, rgb(TEXT)));
}

fn draw_curve(window: &mut Window, plot: Area, curve: &FanCurve) {
    let points: Vec<Point<f32>> = curve.iter().map(|spot| to_screen(plot, *spot)).collect();
    for segment in points.windows(2) {
        polygon(window, &[segment[0], segment[1], point(segment[1].x, plot.bottom()), point(segment[0].x, plot.bottom())], faded(LIME, 0.08));
    }
    polyline(window, &points, 2.5, rgb(LIME));
    for spot in points {
        circle(window, spot, POINT_RADIUS, rgb(BLACK));
        ring(window, spot, POINT_RADIUS + 1.25, 2.5, rgb(LIME));
    }
}

fn show_point_label(window: &mut Window, cx: &mut App, plot: Area, [temperature, percent]: [u8; 2]) {
    let spot = to_screen(plot, [temperature, percent]);
    text(window, cx, point(spot.x, spot.y - 16.0), Anchor::CentreBottom, &Label::new(&format!("{temperature}°C → {percent}%"), 13.0, rgb(TEXT)));
}

/// First and last points keep their temperature; others stay between their neighbours.
fn move_point(curve: &mut FanCurve, index: usize, plot: Area, pointer: Point<f32>) {
    let percent = ((plot.bottom() - pointer.y) / plot.height * 100.0).round().clamp(0.0, 100.0) as u8;
    curve[index][1] = percent;
    let is_end = index == 0 || index + 1 == curve.len();
    if !is_end {
        let lowest = curve[index - 1][0] + 1;
        let highest = curve[index + 1][0].saturating_sub(1).max(lowest);
        let temperature = ((pointer.x - plot.left) / plot.width * 100.0).round().clamp(0.0, 100.0) as u8;
        curve[index][0] = temperature.clamp(lowest, highest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // GPUI's own `test` attribute comes in with `gpui_kit::*`; these are plain tests.
    use core::prelude::v1::test;

    fn plot() -> Area {
        Area::new(0.0, 0.0, 100.0, 100.0)
    }

    #[test]
    fn middle_points_stay_between_neighbours() {
        let mut curve: FanCurve = vec![[0, 10], [30, 30], [50, 40], [100, 100]];
        move_point(&mut curve, 1, plot(), point(80.0, 25.0));
        assert_eq!(curve[1], [49, 75]);
    }

    #[test]
    fn end_points_keep_their_temperature() {
        let mut curve: FanCurve = vec![[0, 10], [50, 40], [100, 100]];
        move_point(&mut curve, 2, plot(), point(40.0, 60.0));
        assert_eq!(curve[2], [100, 40]);
    }

    #[test]
    fn a_press_grabs_the_point_under_the_pointer() {
        let curve: FanCurve = vec![[0, 10], [50, 40], [100, 100]];
        assert_eq!(grabbed_point(plot(), &curve, point(52.0, 58.0)), Some(1));
        assert_eq!(grabbed_point(plot(), &curve, point(30.0, 30.0)), None);
    }
}
