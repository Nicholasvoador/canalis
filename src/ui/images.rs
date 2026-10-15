//! Decoded pictures as preview-sized GPUI images. GPUI keeps an image on the GPU until it is
//! dropped, so pictures that get replaced (live screenshots, video frames) go into a bin that the
//! window empties.

use gpui_kit::{RenderImage, Window};
use image::imageops::{self, FilterType};
use image::{Delay, Frame, RgbaImage};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

/// Wide enough for the largest preview card; full-size frames would waste memory.
pub const PREVIEW_WIDTH: u32 = 640;

pub fn shrink(image: RgbaImage) -> RgbaImage {
    if image.width() <= PREVIEW_WIDTH {
        return image;
    }
    let height = (image.height() as u64 * PREVIEW_WIDTH as u64 / image.width() as u64).max(1) as u32;
    imageops::resize(&image, PREVIEW_WIDTH, height, FilterType::Triangle)
}

pub fn decode_shrunk(bytes: &[u8]) -> Result<RgbaImage, String> {
    image::load_from_memory(bytes).map(|image| shrink(image.to_rgba8())).map_err(|error| error.to_string())
}

/// A still picture.
pub fn still(image: RgbaImage) -> Arc<RenderImage> {
    animation(vec![(image, Duration::ZERO)])
}

/// An animation's frames, each shown for its duration.
pub fn animation(frames: Vec<(RgbaImage, Duration)>) -> Arc<RenderImage> {
    let frames: Vec<Frame> = frames.into_iter().map(|(image, delay)| Frame::from_parts(to_bgra(image), 0, 0, Delay::from_saturating_duration(delay))).collect();
    Arc::new(RenderImage::new(frames))
}

/// GPUI draws pixels in blue, green, red, alpha order.
fn to_bgra(mut image: RgbaImage) -> RgbaImage {
    for pixel in image.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    image
}

/// Pictures that are no longer shown, waiting to be dropped from the GPU.
#[derive(Clone, Default)]
pub struct ImageBin(Rc<RefCell<Vec<Arc<RenderImage>>>>);

impl ImageBin {
    pub fn throw_away(&self, image: Arc<RenderImage>) {
        self.0.borrow_mut().push(image);
    }

    pub fn empty(&self, window: &mut Window) {
        for image in self.0.borrow_mut().drain(..) {
            let _ = window.drop_image(image);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrinks_wide_images_keeping_the_ratio() {
        let shrunk = shrink(RgbaImage::new(1920, 960));
        assert_eq!((shrunk.width(), shrunk.height()), (PREVIEW_WIDTH, PREVIEW_WIDTH / 2));
        assert_eq!(shrink(RgbaImage::new(100, 50)).width(), 100);
    }

    #[test]
    fn red_and_blue_swap_places() {
        let image = RgbaImage::from_pixel(1, 1, image::Rgba([10, 20, 30, 40]));
        assert_eq!(to_bgra(image).get_pixel(0, 0).0, [30, 20, 10, 40]);
    }
}
