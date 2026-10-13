//! The app icon (window, taskbar, tray and panorama.exe): the sidebar's logo, assets/icons/logo.svg,
//! in black on a lime rounded square.

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};
use std::path::Path;

/// The largest size drawn: the window icon and the PNG for Linux launchers.
pub const LARGEST_SIZE: u32 = 256;
const DESIGN_SIZE: f32 = 32.0;
const LIME: [u8; 3] = [0xDE, 0xF7, 0x50];
const BLACK: [u8; 3] = [0x0B, 0x0B, 0x0C];
const BACKGROUND: RoundedRect = RoundedRect { centre: (16.0, 16.0), half_size: (16.0, 16.0), radius: 7.0 };
const LOGO: &str = include_str!("../assets/icons/logo.svg");
/// The logo's size on the tile, so it keeps a margin inside the rounded corners.
const LOGO_SCALE: f32 = 0.78;

/// `size`×`size` pixels, RGBA, row by row.
pub fn rgba(size: u32) -> Vec<u8> {
    let scale = DESIGN_SIZE / size as f32;
    let ink = logo_coverage(size);
    (0..size * size)
        .flat_map(|index| {
            let (x, y) = ((index % size) as f32 + 0.5, (index / size) as f32 + 0.5);
            pixel(coverage(BACKGROUND.distance(x * scale, y * scale), scale), ink[index as usize])
        })
        .collect()
}

/// Writes the icon as a `LARGEST_SIZE` PNG, for the Linux desktop entry.
pub fn save_png(path: &Path) -> Result<(), String> {
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    }
    image::save_buffer(path, &rgba(LARGEST_SIZE), LARGEST_SIZE, LARGEST_SIZE, image::ExtendedColorType::Rgba8).map_err(|error| format!("{}: {error}", path.display()))
}

/// `size`×`size` pixels, ARGB in network byte order, as the freedesktop tray protocol wants.
#[cfg(not(windows))]
pub fn argb(size: u32) -> Vec<u8> {
    rgba(size).chunks_exact(4).flat_map(|pixel| [pixel[3], pixel[0], pixel[1], pixel[2]]).collect()
}

/// How much of each pixel the logo covers, from 0 to 1, row by row.
fn logo_coverage(size: u32) -> Vec<f32> {
    let logo = Tree::from_str(LOGO, &Options::default()).expect("assets/icons/logo.svg is valid SVG");
    let mut pixmap = Pixmap::new(size, size).expect("icon sizes are not zero");
    let scale = size as f32 / DESIGN_SIZE * LOGO_SCALE;
    let margin = size as f32 * (1.0 - LOGO_SCALE) / 2.0;
    resvg::render(&logo, Transform::from_row(scale, 0.0, 0.0, scale, margin, margin), &mut pixmap.as_mut());
    pixmap.pixels().iter().map(|pixel| pixel.alpha() as f32 / 255.0).collect()
}

/// Lime blended towards black by `ink`, with the tile's `background` coverage as alpha.
fn pixel(background: f32, ink: f32) -> [u8; 4] {
    let [red, green, blue] = std::array::from_fn(|channel| (LIME[channel] as f32 * (1.0 - ink) + BLACK[channel] as f32 * ink).round() as u8);
    [red, green, blue, (background * 255.0).round() as u8]
}

/// How much of a pixel a shape covers, from the signed distance to its edge.
fn coverage(distance: f32, pixel_size: f32) -> f32 {
    (0.5 - distance / pixel_size).clamp(0.0, 1.0)
}

struct RoundedRect {
    centre: (f32, f32),
    half_size: (f32, f32),
    radius: f32,
}

impl RoundedRect {
    /// Negative inside, positive outside.
    fn distance(&self, x: f32, y: f32) -> f32 {
        let qx = (x - self.centre.0).abs() - (self.half_size.0 - self.radius);
        let qy = (y - self.centre.1).abs() - (self.half_size.1 - self.radius);
        let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
        outside + qx.max(qy).min(0.0) - self.radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel_at(image: &[u8], size: u32, x: u32, y: u32) -> &[u8] {
        let start = ((y * size + x) * 4) as usize;
        &image[start..start + 4]
    }

    #[test]
    fn corners_are_transparent_and_edges_lime() {
        let image = rgba(32);
        assert_eq!(image.len(), 32 * 32 * 4);
        assert_eq!(pixel_at(&image, 32, 0, 0)[3], 0);
        assert_eq!(pixel_at(&image, 32, 16, 1), [LIME[0], LIME[1], LIME[2], 255]);
    }

    #[test]
    fn the_logo_in_the_middle_is_black() {
        let image = rgba(32);
        assert_eq!(pixel_at(&image, 32, 8, 12), [BLACK[0], BLACK[1], BLACK[2], 255]);
        assert_eq!(pixel_at(&image, 32, 16, 3), [LIME[0], LIME[1], LIME[2], 255]);
    }

    #[test]
    fn scales_to_other_sizes() {
        assert_eq!(rgba(64).len(), 64 * 64 * 4);
    }
}
