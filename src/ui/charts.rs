//! Painted dashboard charts in KANALI's style: sparkline, load gauge and gradient bar.

use super::theme::{CARD, CARD_RAISED, LIME};
use eframe::egui::{Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

const MIN_SPARKLINE_PEAK: f32 = 1.0;
const SMOOTHING_STEPS: usize = 6;
const GAUGE_START_DEGREES: f32 = 200.0;
const GAUGE_SWEEP_DEGREES: f32 = 220.0;
const GAUGE_ARC_WIDTH: f32 = 7.0;
const GAUGE_SEGMENTS: usize = 72;
const GAUGE_TRACK: Color32 = Color32::from_rgb(0x34, 0x37, 0x3D);
const BAR_RADIUS: f32 = 6.0;

/// A scrolling line graph. Samples run oldest (left) to newest (right); `scroll` goes from 0 to 1
/// while the next sample is awaited, sliding the graph one step left so each new sample glides in
/// from the right edge instead of the whole line jumping.
pub struct Sparkline<'a> {
    pub samples: &'a [f32],
    /// The value drawn at the top of the graph.
    pub peak: f32,
    pub scroll: f32,
    pub color: Color32,
}

/// The top of the graph for these samples: the largest one, or a small floor when all are quiet.
pub fn sparkline_peak(samples: &[f32]) -> f32 {
    samples.iter().copied().fold(MIN_SPARKLINE_PEAK, f32::max)
}

/// Smoothed line with a fading fill under it, clipped to `rect`.
pub fn sparkline(painter: &Painter, rect: Rect, line: &Sparkline) {
    if line.samples.len() < 3 {
        return;
    }
    let points = smooth(&sparkline_points(rect, line), rect.bottom());
    let clipped = painter.with_clip_rect(rect.expand2(vec2(0.0, 2.0)).intersect(painter.clip_rect()));
    clipped.add(fill_under(&points, rect.bottom(), line.color));
    clipped.add(Shape::line(points, Stroke::new(2.0, line.color)));
}

/// The newest sample sits one step past the right edge at `scroll` 0 and on it at 1.
fn sparkline_points(rect: Rect, line: &Sparkline) -> Vec<Pos2> {
    let step = rect.width() / (line.samples.len() - 2) as f32;
    let peak = line.peak.max(MIN_SPARKLINE_PEAK);
    let point = |index: usize, value: f32| pos2(rect.left() + step * (index as f32 - line.scroll), rect.bottom() - rect.height() * (value / peak).min(1.0));
    line.samples.iter().enumerate().map(|(index, value)| point(index, *value)).collect()
}

/// Catmull-Rom curve through the points, kept above `floor` so it never dips below the baseline.
fn smooth(points: &[Pos2], floor: f32) -> Vec<Pos2> {
    let at = |index: isize| points[index.clamp(0, points.len() as isize - 1) as usize];
    let mut curve = Vec::with_capacity(points.len() * SMOOTHING_STEPS);
    for index in 0..points.len() as isize - 1 {
        let (before, start, end, after) = (at(index - 1), at(index), at(index + 1), at(index + 2));
        for step in 0..SMOOTHING_STEPS {
            let point = catmull_rom([before, start, end, after], step as f32 / SMOOTHING_STEPS as f32);
            curve.push(pos2(point.x, point.y.min(floor)));
        }
    }
    curve.push(*points.last().expect("at least two points"));
    curve
}

fn catmull_rom([p0, p1, p2, p3]: [Pos2; 4], t: f32) -> Pos2 {
    let (t2, t3) = (t * t, t * t * t);
    let blend = |a: f32, b: f32, c: f32, d: f32| 0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (3.0 * b - a - 3.0 * c + d) * t3);
    pos2(blend(p0.x, p1.x, p2.x, p3.x), blend(p0.y, p1.y, p2.y, p3.y))
}

fn fill_under(points: &[Pos2], bottom: f32, color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    for point in points {
        mesh.colored_vertex(*point, color.gamma_multiply(0.35));
        mesh.colored_vertex(pos2(point.x, bottom), Color32::TRANSPARENT);
    }
    for top_left in (0..points.len() as u32 - 1).map(|index| index * 2) {
        mesh.add_triangle(top_left, top_left + 1, top_left + 2);
        mesh.add_triangle(top_left + 1, top_left + 3, top_left + 2);
    }
    mesh
}

/// The largest gauge that fits in `area`, centred horizontally. `fraction` is 0..=1.
pub fn gauge(painter: &Painter, area: Rect, fraction: f32) {
    let below_centre = (GAUGE_START_DEGREES - 180.0).to_radians().sin();
    let radius = (area.width() / 2.0).min((area.height() - GAUGE_ARC_WIDTH) / (1.0 + below_centre)).max(10.0);
    let centre = pos2(area.center().x, area.top() + radius + GAUGE_ARC_WIDTH / 2.0);
    let fraction = fraction.clamp(0.0, 1.0);
    painter.add(gauge_face(centre, radius - GAUGE_ARC_WIDTH));
    rounded_arc(painter, Arc { centre, radius, end_fraction: 1.0 }, GAUGE_TRACK);
    if fraction > 0.0 {
        rounded_arc(painter, Arc { centre, radius, end_fraction: fraction }, LIME);
    }
    needle(painter, centre, radius, fraction);
}

/// Angle in radians for a gauge reading; 0 is lower left, 0.5 straight up, 1 lower right.
fn gauge_angle(fraction: f32) -> f32 {
    (GAUGE_START_DEGREES - fraction * GAUGE_SWEEP_DEGREES).to_radians()
}

fn point_on_circle(centre: Pos2, radius: f32, angle: f32) -> Pos2 {
    centre + vec2(angle.cos(), -angle.sin()) * radius
}

struct Arc {
    centre: Pos2,
    radius: f32,
    end_fraction: f32,
}

fn rounded_arc(painter: &Painter, arc: Arc, color: Color32) {
    let segments = ((GAUGE_SEGMENTS as f32 * arc.end_fraction).ceil() as usize).max(2);
    let points: Vec<Pos2> =
        (0..=segments).map(|step| point_on_circle(arc.centre, arc.radius, gauge_angle(arc.end_fraction * step as f32 / segments as f32))).collect();
    for end in [points[0], points[segments]] {
        painter.circle_filled(end, GAUGE_ARC_WIDTH / 2.0, color);
    }
    painter.add(Shape::line(points, Stroke::new(GAUGE_ARC_WIDTH, color)));
}

fn gauge_face(centre: Pos2, radius: f32) -> Mesh {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, CARD);
    for step in 0..=GAUGE_SEGMENTS {
        let angle = gauge_angle(step as f32 / GAUGE_SEGMENTS as f32);
        mesh.colored_vertex(point_on_circle(centre, radius, angle), lerp_color(CARD, LIME, 0.10));
        if step > 0 {
            mesh.add_triangle(0, step as u32, step as u32 + 1);
        }
    }
    mesh
}

fn needle(painter: &Painter, centre: Pos2, radius: f32, fraction: f32) {
    let angle = gauge_angle(fraction);
    let tip = point_on_circle(centre, radius * 0.62, angle);
    let side = vec2(angle.sin(), angle.cos()) * 3.5;
    painter.add(Shape::convex_polygon(vec![centre + side, tip, centre - side], LIME, Stroke::NONE));
    painter.circle_filled(centre, 6.0, LIME);
    painter.circle_filled(centre, 2.5, CARD);
}

/// How much of a bar is filled and the colours its gradient runs through, left to right.
pub struct GradientFill {
    pub fraction: f32,
    pub stops: &'static [Color32],
}

pub fn gradient_bar(painter: &Painter, rect: Rect, fill: &GradientFill) {
    painter.rect_filled(rect, BAR_RADIUS, CARD_RAISED);
    let filled = Rect::from_min_size(rect.min, vec2(rect.width() * fill.fraction.clamp(0.0, 1.0), rect.height()));
    if filled.width() >= 1.0 {
        painter.add(rounded_gradient(filled, fill.stops));
    }
}

/// A rounded rectangle built from thin vertical strips, each coloured from `stops`.
fn rounded_gradient(rect: Rect, stops: &[Color32]) -> Mesh {
    let radius = BAR_RADIUS.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let columns = (rect.width() / 2.0).ceil().max(1.0) as u32;
    let mut mesh = Mesh::default();
    for column in 0..=columns {
        let progress = column as f32 / columns as f32;
        let x = rect.left() + rect.width() * progress;
        let inset = corner_inset((x - rect.left()).min(rect.right() - x), radius);
        let color = color_at(stops, progress);
        mesh.colored_vertex(pos2(x, rect.top() + inset), color);
        mesh.colored_vertex(pos2(x, rect.bottom() - inset), color);
        if column > 0 {
            let top = column * 2;
            mesh.add_triangle(top - 2, top - 1, top);
            mesh.add_triangle(top - 1, top + 1, top);
        }
    }
    mesh
}

/// How far a rounded corner pulls the top and bottom edges in, `distance` from the nearest end.
fn corner_inset(distance: f32, radius: f32) -> f32 {
    if distance >= radius {
        return 0.0;
    }
    let offset = radius - distance;
    radius - (radius * radius - offset * offset).max(0.0).sqrt()
}

/// Colour at `progress` (0..=1) along evenly spaced gradient stops.
fn color_at(stops: &[Color32], progress: f32) -> Color32 {
    let scaled = progress.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let index = (scaled.floor() as usize).min(stops.len().saturating_sub(2));
    match stops {
        [only] => *only,
        _ => lerp_color(stops[index], stops[index + 1], scaled - index as f32),
    }
}

fn lerp_color(from: Color32, to: Color32, amount: f32) -> Color32 {
    let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount).round() as u8;
    Color32::from_rgba_premultiplied(channel(from.r(), to.r()), channel(from.g(), to.g()), channel(from.b(), to.b()), channel(from.a(), to.a()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(samples: &[f32], scroll: f32) -> Sparkline<'_> {
        Sparkline { samples, peak: sparkline_peak(samples), scroll, color: Color32::WHITE }
    }

    #[test]
    fn sparkline_scales_the_peak_to_the_top() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
        let points = sparkline_points(rect, &line(&[0.0, 10.0, 5.0], 1.0));
        assert_eq!(points, vec![pos2(-100.0, 50.0), pos2(0.0, 0.0), pos2(100.0, 25.0)]);
    }

    #[test]
    fn new_samples_scroll_in_from_the_right_edge() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
        let arriving = sparkline_points(rect, &line(&[0.0, 10.0, 5.0], 0.0));
        assert_eq!(arriving[0].x, 0.0);
        assert_eq!(arriving[2].x, 200.0);
        assert_eq!(sparkline_points(rect, &line(&[0.0, 10.0, 5.0], 0.5))[2].x, 150.0);
    }

    #[test]
    fn quiet_sparkline_stays_near_the_baseline() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
        assert_eq!(sparkline_points(rect, &line(&[0.0, 0.0, 0.1], 1.0))[2], pos2(100.0, 45.0));
    }

    #[test]
    fn values_above_an_easing_peak_stay_inside_the_graph() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
        let samples = [0.0, 0.0, 20.0];
        let points = sparkline_points(rect, &Sparkline { samples: &samples, peak: 10.0, scroll: 1.0, color: Color32::WHITE });
        assert_eq!(points[2].y, 0.0);
    }

    #[test]
    fn smoothing_passes_through_samples_and_stays_above_the_floor() {
        let points = [pos2(0.0, 50.0), pos2(10.0, 0.0), pos2(20.0, 50.0), pos2(30.0, 50.0)];
        let curve = smooth(&points, 50.0);
        assert_eq!(curve[SMOOTHING_STEPS], points[1]);
        assert_eq!(*curve.last().unwrap(), points[3]);
        assert!(curve.iter().all(|point| point.y <= 50.0));
    }

    #[test]
    fn gauge_runs_from_lower_left_over_the_top_to_lower_right() {
        assert!((gauge_angle(0.0).to_degrees() - 200.0).abs() < 1e-4);
        assert!((gauge_angle(0.5).to_degrees() - 90.0).abs() < 1e-4);
        assert!((gauge_angle(1.0).to_degrees() + 20.0).abs() < 1e-4);
    }

    #[test]
    fn gradient_colours_blend_between_stops() {
        let stops = [Color32::from_rgb(0, 0, 0), Color32::from_rgb(200, 100, 0), Color32::from_rgb(200, 0, 100)];
        assert_eq!(color_at(&stops, 0.0), stops[0]);
        assert_eq!(color_at(&stops, 0.25), Color32::from_rgb(100, 50, 0));
        assert_eq!(color_at(&stops, 1.0), stops[2]);
    }

    #[test]
    fn rounded_corners_pull_in_only_near_the_ends() {
        assert_eq!(corner_inset(0.0, 6.0), 6.0);
        assert_eq!(corner_inset(6.0, 6.0), 0.0);
        assert!(corner_inset(3.0, 6.0) > 0.0);
    }
}
