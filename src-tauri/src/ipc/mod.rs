use crate::state::{ActionSequence, AppState, Intent, MacroConfig, StateManager};
use tauri::{command, State};
use uuid::Uuid;

#[command]
pub async fn add_macro(state: State<'_, StateManager>, config: MacroConfig) -> Result<(), String> {
    state
        .send_intent(Intent::AddMacro(config))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn remove_macro(state: State<'_, StateManager>, id: Uuid) -> Result<(), String> {
    state
        .send_intent(Intent::RemoveMacro(id))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn set_macro_enabled(
    state: State<'_, StateManager>,
    id: Uuid,
    enabled: bool,
) -> Result<(), String> {
    state
        .send_intent(Intent::SetMacroEnabled(id, enabled))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn set_macro_target_app(
    state: State<'_, StateManager>,
    id: Uuid,
    target_app: Option<String>,
) -> Result<(), String> {
    state
        .send_intent(Intent::SetMacroTargetApp(id, target_app))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_state(state: State<'_, StateManager>) -> Result<AppState, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    state
        .send_intent(Intent::GetState(tx))
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_active_app(state: State<'_, StateManager>) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    state
        .send_intent(Intent::GetState(tx))
        .await
        .map_err(|e| e.to_string())?;
    let app_state = rx.await.map_err(|e| e.to_string())?;
    Ok(app_state.active_app)
}

/// Bind a global hotkey to toggle a specific macro.
/// `keycode`: macOS virtual keycode (e.g. 12 = Q, 0 = A)
/// `modifiers`: raw CGEventFlags bits (e.g. Cmd=0x100000, Shift=0x20000)
#[command]
pub async fn bind_hotkey(
    _state: State<'_, StateManager>,
    _macro_id: Uuid,
    _keycode: u16,
    _modifiers: u64,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use crate::platform::macos::observer::{add_hotkey_binding, HotkeyAction, HotkeyBinding};
        add_hotkey_binding(HotkeyBinding {
            keycode: _keycode,
            modifiers: _modifiers,
            action: HotkeyAction::ToggleMacro(_macro_id),
        });
    }
    Ok(())
}

/// Remove all hotkey bindings for a specific macro.
#[command]
pub async fn unbind_hotkey(_state: State<'_, StateManager>, _macro_id: Uuid) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use crate::platform::macos::observer::remove_hotkey_bindings_for;
        remove_hotkey_bindings_for(&_macro_id);
    }
    Ok(())
}

/// Toggle the global engine on/off.
#[command]
pub async fn toggle_engine(state: State<'_, StateManager>) -> Result<(), String> {
    state
        .send_intent(Intent::ToggleEngineHotkey)
        .await
        .map_err(|e| e.to_string())
}

/// Request Accessibility permissions from the OS.
///
/// - Shows the macOS system dialog prompting the user to grant access.
/// - If already trusted, initializes the CGEventTap (idempotent).
/// - Returns `true` if the process currently has Accessibility permissions.
///
/// The SolidJS frontend should call this on launch and display
/// a "Permissions Required" indicator when it returns `false`.
#[command]
pub async fn request_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let granted = crate::platform::macos::check_accessibility_permissions(true);
        if granted {
            crate::platform::macos::observer::initialize_tap();
        }
        Ok(granted)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows/Linux: no accessibility gate
        Ok(true)
    }
}

/// Silent check: returns current accessibility status without prompting.
#[command]
pub async fn check_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(crate::platform::macos::check_accessibility_permissions(
            false,
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

// ── Task 3.7: ActionSequence IPC Commands ────────────────────────

/// Set the full action sequence for a macro.
/// This replaces any existing sequence and restarts the macro
/// in the scheduler if it's currently active.
///
/// Example payload from frontend:
/// ```json
/// {
///   "id": "uuid...",
///   "sequence": {
///     "steps": [
///       { "SustainedHold": { "input": { "MouseButton": "Right" } } },
///       { "InterleavedInterval": { "input": { "MouseButton": "Left" }, "interval_ms": 650 } }
///     ]
///   }
/// }
/// ```
#[command]
pub async fn set_macro_sequence(
    state: State<'_, StateManager>,
    id: Uuid,
    sequence: ActionSequence,
) -> Result<(), String> {
    state
        .send_intent(Intent::UpdateSequence(id, sequence))
        .await
        .map_err(|e| e.to_string())
}

/// Update the interval for a specific step within a running macro.
/// Allows the frontend to live-tune timing without restarting the macro.
/// `step_index` is 0-based and maps to the ActionSequence step order.
#[command]
pub async fn update_step_interval(
    state: State<'_, StateManager>,
    id: Uuid,
    step_index: usize,
    interval_ms: u64,
) -> Result<(), String> {
    state
        .send_intent(Intent::UpdateStepInterval(id, step_index, interval_ms))
        .await
        .map_err(|e| e.to_string())
}

// ── Task 5.1: Profile Management IPC Commands ────────────────────

/// Save the current macro state as a named profile.
/// If a profile with this name already exists, it is overwritten.
#[command]
pub async fn save_profile(
    state: State<'_, StateManager>,
    profile_mgr: State<'_, crate::persistence::ProfileManager>,
    name: String,
) -> Result<(), String> {
    // Snapshot the current state.
    let (tx, rx) = tokio::sync::oneshot::channel();
    state
        .send_intent(Intent::GetState(tx))
        .await
        .map_err(|e| e.to_string())?;
    let app_state = rx.await.map_err(|e| e.to_string())?;

    let profile = crate::persistence::ProfileData {
        name,
        macros: app_state.macros,
        engine_active: app_state.engine_active,
    };

    profile_mgr.save_profile(&profile).await
}

/// Load a named profile, replacing all current macros.
/// Returns the loaded profile data for the frontend to update.
#[command]
pub async fn load_profile(
    state: State<'_, StateManager>,
    profile_mgr: State<'_, crate::persistence::ProfileManager>,
    name: String,
) -> Result<crate::persistence::ProfileData, String> {
    let profile = profile_mgr.load_profile(&name).await?;

    // First, get current state to know which macros to remove.
    let (tx, rx) = tokio::sync::oneshot::channel();
    state
        .send_intent(Intent::GetState(tx))
        .await
        .map_err(|e| e.to_string())?;
    let current = rx.await.map_err(|e| e.to_string())?;

    // Remove all existing macros.
    for id in current.macros.keys() {
        let _ = state.send_intent(Intent::RemoveMacro(*id)).await;
    }

    // Replay the profile macros into the StateActor.
    for config in profile.macros.values() {
        let _ = state.send_intent(Intent::AddMacro(config.clone())).await;
    }

    Ok(profile)
}

/// Delete a saved profile by name.
#[command]
pub async fn delete_profile(
    profile_mgr: State<'_, crate::persistence::ProfileManager>,
    name: String,
) -> Result<(), String> {
    profile_mgr.delete_profile(&name).await
}

/// List all saved profiles (name + macro count).
#[command]
pub async fn list_profiles(
    profile_mgr: State<'_, crate::persistence::ProfileManager>,
) -> Result<Vec<crate::persistence::ProfileSummary>, String> {
    profile_mgr.list_profiles().await
}
