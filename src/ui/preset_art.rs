//! Pictures of the built-in themes for the carousel, cached as presets/preset-N.png. They come from
//! the screen's own theme videos, or from an installed KANALI, or failing both from the live view
//! while that theme is on the screen. The card in the centre plays its theme's video over the top.

use super::images::{ImageBin, still};
use super::theme_player::ThemePlayer;
use crate::kanali_assets::KanaliArchive;
use crate::paths;
use crate::settings::{PRESETS, preset_name};
use crate::theme_video::ThemeVideos;
use crate::ui::images::{decode_shrunk, shrink};
use gpui_kit::RenderImage;
use image::RgbaImage;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

pub struct PresetArt {
    pictures: Vec<Option<Arc<RenderImage>>>,
    pending: Option<Receiver<(usize, Arc<RenderImage>)>>,
    player: ThemePlayer,
}

impl PresetArt {
    pub fn new(bin: ImageBin) -> Self {
        Self { pictures: vec![None; PRESETS.len()], pending: None, player: ThemePlayer::new(bin) }
    }

    pub fn picture(&mut self, index: usize) -> Option<Arc<RenderImage>> {
        self.start_loading();
        while let Some((loaded, picture)) = self.pending.as_ref().and_then(|receiver| receiver.try_recv().ok()) {
            self.pictures[loaded] = Some(picture);
        }
        self.pictures.get(index)?.clone()
    }

    /// The frame of `index`'s theme video to draw now; the still stands in until one arrives.
    pub fn playing_frame(&mut self, index: usize) -> Option<Arc<RenderImage>> {
        self.player.frame(index, preset_name(PRESETS.get(index)?))
    }

    /// Keeps a capture of the screen as the picture of a theme that doesn't have one yet.
    pub fn offer_capture(&mut self, index: usize, capture: &RgbaImage) {
        if !self.pictures.get(index).is_some_and(Option::is_none) {
            return;
        }
        save(index, capture);
        self.pictures[index] = Some(still(capture.clone()));
    }

    fn start_loading(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let mut sources = LateSources::default();
            for index in 0..PRESETS.len() {
                let Some(image) = load_cached(index).or_else(|| import(&mut sources, index)) else { continue };
                if sender.send((index, still(image))).is_err() {
                    return;
                }
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
