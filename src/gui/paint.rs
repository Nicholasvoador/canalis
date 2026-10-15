//! Drawing helpers over GPUI's low-level paint calls, for the parts of the window drawn by hand.

use super::geometry::{Area, to_pixels};
use gpui_kit::*;

/// How text sits against the point it is drawn at.
#[derive(Clone, Copy, PartialEq)]
pub enum Anchor {
    LeftTop,
    LeftCentre,
    LeftBottom,
    CentreTop,
    CentreCentre,
    CentreBottom,
    RightCentre,
}

pub struct Label<'a> {
    pub text: &'a str,
    pub size: f32,
    pub color: Hsla,
    pub bold: bool,
}

impl<'a> Label<'a> {
    pub fn new(text: &'a str, size: f32, color: impl Into<Hsla>) -> Self {
        Self { text, size, color: color.into(), bold: false }
    }

    pub fn bold(self) -> Self {
        Self { bold: true, ..self }
    }
}

const LINE_HEIGHT: f32 = 1.2;

pub fn polyline(window: &mut Window, points: &[Point<f32>], width: f32, color: impl Into<Background>) {
    let mut path = PathBuilder::stroke(px(width));
    path.add_polygon(&points.iter().map(|spot| to_pixels(*spot)).collect::<Vec<_>>(), false);
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

pub fn polygon(window: &mut Window, points: &[Point<f32>], color: impl Into<Background>) {
    let mut path = PathBuilder::fill();
    path.add_polygon(&points.iter().map(|spot| to_pixels(*spot)).collect::<Vec<_>>(), true);
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

pub fn circle(window: &mut Window, centre: Point<f32>, radius: f32, color: impl Into<Background>) {
    rounded_rect(window, Area::centred(centre, radius * 2.0, radius * 2.0), radius, color);
}

pub fn ring(window: &mut Window, centre: Point<f32>, radius: f32, width: f32, color: impl Into<Hsla>) {
    rect_outline(window, Area::centred(centre, radius * 2.0, radius * 2.0), radius, width, color);
}

pub fn rounded_rect(window: &mut Window, area: Area, radius: f32, color: impl Into<Background>) {
    window.paint_quad(fill(area.bounds(), color).corner_radii(px(radius)));
}

pub fn rect_outline(window: &mut Window, area: Area, radius: f32, width: f32, color: impl Into<Hsla>) {
    window.paint_quad(outline(area.bounds(), color, BorderStyle::Solid).corner_radii(px(radius)).border_widths(px(width)));
}

pub fn line(window: &mut Window, from: Point<f32>, to: Point<f32>, width: f32, color: impl Into<Background>) {
    polyline(window, &[from, to], width, color);
}

/// The width and height `label` takes up.
pub fn measure(window: &Window, label: &Label) -> (f32, f32) {
    let shaped = shape(window, label);
    (shaped.width.into(), label.size * LINE_HEIGHT)
}

/// Draws one line of text and returns the area it covers.
pub fn text(window: &mut Window, cx: &mut App, at: Point<f32>, anchor: Anchor, label: &Label) -> Area {
    let shaped = shape(window, label);
    let (width, height) = (f32::from(shaped.width), label.size * LINE_HEIGHT);
    let area = anchored_area(at, anchor, width, height);
    let _ = shaped.paint(to_pixels(point(area.left, area.top)), px(height), TextAlign::Left, None, window, cx);
    area
}

fn shape(window: &Window, label: &Label) -> ShapedLine {
    let mut font = window.text_style().font();
    if label.bold {
        font.weight = FontWeight::BOLD;
    }
    let run = TextRun { len: label.text.len(), font, color: label.color, background_color: None, underline: None, strikethrough: None };
    window.text_system().shape_line(SharedString::from(label.text.to_string()), px(label.size), &[run], None)
}

fn anchored_area(at: Point<f32>, anchor: Anchor, width: f32, height: f32) -> Area {
    let left = match anchor {
        Anchor::LeftTop | Anchor::LeftCentre | Anchor::LeftBottom => at.x,
        Anchor::CentreTop | Anchor::CentreCentre | Anchor::CentreBottom => at.x - width / 2.0,
        Anchor::RightCentre => at.x - width,
    };
    let top = match anchor {
        Anchor::LeftTop | Anchor::CentreTop => at.y,
        Anchor::LeftCentre | Anchor::CentreCentre | Anchor::RightCentre => at.y - height / 2.0,
        Anchor::LeftBottom | Anchor::CentreBottom => at.y - height,
    };
    Area::new(left, top, width, height)
}
