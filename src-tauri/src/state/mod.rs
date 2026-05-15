use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

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

// ── Action Sequence Model ────────────────────────────────────────
// Supports simultaneous "Sustained Holds" and "Interleaved Intervals"
// for complex automation like Minecraft AFK farms.

/// A single step within an action sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            macros: HashMap::new(),
            emergency_stop_active: false,
            active_app: None,
            engine_active: true,
        }
    }
}

pub enum Intent {
    AddMacro(MacroConfig),
    RemoveMacro(Uuid),
    SetMacroEnabled(Uuid, bool),
    SetMacroTargetApp(Uuid, Option<String>),
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
}

pub struct StateActor {
    state: AppState,
    receiver: mpsc::Receiver<Intent>,
    scheduler_tx: mpsc::Sender<crate::scheduler::SchedulerIntent>,
    /// Two-Phase Dispatch: receives ActionReady from the scheduler.
    /// The StateActor validates targeting before performing input injection.
    action_rx: mpsc::Receiver<crate::scheduler::ActionReady>,
    app_handle: tauri::AppHandle,
    /// Platform-specific input provider for actual event injection.
    #[cfg(target_os = "macos")]
    input_provider: crate::platform::macos::MacInputProvider,
    #[cfg(target_os = "windows")]
    input_provider: crate::platform::windows::WindowsInputProvider,
}

impl StateActor {
    pub fn new(
        receiver: mpsc::Receiver<Intent>,
        scheduler_tx: mpsc::Sender<crate::scheduler::SchedulerIntent>,
        action_rx: mpsc::Receiver<crate::scheduler::ActionReady>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            state: AppState::default(),
            receiver,
            scheduler_tx,
            action_rx,
            app_handle,
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
            Intent::AddMacro(config) => {
                self.state.macros.insert(config.id, config);
                self.reevaluate_all_macros().await;
            }
            Intent::RemoveMacro(id) => {
                self.state.macros.remove(&id);
                let _ = self
                    .scheduler_tx
                    .send(crate::scheduler::SchedulerIntent::StopMacro(id))
                    .await;
            }
            Intent::SetMacroEnabled(id, enabled) => {
                if let Some(mac) = self.state.macros.get_mut(&id) {
                    mac.enabled = enabled;
                }
                self.reevaluate_all_macros().await;
            }
            Intent::SetMacroTargetApp(id, target) => {
                if let Some(mac) = self.state.macros.get_mut(&id) {
                    mac.target_app = target;
                }
                self.reevaluate_all_macros().await;
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
            }
            Intent::ActiveAppChanged(app) => {
                self.state.active_app = app;
                self.reevaluate_all_macros().await;
            }
            Intent::ToggleMacroHotkey(id) => {
                if let Some(mac) = self.state.macros.get_mut(&id) {
                    mac.enabled = !mac.enabled;
                }
                self.reevaluate_all_macros().await;
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
            }
            Intent::GetState(reply) => {
                let _ = reply.send(self.state.clone());
            }
        }
    }

    async fn reevaluate_all_macros(&self) {
        if self.state.emergency_stop_active || !self.state.engine_active {
            return;
        }

        let mut trigger_keys = std::collections::HashMap::new();

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

            if let Some(key) = mac.trigger_key {
                trigger_keys.insert(key, mac.id);
            }
        }

        #[cfg(target_os = "macos")]
        crate::platform::macos::observer::update_macro_trigger_keys(trigger_keys.clone());
        #[cfg(target_os = "windows")]
        crate::platform::windows::update_macro_trigger_keys(trigger_keys);
    }

    fn broadcast_state(&self) {
        use tauri::Emitter;
        let _ = self.app_handle.emit("state-changed", &self.state);
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
