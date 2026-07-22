use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::persistence::{ProfileData, ProfileManager};
use crate::platform::InputProvider;

// ── MouseButton mapping ─────────────────────────────────────────
// state::MouseButton (serializable) → platform::MouseButton (injection)
impl From<MouseButton> for crate::platform::MouseButton {
    fn from(btn: MouseButton) -> crate::platform::MouseButton {
        match btn {
            MouseButton::Left => crate::platform::MouseButton::Left,
            MouseButton::Right => crate::platform::MouseButton::Right,
            MouseButton::Middle => crate::platform::MouseButton::Center,
        }
    }
}

// ── Input Event Model ────────────────────────────────────────────
// Serializable input events that can be saved to config files.
// The InputProvider trait is responsible for translating these into
// platform-specific CGEvents (macOS) or SendInput (Windows).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// A serializable input event. Platform-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputEvent {
    /// A mouse button action (click or hold).
    MouseButton(MouseButton),
    /// A keyboard key, identified by virtual keycode.
    /// macOS: CGKeyCode values (e.g., 0=A, 12=Q, 49=Space)
    /// Windows: VK_ codes (mapped at the platform layer)
    Key(u16),
}

/// UX-12: A group of macros that share the same input. Surfaced in the UI
/// as a non-blocking warning ("{A} and {B} both inject Left Click — clicks
/// will fire at 2× rate.").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConflict {
    /// The set of macros sharing this input. Order is non-deterministic
    /// (HashMap iteration order) — frontend sorts by macro name for display.
    pub macros: Vec<Uuid>,
    /// The shared input event. Uses `InputEvent` (Hash + Eq) so the conflict
    /// can be detected by `HashMap<InputEvent, Vec<Uuid>>` accumulation.
    pub input: InputEvent,
}

// ── Action Sequence Model ────────────────────────────────────────
// Supports simultaneous "Sustained Holds" and "Interleaved Intervals"
// for complex automation like Minecraft AFK farms.

/// A single step within an action sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionStep {
    /// Hold a key/button for the entire macro lifetime.
    /// - Sends KeyDown/MouseDown when the macro starts.
    /// - Sends KeyUp/MouseUp when the macro stops.
    /// - Zero CPU overhead (no timer needed).
    SustainedHold { input: InputEvent },
    /// Repeat an input event at a fixed interval.
    /// - Each step gets its own independent timer in the scheduler.
    /// - Fires the event every `interval_ms` milliseconds.
    InterleavedInterval { input: InputEvent, interval_ms: u64 },
}

/// A collection of action steps that execute simultaneously.
/// Example: Hold Right-Click + Left-Click every 650ms (AFK Fish Farm).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionSequence {
    pub steps: Vec<ActionStep>,
}

// ── Macro Configuration ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TriggerMode {
    #[default]
    Pulse,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroConfig {
    pub id: Uuid,
    pub name: String,
    /// Legacy: simple interval for backward compatibility.
    /// Used only when `sequence` is empty (single-click macro).
    pub interval_ms: u64,
    pub enabled: bool,
    /// The target app bundle ID, or None for "Global"
    pub target_app: Option<String>,
    /// Multi-track action sequence. If empty, falls back to
    /// a simple left-click at `interval_ms`.
    #[serde(default)]
    pub sequence: ActionSequence,
    /// The keycode that toggles this macro. If None, it relies on global toggle.
    #[serde(default)]
    pub trigger_key: Option<u16>,
    /// Raw modifier bits for the trigger key. Platform-specific:
    ///   macOS: CGEventFlags bits (Shift=0x20000, Control=0x40000,
    ///          Option=0x80000, Command=0x100000)
    ///   Windows: MOD_* values (MOD_ALT=0x1, MOD_CONTROL=0x2,
    ///            MOD_SHIFT=0x4, MOD_WIN=0x8)
    #[serde(default)]
    pub trigger_modifiers: u64,
    /// The behavior mode of this macro when triggered.
    #[serde(default)]
    pub trigger_mode: TriggerMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub macros: HashMap<Uuid, MacroConfig>,
    pub emergency_stop_active: bool,
    pub active_app: Option<String>,
    pub engine_active: bool,
    /// Suppresses auto-save during profile load batch (RELY-04, D-06).
    /// Not serialized to frontend — internal StateActor flag only.
    #[serde(skip)]
    pub loading_profile: bool,
    /// UX-12: derived list of input-event conflicts between currently
    /// enabled macros. Recomputed by `recompute_conflicts()` after every
    /// state-mutating intent. Sent to the frontend via the `state-changed`
    /// event; not persisted in profile JSON (#[serde(default)]).
    #[serde(default)]
    pub conflicts: Vec<InputConflict>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            macros: HashMap::new(),
            emergency_stop_active: false,
            active_app: None,
            engine_active: true,
            loading_profile: false,
            conflicts: Vec::new(),
        }
    }
}

pub enum Intent {
    /// WR-05: The Uuid is generated by the backend (StateActor), not the frontend.
    /// The oneshot sender carries the assigned Uuid back to the IPC caller.
    AddMacro(MacroConfig, tokio::sync::oneshot::Sender<Uuid>),
    RemoveMacro(Uuid),
    SetMacroEnabled(Uuid, bool),
    SetMacroTargetApp(Uuid, Option<String>),
    SetMacroTriggerKey(Uuid, Option<u16>, Option<u64>),
    /// UX-11: Bind a trigger key for an existing macro. The oneshot sender
    /// carries the conflict result back to the IPC caller — `Ok(())` if the
    /// slot was free or this macro already owned it, `Err(msg)` otherwise.
    /// The `u64` is the modifier bitmask in platform-native format
    /// (CGEventFlags bits on macOS, MOD_* values on Windows).
    BindHotkey(Uuid, u16, u64, tokio::sync::oneshot::Sender<Result<(), String>>),
    /// UX-11: Remove a trigger key bind for a specific macro. Cannot fail —
    /// the absence of a bind is a no-op state. Mirrors `RemoveMacro(Uuid)`.
    UnbindHotkey(Uuid),
    TriggerEmergencyStop,
    ResetEmergencyStop,
    ActiveAppChanged(Option<String>),
    /// Sent from the CGEventTap hotkey handler — toggles a single macro.
    ToggleMacroHotkey(Uuid),
    /// Sent from the CGEventTap hotkey handler — toggles the entire engine.
    ToggleEngineHotkey,
    /// Update the action sequence for an existing macro.
    UpdateSequence(Uuid, ActionSequence),
    /// Live-update the interval for a specific step of a macro.
    UpdateStepInterval(Uuid, usize, u64),
    // Provide a way to reply with the current state if needed
    GetState(tokio::sync::oneshot::Sender<AppState>),
    /// Load a named profile, suppressing per-mutation auto-save during the batch and writing once at the end.
    /// CR-06: Carries a oneshot sender so the StateActor can return the loaded
    /// ProfileData to the IPC layer, eliminating the double-disk-read.
    LoadProfile(String, tokio::sync::oneshot::Sender<Result<crate::persistence::ProfileData, String>>),
}

pub struct StateActor {
    state: AppState,
    receiver: mpsc::Receiver<Intent>,
    scheduler_tx: mpsc::Sender<crate::scheduler::SchedulerIntent>,
    /// Two-Phase Dispatch: receives ActionReady from the scheduler.
    /// The StateActor validates targeting before performing input injection.
    action_rx: mpsc::Receiver<crate::scheduler::ActionReady>,
    app_handle: tauri::AppHandle,
    /// RELY-04: injected for auto-save to the default profile.
    profile_mgr: Arc<ProfileManager>,
    /// Platform-specific input provider for actual event injection.
    #[cfg(target_os = "macos")]
    input_provider: crate::platform::macos::MacInputProvider,
    #[cfg(target_os = "windows")]
    input_provider: crate::platform::windows::WindowsInputProvider,
}

/// UX-11: Return the existing macro_id that holds the given (keycode, modifiers)
/// pair, or `None` if the slot is free.
///
/// Self-rebind is allowed: if the existing holder is `self_id`, returns `None`
/// (same macro, same key+mods is treated as a no-op, not a conflict).
/// See RESEARCH.md D-4.
pub(crate) fn check_trigger_key_conflict(
    state: &AppState,
    self_id: Uuid,
    keycode: u16,
    modifiers: u64,
) -> Option<Uuid> {
    for (other_id, mac) in &state.macros {
        if other_id == &self_id {
            continue; // self — not a conflict
        }
        if mac.trigger_key == Some(keycode) && mac.trigger_modifiers == modifiers {
            return Some(*other_id);
        }
    }
    None
}

/// UX-12: Recompute `state.conflicts` from the current enabled macro set.
///
/// For each enabled macro, expand its sequence with the same legacy fallback
/// the scheduler uses (see `scheduler/mod.rs:257-270`), group macros by
/// `InputEvent`, and emit an `InputConflict` for any group with ≥2 macros.
///
/// O(n × steps) — typical config is <50 macros × <5 steps, negligible cost
/// (RESEARCH.md §3.2 / T-08-10). Pure mutator on `state.conflicts` only;
/// the caller is responsible for `reevaluate_all_macros` and `broadcast_state`.
pub(crate) fn recompute_conflicts(state: &mut AppState) {
    use std::collections::HashMap;
    let mut by_input: HashMap<InputEvent, Vec<Uuid>> = HashMap::new();

    for (id, mac) in &state.macros {
        if !mac.enabled {
            continue;
        }
        // Mirror the scheduler's legacy fallback exactly (scheduler/mod.rs:257-270):
        // empty sequence + Pulse → single left-click interval;
        // empty sequence + Hold  → sustained left-click hold.
        let inputs: Vec<InputEvent> = if mac.sequence.steps.is_empty() {
            vec![InputEvent::MouseButton(MouseButton::Left)]
        } else {
            mac.sequence
                .steps
                .iter()
                .map(|step| match step {
                    ActionStep::SustainedHold { input } => *input,
                    ActionStep::InterleavedInterval { input, .. } => *input,
                })
                .collect()
        };
        for input in inputs {
            by_input.entry(input).or_default().push(*id);
        }
    }

    state.conflicts = by_input
        .into_iter()
        .filter(|(_, ids)| ids.len() >= 2)
        .map(|(input, mut macros)| {
            macros.sort_unstable();
            InputConflict { input, macros }
        })
        .collect();
}

/// UX-11: Build the platform's `HotkeyBinding` Vec from the current macro
/// set. Mirrors the data path of `reevaluate_all_macros` for the per-binding
/// registry — the StateActor is the source of truth, the platform static is
/// a write-only mirror.
///
/// Only macros with a `trigger_key` are included (macros with no key don't
/// participate in the hotkey registry). The returned Vec replaces the
/// platform's `HOTKEY_BINDINGS` entirely on every bind/unbind (RESEARCH.md
/// R-4 — keeps the registry simple, replace is cheap for typical configs).
#[cfg(target_os = "macos")]
pub(crate) fn build_hotkey_bindings_vec(
    macros: &HashMap<Uuid, MacroConfig>,
) -> Vec<crate::platform::macos::observer::HotkeyBinding> {
    use crate::platform::macos::observer::{HotkeyAction, HotkeyBinding};
    macros
        .values()
        .filter_map(|mac| {
            mac.trigger_key.map(|key| HotkeyBinding {
                keycode: key,
                modifiers: mac.trigger_modifiers,
                action: HotkeyAction::ToggleMacro(mac.id),
            })
        })
        .collect()
}

#[cfg(target_os = "windows")]
pub(crate) fn build_hotkey_bindings_vec(
    macros: &HashMap<Uuid, MacroConfig>,
) -> Vec<crate::platform::windows::WindowsHotkeyBinding> {
    macros
        .values()
        .filter_map(|mac| {
            mac.trigger_key.map(|key| crate::platform::windows::WindowsHotkeyBinding {
                keycode: key,
                modifiers: mac.trigger_modifiers,
                macro_id: mac.id,
            })
        })
        .collect()
}

/// On non-{macos,windows} hosts the bind registry is a no-op — the host
/// is unsupported but the StateActor's BindHotkey handler must still compile
/// and return `Ok(())` so the IPC layer doesn't break. Returns an empty Vec
/// that the cfg-gated platform dispatch ignores.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn build_hotkey_bindings_vec<T>(_macros: &HashMap<Uuid, MacroConfig>) -> Vec<T> {
    Vec::new()
}

impl StateActor {
    pub fn new(
        receiver: mpsc::Receiver<Intent>,
        scheduler_tx: mpsc::Sender<crate::scheduler::SchedulerIntent>,
        action_rx: mpsc::Receiver<crate::scheduler::ActionReady>,
        app_handle: tauri::AppHandle,
        profile_mgr: Arc<ProfileManager>,
    ) -> Self {
        Self {
            state: AppState::default(),
            receiver,
            scheduler_tx,
            action_rx,
            app_handle,
            profile_mgr,
            #[cfg(target_os = "macos")]
            input_provider: crate::platform::macos::MacInputProvider::new(),
            #[cfg(target_os = "windows")]
            input_provider: crate::platform::windows::WindowsInputProvider::new(),
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                biased;

                // Priority 1: Intents from IPC / hotkeys / platform observers.
                Some(intent) = self.receiver.recv() => {
                    self.handle_intent(intent).await;
                    self.broadcast_state();
                }

                // Priority 2: Two-Phase Action Dispatch from the scheduler.
                Some(action) = self.action_rx.recv() => {
                    self.handle_action(action);
                }

                // Both channels closed — shutdown.
                else => break,
            }
        }
    }

    /// Two-Phase Dispatch: validate targeting rules, then inject.
    /// @safety-officer: This is the ONLY site where input injection occurs.
    /// The scheduler is never allowed to inject directly.
    fn handle_action(&self, action: crate::scheduler::ActionReady) {
        use crate::scheduler::ActionType;

        // Gate 1: Engine must be active
        if !self.state.engine_active || self.state.emergency_stop_active {
            return;
        }

        // Gate 2: Macro must exist and be enabled
        let mac = match self.state.macros.get(&action.macro_id) {
            Some(m) if m.enabled => m,
            _ => return,
        };

        // Gate 3: Target app must match (or be Global)
        let matches_target = match &mac.target_app {
            Some(target) => self.state.active_app.as_deref() == Some(target.as_str()),
            None => true, // "Global"
        };
        if !matches_target {
            return;
        }

        // ── All gates passed — dispatch input injection ──
        #[cfg(debug_assertions)]
        eprintln!("[Action] macro={} {:?}", mac.name, action.action_type);

        match &action.action_type {
            ActionType::Interval(input) => {
                // Click: press then immediately release.
                self.inject_input(input, true);
                self.inject_input(input, false);
            }
            ActionType::HoldStart(input) => {
                self.inject_input(input, true);
            }
            ActionType::HoldRelease(input) => {
                self.inject_input(input, false);
            }
        }
    }

    /// Translate a platform-agnostic InputEvent into a real OS event.
    fn inject_input(&self, input: &InputEvent, is_down: bool) {
        match input {
            InputEvent::Key(keycode) => {
                self.input_provider.inject_key(*keycode, is_down);
            }
            InputEvent::MouseButton(btn) => {
                // For mouse clicks, we use inject_mouse_click which sends
                // both down+up. For holds, we use inject_key-style approach.
                // The InputProvider trait currently bundles down+up in
                // inject_mouse_click, so for sustained holds we need
                // raw key/button handling. Use inject_key with a
                // virtual mouse-button keycode mapped at the platform layer.
                //
                // For now, we use a simple approach: inject_mouse_click
                // at the current cursor position for the full press+release
                // cycle (intervals), and for holds we track via the platform.
                let platform_btn: crate::platform::MouseButton = (*btn).into();
                if is_down {
                    // For sustained holds and the "down" half of intervals,
                    // we post a raw mouse-down event.
                    self.input_provider
                        .inject_mouse_button_raw(platform_btn, true);
                } else {
                    self.input_provider
                        .inject_mouse_button_raw(platform_btn, false);
                }
            }
        }
    }

    async fn handle_intent(&mut self, intent: Intent) {
        match intent {
            Intent::AddMacro(mut config, reply) => {
                // WR-05: Always generate the UUID on the backend — the client-supplied
                // id (if any) is ignored to prevent spoofing or UUID collisions.
                let new_id = Uuid::new_v4();
                config.id = new_id;
                // UX-11: Pre-check the trigger key against other enabled macros
                // before inserting. If the new macro's trigger_key collides with
                // an existing macro's (keycode, modifiers) pair, drop the trigger
                // (set to None) so the macro is still created but with no hotkey.
                // The full Result<(), String> error path is shipped in plan 08-03
                // (Intent::BindHotkey) — this drop-on-conflict is the interim
                // defense-in-depth (T-08-07 / T-08-12 in 08-02-PLAN.md).
                if let Some(key) = config.trigger_key {
                    if let Some(_conflicting_id) =
                        self.check_trigger_key_conflict(new_id, key, config.trigger_modifiers)
                    {
                        config.trigger_key = None;
                        config.trigger_modifiers = 0;
                    }
                }
                self.state.macros.insert(new_id, config);
                self.reevaluate_all_macros().await;
                // UX-12: refresh the derived `conflicts` field after the
                // new macro lands in `state.macros` and the platform statics
                // are re-pushed. Order matters: reevaluate first so any
                // scheduler / platform-side state is consistent, then
                // recompute the derived conflict graph.
                self.recompute_conflicts();
                self.auto_save_default().await;
                let _ = reply.send(new_id);
            }
            Intent::RemoveMacro(id) => {
                self.state.macros.remove(&id);
                let _ = self
                    .scheduler_tx
                    .send(crate::scheduler::SchedulerIntent::StopMacro(id))
                    .await;
                // UX-12: a removed macro may have been a member of one or
                // more conflict groups — recompute so the warning disappears
                // (or shrinks) on the next state-changed event.
                self.recompute_conflicts();
                self.auto_save_default().await;
            }
            Intent::SetMacroEnabled(id, enabled) => {
                if let Some(mac) = self.state.macros.get_mut(&id) {
                    mac.enabled = enabled;
                }
                self.reevaluate_all_macros().await;
                // UX-12: toggling enabled can add or remove the macro from
                // conflict groups.
                self.recompute_conflicts();
                self.auto_save_default().await;
            }
            Intent::SetMacroTargetApp(id, target) => {
                if let Some(mac) = self.state.macros.get_mut(&id) {
                    mac.target_app = target;
                }
                self.reevaluate_all_macros().await;
                // UX-12: target-app changes don't change the input graph
                // (the conflict set is computed on enabled macros regardless
                // of target), but we recompute for symmetry with the other
                // mutating intents — keeps the invariant "every state change
                // triggers a fresh recompute" simple to reason about.
                self.recompute_conflicts();
                self.auto_save_default().await;
            }
            Intent::SetMacroTriggerKey(id, trigger_key, trigger_modifiers) => {
                // UX-11: Same pre-check as AddMacro. If a DIFFERENT macro
                // already holds the requested (keycode, modifiers) pair, drop
                // the trigger (set to None / 0) on the existing macro instead
                // of applying the change. Self-rebind (same id) is allowed
                // and falls through to the apply branch — `check_trigger_key_conflict`
                // returns None for self_id matches, so the conflict branch
                // is not entered. See T-08-08 / T-08-12 in 08-02-PLAN.md.
                let mut new_key = trigger_key;
                let mut new_mods = trigger_modifiers.unwrap_or(0);
                if let Some(key) = new_key {
                    if let Some(_conflicting_id) =
                        self.check_trigger_key_conflict(id, key, new_mods)
                    {
                        new_key = None;
                        new_mods = 0;
                    }
                }
                if let Some(mac) = self.state.macros.get_mut(&id) {
                    mac.trigger_key = new_key;
                    mac.trigger_modifiers = new_mods;
                }
                self.reevaluate_all_macros().await;
                // UX-12: refresh derived `conflicts` after a successful
                // (or coerced) trigger-key update.
                self.recompute_conflicts();
                self.auto_save_default().await;
            }
            Intent::BindHotkey(macro_id, keycode, modifiers, reply) => {
                // UX-11: Conflict pre-check. If a DIFFERENT macro already holds
                // the (keycode, modifiers) pair, send `Err(msg)` via the
                // oneshot and DO NOT mutate state. The frontend surfaces the
                // error as a `ConflictErrorToast` (Plan 08-05). Self-rebind
                // (same macro, same key+mods) is allowed — falls through
                // because `check_trigger_key_conflict` returns None for the
                // self-id match.
                if let Some(conflicting_id) =
                    self.check_trigger_key_conflict(macro_id, keycode, modifiers)
                {
                    // Look up the conflicting macro's name for the error message.
                    // Fall back to the id as a hex string if the macro has
                    // somehow been removed between the check and the lookup
                    // (race window — defensive only).
                    let conflicting_name = self
                        .state
                        .macros
                        .get(&conflicting_id)
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| format!("{:?}", conflicting_id));
                    let msg = format!(
                        "Key (keycode {}) is already assigned to \"{}\". Unbind it first or pick a different key.",
                        keycode, conflicting_name
                    );
                    let _ = reply.send(Err(msg));
                    return;
                }
                // No conflict — apply the bind. If the macro doesn't exist
                // (race with RemoveMacro), the bind is a silent no-op;
                // the frontend gets Ok(()) and the macro simply doesn't get
                // a hotkey. This matches the "no error on no-op" semantics
                // the original `bind_hotkey` had on macOS pre-Phase-8.
                if let Some(mac) = self.state.macros.get_mut(&macro_id) {
                    mac.trigger_key = Some(keycode);
                    mac.trigger_modifiers = modifiers;
                }
                // Re-build the platform HOTKEY_BINDINGS registry from the
                // current macro set and replace it wholesale (RESEARCH.md
                // R-4). The platform statics are write-only mirrors of the
                // StateActor's view.
                let bindings = build_hotkey_bindings_vec(&self.state.macros);
                #[cfg(target_os = "macos")]
                {
                    crate::platform::macos::observer::update_hotkey_bindings(bindings);
                }
                #[cfg(target_os = "windows")]
                {
                    crate::platform::windows::update_hotkey_bindings(bindings);
                }
                self.reevaluate_all_macros().await;
                // UX-12: a fresh bind can introduce/remove input overlap
                // with other macros — refresh the derived conflict graph.
                self.recompute_conflicts();
                self.auto_save_default().await;
                let _ = reply.send(Ok(()));
            }
            Intent::UnbindHotkey(macro_id) => {
                // UX-11: Clear the trigger on the matching macro (if any),
                // then rebuild the platform HOTKEY_BINDINGS registry.
                // Cannot fail — absence of a bind is a no-op state.
                if let Some(mac) = self.state.macros.get_mut(&macro_id) {
                    mac.trigger_key = None;
                    mac.trigger_modifiers = 0;
                }
                let bindings = build_hotkey_bindings_vec(&self.state.macros);
                #[cfg(target_os = "macos")]
                {
                    crate::platform::macos::observer::update_hotkey_bindings(bindings);
                }
                #[cfg(target_os = "windows")]
                {
                    crate::platform::windows::update_hotkey_bindings(bindings);
                }
                self.reevaluate_all_macros().await;
                // UX-12: a removed bind can drop a macro from conflict
                // groups — refresh the derived field.
                self.recompute_conflicts();
                self.auto_save_default().await;
            }
            Intent::TriggerEmergencyStop => {
                self.state.emergency_stop_active = true;
                self.state.engine_active = false;
                for mac in self.state.macros.values_mut() {
                    mac.enabled = false;
                }
                let _ = self
                    .scheduler_tx
                    .send(crate::scheduler::SchedulerIntent::StopAll)
                    .await;
                self.input_provider.flush_held_inputs();
            }
            Intent::ResetEmergencyStop => {
                self.state.emergency_stop_active = false;
                self.state.engine_active = true;
                self.reevaluate_all_macros().await;
                // UX-12: after a reset, the user's enabled flags are restored
                // (per TriggerEmergencyStop, all macros were disabled). The
                // conflict field must reflect the new enabled/disabled state.
                self.recompute_conflicts();
            }
            Intent::ActiveAppChanged(app) => {
                self.state.active_app = app;
                self.reevaluate_all_macros().await;
                // No recompute_conflicts: the conflict graph is computed on
                // enabled macros regardless of target app. The enabled flags
                // are unchanged by an app change.
            }
            Intent::ToggleMacroHotkey(id) => {
                if let Some(mac) = self.state.macros.get_mut(&id) {
                    mac.enabled = !mac.enabled;
                }
                self.reevaluate_all_macros().await;
                // UX-12: a hotkey toggle flips enabled — same recompute
                // rationale as SetMacroEnabled.
                self.recompute_conflicts();
                // WR-01: Persist the toggled enabled state so it survives restart.
                self.auto_save_default().await;
            }
            Intent::ToggleEngineHotkey => {
                self.state.engine_active = !self.state.engine_active;
                if !self.state.engine_active {
                    let _ = self
                        .scheduler_tx
                        .send(crate::scheduler::SchedulerIntent::StopAll)
                        .await;
                } else {
                    self.reevaluate_all_macros().await;
                }
            }
            Intent::UpdateSequence(id, sequence) => {
                if let Some(mac) = self.state.macros.get_mut(&id) {
                    mac.sequence = sequence;
                }
                self.reevaluate_all_macros().await;
                // UX-12: changing the sequence can add/remove the macro's
                // input from conflict groups.
                self.recompute_conflicts();
                self.auto_save_default().await;
            }
            Intent::UpdateStepInterval(id, step_index, interval_ms) => {
                if let Some(mac) = self.state.macros.get_mut(&id) {
                    if let Some(ActionStep::InterleavedInterval {
                        interval_ms: ref mut ms,
                        ..
                    }) = mac.sequence.steps.get_mut(step_index)
                    {
                        *ms = interval_ms;
                    }
                }
                // Forward live update to scheduler.
                let _ = self
                    .scheduler_tx
                    .send(crate::scheduler::SchedulerIntent::UpdateInterval(
                        id,
                        step_index,
                        interval_ms,
                    ))
                    .await;
                self.auto_save_default().await;
            }
            Intent::GetState(reply) => {
                let _ = reply.send(self.state.clone());
            }
            Intent::LoadProfile(name, reply) => {
                // 1. Clear existing macros and stop all scheduler tasks
                self.state.macros.clear();
                let _ = self
                    .scheduler_tx
                    .send(crate::scheduler::SchedulerIntent::StopAll)
                    .await;
                // 2. Set suppression flag (D-06)
                self.state.loading_profile = true;
                // 3. Load profile and populate macros directly (no per-macro auto-save).
                // CR-06: Return the loaded ProfileData via the oneshot sender so the IPC
                // layer has the result without performing a second disk read.
                match self.profile_mgr.load_profile(&name).await {
                    Ok(profile) => {
                        for (_, config) in profile.macros.clone() {
                            self.state.macros.insert(config.id, config);
                        }
                        // Unconditional assignment — fully restore saved engine state.
                        // Do NOT use `if profile.engine_active { ... = true; }` —
                        // that is a one-way ratchet (can enable, never disable). See Pitfall 6.
                        self.state.engine_active = profile.engine_active;
                        self.reevaluate_all_macros().await;
                        // UX-12: a loaded profile may contain macros whose
                        // enabled flags create input overlap with the
                        // previously-loaded set (or with each other). The
                        // derived `conflicts` field must be recomputed so
                        // the warning surfaces on the next state-changed
                        // event. Do NOT add to the Err branch — state is
                        // unchanged on load failure and the previous
                        // conflicts value remains correct.
                        self.recompute_conflicts();
                        let _ = reply.send(Ok(profile));
                    }
                    Err(e) => {
                        use tauri::Emitter;
                        let _ = self.app_handle.emit("auto-save-error", e.to_string());
                        let _ = reply.send(Err(e));
                    }
                }
                // 4. Clear flag, then write once (D-06)
                self.state.loading_profile = false;
                self.auto_save_default().await;
                // WR-02: Do NOT call self.broadcast_state() here — the outer run() loop
                // already broadcasts state unconditionally after every handle_intent call.
                // Calling it here caused a double state-changed event on every profile load.
            }
        }
    }

    async fn reevaluate_all_macros(&self) {
        // CR-01 fix: HOTKEY_BINDINGS is the single source of truth for hotkey
        // dispatch (the richer, action-typed Phase-8 registry). Refresh it here,
        // unconditionally and BEFORE the engine-active early-return below, so it
        // stays current on every trigger_key mutation (AddMacro, SetMacroTriggerKey,
        // BindHotkey, UnbindHotkey, LoadProfile, RemoveMacro all call this method)
        // independent of engine on/off state. This is the sole registry-refresh
        // site for comprehensive coverage — the second, redundant registry that
        // used to be populated below is removed entirely.
        let bindings = build_hotkey_bindings_vec(&self.state.macros);
        #[cfg(target_os = "macos")]
        {
            crate::platform::macos::observer::update_hotkey_bindings(bindings);
        }
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::update_hotkey_bindings(bindings);
        }

        if self.state.emergency_stop_active || !self.state.engine_active {
            return;
        }

        for mac in self.state.macros.values() {
            let matches_target = match &mac.target_app {
                Some(target) => self.state.active_app.as_deref() == Some(target.as_str()),
                None => true, // "Global"
            };

            if mac.enabled && matches_target {
                let _ = self
                    .scheduler_tx
                    .send(crate::scheduler::SchedulerIntent::StartMacro(mac.clone()))
                    .await;
            } else {
                let _ = self
                    .scheduler_tx
                    .send(crate::scheduler::SchedulerIntent::StopMacro(mac.id))
                    .await;
            }
        }
    }

    /// UX-11 wrapper: delegates to the free function so intent handlers can
    /// call `self.check_trigger_key_conflict(...)` and the helper can be
    /// unit-tested without a Tauri AppHandle.
    fn check_trigger_key_conflict(
        &self,
        self_id: Uuid,
        keycode: u16,
        modifiers: u64,
    ) -> Option<Uuid> {
        check_trigger_key_conflict(&self.state, self_id, keycode, modifiers)
    }

    /// UX-12 wrapper: delegates to the free function. See `recompute_conflicts`
    /// docs for behavior.
    fn recompute_conflicts(&mut self) {
        recompute_conflicts(&mut self.state);
    }

    fn broadcast_state(&self) {
        use tauri::Emitter;
        let _ = self.app_handle.emit("state-changed", &self.state);
    }

    /// Persist the current macro set as the `default` profile.
    /// Suppressed during profile-load batches (D-06).
    /// On failure, emits an `auto-save-error` event to the frontend (D-07).
    async fn auto_save_default(&self) {
        if self.state.loading_profile {
            return;
        }
        let profile = ProfileData {
            name: "default".to_string(),
            macros: self.state.macros.clone(),
            engine_active: self.state.engine_active, // D-PITFALL-4: never hardcode
        };
        if let Err(e) = self.profile_mgr.save_profile(&profile).await {
            #[cfg(debug_assertions)]
            eprintln!("[Persistence] auto-save default failed: {}", e);
            // D-07: emit transient error event — same mechanism as broadcast_state
            use tauri::Emitter;
            let _ = self.app_handle.emit("auto-save-error", e.to_string());
        }
    }
}

#[derive(Clone)]
pub struct StateManager {
    sender: mpsc::Sender<Intent>,
}

impl StateManager {
    pub fn new(sender: mpsc::Sender<Intent>) -> Self {
        Self { sender }
    }

    pub async fn send_intent(&self, intent: Intent) -> Result<(), mpsc::error::SendError<Intent>> {
        self.sender.send(intent).await
    }
}

// ── Unit tests for conflict-detection helpers ────────────────────
//
// These tests exercise the UX-11 (`check_trigger_key_conflict`) and
// UX-12 (`recompute_conflicts`) helpers via the free functions in this
// module, matching the persistence test style (no `StateActor`
// construction required → no Tauri AppHandle / channel plumbing).

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a `MacroConfig` with the given trigger key+mods and an
    /// optional pre-populated `sequence`. Used by all four tests below.
    fn make_macro(id: Uuid, name: &str, trigger_key: Option<u16>, trigger_modifiers: u64, sequence: ActionSequence) -> MacroConfig {
        MacroConfig {
            id,
            name: name.to_string(),
            interval_ms: 100,
            enabled: false,
            target_app: None,
            sequence,
            trigger_key,
            trigger_modifiers,
            trigger_mode: TriggerMode::Pulse,
        }
    }

    /// UX-11: Rebinding the same macro to the same key+mods must succeed.
    /// (Self-rebind is allowed — same macro, same key+mods is a no-op, not
    /// a conflict. See RESEARCH.md D-4.)
    #[test]
    fn self_rebind_allowed() {
        let mut state = AppState::default();
        let id = Uuid::new_v4();
        state.macros.insert(
            id,
            make_macro(id, "self", Some(96), 0, ActionSequence::default()),
        );

        // Self-rebind: same id, same key+mods → no conflict.
        assert_eq!(check_trigger_key_conflict(&state, id, 96, 0), None);
    }

    /// UX-11: Binding a different macro to the same (keycode, modifiers) pair
    /// must report the existing holder's id.
    #[test]
    fn bind_conflict_rejected() {
        let mut state = AppState::default();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        state.macros.insert(
            id_a,
            make_macro(id_a, "alpha", Some(96), 0, ActionSequence::default()),
        );
        state.macros.insert(
            id_b,
            make_macro(id_b, "beta", Some(96), 0, ActionSequence::default()),
        );

        // From `id_a`'s perspective, `id_b` holds the same (96, 0) pair.
        assert_eq!(
            check_trigger_key_conflict(&state, id_a, 96, 0),
            Some(id_b)
        );
        // And symmetrically.
        assert_eq!(
            check_trigger_key_conflict(&state, id_b, 96, 0),
            Some(id_a)
        );
        // Different keycode → no conflict.
        assert_eq!(check_trigger_key_conflict(&state, id_a, 97, 0), None);
    }

    /// UX-12: Two enabled macros sharing the same `InputEvent` appear in
    /// `state.conflicts` after `recompute_conflicts`.
    #[test]
    fn conflict_detection_overlap() {
        let mut state = AppState::default();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        state.macros.insert(
            id_a,
            make_macro(
                id_a,
                "alpha",
                None,
                0,
                ActionSequence {
                    steps: vec![ActionStep::InterleavedInterval {
                        input: InputEvent::MouseButton(MouseButton::Left),
                        interval_ms: 100,
                    }],
                },
            ),
        );
        state.macros.insert(
            id_b,
            make_macro(
                id_b,
                "beta",
                None,
                0,
                ActionSequence {
                    steps: vec![ActionStep::InterleavedInterval {
                        input: InputEvent::MouseButton(MouseButton::Left),
                        interval_ms: 200,
                    }],
                },
            ),
        );
        // Enable both — `recompute_conflicts` only considers enabled macros.
        state.macros.get_mut(&id_a).unwrap().enabled = true;
        state.macros.get_mut(&id_b).unwrap().enabled = true;

        recompute_conflicts(&mut state);

        assert_eq!(state.conflicts.len(), 1, "expected exactly one conflict group");
        assert_eq!(state.conflicts[0].macros.len(), 2);
        // The `macros` vec is `sort_unstable`-d by `Uuid::Ord` (NOT insertion
        // order) — the contract is "both ids present" not "this specific order".
        let mut got = state.conflicts[0].macros.clone();
        got.sort();
        let mut want = vec![id_a, id_b];
        want.sort();
        assert_eq!(got, want);
        assert_eq!(
            state.conflicts[0].input,
            InputEvent::MouseButton(MouseButton::Left)
        );
    }

    /// UX-12: Disabling one of the conflicting macros removes the conflict
    /// from the derived `conflicts` field.
    #[test]
    fn conflict_disappear_on_disable() {
        let mut state = AppState::default();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let mk = |id: Uuid| MacroConfig {
            id,
            name: format!("m{}", id),
            interval_ms: 100,
            enabled: true,
            target_app: None,
            sequence: ActionSequence {
                steps: vec![ActionStep::InterleavedInterval {
                    input: InputEvent::MouseButton(MouseButton::Left),
                    interval_ms: 100,
                }],
            },
            trigger_key: None,
            trigger_modifiers: 0,
            trigger_mode: TriggerMode::Pulse,
        };
        state.macros.insert(id_a, mk(id_a));
        state.macros.insert(id_b, mk(id_b));

        recompute_conflicts(&mut state);
        assert_eq!(state.conflicts.len(), 1);

        // Disable one of the two — conflict group must drop out.
        state.macros.get_mut(&id_b).unwrap().enabled = false;
        recompute_conflicts(&mut state);
        assert!(
            state.conflicts.is_empty(),
            "expected no conflicts after disabling one macro, got {:?}",
            state.conflicts
        );
    }

    /// R-5 / Plan 08-06 Task 3: A v2.0 profile saved before Phase 8 must
    /// deserialize cleanly. The pre-Phase-8 `MacroConfig` did not have a
    /// `trigger_modifiers` field, and the pre-Phase-8 `AppState` did not
    /// have a `conflicts` field. `#[serde(default)]` on both new fields
    /// ensures backwards compatibility.
    ///
    /// This test deserializes a hand-crafted JSON string that matches the
    /// pre-Phase-8 on-disk shape (no `trigger_modifiers` on `MacroConfig`,
    /// no `conflicts` on `AppState`) and asserts that the new fields
    /// default to `0` and `[]` respectively while preserving the original
    /// `trigger_key` and `sequence` data.
    #[test]
    fn profile_backwards_compat() {
        // Pre-Phase-8 on-disk shape: a ProfileData with one MacroConfig
        // that has NO `trigger_modifiers` field. The `trigger_key` is set
        // to F5 (keycode 96) so we can verify it's preserved.
        let pre_phase_8_json = r#"{
            "name": "Default",
            "macros": {
                "11111111-1111-1111-1111-111111111111": {
                    "id": "11111111-1111-1111-1111-111111111111",
                    "name": "Pre-Phase-8 Macro",
                    "interval_ms": 100,
                    "enabled": true,
                    "target_app": null,
                    "sequence": {
                        "steps": [
                            {
                                "InterleavedInterval": {
                                    "input": { "MouseButton": "Left" },
                                    "interval_ms": 100
                                }
                            }
                        ]
                    },
                    "trigger_key": 96,
                    "trigger_mode": "Pulse"
                }
            },
            "engine_active": true
        }"#;

        // Deserialize via ProfileData — the on-disk wrapper.
        let profile: crate::persistence::ProfileData = serde_json::from_str(pre_phase_8_json)
            .expect("pre-Phase-8 profile JSON must deserialize cleanly");

        assert_eq!(profile.name, "Default");
        assert_eq!(profile.macros.len(), 1);
        assert!(profile.engine_active);

        let mac = profile
            .macros
            .values()
            .next()
            .expect("profile must contain one macro");
        assert_eq!(mac.name, "Pre-Phase-8 Macro");
        // trigger_key preserved across the serde boundary.
        assert_eq!(mac.trigger_key, Some(96));
        // trigger_modifiers defaulted to 0 (was missing in the JSON).
        assert_eq!(
            mac.trigger_modifiers, 0,
            "trigger_modifiers must default to 0 when absent from the JSON"
        );
        // sequence preserved.
        assert_eq!(mac.sequence.steps.len(), 1);

        // AppState also gets the conflicts field via #[serde(default)].
        // Round-trip an AppState with no `conflicts` key in the JSON.
        let pre_phase_8_appstate_json = r#"{
            "macros": {},
            "emergency_stop_active": false,
            "active_app": null,
            "engine_active": true
        }"#;
        let state: AppState = serde_json::from_str(pre_phase_8_appstate_json)
            .expect("pre-Phase-8 AppState JSON must deserialize cleanly");
        assert!(
            state.conflicts.is_empty(),
            "conflicts must default to [] when absent from the JSON"
        );
    }
}
