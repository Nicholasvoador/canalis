mod customization;
pub mod home;
pub mod panorama;
pub mod setting;
mod stats_card;

use gpui_kit::*;

/// A page's scrolling content under its fixed header.
fn page_scroll(id: &'static str, content: impl IntoElement) -> impl IntoElement {
    div().id(id).flex_1().min_h_0().overflow_y_scroll().pr(px(12.0)).child(content)
}
