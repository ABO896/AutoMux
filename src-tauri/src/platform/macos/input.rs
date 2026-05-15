use crate::platform::{InputProvider, MouseButton};
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

pub const LLMHF_INJECTED: i64 = 0x414D5558;

pub struct MacInputProvider {
    source: CGEventSource,
}

impl MacInputProvider {
    pub fn new() -> Self {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .expect("Failed to create CGEventSource");
        Self { source }
    }
}

impl InputProvider for MacInputProvider {
    fn inject_key(&self, keycode: u16, is_down: bool) {
        if let Ok(event) = CGEvent::new_keyboard_event(self.source.clone(), keycode, is_down) {
            event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
            event.post(CGEventTapLocation::HID);
        }
    }

    fn inject_mouse_click(&self, button: MouseButton, x: f64, y: f64) {
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
            CGEvent::new_mouse_event(self.source.clone(), event_type_down, position, cg_button)
        {
            event_down.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
            event_down.post(CGEventTapLocation::HID);
        }

        let event_type_up = match button {
            MouseButton::Left => CGEventType::LeftMouseUp,
            MouseButton::Right => CGEventType::RightMouseUp,
            MouseButton::Center => CGEventType::OtherMouseUp,
        };

        if let Ok(event_up) =
            CGEvent::new_mouse_event(self.source.clone(), event_type_up, position, cg_button)
        {
            event_up.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
            event_up.post(CGEventTapLocation::HID);
        }
    }

    fn inject_mouse_move(&self, x: f64, y: f64) {
        let position = CGPoint::new(x, y);
        if let Ok(event) = CGEvent::new_mouse_event(
            self.source.clone(),
            CGEventType::MouseMoved,
            position,
            CGMouseButton::Left,
        ) {
            event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
            event.post(CGEventTapLocation::HID);
        }
    }
}
