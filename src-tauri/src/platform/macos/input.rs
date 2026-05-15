use crate::platform::{InputProvider, MouseButton};
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

pub const LLMHF_INJECTED: i64 = 0x414D5558;

/// macOS input injection via CoreGraphics.
///
/// @safety-officer: CGEventSource wraps a NonNull pointer that is NOT Send.
/// We create a fresh CGEventSource per injection call instead of storing one,
/// because the StateActor runs on a tokio Send task. Creation cost is negligible
/// (~200ns — it's a CF object lookup, not a system call).
pub struct MacInputProvider;

impl MacInputProvider {
    pub fn new() -> Self {
        Self
    }

    /// Create a CGEventSource for the current call.
    fn source() -> CGEventSource {
        CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .expect("Failed to create CGEventSource")
    }
}

impl InputProvider for MacInputProvider {
    fn inject_key(&self, keycode: u16, is_down: bool) {
        let source = Self::source();
        if let Ok(event) = CGEvent::new_keyboard_event(source, keycode, is_down) {
            event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
            event.post(CGEventTapLocation::HID);
        }
    }

    fn inject_mouse_click(&self, button: MouseButton, x: f64, y: f64) {
        let source = Self::source();
        let position = CGPoint::new(x, y);

        let cg_button = match button {
            MouseButton::Left => CGMouseButton::Left,
            MouseButton::Right => CGMouseButton::Right,
            MouseButton::Center => CGMouseButton::Center,
        };

        let event_type_down = match button {
            MouseButton::Left => CGEventType::LeftMouseDown,
            MouseButton::Right => CGEventType::RightMouseDown,
            MouseButton::Center => CGEventType::OtherMouseDown,
        };

        if let Ok(event_down) =
            CGEvent::new_mouse_event(source.clone(), event_type_down, position, cg_button)
        {
            event_down.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
            event_down.post(CGEventTapLocation::HID);
        }

        let event_type_up = match button {
            MouseButton::Left => CGEventType::LeftMouseUp,
            MouseButton::Right => CGEventType::RightMouseUp,
            MouseButton::Center => CGEventType::OtherMouseUp,
        };

        if let Ok(event_up) = CGEvent::new_mouse_event(source, event_type_up, position, cg_button) {
            event_up.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
            event_up.post(CGEventTapLocation::HID);
        }
    }

    fn inject_mouse_move(&self, x: f64, y: f64) {
        let source = Self::source();
        let position = CGPoint::new(x, y);
        if let Ok(event) = CGEvent::new_mouse_event(
            source,
            CGEventType::MouseMoved,
            position,
            CGMouseButton::Left,
        ) {
            event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
            event.post(CGEventTapLocation::HID);
        }
    }

    fn inject_mouse_button_raw(&self, button: MouseButton, is_down: bool) {
        let source = Self::source();

        // Get the current cursor position for the event.
        let pos = if let Ok(ev) = CGEvent::new(source.clone()) {
            ev.location()
        } else {
            CGPoint::new(0.0, 0.0)
        };

        let cg_button = match button {
            MouseButton::Left => CGMouseButton::Left,
            MouseButton::Right => CGMouseButton::Right,
            MouseButton::Center => CGMouseButton::Center,
        };

        let event_type = match (button, is_down) {
            (MouseButton::Left, true) => CGEventType::LeftMouseDown,
            (MouseButton::Left, false) => CGEventType::LeftMouseUp,
            (MouseButton::Right, true) => CGEventType::RightMouseDown,
            (MouseButton::Right, false) => CGEventType::RightMouseUp,
            (MouseButton::Center, true) => CGEventType::OtherMouseDown,
            (MouseButton::Center, false) => CGEventType::OtherMouseUp,
        };

        if let Ok(event) = CGEvent::new_mouse_event(source, event_type, pos, cg_button) {
            event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
            event.post(CGEventTapLocation::HID);
        }
    }

    fn flush_held_inputs(&self) {
        crate::platform::macos::observer::flush_held_inputs();
    }
}

impl Default for MacInputProvider {
    fn default() -> Self {
        Self::new()
    }
}
