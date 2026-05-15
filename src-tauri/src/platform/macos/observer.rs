use crate::platform::PlatformObserver;
use core_graphics::event::{CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, CGEventFlags, EventField};
use core_foundation::runloop::CFRunLoop;
use core_foundation_sys::runloop::kCFRunLoopCommonModes;
use std::thread;
use std::process;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashSet;
use std::ptr::NonNull;

use objc2::msg_send;
use objc2::rc::Retained;
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidActivateApplicationNotification};
use objc2_foundation::{NSNotification, NSObject};
use block2::RcBlock;
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

fn get_registry() -> &'static Mutex<HashSet<ActiveInput>> {
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
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
    get_hotkey_bindings().lock().unwrap().retain(|b| {
        !matches!(&b.action, HotkeyAction::ToggleMacro(id) if id == macro_id)
    });
}

/// Returns whether the CGEventTap has been initialized.
pub fn is_tap_initialized() -> bool {
    TAP_INITIALIZED.load(Ordering::SeqCst)
}

/// Initialize the CGEventTap for input tracking, emergency stop, and global hotkeys.
///
/// **Idempotent**: the first call spawns a tap thread; subsequent calls are no-ops.
///
/// @safety-officer: `compare_exchange(false, true, SeqCst, SeqCst)` guarantees:
///   - Exactly ONE thread is ever spawned, even under concurrent calls.
///   - No thread leak, no duplicate registry, no duplicate tap.
///   - Reverting `TAP_INITIALIZED` is intentionally impossible—once active,
///     the tap lives until process termination.
pub fn initialize_tap() -> bool {
    // Atomic CAS: only the thread that flips false→true proceeds.
    if TAP_INITIALIZED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // Already initialized — success (idempotent).
        return true;
    }

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
                CGEventType::MouseMoved,
                CGEventType::LeftMouseDragged,
                CGEventType::RightMouseDragged,
                CGEventType::OtherMouseDragged,
                CGEventType::ScrollWheel,
            ],
            |_proxy, event_type, event| {
                let user_data = event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA);
                if user_data == crate::platform::macos::input::LLMHF_INJECTED {
                    let mut reg = get_registry().lock().unwrap();
                    match event_type {
                        CGEventType::KeyDown => {
                            let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                            reg.insert(ActiveInput::Key(keycode as u16));
                        }
                        CGEventType::KeyUp => {
                            let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
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

                        let reg = get_registry().lock().unwrap();
                        let pos = event.location();
                        if let Ok(source) = core_graphics::event_source::CGEventSource::new(
                            core_graphics::event_source::CGEventSourceStateID::HIDSystemState,
                        ) {
                            for input in reg.iter() {
                                match input {
                                    ActiveInput::Key(k) => {
                                        if let Ok(up_event) = core_graphics::event::CGEvent::new_keyboard_event(source.clone(), *k, false) {
                                            up_event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, crate::platform::macos::input::LLMHF_INJECTED);
                                            up_event.post(CGEventTapLocation::HID);
                                        }
                                    }
                                    ActiveInput::Mouse(m) => {
                                        let (ev_type, btn) = match m {
                                            0 => (CGEventType::LeftMouseUp, core_graphics::event::CGMouseButton::Left),
                                            1 => (CGEventType::RightMouseUp, core_graphics::event::CGMouseButton::Right),
                                            _ => (CGEventType::OtherMouseUp, core_graphics::event::CGMouseButton::Center),
                                        };
                                        if let Ok(up_event) = core_graphics::event::CGEvent::new_mouse_event(source.clone(), ev_type, pos, btn) {
                                            up_event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, crate::platform::macos::input::LLMHF_INJECTED);
                                            up_event.post(CGEventTapLocation::HID);
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

        match tap_result {
            Ok(tap) => {
                let current_loop = CFRunLoop::get_current();
                if let Ok(source) = tap.mach_port.create_runloop_source(0) {
                    current_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });
                    tap.enable();
                    CFRunLoop::run_current();
                }
            }
            Err(_) => {
                eprintln!("Failed to create CGEventTap. Make sure the app has Accessibility permissions.");
                // Reset flag so initialization can be retried after permissions are granted
                TAP_INITIALIZED.store(false, Ordering::SeqCst);
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

impl PlatformObserver for MacPlatformObserver {
    fn get_active_app(&self) -> Option<String> {
        get_active_app_state().lock().unwrap().clone()
    }

    fn start_observing(&mut self) {
        // 1. Initialize NSWorkspace observer for Active App changes
        let workspace = NSWorkspace::sharedWorkspace();
        let center = workspace.notificationCenter();
        let notif_name = unsafe { NSWorkspaceDidActivateApplicationNotification };
        
        let handler = RcBlock::new(|_notification: NonNull<NSNotification>| {
            let ws = NSWorkspace::sharedWorkspace();
            if let Some(app) = ws.frontmostApplication() {
                if let Some(bundle_id) = app.bundleIdentifier() {
                    let id_string = bundle_id.to_string();
                    let mut active_app = get_active_app_state().lock().unwrap();
                    *active_app = Some(id_string.clone());
                    drop(active_app); // Release lock before sending

                    // Dispatch ActiveAppChanged to the StateActor so
                    // macros re-evaluate their targeting rules.
                    if let Some(tx) = STATE_TX.get() {
                        let _ = tx.try_send(crate::state::Intent::ActiveAppChanged(Some(id_string)));
                    }
                }
            }
        });

        let observer: Option<Retained<NSObject>> = unsafe {
            // Using a normal msg_send! and transmuting back to Retained as per modern objc2 
            // but we can just use msg_send! that returns a pointer if msg_send_id! is deprecated.
            // Actually msg_send! might return Retained directly in newer objc2 if the selector matches init/new/copy/mutableCopy,
            // but for addObserverForName we should use msg_send! and it returns *mut NSObject.
            // Since observer_test4 compiled with msg_send!, let's use it properly.
            // Wait, msg_send! returns Retained in 0.4+ if we use msg_send_id!.
            // In 0.5/0.6 it's just msg_send!.
            // Let's use msg_send! directly and type hint it to return Option<Retained<NSObject>>
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
        }

        // Initialize current active app
        if let Some(app) = workspace.frontmostApplication() {
            if let Some(bundle_id) = app.bundleIdentifier() {
                let mut active_app = get_active_app_state().lock().unwrap();
                *active_app = Some(bundle_id.to_string());
            }
        }

        // 2. CGEventTap — only if Accessibility is already granted
        if super::check_accessibility_permissions(false) {
            initialize_tap();
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
