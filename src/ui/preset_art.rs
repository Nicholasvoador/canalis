//! Pictures of the built-in themes for the carousel, cached as presets/preset-N.png. They come from
//! the screen's own theme videos, or from an installed KANALI, or failing both from the live view
//! while that theme is on the screen. The card in the centre plays its theme's video over the top.

use super::images::{decode_shrunk, shrink, texture};
use super::theme_player::ThemePlayer;
use crate::kanali_assets::KanaliArchive;
use crate::paths;
use crate::settings::{PRESETS, preset_name};
use crate::theme_video::ThemeVideos;
use eframe::egui::{Context, TextureHandle};
use image::RgbaImage;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

#[derive(Default)]
pub struct PresetArt {
    textures: Vec<Option<TextureHandle>>,
    pending: Option<Receiver<(usize, TextureHandle)>>,
    started: bool,
    player: ThemePlayer,
}

impl PresetArt {
    pub fn texture(&mut self, ctx: &Context, index: usize) -> Option<&TextureHandle> {
        self.start_loading(ctx);
        while let Some((loaded, handle)) = self.pending.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.textures[loaded] = Some(handle);
        }
        self.textures.get(index)?.as_ref()
    }

    /// The frame of `index`'s theme video to draw now; the still stands in until one arrives.
    pub fn playing_frame(&mut self, ctx: &Context, index: usize) -> Option<&TextureHandle> {
        self.player.frame(ctx, index, preset_name(PRESETS.get(index)?))
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
        self.textures = vec![None; PRESETS.len()];
        let (sender, receiver) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut sources = LateSources::default();
            for index in 0..PRESETS.len() {
                let Some(image) = load_cached(index).or_else(|| import(&mut sources, index)) else { continue };
                if sender.send((index, texture(&ctx, &format!("preset-{index}"), &image))).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        });
        self.pending = Some(receiver);
    }
}

/// The screen and an installed KANALI are only reached for when a picture is actually missing.
#[derive(Default)]
struct LateSources {
    videos: Option<Option<ThemeVideos>>,
    archive: Option<Option<KanaliArchive>>,
}

impl LateSources {
    fn videos(&mut self) -> Option<&ThemeVideos> {
        self.videos.get_or_insert_with(ThemeVideos::open).as_ref()
    }

    fn archive(&mut self) -> Option<&KanaliArchive> {
        self.archive.get_or_insert_with(KanaliArchive::installed).as_ref()
    }
}

fn cached_file(index: usize) -> PathBuf {
    paths::preset_art_directory().join(format!("preset-{}.png", index + 1))
}

fn load_cached(index: usize) -> Option<RgbaImage> {
    image::open(cached_file(index)).ok().map(|image| image.to_rgba8())
}

/// A missing picture is taken from the screen's own theme video, or from an installed KANALI.
fn import(sources: &mut LateSources, index: usize) -> Option<RgbaImage> {
    let name = preset_name(PRESETS[index]);
    let poster = match sources.videos().and_then(|videos| videos.poster(name)) {
        Some(poster) => shrink(poster),
        None => decode_shrunk(&sources.archive()?.preset_poster(name)?).ok()?,
    };
    save(index, &poster);
    Some(poster)
}

/// The cache only saves work later, so failing to write it is reported and otherwise ignored.
fn save(index: usize, image: &RgbaImage) {
    let file = cached_file(index);
    let saved = std::fs::create_dir_all(paths::preset_art_directory()).map_err(|error| error.to_string()).and_then(|()| image.save(&file).map_err(|error| error.to_string()));
    if let Err(error) = saved {
        eprintln!("canalis: cannot save {}: {error}", file.display());
    }
}
