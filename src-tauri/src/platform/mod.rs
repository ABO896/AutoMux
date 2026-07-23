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

    /// Flushes all inputs that were sent as "down" but haven't been released yet.
    /// Used by Emergency Stop to prevent stuck keys.
    fn flush_held_inputs(&self);
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

/// @safety-officer: LLKHF_INJECTED marks a `WH_KEYBOARD_LL` event as
/// `SendInput`/`keybd_event`-synthesized. Windows' `hook_callback`
/// (`platform/windows/mod.rs`) MUST treat these identically to the macOS
/// CGEventTap's `LLMHF_INJECTED` guard (`platform/macos/observer.rs:279-281`)
/// by skipping the emergency-stop check AND hotkey matching, or a macro's
/// own injected keystroke can self-trigger a toggle or emergency stop
/// (CR-02, `.planning/todos/pending/2026-07-23-windows-hook-injected-event-filtering.md`).
///
/// Defined here in `platform/mod.rs` rather than inside `platform::windows`
/// because `pub mod windows;` above is itself gated
/// `#[cfg(target_os = "windows")]` — anything defined inside that module,
/// ungated or not, is compiled out entirely on non-Windows hosts. Living in
/// this unconditionally-compiled parent module is what makes the predicate
/// (and its `injected_filter_tests` below) actually unit-testable via plain
/// `cargo test` on this macOS host, independent of Windows cross-compile
/// target availability.
#[allow(dead_code)]
pub(crate) fn flags_indicate_injected(flags: u32) -> bool {
    // Win32 `winuser.h` constant, stable since Windows 2000 — same
    // "hardcode the stable Win32 constant with a citation comment" pattern
    // already used for MOD_ALT/MOD_CONTROL/MOD_SHIFT/MOD_WIN in
    // `platform::windows::tests::windows_mod_constants`.
    const LLKHF_INJECTED: u32 = 0x0000_0010;
    flags & LLKHF_INJECTED != 0
}

/// CR-02: `hook_callback` only compiles under `#[cfg(target_os = "windows")]`,
/// and this host has no `x86_64-pc-windows-msvc` target installed, so it
/// cannot be exercised end-to-end here. `flags_indicate_injected` was
/// extracted as a plain, host-portable `u32 -> bool` predicate specifically
/// so the fix's core logic — distinguishing SendInput-injected keystrokes
/// from real hardware input — gets real, automated regression coverage
/// regardless of Windows cross-compile availability.
#[cfg(test)]
mod injected_filter_tests {
    use super::flags_indicate_injected;

    #[test]
    fn real_hardware_keydown_is_not_injected() {
        assert!(!flags_indicate_injected(0x0));
    }

    #[test]
    fn extended_key_flag_alone_is_not_injected() {
        // LLKHF_EXTENDED (0x1) is set for many real keys (arrows, numpad)
        // and must not be mistaken for LLKHF_INJECTED (0x10).
        assert!(!flags_indicate_injected(0x1));
    }

    #[test]
    fn sendinput_event_is_injected() {
        assert!(flags_indicate_injected(0x10));
    }

    #[test]
    fn injected_plus_extended_flags_still_detected() {
        assert!(flags_indicate_injected(0x11));
    }
}
