#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Center,
}

pub trait InputProvider {
    fn inject_key(&self, keycode: u16, is_down: bool);
    fn inject_mouse_click(&self, button: MouseButton, x: f64, y: f64);
    fn inject_mouse_move(&self, x: f64, y: f64);
    /// Send an individual mouse button down or up event at the current cursor position.
    /// Required for sustained holds (where down and up are separated in time).
    fn inject_mouse_button_raw(&self, button: MouseButton, is_down: bool);
}

pub trait PlatformObserver {
    fn get_active_app(&self) -> Option<String>;
    fn start_observing(&mut self);
    fn stop_observing(&mut self);
}

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;
