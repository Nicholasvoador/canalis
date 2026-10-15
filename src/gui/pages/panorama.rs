//! PANORAMA, laid out like KANALI's page: built-in themes in a carousel, your own media, and the
//! live stats drawn over either, plus screen and fan settings.

use super::page_scroll;
use crate::gui::carousel::{Carousel, CarouselClick};
use crate::gui::crop_dialog::CropDialog;
use crate::gui::fan_curve::{DragHandlers, FanCurveEditor};
use crate::gui::geometry::from_pixels;
use crate::gui::images::ImageBin;
use crate::gui::screen_preview::PreviewSources;
use crate::gui::shell::CanalisWindow;
use crate::gui::widgets::{bold, card, card_inner_width, muted, page_header, pill, segmented, setting_row, setting_switch, style_row, tabs};
use crate::settings::{CELSIUS, FAHRENHEIT, FIXED_MODE, FanCurvePreset, OverlayStyle, PRESETS, SMART_MODE, Settings};
use crate::ui::StartPage;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

const SLIDER_WIDTH: f32 = 260.0;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum PanoramaTab {
    #[default]
    Presets,
    Customization,
    Screen,
    Fan,
}

impl PanoramaTab {
    pub fn for_start_page(page: StartPage) -> Self {
        match page {
            StartPage::Customization => PanoramaTab::Customization,
            StartPage::Screen => PanoramaTab::Screen,
            StartPage::Fan => PanoramaTab::Fan,
            _ => PanoramaTab::Presets,
        }
    }
}

pub struct PanoramaPage {
    pub tab: PanoramaTab,
    pub carousel: Carousel,
    /// Which half of the split screen is being edited: 0 left, 1 right.
    pub half: usize,
    pub cropper: CropDialog,
    pub preview: PreviewSources,
    pub fan_curve: FanCurveEditor,
}

impl PanoramaPage {
    pub fn new(tab: PanoramaTab, settings: &Settings, bin: &ImageBin) -> Self {
        Self {
            tab,
            carousel: Carousel::new(settings.preset_index().unwrap_or(0)),
            half: 0,
            cropper: CropDialog::new(bin.clone()),
            preview: PreviewSources::new(bin),
            fan_curve: FanCurveEditor::default(),
        }
    }
}

impl CanalisWindow {
    pub fn panorama_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let options = [(PanoramaTab::Presets, "Pre-set"), (PanoramaTab::Customization, "Customization"), (PanoramaTab::Screen, "Screen"), (PanoramaTab::Fan, "Fan")];
        let tabs = tabs(self.panorama.tab, &options, |this, tab| this.panorama.tab = tab, cx);
        let content = match self.panorama.tab {
            PanoramaTab::Presets => self.preset_tab(window, cx).into_any_element(),
            PanoramaTab::Customization => self.customization_tab(window, cx).into_any_element(),
            PanoramaTab::Screen => self.screen_tab(window, cx).into_any_element(),
            PanoramaTab::Fan => self.fan_tab(window, cx).into_any_element(),
        };
        div().flex().flex_col().size_full().child(page_header("PANORAMA")).child(tabs).child(page_scroll("panorama-scroll", content)).into_any_element()
    }

    /// The stats being edited: the built-in themes', the full-screen media's or one half's.
    pub fn shown_stats(&mut self) -> (&mut Vec<String>, &mut OverlayStyle) {
        let settings = &mut self.editor.settings;
        match self.panorama.tab {
            PanoramaTab::Customization if settings.custom.is_split() => {
                let half = &mut settings.custom.halves[self.panorama.half];
                (&mut half.overlay, &mut half.style)
            }
            PanoramaTab::Customization => (&mut settings.custom.overlay, &mut settings.custom.style),
            _ => (&mut settings.overlay, &mut settings.overlay_style),
        }
    }

    fn preset_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The centre card plays its theme's video, so the page keeps drawing frames.
        window.request_animation_frame();
        let selected = self.editor.settings.preset_index();
        let carousel = self.panorama.carousel.element(card_inner_width(window), &mut self.panorama.preview.presets, selected);
        let carousel = div().id("carousel").pt_2().child(carousel).on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, cx| this.click_carousel(from_pixels(event.position), cx)));
        let themes = card().child(bold("Pre-set", 19.0)).child(muted("Click a side card to bring it forward, then click the centre card to show it on the screen.")).child(carousel);
        div().flex().flex_col().gap(px(14.0)).child(themes).child(self.stats_card("", window, cx))
    }

    fn click_carousel(&mut self, spot: Point<f32>, cx: &mut Context<Self>) {
        match self.panorama.carousel.click(spot) {
            CarouselClick::Choose(index) => {
                self.editor.settings.theme = PRESETS[index].into();
                self.settings_changed(cx);
            }
            CarouselClick::Moved => cx.notify(),
            CarouselClick::Missed => {}
        }
    }

    fn screen_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = &self.editor.settings;
        let brightness = settings.brightness;
        let slider = self.sliders.brightness.show(brightness as f32, self.dragging, window, cx);
        let brightness_control = div().flex().items_center().gap_3().child(div().w(px(SLIDER_WIDTH)).child(slider)).child(format!("{brightness}%"));
        let units = vec![(FAHRENHEIT.to_string(), "°F".to_string()), (CELSIUS.to_string(), "°C".to_string())];
        let settings = &self.editor.settings;
        card()
            .child(setting_row("Screen display", "Turn the cooler's screen on or off.", setting_switch("screen-display", settings.enable, |this, on| this.editor.settings.enable = on, cx)))
            .child(setting_row("Brightness", "", brightness_control))
            .child(setting_row(
                "Mirror mode",
                "A mirror image of the screen for left-mounted cases. The cooler restarts when this changes.",
                setting_switch("mirror-mode", settings.is_mirrored(), |this, on| this.editor.settings.set_mirrored(on), cx),
            ))
            .child(setting_row(
                "Black screen while the PC sleeps",
                "On: the screen goes dark when the PC is off or the app is closed. Off: it shows a sleep animation.",
                setting_switch("sleep-display", settings.display_in_sleep, |this, on| this.editor.settings.display_in_sleep = on, cx),
            ))
            .child(setting_row("Temperature unit", "", segmented("unit", &settings.temperature_unit.clone(), units, |this, unit| this.editor.settings.temperature_unit = unit, cx)))
    }

    fn fan_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let connected = self.connected();
        let fan_rpm = self.status().number("/device_status/status/fanLCD");
        let mode = self.editor.settings.fan.mode.clone();
        let modes = vec![(SMART_MODE.to_string(), "Smart (follows CPU temperature)".to_string()), (FIXED_MODE.to_string(), "Fixed speed".to_string())];
        let title = div().flex().items_center().gap_3().child(bold("Pump-head fan", 19.0)).when(connected, |title| title.child(muted(format!("· {fan_rpm:.0} RPM now"))));
        let controls = if mode == FIXED_MODE { self.fixed_fan_speed(window, cx).into_any_element() } else { self.smart_fan_curve(window, cx).into_any_element() };
        card()
            .child(title)
            .child(muted("The pump speed itself is fixed by the cooler; only the small fan on the pump head can be controlled."))
            .child(div().py_2().child(segmented("fan-mode", &mode, modes, |this, mode| this.editor.settings.fan.mode = mode, cx)))
            .child(controls)
    }

    fn fixed_fan_speed(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let percent = self.editor.settings.fan.fixed_percent;
        let slider = self.sliders.fan_speed.show(percent as f32, self.dragging, window, cx);
        div().flex().items_center().gap_3().child(div().w(px(SLIDER_WIDTH)).child(slider)).child(format!("{percent}%")).child(muted("Fan speed"))
    }

    fn smart_fan_curve(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let curve = self.editor.settings.fan.smart_curve.clone();
        let presets = FanCurvePreset::ALL.into_iter().enumerate().map(|(index, preset)| {
            pill("fan-preset", index, preset.label(), curve == preset.curve()).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.editor.settings.fan.smart_curve = preset.curve();
                this.settings_changed(cx);
            }))
        });
        let current_temperature = self.connected().then(|| self.status().number("/snapshot/cpu/temperature") as f32);
        let entity = cx.entity().downgrade();
        let released = entity.clone();
        let handlers = DragHandlers {
            moved: move |spot, _: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |this, cx| {
                    this.panorama.fan_curve.drag(&mut this.editor.settings.fan.smart_curve, spot);
                    this.settings_changed(cx);
                });
            },
            released: move |_: &mut Window, cx: &mut App| {
                let _ = released.update(cx, |this, cx| {
                    this.panorama.fan_curve.dragging = None;
                    this.dragging = false;
                    this.settings_changed(cx);
                });
            },
        };
        let editor = self.panorama.fan_curve.element(card_inner_width(window), &curve, current_temperature, handlers);
        let editor = div()
            .id("fan-curve")
            .child(editor)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, cx| {
                this.dragging = this.panorama.fan_curve.press(&this.editor.settings.fan.smart_curve, from_pixels(event.position));
                cx.notify();
            }))
            .on_mouse_move(cx.listener(|_, _: &MouseMoveEvent, _, cx| cx.notify()));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(style_row("Start from", div().flex().flex_wrap().gap(px(8.0)).children(presets)))
            .child(editor)
            .child(muted("Drag the points. Temperatures are CPU °C; values are fan %.").text_size(px(12.5)))
    }
}
