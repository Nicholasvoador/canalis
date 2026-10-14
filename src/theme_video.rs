//! The screen's own videos of its built-in themes.
//!
//! The screen keeps all fourteen in `/system/media/video`, named after the themes, so Canalis can
//! show TRYX's artwork without shipping any of it: ffmpeg takes a still out of one for the
//! carousel, and the card in the centre plays the video itself. Without ffmpeg installed the
//! carousel falls back to screenshots of each theme as it is shown.

use crate::hidden_command::hidden_command;
use crate::kanali_assets::loose_name;
use crate::media::AdbScreen;
use crate::paths;
use image::RgbaImage;
use std::path::{Path, PathBuf};

/// Far enough in to clear the fade from black that most of the themes open with.
const POSTER_SECONDS: &str = "2";
const POSTER_FILTER: &str = "scale=640:-2";
/// Mean brightness under which a frame is taken to be part of a fade, and a later one tried.
const DARK_FRAME: u32 = 12;
/// Later moments to try when the first one is still dark.
const RETRY_SECONDS: [&str; 3] = ["4", "6", "8"];

pub struct ThemeVideos {
    screen: AdbScreen,
    file_names: Vec<String>,
}

impl ThemeVideos {
    /// The screen's theme videos, when adb lists them and ffmpeg is there to decode one.
    pub fn open() -> Option<Self> {
        if !ffmpeg_runs() {
            return None;
        }
        let screen = AdbScreen::new();
        let file_names = screen.theme_video_names().ok()?;
        (!file_names.is_empty()).then_some(Self { screen, file_names })
    }

    /// A still of the theme named like "Cyber Bunker", taken from a copy that is not kept.
    pub fn poster(&self, theme_name: &str) -> Option<RgbaImage> {
        let file_name = self.file_name(theme_name)?;
        let video = ScratchVideo::new(file_name);
        self.screen.pull_theme_video(file_name, video.path()).ok()?;
        poster(video.path())
    }

    /// The local copy of a theme's video, fetched from the screen the first time it is played.
    pub fn local_copy(&self, theme_name: &str) -> Option<PathBuf> {
        let file_name = self.file_name(theme_name)?;
        let local = paths::theme_video_directory().join(file_name);
        if local.is_file() {
            return Some(local);
        }
        std::fs::create_dir_all(paths::theme_video_directory()).ok()?;
        // A part-written file would be played as a broken video, so it lands under another name first.
        let partial = local.with_extension("part");
        self.screen.pull_theme_video(file_name, &partial).ok()?;
        std::fs::rename(&partial, &local).ok()?;
        Some(local)
    }

    fn file_name(&self, theme_name: &str) -> Option<&str> {
        let wanted = loose_name(theme_name);
        self.file_names.iter().find(|name| loose_name(trim_extension(name)) == wanted).map(String::as_str)
    }
}

/// A copy of a theme video taken only for its still, and removed again once that is decoded.
struct ScratchVideo(PathBuf);

impl ScratchVideo {
    fn new(file_name: &str) -> Self {
        Self(std::env::temp_dir().join(format!("canalis-still-{}-{file_name}", std::process::id())))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchVideo {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn poster(video: &Path) -> Option<RgbaImage> {
    let mut frame = decode_frame(video, POSTER_SECONDS)?;
    for seconds in RETRY_SECONDS {
        if brightness(&frame) >= DARK_FRAME {
            break;
        }
        match decode_frame(video, seconds) {
            Some(later) => frame = later,
            None => break,
        }
    }
    Some(frame)
}

/// Seeking belongs before the input, where it costs one jump instead of decoding up to that point.
fn decode_frame(video: &Path, seconds: &str) -> Option<RgbaImage> {
    let encode = ["-frames:v", "1", "-vf", POSTER_FILTER, "-f", "image2pipe", "-c:v", "png"];
    let output = hidden_command(ffmpeg_program()).args(["-v", "error", "-nostdin", "-ss", seconds]).arg("-i").arg(video).args(encode).arg("-").output().ok()?;
    output.status.success().then_some(())?;
    image::load_from_memory(&output.stdout).ok().map(|image| image.to_rgba8())
}

fn ffmpeg_runs() -> bool {
    hidden_command(ffmpeg_program()).arg("-version").output().is_ok_and(|output| output.status.success())
}

/// ffmpeg from where winget puts it on Windows (PATH only picks that up after logging in again),
/// or from PATH.
pub fn ffmpeg_program() -> PathBuf {
    if let Some(chosen) = std::env::var_os("CANALIS_FFMPEG") {
        return PathBuf::from(chosen);
    }
    ffmpeg_install_locations().into_iter().find(|path| path.is_file()).unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

#[cfg(windows)]
fn ffmpeg_install_locations() -> Vec<PathBuf> {
    let Some(winget) = std::env::var_os("LOCALAPPDATA").map(|local| PathBuf::from(local).join(r"Microsoft\WinGet")) else {
        return Vec::new();
    };
    let packages = std::fs::read_dir(winget.join("Packages")).into_iter().flatten().flatten();
    let builds = packages
        .filter(|package| package.file_name().to_string_lossy().starts_with("Gyan.FFmpeg"))
        .flat_map(|package| std::fs::read_dir(package.path()).into_iter().flatten().flatten())
        .map(|build| build.path().join(r"bin\ffmpeg.exe"));
    std::iter::once(winget.join(r"Links\ffmpeg.exe")).chain(builds).collect()
}

#[cfg(not(windows))]
fn ffmpeg_install_locations() -> Vec<PathBuf> {
    Vec::new()
}

fn brightness(image: &RgbaImage) -> u32 {
    let pixels = image.pixels().len().max(1) as u64;
    let total: u64 = image.pixels().map(|pixel| (pixel[0] as u64 + pixel[1] as u64 + pixel[2] as u64) / 3).sum();
    (total / pixels) as u32
}

fn trim_extension(file_name: &str) -> &str {
    file_name.rsplit_once('.').map_or(file_name, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_theme_names_to_video_files() {
        assert_eq!(loose_name(trim_extension("Cooling_delivery.mp4")), loose_name("Cooling delivery"));
        assert_eq!(loose_name(trim_extension("The_Battle_of_Ice_and_Fire.mp4")), loose_name("The Battle of Ice and Fire"));
        assert_eq!(loose_name(trim_extension("Exo-Ecologies.mp4")), loose_name("Exo-Ecologies"));
        assert_ne!(loose_name(trim_extension("Go_Duck.mp4")), loose_name("Oasis"));
    }

    #[test]
    fn a_black_frame_reads_as_dark() {
        assert_eq!(brightness(&RgbaImage::new(4, 4)), 0);
        assert!(brightness(&RgbaImage::from_pixel(4, 4, image::Rgba([200, 200, 200, 255]))) > DARK_FRAME);
    }

    /// Needs the screen plugged in, so it sits out of the usual run: cargo test -- --ignored
    #[test]
    #[ignore]
    fn takes_a_still_and_a_video_from_the_screen() {
        let videos = ThemeVideos::open().expect("a connected screen and ffmpeg");
        let poster = videos.poster("Cooling delivery").expect("a still for a built-in theme");
        assert_eq!(poster.width(), 640);
        assert!(brightness(&poster) >= DARK_FRAME, "the still is inside the fade from black");
        assert!(videos.local_copy("Cooling delivery").is_some_and(|video| video.is_file()));
    }
}
