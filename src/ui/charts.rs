//! Painted dashboard charts in KANALI's style: sparkline, load gauge and gradient bar.

use super::geometry::Area;
use super::paint::{circle, polygon, polyline, rounded_rect};
use super::theme::{CARD, CARD_RAISED, LIME, faded, mix};
use gpui_kit::*;

const MIN_SPARKLINE_PEAK: f32 = 1.0;
const SMOOTHING_STEPS: usize = 6;
const GAUGE_START_DEGREES: f32 = 200.0;
const GAUGE_SWEEP_DEGREES: f32 = 220.0;
const GAUGE_ARC_WIDTH: f32 = 7.0;
const GAUGE_SEGMENTS: usize = 72;
const GAUGE_TRACK: u32 = 0x34373D;
const BAR_RADIUS: f32 = 6.0;

/// A scrolling line graph. Samples run oldest (left) to newest (right); `scroll` goes from 0 to 1
/// while the next sample is awaited, sliding the graph one step left so each new sample glides in
/// from the right edge instead of the whole line jumping.
pub struct Sparkline<'a> {
    pub samples: &'a [f32],
    /// The value drawn at the top of the graph.
    pub peak: f32,
    pub scroll: f32,
    pub color: u32,
}

/// The top of the graph for these samples: the largest one, or a small floor when all are quiet.
pub fn sparkline_peak(samples: &[f32]) -> f32 {
    samples.iter().copied().fold(MIN_SPARKLINE_PEAK, f32::max)
}

/// Smoothed line with a fading fill under it, clipped to `area`.
pub fn sparkline(window: &mut Window, area: Area, line: &Sparkline) {
    if line.samples.len() < 3 {
        return;
    }
    let points = smooth(&sparkline_points(area, line), area.bottom());
    let clip = Area::new(area.left, area.top - 2.0, area.width, area.height + 4.0);
    window.with_content_mask(Some(ContentMask { bounds: clip.bounds() }), |window| {
        let mut under = points.clone();
        under.extend([point(area.right() + area.width, area.bottom()), point(points[0].x, area.bottom())]);
        let fade = linear_gradient(180.0, linear_color_stop(faded(line.color, 0.35), 0.0), linear_color_stop(faded(line.color, 0.0), 1.0));
        polygon(window, &under, fade);
        polyline(window, &points, 2.0, rgb(line.color));
    });
}

/// The newest sample sits one step past the right edge at `scroll` 0 and on it at 1.
fn sparkline_points(area: Area, line: &Sparkline) -> Vec<Point<f32>> {
    let step = area.width / (line.samples.len() - 2) as f32;
    let peak = line.peak.max(MIN_SPARKLINE_PEAK);
    let spot = |index: usize, value: f32| point(area.left + step * (index as f32 - line.scroll), area.bottom() - area.height * (value / peak).min(1.0));
    line.samples.iter().enumerate().map(|(index, value)| spot(index, *value)).collect()
}

/// Catmull-Rom curve through the points, kept above `floor` so it never dips below the baseline.
fn smooth(points: &[Point<f32>], floor: f32) -> Vec<Point<f32>> {
    let at = |index: isize| points[index.clamp(0, points.len() as isize - 1) as usize];
    let mut curve = Vec::with_capacity(points.len() * SMOOTHING_STEPS);
    for index in 0..points.len() as isize - 1 {
        let (before, start, end, after) = (at(index - 1), at(index), at(index + 1), at(index + 2));
        for step in 0..SMOOTHING_STEPS {
            let spot = catmull_rom([before, start, end, after], step as f32 / SMOOTHING_STEPS as f32);
            curve.push(point(spot.x, spot.y.min(floor)));
        }
    }
    curve.push(*points.last().expect("at least two points"));
    curve
}

fn catmull_rom([p0, p1, p2, p3]: [Point<f32>; 4], t: f32) -> Point<f32> {
    let (t2, t3) = (t * t, t * t * t);
    let blend = |a: f32, b: f32, c: f32, d: f32| 0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (3.0 * b - a - 3.0 * c + d) * t3);
    point(blend(p0.x, p1.x, p2.x, p3.x), blend(p0.y, p1.y, p2.y, p3.y))
}

/// The largest gauge that fits in `area`, centred horizontally. `fraction` is 0..=1.
pub fn gauge(window: &mut Window, area: Area, fraction: f32) {
    let below_centre = (GAUGE_START_DEGREES - 180.0).to_radians().sin();
    let radius = (area.width / 2.0).min((area.height - GAUGE_ARC_WIDTH) / (1.0 + below_centre)).max(10.0);
    let centre = point(area.centre().x, area.top + radius + GAUGE_ARC_WIDTH / 2.0);
    let fraction = fraction.clamp(0.0, 1.0);
    gauge_face(window, centre, radius - GAUGE_ARC_WIDTH);
    rounded_arc(window, Arc { centre, radius, end_fraction: 1.0 }, GAUGE_TRACK);
    if fraction > 0.0 {
        rounded_arc(window, Arc { centre, radius, end_fraction: fraction }, LIME);
    }
    needle(window, centre, radius, fraction);
}

/// Angle in radians for a gauge reading; 0 is lower left, 0.5 straight up, 1 lower right.
fn gauge_angle(fraction: f32) -> f32 {
    (GAUGE_START_DEGREES - fraction * GAUGE_SWEEP_DEGREES).to_radians()
}

fn point_on_circle(centre: Point<f32>, radius: f32, angle: f32) -> Point<f32> {
    point(centre.x + angle.cos() * radius, centre.y - angle.sin() * radius)
}

struct Arc {
    centre: Point<f32>,
    radius: f32,
    end_fraction: f32,
}

fn rounded_arc(window: &mut Window, arc: Arc, color: u32) {
    let segments = ((GAUGE_SEGMENTS as f32 * arc.end_fraction).ceil() as usize).max(2);
    let points: Vec<Point<f32>> =
        (0..=segments).map(|step| point_on_circle(arc.centre, arc.radius, gauge_angle(arc.end_fraction * step as f32 / segments as f32))).collect();
    for end in [points[0], points[segments]] {
        circle(window, end, GAUGE_ARC_WIDTH / 2.0, rgb(color));
    }
    polyline(window, &points, GAUGE_ARC_WIDTH, rgb(color));
}

/// The faint lime glow inside the arc, brighter towards the top.
fn gauge_face(window: &mut Window, centre: Point<f32>, radius: f32) {
    let mut sector = vec![centre];
    sector.extend((0..=GAUGE_SEGMENTS).map(|step| point_on_circle(centre, radius, gauge_angle(step as f32 / GAUGE_SEGMENTS as f32))));
    let glow = linear_gradient(180.0, linear_color_stop(mix(CARD, LIME, 0.10), 0.0), linear_color_stop(rgb(CARD), 1.0));
    polygon(window, &sector, glow);
}

fn needle(window: &mut Window, centre: Point<f32>, radius: f32, fraction: f32) {
    let angle = gauge_angle(fraction);
    let tip = point_on_circle(centre, radius * 0.62, angle);
    let side = point(angle.sin() * 3.5, angle.cos() * 3.5);
    polygon(window, &[centre + side, tip, centre - side], rgb(LIME));
    circle(window, centre, 6.0, rgb(LIME));
    circle(window, centre, 2.5, rgb(CARD));
}

/// How much of a bar is filled and the colours its gradient runs through, left to right.
pub struct GradientFill {
    pub fraction: f32,
    pub stops: &'static [u32],
}

/// A rounded bar whose filled part runs through the gradient's colours. GPUI gradients have two
/// stops, so each pair of neighbouring colours is painted into its own slice of the filled part.
pub fn gradient_bar(window: &mut Window, area: Area, fill: &GradientFill) {
    rounded_rect(window, area, BAR_RADIUS, rgb(CARD_RAISED));
    let filled = Area { width: area.width * fill.fraction.clamp(0.0, 1.0), ..area };
    if filled.width < 1.0 {
        return;
    }
    let radius = BAR_RADIUS.min(filled.width / 2.0);
    let slices = fill.stops.len().saturating_sub(1).max(1);
    for slice in 0..slices {
        let (start, end) = (slice as f32 / slices as f32, (slice + 1) as f32 / slices as f32);
        let (from, to) = (fill.stops[slice], fill.stops[(slice + 1).min(fill.stops.len() - 1)]);
        let clip = Area { left: filled.left + filled.width * start, width: filled.width * (end - start), ..filled };
        let colours = linear_gradient(90.0, linear_color_stop(rgb(from), start), linear_color_stop(rgb(to), end));
        window.with_content_mask(Some(ContentMask { bounds: clip.bounds() }), |window| rounded_rect(window, filled, radius, colours));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // GPUI's own `test` attribute comes in with `gpui_kit::*`; these are plain tests.
    use core::prelude::v1::test;

    fn line(samples: &[f32], scroll: f32) -> Sparkline<'_> {
        Sparkline { samples, peak: sparkline_peak(samples), scroll, color: 0xFFFFFF }
    }

    fn graph() -> Area {
        Area::new(0.0, 0.0, 100.0, 50.0)
    }

    #[test]
    fn sparkline_scales_the_peak_to_the_top() {
        let points = sparkline_points(graph(), &line(&[0.0, 10.0, 5.0], 1.0));
        assert_eq!(points, vec![point(-100.0, 50.0), point(0.0, 0.0), point(100.0, 25.0)]);
    }

    #[test]
    fn new_samples_scroll_in_from_the_right_edge() {
        let arriving = sparkline_points(graph(), &line(&[0.0, 10.0, 5.0], 0.0));
        assert_eq!(arriving[0].x, 0.0);
        assert_eq!(arriving[2].x, 200.0);
        assert_eq!(sparkline_points(graph(), &line(&[0.0, 10.0, 5.0], 0.5))[2].x, 150.0);
    }

    #[test]
    fn quiet_sparkline_stays_near_the_baseline() {
        assert_eq!(sparkline_points(graph(), &line(&[0.0, 0.0, 0.1], 1.0))[2], point(100.0, 45.0));
    }

    #[test]
    fn values_above_an_easing_peak_stay_inside_the_graph() {
        let samples = [0.0, 0.0, 20.0];
        let points = sparkline_points(graph(), &Sparkline { samples: &samples, peak: 10.0, scroll: 1.0, color: 0xFFFFFF });
        assert_eq!(points[2].y, 0.0);
    }

    #[test]
    fn smoothing_passes_through_samples_and_stays_above_the_floor() {
        let points = [point(0.0, 50.0), point(10.0, 0.0), point(20.0, 50.0), point(30.0, 50.0)];
        let curve = smooth(&points, 50.0);
        assert_eq!(curve[SMOOTHING_STEPS], points[1]);
        assert_eq!(*curve.last().unwrap(), points[3]);
        assert!(curve.iter().all(|spot| spot.y <= 50.0));
    }

    #[test]
    fn gauge_runs_from_lower_left_over_the_top_to_lower_right() {
        assert!((gauge_angle(0.0).to_degrees() - 200.0).abs() < 1e-4);
        assert!((gauge_angle(0.5).to_degrees() - 90.0).abs() < 1e-4);
        assert!((gauge_angle(1.0).to_degrees() + 20.0).abs() < 1e-4);
    }
}
