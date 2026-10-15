//! PANORAMA's Customization tab: the preview, your own media (full screen or one file per half of
//! a split screen) and the crop window that new files pass through.

use crate::crop::Shape;
use crate::ui::crop_dialog::{CANVAS_WIDTH, CropView, Decision};
use crate::ui::geometry::from_pixels;
use crate::ui::jobs::Job;
use crate::ui::pointer::follow_drag;
use crate::ui::screen_preview::{MAX_PREVIEW_WIDTH, screen_preview};
use crate::ui::shell::CanalisWindow;
use crate::ui::crop_dialog::CropCanvas;
use crate::ui::theme::{BLACK, BORDER, CARD, faded};
use crate::ui::widgets::{bold, busy_note, card, card_inner_width, muted, pill, primary_button, secondary_button, segmented, style_row};
use crate::media::SUPPORTED_EXTENSIONS;
use crate::settings::{CUSTOMIZATION, CustomTheme, PLAY_MODES};
use gpui_kit::component::Sizable;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::PathBuf;

const HALF_NAMES: [&str; 2] = ["Left half", "Right half"];

impl CanalisWindow {
    pub fn customization_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // GIFs and the screen's filters move, so the page keeps drawing frames.
        window.request_animation_frame();
        let split = self.editor.settings.custom.is_split();
        let half = self.panorama.half;
        let cards = div().flex().flex_col().gap(px(14.0)).child(self.preview_card(window, cx)).child(self.media_library(cx));
        if split {
            cards.child(self.half_card(cx)).child(self.stats_card(HALF_NAMES[half], window, cx))
        } else {
            cards.child(self.stats_card("", window, cx))
        }
    }

    fn preview_card(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let live = self.panorama.preview.live.is_on();
        let live_switch = Switch::new("live-view").checked(live).on_click(cx.listener(|this, on: &bool, _, cx| {
            this.panorama.preview.live.set_on(*on);
            cx.notify();
        }));
        let live_label = div().id("live-view-label").child("Live view").tooltip(|window, cx| Tooltip::new("Screenshots of the cooler's screen over USB, every couple of seconds").build(window, cx));
        let title = div().flex().items_center().justify_between().child(bold("Preview", 19.0)).child(div().flex().items_center().gap_3().child(live_label).child(live_switch));
        let width = card_inner_width(window).min(MAX_PREVIEW_WIDTH);
        let preview = screen_preview(width, &self.editor.settings, &self.status.current, &mut self.panorama.preview);
        card().child(title).child(div().flex().justify_center().pt_2().child(preview))
    }

    fn media_library(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let custom = &self.editor.settings.custom;
        let split = custom.is_split();
        let title = div().flex().items_center().justify_between().child(bold("Your media", 19.0)).child(self.add_media_button(cx));
        let layout = segmented("layout", &split, vec![(false, "Full screen".into()), (true, "Split screen".into())], |this, split| this.editor.settings.custom.set_split(split), cx);
        let show_button = !self.editor.settings.is_custom_theme() && !custom.media.is_empty();
        card()
            .child(title)
            .child(muted("Images, GIFs and videos are copied to the screen over USB (adb). The panel is 2240×1080 (2:1)."))
            .child(div().pt_3().child(style_row("Layout", layout)))
            .child(media_list(custom, cx))
            .when(!split && !custom.media.is_empty(), |library| {
                let modes = PLAY_MODES.iter().map(|mode| (mode.to_string(), mode.to_string())).collect();
                library.child(div().pt_2().child(style_row("Play mode", segmented("play-mode", &custom.play_mode, modes, |this, mode| this.editor.settings.custom.play_mode = mode, cx))))
            })
            .when(show_button, |library| {
                library.child(div().pt_2().flex().child(primary_button("show-media", "Show my media on the screen").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.editor.settings.theme = CUSTOMIZATION.into();
                    this.settings_changed(cx);
                }))))
            })
    }

    /// Picked files go through the crop window first, in the shape they will be shown in.
    fn add_media_button(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(label) = self.busy_label() {
            return busy_note(label).into_any_element();
        }
        primary_button("add-media", "Add media…").on_click(cx.listener(|_, _: &ClickEvent, _, cx| pick_media(cx))).into_any_element()
    }

    fn crop_picked(&mut self, files: Vec<PathBuf>, cx: &mut Context<Self>) {
        let shape = if self.editor.settings.custom.is_split() { Shape::Square } else { Shape::Wide };
        self.panorama.cropper.open(files, shape);
        cx.notify();
    }

    /// Split screen: pick a half, then the file it shows; the stats card below edits the same half.
    fn half_card(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let custom = &self.editor.settings.custom;
        let half = self.panorama.half;
        let halves = HALF_NAMES.iter().enumerate().map(|(index, name)| (index, name.to_string())).collect();
        let title = div().flex().items_center().justify_between().child(bold("Halves", 19.0)).child(segmented("half", &half, halves, |this, half| this.panorama.half = half, cx));
        let shown = custom.halves[half].media.clone();
        let none = pill("half-media", 0, "None", shown.is_none()).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.editor.settings.custom.halves[half].media = None;
            this.settings_changed(cx);
        }));
        let files = custom.media.clone().into_iter().enumerate().map(|(index, name)| {
            let selected = shown.as_deref() == Some(name.as_str());
            pill("half-media", index + 1, name.clone(), selected).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.editor.settings.custom.halves[half].media = Some(name.clone());
                this.settings_changed(cx);
            }))
        });
        card()
            .child(title)
            .child(muted("Each half shows one file. Square (1:1) media fills a half best: 1120×1080."))
            .child(div().pt_3().child(style_row("Media", div().flex().flex_wrap().gap(px(8.0)).child(none).children(files))))
    }

    /// The crop window over the whole window, while picked files wait to be decided.
    pub fn crop_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.panorama.cropper.is_open() {
            return None;
        }
        if self.panorama.cropper.poll() {
            window.request_animation_frame();
        }
        self.start_cropped_uploads(cx);
        let view = self.panorama.cropper.view()?;
        let body = match view {
            CropView::Loading => muted("Loading the picture…").into_any_element(),
            CropView::Ready(canvas) => self.crop_controls(canvas, window, cx).into_any_element(),
        };
        let dialog = div()
            .id("crop-dialog")
            .flex()
            .flex_col()
            .gap_2()
            .w(px(CANVAS_WIDTH + 48.0))
            .p_6()
            .bg(rgb(CARD))
            .border_1()
            .border_color(rgb(BORDER))
            .rounded(px(16.0))
            .child(bold("Crop for the screen", 19.0))
            .child(muted(self.panorama.cropper.caption()).pb_3())
            .child(body);
        Some(div().id("crop-backdrop").absolute().inset_0().flex().items_center().justify_center().bg(faded(BLACK, 0.6)).occlude().child(dialog))
    }

    fn crop_controls(&mut self, canvas: CropCanvas, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shapes = vec![(Shape::Wide, "Whole screen · 2240×1080".to_string()), (Shape::Square, "Half of a split screen · 1120×1080".to_string())];
        let shape = self.panorama.cropper.shape;
        let canvas = div()
            .id("crop-canvas")
            .relative()
            .cursor_grab()
            .child(canvas.element())
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, cx| {
                this.panorama.cropper.drag_from = Some(from_pixels(event.position));
                cx.notify();
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                this.panorama.cropper.zoom(f32::from(event.delta.pixel_delta(window.line_height()).y));
                cx.notify();
            }))
            .when(self.panorama.cropper.drag_from.is_some(), |canvas| canvas.child(self.crop_drag_follower(cx)));
        let slider = self.sliders.crop_size.show(self.panorama.cropper.scale(), self.dragging, window, cx);
        let size_row = div().flex().items_center().gap_3().pt_2().child(muted("Size")).child(div().w(px(220.0)).child(slider)).child(muted("Drag the frame to move it; scroll to resize."));
        let decide = |decision: fn() -> Decision| cx.listener(move |this: &mut Self, _: &ClickEvent, _, cx| this.decide_crop(decision(), cx));
        let buttons = div()
            .flex()
            .gap_3()
            .pt_3()
            .child(primary_button("crop-and-copy", "Crop and copy").on_click(decide(|| Decision::Crop)))
            .child(secondary_button("copy-uncropped", "Copy without cropping").on_click(decide(|| Decision::Keep)))
            .child(secondary_button("skip-file", "Skip this file").on_click(decide(|| Decision::Skip)));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(segmented("crop-shape", &shape, shapes, |this, shape| this.panorama.cropper.shape = shape, cx))
            .child(div().flex().justify_center().pt_2().child(canvas))
            .child(size_row)
            .child(buttons)
    }

    /// While the frame is dragged, follows the pointer anywhere in the window until it is let go.
    fn crop_drag_follower(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (moved, released) = (cx.entity().downgrade(), cx.entity().downgrade());
        canvas(
            |_, _, _| {},
            move |_, (), window, _| {
                follow_drag(
                    window,
                    move |spot, _, cx| {
                        let _ = moved.update(cx, |this, cx| {
                            this.panorama.cropper.drag_to(spot);
                            cx.notify();
                        });
                    },
                    move |_, cx| {
                        let _ = released.update(cx, |this, cx| {
                            this.panorama.cropper.drag_from = None;
                            cx.notify();
                        });
                    },
                )
            },
        )
        .absolute()
        .size(px(1.0))
    }

    fn decide_crop(&mut self, decision: Decision, cx: &mut Context<Self>) {
        self.panorama.cropper.decide(decision);
        self.start_cropped_uploads(cx);
        cx.notify();
    }

    /// Once every picked file is decided, copies the chosen ones to the screen.
    fn start_cropped_uploads(&mut self, cx: &mut Context<Self>) {
        if let Some(uploads) = self.panorama.cropper.finished() {
            self.start_job(Job::add_media(uploads), cx);
        }
    }
}

/// The file picker runs its own message loop until it closes, so it opens on a background thread
/// where it can't run the window's work in the middle of handling the click.
fn pick_media(cx: &mut Context<CanalisWindow>) {
    cx.spawn(async move |this, cx| {
        let picked = cx.background_executor().spawn(async { rfd::FileDialog::new().add_filter("Images and videos", &SUPPORTED_EXTENSIONS).pick_files() }).await;
        if let Some(files) = picked.filter(|files| !files.is_empty()) {
            let _ = this.update(cx, |this, cx| this.crop_picked(files, cx));
        }
    })
    .detach();
}

fn media_list(custom: &CustomTheme, cx: &mut Context<CanalisWindow>) -> AnyElement {
    if custom.media.is_empty() {
        return muted("No media yet. Add a file to show it on the screen.").into_any_element();
    }
    let rows = custom.media.iter().enumerate().map(|(index, name)| {
        let where_shown = custom.is_split().then(|| shown_on(custom, name)).flatten();
        let remove = name.clone();
        div()
            .flex()
            .items_center()
            .gap_3()
            .child(muted(format!("{}.", index + 1)))
            .child(name.clone())
            .children(where_shown.map(|place| muted(format!("· {place}"))))
            .child(secondary_button(("remove-media", index), "Remove").xsmall().on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.editor.settings.custom.remove_media(&remove);
                this.settings_changed(cx);
            })))
    });
    div().flex().flex_col().gap_2().children(rows).into_any_element()
}

/// "left half", "right half" or "both halves": where a library file is shown in split screen.
fn shown_on(custom: &CustomTheme, name: &str) -> Option<String> {
    let halves: Vec<usize> = (0..HALF_NAMES.len()).filter(|&index| custom.halves[index].media.as_deref() == Some(name)).collect();
    match halves[..] {
        [] => None,
        [index] => Some(HALF_NAMES[index].to_lowercase()),
        _ => Some("both halves".into()),
    }
}
