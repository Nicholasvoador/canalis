//! Reusable pieces of the KANALI-style layout.

use super::shell::CanalisWindow;
use super::theme::{BLACK, BORDER, CARD, CARD_RAISED, LIME, MUTED, TEXT, faded};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::switch::Switch;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::rc::Rc;

pub const CARD_GAP: f32 = 20.0;
const CARD_PADDING: f32 = 24.0;
/// The sidebar, the page's side margins and the scroll bar's gutter.
const WIDTH_AROUND_CONTENT: f32 = 272.0 + 72.0 + 12.0;

/// How wide a page's cards are.
pub fn content_width(window: &Window) -> f32 {
    f32::from(window.viewport_size().width) - WIDTH_AROUND_CONTENT
}

/// How much room there is inside a card that spans the page.
pub fn card_inner_width(window: &Window) -> f32 {
    content_width(window) - 2.0 * (CARD_PADDING + 1.0)
}

pub fn bold(text: impl Into<SharedString>, size: f32) -> Div {
    div().text_size(px(size)).font_weight(FontWeight::BOLD).text_color(rgb(TEXT)).child(text.into())
}

pub fn muted(text: impl Into<SharedString>) -> Div {
    div().text_color(rgb(MUTED)).child(text.into())
}

pub fn card() -> Div {
    div().flex().flex_col().gap_2().p(px(CARD_PADDING)).bg(rgb(CARD)).border_1().border_color(rgb(BORDER)).rounded(px(16.0))
}

pub fn divider() -> Div {
    div().h(px(1.0)).w_full().flex_none().bg(rgb(BORDER))
}

pub fn page_header(title: &'static str) -> Div {
    div().flex().flex_col().gap(px(14.0)).pt_2().pb(px(14.0)).child(bold(title, 28.0)).child(divider())
}

/// KANALI-style text tabs with a short lime underline under the selected one.
pub fn tabs<T: PartialEq + Copy + 'static>(current: T, options: &[(T, &'static str)], pick: fn(&mut CanalisWindow, T), cx: &mut Context<CanalisWindow>) -> Div {
    let row = div().flex().gap(px(32.0)).pb(px(18.0));
    row.children(options.iter().enumerate().map(|(index, &(value, label))| {
        let selected = current == value;
        let text = if selected { div().font_weight(FontWeight::BOLD).text_color(rgb(TEXT)) } else { div().text_color(rgb(MUTED)) };
        let underline = div().mt(px(5.0)).w(px(22.0)).h(px(3.0)).rounded(px(2.0)).when(selected, |line| line.bg(rgb(LIME)));
        div()
            .id(("tab", index))
            .flex()
            .flex_col()
            .cursor_pointer()
            .child(text.child(label))
            .child(underline)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                pick(this, value);
                cx.notify();
            }))
    }))
}

/// Row of mutually exclusive pills; picking one changes the settings.
pub fn segmented<T: PartialEq + Clone + 'static>(
    id: &'static str,
    current: &T,
    options: Vec<(T, String)>,
    pick: impl Fn(&mut CanalisWindow, T) + 'static,
    cx: &mut Context<CanalisWindow>,
) -> Div {
    let pick = Rc::new(pick);
    div().flex().flex_wrap().gap(px(6.0)).children(options.into_iter().enumerate().map(|(index, (value, label))| {
        let selected = *current == value;
        let pick = pick.clone();
        pill(SharedString::from(id), index, label, selected).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            pick(this, value.clone());
            this.settings_changed(cx);
        }))
    }))
}

/// A rounded button that stays on one line, lime when selected.
pub fn pill(id: impl Into<SharedString>, index: usize, label: impl Into<SharedString>, selected: bool) -> Stateful<Div> {
    let (fill, text, border) = if selected { (LIME, BLACK, LIME) } else { (CARD_RAISED, TEXT, BORDER) };
    div()
        .id((id.into(), index))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(px(32.0))
        .min_w(px(56.0))
        .px(px(14.0))
        .rounded(px(18.0))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(fill))
        .text_color(rgb(text))
        .whitespace_nowrap()
        .cursor_pointer()
        .when(!selected, |pill| pill.hover(|style| style.bg(rgb(BORDER))))
        .child(label.into())
}

/// A pill that can't be picked right now.
pub fn disabled_pill(id: impl Into<SharedString>, index: usize, label: impl Into<SharedString>) -> Stateful<Div> {
    pill(id, index, label, false).opacity(0.4).cursor_default()
}

pub fn primary_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    Button::new(id).primary().label(label.into())
}

pub fn secondary_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    Button::new(id).outline().label(label.into())
}

/// A switch that turns a setting on or off.
pub fn setting_switch(id: &'static str, checked: bool, apply: fn(&mut CanalisWindow, bool), cx: &mut Context<CanalisWindow>) -> Switch {
    Switch::new(id).checked(checked).on_click(cx.listener(move |this, checked: &bool, _, cx| {
        apply(this, *checked);
        this.settings_changed(cx);
    }))
}

/// Title and description on the left, control on the right, with a divider underneath.
pub fn setting_row(title: impl Into<SharedString>, description: &'static str, control: impl IntoElement) -> Div {
    let text = div().flex().flex_col().flex_1().gap_1().child(bold(title, 15.5)).when(!description.is_empty(), |text| text.child(muted(description).text_size(px(13.0))));
    let row = div().flex().items_center().gap_6().py(px(6.0)).child(text).child(div().flex().items_center().gap_3().child(control));
    div().flex().flex_col().gap(px(6.0)).child(row).child(divider())
}

/// A label column on the left, then the row's controls, wrapping onto more lines.
pub fn style_row(label: &'static str, controls: impl IntoElement) -> Div {
    let label = div().w(px(104.0)).flex_none().h(px(32.0)).flex().items_center().text_color(rgb(MUTED)).child(label);
    div().flex().items_start().pb(px(12.0)).child(label).child(div().flex().flex_1().flex_wrap().items_center().gap(px(8.0)).child(controls))
}

/// A small coloured dot followed by text, used for connection state.
pub fn status_dot(color: u32, text: impl Into<SharedString>) -> Div {
    div().flex().items_center().gap_2().child(div().size(px(10.0)).rounded_full().bg(rgb(color))).child(text.into())
}

/// Key/value line inside info cards.
pub fn info_line(key: &'static str, value: impl Into<SharedString>) -> Div {
    div().flex().justify_between().gap_4().py_1().child(muted(key)).child(div().text_color(rgb(TEXT)).child(value.into()))
}

/// A spinner-like note shown while a job runs.
pub fn busy_note(label: &'static str) -> Div {
    div().flex().items_center().gap_2().text_color(rgb(MUTED)).child(div().size(px(8.0)).rounded_full().bg(faded(LIME, 0.8))).child(label)
}
