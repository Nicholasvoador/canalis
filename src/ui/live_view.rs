//! Live view: screenshots of the cooler's screen over adb, about every two seconds while the
//! preview is on screen. Each screenshot takes about a second over USB.

use super::images::{decode_shrunk, texture};
use crate::media::AdbScreen;
use eframe::egui::{Context, TextureHandle};
use image::RgbaImage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const INTERVAL: Duration = Duration::from_secs(2);
/// Capturing pauses when the preview hasn't been drawn for this long (another page is open).
const UNWATCHED_AFTER: Duration = Duration::from_secs(3);

pub struct Capture {
    pub texture: TextureHandle,
    pub image: RgbaImage,
    pub taken: Instant,
}

pub enum LiveFrame<'a> {
    Waiting,
    Image(&'a Capture),
    Failed(&'a str),
}

#[derive(Default)]
pub struct LiveView {
    worker: Option<Worker>,
    latest: Option<Capture>,
    error: Option<String>,
}

struct Worker {
    running: Arc<AtomicBool>,
    last_watched: Arc<Mutex<Instant>>,
    captures: Receiver<Result<Capture, String>>,
}

impl LiveView {
    pub fn is_on(&self) -> bool {
        self.worker.is_some()
    }

    pub fn set_on(&mut self, on: bool, ctx: &Context) {
        match (on, self.worker.take()) {
            (true, None) => self.worker = Some(Worker::start(ctx)),
            (true, worker) => self.worker = worker,
            (false, Some(worker)) => {
                worker.running.store(false, Ordering::Relaxed);
                (self.latest, self.error) = (None, None);
            }
            (false, None) => {}
        }
    }

    /// The newest screenshot; drawing it also tells the worker that someone is watching.
    pub fn frame(&mut self) -> LiveFrame<'_> {
        if let Some(worker) = &self.worker {
            *worker.last_watched.lock().expect("live view clock") = Instant::now();
            for capture in worker.captures.try_iter() {
                match capture {
                    Ok(capture) => (self.latest, self.error) = (Some(capture), None),
                    Err(error) => self.error = Some(error),
                }
            }
        }
        match (&self.latest, &self.error) {
            (_, Some(error)) => LiveFrame::Failed(error),
            (Some(capture), None) => LiveFrame::Image(capture),
            (None, None) => LiveFrame::Waiting,
        }
    }
}

impl Worker {
    fn start(ctx: &Context) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let last_watched = Arc::new(Mutex::new(Instant::now()));
        let (sender, captures) = channel();
        let (ctx, still_running, watched) = (ctx.clone(), running.clone(), last_watched.clone());
        std::thread::spawn(move || {
            let screen = AdbScreen::new();
            while still_running.load(Ordering::Relaxed) {
                if watched.lock().expect("live view clock").elapsed() < UNWATCHED_AFTER {
                    if sender.send(capture(&ctx, &screen)).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
                std::thread::sleep(INTERVAL);
            }
        });
        Self { running, last_watched, captures }
    }
}

fn capture(ctx: &Context, screen: &AdbScreen) -> Result<Capture, String> {
    let image = decode_shrunk(&screen.screenshot_png()?).map_err(|error| format!("unreadable screenshot: {error}"))?;
    Ok(Capture { texture: texture(ctx, "live-view", &image), image, taken: Instant::now() })
}
