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
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

use super::{InputProvider, MouseButton, PlatformObserver};
use std::collections::HashSet;
use std::sync::Mutex;

static HELD_INPUTS: OnceLock<Mutex<HashSet<HeldInputKey>>> = OnceLock::new();

fn get_held_inputs() -> &'static Mutex<HashSet<HeldInputKey>> {
    HELD_INPUTS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub struct WindowsInputProvider;

/// Hashable key for the held inputs set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HeldInputKey {
    Key(u16),
    Mouse(MouseButton),
}

impl WindowsInputProvider {
    pub fn new() -> Self {
        Self
    }

    /// @safety-officer: Emergency Stop entry point.
    /// Iterates through all held keys/buttons and sends the corresponding
    /// release events. This guarantees no "stuck keys" after shutdown.
    pub fn flush_held_inputs(&self) {
        Self::flush_all_held_inputs();
    }

    pub fn flush_all_held_inputs() {
        let held: Vec<HeldInputKey> = {
            let mut guard = get_held_inputs().lock().unwrap();
            guard.drain().collect()
        };

        let provider = WindowsInputProvider::new();
        for input in held {
            match input {
                HeldInputKey::Key(keycode) => {
                    provider.send_key_event(keycode, false);
                }
                HeldInputKey::Mouse(button) => {
                    provider.send_mouse_button(button, false);
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
            let mut guard = get_held_inputs().lock().unwrap();
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
            let mut guard = get_held_inputs().lock().unwrap();
            // Click = press + release, so we don't track in registry.
            // For sustained holds, the caller uses inject_key-style calls.
            guard.insert(key);
        }
        self.send_mouse_button(button, true);
        self.send_mouse_button(button, false);
        {
            let mut guard = get_held_inputs().lock().unwrap();
            guard.remove(&key);
        }
    }

    fn inject_mouse_move(&self, x: f64, y: f64) {
        self.send_mouse_move(x, y);
    }

    fn inject_mouse_button_raw(&self, button: MouseButton, is_down: bool) {
        let key = HeldInputKey::Mouse(button);
        {
            let mut guard = get_held_inputs().lock().unwrap();
            if is_down {
                guard.insert(key);
            } else {
                guard.remove(&key);
            }
        }
        self.send_mouse_button(button, is_down);
    }
}

// ── PlatformObserver (Windows) ──────────────────────────────────

pub struct WindowsPlatformObserver;

impl WindowsPlatformObserver {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "windows")]
unsafe fn get_app_name_from_hwnd(hwnd: HWND) -> Option<String> {
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

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
    QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_FORMAT(0),
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut len,
    )
    .ok()?;

    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

impl PlatformObserver for WindowsPlatformObserver {
    #[cfg(target_os = "windows")]
    fn get_active_app(&self) -> Option<String> {
        unsafe { get_app_name_from_hwnd(GetForegroundWindow()) }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_active_app(&self) -> Option<String> {
        None
    }

    fn start_observing(&mut self) {
        initialize_hook();

        #[cfg(debug_assertions)]
        eprintln!("[WindowsObserver] start_observing — push-based mode initialized via hooks");
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
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use uuid::Uuid;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{
    SetWinEventHook, UnhookWinEvent, EVENT_SYSTEM_FOREGROUND, HWINEVENTHOOK, WINEVENT_OUTOFCONTEXT,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_SHIFT};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

static STATE_TX: OnceLock<tokio::sync::mpsc::Sender<crate::state::Intent>> = OnceLock::new();
static MACRO_TRIGGER_KEYS: OnceLock<Mutex<HashMap<u16, Uuid>>> = OnceLock::new();
static HOOK_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn set_state_tx(tx: tokio::sync::mpsc::Sender<crate::state::Intent>) {
    let _ = STATE_TX.set(tx);
}

fn get_macro_trigger_keys() -> &'static Mutex<HashMap<u16, Uuid>> {
    MACRO_TRIGGER_KEYS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn update_macro_trigger_keys(keys: HashMap<u16, Uuid>) {
    *get_macro_trigger_keys().lock().unwrap() = keys;
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn hook_callback(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode >= 0 {
        let kb_struct = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let msg_id = wparam.0 as u32;
        if msg_id == WM_KEYDOWN || msg_id == WM_SYSKEYDOWN {
            let keycode = kb_struct.vkCode as u16;

            // Emergency stop check: Ctrl + Shift + Q
            let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let shift_down = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;

            // VK_Q is 0x51
            if keycode == 0x51 && ctrl_down && shift_down {
                println!("EMERGENCY STOP TRIGGERED");
                if let Some(tx) = STATE_TX.get() {
                    let _ = tx.try_send(crate::state::Intent::TriggerEmergencyStop);
                }
                std::process::exit(1);
            }

            // MACRO TRIGGER KEYS (O(1) lookup)
            if let Ok(trigger_keys) = get_macro_trigger_keys().try_lock() {
                if let Some(&macro_id) = trigger_keys.get(&keycode) {
                    if let Some(tx) = STATE_TX.get() {
                        let _ = tx.try_send(crate::state::Intent::ToggleMacroHotkey(macro_id));
                    }
                }
            }
        }
    }
    CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn win_event_hook_callback(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if event == EVENT_SYSTEM_FOREGROUND {
        let app_name = get_app_name_from_hwnd(hwnd);
        if let Some(tx) = STATE_TX.get() {
            let _ = tx.try_send(crate::state::Intent::ActiveAppChanged(app_name));
        }
    }
}

#[cfg(target_os = "windows")]
pub fn initialize_hook() -> bool {
    if HOOK_INITIALIZED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return true;
    }

    std::thread::spawn(|| unsafe {
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(hook_callback),
            windows::Win32::Foundation::HINSTANCE::default(),
            0,
        );

        if hook.is_err() {
            eprintln!("Failed to install Windows keyboard hook");
            HOOK_INITIALIZED.store(false, Ordering::SeqCst);
            return;
        }

        let event_hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            HMODULE::default(),
            Some(win_event_hook_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnhookWindowsHookEx(hook.unwrap());
        let _ = UnhookWinEvent(event_hook);
    });

    true
}

#[cfg(not(target_os = "windows"))]
pub fn initialize_hook() -> bool {
    false
}
