//! The theme video playing on the carousel's card in the centre.
//!
//! ffmpeg reads the local copy of the video at its own speed (`-re`) and writes plain RGBA frames,
//! which a thread turns into pictures as they arrive. Only one card plays at a time, so moving
//! along the carousel stops the video behind it; a full channel pauses the decoder while nothing
//! is being drawn, and the card falls back to its still until the first frame comes through.

use super::images::{ImageBin, still};
use crate::hidden_command::hidden_command;
use crate::theme_video::{ThemeVideos, ffmpeg_program};
use gpui_kit::RenderImage;
use image::RgbaImage;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};

/// The centre card is around 600 points wide, and the themes are all twice as wide as they are tall.
const FRAME_WIDTH: u32 = 480;
const FRAME_HEIGHT: u32 = 240;
const FRAME_BYTES: usize = (FRAME_WIDTH * FRAME_HEIGHT * 4) as usize;
const FRAMES_PER_SECOND: &str = "30";
/// Enough to keep drawing smooth, and few enough that an unwatched video stops decoding.
const FRAMES_AHEAD: usize = 2;

pub struct ThemePlayer {
    playing: Option<Playing>,
    bin: ImageBin,
}

struct Playing {
    index: usize,
    frames: Receiver<Arc<RenderImage>>,
    picture: Option<Arc<RenderImage>>,
    decoder: Arc<Decoder>,
}

impl ThemePlayer {
    pub fn new(bin: ImageBin) -> Self {
        Self { playing: None, bin }
    }

    /// The frame to draw for `index`, starting that theme's video and stopping any other.
    pub fn frame(&mut self, index: usize, theme_name: &str) -> Option<Arc<RenderImage>> {
        if self.playing.as_ref().is_none_or(|playing| playing.index != index) {
            self.start(index, theme_name);
        }
        let playing = self.playing.as_mut()?;
        while let Ok(picture) = playing.frames.try_recv() {
            if let Some(old) = playing.picture.replace(picture) {
                self.bin.throw_away(old);
            }
        }
        playing.picture.clone()
    }

    /// Dropping the last video closes its channel, which stops the decoder behind it.
    fn start(&mut self, index: usize, theme_name: &str) {
        if let Some(old) = self.playing.take().and_then(|playing| playing.picture.clone()) {
            self.bin.throw_away(old);
        }
        let (sender, frames) = sync_channel(FRAMES_AHEAD);
        let decoder = Arc::new(Decoder::default());
        let (theme_name, started) = (theme_name.to_string(), decoder.clone());
        std::thread::spawn(move || {
            play(&theme_name, &sender, &started);
            started.stop();
        });
        self.playing = Some(Playing { index, frames, picture: None, decoder });
    }
}

impl Drop for Playing {
    fn drop(&mut self) {
        self.decoder.stop();
    }
}

/// The running ffmpeg, shared so that the window closing stops it as surely as the card changing.
#[derive(Default)]
struct Decoder(Mutex<Option<Child>>);

impl Decoder {
    fn hold(&self, child: Child) {
        *self.0.lock().expect("theme video decoder") = Some(child);
    }

    fn stop(&self) {
        if let Some(mut child) = self.0.lock().expect("theme video decoder").take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn play(theme_name: &str, frames: &SyncSender<Arc<RenderImage>>, decoder: &Decoder) {
    let Some(video) = ThemeVideos::open().and_then(|videos| videos.local_copy(theme_name)) else {
        return;
    };
    let Some(mut child) = decode(&video) else { return };
    let Some(mut output) = child.stdout.take() else { return };
    decoder.hold(child);
    let mut frame = vec![0u8; FRAME_BYTES];
    while output.read_exact(&mut frame).is_ok() {
        let Some(image) = RgbaImage::from_raw(FRAME_WIDTH, FRAME_HEIGHT, frame.clone()) else { break };
        if frames.send(still(image)).is_err() {
            break;
        }
    }
}

/// `-stream_loop -1` plays the theme over and over, as the screen does, and `-re` at its own speed.
fn decode(video: &Path) -> Option<Child> {
    let filter = format!("scale={FRAME_WIDTH}:{FRAME_HEIGHT},fps={FRAMES_PER_SECOND}");
    hidden_command(ffmpeg_program())
        .args(["-v", "error", "-nostdin", "-stream_loop", "-1", "-re"])
        .arg("-i")
        .arg(video)
        .args(["-an", "-vf", &filter, "-f", "rawvideo", "-pix_fmt", "rgba", "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}
