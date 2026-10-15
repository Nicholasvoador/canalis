//! Following the mouse while something is dragged.

use super::geometry::from_pixels;
use gpui_kit::*;

/// While a drag goes on, pointer moves and the release anywhere in the window reach `moved` and
/// `released`, so the drag doesn't stop at the edge of what is being dragged. Call while painting.
pub fn follow_drag(window: &mut Window, moved: impl Fn(Point<f32>, &mut Window, &mut App) + 'static, released: impl Fn(&mut Window, &mut App) + 'static) {
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if phase == DispatchPhase::Bubble {
            moved(from_pixels(event.position), window, cx);
        }
    });
    window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
        if phase == DispatchPhase::Bubble {
            released(window, cx);
        }
    });
}
