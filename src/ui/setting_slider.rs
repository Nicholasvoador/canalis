//! A GPUI Kit slider tied to one setting: dragging changes the setting (saved once released), and
//! the slider follows the setting when it changes elsewhere, such as from the command line.

use super::shell::CanalisWindow;
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::*;

/// Writes a slider's value into the settings.
pub type ApplyValue = fn(&mut CanalisWindow, f32);

pub struct SliderRange {
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

pub struct SettingSlider {
    state: Entity<SliderState>,
}

impl SettingSlider {
    pub fn new(range: SliderRange, apply: ApplyValue, cx: &mut Context<CanalisWindow>) -> Self {
        let state = cx.new(|_| SliderState::new().min(range.min).max(range.max).step(range.step).default_value(range.min));
        cx.subscribe(&state, move |this, _, event: &SliderEvent, cx| {
            let (value, dragging) = match event {
                SliderEvent::Change(value) => (value.start(), true),
                SliderEvent::Release(value) => (value.start(), false),
            };
            apply(this, value);
            this.dragging = dragging;
            this.settings_changed(cx);
        })
        .detach();
        Self { state }
    }

    /// The slider, moved to `value` first unless the user is dragging it.
    pub fn show(&self, value: f32, dragging: bool, window: &mut Window, cx: &mut App) -> Slider {
        let shown = self.state.read(cx).value().start();
        if !dragging && (shown - value).abs() > f32::EPSILON {
            self.state.update(cx, |state, cx| state.set_value(value, window, cx));
        }
        Slider::new(&self.state)
    }
}
