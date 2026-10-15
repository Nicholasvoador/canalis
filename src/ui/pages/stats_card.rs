//! KANALI's "System Information Display": up to three stats and how they are drawn over the
//! picture, for the built-in themes, the full-screen media or one half of a split screen.

use crate::ui::shell::CanalisWindow;
use crate::ui::theme::{BORDER, LIME};
use crate::ui::widgets::{bold, card, disabled_pill, muted, pill, segmented, style_row};
use crate::ui::screen_preview::parse_hex_color;
use crate::settings::{ALIGNMENTS, CPU_BADGE, DARK_TEXT, FILTERS, GPU_BADGE, LIGHT_TEXT, MAX_OVERLAY_FIELDS, OVERLAY_FIELDS, badge_is_available};
use crate::ui::format::field_title;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

impl CanalisWindow {
    /// `area` names the half of a split screen being edited, or is empty.
    pub fn stats_card(&mut self, area: &'static str, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (overlay, style) = self.shown_stats();
        let (overlay, style) = (overlay.clone(), style.clone());
        let title = div()
            .flex()
            .items_center()
            .gap_3()
            .child(bold("System Information Display", 19.0))
            .when(!area.is_empty(), |title| title.child(bold(format!("· {area}"), 19.0).text_color(rgb(LIME))))
            .child(muted(format!("Select up to {MAX_OVERLAY_FIELDS} items")));
        let alignments = ALIGNMENTS.iter().map(|align| (align.to_string(), align.to_string())).collect();
        card()
            .child(div().pb(px(14.0)).child(title))
            .child(style_row("Stats", div().flex().flex_wrap().gap(px(8.0)).children(OVERLAY_FIELDS.iter().enumerate().map(|(index, field)| stat_chip(index, field, &overlay, cx)))))
            .child(style_row("Text colour", div().flex().gap(px(8.0)).children([DARK_TEXT, LIGHT_TEXT].into_iter().enumerate().map(|(index, color)| colour_swatch(index, color, style.color == color, cx)))))
            .child(style_row("Alignment", segmented("align", &style.align, alignments, |this, align| this.shown_stats().1.align = align, cx)))
            .child(style_row("Filter", self.filter_controls(&style.filter, window, cx)))
            .child(style_row("Badges", div().flex().gap(px(8.0)).children([CPU_BADGE, GPU_BADGE].into_iter().enumerate().map(|(index, badge)| badge_chip(index, badge, &style.badges, &overlay, cx)))))
    }

    fn filter_controls(&mut self, filter: &crate::settings::Filter, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut options = vec![(None, "No filter".to_string())];
        options.extend(FILTERS.iter().map(|name| (Some(name.to_string()), name.to_string())));
        let kinds = segmented("filter", &filter.value, options, |this, kind| this.shown_stats().1.filter.value = kind, cx);
        let slider = self.sliders.filter_opacity.show(filter.opacity as f32, self.dragging, window, cx).disabled(filter.value.is_none());
        let opacity = div()
            .flex()
            .items_center()
            .gap_3()
            .pl_3()
            .when(filter.value.is_none(), |opacity| opacity.opacity(0.5))
            .child(muted("Opacity"))
            .child(div().w(px(110.0)).child(slider))
            .child(div().w(px(40.0)).child(format!("{}%", filter.opacity)));
        div().flex().flex_wrap().items_center().gap(px(8.0)).child(kinds).child(opacity)
    }
}

fn stat_chip(index: usize, field: &'static str, overlay: &[String], cx: &mut Context<CanalisWindow>) -> AnyElement {
    let selected = overlay.iter().any(|chosen| chosen == field);
    if !selected && overlay.len() >= MAX_OVERLAY_FIELDS {
        return disabled_pill("stat", index, field_title(field).to_string()).into_any_element();
    }
    pill("stat", index, field_title(field).to_string(), selected)
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            toggle(this.shown_stats().0, field);
            this.settings_changed(cx);
        }))
        .into_any_element()
}

fn colour_swatch(index: usize, color: &'static str, selected: bool, cx: &mut Context<CanalisWindow>) -> impl IntoElement + use<> {
    let hint = if color == DARK_TEXT { "Black text" } else { "Light grey text" };
    let (border_width, border) = if selected { (px(2.0), LIME) } else { (px(1.0), BORDER) };
    div()
        .id(("swatch", index))
        .size(px(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(8.0))
        .border(border_width)
        .border_color(rgb(border))
        .cursor_pointer()
        .child(div().size(px(22.0)).rounded(px(4.0)).bg(rgb(parse_hex_color(color).unwrap_or(0xFFFFFF))))
        .tooltip(move |window, cx| Tooltip::new(hint).build(window, cx))
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.shown_stats().1.color = color.into();
            this.settings_changed(cx);
        }))
}

fn badge_chip(index: usize, badge: &'static str, badges: &[String], overlay: &[String], cx: &mut Context<CanalisWindow>) -> AnyElement {
    if !badge_is_available(badge, overlay) {
        let hint = format!("Pick a {} stat to show this badge", badge.replace(" Badge", ""));
        return disabled_pill("badge", index, badge).tooltip(move |window, cx| Tooltip::new(hint.clone()).build(window, cx)).into_any_element();
    }
    let selected = badges.iter().any(|chosen| chosen == badge);
    pill("badge", index, badge, selected)
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            toggle(&mut this.shown_stats().1.badges, badge);
            this.settings_changed(cx);
        }))
        .into_any_element()
}

/// Adds `item` to the list, or removes it if it is already there.
fn toggle(list: &mut Vec<String>, item: &str) {
    if list.iter().any(|chosen| chosen == item) {
        list.retain(|chosen| chosen != item);
    } else {
        list.push(item.to_string());
    }
}
