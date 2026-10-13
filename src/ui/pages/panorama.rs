//! PANORAMA, laid out like KANALI's page: built-in themes in a carousel, your own media, and the
//! live stats drawn over either, plus screen and fan settings.

use super::PageContext;
use crate::media::SUPPORTED_EXTENSIONS;
use crate::settings::{
    ALIGNMENTS, CELSIUS, CPU_BADGE, CUSTOMIZATION, DARK_TEXT, FAHRENHEIT, FILTERS, FIXED_MODE, FanCurvePreset, Filter, GPU_BADGE, LIGHT_TEXT,
    MAX_OVERLAY_FIELDS, OVERLAY_FIELDS, OverlayStyle, PLAY_MODES, PRESETS, SMART_MODE, Settings, badge_is_available,
};
use crate::ui::carousel::Carousel;
use crate::ui::fan_curve::fan_curve_editor;
use crate::ui::format::field_title;
use crate::ui::screen_preview::{parse_hex_color, screen_preview};
use crate::ui::theme::{BORDER, LIME, MUTED};
use crate::ui::widgets::{bold, card, muted, page_header, pill, pill_button, primary_button, segmented, setting_row, tabs, toggle};
use eframe::egui::{self, Color32, Sense, Stroke, StrokeKind, Ui, vec2};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum PanoramaTab {
    #[default]
    Presets,
    Customization,
    Screen,
    Fan,
}

pub struct PanoramaPage {
    pub tab: PanoramaTab,
    carousel: Carousel,
}

impl PanoramaPage {
    pub fn new(tab: PanoramaTab, settings: &Settings) -> Self {
        Self { tab, carousel: Carousel::new(settings.preset_index().unwrap_or(0)) }
    }
}

pub fn show(ui: &mut Ui, state: &mut PanoramaPage, page: &mut PageContext) {
    page_header(ui, "PANORAMA");
    let options = [(PanoramaTab::Presets, "Pre-set"), (PanoramaTab::Customization, "Customization"), (PanoramaTab::Screen, "Screen"), (PanoramaTab::Fan, "Fan")];
    tabs(ui, &mut state.tab, &options);
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| match state.tab {
        PanoramaTab::Presets => preset_tab(ui, &mut state.carousel, page),
        PanoramaTab::Customization => customization_tab(ui, page),
        PanoramaTab::Screen => screen_settings(ui, &mut page.editor.settings),
        PanoramaTab::Fan => fan_settings(ui, page),
    });
}

fn preset_tab(ui: &mut Ui, carousel: &mut Carousel, page: &mut PageContext) {
    card(ui, |ui| {
        ui.label(bold("Pre-set", 19.0));
        ui.label(muted("Click a side card to bring it forward, then click the centre card to show it on the screen."));
        ui.add_space(10.0);
        let selected = page.editor.settings.preset_index();
        if let Some(index) = carousel.show(ui, &mut page.preview.presets, selected) {
            page.editor.settings.theme = PRESETS[index].into();
        }
    });
    ui.add_space(14.0);
    let settings = &mut page.editor.settings;
    stats_card(ui, &mut settings.overlay, &mut settings.overlay_style);
}

fn customization_tab(ui: &mut Ui, page: &mut PageContext) {
    preview_card(ui, page);
    ui.add_space(14.0);
    media_library(ui, page);
    ui.add_space(14.0);
    let custom = &mut page.editor.settings.custom;
    stats_card(ui, &mut custom.overlay, &mut custom.style);
}

fn preview_card(ui: &mut Ui, page: &mut PageContext) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(bold("Preview", 19.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut live = page.preview.live.is_on();
                if toggle(ui, &mut live) {
                    page.preview.live.set_on(live, ui.ctx());
                }
                ui.label("Live view").on_hover_text("Screenshots of the cooler's screen over USB, every couple of seconds");
            });
        });
        ui.add_space(10.0);
        ui.vertical_centered(|ui| screen_preview(ui, &page.editor.settings, page.status, page.preview));
    });
}

fn media_library(ui: &mut Ui, page: &mut PageContext) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(bold("Your media", 19.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| add_media_button(ui, page));
        });
        ui.label(muted("Images, GIFs and videos are copied to the screen over USB (adb). Use a 2:1 ratio; the panel is 2240×1080."));
        ui.add_space(10.0);
        let settings = &mut page.editor.settings;
        media_list(ui, settings);
        if !settings.is_custom_theme() && !settings.custom.media.is_empty() {
            ui.add_space(10.0);
            if primary_button(ui, "Show my media on the screen").clicked() {
                settings.theme = CUSTOMIZATION.into();
            }
        }
    });
}

fn add_media_button(ui: &mut Ui, page: &mut PageContext) {
    if let Some(label) = page.jobs.busy_label() {
        ui.spinner();
        ui.label(muted(label));
        return;
    }
    if primary_button(ui, "Add media…").clicked() {
        let picked = rfd::FileDialog::new().add_filter("Images and videos", &SUPPORTED_EXTENSIONS).pick_files();
        if let Some(files) = picked.filter(|files| !files.is_empty()) {
            page.jobs.add_media(files, ui.ctx());
        }
    }
}

fn media_list(ui: &mut Ui, settings: &mut Settings) {
    if settings.custom.media.is_empty() {
        ui.label(muted("No media yet. Add a file to show it on the screen."));
        return;
    }
    let mut remove = None;
    for (index, name) in settings.custom.media.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}.", index + 1)).color(MUTED));
            ui.label(name);
            if ui.small_button("Remove").clicked() {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        settings.custom.media.remove(index);
    }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("Play mode");
        let options: Vec<(String, String)> = PLAY_MODES.iter().map(|mode| (mode.to_string(), mode.to_string())).collect();
        segmented(ui, &mut settings.custom.play_mode, &options);
    });
}

/// KANALI's "System Information Display": up to three stats and how they are drawn.
fn stats_card(ui: &mut Ui, overlay: &mut Vec<String>, style: &mut OverlayStyle) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(bold("System Information Display", 19.0));
            ui.label(muted(format!("Select up to {MAX_OVERLAY_FIELDS} items")));
        });
        ui.add_space(10.0);
        style_controls(ui, style, overlay);
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for field in OVERLAY_FIELDS {
                overlay_chip(ui, overlay, field);
            }
        });
    });
}

fn style_controls(ui: &mut Ui, style: &mut OverlayStyle, overlay: &[String]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(10.0, 8.0);
        for color in [DARK_TEXT, LIGHT_TEXT] {
            if colour_swatch(ui, color, style.color == color).clicked() {
                style.color = color.into();
            }
        }
        ui.add_space(14.0);
        let alignments: Vec<(String, String)> = ALIGNMENTS.iter().map(|align| (align.to_string(), align.to_string())).collect();
        segmented(ui, &mut style.align, &alignments);
        ui.add_space(14.0);
        filter_controls(ui, &mut style.filter);
        ui.add_space(14.0);
        for badge in [CPU_BADGE, GPU_BADGE] {
            badge_chip(ui, &mut style.badges, badge, badge_is_available(badge, overlay));
        }
    });
}

fn colour_swatch(ui: &mut Ui, color: &str, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
    ui.painter().rect_filled(rect.shrink(5.0), 4, parse_hex_color(color).unwrap_or(Color32::WHITE));
    let border = if selected { Stroke::new(2.0, LIME) } else { Stroke::new(1.0, BORDER) };
    ui.painter().rect_stroke(rect, 8, border, StrokeKind::Inside);
    response.on_hover_text(if color == DARK_TEXT { "Black text" } else { "Light grey text" }).on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn filter_controls(ui: &mut Ui, filter: &mut Filter) {
    let mut options = vec![(None, "No filter".to_string())];
    options.extend(FILTERS.iter().map(|name| (Some(name.to_string()), name.to_string())));
    segmented(ui, &mut filter.value, &options);
    ui.spacing_mut().slider_width = 100.0;
    let slider = egui::Slider::new(&mut filter.opacity, 0..=100).step_by(25.0).suffix("%");
    ui.add_enabled(filter.value.is_some(), slider).on_hover_text("Filter opacity");
}

fn badge_chip(ui: &mut Ui, badges: &mut Vec<String>, badge: &str, available: bool) {
    let selected = available && badges.iter().any(|chosen| chosen == badge);
    let hint = format!("Pick a {} stat to show this badge", badge.replace(" Badge", ""));
    let response = ui.add_enabled(available, pill_button(badge, selected)).on_disabled_hover_text(hint);
    if response.clicked() {
        if selected {
            badges.retain(|chosen| chosen != badge);
        } else {
            badges.push(badge.to_string());
        }
    }
}

fn overlay_chip(ui: &mut Ui, overlay: &mut Vec<String>, field: &str) {
    let selected = overlay.iter().any(|chosen| chosen == field);
    let full = overlay.len() >= MAX_OVERLAY_FIELDS;
    let response = ui.add_enabled(selected || !full, pill_button(field_title(field), selected));
    if response.clicked() {
        if selected {
            overlay.retain(|chosen| chosen != field);
        } else {
            overlay.push(field.to_string());
        }
    }
}

fn screen_settings(ui: &mut Ui, settings: &mut Settings) {
    card(ui, |ui| {
        setting_row(ui, "Screen display", "Turn the cooler's screen on or off.", |ui| {
            toggle(ui, &mut settings.enable);
        });
        setting_row(ui, "Brightness", "", |ui| {
            ui.add(egui::Slider::new(&mut settings.brightness, 0..=100).suffix("%"));
        });
        setting_row(ui, "Mirror mode", "A mirror image of the screen for left-mounted cases. The cooler restarts when this changes.", |ui| {
            let mut mirrored = settings.is_mirrored();
            if toggle(ui, &mut mirrored) {
                settings.set_mirrored(mirrored);
            }
        });
        setting_row(
            ui,
            "Black screen while the PC sleeps",
            "On: the screen goes dark when the PC is off or the app is closed. Off: it shows a sleep animation.",
            |ui| {
                toggle(ui, &mut settings.display_in_sleep);
            },
        );
        setting_row(ui, "Temperature unit", "", |ui| {
            let options = [(FAHRENHEIT.to_string(), "°F".to_string()), (CELSIUS.to_string(), "°C".to_string())];
            segmented(ui, &mut settings.temperature_unit, &options);
        });
    });
}

fn fan_settings(ui: &mut Ui, page: &mut PageContext) {
    let current_temperature = page.connected().then(|| page.status.number("/snapshot/cpu/temperature") as f32);
    let fan_rpm = page.status.number("/device_status/status/fanLCD");
    let connected = page.connected();
    let fan = &mut page.editor.settings.fan;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(bold("Pump-head fan", 19.0));
            if connected {
                ui.label(muted(format!("· {fan_rpm:.0} RPM now")));
            }
        });
        ui.label(muted("The pump speed itself is fixed by the cooler; only the small fan on the pump head can be controlled."));
        ui.add_space(12.0);
        segmented(ui, &mut fan.mode, &[(SMART_MODE.into(), "Smart (follows CPU temperature)".into()), (FIXED_MODE.into(), "Fixed speed".into())]);
        ui.add_space(14.0);
        if fan.mode == FIXED_MODE {
            ui.add(egui::Slider::new(&mut fan.fixed_percent, 0..=100).suffix("%").text("Fan speed"));
            return;
        }
        ui.horizontal(|ui| {
            ui.label("Start from");
            for preset in FanCurvePreset::ALL {
                if pill(ui, preset.label(), fan.smart_curve == preset.curve()).clicked() {
                    fan.smart_curve = preset.curve();
                }
            }
        });
        ui.add_space(10.0);
        page.interacting |= fan_curve_editor(ui, &mut fan.smart_curve, current_temperature);
        ui.label(muted("Drag the points. Temperatures are CPU °C; values are fan %.").size(12.5));
    });
}
