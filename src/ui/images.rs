//! Turning decoded pictures into preview-sized egui textures.

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::RgbaImage;
use image::imageops::{self, FilterType};

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

pub fn texture(ctx: &Context, name: &str, image: &RgbaImage) -> TextureHandle {
    let pixels = ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], image.as_raw());
    ctx.load_texture(name, pixels, TextureOptions::LINEAR)
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
}
