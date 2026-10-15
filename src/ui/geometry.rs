//! Plain rectangles and points for the painted parts of the window, so their layout maths can be
//! tested without a window.

use gpui_kit::{Bounds, Pixels, Point, point, px, size};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Area {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl Area {
    pub const fn new(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self { left, top, width, height }
    }

    pub fn from_corners(min: Point<f32>, max: Point<f32>) -> Self {
        Self::new(min.x, min.y, max.x - min.x, max.y - min.y)
    }

    pub fn centred(centre: Point<f32>, width: f32, height: f32) -> Self {
        Self::new(centre.x - width / 2.0, centre.y - height / 2.0, width, height)
    }

    pub fn from_bounds(bounds: Bounds<Pixels>) -> Self {
        Self::new(bounds.origin.x.into(), bounds.origin.y.into(), bounds.size.width.into(), bounds.size.height.into())
    }

    pub fn bounds(self) -> Bounds<Pixels> {
        Bounds { origin: point(px(self.left), px(self.top)), size: size(px(self.width), px(self.height)) }
    }

    pub fn right(self) -> f32 {
        self.left + self.width
    }

    pub fn bottom(self) -> f32 {
        self.top + self.height
    }

    pub fn centre(self) -> Point<f32> {
        point(self.left + self.width / 2.0, self.top + self.height / 2.0)
    }

    pub fn shrink(self, amount: f32) -> Self {
        Self::new(self.left + amount, self.top + amount, self.width - 2.0 * amount, self.height - 2.0 * amount)
    }

    pub fn translate(self, x: f32, y: f32) -> Self {
        Self::new(self.left + x, self.top + y, self.width, self.height)
    }

    pub fn contains(self, spot: Point<f32>) -> bool {
        (self.left..=self.right()).contains(&spot.x) && (self.top..=self.bottom()).contains(&spot.y)
    }

    /// The smallest area covering both.
    pub fn union(self, other: Self) -> Self {
        let min = point(self.left.min(other.left), self.top.min(other.top));
        let max = point(self.right().max(other.right()), self.bottom().max(other.bottom()));
        Self::from_corners(min, max)
    }
}

pub fn to_pixels(spot: Point<f32>) -> Point<Pixels> {
    point(px(spot.x), px(spot.y))
}

pub fn from_pixels(spot: Point<Pixels>) -> Point<f32> {
    point(spot.x.into(), spot.y.into())
}
