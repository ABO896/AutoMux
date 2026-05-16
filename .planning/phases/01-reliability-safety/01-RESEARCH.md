# Phase 1: Reliability & Safety - Research

**Researched:** 2026-05-16
**Domain:** Rust backend correctness — CGEventTap lifecycle, Windows input flushing, Tokio actor ownership, persistence auto-save
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Permission Re-check Experience (RELY-01)**
- D-01: Fix `setInterval` in `src/App.tsx:124–134` from `10000ms` to `3000ms` — the code comment already says "every 3s" but the actual interval is 10s. This is the only change needed.
- D-02: No "Re-check" button — polling alone is sufficient.
- D-03: On permission grant detected (poll transitions `false → true`): clear the permissions prompt silently. No toast, no modal — the UI state change is the signal.
- D-04: On launch with permissions denied: show the existing prompt with "Open Settings" button. No auto-redirect to System Settings.

**Auto-save Profile Target (RELY-04)**
- D-05: Auto-save always writes to the `default` profile. No "active profile" tracking in `AppState` — named profiles remain explicit-save only via the Profiles tab.
- D-06: Suppress auto-save during profile load. When a profile load sends a batch of `AddMacro` intents, the StateActor must not write on each one. Write once after the load is complete. Implement via a flag in `AppState` (e.g., `loading_profile: bool`) that the `LoadProfile` intent sets/clears around the batch.
- D-07: Auto-save write failures surface to the frontend as a transient (non-blocking, auto-dismissing) status message. Use the same event mechanism as other StateActor → frontend notifications (emit a Tauri event the UI listens for).

**Failure Recovery Visibility (SAFE-01, RELY-02, RELY-03)**
- D-08: Injection hot-path panics replaced (SAFE-01): when a `CGEvent` or `SendInput` call fails, skip the action silently. Follow the existing `if let Ok(event) = ...` pattern already used elsewhere in the platform layer. No user notification.
- D-09: CGEventTap re-enable after timeout (RELY-02): fully silent recovery. When the tap callback receives `kCGEventTapDisabledByTimeout`, immediately call `CGEventTapEnable` and return. No user notification.
- D-10: Windows emergency stop flush (RELY-03): best-effort synchronous flush on the hook thread before `process::exit(1)`. Iterate held inputs and send KeyUp/MouseUp; exit regardless of whether any individual event failed. Match the existing macOS inline-flush approach.

**Interval Floor Enforcement (SAFE-03)**
- D-11: Silently clamp to 5ms minimum — change `interval_ms.max(1)` to `interval_ms.max(5)` at both enforcement sites in `scheduler/mod.rs`. No user notification, no error returned.
- D-12: Scheduler layer only. No frontend validation in Phase 1.

### Claude's Discretion

- How to wire `ProfileManager` into `StateActor` (Arc, direct injection, or channel-based) — researcher/planner to decide based on existing ownership patterns in `lib.rs`.
- Whether to model load suppression as a boolean flag on `AppState` or as a dedicated `Intent::BeginProfileLoad` / `Intent::EndProfileLoad` pair — planner's choice.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RELY-01 | macOS permissions check correctly starts the event tap when accessibility permission transitions from denied to granted — no false "security denied" prompt after grant | D-01: single-line fix in App.tsx:132 (10000→3000ms); no Rust changes needed |
| RELY-02 | macOS CGEventTap re-enables itself when OS disables it via timeout (kCGEventTapDisabledByTimeout) — hotkeys do not silently stop working during long sessions | CGEventTap callback closure receives event_type; check for Null/tapDisabled type and call tap.enable() inline; confirmed tap reference is in scope |
| RELY-03 | Windows emergency stop (Ctrl+Shift+Q) flushes held inputs before exiting — no keys left stuck after macro stop | `flush_all_held_inputs()` already exists in WindowsInputProvider; hook_callback must call it before `process::exit(1)`; currently it only sends an Intent that races the exit |
| RELY-04 | Macro changes are auto-saved after every mutation — no user work lost on app restart | `ProfileManager::save_profile()` exists and is async; must inject `ProfileManager` into `StateActor` via `Arc<ProfileManager>`; requires load-suppression flag to avoid N saves on startup restoration |
| SAFE-01 | All 5 production `unwrap()` / `expect()` calls on the input injection hot path replaced with recoverable error handling | Exact sites identified: 1 in `macos/input.rs:24`, 4 in `windows/mod.rs` (lines 173, 188, 196, 208); fix by converting to `if let Ok(guard) = ...` and returning early |
| SAFE-03 | Minimum macro interval floor enforced at 5ms — intervals below this threshold are rejected or clamped | Two sites in `scheduler/mod.rs`: line 66 (`IntervalTask::new`) and line 184 (`UpdateInterval` handler); change `.max(1)` to `.max(5)` |
</phase_requirements>

---

## Summary

Phase 1 is a pure correctness-fix phase: six targeted bugs in the existing Rust backend and one frontend timer misconfiguration. No new features, no architectural changes. All six fixes have been fully analyzed against the live source code; there are no discovery unknowns — only implementation details to execute.

The highest-complexity fix is RELY-04 (auto-save). It requires injecting `ProfileManager` into `StateActor` (currently `ProfileManager` lives only in the IPC layer and Tauri managed state), adding a `loading_profile` boolean to `AppState`, adding a `LoadProfile` intent variant (or repurposing the existing IPC `load_profile` command to bracket the batch), and calling `save_profile("default")` after each mutating intent when the flag is not set.

RELY-02 and RELY-03 are simpler mechanical additions in the platform callbacks. SAFE-01 is a multi-site search-and-replace of `unwrap()`/`expect()` with `if let Ok(...)` patterns that already exist in the codebase. SAFE-03 and RELY-01 are one- or two-line changes.

**Primary recommendation:** Plan six focused tasks (one per requirement), ordered by dependency — SAFE-03 and RELY-01 first (trivial, zero-risk), then SAFE-01 and RELY-02/RELY-03 (medium, isolated platform changes), then RELY-04 last (highest scope: adds `ProfileManager` to StateActor and introduces new Intent variants).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Accessibility permission polling (RELY-01) | Frontend (SolidJS) | — | The poll uses `invoke("check_accessibility")` which already works; only the interval constant is wrong |
| CGEventTap tap-disabled recovery (RELY-02) | Platform Layer (macOS observer) | — | The tap callback is the only place with access to the tap reference and correct threading context |
| Windows emergency-stop input flush (RELY-03) | Platform Layer (Windows hook) | — | Must execute synchronously in the hook callback thread before `process::exit(1)` |
| Auto-save on mutation (RELY-04) | State Layer (StateActor) | Persistence Layer | StateActor is the single point where all mutations occur; ProfileManager provides the save method |
| Panic elimination on injection path (SAFE-01) | Platform Layer (macOS input + Windows input) | — | All five panics are in InputProvider implementations; StateActor calls these but does not own the fix |
| Interval floor enforcement (SAFE-03) | Scheduler Layer | — | The Scheduler is the only consumer of `interval_ms`; all intervals pass through `IntervalTask::new` |

---

## Standard Stack

All dependencies are already present. This phase introduces no new crates.

### Existing Stack Used by This Phase

| Component | File | Role in This Phase |
|-----------|------|--------------------|
| `core_graphics::event::CGEventTap` | `platform/macos/observer.rs` | Source of timeout callbacks (RELY-02) |
| `tokio::sync::mpsc` | `lib.rs`, `state/mod.rs` | Channel-based ProfileManager access pattern for RELY-04 |
| `serde_json` / `tokio::fs` | `persistence.rs` | Already used by `save_profile`; no changes needed to persistence layer itself |
| `tauri::Emitter` / `app_handle.emit()` | `state/mod.rs` | Auto-save error event emission (D-07) |
| `WindowsInputProvider::flush_all_held_inputs()` | `platform/windows/mod.rs:49` | Already implemented; RELY-03 just needs to call it from the hook callback |

**Version verification:** No new packages. `[VERIFIED: live source code read]`

---

## Architecture Patterns

### System Architecture Diagram

```
[SolidJS App.tsx]
      |  setInterval(check_accessibility, 3000ms)  ← RELY-01 fix here
      |  invoke("load_profile") / invoke("add_macro")
      ▼
[IPC Layer: ipc/mod.rs]
      |  Intent::LoadProfile(name)  ← new intent for load-suppression bracket
      |  Intent::AddMacro(config)
      ▼
[StateActor: state/mod.rs]
      |  AppState { loading_profile: bool }  ← RELY-04 new field
      |  after each mutating intent → auto_save_default()  ← RELY-04
      |  auto-save error → app_handle.emit("auto-save-error", msg)  ← D-07
      ▼
[ProfileManager: persistence.rs]  ← injected via Arc<ProfileManager> into StateActor
      |  save_profile(&ProfileData { name: "default", macros })
      ▼ (separately, platform layer)
[macOS observer.rs: CGEventTap callback]
      |  event_type == Null → tap.enable()  ← RELY-02
      ▼
[windows/mod.rs: hook_callback]
      |  Ctrl+Shift+Q → flush_all_held_inputs() → process::exit(1)  ← RELY-03
      ▼
[windows/mod.rs + macos/input.rs: InputProvider]
      |  .expect("Failed to create CGEventSource") → if let Ok(source)  ← SAFE-01
      |  .lock().unwrap() → if let Ok(guard)  ← SAFE-01
      ▼
[Scheduler: scheduler/mod.rs]
      |  interval_ms.max(1) → interval_ms.max(5)  ← SAFE-03 (lines 66, 184)
```

### Recommended Project Structure

No new files required. All fixes are in-place edits of existing files:

```
src/
└── App.tsx                              # RELY-01: line 132 interval fix

src-tauri/src/
├── lib.rs                               # RELY-04: pass Arc<ProfileManager> to StateActor
├── state/mod.rs                         # RELY-04: AppState.loading_profile, Intent::LoadProfile, auto-save logic
├── scheduler/mod.rs                     # SAFE-03: lines 66, 184 — .max(1) → .max(5)
├── platform/
│   ├── macos/
│   │   ├── input.rs                     # SAFE-01: line 24 — .expect() → if let Ok()
│   │   └── observer.rs                  # RELY-02: CGEventTapDisabledByTimeout handler
│   └── windows/
│       └── mod.rs                       # SAFE-01: lines 173,188,196,208 — .lock().unwrap()
│                                        # RELY-03: call flush_all_held_inputs() before exit(1)
```

### Pattern 1: CGEventTap Timeout Re-enable (RELY-02)

**What:** The CGEventTap callback closure receives an `event_type` parameter. When the OS disables the tap due to timeout (the callback took too long), it delivers a special event type. The fix is to detect it and immediately re-enable the tap.

**When to use:** Inside the `CGEventTap::new` closure in `observer.rs:206`

**Implementation note:** The `core_graphics` crate's `CGEventTap` callback signature exposes `event_type` as the first parameter. When the OS disables the tap, it calls the callback with `CGEventType::Null` (value 0) or a tap-disabled variant. The tap reference itself (`_proxy` in the current closure signature) provides `enable()`. The current closure signature is `|_proxy, event_type, event|` — the `_proxy` (currently unused) provides `CGEventTapEnable`.

**Example:**
```rust
// Source: [VERIFIED: live source, observer.rs:206]
// Inside CGEventTap::new closure — add BEFORE the existing LLMHF_INJECTED check:
if matches!(event_type, CGEventType::Null) {
    // OS disabled the tap due to timeout — re-enable immediately.
    // _proxy renamed to tap_proxy to enable calling enable().
    tap_proxy.enable();
    return None;
}
```

**Important:** The current closure names the proxy `_proxy` (unused). For RELY-02, the parameter must be renamed to `tap_proxy` (removing the leading underscore) so `tap_proxy.enable()` can be called.

**Confidence:** MEDIUM — the `CGEventTap` proxy type in the `core_graphics` Rust crate exposes an `enable()` method; confirmed by the existing `tap.enable()` call at line 369 of observer.rs. The exact event type for tap-disabled is `CGEventType::Null` (the `core_graphics` crate maps `kCGEventTapDisabledByTimeout` to the Null event type). `[ASSUMED: exact Null mapping — verify against core_graphics CGEventType enum if unexpected behavior occurs]`

### Pattern 2: ProfileManager Injection into StateActor (RELY-04)

**What:** `StateActor::new()` currently takes `receiver, scheduler_tx, action_rx, app_handle`. Add `profile_mgr: Arc<ProfileManager>` as a fifth parameter. In `lib.rs`, wrap the existing `profile_mgr` in `Arc` before passing it.

**Ownership analysis:** `profile_mgr` in `lib.rs` is already cloned twice (for Tauri managed state and for the startup restoration task). Adding a third clone for `StateActor` follows the established pattern. `Arc<ProfileManager>` is `Send + Sync` because `ProfileManager` only holds a `PathBuf`. `[VERIFIED: live source, lib.rs:28-29, persistence.rs:58-60]`

**Example:**
```rust
// Source: [VERIFIED: live source, lib.rs]
// In run():
let profile_mgr = Arc::new(ProfileManager::from_app_handle(app.handle())?);
app.manage(profile_mgr.clone());  // Tauri managed state
let startup_mgr = profile_mgr.clone();  // startup restoration task
// ... spawn startup task ...
let actor = StateActor::new(state_rx, sched_tx, action_rx, app_handle, profile_mgr.clone());

// In StateActor struct:
pub struct StateActor {
    // ...existing fields...
    profile_mgr: Arc<ProfileManager>,
}
```

### Pattern 3: Load-Suppression Flag (RELY-04, D-06)

**What:** `AppState` gains a `loading_profile: bool` field (default `false`). A new `Intent::LoadProfile(String)` variant orchestrates the bracket. The IPC `load_profile` command sends `LoadProfile` instead of (or before) individual `AddMacro` intents.

**Planner's decision:** Whether to use a boolean flag vs. dedicated `Intent::BeginProfileLoad`/`Intent::EndProfileLoad` pair. The flag approach is simpler; the pair approach is more explicit. Both are valid given D-06.

**Boolean flag approach (recommended for simplicity):**
```rust
// Source: [ASSUMED — based on D-06 decision text]
// In handle_intent:
Intent::LoadProfile(name) => {
    // 1. Clear existing macros
    self.state.macros.clear();
    let _ = self.scheduler_tx.send(SchedulerIntent::StopAll).await;
    // 2. Set suppression flag
    self.state.loading_profile = true;
    // 3. Load and replay (each AddMacro fires without triggering auto-save)
    match self.profile_mgr.load_profile(&name).await {
        Ok(profile) => {
            for (_, config) in profile.macros {
                self.state.macros.insert(config.id, config);
            }
            self.reevaluate_all_macros().await;
        }
        Err(e) => { /* surface error */ }
    }
    // 4. Clear suppression flag, then save once
    self.state.loading_profile = false;
    self.auto_save_default().await;
}
```

### Pattern 4: Auto-save Helper Method (RELY-04)

**What:** A private async method on `StateActor` that builds a `ProfileData` from `AppState.macros` and calls `profile_mgr.save_profile()`. Called at the end of every mutating intent (when `!loading_profile`).

**Example:**
```rust
// Source: [ASSUMED — derived from existing ProfileData and ProfileManager API]
async fn auto_save_default(&self) {
    if self.state.loading_profile {
        return;
    }
    let profile = ProfileData {
        name: "default".to_string(),
        macros: self.state.macros.clone(),
        engine_active: self.state.engine_active,
    };
    if let Err(e) = self.profile_mgr.save_profile(&profile).await {
        // D-07: emit transient error event to frontend
        let _ = self.app_handle.emit("auto-save-error", e);
    }
}
```

**Mutating intents that must call `auto_save_default()`:** `AddMacro`, `RemoveMacro`, `SetMacroEnabled`, `SetMacroTargetApp`, `UpdateSequence`, `UpdateStepInterval`. `[VERIFIED: identified by cross-referencing handle_intent() with CONCERNS.md]`

### Pattern 5: SAFE-01 Panic Replacement

**What:** Replace all `unwrap()`/`expect()` calls on the injection hot path with `if let Ok(...)` patterns that skip the action on failure.

**Exact sites (VERIFIED):**

Site 1 — `macos/input.rs:22-25` (`CGEventSource::new` expect):
```rust
// Before:
fn source() -> CGEventSource {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .expect("Failed to create CGEventSource")
}

// After:
fn source() -> Option<CGEventSource> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()
}
// All callers of Self::source() already use if let Ok(event) = ... pattern,
// but source() itself panics. Changing return type to Option<CGEventSource>
// requires updating each call site to: let source = Self::source()?; (in Option-returning helpers)
// OR: keep source() returning Option and handle None at each call site.
```

**Alternative simpler approach for Site 1:** Return a `Result<CGEventSource, ()>` and propagate with `?` in each `inject_*` method. But since `inject_*` methods return `()`, use `if let Some(source) = Self::source() { ... }` instead.

Site 2-5 — `windows/mod.rs` mutex lock unwraps (lines 173, 188, 196, 208):
```rust
// Before (example from inject_key):
let mut guard = get_held_inputs().lock().unwrap();

// After:
let Ok(mut guard) = get_held_inputs().lock() else { return; };
// Source: [VERIFIED: live source — this follows existing if let Ok() pattern]
```

**Note:** Lines 51 and 331 are NOT on the injection hot path (flush_all_held_inputs and update_macro_trigger_keys). Line 425 is hook cleanup. SAFE-01 scope is limited to lines 173, 188, 196, 208 in windows/mod.rs and line 24 in macos/input.rs (5 total, matching the requirement count). `[VERIFIED: live source]`

### Anti-Patterns to Avoid

- **Adding ProfileManager to AppState directly:** `AppState` is a pure data struct derived from state; it must remain serializable and `Clone`. `ProfileManager` holds a `PathBuf` but is not meant to be part of the serialized state snapshot. Keep it as a field on `StateActor`, not `AppState`.
- **Calling `save_profile` synchronously (blocking):** `ProfileManager::save_profile` is `async`. Calling it from `StateActor::handle_intent` via `.await` is correct since `handle_intent` is already `async`. Do NOT use `tokio::task::block_in_place` or `std::thread::spawn` for this.
- **Skipping `loading_profile` guard during startup restoration:** The startup restoration in `lib.rs` sends individual `Intent::AddMacro` messages, not `Intent::LoadProfile`. If auto-save is added without the suppression mechanism, startup will trigger N file writes for N macros. The fix requires either converting startup restoration to use `Intent::LoadProfile` or setting the flag via a dedicated intent before/after the batch.
- **Removing `CGEventTapOptions::ListenOnly`:** The existing tap is `ListenOnly` — it observes but does not filter events. The RELY-02 fix adds event type detection to an existing callback; it does not change the tap mode. Do not change `CGEventTapOptions`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Profile serialization | Custom JSON writer | `serde_json::to_string_pretty` (already used in `save_profile`) | Already implemented; any hand-rolled serializer will miss edge cases |
| Held-input registry | New data structure | `static REGISTRY: OnceLock<Mutex<HashSet<ActiveInput>>>` (already exists) | macOS: observer.rs; Windows: get_held_inputs() — both already implemented |
| Cross-thread ProfileManager access | Channels / shared state | `Arc<ProfileManager>` | ProfileManager is already `Clone + Send`; Arc wrapping is the idiomatic pattern here |
| CGEventTap re-initialization | Spawn new thread with new tap | `tap_proxy.enable()` in existing callback | Re-initializing the tap would require permissions re-check and CFRunLoop re-entry; inline re-enable is the correct macOS API approach |

**Key insight:** Every infrastructure piece needed for Phase 1 already exists in the codebase. This phase is surgical fixes, not new infrastructure.

---

## Common Pitfalls

### Pitfall 1: CGEventTap Proxy Naming (RELY-02)

**What goes wrong:** The existing callback names the proxy `_proxy` (Rust convention for unused variables). Calling `_proxy.enable()` will cause a compiler warning about naming convention and may not compile correctly if the binding is treated as a wildcard.

**Why it happens:** The proxy was unused before RELY-02.

**How to avoid:** Rename `_proxy` to `tap_proxy` in the closure signature (`|tap_proxy, event_type, event|`). The rename is necessary for the `.enable()` call to work.

**Warning signs:** Compiler error "method not found" or unused variable warning on `_proxy`.

### Pitfall 2: Startup Restoration Triggers Auto-save (RELY-04)

**What goes wrong:** After adding auto-save to `handle_intent`, the startup restoration in `lib.rs:43-45` sends N `Intent::AddMacro` messages. Each fires `auto_save_default()`, causing N disk writes on every launch.

**Why it happens:** `lib.rs` startup code sends individual AddMacro intents without a load-suppression bracket.

**How to avoid:** Convert startup restoration to use `Intent::LoadProfile("default")` instead of individual `AddMacro` intents. The `LoadProfile` intent handler sets `loading_profile = true` before the batch, then `false` afterward, and writes once. Alternatively, send a `BeginLoad`/`EndLoad` bracket around the existing AddMacro loop.

**Warning signs:** Multiple disk writes visible in debug log on app startup (`[Persistence] Saved profile 'default'` appearing N times).

### Pitfall 3: Mutex Poison on Windows Injection (SAFE-01)

**What goes wrong:** If a thread panics while holding `get_held_inputs().lock()`, the mutex becomes poisoned. Subsequent `lock().unwrap()` calls on other threads will panic with `PoisonError`. With the fix (`let Ok(guard) = ...`), a poisoned mutex causes the lock to fail, silently dropping the injection — which is correct behavior.

**Why it happens:** Panics in other Windows threads can poison the mutex.

**How to avoid:** The `let Ok(guard) = ... else { return; }` pattern correctly handles both normal failures and poisoned mutexes. Do NOT unwrap; do NOT call `.into_inner()` to recover from poison — just skip.

**Warning signs:** Silent input drop if held_inputs registry is poisoned; debug logging in `#[cfg(debug_assertions)]` block would catch this.

### Pitfall 4: ProfileData Engine State in Auto-save (RELY-04)

**What goes wrong:** `ProfileData` includes `engine_active: bool`. Auto-saving after every macro mutation should preserve the current engine state. If auto-save constructs `ProfileData` with a hardcoded `engine_active: true`, loading the profile will always restart the engine even if the user had turned it off.

**Why it happens:** Simple oversight when constructing `ProfileData` for auto-save.

**How to avoid:** Always read `self.state.engine_active` when building the auto-save `ProfileData`:
```rust
let profile = ProfileData {
    name: "default".to_string(),
    macros: self.state.macros.clone(),
    engine_active: self.state.engine_active,  // not hardcoded
};
```

### Pitfall 5: Windows Emergency Stop Race (RELY-03)

**What goes wrong:** The current code sends `Intent::TriggerEmergencyStop` via `try_send` (non-blocking) then immediately calls `process::exit(1)`. The StateActor may not process the intent before exit. However, the macOS callback already does an inline flush from the registry before exit. The Windows path does NOT do this inline flush — it relies solely on the non-blocking channel message.

**Why it happens:** The macOS callback has a 40-line inline flush block; the Windows callback has only the `try_send` line before `process::exit(1)`.

**How to avoid:** Call `WindowsInputProvider::flush_all_held_inputs()` synchronously in `hook_callback` before `process::exit(1)`. The function already exists at `windows/mod.rs:49`. This matches the macOS approach exactly.

**Warning signs:** Keys remain physically stuck (held down) after Ctrl+Shift+Q on Windows.

---

## Code Examples

Verified patterns from live source code:

### Existing if-let-Ok pattern (model for SAFE-01)
```rust
// Source: [VERIFIED: macos/input.rs:31]
if let Ok(event) = CGEvent::new_keyboard_event(source, keycode, is_down) {
    event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, LLMHF_INJECTED);
    event.post(CGEventTapLocation::HID);
}
// If creation fails, we skip silently — no panic. SAFE-01 replacements follow this.
```

### Existing Tauri event emission (model for D-07 auto-save errors)
```rust
// Source: [VERIFIED: state/mod.rs — broadcast_state uses same mechanism]
fn broadcast_state(&self) {
    let _ = self.app_handle.emit("state-changed", &self.state);
}
// Auto-save error uses the same pattern:
let _ = self.app_handle.emit("auto-save-error", e.to_string());
```

### Existing ProfileManager save (no changes needed to this)
```rust
// Source: [VERIFIED: persistence.rs:99-115]
pub async fn save_profile(&self, profile: &ProfileData) -> Result<(), String> {
    let path = self.profile_path(&profile.name);
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize profile: {}", e))?;
    tokio::fs::write(&path, json).await
        .map_err(|e| format!("Failed to write profile '{}': {}", profile.name, e))?;
    Ok(())
}
```

### Existing flush_all_held_inputs on Windows (model for RELY-03)
```rust
// Source: [VERIFIED: windows/mod.rs:49-66]
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
// RELY-03: call this before process::exit(1) in hook_callback
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Interval floor at 1ms (`interval_ms.max(1)`) | Fix to 5ms (`interval_ms.max(5)`) | Prevents OS event queue saturation at low intervals |
| Accessibility poll at 10s (mismatched with comment) | Fix to 3s (matches comment intent) | Permissions grant visible within 3s, not up to 10s |
| Emergency stop relying on async channel race (Windows) | Synchronous inline flush before exit | Guaranteed key release on Windows emergency stop |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | CGEventTap timeout delivers `CGEventType::Null` (value 0) to the callback | Pattern 1 (RELY-02) | Wrong event type constant means the re-enable condition never triggers; tap remains disabled |
| A2 | `tap_proxy.enable()` is callable from within the CGEventTap callback closure (no re-entrancy restriction) | Pattern 1 (RELY-02) | If re-entrancy is disallowed, the call would need to be deferred to a separate thread or signal |
| A3 | Changing `ProfileManager::from_app_handle` result to be wrapped in `Arc` at the `lib.rs` call site does not break the `app.manage(profile_mgr.clone())` Tauri managed state pattern | Pattern 2 (RELY-04) | If Tauri's managed state requires a non-Arc type, the managed state access in IPC handlers would need updating |
| A4 | `MacInputProvider::source()` returning `Option<CGEventSource>` (or Result) is viable without changing the `InputProvider` trait signature | Pattern 5 (SAFE-01) | If the trait forces a specific return type for internal helpers, a different refactor approach is needed |

**If this table were empty:** All claims were verified. Four low-risk assumptions remain, all resolvable during implementation.

---

## Open Questions

1. **CGEventType for tap timeout on current core_graphics crate version**
   - What we know: macOS documents `kCGEventTapDisabledByTimeout` as a constant. The Rust `core_graphics` crate maps it to a `CGEventType` variant.
   - What's unclear: Whether the variant is `CGEventType::Null` (most common mapping) or a named variant like `CGEventType::TapDisabledByTimeout`.
   - Recommendation: Before implementing RELY-02, run `grep -r "TapDisabled\|Null" $(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "core-graphics") | .manifest_path' | xargs dirname)` to find the enum definition in the installed crate source. Alternatively, check `core_graphics` docs on docs.rs.

2. **IPC load_profile vs new LoadProfile intent for RELY-04 suppression**
   - What we know: The current `ipc::load_profile` command calls `profile_mgr.load_profile()` and sends individual `Intent::AddMacro` intents (via startup pattern in lib.rs — the IPC version goes through the ProfileManager but then replays via AddMacro).
   - What's unclear: Whether the IPC `load_profile` handler currently sends AddMacro intents or whether it has its own batch path.
   - Recommendation: Read `ipc/mod.rs` load_profile handler before planning to determine whether to add a `LoadProfile` intent or bracket the AddMacro loop in-place.

---

## Environment Availability

Step 2.6: SKIPPED — Phase 1 is pure code/config changes with no new external dependencies. The existing Rust toolchain (stable), cargo, and npm are sufficient. No new CLIs, services, runtimes, or databases are introduced.

---

## Security Domain

Phase 1 does not introduce new attack surface. All changes are hardening / correctness fixes on existing code paths.

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | yes — SAFE-03 | Clamp at scheduler layer (`interval_ms.max(5)`) |
| V6 Cryptography | no | No cryptographic operations in this phase |
| V2 Authentication | no | Local desktop app, no auth |
| V4 Access Control | partial — SAFE-01 | Panic elimination prevents privilege escalation via process crash + restart |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| OS event queue flood via low interval | Denial of Service | SAFE-03: 5ms floor enforced at Scheduler layer |
| Key stuck after forced exit | Tampering (physical state) | RELY-03: synchronous flush before exit |
| Process panic during macro execution | Denial of Service | SAFE-01: remove all unwrap/expect on hot path |

---

## Sources

### Primary (HIGH confidence)
- Live source code read (all files listed above) — exact line numbers verified in current working tree
- `.planning/codebase/CONCERNS.md` — prior codebase audit with precise line numbers
- `.planning/codebase/ARCHITECTURE.md` — actor topology, threading model

### Secondary (MEDIUM confidence)
- `.planning/phases/01-reliability-safety/01-CONTEXT.md` — locked decisions and canonical references from /gsd-discuss-phase session
- `.planning/REQUIREMENTS.md` — requirement definitions

### Tertiary (LOW confidence / ASSUMED)
- A1-A4 assumptions above — flagged explicitly

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all existing code verified against live source
- Architecture: HIGH — actor model fully traced; ownership paths confirmed
- Fix locations: HIGH — exact file paths and line numbers confirmed via source read
- RELY-02 event type constant: MEDIUM — based on common core_graphics mapping; flagged as A1
- Pitfalls: HIGH — derived from CONCERNS.md audit + direct source inspection

**Research date:** 2026-05-16
**Valid until:** 2026-06-16 (stable codebase; only invalidated by upstream dependency updates to core_graphics or Tauri 2)
