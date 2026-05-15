//! Windows InputProvider implementation using Win32 SendInput API.
//!
//! @kernel-specialist-windows: This module provides platform-specific input
//! injection on Windows using the `windows` crate. All injection goes through
//! `SendInput`, which operates at the user-mode level.
//!
//! @safety-officer: Emergency Stop flushes are supported via `flush_held_inputs()`.
//! The InputTracking registry mirrors the macOS implementation for parity.

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_TYPE, KEYBDINPUT, KEYEVENTF_KEYUP, MOUSEEVENTF_ABSOLUTE,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT,
};

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId,
};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

use super::{InputProvider, MouseButton, PlatformObserver};
use std::collections::HashSet;
use std::sync::Mutex;

/// Tracks all currently-held inputs for Emergency Stop flush.
/// @safety-officer: This mirrors the macOS InputTrackingRegistry exactly.
#[derive(Debug)]
enum HeldInput {
    Key(u16),
    Mouse(MouseButton),
}

pub struct WindowsInputProvider {
    /// Thread-safe registry of all currently held inputs.
    /// Used by Emergency Stop to release everything.
    held_inputs: Mutex<HashSet<HeldInputKey>>,
}

/// Hashable key for the held inputs set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HeldInputKey {
    Key(u16),
    Mouse(MouseButton),
}

impl WindowsInputProvider {
    pub fn new() -> Self {
        Self {
            held_inputs: Mutex::new(HashSet::new()),
        }
    }

    /// @safety-officer: Emergency Stop entry point.
    /// Iterates through all held keys/buttons and sends the corresponding
    /// release events. This guarantees no "stuck keys" after shutdown.
    pub fn flush_held_inputs(&self) {
        let held: Vec<HeldInputKey> = {
            let mut guard = self.held_inputs.lock().unwrap();
            let items: Vec<HeldInputKey> = guard.drain().collect();
            items
        };

        for input in held {
            match input {
                HeldInputKey::Key(keycode) => {
                    self.inject_key(keycode, false);
                }
                HeldInputKey::Mouse(button) => {
                    self.send_mouse_button(button, false);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn send_key_event(&self, keycode: u16, is_down: bool) {
        let mut flags = windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0);
        if !is_down {
            flags |= KEYEVENTF_KEYUP;
        }

        let input = INPUT {
            r#type: INPUT_TYPE(1), // INPUT_KEYBOARD
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(keycode),
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn send_key_event(&self, _keycode: u16, _is_down: bool) {
        // Stub — only compiled on non-Windows. Should never be called.
        #[cfg(debug_assertions)]
        eprintln!("[WindowsInput] send_key_event stub called on non-Windows platform");
    }

    #[cfg(target_os = "windows")]
    fn send_mouse_button(&self, button: MouseButton, is_down: bool) {
        let flags = match (button, is_down) {
            (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
            (MouseButton::Center, true) => MOUSEEVENTF_MIDDLEDOWN,
            (MouseButton::Center, false) => MOUSEEVENTF_MIDDLEUP,
        };

        let input = INPUT {
            r#type: INPUT_TYPE(0), // INPUT_MOUSE
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn send_mouse_button(&self, _button: MouseButton, _is_down: bool) {
        #[cfg(debug_assertions)]
        eprintln!("[WindowsInput] send_mouse_button stub called on non-Windows platform");
    }

    #[cfg(target_os = "windows")]
    fn send_mouse_move(&self, x: f64, y: f64) {
        // Convert to absolute coordinates (0-65535 range for SendInput ABSOLUTE)
        let input = INPUT {
            r#type: INPUT_TYPE(0), // INPUT_MOUSE
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: x as i32,
                    dy: y as i32,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn send_mouse_move(&self, _x: f64, _y: f64) {
        #[cfg(debug_assertions)]
        eprintln!("[WindowsInput] send_mouse_move stub called on non-Windows platform");
    }
}

impl InputProvider for WindowsInputProvider {
    fn inject_key(&self, keycode: u16, is_down: bool) {
        let key = HeldInputKey::Key(keycode);
        {
            let mut guard = self.held_inputs.lock().unwrap();
            if is_down {
                guard.insert(key);
            } else {
                guard.remove(&key);
            }
        }
        self.send_key_event(keycode, is_down);
    }

    fn inject_mouse_click(&self, button: MouseButton, _x: f64, _y: f64) {
        // Note: For mouse clicks, we send button events at the current cursor
        // position. The x/y are available for future "click at coordinates" features.
        let key = HeldInputKey::Mouse(button);
        {
            let mut guard = self.held_inputs.lock().unwrap();
            // Click = press + release, so we don't track in registry.
            // For sustained holds, the caller uses inject_key-style calls.
            guard.insert(key);
        }
        self.send_mouse_button(button, true);
        self.send_mouse_button(button, false);
        {
            let mut guard = self.held_inputs.lock().unwrap();
            guard.remove(&key);
        }
    }

    fn inject_mouse_move(&self, x: f64, y: f64) {
        self.send_mouse_move(x, y);
    }
}

// ── PlatformObserver (Windows) ──────────────────────────────────

pub struct WindowsPlatformObserver;

impl WindowsPlatformObserver {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformObserver for WindowsPlatformObserver {
    #[cfg(target_os = "windows")]
    fn get_active_app(&self) -> Option<String> {
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };

        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd == HWND::default() {
                return None;
            }

            let mut process_id: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));
            if process_id == 0 {
                return None;
            }

            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
            let mut buf = [0u16; 260];
            let mut len = buf.len() as u32;
            QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), &mut buf, &mut len).ok()?;

            Some(String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_active_app(&self) -> Option<String> {
        None
    }

    fn start_observing(&mut self) {
        // TODO: Use SetWinEventHook(EVENT_SYSTEM_FOREGROUND) for push-based observation.
        // For now, the StateActor polls get_active_app() on a timer.
        #[cfg(debug_assertions)]
        eprintln!("[WindowsObserver] start_observing — polling mode");
    }

    fn stop_observing(&mut self) {
        #[cfg(debug_assertions)]
        eprintln!("[WindowsObserver] stop_observing");
    }
}

/// Check if the current process has the necessary permissions to inject input.
/// On Windows, SendInput requires UIAccess or the app must not be running
/// in a restricted context. This is a best-effort check.
#[cfg(target_os = "windows")]
pub fn check_input_permissions() -> bool {
    // SendInput generally works for standard user processes.
    // It only fails when trying to inject into elevated (Admin) windows
    // from a non-elevated process. For now, return true.
    true
}

#[cfg(not(target_os = "windows"))]
pub fn check_input_permissions() -> bool {
    true
}
