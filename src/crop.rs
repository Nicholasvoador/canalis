//! Cropping media to the screen's shape before it is copied over, as KANALI does: 2:1 for the
//! whole screen, 1:1 for one half of a split screen. Images and GIFs are cropped here; videos
//! go through ffmpeg, when it is installed.

use crate::hidden_command::hidden_command;
use crate::media::{AdbScreen, is_video};
use crate::theme_video::{ffmpeg_program, video_still};
use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
use image::codecs::jpeg::JpegEncoder;
use image::{AnimationDecoder, Frame, RgbaImage, imageops};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// Smallest crop offered, as a share of the largest one that fits.
pub const MIN_SCALE: f32 = 0.2;
/// Media this close to the shape already is left as it is; KANALI's own 2:1 and 1:1 crops pass.
const SHAPE_TOLERANCE: f64 = 0.04;
const PANEL_WIDTH: f64 = 2240.0;
const PANEL_HEIGHT: f64 = 1080.0;
const JPEG_QUALITY: u8 = 92;
/// Cropped GIFs are re-encoded at this speed: 1 is the best colours, 30 the fastest.
const GIF_ENCODER_SPEED: i32 = 10;

/// The two shapes the screen shows media in: about 2:1 and 1:1, cropped to the panel's exact pixels.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Shape {
    /// The whole 2240×1080 panel.
    #[default]
    Wide,
    /// One 1120×1080 half of a split screen.
    Square,
}

impl Shape {
    pub fn ratio(self) -> f64 {
        match self {
            Self::Wide => PANEL_WIDTH / PANEL_HEIGHT,
            Self::Square => PANEL_WIDTH / 2.0 / PANEL_HEIGHT,
        }
    }

    pub fn fits(self, (width, height): (u32, u32)) -> bool {
        let ratio = width as f64 / height.max(1) as f64;
        (ratio / self.ratio() - 1.0).abs() < SHAPE_TOLERANCE
    }
}

/// Where a crop sits: its size as a share of the largest crop that fits, and its centre as a share
/// of the picture's width and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    pub scale: f32,
    pub center: [f32; 2],
}

impl Default for Framing {
    fn default() -> Self {
        Self { scale: 1.0, center: [0.5, 0.5] }
    }
}

/// A crop window in the source's pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl CropRect {
    /// The crop of `shape` placed by `framing`, kept inside a picture of `size`.
    pub fn framed(size: (u32, u32), shape: Shape, framing: Framing) -> Self {
        let (width, height) = (size.0 as f64, size.1 as f64);
        let (largest_width, largest_height) = if width / height > shape.ratio() { (height * shape.ratio(), height) } else { (width, width / shape.ratio()) };
        let scale = framing.scale.clamp(MIN_SCALE, 1.0) as f64;
        let (crop_width, crop_height) = ((largest_width * scale).round().max(1.0), (largest_height * scale).round().max(1.0));
        let x = (framing.center[0] as f64 * width - crop_width / 2.0).clamp(0.0, width - crop_width);
        let y = (framing.center[1] as f64 * height - crop_height / 2.0).clamp(0.0, height - crop_height);
        Self { x: x.round() as u32, y: y.round() as u32, width: crop_width as u32, height: crop_height as u32 }
    }

    /// The centre of this crop as a share of the picture, for a framing that stays inside it.
    pub fn center_in(self, (width, height): (u32, u32)) -> [f32; 2] {
        [(self.x as f32 + self.width as f32 / 2.0) / width as f32, (self.y as f32 + self.height as f32 / 2.0) / height as f32]
    }

    /// Video encoders need even sizes and offsets.
    fn even(self) -> Self {
        let even = |value: u32| value & !1;
        Self { x: even(self.x), y: even(self.y), width: even(self.width).max(2), height: even(self.height).max(2) }
    }
}

/// A file chosen to go to the screen, and the crop to apply to it on the way.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaUpload {
    pub file: PathBuf,
    pub crop: Option<CropRect>,
}

impl MediaUpload {
    /// Cropped evenly from both sides to `shape`, unless it already has that shape.
    pub fn centered(file: PathBuf, shape: Shape) -> Result<Self, String> {
        let size = media_size(&file)?;
        let crop = (!shape.fits(size)).then(|| CropRect::framed(size, shape, Framing::default()));
        Ok(Self { file, crop })
    }

    /// Crops the file if asked, copies it to the screen, and returns the name it has there.
    pub fn copy_to_screen(&self, screen: &AdbScreen) -> Result<String, String> {
        match self.crop {
            Some(crop) => screen.add(cropped_copy(&self.file, crop)?.path()),
            None => screen.add(&self.file),
        }
    }
}

/// The first frame of a file, which is what a crop is chosen on.
pub fn first_frame(file: &Path) -> Result<RgbaImage, String> {
    if is_video(file) {
        return video_still(file, "0", None).ok_or_else(|| "videos can only be cropped with ffmpeg installed".to_string());
    }
    image::open(file).map(|image| image.to_rgba8()).map_err(|error| format!("{}: {error}", file.display()))
}

fn media_size(file: &Path) -> Result<(u32, u32), String> {
    if is_video(file) {
        return first_frame(file).map(|frame| frame.dimensions());
    }
    image::image_dimensions(file).map_err(|error| format!("{}: {error}", file.display()))
}

/// A cropped copy in a scratch folder, named like the original so it keeps that name on the
/// screen. It is deleted once dropped.
pub struct CroppedCopy(PathBuf);

impl CroppedCopy {
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for CroppedCopy {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub fn cropped_copy(source: &Path, crop: CropRect) -> Result<CroppedCopy, String> {
    let directory = std::env::temp_dir().join(format!("canalis-crop-{}", std::process::id()));
    std::fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let copy = CroppedCopy(directory.join(source.file_name().ok_or("the file has no name")?));
    crop_file(source, crop, copy.path())?;
    Ok(copy)
}

fn crop_file(source: &Path, crop: CropRect, destination: &Path) -> Result<(), String> {
    let extension = source.extension().map(|extension| extension.to_string_lossy().to_lowercase()).unwrap_or_default();
    match extension.as_str() {
        _ if is_video(source) => crop_video(source, crop, destination),
        "gif" => crop_gif(source, crop, destination).map_err(|error| format!("cannot crop {}: {error}", source.display())),
        _ => crop_image(source, crop, destination).map_err(|error| format!("cannot crop {}: {error}", source.display())),
    }
}

fn crop_image(source: &Path, crop: CropRect, destination: &Path) -> image::ImageResult<()> {
    let image = image::open(source)?;
    let cropped = image.crop_imm(crop.x, crop.y, crop.width, crop.height);
    let is_jpeg = destination.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg"));
    if is_jpeg {
        let writer = BufWriter::new(File::create(destination)?);
        return cropped.to_rgb8().write_with_encoder(JpegEncoder::new_with_quality(writer, JPEG_QUALITY));
    }
    cropped.save(destination)
}

/// Every frame keeps its delay, and the GIF still loops forever.
fn crop_gif(source: &Path, crop: CropRect, destination: &Path) -> image::ImageResult<()> {
    let frames = GifDecoder::new(BufReader::new(File::open(source)?))?.into_frames().collect_frames()?;
    let mut encoder = GifEncoder::new_with_speed(BufWriter::new(File::create(destination)?), GIF_ENCODER_SPEED);
    encoder.set_repeat(Repeat::Infinite)?;
    for frame in frames {
        let cropped = imageops::crop_imm(frame.buffer(), crop.x, crop.y, crop.width, crop.height).to_image();
        encoder.encode_frame(Frame::from_parts(cropped, 0, 0, frame.delay()))?;
    }
    Ok(())
}

fn crop_video(source: &Path, crop: CropRect, destination: &Path) -> Result<(), String> {
    let crop = crop.even();
    let filter = format!("crop={}:{}:{}:{}", crop.width, crop.height, crop.x, crop.y);
    let output = hidden_command(ffmpeg_program())
        .args(["-v", "error", "-nostdin", "-y", "-i"])
        .arg(source)
        .args(["-vf", &filter, "-c:a", "copy"])
        .arg(destination)
        .output()
        .map_err(|error| format!("videos can only be cropped with ffmpeg installed ({error})"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!("ffmpeg could not crop {}: {}", source.display(), String::from_utf8_lossy(&output.stderr).trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn largest_centred_crop_of_each_shape() {
        assert_eq!(CropRect::framed((1920, 1080), Shape::Wide, Framing::default()), CropRect { x: 0, y: 77, width: 1920, height: 926 });
        assert_eq!(CropRect::framed((1920, 1080), Shape::Square, Framing::default()), CropRect { x: 400, y: 0, width: 1120, height: 1080 });
        assert_eq!(CropRect::framed((4480, 2160), Shape::Wide, Framing::default()), CropRect { x: 0, y: 0, width: 4480, height: 2160 });
    }

    #[test]
    fn crop_stays_inside_the_picture() {
        let framing = Framing { scale: 0.5, center: [0.99, 0.0] };
        let crop = CropRect::framed((1000, 1000), Shape::Wide, framing);
        assert_eq!((crop.x, crop.y, crop.width, crop.height), (500, 0, 500, 241));
        assert_eq!(crop.center_in((1000, 1000))[0], 0.75);
    }

    #[test]
    fn scale_has_a_floor() {
        let crop = CropRect::framed((1000, 500), Shape::Wide, Framing { scale: 0.0, center: [0.5, 0.5] });
        assert_eq!(crop.width, (1000.0 * MIN_SCALE) as u32);
    }

    #[test]
    fn media_of_the_right_shape_is_left_alone() {
        assert!(Shape::Wide.fits((2240, 1080)) && Shape::Wide.fits((1920, 960)));
        assert!(Shape::Square.fits((1120, 1080)) && Shape::Square.fits((1080, 1080)));
        assert!(!Shape::Wide.fits((1920, 1080)));
        assert!(!Shape::Square.fits((1920, 1080)));
    }

    #[test]
    fn video_crops_are_even() {
        assert_eq!(CropRect { x: 3, y: 5, width: 1081, height: 1079 }.even(), CropRect { x: 2, y: 4, width: 1080, height: 1078 });
    }

    #[test]
    fn crops_images_and_keeps_gif_frames() {
        let directory = std::env::temp_dir().join(format!("canalis-crop-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let picture = directory.join("picture.png");
        RgbaImage::from_fn(40, 10, |x, _| Rgba([x as u8 * 6, 0, 0, 255])).save(&picture).unwrap();
        let square = CropRect::framed((40, 10), Shape::Square, Framing::default());
        let copy = cropped_copy(&picture, square).unwrap();
        assert_eq!(image::image_dimensions(copy.path()).unwrap(), (10, 10));
        assert_eq!(image::open(copy.path()).unwrap().to_rgba8().get_pixel(0, 0)[0], 15 * 6);

        let gif = directory.join("loop.gif");
        write_two_frame_gif(&gif);
        let copy = cropped_copy(&gif, square).unwrap();
        let frames = GifDecoder::new(BufReader::new(File::open(copy.path()).unwrap())).unwrap().into_frames().collect_frames().unwrap();
        assert_eq!((frames.len(), frames[0].buffer().dimensions()), (2, (10, 10)));
        let copied_path = copy.path().to_path_buf();
        drop(copy);
        assert!(!copied_path.exists(), "the scratch copy is removed");
    }

    fn write_two_frame_gif(path: &Path) {
        let mut encoder = GifEncoder::new(File::create(path).unwrap());
        for shade in [0, 255] {
            let frame = RgbaImage::from_pixel(40, 10, Rgba([shade, shade, shade, 255]));
            encoder.encode_frame(Frame::from_parts(frame, 0, 0, image::Delay::from_numer_denom_ms(100, 1))).unwrap();
        }
    }
}
