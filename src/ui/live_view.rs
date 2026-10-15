//! Live view: screenshots of the cooler's screen over adb, about every two seconds while the
//! preview is on screen. Each screenshot takes about a second over USB.

use super::images::{ImageBin, still};
use crate::media::AdbScreen;
use crate::ui::images::decode_shrunk;
use gpui_kit::RenderImage;
use image::RgbaImage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const INTERVAL: Duration = Duration::from_secs(2);
/// Capturing pauses when the preview hasn't been drawn for this long (another page is open).
const UNWATCHED_AFTER: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct Capture {
    pub picture: Arc<RenderImage>,
    pub image: RgbaImage,
    pub taken: Instant,
}

pub enum LiveFrame {
    Waiting,
    Image(Capture),
    Failed(String),
}

pub struct LiveView {
    worker: Option<Worker>,
    latest: Option<Capture>,
    error: Option<String>,
    bin: ImageBin,
}

struct Worker {
    running: Arc<AtomicBool>,
    last_watched: Arc<Mutex<Instant>>,
    captures: Receiver<Result<Capture, String>>,
}

impl LiveView {
    pub fn new(bin: ImageBin) -> Self {
        Self { worker: None, latest: None, error: None, bin }
    }

    pub fn is_on(&self) -> bool {
        self.worker.is_some()
    }

    pub fn set_on(&mut self, on: bool) {
        match (on, self.worker.take()) {
            (true, None) => self.worker = Some(Worker::start()),
            (true, worker) => self.worker = worker,
            (false, Some(worker)) => {
                worker.running.store(false, Ordering::Relaxed);
                self.replace_latest(None);
                self.error = None;
            }
            (false, None) => {}
        }
    }

    /// The newest screenshot; drawing it also tells the worker that someone is watching.
    pub fn frame(&mut self) -> LiveFrame {
        if let Some(worker) = &self.worker {
            *worker.last_watched.lock().expect("live view clock") = Instant::now();
            let arrived: Vec<_> = worker.captures.try_iter().collect();
            for capture in arrived {
                match capture {
                    Ok(capture) => {
                        self.replace_latest(Some(capture));
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        }
        match (&self.latest, &self.error) {
            (_, Some(error)) => LiveFrame::Failed(error.clone()),
            (Some(capture), None) => LiveFrame::Image(capture.clone()),
            (None, None) => LiveFrame::Waiting,
        }
    }

    fn replace_latest(&mut self, capture: Option<Capture>) {
        if let Some(old) = std::mem::replace(&mut self.latest, capture) {
            self.bin.throw_away(old.picture);
        }
    }
}

impl Worker {
    fn start() -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let last_watched = Arc::new(Mutex::new(Instant::now()));
        let (sender, captures) = channel();
        let (still_running, watched) = (running.clone(), last_watched.clone());
        std::thread::spawn(move || {
            let screen = AdbScreen::new();
            while still_running.load(Ordering::Relaxed) {
                if watched.lock().expect("live view clock").elapsed() < UNWATCHED_AFTER && sender.send(capture(&screen)).is_err() {
                    return;
                }
                std::thread::sleep(INTERVAL);
            }
        });
        Self { running, last_watched, captures }
    }
}

fn capture(screen: &AdbScreen) -> Result<Capture, String> {
    let image = decode_shrunk(&screen.screenshot_png()?).map_err(|error| format!("unreadable screenshot: {error}"))?;
    Ok(Capture { picture: still(image.clone()), image, taken: Instant::now() })
}
