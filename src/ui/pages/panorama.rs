//! PANORAMA: screen, theme and fan settings for the cooler.

use super::PageContext;
use crate::media::SUPPORTED_EXTENSIONS;
use crate::settings::{
    ALIGNMENTS, CELSIUS, CUSTOMIZATION, FAHRENHEIT, FIXED_MODE, FanCurvePreset, MAX_OVERLAY_FIELDS, OVERLAY_FIELDS, PRESETS, ROTATIONS, SMART_MODE,
    Settings,
};
use crate::ui::fan_curve::fan_curve_editor;
use crate::ui::screen_preview::{parse_hex_color, screen_preview};
use crate::ui::theme::{BORDER, CARD_RAISED, LIME, MUTED, TEXT};
use crate::ui::widgets::{bold, card, muted, page_header, pill, primary_button, segmented, setting_row, tabs, toggle};
use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke, StrokeKind, Ui, vec2};

const THEME_TILE: egui::Vec2 = egui::Vec2::new(200.0, 112.0);

#[derive(Clone, Copy, PartialEq, Default)]
pub enum PanoramaTab {
    #[default]
    Screen,
    Theme,
    Fan,
}

pub fn show(ui: &mut Ui, tab: &mut PanoramaTab, page: &mut PageContext) {
    page_header(ui, "PANORAMA");
    tabs(ui, tab, &[(PanoramaTab::Screen, "Screen"), (PanoramaTab::Theme, "Theme"), (PanoramaTab::Fan, "Fan")]);
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| match tab {
        PanoramaTab::Screen => screen_settings(ui, &mut page.editor.settings),
        PanoramaTab::Theme => theme_settings(ui, page),
        PanoramaTab::Fan => fan_settings(ui, page),
    });
}

fn screen_settings(ui: &mut Ui, settings: &mut Settings) {
    card(ui, |ui| {
        setting_row(ui, "Screen display", "Turn the cooler's screen on or off.", |ui| {
            toggle(ui, &mut settings.enable);
        });
        setting_row(ui, "Brightness", "", |ui| {
            ui.add(egui::Slider::new(&mut settings.brightness, 0..=100).suffix("%"));
        });
        setting_row(ui, "Screen rotation", "Match how the pump head is mounted.", |ui| {
            let options: Vec<(u16, String)> = ROTATIONS.iter().rev().map(|degrees| (*degrees, format!("{degrees}°"))).collect();
            segmented(ui, &mut settings.rotate, &options);
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

fn theme_settings(ui: &mut Ui, page: &mut PageContext) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(bold("Preview", 19.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| add_media_button(ui, page));
        });
        ui.add_space(10.0);
        ui.vertical_centered(|ui| screen_preview(ui, &page.editor.settings, page.status, page.media_preview));
    });
    ui.add_space(14.0);
    card(ui, |ui| {
        ui.label(bold("Theme", 19.0));
        ui.add_space(10.0);
        theme_gallery(ui, &mut page.editor.settings);
    });
    ui.add_space(14.0);
    if page.editor.settings.is_custom_theme() {
        media_library(ui, page);
        ui.add_space(14.0);
    }
    overlay_settings(ui, &mut page.editor.settings);
}

fn theme_gallery(ui: &mut Ui, settings: &mut Settings) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
        let themes = PRESETS.iter().copied().chain(std::iter::once(CUSTOMIZATION));
        for (index, theme) in themes.enumerate() {
            if theme_tile(ui, index, theme, settings.theme == theme).clicked() {
                settings.theme = theme.to_string();
            }
        }
    });
}

fn theme_tile(ui: &mut Ui, index: usize, theme: &str, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(THEME_TILE, Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, 14, CARD_RAISED);
    let border = if selected { Stroke::new(2.5, LIME) } else if response.hovered() { Stroke::new(1.5, MUTED) } else { Stroke::new(1.0, BORDER) };
    painter.rect_stroke(rect, 14, border, StrokeKind::Inside);
    let (number, name) = split_theme_name(index, theme);
    painter.text(rect.left_top() + vec2(14.0, 12.0), Align2::LEFT_TOP, number, FontId::proportional(12.0), if selected { LIME } else { MUTED });
    painter.text(rect.left_bottom() + vec2(14.0, -14.0), Align2::LEFT_BOTTOM, name, FontId::new(15.0, crate::ui::theme::bold_family()), TEXT);
    response
}

/// "Pre-set 3: Quantum time capsule" -> ("Preset 3", "Quantum time capsule").
fn split_theme_name(index: usize, theme: &str) -> (String, String) {
    match theme.split_once(": ") {
        Some((_, name)) => (format!("Preset {}", index + 1), name.to_string()),
        None => ("Your media".into(), theme.to_string()),
    }
}

fn media_library(ui: &mut Ui, page: &mut PageContext) {
    card(ui, |ui| {
        ui.label(bold("Your media", 19.0));
        ui.label(muted("Images, GIFs and videos are copied to the screen over USB (adb). Use a 2:1 ratio; the panel is 2240×1080."));
        ui.add_space(10.0);
        media_list(ui, &mut page.editor.settings);
        ui.add_space(10.0);
        custom_text_settings(ui, &mut page.editor.settings);
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
    if settings.custom.media.len() > 1 {
        ui.horizontal(|ui| {
            ui.label("Playback");
            segmented(ui, &mut settings.custom.play_mode, &[("Single".into(), "First only".into()), ("Loop".into(), "Cycle all".into())]);
        });
    }
}

fn custom_text_settings(ui: &mut Ui, settings: &mut Settings) {
    ui.horizontal(|ui| {
        ui.label("Overlay text colour");
        let mut color = parse_hex_color(&settings.custom.color).unwrap_or(Color32::WHITE);
        if ui.color_edit_button_srgba(&mut color).changed() {
            settings.custom.color = format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());
        }
        ui.add_space(24.0);
        ui.label("Alignment");
        let options: Vec<(String, String)> = ALIGNMENTS.iter().map(|align| (align.to_string(), align.to_string())).collect();
        segmented(ui, &mut settings.custom.align, &options);
    });
}

fn overlay_settings(ui: &mut Ui, settings: &mut Settings) {
    card(ui, |ui| {
        ui.label(bold("Live stats on the screen", 19.0));
        ui.label(muted(format!("Pick up to {MAX_OVERLAY_FIELDS}. They are drawn over the theme.")));
        ui.add_space(10.0);
        let overlay = settings.active_overlay_mut();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for field in OVERLAY_FIELDS {
                overlay_chip(ui, overlay, field);
            }
        });
    });
}

fn overlay_chip(ui: &mut Ui, overlay: &mut Vec<String>, field: &str) {
    let selected = overlay.iter().any(|chosen| chosen == field);
    let full = overlay.len() >= MAX_OVERLAY_FIELDS;
    let response = ui.add_enabled_ui(selected || !full, |ui| pill(ui, field, selected)).inner;
    if response.clicked() {
        if selected {
            overlay.retain(|chosen| chosen != field);
        } else {
            overlay.push(field.to_string());
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_names_split_into_number_and_title() {
        assert_eq!(split_theme_name(2, PRESETS[2]), ("Preset 3".into(), "Quantum time capsule".into()));
        assert_eq!(split_theme_name(7, CUSTOMIZATION), ("Your media".into(), "Customization".into()));
    }
}
