//! The crop window shown after picking media, like KANALI's: a 2:1 or 1:1 frame over the picture,
//! dragged into place and sized with the slider or the mouse wheel, one file at a time. Files that
//! already have the shape skip it, and so do videos when ffmpeg can't open them.

use super::geometry::Area;
use super::images::{ImageBin, still};
use super::paint::rect_outline;
use super::theme::LIME;
use crate::crop::{CropRect, Framing, MIN_SCALE, MediaUpload, Shape, first_frame};
use crate::ui::images::shrink;
use gpui_kit::*;
use image::RgbaImage;
use std::cell::Cell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

pub const CANVAS_WIDTH: f32 = 640.0;
const CANVAS_HEIGHT: f32 = 400.0;
/// How much one point of mouse-wheel scrolling grows or shrinks the crop.
const WHEEL_ZOOM: f32 = 0.002;
const OUTSIDE_SHADE: u32 = 0x000000AA;

pub struct CropDialog {
    waiting: VecDeque<PathBuf>,
    current: Option<Candidate>,
    chosen: Vec<MediaUpload>,
    pub shape: Shape,
    /// Where the pointer was when the frame was last moved, while it is being dragged.
    pub drag_from: Option<Point<f32>>,
    canvas: Rc<Cell<Area>>,
    bin: ImageBin,
}

struct Candidate {
    file: PathBuf,
    picture: Picture,
    framing: Framing,
}

enum Picture {
    Loading(Receiver<Result<RgbaImage, String>>),
    Ready(LoadedPicture),
}

struct LoadedPicture {
    image: Arc<RenderImage>,
    size: (u32, u32),
}

pub enum Decision {
    Crop,
    Keep,
    Skip,
}

/// What the window draws for the file being decided.
pub enum CropView {
    Loading,
    Ready(CropCanvas),
}

pub struct CropCanvas {
    pub width: f32,
    pub height: f32,
    image: Arc<RenderImage>,
    /// The crop frame inside the canvas.
    frame: Area,
    canvas: Rc<Cell<Area>>,
}

impl CropDialog {
    pub fn new(bin: ImageBin) -> Self {
        Self { waiting: VecDeque::new(), current: None, chosen: Vec::new(), shape: Shape::default(), drag_from: None, canvas: Rc::default(), bin }
    }

    pub fn is_open(&self) -> bool {
        self.current.is_some() || !self.waiting.is_empty()
    }

    /// Starts on freshly picked files; `shape` is the one the screen will show them in.
    pub fn open(&mut self, files: Vec<PathBuf>, shape: Shape) {
        self.waiting.extend(files);
        self.shape = shape;
        self.load_next();
    }

    /// Picks up a finished load; returns true while a picture is still loading.
    pub fn poll(&mut self) -> bool {
        self.load_next();
        let Some(Candidate { picture: Picture::Loading(receiver), .. }) = &self.current else { return false };
        let Ok(loaded) = receiver.try_recv() else { return true };
        match loaded {
            Ok(frame) if !self.shape.fits(frame.dimensions()) => {
                let size = frame.dimensions();
                if let Some(candidate) = &mut self.current {
                    candidate.picture = Picture::Ready(LoadedPicture { image: still(shrink(frame)), size });
                }
            }
            _ => self.decide(Decision::Keep),
        }
        self.current.as_ref().is_some_and(|candidate| matches!(candidate.picture, Picture::Loading(_)))
    }

    /// Files decided so far, once every file has been decided.
    pub fn finished(&mut self) -> Option<Vec<MediaUpload>> {
        if self.is_open() {
            return None;
        }
        Some(std::mem::take(&mut self.chosen)).filter(|chosen| !chosen.is_empty())
    }

    pub fn caption(&self) -> String {
        let name = self.current.as_ref().and_then(|candidate| candidate.file.file_name()).map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        match self.waiting.len() {
            0 => name,
            remaining => format!("{name} · {remaining} more after this"),
        }
    }

    pub fn scale(&self) -> f32 {
        self.current.as_ref().map_or(1.0, |candidate| candidate.framing.scale)
    }

    pub fn set_scale(&mut self, scale: f32) {
        if let Some(candidate) = &mut self.current {
            candidate.framing.scale = scale.clamp(MIN_SCALE, 1.0);
        }
    }

    pub fn zoom(&mut self, scroll: f32) {
        self.set_scale(self.scale() + scroll * WHEEL_ZOOM);
    }

    /// Moves the frame by however far the pointer moved since the last call.
    pub fn drag_to(&mut self, pointer: Point<f32>) {
        let (Some(from), Some(candidate)) = (self.drag_from, &mut self.current) else { return };
        let canvas = self.canvas.get();
        candidate.framing.center[0] += (pointer.x - from.x) / canvas.width.max(1.0);
        candidate.framing.center[1] += (pointer.y - from.y) / canvas.height.max(1.0);
        self.drag_from = Some(pointer);
    }

    pub fn view(&mut self) -> Option<CropView> {
        let shape = self.shape;
        let canvas = self.canvas.clone();
        let candidate = self.current.as_mut()?;
        let Picture::Ready(picture) = &candidate.picture else { return Some(CropView::Loading) };
        let crop = CropRect::framed(picture.size, shape, candidate.framing);
        candidate.framing.center = crop.center_in(picture.size);
        let (width, height) = (picture.size.0 as f32, picture.size.1 as f32);
        let fit = (CANVAS_WIDTH / width).min(CANVAS_HEIGHT / height);
        let frame = Area::new(crop.x as f32 * fit, crop.y as f32 * fit, crop.width as f32 * fit, crop.height as f32 * fit);
        Some(CropView::Ready(CropCanvas { width: width * fit, height: height * fit, image: picture.image.clone(), frame, canvas }))
    }

    pub fn decide(&mut self, decision: Decision) {
        let shape = self.shape;
        let Some(candidate) = self.current.take() else { return };
        let crop = match (&decision, &candidate.picture) {
            (Decision::Crop, Picture::Ready(picture)) => Some(CropRect::framed(picture.size, shape, candidate.framing)),
            _ => None,
        };
        if let Picture::Ready(picture) = candidate.picture {
            self.bin.throw_away(picture.image);
        }
        if !matches!(decision, Decision::Skip) {
            self.chosen.push(MediaUpload { file: candidate.file, crop });
        }
        self.drag_from = None;
        self.load_next();
    }

    fn load_next(&mut self) {
        if self.current.is_some() {
            return;
        }
        let Some(file) = self.waiting.pop_front() else { return };
        let (sender, receiver) = channel();
        let source = file.clone();
        std::thread::spawn(move || {
            let _ = sender.send(first_frame(&source));
        });
        self.current = Some(Candidate { file, picture: Picture::Loading(receiver), framing: Framing::default() });
    }
}

impl CropCanvas {
    /// The picture with everything outside the crop shaded.
    pub fn element(self) -> impl IntoElement {
        let (width, height) = (self.width, self.height);
        canvas(
            move |bounds, _, _| self.canvas.set(Area::from_bounds(bounds)),
            move |bounds, (), window, _| {
                let canvas = Area::from_bounds(bounds);
                let _ = window.paint_image(bounds, bounds, Corners::default(), self.image.clone(), 0, false);
                shade_outside(window, canvas, self.frame.translate(canvas.left, canvas.top));
            },
        )
        .w(px(width))
        .h(px(height))
    }
}

fn shade_outside(window: &mut Window, canvas: Area, frame: Area) {
    let outside = [
        Area::from_corners(point(canvas.left, canvas.top), point(canvas.right(), frame.top)),
        Area::from_corners(point(canvas.left, frame.bottom()), point(canvas.right(), canvas.bottom())),
        Area::from_corners(point(canvas.left, frame.top), point(frame.left, frame.bottom())),
        Area::from_corners(point(frame.right(), frame.top), point(canvas.right(), frame.bottom())),
    ];
    for area in outside.into_iter().filter(|area| area.width > 0.0 && area.height > 0.0) {
        window.paint_quad(fill(area.bounds(), rgba(OUTSIDE_SHADE)));
    }
    rect_outline(window, frame, 0.0, 2.0, rgb(LIME));
}
