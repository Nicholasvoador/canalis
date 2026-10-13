//! The app icon (window, taskbar, tray and panorama.exe), drawn in code: the sidebar's brand mark
//! (a screen outline with a bar) in black on a lime rounded square. Shapes are anti-aliased from signed distances.

use std::path::Path;

/// The largest size drawn: the window icon and the PNG for Linux launchers.
pub const LARGEST_SIZE: u32 = 256;
const DESIGN_SIZE: f32 = 32.0;
const LIME: [u8; 3] = [0xDE, 0xF7, 0x50];
const BLACK: [u8; 3] = [0x0B, 0x0B, 0x0C];
const BACKGROUND: RoundedRect = RoundedRect { centre: (16.0, 16.0), half_size: (16.0, 16.0), radius: 7.0 };
const SCREEN: RoundedRect = RoundedRect { centre: (16.0, 16.0), half_size: (10.5, 7.0), radius: 3.0 };
const SCREEN_LINE_WIDTH: f32 = 2.6;
const BAR: RoundedRect = RoundedRect { centre: (16.0, 16.0), half_size: (4.5, 1.4), radius: 1.4 };

/// `size`×`size` pixels, RGBA, row by row.
pub fn rgba(size: u32) -> Vec<u8> {
    let scale = DESIGN_SIZE / size as f32;
    (0..size * size)
        .flat_map(|index| {
            let (x, y) = ((index % size) as f32 + 0.5, (index / size) as f32 + 0.5);
            pixel(x * scale, y * scale, scale)
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

/// Colour at a point given in 32-unit design coordinates; `pixel_size` is one pixel in those units.
fn pixel(x: f32, y: f32, pixel_size: f32) -> [u8; 4] {
    let background = coverage(BACKGROUND.distance(x, y), pixel_size);
    let screen_outline = coverage(SCREEN.distance(x, y).abs() - SCREEN_LINE_WIDTH / 2.0, pixel_size);
    let bar = coverage(BAR.distance(x, y), pixel_size);
    let ink = screen_outline.max(bar);
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
    fn the_bar_in_the_middle_is_black() {
        let image = rgba(32);
        assert_eq!(pixel_at(&image, 32, 15, 15), [BLACK[0], BLACK[1], BLACK[2], 255]);
    }

    #[test]
    fn scales_to_other_sizes() {
        assert_eq!(rgba(64).len(), 64 * 64 * 4);
    }
}
