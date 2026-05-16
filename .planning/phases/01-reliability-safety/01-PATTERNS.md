# Phase 1: Reliability & Safety - Pattern Map

**Mapped:** 2026-05-16
**Files analyzed:** 7 (all modifications — no new files created in this phase)
**Analogs found:** 7 / 7 (all files are their own analog — this phase is surgical in-place fixes)

---

## File Classification

| Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------|------|-----------|----------------|---------------|
| `src/App.tsx` | component | request-response | `src/App.tsx` (self) | exact |
| `src-tauri/src/scheduler/mod.rs` | service | event-driven | `src-tauri/src/scheduler/mod.rs` (self) | exact |
| `src-tauri/src/platform/macos/input.rs` | platform-provider | request-response | `src-tauri/src/platform/macos/input.rs` (self) | exact |
| `src-tauri/src/platform/macos/observer.rs` | platform-observer | event-driven | `src-tauri/src/platform/macos/observer.rs` (self) | exact |
| `src-tauri/src/platform/windows/mod.rs` | platform-provider + observer | event-driven | `src-tauri/src/platform/windows/mod.rs` (self) | exact |
| `src-tauri/src/state/mod.rs` | state-actor | CRUD + event-driven | `src-tauri/src/state/mod.rs` (self) | exact |
| `src-tauri/src/lib.rs` | config/wiring | request-response | `src-tauri/src/lib.rs` (self) | exact |

> Note: Phase 1 is pure in-place fixes. Every modified file is its own best analog. Pattern extraction below shows the exact existing code adjacent to each fix site.

---

## Pattern Assignments

### RELY-01 — `src/App.tsx` (component, request-response)

**Fix:** Change `setInterval` from `10000` to `3000` on line 133.

**Existing code at fix site** (`src/App.tsx:124–135`):
```typescript
// Poll accessibility every 3s
createEffect(() => {
  const interval = setInterval(async () => {
    try {
      const ok = await invoke<boolean>("check_accessibility");
      setAccessibility(ok);
    } catch (_) {
      /* ignore */
    }
  }, 10000);  // <-- CHANGE THIS to 3000
  onCleanup(() => clearInterval(interval));
});
```

**Pattern:** `createEffect` + `setInterval` + `onCleanup` — the SolidJS polling pattern already in use. The fix is one character change on line 133. No structural change needed.

**Error handling pattern** (lines 128–131): Silent `catch (_)` with `/* ignore */` comment — this is the established pattern for background polls in this codebase. Do not add logging or user notification (D-03).

---

### SAFE-03 — `src-tauri/src/scheduler/mod.rs` (service, event-driven)

**Fix:** Change `.max(1)` to `.max(5)` at two sites.

**Site 1** — `IntervalTask::new` (line 66):
```rust
impl IntervalTask {
    fn new(step_id: StepId, input: InputEvent, interval_ms: u64) -> Self {
        let interval = Duration::from_millis(interval_ms.max(1));  // <-- CHANGE to .max(5)
        Self {
            step_id,
            input,
            interval,
            next_fire: Instant::now() + interval,
        }
    }
```

**Site 2** — `UpdateInterval` handler (line 184):
```rust
if let Some(task) = self.interval_tasks.get_mut(&step_id) {
    let old_fire = task.next_fire;
    task.interval = Duration::from_millis(new_ms.max(1));  // <-- CHANGE to .max(5)
    task.next_fire = Instant::now() + task.interval;
```

**Pattern:** Both sites use the same `.max(N)` clamping idiom on `interval_ms: u64`. The change is mechanical — two identical substitutions. No new pattern needed.

---

### SAFE-01 (macOS) — `src-tauri/src/platform/macos/input.rs` (platform-provider, request-response)

**Fix:** Change `source()` from a panicking function to one returning `Option<CGEventSource>`.

**Current panicking code** (lines 22–25):
```rust
fn source() -> CGEventSource {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .expect("Failed to create CGEventSource")
}
```

**Pattern to copy for the fix** — the `if let Ok(...)` pattern already used throughout the same file (lines 31, 53–58, 66–69, 75–83, 90–93, 111–114):
```rust
// Existing pattern — model for all callers after source() returns Option:
if let Ok(event) = CGEvent::new_keyboard_event(source, keycode, is_down) {
    event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
    event.post(CGEventTapLocation::HID);
}
// If creation fails, we skip silently — no panic.
```

**Target pattern for source() fix:**
```rust
// After fix — source() returns Option<CGEventSource>:
fn source() -> Option<CGEventSource> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()
}

// Each inject_* method guards on Some:
fn inject_key(&self, keycode: u16, is_down: bool) {
    let Some(source) = Self::source() else { return; };
    if let Ok(event) = CGEvent::new_keyboard_event(source, keycode, is_down) {
        event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
        event.post(CGEventTapLocation::HID);
    }
}
```

**Debug logging pattern** (already present in codebase via `#[cfg(debug_assertions)]` in `persistence.rs:108`):
```rust
#[cfg(debug_assertions)]
eprintln!("[MacInput] CGEventSource creation failed — skipping injection");
```
This is optional but follows the project convention if a trace is desired.

---

### SAFE-01 (Windows) — `src-tauri/src/platform/windows/mod.rs` (platform-provider, event-driven)

**Fix:** Replace `lock().unwrap()` with `let Ok(...) else { return; }` at four mutex lock sites.

**Current panicking code** (lines 173, 188, 196, 208 — examples):
```rust
// Line 173 (inject_key):
let mut guard = get_held_inputs().lock().unwrap();

// Line 188 (inject_mouse_click):
let mut guard = get_held_inputs().lock().unwrap();

// Line 196 (inject_mouse_click second lock):
let mut guard = get_held_inputs().lock().unwrap();

// Line 208 (inject_mouse_button_raw):
let mut guard = get_held_inputs().lock().unwrap();
```

**Pattern to copy for the fix** — already used elsewhere in the same file (lines 356):
```rust
// Existing non-panicking mutex pattern (line 356 in hook_callback):
if let Ok(trigger_keys) = get_macro_trigger_keys().try_lock() {
    if let Some(&macro_id) = trigger_keys.get(&keycode) { ... }
}
```

**Target pattern for each unwrap() site:**
```rust
// After fix — let-else early return on lock failure:
let Ok(mut guard) = get_held_inputs().lock() else { return; };
```

**Scope:** Lines 51 and 331 (`flush_all_held_inputs` internal lock and `update_macro_trigger_keys`) are NOT in scope for SAFE-01. Only lines 173, 188, 196, 208 in the `InputProvider` impl are hot-path injection sites.

---

### RELY-02 — `src-tauri/src/platform/macos/observer.rs` (platform-observer, event-driven)

**Fix:** Add CGEventTap timeout re-enable in the CGEventTap callback closure.

**Current callback signature** (line 206):
```rust
|_proxy, event_type, event| {
    let user_data = event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA);
    if user_data == crate::platform::macos::input::LLMHF_INJECTED {
        // ... registry tracking ...
        return Some(event.clone());
    }

    if matches!(event_type, CGEventType::KeyDown) {
        // ... hotkey and emergency stop handling ...
    }
    // ...
}
```

**Target pattern after fix** — add tap-disabled detection BEFORE the existing LLMHF_INJECTED check:
```rust
// Rename _proxy → tap_proxy in closure signature to enable .enable() call:
|tap_proxy, event_type, event| {
    // RELY-02: Re-enable tap if OS disabled it.
    // VERIFIED: core-graphics 0.24.0/src/event.rs:140-142 defines the named
    // variants TapDisabledByTimeout (0xFFFFFFFE) and TapDisabledByUserInput
    // (0xFFFFFFFF). Both must be matched; CGEventType::Null (value 0) is NOT
    // the timeout signal.
    if matches!(
        event_type,
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
    ) {
        tap_proxy.enable();
        return None;
    }

    // ... existing LLMHF_INJECTED check and hotkey handling unchanged ...
}
```

**Precedent in the same file** — `tap.enable()` is already called at observer.rs line 369 (in `initialize_tap`), confirming the method exists on the proxy type:
```rust
// Existing enable() call (line 369, in initialize_tap):
tap.enable();
```

**Critical renaming note:** The closure parameter is currently `_proxy` (Rust unused-variable convention). It must be renamed to `tap_proxy` — otherwise `.enable()` cannot be called. The rename removes the leading underscore.

---

### RELY-03 — `src-tauri/src/platform/windows/mod.rs` (platform-observer, event-driven)

**Fix:** Call `WindowsInputProvider::flush_all_held_inputs()` synchronously in `hook_callback` before `process::exit(1)`.

**Current emergency stop code** (lines 347–352):
```rust
if keycode == 0x51 && ctrl_down && shift_down {
    println!("EMERGENCY STOP TRIGGERED");
    if let Some(tx) = STATE_TX.get() {
        let _ = tx.try_send(crate::state::Intent::TriggerEmergencyStop);
    }
    std::process::exit(1);  // <-- keys may be stuck at this point
}
```

**Pattern to copy** — macOS inline flush approach in `observer.rs:266–320` (emergency stop uses an inline loop over `get_registry()` before exit). The Windows equivalent uses the existing `flush_all_held_inputs()` function:

**`flush_all_held_inputs` implementation** (lines 49–66 — read-only reference, no changes needed here):
```rust
pub fn flush_all_held_inputs() {
    let held: Vec<HeldInputKey> = {
        let mut guard = get_held_inputs().lock().unwrap();
        guard.drain().collect()
    };
    let provider = WindowsInputProvider::new();
    for input in held {
        match input {
            HeldInputKey::Key(keycode) => { provider.send_key_event(keycode, false); }
            HeldInputKey::Mouse(button) => { provider.send_mouse_button(button, false); }
        }
    }
}
```

**Target pattern for hook_callback fix:**
```rust
if keycode == 0x51 && ctrl_down && shift_down {
    println!("EMERGENCY STOP TRIGGERED");
    if let Some(tx) = STATE_TX.get() {
        let _ = tx.try_send(crate::state::Intent::TriggerEmergencyStop);
    }
    // RELY-03: Synchronous flush before exit — matches macOS approach.
    // Best-effort: exit regardless of individual event failures.
    WindowsInputProvider::flush_all_held_inputs();
    std::process::exit(1);
}
```

---

### RELY-04 — `src-tauri/src/state/mod.rs` + `src-tauri/src/lib.rs` (state-actor, CRUD)

This is the highest-complexity fix. It touches both files.

#### Part A: `AppState` — add `loading_profile` field

**Current `AppState` struct** (`state/mod.rs:99–116`):
```rust
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
```

**Target pattern — add `loading_profile`:**
```rust
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
}
// Default impl: add loading_profile: false
```

#### Part B: `Intent` enum — add `LoadProfile` variant

**Current `Intent` enum** (`state/mod.rs:118–136`):
```rust
pub enum Intent {
    AddMacro(MacroConfig),
    RemoveMacro(Uuid),
    SetMacroEnabled(Uuid, bool),
    SetMacroTargetApp(Uuid, Option<String>),
    TriggerEmergencyStop,
    ResetEmergencyStop,
    ActiveAppChanged(Option<String>),
    ToggleMacroHotkey(Uuid),
    ToggleEngineHotkey,
    UpdateSequence(Uuid, ActionSequence),
    UpdateStepInterval(Uuid, usize, u64),
    GetState(tokio::sync::oneshot::Sender<AppState>),
}
```

**Target pattern — add `LoadProfile` variant:**
```rust
pub enum Intent {
    // ... existing variants unchanged ...
    /// Load a named profile, suppressing per-mutation auto-save during the batch.
    /// Handler clears macros, replays all macros from profile, then saves once.
    LoadProfile(String),
}
```

#### Part C: `StateActor` — add `profile_mgr` field

**Current `StateActor` struct** (`state/mod.rs:138–151`):
```rust
pub struct StateActor {
    state: AppState,
    receiver: mpsc::Receiver<Intent>,
    scheduler_tx: mpsc::Sender<crate::scheduler::SchedulerIntent>,
    action_rx: mpsc::Receiver<crate::scheduler::ActionReady>,
    app_handle: tauri::AppHandle,
    #[cfg(target_os = "macos")]
    input_provider: crate::platform::macos::MacInputProvider,
    #[cfg(target_os = "windows")]
    input_provider: crate::platform::windows::WindowsInputProvider,
}
```

**Target pattern — add `profile_mgr`:**
```rust
use std::sync::Arc;
use crate::persistence::ProfileManager;

pub struct StateActor {
    state: AppState,
    receiver: mpsc::Receiver<Intent>,
    scheduler_tx: mpsc::Sender<crate::scheduler::SchedulerIntent>,
    action_rx: mpsc::Receiver<crate::scheduler::ActionReady>,
    app_handle: tauri::AppHandle,
    profile_mgr: Arc<ProfileManager>,  // RELY-04: injected for auto-save
    #[cfg(target_os = "macos")]
    input_provider: crate::platform::macos::MacInputProvider,
    #[cfg(target_os = "windows")]
    input_provider: crate::platform::windows::WindowsInputProvider,
}
```

**`StateActor::new` signature update:**
```rust
pub fn new(
    receiver: mpsc::Receiver<Intent>,
    scheduler_tx: mpsc::Sender<crate::scheduler::SchedulerIntent>,
    action_rx: mpsc::Receiver<crate::scheduler::ActionReady>,
    app_handle: tauri::AppHandle,
    profile_mgr: Arc<ProfileManager>,  // RELY-04: new parameter
) -> Self {
    Self {
        state: AppState::default(),
        receiver,
        scheduler_tx,
        action_rx,
        app_handle,
        profile_mgr,
        // ... existing platform input_provider fields ...
    }
}
```

#### Part D: `auto_save_default` private helper

**Model:** `broadcast_state` method pattern (`state/mod.rs:402–405`):
```rust
fn broadcast_state(&self) {
    use tauri::Emitter;
    let _ = self.app_handle.emit("state-changed", &self.state);
}
```

**Target pattern — new async helper:**
```rust
async fn auto_save_default(&self) {
    if self.state.loading_profile {
        return;
    }
    use crate::persistence::ProfileData;
    let profile = ProfileData {
        name: "default".to_string(),
        macros: self.state.macros.clone(),
        engine_active: self.state.engine_active,  // D-PITFALL-4: never hardcode
    };
    if let Err(e) = self.profile_mgr.save_profile(&profile).await {
        // D-07: emit transient error event — same mechanism as broadcast_state
        use tauri::Emitter;
        let _ = self.app_handle.emit("auto-save-error", e.to_string());
    }
}
```

#### Part E: `handle_intent` — call `auto_save_default` after mutating intents

**Mutating intents** (identified from `state/mod.rs:271–363`):
`AddMacro`, `RemoveMacro`, `SetMacroEnabled`, `SetMacroTargetApp`, `UpdateSequence`, `UpdateStepInterval`

**Pattern** — append `.await` call after each mutating arm. Example for `AddMacro` (lines 272–275):
```rust
Intent::AddMacro(config) => {
    self.state.macros.insert(config.id, config);
    self.reevaluate_all_macros().await;
    self.auto_save_default().await;  // RELY-04: add this line
}
```

#### Part F: `Intent::LoadProfile` handler

**Pattern modeled on `TriggerEmergencyStop` handler** (lines 296–307) — uses scheduler stop + state mutation:
```rust
Intent::LoadProfile(name) => {
    // 1. Clear existing macros and stop all scheduler tasks
    self.state.macros.clear();
    let _ = self.scheduler_tx
        .send(crate::scheduler::SchedulerIntent::StopAll)
        .await;
    // 2. Set suppression flag (D-06)
    self.state.loading_profile = true;
    // 3. Load profile and populate macros directly (no per-macro auto-save)
    match self.profile_mgr.load_profile(&name).await {
        Ok(profile) => {
            for (_, config) in profile.macros {
                self.state.macros.insert(config.id, config);
            }
            // Unconditional assignment — fully restore saved engine state.
            // Do NOT use `if profile.engine_active { ... = true; }` —
            // that is a one-way ratchet (can enable, never disable). See Pitfall 6.
            self.state.engine_active = profile.engine_active;
            self.reevaluate_all_macros().await;
        }
        Err(e) => {
            use tauri::Emitter;
            let _ = self.app_handle.emit("auto-save-error", e.to_string());
        }
    }
    // 4. Clear flag, then write once (D-06)
    self.state.loading_profile = false;
    self.auto_save_default().await;
}
```

#### Part G: `lib.rs` — wire `Arc<ProfileManager>` into `StateActor`

**Current wiring** (`lib.rs:28–56`):
```rust
let profile_mgr = ProfileManager::from_app_handle(app.handle())?;
app.manage(profile_mgr.clone());

let startup_tx = state_tx.clone();
let startup_mgr = profile_mgr.clone();
tauri::async_runtime::spawn(async move {
    let profile = startup_mgr.load_or_create_default().await;
    for (_, config) in profile.macros {
        let _ = startup_tx.send(Intent::AddMacro(config)).await;
    }
});

let actor = StateActor::new(state_rx, sched_tx, action_rx, app_handle);
```

**Target wiring — Arc wrapping + convert startup to LoadProfile intent:**
```rust
use std::sync::Arc;

let profile_mgr = Arc::new(ProfileManager::from_app_handle(app.handle())?);
app.manage(profile_mgr.clone());  // Tauri managed state still works with Arc clone

// RELY-04 PITFALL-2: Convert startup restoration to use LoadProfile intent
// to prevent N auto-saves on startup. Send a single bracketed intent.
let startup_tx = state_tx.clone();
tauri::async_runtime::spawn(async move {
    // Send LoadProfile("default") — StateActor handles the full batch with suppression
    let _ = startup_tx.send(Intent::LoadProfile("default".to_string())).await;
});

let app_handle = app.handle().clone();
let actor = StateActor::new(state_rx, sched_tx, action_rx, app_handle, profile_mgr.clone());
```

**Note:** `ProfileManager` is `Clone` (derives `Clone`, `lib.rs:58`). Wrapping in `Arc` adds one pointer indirection. All existing `app.manage(profile_mgr.clone())` and IPC `State<'_, ProfileManager>` accesses work unchanged because Tauri's managed state stores the `Arc<ProfileManager>` — IPC handlers must update their state type annotation to `State<'_, Arc<ProfileManager>>`.

---

## Shared Patterns

### if-let-Ok / let-else — Silent skip on failure
**Source:** `src-tauri/src/platform/macos/input.rs:31,53,66,75,90,111`
**Apply to:** SAFE-01 fixes in both `macos/input.rs` and `windows/mod.rs`
```rust
// macOS variant (Result):
if let Ok(event) = CGEvent::new_keyboard_event(source, keycode, is_down) {
    // use event
}

// Windows variant (let-else early return):
let Ok(mut guard) = get_held_inputs().lock() else { return; };
```

### Tauri event emission for StateActor → frontend notifications
**Source:** `src-tauri/src/state/mod.rs:402–405`
**Apply to:** RELY-04 `auto_save_default` error emission (D-07)
```rust
fn broadcast_state(&self) {
    use tauri::Emitter;
    let _ = self.app_handle.emit("state-changed", &self.state);
}
// Pattern: use tauri::Emitter; let _ = self.app_handle.emit("event-name", payload);
```

### `#[cfg(debug_assertions)]` debug trace guard
**Source:** `src-tauri/src/state/mod.rs:222–223`, `persistence.rs:108–112`
**Apply to:** Any optional debug logging added alongside SAFE-01 or RELY-04 fixes
```rust
#[cfg(debug_assertions)]
eprintln!("[ComponentName] descriptive message: {}", value);
```

### `try_send` for fire-and-forget from non-async contexts
**Source:** `src-tauri/src/platform/macos/observer.rs:263`, `src-tauri/src/platform/windows/mod.rs:350`
**Apply to:** Any platform-layer Intent dispatch (already used correctly; do NOT change to `.await` channel sends in callbacks)
```rust
if let Some(tx) = STATE_TX.get() {
    let _ = tx.try_send(crate::state::Intent::SomeVariant);
}
```

### `Arc::clone` for shared ownership of non-`Copy` services
**Source:** `src-tauri/src/lib.rs:29,34` (existing `profile_mgr.clone()` pattern before Arc)
**Apply to:** RELY-04 `Arc<ProfileManager>` injection into `StateActor`
```rust
// After wrapping in Arc:
let profile_mgr = Arc::new(ProfileManager::from_app_handle(app.handle())?);
app.manage(profile_mgr.clone());       // clone 1: Tauri managed state
// ...
let actor = StateActor::new(..., profile_mgr.clone());  // clone 2: StateActor
```

---

## No Analog Found

All files have existing code at the exact fix sites. No file requires patterns from outside the codebase.

---

## Key Anti-Patterns (from RESEARCH.md)

| Anti-Pattern | Where It Would Appear | Correct Alternative |
|---|---|---|
| Adding `ProfileManager` to `AppState` | `state/mod.rs` AppState struct | Keep on `StateActor` struct — `AppState` must stay serializable |
| Calling `save_profile` with `.block_in_place` | `auto_save_default` helper | Use `.await` — `handle_intent` is already `async` |
| Skipping `loading_profile` guard during startup | `lib.rs` startup restoration | Convert startup to `Intent::LoadProfile`, not individual `AddMacro` intents |
| Removing `CGEventTapOptions::ListenOnly` | `observer.rs` tap initialization | Preserve existing tap options; RELY-02 only adds event_type detection |
| Calling `.into_inner()` on poisoned mutex | Windows `inject_*` methods | Use `let Ok(guard) = ... else { return; }` — skip on poison |
| Hardcoding `engine_active: true` in auto-save | `auto_save_default` ProfileData construction | Always read `self.state.engine_active` |
| One-way ratchet `if profile.engine_active { self.state.engine_active = true; }` in LoadProfile | `Intent::LoadProfile` handler | Unconditional assignment: `self.state.engine_active = profile.engine_active;` |
| Matching `CGEventType::Null` for tap-disabled signal | RELY-02 closure branch | Match the named variants `CGEventType::TapDisabledByTimeout \| CGEventType::TapDisabledByUserInput` (verified against core-graphics 0.24.0/src/event.rs:140-142) |

---

## Metadata

**Analog search scope:** All 7 files listed as fix targets; their own existing code is the reference
**Files read:** `src/App.tsx`, `src-tauri/src/scheduler/mod.rs`, `src-tauri/src/platform/macos/input.rs`, `src-tauri/src/platform/macos/observer.rs` (partial — lines 195–295), `src-tauri/src/platform/windows/mod.rs` (lines 1–217, 335–436), `src-tauri/src/state/mod.rs` (full), `src-tauri/src/persistence.rs` (full), `src-tauri/src/lib.rs` (full), `src-tauri/src/ipc/mod.rs` (lines 1–146), `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/core-graphics-0.24.0/src/event.rs` (CGEventType enum)
**Pattern extraction date:** 2026-05-16
**Revision pass:** 2026-05-16 — fixed LoadProfile one-way ratchet on `engine_active`; updated RELY-02 CGEventType variant from `Null` to `TapDisabledByTimeout|TapDisabledByUserInput` (verified against installed crate source)
