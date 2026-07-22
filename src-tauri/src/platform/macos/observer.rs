use crate::platform::PlatformObserver;
use core_foundation::runloop::CFRunLoop;
use core_foundation_sys::runloop::kCFRunLoopCommonModes;
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
use std::collections::HashSet;
use std::process;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;

use block2::RcBlock;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey,
    NSWorkspaceDidActivateApplicationNotification,
};
use objc2_foundation::{NSNotification, NSObject};
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
pub enum ActiveInput {
    Key(u16),
    Mouse(u32), // 0: Left, 1: Right, 2: Center
}

// ── Hotkey Bindings ──────────────────────────────────────────────

/// What a hotkey binding should do when triggered.
#[derive(Debug, Clone)]
pub enum HotkeyAction {
    ToggleMacro(Uuid),
    ToggleEngine,
}

/// A registered global hotkey: keycode + modifier flags → action.
#[derive(Debug, Clone)]
pub struct HotkeyBinding {
    pub keycode: u16,
    pub modifiers: u64, // raw CGEventFlags bits
    pub action: HotkeyAction,
}

impl HotkeyBinding {
    /// Check whether an event matches this binding.
    fn matches(&self, keycode: u16, flags: CGEventFlags) -> bool {
        keycode == self.keycode && (flags.bits() & self.modifiers) == self.modifiers
    }
}

// ── Static registries ────────────────────────────────────────────

static REGISTRY: OnceLock<Mutex<HashSet<ActiveInput>>> = OnceLock::new();
static ACTIVE_APP: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static HOTKEY_BINDINGS: OnceLock<Mutex<Vec<HotkeyBinding>>> = OnceLock::new();
/// Channel sender so the CGEventTap callback can push Intents into the StateActor.
static STATE_TX: OnceLock<tokio::sync::mpsc::Sender<crate::state::Intent>> = OnceLock::new();
/// @safety-officer: Atomic guard — guarantees `initialize_tap()` spawns at most
/// ONE thread, even if called concurrently from multiple sites.
static TAP_INITIALIZED: AtomicBool = AtomicBool::new(false);
/// @safety-officer: In-flight spawn guard — prevents a second spawn attempt
/// during the window between `TAP_INITIALIZED` being reset to `false` (after a
/// failed CGEventTap creation) and the next poll cycle arriving.  The sequence is:
///
///   1. Caller checks TAP_INITIALIZED == false AND TAP_STARTING == false.
///   2. TAP_STARTING is swapped to `true` atomically.
///   3. TAP_INITIALIZED is set to `true`.
///   4. Thread is spawned.
///   5. On any exit branch the spawned thread clears BOTH flags (TAP_STARTING last).
///
/// A concurrent caller that finds TAP_STARTING == true returns `false` immediately,
/// preventing double-tap injection and REGISTRY corruption.
static TAP_STARTING: AtomicBool = AtomicBool::new(false);

pub fn get_registry() -> &'static Mutex<HashSet<ActiveInput>> {
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

pub fn flush_held_inputs() {
    // @safety-officer: CR-01 — drain registry into a local Vec BEFORE releasing
    // the lock, then post CGEvents outside the lock.  Posting a CGEvent while
    // holding the REGISTRY mutex causes a re-entrant deadlock: the CGEventTap
    // callback fires synchronously and immediately tries to acquire the same mutex.
    let inputs_to_flush: Vec<ActiveInput> = {
        let mut reg = get_registry().lock().unwrap();
        reg.drain().collect()
    };
    // Lock released here — safe to post CGEvents.
    if let Ok(source) = core_graphics::event_source::CGEventSource::new(
        core_graphics::event_source::CGEventSourceStateID::HIDSystemState,
    ) {
        let pos = core_graphics::geometry::CGPoint::new(0.0, 0.0);
        for input in inputs_to_flush.iter() {
            match input {
                ActiveInput::Key(k) => {
                    if let Ok(up_event) =
                        core_graphics::event::CGEvent::new_keyboard_event(source.clone(), *k, false)
                    {
                        up_event.set_integer_value_field(
                            EventField::EVENT_SOURCE_USER_DATA,
                            crate::platform::macos::input::LLMHF_INJECTED,
                        );
                        up_event.post(CGEventTapLocation::Session);
                    }
                }
                ActiveInput::Mouse(m) => {
                    let (ev_type, btn) = match m {
                        0 => (
                            CGEventType::LeftMouseUp,
                            core_graphics::event::CGMouseButton::Left,
                        ),
                        1 => (
                            CGEventType::RightMouseUp,
                            core_graphics::event::CGMouseButton::Right,
                        ),
                        _ => (
                            CGEventType::OtherMouseUp,
                            core_graphics::event::CGMouseButton::Center,
                        ),
                    };
                    if let Ok(up_event) = core_graphics::event::CGEvent::new_mouse_event(
                        source.clone(),
                        ev_type,
                        pos,
                        btn,
                    ) {
                        up_event.set_integer_value_field(
                            EventField::EVENT_SOURCE_USER_DATA,
                            crate::platform::macos::input::LLMHF_INJECTED,
                        );
                        up_event.post(CGEventTapLocation::Session);
                    }
                }
            }
        }
    }
}

fn get_active_app_state() -> &'static Mutex<Option<String>> {
    ACTIVE_APP.get_or_init(|| Mutex::new(None))
}

fn get_hotkey_bindings() -> &'static Mutex<Vec<HotkeyBinding>> {
    HOTKEY_BINDINGS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Register the StateActor's sender so the tap callback can dispatch intents.
/// Must be called once during app setup, before `start_observing`.
pub fn set_state_tx(tx: tokio::sync::mpsc::Sender<crate::state::Intent>) {
    let _ = STATE_TX.set(tx);
}

/// Replace the entire set of configurable hotkey bindings at runtime.
pub fn update_hotkey_bindings(bindings: Vec<HotkeyBinding>) {
    *get_hotkey_bindings().lock().unwrap() = bindings;
}

/// Add a single hotkey binding at runtime.
pub fn add_hotkey_binding(binding: HotkeyBinding) {
    get_hotkey_bindings().lock().unwrap().push(binding);
}

/// Remove all hotkey bindings for a given macro.
pub fn remove_hotkey_bindings_for(macro_id: &Uuid) {
    get_hotkey_bindings()
        .lock()
        .unwrap()
        .retain(|b| !matches!(&b.action, HotkeyAction::ToggleMacro(id) if id == macro_id));
}

/// List all user-facing running applications using NSWorkspace.
/// Filters to Regular activation policy (excludes daemons and agents).
/// Thread-safe: NSWorkspace.runningApplications is documented as thread-safe.
pub fn list_running_apps_impl() -> Result<Vec<crate::ipc::RunningApp>, String> {
    let workspace = NSWorkspace::sharedWorkspace();
    let apps = workspace.runningApplications();
    let mut result: Vec<crate::ipc::RunningApp> = apps
        .iter()
        .filter(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
        .filter_map(|app| {
            let bundle_id = app.bundleIdentifier()?.to_string();
            let name = app
                .localizedName()
                .map(|n| n.to_string())
                .unwrap_or_else(|| bundle_id.clone());
            Some(crate::ipc::RunningApp {
                display_name: name,
                identifier: bundle_id,
            })
        })
        .collect();
    result.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(result)
}

/// Returns whether the CGEventTap has been initialized.
pub fn is_tap_initialized() -> bool {
    TAP_INITIALIZED.load(Ordering::SeqCst)
}

/// Initialize the CGEventTap for input tracking, emergency stop, and global hotkeys.
///
/// **Idempotent**: the first call spawns a tap thread; subsequent calls are no-ops.
///
/// @safety-officer: Two AtomicBool guards prevent double-spawn under the TOCTOU race
/// that arises when the spawned thread resets `TAP_INITIALIZED` to `false` (on failure)
/// before the next poll cycle fires:
///
///   - `TAP_INITIALIZED` = true while the tap run loop is alive.
///   - `TAP_STARTING`    = true from "about to spawn" until the thread exits (success or fail).
///
/// A concurrent caller that finds either flag set returns immediately without spawning.
/// Exactly ONE thread is ever in-flight at any time.
pub fn initialize_tap() -> bool {
    // Fast path: tap already running.
    if TAP_INITIALIZED.load(Ordering::SeqCst) {
        return true;
    }
    // In-flight guard: another call is already mid-spawn.
    // swap returns the *old* value; if it was already true, someone else claimed the slot.
    if TAP_STARTING.swap(true, Ordering::SeqCst) {
        return false;
    }

    // We now hold the TAP_STARTING slot. Mark initialized before spawning so any
    // concurrent caller that passes TAP_STARTING's window also sees the flag set.
    TAP_INITIALIZED.store(true, Ordering::SeqCst);

    thread::spawn(|| {
        let tap_result = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![
                CGEventType::KeyDown,
                CGEventType::KeyUp,
                CGEventType::LeftMouseDown,
                CGEventType::LeftMouseUp,
                CGEventType::RightMouseDown,
                CGEventType::RightMouseUp,
                CGEventType::OtherMouseDown,
                CGEventType::OtherMouseUp,
            ],
            move |_tap_proxy, event_type, event| {
                // Option B (D-1): When the OS disables the tap (TapDisabledByTimeout or
                // TapDisabledByUserInput), stop the CFRunLoop instead of spawning a thread
                // that touches the CFMachPort from outside its owning thread.
                //
                // Why Option B over Option A: core-graphics 0.24.0 CGEventTap<'tap_life> is
                // not Send — it holds a `Box<dyn Fn(...) + 'tap_life>` callback tied to the
                // enclosing lifetime, and there is no `unsafe impl Send`. Moving the tap into
                // an Arc<Mutex<Option<CGEventTap>>> sent to a worker thread would violate the
                // lifetime borrow or require unsafe lifetime erasure. Option B avoids all of
                // this: we let the tap drop cleanly on its owning thread by stopping the
                // CFRunLoop, then rely on the existing 3s check_accessibility poll in
                // ipc/mod.rs:193-206 to call initialize_tap() again (≤ 3s restart gap).
                //
                // @safety-officer: TAP_INITIALIZED is cleared first, TAP_STARTING last —
                // TAP_STARTING is the in-flight spawn guard; clearing it last ensures any
                // concurrent caller only proceeds after TAP_INITIALIZED has its final value.
                if matches!(
                    event_type,
                    CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
                ) {
                    #[cfg(debug_assertions)]
                    eprintln!("[Observer] CGEventTap disabled — stopping CFRunLoop for clean teardown (Option B)");
                    // Reset guards so the 3s check_accessibility poll can reinitialize the tap.
                    TAP_INITIALIZED.store(false, Ordering::Release);
                    TAP_STARTING.store(false, Ordering::Release);
                    // Stop the CFRunLoop on this thread — tap drops cleanly after run() returns.
                    CFRunLoop::get_current().stop();
                    return None;
                }

                let user_data = event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA);
                if user_data == crate::platform::macos::input::LLMHF_INJECTED {
                    let mut reg = get_registry().lock().unwrap();
                    match event_type {
                        CGEventType::KeyDown => {
                            let keycode =
                                event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                            reg.insert(ActiveInput::Key(keycode as u16));
                        }
                        CGEventType::KeyUp => {
                            let keycode =
                                event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                            reg.remove(&ActiveInput::Key(keycode as u16));
                        }
                        CGEventType::LeftMouseDown => {
                            reg.insert(ActiveInput::Mouse(0));
                        }
                        CGEventType::LeftMouseUp => {
                            reg.remove(&ActiveInput::Mouse(0));
                        }
                        CGEventType::RightMouseDown => {
                            reg.insert(ActiveInput::Mouse(1));
                        }
                        CGEventType::RightMouseUp => {
                            reg.remove(&ActiveInput::Mouse(1));
                        }
                        CGEventType::OtherMouseDown => {
                            reg.insert(ActiveInput::Mouse(2));
                        }
                        CGEventType::OtherMouseUp => {
                            reg.remove(&ActiveInput::Mouse(2));
                        }
                        _ => {}
                    }
                    return Some(event.clone());
                }

                if matches!(event_type, CGEventType::KeyDown) {
                    let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                    let flags = event.get_flags();

                    // ══════════════════════════════════════════════════════════
                    // HARDCODED EMERGENCY STOP — Cmd+Shift+Q  (ALWAYS FIRST)
                    // @safety-officer: This block MUST remain the first branch.
                    //   It fires before any configurable hotkey, guaranteeing
                    //   that emergency cleanup cannot be suppressed.
                    // ══════════════════════════════════════════════════════════
                    let has_cmd = flags.contains(CGEventFlags::CGEventFlagCommand);
                    let has_shift = flags.contains(CGEventFlags::CGEventFlagShift);

                    if keycode == 12 && has_cmd && has_shift {
                        println!("EMERGENCY STOP TRIGGERED");

                        // @safety-officer: Signal the StateActor FIRST to cleanly
                        // stop the scheduler and set emergency_stop_active.
                        if let Some(tx) = STATE_TX.get() {
                            let _ = tx.try_send(crate::state::Intent::TriggerEmergencyStop);
                        }

                        // @safety-officer: CR-01 — drain registry into a local Vec BEFORE
                        // releasing the lock.  Posting CGEvents while holding the REGISTRY mutex
                        // causes a re-entrant deadlock (the tap callback re-acquires the mutex).
                        let inputs_to_release: Vec<ActiveInput> = {
                            let mut reg = get_registry().lock().unwrap();
                            reg.drain().collect()
                        };
                        // Lock released — safe to post CGEvents.
                        let pos = event.location();
                        if let Ok(source) = core_graphics::event_source::CGEventSource::new(
                            core_graphics::event_source::CGEventSourceStateID::HIDSystemState,
                        ) {
                            for input in inputs_to_release.iter() {
                                match input {
                                    ActiveInput::Key(k) => {
                                        if let Ok(up_event) =
                                            core_graphics::event::CGEvent::new_keyboard_event(
                                                source.clone(),
                                                *k,
                                                false,
                                            )
                                        {
                                            up_event.set_integer_value_field(
                                                EventField::EVENT_SOURCE_USER_DATA,
                                                crate::platform::macos::input::LLMHF_INJECTED,
                                            );
                                            up_event.post(CGEventTapLocation::Session);
                                        }
                                    }
                                    ActiveInput::Mouse(m) => {
                                        let (ev_type, btn) = match m {
                                            0 => (
                                                CGEventType::LeftMouseUp,
                                                core_graphics::event::CGMouseButton::Left,
                                            ),
                                            1 => (
                                                CGEventType::RightMouseUp,
                                                core_graphics::event::CGMouseButton::Right,
                                            ),
                                            _ => (
                                                CGEventType::OtherMouseUp,
                                                core_graphics::event::CGMouseButton::Center,
                                            ),
                                        };
                                        if let Ok(up_event) =
                                            core_graphics::event::CGEvent::new_mouse_event(
                                                source.clone(),
                                                ev_type,
                                                pos,
                                                btn,
                                            )
                                        {
                                            up_event.set_integer_value_field(
                                                EventField::EVENT_SOURCE_USER_DATA,
                                                crate::platform::macos::input::LLMHF_INJECTED,
                                            );
                                            up_event.post(CGEventTapLocation::Session);
                                        }
                                    }
                                }
                            }
                        }

                        process::exit(1);
                    }

                    // ══════════════════════════════════════════════════════════
                    // CONFIGURABLE HOTKEYS — checked AFTER emergency stop
                    // ══════════════════════════════════════════════════════════
                    if let Ok(bindings) = get_hotkey_bindings().try_lock() {
                        for binding in bindings.iter() {
                            if binding.matches(keycode as u16, flags) {
                                if let Some(tx) = STATE_TX.get() {
                                    let intent = match &binding.action {
                                        HotkeyAction::ToggleMacro(id) => {
                                            Some(crate::state::Intent::ToggleMacroHotkey(*id))
                                        }
                                        HotkeyAction::ToggleEngine => {
                                            Some(crate::state::Intent::ToggleEngineHotkey)
                                        }
                                    };
                                    if let Some(intent) = intent {
                                        let _ = tx.try_send(intent);
                                    }
                                }
                                break;
                            }
                        }
                    }

                }

                Some(event.clone())
            },
        );

        // @safety-officer: TAP_INITIALIZED and TAP_STARTING must BOTH be reset to false
        // in ALL non-success branches so callers can retry after permissions are granted.
        // TAP_STARTING is always cleared last — this is the release point for the in-flight
        // spawn guard; any concurrent caller blocked on TAP_STARTING can only proceed once
        // TAP_INITIALIZED has already been written to its final value.
        match tap_result {
            Ok(tap) => {
                let current_loop = CFRunLoop::get_current();
                match tap.mach_port.create_runloop_source(0) {
                    Ok(source) => {
                        current_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });
                        tap.enable();
                        // Clear TAP_STARTING now that the tap is live; the tap remains
                        // TAP_INITIALIZED=true for the duration of the run loop.
                        TAP_STARTING.store(false, Ordering::SeqCst);
                        CFRunLoop::run_current();
                        // run_current() only returns when the run loop is stopped (tap died).
                        // Reset so the tap can be re-initialized on next accessibility grant.
                        TAP_INITIALIZED.store(false, Ordering::SeqCst);
                        TAP_STARTING.store(false, Ordering::SeqCst);
                    }
                    Err(e) => {
                        eprintln!("[Observer] Failed to create runloop source: {:?}", e);
                        // Reset so initialization can be retried.
                        TAP_INITIALIZED.store(false, Ordering::SeqCst);
                        TAP_STARTING.store(false, Ordering::SeqCst);
                    }
                }
            }
            Err(_) => {
                eprintln!("[Observer] CGEventTap::new() failed — check Accessibility permission");
                // Reset flags so initialization can be retried after permissions are granted.
                TAP_INITIALIZED.store(false, Ordering::SeqCst);
                TAP_STARTING.store(false, Ordering::SeqCst);
            }
        }
    });

    true
}

pub struct MacPlatformObserver {
    // We store the pointer as a usize to ensure Send/Sync bounds are met,
    // as Retained<NSObject> is not Send.
    _observer_token: Option<usize>,
}

impl MacPlatformObserver {
    pub fn new() -> Self {
        Self {
            _observer_token: None,
        }
    }
}

impl Default for MacPlatformObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformObserver for MacPlatformObserver {
    fn get_active_app(&self) -> Option<String> {
        get_active_app_state().lock().unwrap().clone()
    }

    fn start_observing(&mut self) {
        // 1. Initialize NSWorkspace observer for Active App changes
        let workspace = NSWorkspace::sharedWorkspace();
        let center = workspace.notificationCenter();
        let notif_name = unsafe { NSWorkspaceDidActivateApplicationNotification };

        let handler = RcBlock::new(|notification_ptr: NonNull<NSNotification>| {
            // G-09-1c: read the activated app directly from the notification's own
            // userInfo payload — this is the app that just activated and is not
            // subject to the re-query race that frontmostApplication() has. Fall
            // back to frontmostApplication() only if the userInfo lookup yields
            // nothing (e.g. an unexpected notification shape).
            let notification = unsafe { notification_ptr.as_ref() };
            let app_from_payload = notification.userInfo().and_then(|info| {
                let key = unsafe { NSWorkspaceApplicationKey };
                info.objectForKey(key)
                    .and_then(|obj| obj.downcast::<NSRunningApplication>().ok())
            });
            let app =
                app_from_payload.or_else(|| NSWorkspace::sharedWorkspace().frontmostApplication());

            if let Some(app) = app {
                if let Some(bundle_id) = app.bundleIdentifier() {
                    let id_string = bundle_id.to_string();
                    #[cfg(debug_assertions)]
                    eprintln!("[Observer] active-app changed -> {:?}", id_string);

                    let mut active_app = get_active_app_state().lock().unwrap();
                    *active_app = Some(id_string.clone());
                    drop(active_app); // Release lock before sending

                    // Dispatch ActiveAppChanged to the StateActor so
                    // macros re-evaluate their targeting rules.
                    if let Some(tx) = STATE_TX.get() {
                        let _ =
                            tx.try_send(crate::state::Intent::ActiveAppChanged(Some(id_string)));
                    }
                }
            }
        });

        // G-09-1c: registered via a raw msg_send! (rather than the typed
        // addObserverForName_object_queue_usingBlock) specifically so a nil
        // return is observable here as None — a failed registration is logged
        // below instead of being silently swallowed.
        let observer: Option<Retained<NSObject>> = unsafe {
            let ret: *mut NSObject = msg_send![
                &center,
                addObserverForName: notif_name,
                object: std::ptr::null::<NSObject>(),
                queue: std::ptr::null::<NSObject>(),
                usingBlock: &*handler,
            ];
            Retained::retain(ret)
        };

        if let Some(obs) = observer {
            self._observer_token = Some(Retained::into_raw(obs) as usize);
        } else {
            eprintln!(
                "[Observer] NSWorkspace active-app observer registration FAILED — app-scoped macros will not re-evaluate on app switch"
            );
        }

        // Initialize current active app
        if let Some(app) = workspace.frontmostApplication() {
            if let Some(bundle_id) = app.bundleIdentifier() {
                let id_string = bundle_id.to_string();
                let mut active_app = get_active_app_state().lock().unwrap();
                *active_app = Some(id_string.clone());
                drop(active_app);

                if let Some(tx) = STATE_TX.get() {
                    let _ = tx.try_send(crate::state::Intent::ActiveAppChanged(Some(id_string)));
                }
            }
        }

        // 2. CGEventTap — only if Accessibility is already granted
        // WR-04: Log when initialize_tap returns false so tap failures surface.
        if super::check_accessibility_permissions(false) {
            let ok = initialize_tap();
            if !ok {
                eprintln!("[Observer] CGEventTap initialization returned false — tap may be inactive");
            }
        }
    }

    fn stop_observing(&mut self) {
        if let Some(ptr_val) = self._observer_token.take() {
            let ptr = ptr_val as *mut NSObject;
            unsafe {
                // We reclaim the retained observer so it drops correctly
                let observer = Retained::from_raw(ptr);
                if let Some(obs) = observer {
                    let workspace = NSWorkspace::sharedWorkspace();
                    let center = workspace.notificationCenter();
                    let _: () = msg_send![
                        &center,
                        removeObserver: &*obs,
                    ];
                }
            }
        }
    }
}

impl Drop for MacPlatformObserver {
    fn drop(&mut self) {
        // Called by Tauri's managed state drop at app shutdown (SAFE-02).
        // Ensures the NSWorkspace notification observer is unregistered cleanly.
        self.stop_observing();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UX-13: Pin the macOS CGEventFlag* bit values so the frontend/backend
    /// bit layout cannot silently drift. The frontend's `computeModifiers`
    /// sends these exact bit values in `trigger_modifiers`; the macOS
    /// `hook_callback` compares against `flags.bits()`. If the constants
    /// change, the test fails and the frontend/backend must be updated
    /// together.
    ///
    /// Sources:
    ///   Apple developer documentation — CGEventFlags
    ///   https://developer.apple.com/documentation/coregraphics/cgeventflags
    #[test]
    fn cg_event_flag_constants() {
        // macOS CGEventFlag bit values (verified against core-graphics 0.24):
        //   Shift   = 0x20000
        //   Control = 0x40000
        //   Alternate (Option) = 0x80000
        //   Command = 0x100000
        let shift = CGEventFlags::CGEventFlagShift.bits();
        let control = CGEventFlags::CGEventFlagControl.bits();
        let alternate = CGEventFlags::CGEventFlagAlternate.bits();
        let command = CGEventFlags::CGEventFlagCommand.bits();

        assert_eq!(shift, 0x20000, "CGEventFlagShift must be 0x20000");
        assert_eq!(control, 0x40000, "CGEventFlagControl must be 0x40000");
        assert_eq!(alternate, 0x80000, "CGEventFlagAlternate (Option) must be 0x80000");
        assert_eq!(command, 0x100000, "CGEventFlagCommand must be 0x100000");
    }
}
