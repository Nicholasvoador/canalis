//! The crop window shown after picking media, like KANALI's: a 2:1 or 1:1 frame over the picture,
//! dragged into place and sized with the slider or the mouse wheel, one file at a time. Files that
//! already have the shape skip it, and so do videos when ffmpeg can't open them.

use super::images::{shrink, texture};
use super::theme::LIME;
use super::widgets::{bold, muted, primary_button, secondary_button, segmented};
use crate::crop::{CropRect, Framing, MIN_SCALE, MediaUpload, Shape, first_frame};
use eframe::egui::{self, Color32, Context, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2, pos2, vec2};
use image::RgbaImage;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

const CANVAS_SIZE: Vec2 = vec2(640.0, 400.0);
/// How much one point of mouse-wheel scrolling grows or shrinks the crop.
const WHEEL_ZOOM: f32 = 0.002;
const OUTSIDE_SHADE: Color32 = Color32::from_black_alpha(170);

#[derive(Default)]
pub struct CropDialog {
    waiting: VecDeque<PathBuf>,
    current: Option<Candidate>,
    chosen: Vec<MediaUpload>,
    shape: Shape,
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
    texture: TextureHandle,
    size: (u32, u32),
}

enum Decision {
    Crop(CropRect),
    Keep,
    Skip,
}

impl CropDialog {
    pub fn is_open(&self) -> bool {
        self.current.is_some() || !self.waiting.is_empty()
    }

    /// Starts on freshly picked files; `shape` is the one the screen will show them in.
    pub fn open(&mut self, files: Vec<PathBuf>, shape: Shape) {
        self.waiting.extend(files);
        self.shape = shape;
    }

    /// Draws the window. Once every file is decided, returns the ones to copy to the screen.
    pub fn show(&mut self, ctx: &Context) -> Option<Vec<MediaUpload>> {
        if !self.is_open() {
            return None;
        }
        self.load_next(ctx);
        self.settle_loaded(ctx);
        if let Some(decision) = self.window(ctx) {
            self.decide(decision);
        }
        if self.is_open() {
            return None;
        }
        Some(std::mem::take(&mut self.chosen)).filter(|chosen| !chosen.is_empty())
    }

    fn load_next(&mut self, ctx: &Context) {
        if self.current.is_some() {
            return;
        }
        let Some(file) = self.waiting.pop_front() else { return };
        let (sender, receiver) = channel();
        let (ctx, source) = (ctx.clone(), file.clone());
        std::thread::spawn(move || {
            let _ = sender.send(first_frame(&source));
            ctx.request_repaint();
        });
        self.current = Some(Candidate { file, picture: Picture::Loading(receiver), framing: Framing::default() });
    }

    /// A file that can't be shown, or already has the shape, goes over as it is.
    fn settle_loaded(&mut self, ctx: &Context) {
        let Some(Candidate { file, picture: Picture::Loading(receiver), .. }) = &self.current else { return };
        let Ok(loaded) = receiver.try_recv() else { return };
        match loaded {
            Ok(frame) if !self.shape.fits(frame.dimensions()) => {
                let size = frame.dimensions();
                let texture = texture(ctx, &format!("crop-{}", file.display()), &shrink(frame));
                if let Some(candidate) = &mut self.current {
                    candidate.picture = Picture::Ready(LoadedPicture { texture, size });
                }
            }
            _ => self.decide(Decision::Keep),
        }
    }

    fn decide(&mut self, decision: Decision) {
        let Some(candidate) = self.current.take() else { return };
        let crop = match decision {
            Decision::Crop(crop) => Some(crop),
            Decision::Keep => None,
            Decision::Skip => return,
        };
        self.chosen.push(MediaUpload { file: candidate.file, crop });
    }

    fn window(&mut self, ctx: &Context) -> Option<Decision> {
        let (shape, remaining) = (&mut self.shape, self.waiting.len());
        let candidate = self.current.as_mut()?;
        let modal = egui::Modal::new(egui::Id::new("crop-media")).show(ctx, |ui| {
            ui.set_width(CANVAS_SIZE.x);
            ui.label(bold("Crop for the screen", 19.0));
            ui.label(muted(caption(candidate, remaining)));
            ui.add_space(12.0);
            match &candidate.picture {
                Picture::Loading(_) => {
                    ui.spinner();
                    None
                }
                Picture::Ready(picture) => crop_controls(ui, picture, shape, &mut candidate.framing),
            }
        });
        if modal.should_close() { Some(Decision::Skip) } else { modal.inner }
    }
}

fn caption(candidate: &Candidate, remaining: usize) -> String {
    let name = candidate.file.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    match remaining {
        0 => name,
        _ => format!("{name} · {remaining} more after this"),
    }
}

fn crop_controls(ui: &mut Ui, picture: &LoadedPicture, shape: &mut Shape, framing: &mut Framing) -> Option<Decision> {
    let shapes = [(Shape::Wide, "Whole screen · 2240×1080".to_string()), (Shape::Square, "Half of a split screen · 1120×1080".to_string())];
    segmented(ui, shape, &shapes);
    ui.add_space(12.0);
    let crop = ui.vertical_centered(|ui| crop_canvas(ui, picture, *shape, framing)).inner;
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label(muted("Size"));
        ui.spacing_mut().slider_width = 220.0;
        ui.add(egui::Slider::new(&mut framing.scale, MIN_SCALE..=1.0).show_value(false));
        ui.label(muted("Drag the frame to move it; scroll to resize."));
    });
    ui.add_space(14.0);
    decision_buttons(ui, crop)
}

fn decision_buttons(ui: &mut Ui, crop: CropRect) -> Option<Decision> {
    ui.horizontal(|ui| {
        if primary_button(ui, "Crop and copy").clicked() {
            return Some(Decision::Crop(crop));
        }
        if secondary_button(ui, "Copy without cropping").clicked() {
            return Some(Decision::Keep);
        }
        secondary_button(ui, "Skip this file").clicked().then_some(Decision::Skip)
    })
    .inner
}

/// The picture fitted into the canvas, with everything outside the crop shaded.
fn crop_canvas(ui: &mut Ui, picture: &LoadedPicture, shape: Shape, framing: &mut Framing) -> CropRect {
    let size = vec2(picture.size.0 as f32, picture.size.1 as f32);
    let fit = (CANVAS_SIZE.x / size.x).min(CANVAS_SIZE.y / size.y);
    let (canvas, response) = ui.allocate_exact_size(size * fit, Sense::drag());
    let drag = response.drag_delta();
    framing.center[0] += drag.x / canvas.width();
    framing.center[1] += drag.y / canvas.height();
    if response.hovered() {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        framing.scale = (framing.scale + scroll * WHEEL_ZOOM).clamp(MIN_SCALE, 1.0);
    }
    let crop = CropRect::framed(picture.size, shape, *framing);
    framing.center = crop.center_in(picture.size);
    egui::Image::from_texture(&picture.texture).paint_at(ui, canvas);
    let frame = Rect::from_min_size(canvas.min + vec2(crop.x as f32, crop.y as f32) * fit, vec2(crop.width as f32, crop.height as f32) * fit);
    shade_outside(ui, canvas, frame);
    response.on_hover_cursor(egui::CursorIcon::Grab);
    crop
}

fn shade_outside(ui: &Ui, canvas: Rect, frame: Rect) {
    let painter = ui.painter_at(canvas);
    let outside = [
        Rect::from_min_max(canvas.min, pos2(canvas.max.x, frame.min.y)),
        Rect::from_min_max(pos2(canvas.min.x, frame.max.y), canvas.max),
        Rect::from_min_max(pos2(canvas.min.x, frame.min.y), pos2(frame.min.x, frame.max.y)),
        Rect::from_min_max(pos2(frame.max.x, frame.min.y), pos2(canvas.max.x, frame.max.y)),
    ];
    for area in outside {
        painter.rect_filled(area, 0, OUTSIDE_SHADE);
    }
    painter.rect_stroke(frame, 0, Stroke::new(2.0, LIME), StrokeKind::Inside);
}
