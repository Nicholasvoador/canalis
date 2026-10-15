//! The media shown on the screen, decoded for the window's preview. GIFs animate; videos have no preview.

use super::images::{ImageBin, animation};
use crate::media::{AdbScreen, is_video};
use crate::ui::images::shrink;
use gpui_kit::RenderImage;
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, RgbaImage};
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

const MAX_FRAMES: usize = 150;
const MIN_FRAME_DELAY: Duration = Duration::from_millis(20);

pub enum MediaFrame {
    /// The picture and which of its frames to draw now.
    Image(Arc<RenderImage>, usize),
    Loading,
    Unavailable(String),
}

pub struct MediaPreview {
    name: String,
    state: State,
    pending: Option<Receiver<State>>,
    bin: ImageBin,
}

enum State {
    Loading,
    Ready(Animation),
    Unavailable(String),
}

struct Animation {
    image: Arc<RenderImage>,
    delays: Vec<Duration>,
    started: Instant,
}

impl MediaPreview {
    pub fn new(bin: ImageBin) -> Self {
        Self { name: String::new(), state: State::Loading, pending: None, bin }
    }

    /// The frame of `name` to draw now; loading starts on first use and runs in the background.
    pub fn frame(&mut self, name: &str) -> MediaFrame {
        if self.name != name {
            self.start_loading(name);
        }
        if let Some(state) = self.pending.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.replace_state(state);
            self.pending = None;
        }
        match &self.state {
            State::Loading => MediaFrame::Loading,
            State::Unavailable(reason) => MediaFrame::Unavailable(reason.clone()),
            State::Ready(animation) => MediaFrame::Image(animation.image.clone(), animation.current_frame()),
        }
    }

    fn start_loading(&mut self, name: &str) {
        let (sender, receiver) = channel();
        let owned_name = name.to_string();
        std::thread::spawn(move || {
            let _ = sender.send(load(&owned_name).map_or_else(State::Unavailable, State::Ready));
        });
        self.name = name.to_string();
        self.replace_state(State::Loading);
        self.pending = Some(receiver);
    }

    fn replace_state(&mut self, state: State) {
        if let State::Ready(old) = std::mem::replace(&mut self.state, state) {
            self.bin.throw_away(old.image);
        }
    }
}

impl Animation {
    fn current_frame(&self) -> usize {
        let total: Duration = self.delays.iter().sum();
        if self.delays.len() < 2 || total.is_zero() {
            return 0;
        }
        let mut remaining = Duration::from_nanos((self.started.elapsed().as_nanos() % total.as_nanos()) as u64);
        for (index, delay) in self.delays.iter().enumerate() {
            if remaining < *delay {
                return index;
            }
            remaining -= *delay;
        }
        0
    }
}

fn load(name: &str) -> Result<Animation, String> {
    if is_video(Path::new(name)) {
        return Err(format!("Video: {name}"));
    }
    let local_file = AdbScreen::new().local_copy(name)?;
    let frames = decode(&local_file).map_err(|error| format!("Cannot preview {name}: {error}"))?;
    let delays = frames.iter().map(|(_, delay)| *delay).collect();
    Ok(Animation { image: animation(frames), delays, started: Instant::now() })
}

fn decode(path: &Path) -> Result<Vec<(RgbaImage, Duration)>, image::ImageError> {
    let is_gif = path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("gif"));
    if !is_gif {
        return Ok(vec![(shrink(image::open(path)?.to_rgba8()), Duration::ZERO)]);
    }
    let file = std::fs::File::open(path).map_err(image::ImageError::IoError)?;
    GifDecoder::new(BufReader::new(file))?.into_frames().take(MAX_FRAMES).map(|frame| frame.map(shrink_frame)).collect()
}

fn shrink_frame(frame: image::Frame) -> (RgbaImage, Duration) {
    let delay = Duration::from(frame.delay()).max(MIN_FRAME_DELAY);
    (shrink(frame.into_buffer()), delay)
}
