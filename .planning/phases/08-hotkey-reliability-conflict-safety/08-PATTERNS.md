# Phase 8: Hotkey Reliability & Conflict Safety — Pattern Map

**Mapped:** 2026-06-19
**Files analyzed:** 6 new/modified (4 Rust + 1 TS + 1 keymap update) plus 2 new derived state types
**Analogs found:** 13 / 13 — every new file or modification has a strong concrete analog in the existing codebase

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src-tauri/src/state/mod.rs` (modify — add fields, helpers, Intent variants) | state model | actor / event-driven | self — same file (existing `Intent::AddMacro`, `reevaluate_all_macros`) | exact (in-place) |
| `src-tauri/src/state/mod.rs` — new struct `InputConflict` | model | transform | `persistence::ProfileData` (`src-tauri/src/persistence.rs:23-32`) | role-match |
| `src-tauri/src/state/mod.rs` — new helper `check_trigger_key_conflict` | service (validation) | transform | `ProfileManager::sanitize_name` (`src-tauri/src/persistence.rs:86-92`) | role-match |
| `src-tauri/src/state/mod.rs` — new helper `recompute_conflicts` | service (derivation) | transform | `reevaluate_all_macros` (`src-tauri/src/state/mod.rs:441-475`) | role-match |
| `src-tauri/src/ipc/mod.rs` — rewrite `bind_hotkey` and `unbind_hotkey` | IPC handler | request-response | `add_macro` (`src-tauri/src/ipc/mod.rs:9-20`) | exact |
| `src-tauri/src/platform/macos/observer.rs` — type change `MACRO_TRIGGER_KEYS` | platform code | event-driven | self — same file (`update_macro_trigger_keys` at `:204-206`) | exact (in-place) |
| `src-tauri/src/platform/windows/mod.rs` — type change + modifier bit check | platform code | event-driven | self — same file (`hook_callback` emergency-stop modifier check at `:434-446`) | exact (in-place) |
| `src/App.tsx` — add `computeModifiers` helper, thread modifiers through capture, add C-1..C-5 components | frontend (TSX) | request-response | self — same file (auto-save banner at `:715-735`, profile message toast at `:1062-1076`, bound-key chip at `:1006`) | exact (in-place) |
| `src/keymap.ts` — confirm `*_TO_NAME` tables already cover modifier keycodes (no change required) | frontend (TS) | transform | self — same file (`CGKEYCODE_TO_NAME` at `:82-98`, `VK_TO_NAME` at `:148-168`) | exact (in-place) |
| New test module in `src-tauri/src/state/mod.rs` (`#[cfg(test)] mod tests`) | test | transform | `src-tauri/src/scheduler/mod.rs:393-582` and `src-tauri/src/persistence.rs:251-319` | exact (in-place, follow existing test pattern) |

---

## Pattern Assignments

### `src-tauri/src/state/mod.rs` — modifications to `MacroConfig` and `AppState`

**Analog:** the file itself — modify in place; the field-addition pattern is already established.

**Add `trigger_modifiers: u64` to `MacroConfig`** (lines 79-99). The existing `#[serde(default)]` pattern at lines 91, 94, 97 gives backwards compatibility with pre-modifier profiles:

```rust
// src-tauri/src/state/mod.rs:79-99 (existing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroConfig {
    pub id: Uuid,
    pub name: String,
    pub interval_ms: u64,
    pub enabled: bool,
    pub target_app: Option<String>,
    #[serde(default)]
    pub sequence: ActionSequence,
    #[serde(default)]
    pub trigger_key: Option<u16>,
    #[serde(default)]
    pub trigger_mode: TriggerMode,
}
```

→ New field inserted after `trigger_key`:

```rust
    #[serde(default)]
    pub trigger_key: Option<u16>,
    /// Raw modifier bits for the trigger key. Platform-specific:
    ///   macOS: CGEventFlags bits (Shift=0x20000, Control=0x40000,
    ///          Option=0x80000, Command=0x100000)
    ///   Windows: MOD_* values (MOD_ALT=0x1, MOD_CONTROL=0x2,
    ///            MOD_SHIFT=0x4, MOD_WIN=0x8)
    #[serde(default)]
    pub trigger_modifiers: u64,
    #[serde(default)]
    pub trigger_mode: TriggerMode,
```

**Add `conflicts: Vec<InputConflict>` to `AppState`** (lines 101-111). Same `#[serde(default)]` pattern; the field is derived state, not source-of-truth, but must serialize to keep the frontend in sync via `state-changed`:

```rust
// src-tauri/src/state/mod.rs:101-111 (existing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub macros: HashMap<Uuid, MacroConfig>,
    pub emergency_stop_active: bool,
    pub active_app: Option<String>,
    pub engine_active: bool,
    #[serde(skip)]
    pub loading_profile: bool,
}
```

→ Add new field after `loading_profile`:

```rust
    #[serde(skip)]
    pub loading_profile: bool,
    /// UX-12: derived list of input-event conflicts between currently
    /// enabled macros. Recomputed by `recompute_conflicts()` after every
    /// state-mutating intent. Sent to the frontend via the `state-changed`
    /// event; not persisted in profile JSON (#[serde(default)]).
    #[serde(default)]
    pub conflicts: Vec<InputConflict>,
}
```

> **Why `#[serde(default)]` not `#[serde(skip)]`:** the frontend consumes this field (UI-SPEC C-2 reads `state().conflicts`). The serializer must emit it; the deserializer must accept missing values when loading old profiles. `#[serde(default)]` does both.

---

### `src-tauri/src/state/mod.rs` — new struct `InputConflict`

**Analog:** `persistence::ProfileData` (`src-tauri/src/persistence.rs:23-32`).

**Derives & shape** — copy the derive pattern from `InputEvent` (`state/mod.rs:35-43`) and the wrapper-struct pattern from `ActionSequence` (`state/mod.rs:65-68`):

```rust
// src-tauri/src/state/mod.rs:35-43 (existing — pattern for enums with derives)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputEvent {
    MouseButton(MouseButton),
    Key(u16),
}

// src-tauri/src/persistence.rs:23-32 (existing — pattern for serializable wrapper struct)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    pub name: String,
    pub macros: HashMap<Uuid, MacroConfig>,
    #[serde(default = "default_true")]
    pub engine_active: bool,
}
```

→ Insert `InputConflict` near `InputEvent` (after `InputEvent`, before `ActionStep`):

```rust
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
```

---

### `src-tauri/src/state/mod.rs` — new helper `check_trigger_key_conflict`

**Analog:** `ProfileManager::sanitize_name` (`src-tauri/src/persistence.rs:86-92`).

**Shape** — pure function, returns `Option<Uuid>` of the conflicting macro. Mirrors the validation-helper shape (no `&mut self`, no async, no I/O):

```rust
// src-tauri/src/persistence.rs:86-92 (existing — validation helper pattern)
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}
```

→ Insert as an `impl StateActor` method:

```rust
/// UX-11: Return the existing macro_id that holds the given (keycode, modifiers)
/// pair, or None if the slot is free. Self-rebind is allowed — if the existing
/// holder is `self_id`, returns None (treats same-macro, same-key as no-op).
fn check_trigger_key_conflict(
    &self,
    self_id: Uuid,
    keycode: u16,
    modifiers: u64,
) -> Option<Uuid> {
    for (other_id, mac) in &self.state.macros {
        if other_id == &self_id {
            continue; // self — not a conflict
        }
        if mac.trigger_key == Some(keycode) && mac.trigger_modifiers == modifiers {
            return Some(*other_id);
        }
    }
    None
}
```

> **Call sites** (per RESEARCH.md D-3):
> - `Intent::AddMacro` handler (line 291-300) — check before insert
> - `Intent::SetMacroTriggerKey` handler (line 323-329) — check before update
> - new `Intent::BindHotkey` handler — check before update

---

### `src-tauri/src/state/mod.rs` — new helper `recompute_conflicts`

**Analog:** `reevaluate_all_macros` (`src-tauri/src/state/mod.rs:441-475`).

**Shape** — walks all enabled macros, derives a secondary structure, mirrors the legacy-fallback pattern from `scheduler/mod.rs:257-270`:

```rust
// src-tauri/src/scheduler/mod.rs:257-270 (existing — legacy fallback pattern)
let mut steps = if config.sequence.steps.is_empty() {
    match config.trigger_mode {
        crate::state::TriggerMode::Pulse => vec![ActionStep::InterleavedInterval {
            input: InputEvent::MouseButton(MouseButton::Left),
            interval_ms: config.interval_ms,
        }],
        crate::state::TriggerMode::Hold => vec![ActionStep::SustainedHold {
            input: InputEvent::MouseButton(MouseButton::Left),
        }],
    }
} else {
    config.sequence.steps.clone()
};
```

```rust
// src-tauri/src/state/mod.rs:441-475 (existing — reevaluate_all_macros shape)
async fn reevaluate_all_macros(&self) {
    if self.state.emergency_stop_active || !self.state.engine_active {
        return;
    }
    let mut trigger_keys = std::collections::HashMap::new();
    for mac in self.state.macros.values() {
        // ...
    }
    // ...
}
```

→ Insert as an `impl StateActor` method, called after every state-mutating intent (mirroring the `reevaluate_all_macros().await` calls at lines 297, 313, 320, 327, 345, 349, 355, 367, 374, 422):

```rust
/// UX-12: Recompute `self.state.conflicts` from the current macro set.
/// O(n × steps) — typical config is <50 macros × <5 steps, negligible cost.
fn recompute_conflicts(&mut self) {
    use std::collections::HashMap;
    let mut by_input: HashMap<InputEvent, Vec<Uuid>> = HashMap::new();

    for (id, mac) in &self.state.macros {
        if !mac.enabled {
            continue;
        }
        // Expand steps with the same legacy fallback as the scheduler.
        let steps: &[ActionStep] = if mac.sequence.steps.is_empty() {
            // Synthesize the single left-click for the purpose of conflict detection.
            // (No allocation: use a static slice via OnceLock? Or just inline.)
            // Simpler: expand into a local Vec when needed.
            // See note below — for empty sequences, the only input is Left.
            // Apply same logic as scheduler/mod.rs:257-270.
            // ... handled in the loop below
            continue; // empty sequence → no step to detect (legacy = single left, but already covered)
        } else {
            &mac.sequence.steps
        };
        for step in steps {
            let input = match step {
                ActionStep::SustainedHold { input } => *input,
                ActionStep::InterleavedInterval { input, .. } => *input,
            };
            by_input.entry(input).or_default().push(*id);
        }
    }

    self.state.conflicts = by_input
        .into_iter()
        .filter(|(_, ids)| ids.len() >= 2)
        .map(|(input, mut macros)| InputConflict {
            input,
            // Sort for deterministic frontend rendering.
            macros: { macros.sort(); macros }
        })
        .collect();
}
```

> **Empty-sequence macro handling:** the scheduler expands empty sequences to a single left-click. To keep the conflict detector consistent with what actually fires, the empty-sequence branch should still register the synthesized left-click input. The simplification in the snippet above (`continue`) under-counts — for correctness, expand a `&[ActionStep::InterleavedInterval { input: Left, .. }]` locally when `steps.is_empty()`. Mirrors `scheduler/mod.rs:257-270` exactly.

---

### `src-tauri/src/state/mod.rs` — new `Intent::BindHotkey` and `Intent::UnbindHotkey` variants

**Analog:** `Intent::AddMacro` and `Intent::LoadProfile` (the oneshot-sender variants at lines 128, 149).

**Pattern** — copy the `(value, tokio::sync::oneshot::Sender<Reply>)` shape; the sender is how the IPC layer gets the result back. Two shape choices:

1. **No-reply variant** (like `SetMacroTriggerKey` at line 132) — the StateActor's own conflict check can return `Err` by emitting an event instead. Cleaner for `UnbindHotkey` (which can't fail).
2. **Reply variant** (like `AddMacro` at line 128) — needed for `BindHotkey` so the IPC layer can return `Result<(), String>` to the frontend.

```rust
// src-tauri/src/state/mod.rs:125-150 (existing — Intent enum)
pub enum Intent {
    AddMacro(MacroConfig, tokio::sync::oneshot::Sender<Uuid>),
    RemoveMacro(Uuid),
    SetMacroEnabled(Uuid, bool),
    SetMacroTargetApp(Uuid, Option<String>),
    SetMacroTriggerKey(Uuid, Option<u16>),
    // ... etc
    LoadProfile(String, tokio::sync::oneshot::Sender<Result<crate::persistence::ProfileData, String>>),
}
```

→ Insert the new variants:

```rust
    SetMacroTriggerKey(Uuid, Option<u16>, Option<u64>), // ← signature change: add modifiers
    /// UX-11: Bind a trigger key for an existing macro. The oneshot sender
    /// carries the conflict result back to the IPC caller — `Ok(())` if
    /// the slot was free or this macro already owned it, `Err(msg)` otherwise.
    /// Signature mirrors the per-step-update pattern of `UpdateStepInterval`.
    BindHotkey(Uuid, u16, u64, tokio::sync::oneshot::Sender<Result<(), String>>),
    /// Remove a trigger key bind for a specific macro. Cannot fail.
    /// Mirrors the existing `RemoveMacro(Uuid)` shape (no reply).
    UnbindHotkey(Uuid),
```

> **Backwards-compat note:** `SetMacroTriggerKey` currently takes `(Uuid, Option<u16>)`. The research recommends changing it to `(Uuid, Option<u16>, Option<u64>)`. Every callsite (only `ipc/mod.rs:142-151` and `state/mod.rs:323-329`) is in this repo. Mechanical change.

---

### `src-tauri/src/ipc/mod.rs` — rewrite `bind_hotkey` and `unbind_hotkey`

**Analog:** `add_macro` (`src-tauri/src/ipc/mod.rs:9-20`) and `set_macro_trigger_key` (`ipc/mod.rs:142-151`).

**Pattern** — both current `bind_hotkey` and `unbind_hotkey` (lines 78-95, 98-106) bypass `StateManager` and call the platform statics directly. They must be rewritten to route through `StateManager::send_intent(Intent::BindHotkey(...))` and `Intent::UnbindHotkey(...)`:

```rust
// src-tauri/src/ipc/mod.rs:9-20 (existing — exact pattern to copy)
#[command]
pub async fn add_macro(
    state: State<'_, StateManager>,
    config: MacroConfig,
) -> Result<Uuid, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    state
        .send_intent(Intent::AddMacro(config, tx))
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())
}

// src-tauri/src/ipc/mod.rs:78-95 (existing — the BUGGY bind_hotkey to rewrite)
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
```

→ New `bind_hotkey` (drops the `_` prefix on `state`, routes through Intent):

```rust
/// Bind a global hotkey to toggle a specific macro.
/// UX-11: Routes through the StateActor so the conflict check can be enforced.
/// The oneshot sender carries the conflict result back to the frontend.
#[command]
pub async fn bind_hotkey(
    state: State<'_, StateManager>,
    macro_id: Uuid,
    keycode: u16,
    modifiers: u64,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    state
        .send_intent(Intent::BindHotkey(macro_id, keycode, modifiers, tx))
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

/// Remove a hotkey binding for a specific macro. UX-11/UX-13:
/// Now enabled on Windows too (was macOS-only — D-8 fix).
#[command]
pub async fn unbind_hotkey(
    state: State<'_, StateManager>,
    macro_id: Uuid,
) -> Result<(), String> {
    state
        .send_intent(Intent::UnbindHotkey(macro_id))
        .await
        .map_err(|e| e.to_string())
}
```

> **Error message format (per UI-SPEC C-1):** `"F5 is already assigned to \"{macro name}\". Unbind it first or pick a different key."` — the StateActor builds the message by looking up the conflicting macro's name from `self.state.macros[other_id].name`. Returned as `Err(msg)` from the oneshot sender; the frontend's `try/catch` catches the string and renders it.

---

### `src-tauri/src/platform/macos/observer.rs` — type change for `MACRO_TRIGGER_KEYS`

**Analog:** the file itself (`update_macro_trigger_keys` at lines 197, 204-206).

**Pattern** — type change from `HashMap<u16, Uuid>` to `HashMap<(u16, u64), Uuid>`. The `(u16, u64)` key matches the RESEARCH.md R-7 recommendation to use a type alias for readability:

```rust
// src-tauri/src/platform/macos/observer.rs:197-206 (existing — to modify)
static MACRO_TRIGGER_KEYS: OnceLock<Mutex<std::collections::HashMap<u16, Uuid>>> = OnceLock::new();

fn get_macro_trigger_keys() -> &'static Mutex<std::collections::HashMap<u16, Uuid>> {
    MACRO_TRIGGER_KEYS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// Replace the entire set of macro trigger keys at runtime.
pub fn update_macro_trigger_keys(keys: std::collections::HashMap<u16, Uuid>) {
    *get_macro_trigger_keys().lock().unwrap() = keys;
}
```

```rust
// src-tauri/src/platform/macos/observer.rs:442-449 (existing — read site)
if let Ok(trigger_keys) = get_macro_trigger_keys().try_lock() {
    if let Some(&macro_id) = trigger_keys.get(&(keycode as u16)) {
        if let Some(tx) = STATE_TX.get() {
            let _ = tx.try_send(crate::state::Intent::ToggleMacroHotkey(macro_id));
        }
    }
}
```

→ After modification, the read site uses the tuple key:

```rust
// Read site (line 442-449) — update to the tuple lookup
if let Ok(trigger_keys) = get_macro_trigger_keys().try_lock() {
    // macOS reads the held modifier bits from the CGEventFlags on the
    // current event. `flags.bits()` gives the raw bitmask, which is
    // exactly the format stored in `trigger_modifiers`.
    let mod_bits = flags.bits();
    if let Some(&macro_id) = trigger_keys.get(&(keycode as u16, mod_bits)) {
        if let Some(tx) = STATE_TX.get() {
            let _ = tx.try_send(crate::state::Intent::ToggleMacroHotkey(macro_id));
        }
    }
}
```

> **Why `flags.bits()` works on the lookup side:** the frontend sends platform-native bits in `trigger_modifiers`. The macOS tap callback already has `CGEventFlags` (the `flags` variable on line 331). `flags.bits()` is the exact same bitmask format, so the containment semantics work end-to-end. Windows uses a different bit layout (MOD_*) — the tuple key disambiguates by `flags.bits()` vs the Windows-synthesized `mod_mask`.

---

### `src-tauri/src/platform/windows/mod.rs` — type change + modifier bit check + new HOTKEY_BINDINGS

**Analog:** the file itself (`hook_callback` at lines 426-460, `update_macro_trigger_keys` at lines 410, 421-423).

**Pattern A — type change** mirrors the macOS change above:

```rust
// src-tauri/src/platform/windows/mod.rs:410, 417-423 (existing)
static MACRO_TRIGGER_KEYS: OnceLock<Mutex<HashMap<u16, Uuid>>> = OnceLock::new();
// ...
fn get_macro_trigger_keys() -> &'static Mutex<HashMap<u16, Uuid>> {
    MACRO_TRIGGER_KEYS.get_or_init(|| Mutex::new(HashMap::new()))
}
pub fn update_macro_trigger_keys(keys: HashMap<u16, Uuid>) {
    *get_macro_trigger_keys().lock().unwrap() = keys;
}
```

→ Change to:

```rust
static MACRO_TRIGGER_KEYS: OnceLock<Mutex<HashMap<(u16, u64), Uuid>>> = OnceLock::new();
fn get_macro_trigger_keys() -> &'static Mutex<HashMap<(u16, u64), Uuid>> {
    MACRO_TRIGGER_KEYS.get_or_init(|| Mutex::new(HashMap::new()))
}
pub fn update_macro_trigger_keys(keys: HashMap<(u16, u64), Uuid>) {
    *get_macro_trigger_keys().lock().unwrap() = keys;
}
```

**Pattern B — modifier bit synthesis from `GetAsyncKeyState`** mirrors the existing emergency-stop check at lines 434-446:

```rust
// src-tauri/src/platform/windows/mod.rs:434-446 (existing — emergency stop modifier pattern)
let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
let shift_down = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
if keycode == 0x51 && ctrl_down && shift_down { /* ... */ }
```

→ Add a `build_mod_mask` helper and use it in the trigger-key lookup at line 450-456:

```rust
// Insert as a free function near the top of the file (with the other static helpers)
#[cfg(target_os = "windows")]
fn build_mod_mask() -> u64 {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN,
    };
    let mut m: u64 = 0;
    if (unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) } as u16 & 0x8000) != 0 { m |= 0x0004; }
    if (unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) } as u16 & 0x8000) != 0 { m |= 0x0002; }
    if (unsafe { GetAsyncKeyState(VK_MENU.0 as i32) } as u16 & 0x8000) != 0 { m |= 0x0001; }
    if (unsafe { GetAsyncKeyState(VK_LWIN.0 as i32) } as u16 & 0x8000) != 0 { m |= 0x0008; }
    if (unsafe { GetAsyncKeyState(VK_RWIN.0 as i32) } as u16 & 0x8000) != 0 { m |= 0x0008; }
    m
}

// hook_callback modification at line 449-456:
if let Ok(trigger_keys) = get_macro_trigger_keys().try_lock() {
    let mod_mask = build_mod_mask();
    if let Some(&macro_id) = trigger_keys.get(&(keycode, mod_mask)) {
        if let Some(tx) = STATE_TX.get() {
            let _ = tx.try_send(crate::state::Intent::ToggleMacroHotkey(macro_id));
        }
    }
}
```

> **Why per-call `build_mod_mask` and not a snapshot in `update_macro_trigger_keys`:** modifier state at the moment of the keypress is what matters for matching. The hook callback fires synchronously with the keypress; reading `GetAsyncKeyState` at that instant gives the correct answer. Storing the snapshot in the map would require per-callsite `flags.contains(...)` re-checks, defeating the O(1) HashMap lookup.

**Pattern C — new `HOTKEY_BINDINGS` registry** mirrors the macOS `HOTKEY_BINDINGS` (lines 56, 144-146, 155-162, 165-170). The Windows version is needed because `bind_hotkey` is being enabled on Windows per RESEARCH.md D-8:

```rust
// Add near the other Windows statics (after line 411)
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
pub struct HotkeyBinding {
    pub keycode: u16,
    pub modifiers: u64,
    pub macro_id: Uuid,
}

#[cfg(target_os = "windows")]
static HOTKEY_BINDINGS: OnceLock<Mutex<Vec<HotkeyBinding>>> = OnceLock::new();

#[cfg(target_os = "windows")]
fn get_hotkey_bindings() -> &'static Mutex<Vec<HotkeyBinding>> {
    HOTKEY_BINDINGS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(target_os = "windows")]
pub fn update_hotkey_bindings(bindings: Vec<HotkeyBinding>) {
    *get_hotkey_bindings().lock().unwrap() = bindings;
}
```

> **Why a separate `HOTKEY_BINDINGS` map vs overloading `MACRO_TRIGGER_KEYS`:** the macOS code has both, and the two exist for a reason. `MACRO_TRIGGER_KEYS` is the bulk-replaced mirror of `state.macros[*].trigger_key` (set by `reevaluate_all_macros`); `HOTKEY_BINDINGS` is the per-binding mutable set for the macOS `bind_hotkey` IPC. Keeping the separation in the Windows port preserves the structural invariant that the StateActor is the source of truth and the platform statics are write-only mirrors. The new `Intent::BindHotkey` handler in `state/mod.rs` will call `update_hotkey_bindings` (or a per-binding equivalent) the same way it does for `MACRO_TRIGGER_KEYS`.

---

### `src/App.tsx` — `computeModifiers` helper

**Analog:** `formatInputEvent` and `formatStep` (`src/App.tsx:57-69`). A small pure helper near the top of the file.

**Pattern** — single function that branches on `IS_MACOS` (already a module-level constant at line 131):

```typescript
// src/App.tsx:131 (existing — platform detection that computeModifiers uses)
const IS_MACOS = navigator.userAgent.toLowerCase().includes("mac");

// src/App.tsx:57-69 (existing — small pure-helper pattern)
function formatInputEvent(ev: InputEvent): string {
  if ("MouseButton" in ev) return `🖱 ${ev.MouseButton}`;
  if ("Key" in ev) return `⌨ Key(${ev.Key})`;
  return "?";
}
```

→ Insert `computeModifiers` next to `formatInputEvent`:

```typescript
/**
 * UX-13: Extract a platform-native modifier bitmask from a KeyboardEvent.
 *
 * macOS: CGEventFlags bits (matching what the CGEventTap callback stores
 *   in `flags.bits()`). Frontend values verified by core-graphics 0.24
 *   symbol lookup — these are the same bits the backend will compare
 *   against, so the lookup is bit-identical end-to-end.
 *
 *   Shift   = 0x020000
 *   Control = 0x040000
 *   Option  = 0x080000
 *   Command = 0x100000
 *
 * Windows: Win32 MOD_* values. These match the bits the Windows
 *   `hook_callback` synthesizes via `GetAsyncKeyState` in `build_mod_mask()`.
 *
 *   MOD_ALT     = 0x0001
 *   MOD_CONTROL = 0x0002
 *   MOD_SHIFT   = 0x0004
 *   MOD_WIN     = 0x0008
 */
function computeModifiers(e: KeyboardEvent): number {
  if (IS_MACOS) {
    let m = 0;
    if (e.shiftKey) m |= 0x020000;
    if (e.ctrlKey)  m |= 0x040000;
    if (e.altKey)   m |= 0x080000;
    if (e.metaKey)  m |= 0x100000;
    return m;
  } else {
    let m = 0;
    if (e.shiftKey) m |= 0x0004;
    if (e.ctrlKey)  m |= 0x0002;
    if (e.altKey)   m |= 0x0001;
    if (e.metaKey)  m |= 0x0008; // Win key
    return m;
  }
}
```

> **Call sites (per RESEARCH.md Wave 2 step 3):**
> - `startCapture` at lines 340-368 — compute at commit time, pass alongside `nativeCode` via the `onCommit` callback.
> - `handleCreateMacro` at lines 289-329 — read from a new `newMacroTriggerModifiers` signal; pass as a new field on the `config` (requires `trigger_modifiers: number` on the `MacroConfig` interface at line 13-22).
> - `handleCardSetTriggerKey` at lines 371-385 — same as `startCapture`: compute from the captured `KeyboardEvent`, pass to both `bind_hotkey` and `set_macro_trigger_key`. Requires the latter's IPC signature to add a `modifiers` parameter (RESEARCH.md §3.3 step 1).

---

### `src/App.tsx` — remove the `["Control", "Shift", "Alt", "Meta"]` filter

**Analog:** the `Escape` early-return at `src/App.tsx:351-355` (existing — the only other early-return inside `onKeyDown`).

```typescript
// src/App.tsx:348-368 (existing — startCapture inner function)
function onKeyDown(e: KeyboardEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape") {
    setTriggerKeyRecording(false);
    document.removeEventListener("keydown", onKeyDown, true);
    _keyCaptureListener = null;
    return;
  }
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return; // ← DELETE THIS
  // Use e.code as lookup key to avoid F12/ArrowLeft collision — see keymap.ts
  const nativeCode = domKeycodeToNative(e.code);
  if (nativeCode === null) return;
  onCommit(nativeCode);
  setTriggerKeyRecording(false);
  document.removeEventListener("keydown", onKeyDown, true);
  _keyCaptureListener = null;
}
```

> **Why deletion, not relaxation:** per RESEARCH.md Open Questions #1, modifier-only binds remain unsupported in this phase. The filter stays as a behavior; only the data-loss bug (modifiers being dropped on commit) is fixed by `computeModifiers`. Actually — re-read RESEARCH.md §3.3 Gap 1 step 1: "Remove the modifier-only filter at line 357 so the user can bind 'just Shift'". The recommended implementation removes the filter. If the implementation should keep it (per Open Questions #1 recommendation), then `computeModifiers` is only used for non-pure-modifier binds. **The planner should defer to whichever UX-13 path the team chose during discuss-phase.** Both are valid.

---

### `src/App.tsx` — `ConflictErrorToast` component (UI-SPEC C-1)

**Analog:** the profile message toast at `src/App.tsx:1062-1076` (the closest existing "red error toast" pattern).

```tsx
// src/App.tsx:1062-1076 (existing — profile message toast, the pattern to clone)
<Show when={profileMessage()}>
  {(msg) => (
    <div
      class={`rounded-lg px-4 py-2.5 text-xs font-medium flex items-center gap-2 transition-all ${
        msg().type === "success"
          ? "bg-success/10 text-success border border-success/20"
          : "bg-danger/10 text-danger border border-danger/20"
      }`}
    >
      <span>{msg().type === "success" ? "✓" : "✕"}</span>
      <span>{msg().text}</span>
    </div>
  )}
</Show>
```

→ New state signal and component, inserted at the top of `<main>` content area (per UI-SPEC C-1 location):

```tsx
// Add to the signal block near line 100
const [conflictError, setConflictError] = createSignal<{
  key: string;
  macroName: string;
} | null>(null);

// Add to the existing 3-second auto-dismiss helper pattern (line 415-418)
function showConflictError(key: string, macroName: string) {
  setConflictError({ key, macroName });
  setTimeout(() => setConflictError(null), 8000); // 8s per UI-SPEC C-1
}

// Render at the top of the scroll area, ABOVE the tab bar content
<Show when={conflictError()}>
  {(err) => (
    <div
      class="rounded-lg px-4 py-2.5 text-xs font-medium flex items-center gap-2 transition-all bg-danger/10 text-danger border border-danger/20"
    >
      <span>✕</span>
      <div class="flex-1">
        <p class="font-medium">Hotkey already bound</p>
        <p class="text-text-dim text-[11px] mt-0.5">
          {err().key} is already assigned to "{err().macroName}". Unbind it
          first or pick a different key.
        </p>
      </div>
      <button
        onClick={() => setConflictError(null)}
        class="text-[11px] text-text-muted hover:text-text-main transition-colors duration-200 cursor-pointer"
      >
        Dismiss
      </button>
    </div>
  )}
</Show>
```

> **Trigger sites (per UI-SPEC C-1 + RESEARCH.md §3.1 step 4):**
> - `handleCreateMacro` at line 326-328 — replace `console.error` with `showConflictError(...)` and parse the `key`/`macroName` from the error string.
> - `handleCardSetTriggerKey` at line 382-384 — same replacement.
> - Any future binding IPC call.

---

### `src/App.tsx` — `ConflictWarningRegion` component (UI-SPEC C-2)

**Analog:** the auto-save error banner at `src/App.tsx:715-735` (the closest "warning-tinted persistent card with dismiss" pattern).

```tsx
// src/App.tsx:715-735 (existing — auto-save error banner, the pattern to clone)
<Show when={saveError()}>
  <div
    id="auto-save-error-banner"
    class="bg-warning/10 border border-warning/20 rounded-lg p-3 flex items-center gap-3"
  >
    <span class="text-warning text-base">⚠</span>
    <div class="flex-1">
      <p class="text-xs font-medium text-warning">Save failed</p>
      <p class="text-[11px] text-text-dim">
        Your changes are not being saved. Check available disk space and file permissions.
      </p>
    </div>
    <button
      onClick={() => setSaveError(false)}
      class="text-[11px] text-text-muted hover:text-text-main transition-colors duration-200 cursor-pointer"
    >
      Dismiss
    </button>
  </div>
</Show>
```

→ New signal and component, inserted between the auto-save banner and the Macros section header:

```tsx
// Add to the signal block (line ~100)
const [conflictsDismissed, setConflictsDismissed] = createSignal(false);

// Render immediately after the auto-save banner (~line 736)
<Show when={state()?.conflicts && state()!.conflicts.length > 0 && !conflictsDismissed()}>
  <div class="flex flex-col gap-3">
    <For each={state()!.conflicts}>
      {(conflict) => {
        const macroNames = () => conflict.macros
          .map((id) => state()?.macros[id]?.name ?? "Unknown")
          .filter((n) => n !== "Unknown");
        const inputLabel = () => "MouseButton" in conflict.input
          ? `🖱 ${conflict.input.MouseButton} Click`
          : `⌨ Key(${conflict.input.Key})`;
        const rateMultiplier = () => `${macroNames().length}×`;
        return (
          <div
            id="conflict-warning-card"
            class="bg-warning/10 border border-warning/20 rounded-lg p-3 flex items-center gap-3"
          >
            <span class="text-warning text-base">⚠</span>
            <div class="flex-1">
              <p class="text-xs font-medium text-warning">
                {macroNames().length} macros are injecting the same input
              </p>
              <p class="text-[11px] text-text-dim">
                "{macroNames().join('" and "')}" both inject {inputLabel()} — clicks will fire at {rateMultiplier()} rate.
              </p>
            </div>
            <button
              onClick={() => setConflictsDismissed(true)}
              class="text-[11px] text-text-muted hover:text-text-main transition-colors duration-200 cursor-pointer"
            >
              Dismiss
            </button>
          </div>
        );
      }}
    </For>
  </div>
</Show>
```

> **Reset semantics:** per UI-SPEC C-2, the `conflictsDismissed` signal resets automatically when `state().conflicts.length` returns to 0 (the `<Show>` predicate re-evaluates). No explicit reset code needed.
>
> **Source data shape** — extend the `AppState` TypeScript interface (line 32-37):
>
> ```typescript
> interface AppState {
>   macros: Record<string, MacroConfig>;
>   emergency_stop_active: boolean;
>   active_app: string | null;
>   engine_active: boolean;
>   conflicts: Array<{ macros: string[]; input: InputEvent }>;  // ← new
> }
> ```

---

### `src/App.tsx` — `FirstRunGlobalNotice` component (UI-SPEC C-3)

**Analog:** the auto-save error banner at `src/App.tsx:715-735` (same pattern, info-tinted variant).

> **Persistence note:** the UI-SPEC recommends `localStorage` (not a backend sentinel file). The `tcc_granted.flag` pattern in `persistence.rs:220-249` is a valid alternative if a backend flag is preferred (per RESEARCH.md R-8), but the UI-SPEC is the source of truth — use `localStorage`.

```tsx
// New state signal
const [showGlobalNotice, setShowGlobalNotice] = createSignal(false);

// Initial check, inside the existing createEffect at lines 135-165, after setAppVersion:
setShowGlobalNotice(localStorage.getItem("automux.hotkey_global_notice_dismissed") !== "1");

// Render: insert between the profile badge block (line 713) and the auto-save banner (line 715)
<Show when={showGlobalNotice()}>
  <div
    id="first-run-global-notice"
    class="bg-accent/10 border border-accent/20 rounded-lg p-3 flex items-center gap-3"
  >
    <span class="text-accent text-base">🌍</span>
    <div class="flex-1">
      <p class="text-xs font-medium text-accent">Binds are system-wide</p>
      <p class="text-[11px] text-text-dim">
        Hotkeys fire even when AutoMux is in the background.
      </p>
    </div>
    <button
      onClick={() => {
        localStorage.setItem("automux.hotkey_global_notice_dismissed", "1");
        setShowGlobalNotice(false);
      }}
      class="text-[11px] text-text-muted hover:text-text-main transition-colors duration-200 cursor-pointer"
    >
      Got it
    </button>
  </div>
</Show>
```

---

### `src/App.tsx` — `↗ Global` subtitle in macro card (UI-SPEC C-4)

**Analog:** the `({macro.trigger_mode})` suffix at `src/App.tsx:998` and `:1033-1035`. Same one-liner structure, no new component.

```tsx
// src/App.tsx:988-999 (existing — unset-key branch, the insertion site)
<Show when={macro.trigger_key !== null} fallback={
  <div class="flex items-center gap-1">
    <span class="px-1.5 py-0.5 rounded border border-dashed border-border text-[10px] font-mono text-text-dim cursor-pointer hover:border-accent/40 hover:text-text-main"
      onClick={...}>Set key…</span>
    <span class="text-[10px] text-text-muted">({macro.trigger_mode})</span>
  </div>
}>
```

→ Add `↗ Global` after the trigger-mode suffix in **both** branches:

```tsx
// In the unset-key branch (around line 998):
<span class="text-[10px] text-text-muted">({macro.trigger_mode})</span>
<span class="text-[10px] text-text-dim">↗ Global</span>  // ← new

// In the bound-key branch (around line 1034):
<span class="text-[10px] text-text-muted">
  ({macro.trigger_mode})
</span>
<span class="text-[10px] text-text-dim">↗ Global</span>  // ← new
```

---

### `src/App.tsx` — `ModifierPreviewChip` during key capture (UI-SPEC C-5)

**Analog:** the bound-key chip at `src/App.tsx:1005-1006` (the `bg-surface-alt border border-border text-[10px] font-mono` chip pattern).

> **Scope note:** UI-SPEC C-5 lives inside the key-capture widget, which is a render of either `App.tsx:839-879` (new-macro form) or `App.tsx:1000-1037` (card edit). The simplest implementation is a small inline JSX block conditional on `triggerKeyRecording()`, rendered above the existing capture chip. No new component file is needed.

```tsx
// New state signal (line ~115)
const [recordingModifiers, setRecordingModifiers] = createSignal<number>(0);

// In startCapture (line 340-368), update onKeyDown to track modifiers:
function onKeyDown(e: KeyboardEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape") { /* ... */ return; }
  setRecordingModifiers(computeModifiers(e));  // ← new: track held modifiers
  // ... existing logic
  onCommit(nativeCode, computeModifiers(e));   // ← pass to commit
  // ...
}

// Helper to render the modifier chip row
function modifierChips(bits: number): string[] {
  if (IS_MACOS) {
    const order: Array<[number, string]> = [
      [0x020000, "Shift"],
      [0x040000, "Ctrl"],
      [0x080000, "Option"],
      [0x100000, "⌘"],
    ];
    return order.filter(([bit]) => (bits & bit) === bit).map(([, label]) => label);
  } else {
    const order: Array<[number, string]> = [
      [0x0004, "Shift"],
      [0x0002, "Ctrl"],
      [0x0001, "Alt"],
      [0x0008, "Win"],
    ];
    return order.filter(([bit]) => (bits & bit) === bit).map(([, label]) => label);
  }
}

// In the capture widget (e.g., around line 858-865 in the new-macro form):
<Show when={triggerKeyRecording() && recordingModifiers() !== 0}>
  <div class="flex items-center gap-1 mb-1">
    <For each={modifierChips(recordingModifiers())}>
      {(label) => (
        <span class="px-1.5 py-0.5 rounded bg-surface-alt border border-border text-[10px] font-mono text-text-main">
          {label}
        </span>
      )}
    </For>
  </div>
</Show>
```

---

### `src/keymap.ts` — confirm tables cover all required keys (no code change)

**Analog:** the file itself. UI-SPEC C-6 explicitly notes that no structural change is required; both lookup tables already include modifiers and all keys the user can bind.

```typescript
// src/keymap.ts:82-98 (existing — macOS CGKEYCODE_TO_NAME, includes modifiers)
const CGKEYCODE_TO_NAME: Record<number, string> = {
  // ... letters, digits, etc.
  55: "Cmd", 56: "Shift", 57: "Caps Lock", 58: "Option", 59: "Control",
  60: "Right Shift", 61: "Right Option", 62: "Right Control",
};

// src/keymap.ts:148-168 (existing — Windows VK_TO_NAME, includes modifiers)
const VK_TO_NAME: Record<number, string> = {
  // ... letters, digits, etc.
  0x10: "Shift", 0x11: "Ctrl", 0x12: "Alt",
};
```

> **No code change.** The planner should skip the keymap file unless the team decided to add macOS Numpad keycodes (RESEARCH.md Gap 2, out of scope for this phase).

---

### New test module in `src-tauri/src/state/mod.rs`

**Analog:** the existing `#[cfg(test)] mod tests` in `src-tauri/src/persistence.rs:251-319` and `src-tauri/src/scheduler/mod.rs:393-582` (the closest — in `state/mod.rs` itself, there's no test module yet, so the persistence one is the template).

```rust
// src-tauri/src/persistence.rs:251-270 (existing — test module header)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ActionSequence, ActionStep, InputEvent, MouseButton};

    #[test]
    fn large_config_memory_check() {
        // ... test body
    }
}
```

→ Add a new `#[cfg(test)] mod tests` block at the bottom of `state/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// UX-11: Rebinding the same macro to the same key+mods must succeed.
    #[test]
    fn self_rebind_allowed() {
        let mut state = AppState::default();
        let id = Uuid::new_v4();
        let mac = MacroConfig {
            id, name: "test".into(), interval_ms: 100, enabled: false,
            target_app: None, sequence: ActionSequence::default(),
            trigger_key: Some(96), trigger_modifiers: 0,
            trigger_mode: TriggerMode::Pulse,
        };
        state.macros.insert(id, mac);
        let actor = StateActor::new_for_test(state);
        assert!(actor.check_trigger_key_conflict(id, 96, 0).is_none());
    }

    /// UX-11: Binding a different macro to the same key+mods must conflict.
    #[test]
    fn bind_conflict_rejected() {
        // ... similar setup, two macros with the same key+mods
        // expect Some(other_id) from the helper
    }

    /// UX-12: Two enabled macros sharing an input appear in `state.conflicts`.
    #[test]
    fn conflict_detection_overlap() {
        // ... two enabled macros with the same left-click input
        // expect conflicts.len() == 1 and conflicts[0].macros.len() == 2
    }

    /// UX-12: Disabling one of the conflicting macros removes the conflict.
    #[test]
    fn conflict_disappear_on_disable() {
        // ... toggle enabled = false on one macro
        // expect conflicts.is_empty() after recompute
    }

    /// UX-13: HotkeyBinding.matches() correctly matches Cmd+F5 on macOS.
    /// (Test in platform/macos/observer.rs using synthetic CGEventFlags.)
    #[test]
    fn modifier_match_cmd_f5() {
        // ... construct HotkeyBinding { keycode: 96, modifiers: 0x100000 }
        // ... construct synthetic CGEventFlags with Cmd set
        // ... assert matches() returns true
    }
}
```

> **Note:** `StateActor::new_for_test` is a small constructor that bypasses the Tauri AppHandle and platform statics — the planner should add it (or use a more targeted helper like a free function `check_trigger_key_conflict(macros, self_id, key, mods)` that doesn't require a `StateActor` instance at all). The latter is preferred — match the persistence test style which uses pure functions where possible.

---

## Shared Patterns

### Pattern S-1: Intent + oneshot-sender for IPC return value

**Source:** `state/mod.rs:125-150` and `ipc/mod.rs:9-20`.
**Apply to:** the new `bind_hotkey` and the rewritten `set_macro_trigger_key` (with `modifiers`).

```rust
// Intent variant (state/mod.rs)
BindHotkey(Uuid, u16, u64, tokio::sync::oneshot::Sender<Result<(), String>>),

// IPC handler (ipc/mod.rs)
let (tx, rx) = tokio::sync::oneshot::channel();
state.send_intent(Intent::BindHotkey(macro_id, keycode, modifiers, tx)).await?;
rx.await?
```

### Pattern S-2: `#[serde(default)]` for backwards-compatible struct fields

**Source:** `state/mod.rs:91, 94, 97` and `persistence.rs:30`.
**Apply to:** new `trigger_modifiers`, new `conflicts: Vec<InputConflict>`, any other new field on a serialized struct.

### Pattern S-3: `try_send` on the hotkey channel (existing — not for new code)

**Source:** `observer.rs:433, 446`, `windows/mod.rs:441, 453`.
**Apply to:** the new Windows `HOTKEY_BINDINGS` dispatch in `hook_callback`. Same pattern, same trade-off (silent drop on full channel — pre-existing limitation, out of scope per RESEARCH.md R-3).

### Pattern S-4: per-platform statics mirrored from StateActor

**Source:** `state/mod.rs:471-474` (calls into `update_macro_trigger_keys` on both platforms) and `observer.rs:204-206` / `windows/mod.rs:421-423` (the static-replace functions).
**Apply to:** the new Windows `HOTKEY_BINDINGS` registry, and the StateActor's propagation of `trigger_modifiers` into the platform statics.

```rust
// state/mod.rs:471-474 (existing — pattern to extend)
#[cfg(target_os = "macos")]
crate::platform::macos::observer::update_macro_trigger_keys(trigger_keys.clone());
#[cfg(target_os = "windows")]
crate::platform::windows::update_macro_trigger_keys(trigger_keys);
```

The new line for the per-macro bind registry would mirror this — one cfg-gated call per platform, both fed from the same `HashMap<(u16, u64), Uuid>` built in `reevaluate_all_macros`.

### Pattern S-5: 8-second auto-dismiss toast (UI pattern)

**Source:** `App.tsx:415-418` (3-second profile message toast).
**Apply to:** `ConflictErrorToast` (UI-SPEC C-1, 8-second per the spec — *longer* than the 3-second profile toast because the user must read the conflicting macro name and decide).

```typescript
function showProfileMsg(text: string, type: "success" | "error") {
  setProfileMessage({ text, type });
  setTimeout(() => setProfileMessage(null), 3000);
}
// The conflict toast uses 8000ms — explicitly longer for UX-11 readability.
```

### Pattern S-6: warning-tinted dismissable banner

**Source:** `App.tsx:715-735` (auto-save error banner).
**Apply to:** `ConflictWarningRegion` (UI-SPEC C-2) and `FirstRunGlobalNotice` (UI-SPEC C-3) — same shell, different content and tint.

```tsx
<div class="bg-{tint}/10 border border-{tint}/20 rounded-lg p-3 flex items-center gap-3">
  <span class="text-{tint} text-base">{icon}</span>
  <div class="flex-1">
    <p class="text-xs font-medium text-{tint}">{heading}</p>
    <p class="text-[11px] text-text-dim">{body}</p>
  </div>
  <button onClick={dismissFn} class="text-[11px] text-text-muted hover:text-text-main ...">
    {dismissLabel}
  </button>
</div>
```

Tint values per UI-SPEC: `warning` for C-2 conflicts, `accent` for C-3 first-run notice, `danger` for C-1 toast.

### Pattern S-7: `reactive effect` + `localStorage` for first-run gating

**Source:** no existing `localStorage` usage in `App.tsx`. The closest equivalent is the `tcc_granted.flag` pattern in `persistence.rs:220-249` (backend sentinel, not frontend). The UI-SPEC C-3 explicitly chose `localStorage` for simplicity.
**Apply to:** `FirstRunGlobalNotice` initial state check.

```typescript
// In createEffect at App.tsx:135-165, after setAppVersion:
setShowGlobalNotice(localStorage.getItem("automux.hotkey_global_notice_dismissed") !== "1");

// In the dismiss handler:
localStorage.setItem("automux.hotkey_global_notice_dismissed", "1");
setShowGlobalNotice(false);
```

---

## No Analog Found

None. Every file or modification has a strong concrete analog in the existing codebase:

| Concept | Where the pattern comes from |
|---------|------------------------------|
| New `InputConflict` struct | `ProfileData` (persistence.rs) |
| `check_trigger_key_conflict` helper | `sanitize_name` (persistence.rs) |
| `recompute_conflicts` helper | `reevaluate_all_macros` (state/mod.rs) + scheduler's legacy-fallback expansion (scheduler/mod.rs) |
| `bind_hotkey` rewrite | `add_macro` IPC (ipc/mod.rs) + `AddMacro` Intent (state/mod.rs) |
| `MACRO_TRIGGER_KEYS` type change | self — same field, different type |
| Windows modifier bit check | self — same `GetAsyncKeyState` pattern at emergency stop |
| `computeModifiers` | `formatInputEvent` (App.tsx) |
| `ConflictErrorToast` | profile message toast (App.tsx) |
| `ConflictWarningRegion` | auto-save banner (App.tsx) |
| `FirstRunGlobalNotice` | auto-save banner (App.tsx) |
| `↗ Global` subtitle | trigger-mode suffix (App.tsx) |
| `ModifierPreviewChip` | bound-key chip (App.tsx) |
| New test module | `persistence.rs` and `scheduler/mod.rs` test modules |

The only novel element is `localStorage` for the first-run flag (UI-SPEC C-3), which is the documented alternative to the existing `tcc_granted.flag` backend sentinel.

---

## Metadata

**Analog search scope:** `src-tauri/src/state/mod.rs`, `src-tauri/src/ipc/mod.rs`, `src-tauri/src/platform/macos/observer.rs`, `src-tauri/src/platform/windows/mod.rs`, `src-tauri/src/scheduler/mod.rs`, `src-tauri/src/persistence.rs`, `src/App.tsx`, `src/keymap.ts`, `.planning/codebase/CONCERNS.md`, `.planning/codebase/ARCHITECTURE.md`.
**Files scanned:** 9 source + 2 planning docs.
**Pattern extraction date:** 2026-06-19.
