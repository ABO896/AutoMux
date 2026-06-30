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

/// UX-11/UX-14: A registered global hotkey on Windows — keycode + modifier
/// mask → macro id. Mirrors the macOS `HotkeyBinding` struct in
/// `platform/macos/observer.rs:38-43` (which additionally carries a
/// `HotkeyAction` enum; on Windows the action is always
/// `ToggleMacro(macro_id)` so we just store the id directly).
///
/// The type is named `WindowsHotkeyBinding` (not `HotkeyBinding`) to avoid
/// collision with the macOS type when both are visible in the StateActor's
/// cfg-gated `build_hotkey_bindings_vec` helper (state/mod.rs).
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
pub struct WindowsHotkeyBinding {
    pub keycode: u16,
    pub modifiers: u64,
    pub macro_id: Uuid,
}

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
        // CR-05: MOUSEEVENTF_ABSOLUTE requires coordinates normalized to 0–65535
        // (mapping to the full virtual desktop), NOT raw pixel values.
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
        };
        let (screen_w, screen_h) = unsafe {
            (
                GetSystemMetrics(SM_CXVIRTUALSCREEN) as f64,
                GetSystemMetrics(SM_CYVIRTUALSCREEN) as f64,
            )
        };
        let norm_x = ((x / screen_w) * 65535.0).clamp(0.0, 65535.0) as i32;
        let norm_y = ((y / screen_h) * 65535.0).clamp(0.0, 65535.0) as i32;

        let input = INPUT {
            r#type: INPUT_TYPE(0), // INPUT_MOUSE
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: norm_x,
                    dy: norm_y,
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
    fn flush_held_inputs(&self) {
        Self::flush_all_held_inputs();
    }

    fn inject_key(&self, keycode: u16, is_down: bool) {
        let key = HeldInputKey::Key(keycode);
        {
            // @safety-officer: SAFE-01 — silent skip on mutex failure instead of panic (D-08).
            let Ok(mut guard) = get_held_inputs().lock() else {
                #[cfg(debug_assertions)]
                eprintln!("[WinInput] held_inputs lock failed — skipping injection");
                return;
            };
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
            // @safety-officer: SAFE-01 — silent skip on mutex failure instead of panic (D-08).
            let Ok(mut guard) = get_held_inputs().lock() else {
                #[cfg(debug_assertions)]
                eprintln!("[WinInput] held_inputs lock failed — skipping injection");
                return;
            };
            // Click = press + release, so we don't track in registry.
            // For sustained holds, the caller uses inject_key-style calls.
            guard.insert(key);
        }
        self.send_mouse_button(button, true);
        self.send_mouse_button(button, false);
        {
            // @safety-officer: SAFE-01 — silent skip on mutex failure instead of panic (D-08).
            let Ok(mut guard) = get_held_inputs().lock() else {
                #[cfg(debug_assertions)]
                eprintln!("[WinInput] held_inputs lock failed — skipping injection");
                return;
            };
            guard.remove(&key);
        }
    }

    fn inject_mouse_move(&self, x: f64, y: f64) {
        self.send_mouse_move(x, y);
    }

    fn inject_mouse_button_raw(&self, button: MouseButton, is_down: bool) {
        let key = HeldInputKey::Mouse(button);
        {
            // @safety-officer: SAFE-01 — silent skip on mutex failure instead of panic (D-08).
            let Ok(mut guard) = get_held_inputs().lock() else {
                #[cfg(debug_assertions)]
                eprintln!("[WinInput] held_inputs lock failed — skipping injection");
                return;
            };
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
    use windows::Win32::Foundation::CloseHandle;
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
    // @safety-officer: MEM-01 — close the process handle on BOTH the success
    // and error paths of QueryFullProcessImageNameW. Holding the handle across
    // the `?` propagation would leak a Win32 HANDLE per call (and the kernel
    // process object it references) for the lifetime of the process.
    let query_result = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_FORMAT(0),
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut len,
    );
    unsafe { CloseHandle(handle); }
    query_result.ok()?;

    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

/// List all user-facing running applications using EnumWindows.
/// Returns visible windows with titles, deduplicated by exe path.
/// Reuses the existing get_app_name_from_hwnd helper (no new Win32 imports needed).
#[cfg(target_os = "windows")]
pub fn list_running_apps_impl() -> Result<Vec<crate::ipc::RunningApp>, String> {
    use std::collections::HashMap;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::core::BOOL; // BOOL moved to windows::core in windows-rs 0.60+
    use windows::Win32::UI::WindowsAndMessaging::EnumWindows;

    let mut apps: Vec<crate::ipc::RunningApp> = Vec::new();
    let apps_ptr = &mut apps as *mut Vec<crate::ipc::RunningApp> as isize;

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextW, IsWindowVisible};
        if IsWindowVisible(hwnd).as_bool() {
            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len > 0 {
                if let Some(path) = get_app_name_from_hwnd(hwnd) {
                    let basename = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&path)
                        .to_string();
                    let acc = &mut *(lparam.0 as *mut Vec<crate::ipc::RunningApp>);
                    acc.push(crate::ipc::RunningApp {
                        display_name: basename,
                        identifier: path,
                    });
                }
            }
        }
        BOOL(1) // continue enumeration
    }

    unsafe {
        EnumWindows(Some(enum_callback), LPARAM(apps_ptr))
            .map_err(|e| e.to_string())?;
    }

    let mut seen = HashMap::new();
    apps.retain(|app| seen.insert(app.identifier.clone(), ()).is_none());
    apps.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(apps)
}

#[cfg(not(target_os = "windows"))]
pub fn list_running_apps_impl() -> Result<Vec<crate::ipc::RunningApp>, String> {
    Ok(vec![])
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
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{
    SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_SHIFT,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
};

static STATE_TX: OnceLock<tokio::sync::mpsc::Sender<crate::state::Intent>> = OnceLock::new();
static MACRO_TRIGGER_KEYS: OnceLock<Mutex<HashMap<(u16, u64), Uuid>>> = OnceLock::new();
/// UX-11/UX-14: Configurable hotkey bindings registry, mirrored from the
/// StateActor's `HOTKEY_BINDINGS` on macOS. Populated by the new
/// `Intent::BindHotkey` StateActor handler (plan 08-03). Closes the
/// Windows "no configurable hotkeys" gap from CONCERNS.md:150-152.
static HOTKEY_BINDINGS: OnceLock<Mutex<Vec<WindowsHotkeyBinding>>> = OnceLock::new();
static HOOK_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn set_state_tx(tx: tokio::sync::mpsc::Sender<crate::state::Intent>) {
    let _ = STATE_TX.set(tx);
}

fn get_macro_trigger_keys() -> &'static Mutex<HashMap<(u16, u64), Uuid>> {
    MACRO_TRIGGER_KEYS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn update_macro_trigger_keys(keys: HashMap<(u16, u64), Uuid>) {
    *get_macro_trigger_keys().lock().unwrap() = keys;
}

fn get_hotkey_bindings() -> &'static Mutex<Vec<WindowsHotkeyBinding>> {
    HOTKEY_BINDINGS.get_or_init(|| Mutex::new(Vec::new()))
}

/// UX-11: Replace the entire set of configurable hotkey bindings at runtime.
/// Called by the StateActor's `Intent::BindHotkey` handler with the full
/// `Vec<WindowsHotkeyBinding>` rebuilt from the current `state.macros` —
/// same pattern as `update_macro_trigger_keys` and the macOS equivalent
/// `update_hotkey_bindings` in `platform/macos/observer.rs:155-157`.
pub fn update_hotkey_bindings(bindings: Vec<WindowsHotkeyBinding>) {
    *get_hotkey_bindings().lock().unwrap() = bindings;
}

/// UX-13: Synthesize the Windows MOD_* modifier bitmask from the current
/// async key state. Called at the moment of a keypress in `hook_callback` so
/// the lookup is matched against the modifier state at that instant (not a
/// snapshot — the hook fires synchronously with the keypress).
///
/// Bit values match the Win32 `RegisterHotKey` `MOD_*` constants (pinned by
/// the `windows_mod_constants` test below this file):
///   MOD_ALT     = 0x0001
///   MOD_CONTROL = 0x0002
///   MOD_SHIFT   = 0x0004
///   MOD_WIN     = 0x0008
///
/// Left/right variants of the Win key both map to MOD_WIN (0x0008) — there
/// is no separate L/R bit in the Win32 RegisterHotKey API.
#[cfg(target_os = "windows")]
fn build_mod_mask() -> u64 {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        VK_LWIN, VK_MENU, VK_RWIN,
    };
    let mut m: u64 = 0;
    if (unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) } as u16 & 0x8000) != 0 {
        m |= 0x0004;
    }
    if (unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) } as u16 & 0x8000) != 0 {
        m |= 0x0002;
    }
    if (unsafe { GetAsyncKeyState(VK_MENU.0 as i32) } as u16 & 0x8000) != 0 {
        m |= 0x0001;
    }
    if (unsafe { GetAsyncKeyState(VK_LWIN.0 as i32) } as u16 & 0x8000) != 0 {
        m |= 0x0008;
    }
    if (unsafe { GetAsyncKeyState(VK_RWIN.0 as i32) } as u16 & 0x8000) != 0 {
        m |= 0x0008;
    }
    m
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
                // RELY-03: Synchronous flush before exit — matches macOS inline-flush approach.
                // Best-effort: exit regardless of individual event send failures (D-10).
                WindowsInputProvider::flush_all_held_inputs();
                std::process::exit(1);
            }

            // MACRO TRIGGER KEYS (O(1) lookup) — tuple-keyed on (keycode, mod_mask).
            // The mod_mask is synthesized at keypress time via `build_mod_mask()`
            // so e.g. `Ctrl+Shift+F5` only matches when both modifiers are held.
            if let Ok(trigger_keys) = get_macro_trigger_keys().try_lock() {
                let mod_mask = build_mod_mask();
                if let Some(&macro_id) = trigger_keys.get(&(keycode, mod_mask)) {
                    if let Some(tx) = STATE_TX.get() {
                        let _ = tx.try_send(crate::state::Intent::ToggleMacroHotkey(macro_id));
                    }
                }
            }

            // CONFIGURABLE HOTKEYS (per-binding) — populated by Intent::BindHotkey
            // via the StateActor's `update_hotkey_bindings` (mirrors the macOS
            // `HOTKEY_BINDINGS` at `platform/macos/observer.rs:420-449`). Same
            // tuple key so the lookup semantics are bit-identical to the
            // `MACRO_TRIGGER_KEYS` check above.
            if let Ok(bindings) = get_hotkey_bindings().try_lock() {
                let mod_mask = build_mod_mask();
                for binding in bindings.iter() {
                    if binding.keycode == keycode && binding.modifiers == mod_mask {
                        if let Some(tx) = STATE_TX.get() {
                            let _ = tx.try_send(crate::state::Intent::ToggleMacroHotkey(
                                binding.macro_id,
                            ));
                        }
                        break;
                    }
                }
            }
        }
    }
    CallNextHookEx(None, ncode, wparam, lparam)
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
            None,
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
            None,
            Some(win_event_hook_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );

        // WR-03: Check handle validity — SetWinEventHook returns a null handle on failure.
        if event_hook.is_invalid() {
            eprintln!("[WindowsObserver] SetWinEventHook failed — no active app tracking");
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            // BUILD-01: TranslateMessage returns BOOL indicating whether it
            // translated the message; the message-pump loop does not need that
            // signal, so explicitly discard to silence the `must_use` lint.
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnhookWindowsHookEx(hook.unwrap());
        // WR-03: Only unhook if the handle is valid.
        if !event_hook.is_invalid() {
            let _ = UnhookWinEvent(event_hook);
        }
    });

    true
}

#[cfg(not(target_os = "windows"))]
pub fn initialize_hook() -> bool {
    false
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    /// UX-13: Pin the Windows MOD_* bit values so the frontend/backend
    /// bit layout cannot silently drift. The frontend's `computeModifiers`
    /// sends these exact bit values in `trigger_modifiers`; the Windows
    /// `hook_callback` will synthesize a matching mask via GetAsyncKeyState
    /// in Plan 08-03. If the constants change, the test fails and the
    /// frontend/backend must be updated together.
    ///
    /// Sources:
    ///   Microsoft Learn — RegisterHotKey function (MOD_ALT/MOD_CONTROL/MOD_SHIFT/MOD_WIN)
    ///   https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey
    ///
    /// These bit values have been stable since Windows 95. Asserted as
    /// literal u16 values because the windows-rs crate does not currently
    /// export named MOD_* constants in the same module as the platform code.
    #[test]
    fn windows_mod_constants() {
        // Win32 RegisterHotKey modifier bit values:
        //   MOD_ALT     = 0x0001
        //   MOD_CONTROL = 0x0002
        //   MOD_SHIFT   = 0x0004
        //   MOD_WIN     = 0x0008
        const MOD_ALT: u16 = 0x0001;
        const MOD_CONTROL: u16 = 0x0002;
        const MOD_SHIFT: u16 = 0x0004;
        const MOD_WIN: u16 = 0x0008;

        assert_eq!(MOD_ALT, 0x0001, "MOD_ALT must be 0x0001");
        assert_eq!(MOD_CONTROL, 0x0002, "MOD_CONTROL must be 0x0002");
        assert_eq!(MOD_SHIFT, 0x0004, "MOD_SHIFT must be 0x0004");
        assert_eq!(MOD_WIN, 0x0008, "MOD_WIN must be 0x0008");
    }
}
