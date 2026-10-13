//! Pictures of the built-in themes for the carousel, cached as presets/preset-N.png. They come from
//! an installed KANALI the first time, or from the live view while that theme is on the screen.

use super::images::{decode_shrunk, texture};
use crate::kanali_assets::KanaliArchive;
use crate::paths;
use crate::settings::{PRESETS, preset_name};
use eframe::egui::{Context, TextureHandle};
use image::RgbaImage;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

pub struct PresetArt {
    textures: Vec<Option<TextureHandle>>,
    pending: Option<Receiver<(usize, TextureHandle)>>,
    started: bool,
}

impl Default for PresetArt {
    fn default() -> Self {
        Self { textures: vec![None; PRESETS.len()], pending: None, started: false }
    }
}

impl PresetArt {
    pub fn texture(&mut self, ctx: &Context, index: usize) -> Option<&TextureHandle> {
        self.start_loading(ctx);
        while let Some((loaded, handle)) = self.pending.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.textures[loaded] = Some(handle);
        }
        self.textures.get(index)?.as_ref()
    }

    /// Keeps a capture of the screen as the picture of a theme that doesn't have one yet.
    pub fn offer_capture(&mut self, ctx: &Context, index: usize, capture: &RgbaImage) {
        if !self.textures.get(index).is_some_and(Option::is_none) {
            return;
        }
        save(index, capture);
        self.textures[index] = Some(texture(ctx, &format!("preset-{index}"), capture));
    }

    fn start_loading(&mut self, ctx: &Context) {
        if self.started {
            return;
        }
        self.started = true;
        let (sender, receiver) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let archive = KanaliArchive::installed();
            for index in 0..PRESETS.len() {
                let Some(image) = load_cached(index).or_else(|| import_from_kanali(archive.as_ref(), index)) else { continue };
                if sender.send((index, texture(&ctx, &format!("preset-{index}"), &image))).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        });
        self.pending = Some(receiver);
    }
}

fn cached_file(index: usize) -> PathBuf {
    paths::preset_art_directory().join(format!("preset-{}.png", index + 1))
}

fn load_cached(index: usize) -> Option<RgbaImage> {
    image::open(cached_file(index)).ok().map(|image| image.to_rgba8())
}

fn import_from_kanali(archive: Option<&KanaliArchive>, index: usize) -> Option<RgbaImage> {
    let image = decode_shrunk(&archive?.preset_poster(preset_name(PRESETS[index]))?).ok()?;
    save(index, &image);
    Some(image)
}

/// The cache only saves work later, so failing to write it is reported and otherwise ignored.
fn save(index: usize, image: &RgbaImage) {
    let file = cached_file(index);
    let saved = std::fs::create_dir_all(paths::preset_art_directory()).map_err(|error| error.to_string()).and_then(|()| image.save(&file).map_err(|error| error.to_string()));
    if let Err(error) = saved {
        eprintln!("canalis: cannot save {}: {error}", file.display());
    }
}
