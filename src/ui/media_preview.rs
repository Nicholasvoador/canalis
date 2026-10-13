//! The media shown on the screen, decoded for the window's preview. GIFs animate; videos have no preview.

use super::images::{shrink, texture};
use crate::media::AdbScreen;
use eframe::egui::{Context, TextureHandle};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, RgbaImage};
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

const MAX_FRAMES: usize = 150;
const MIN_FRAME_DELAY: Duration = Duration::from_millis(20);
const VIDEO_EXTENSIONS: [&str; 2] = ["mp4", "avi"];

pub enum MediaFrame<'a> {
    Image(&'a TextureHandle),
    Loading,
    Unavailable(&'a str),
}

#[derive(Default)]
pub struct MediaPreview {
    name: String,
    state: State,
    pending: Option<Receiver<State>>,
}

#[derive(Default)]
enum State {
    #[default]
    Loading,
    Ready(Animation),
    Unavailable(String),
}

struct Animation {
    frames: Vec<(TextureHandle, Duration)>,
    started: Instant,
}

impl MediaPreview {
    /// The frame of `name` to draw now; loading starts on first use and runs in the background.
    pub fn frame(&mut self, ctx: &Context, name: &str) -> MediaFrame<'_> {
        if self.name != name {
            self.start_loading(ctx, name);
        }
        if let Some(state) = self.pending.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.state = state;
            self.pending = None;
        }
        match &self.state {
            State::Loading => MediaFrame::Loading,
            State::Unavailable(reason) => MediaFrame::Unavailable(reason),
            State::Ready(animation) => MediaFrame::Image(animation.current_frame(ctx)),
        }
    }

    fn start_loading(&mut self, ctx: &Context, name: &str) {
        let (sender, receiver) = channel();
        let (ctx, owned_name) = (ctx.clone(), name.to_string());
        std::thread::spawn(move || {
            let state = load(&ctx, &owned_name).map_or_else(State::Unavailable, State::Ready);
            let _ = sender.send(state);
            ctx.request_repaint();
        });
        self.name = name.to_string();
        self.state = State::Loading;
        self.pending = Some(receiver);
    }
}

impl Animation {
    fn current_frame(&self, ctx: &Context) -> &TextureHandle {
        if self.frames.len() == 1 {
            return &self.frames[0].0;
        }
        let total: Duration = self.frames.iter().map(|(_, delay)| *delay).sum();
        let mut remaining = Duration::from_nanos((self.started.elapsed().as_nanos() % total.as_nanos()) as u64);
        for (texture, delay) in &self.frames {
            if remaining < *delay {
                ctx.request_repaint_after(*delay - remaining);
                return texture;
            }
            remaining -= *delay;
        }
        &self.frames[0].0
    }
}

fn load(ctx: &Context, name: &str) -> Result<Animation, String> {
    if VIDEO_EXTENSIONS.iter().any(|extension| name.to_lowercase().ends_with(extension)) {
        return Err(format!("Video: {name}"));
    }
    let local_file = AdbScreen::new().local_copy(name)?;
    let images = decode(&local_file).map_err(|error| format!("Cannot preview {name}: {error}"))?;
    let frames = images
        .into_iter()
        .enumerate()
        .map(|(index, (image, delay))| (texture(ctx, &format!("media-{name}-{index}"), &image), delay))
        .collect();
    Ok(Animation { frames, started: Instant::now() })
}

fn decode(path: &Path) -> Result<Vec<(RgbaImage, Duration)>, image::ImageError> {
    let is_gif = path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("gif"));
    if !is_gif {
        return Ok(vec![(shrink(image::open(path)?.to_rgba8()), Duration::ZERO)]);
    }
    let file = std::fs::File::open(path).map_err(image::ImageError::IoError)?;
    GifDecoder::new(BufReader::new(file))?
        .into_frames()
        .take(MAX_FRAMES)
        .map(|frame| frame.map(shrink_frame))
        .collect()
}

fn shrink_frame(frame: image::Frame) -> (RgbaImage, Duration) {
    let delay = Duration::from(frame.delay()).max(MIN_FRAME_DELAY);
    (shrink(frame.into_buffer()), delay)
}
