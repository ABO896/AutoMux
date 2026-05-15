<!-- refreshed: 2026-05-15 -->
# Architecture

**Analysis Date:** 2026-05-15

## System Overview

```text
┌─────────────────────────────────────────────────────────────────┐
│                  SolidJS Frontend (WebView)                      │
│                    `src/App.tsx`                                  │
│   invoke() calls ──────────────────── listen("state-changed")   │
└────────────────────────┬────────────────────────────────────────┘
                         │ Tauri IPC bridge
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   IPC Command Layer                              │
│              `src-tauri/src/ipc/mod.rs`                         │
│  add_macro | remove_macro | set_macro_enabled | toggle_engine   │
│  save_profile | load_profile | list_profiles | get_state ...    │
└──────────────┬──────────────────────────────────────────────────┘
               │ Intent enum via mpsc channel
               ▼
┌──────────────────────────────────────────────────────────────┐
│              StateActor (single async task)                   │
│              `src-tauri/src/state/mod.rs`                     │
│  - Owns AppState (macros HashMap, engine flags, active_app)  │
│  - Processes Intents, routes to Scheduler                    │
│  - Validates targeting before injecting input (Gate 1-3)     │
│  - Broadcasts state-changed events to frontend               │
└────────┬─────────────────────────┬───────────────────────────┘
         │ SchedulerIntent          │ ActionReady (two-phase)
         ▼                          │
┌────────────────────────┐          │
│  Scheduler (single     │──────────┘
│  async task)           │
│  `src-tauri/src/       │
│   scheduler/mod.rs`    │
│  - BTreeMap timeline   │
│  - Per-step timers     │
│  - No input injection  │
└────────────────────────┘
         │
         │ (separately)
         ▼
┌─────────────────────────────────────────────────────────────┐
│              Platform Layer                                  │
│         `src-tauri/src/platform/`                            │
│  InputProvider trait  │  PlatformObserver trait             │
│  macos/input.rs       │  macos/observer.rs                  │
│  windows/mod.rs       │  windows/mod.rs                     │
│  (CGEvent / SendInput)│  (CGEventTap / Win32 hooks)         │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│              Persistence Layer                               │
│         `src-tauri/src/persistence.rs`                       │
│  ProfileManager — JSON files in OS app data dir             │
│  macOS: ~/Library/Application Support/com.alvaro.automux/   │
│  Windows: %APPDATA%/com.alvaro.automux/profiles/            │
└─────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| SolidJS App | UI rendering, invoke calls, event listening | `src/App.tsx` |
| IPC Layer | Translate Tauri commands to Intent messages | `src-tauri/src/ipc/mod.rs` |
| StateActor | Own AppState, process Intents, gate input injection | `src-tauri/src/state/mod.rs` |
| StateManager | Thread-safe handle for sending Intents | `src-tauri/src/state/mod.rs` |
| Scheduler | Timer-based action firing, no input injection | `src-tauri/src/scheduler/mod.rs` |
| Platform (macOS) | CGEvent input injection + CGEventTap observer | `src-tauri/src/platform/macos/` |
| Platform (Windows) | SendInput injection + Win32 hook observer | `src-tauri/src/platform/windows/mod.rs` |
| ProfileManager | JSON profile save/load/list/delete | `src-tauri/src/persistence.rs` |
| AppState | In-memory macro registry + engine flags | `src-tauri/src/state/mod.rs` |

## Pattern Overview

**Overall:** Actor model with two-phase dispatch and platform abstraction traits.

**Key Characteristics:**
- Single `StateActor` async task owns all mutable state — no shared mutexes on the hot path
- Single `Scheduler` async task owns all timers — O(s) memory where s = active interval steps
- Two-phase dispatch: Scheduler fires `ActionReady` → StateActor validates targeting → injects
- Platform abstraction via `InputProvider` and `PlatformObserver` traits with per-OS implementations
- Frontend receives state via Tauri events (`state-changed`); all mutations go through `invoke()`

## Layers

**Frontend (SolidJS):**
- Purpose: User interface only — renders state, calls backend commands
- Location: `src/`
- Contains: Single `App.tsx` component with all UI, SolidJS signals for reactive state
- Depends on: Tauri JS API (`@tauri-apps/api/core` invoke, `@tauri-apps/api/event` listen)
- Used by: End user via Tauri WebView

**IPC Layer:**
- Purpose: Thin translation from Tauri `#[command]` invocations to `Intent` enum messages
- Location: `src-tauri/src/ipc/mod.rs`
- Contains: One async function per IPC command, all delegating to `StateManager::send_intent()`
- Depends on: `StateManager`, `state::Intent`, `persistence::ProfileManager`
- Used by: Tauri `invoke_handler!` macro registration in `lib.rs`

**State Layer:**
- Purpose: Authoritative runtime state, Intent processing, input injection gating
- Location: `src-tauri/src/state/mod.rs`
- Contains: `AppState`, `MacroConfig`, `ActionSequence`, `Intent` enum, `StateActor`, `StateManager`
- Depends on: `scheduler::SchedulerIntent`, `platform::InputProvider`, `tauri::Emitter`
- Used by: IPC layer (writes via Intent), Scheduler (reads ActionReady back to it)

**Scheduler Layer:**
- Purpose: Pure timer management — fires `ActionReady` signals, never injects input
- Location: `src-tauri/src/scheduler/mod.rs`
- Contains: `Scheduler`, `SchedulerIntent`, `ActionReady`, `ActionType`, `IntervalTask`
- Depends on: `state::ActionStep`, `state::MacroConfig`
- Used by: StateActor (sends `SchedulerIntent`, receives `ActionReady`)

**Platform Layer:**
- Purpose: OS-specific input injection and active-app observation
- Location: `src-tauri/src/platform/`
- Contains: `InputProvider` trait, `PlatformObserver` trait, macOS impl, Windows impl
- Depends on: CoreGraphics (macOS), Win32 (Windows)
- Used by: StateActor (injection), `lib.rs` (observer startup)

**Persistence Layer:**
- Purpose: Save/load named macro profiles as JSON files
- Location: `src-tauri/src/persistence.rs`
- Contains: `ProfileManager`, `ProfileData`, `ProfileSummary`
- Depends on: `state::MacroConfig`, `serde_json`, `tokio::fs`
- Used by: IPC layer (profile commands), `lib.rs` (startup restoration)

## Data Flow

### Primary: User Creates a Macro

1. User fills form in frontend → `handleCreateMacro()` calls `invoke("add_macro", { config })` (`src/App.tsx:184`)
2. Tauri routes to `ipc::add_macro` command (`src-tauri/src/ipc/mod.rs:6`)
3. IPC sends `Intent::AddMacro(config)` via `StateManager::send_intent()` (`src-tauri/src/ipc/mod.rs:8`)
4. `StateActor::handle_intent()` inserts macro into `AppState.macros` and calls `reevaluate_all_macros()` (`src-tauri/src/state/mod.rs:273`)
5. If macro is enabled and target app matches, `SchedulerIntent::StartMacro` is sent to Scheduler (`src-tauri/src/state/mod.rs:381`)
6. `StateActor::broadcast_state()` emits `state-changed` event via `app_handle.emit()` (`src-tauri/src/state/mod.rs:402-405`)
7. Frontend `listen("state-changed")` handler updates SolidJS signals → UI re-renders (`src/App.tsx:115`)

### Primary: Macro Fires an Action (Two-Phase Dispatch)

1. `Scheduler::fire_due_actions()` detects timer expiry for an `IntervalTask` (`src-tauri/src/scheduler/mod.rs:292`)
2. Scheduler sends `ActionReady { macro_id, action_type: ActionType::Interval(input), ... }` via `action_tx` (`src-tauri/src/scheduler/mod.rs:301`)
3. `StateActor::run()` receives from `action_rx` with `biased` priority 2 (`src-tauri/src/state/mod.rs:185`)
4. `StateActor::handle_action()` checks three gates: engine active, macro enabled, target app matches (`src-tauri/src/state/mod.rs:198-219`)
5. If all gates pass, `inject_input()` is called → `InputProvider::inject_key()` or `inject_mouse_button_raw()` (`src-tauri/src/state/mod.rs:241`)
6. Platform layer posts the OS event (CGEvent on macOS, SendInput on Windows)

### Platform Event: Hotkey Pressed

1. macOS `CGEventTap` callback fires in background thread (`src-tauri/src/platform/macos/observer.rs`)
2. Observer matches keycode + modifiers against `HOTKEY_BINDINGS` static registry
3. Sends `Intent::ToggleMacroHotkey(uuid)` or `Intent::ToggleEngineHotkey` via static `STATE_TX`
4. StateActor processes the Intent — same path as IPC intents

### Platform Event: Active App Changed

1. macOS `NSWorkspace` notification or Windows hook fires → observer calls `set_active_app(bundle_id)` 
2. Sends `Intent::ActiveAppChanged(Some(bundle_id))` via `STATE_TX`
3. StateActor updates `AppState.active_app` and calls `reevaluate_all_macros()` → Scheduler starts/stops per-app macros

### Startup Restoration

1. `lib.rs::run()` initializes `ProfileManager` (`src-tauri/src/lib.rs:28`)
2. Async task calls `ProfileManager::load_or_create_default()` (`src-tauri/src/lib.rs:35-46`)
3. Each `MacroConfig` in the default profile is sent as `Intent::AddMacro` to the StateActor
4. StateActor processes them as normal macro additions

**State Management:**
- All runtime state lives in `StateActor.state: AppState` (single-owner, no shared mutexes on hot path)
- Static `OnceLock<Mutex<...>>` used in platform observer for cross-thread registries (hotkey bindings, active app, registry of held inputs)
- Frontend state is a read-only projection: StateActor pushes full `AppState` on every change

## Key Abstractions

**`InputProvider` trait:**
- Purpose: Platform-agnostic interface for injecting OS input events
- Implementations: `MacInputProvider` (`platform/macos/input.rs`), `WindowsInputProvider` (`platform/windows/mod.rs`)
- Methods: `inject_key`, `inject_mouse_click`, `inject_mouse_move`, `inject_mouse_button_raw`, `flush_held_inputs`
- Pattern: Stateless struct; new `CGEventSource` created per call on macOS for thread safety

**`PlatformObserver` trait:**
- Purpose: Platform-agnostic interface for observing active app changes
- Implementations: `MacPlatformObserver` (`platform/macos/observer.rs`), `WindowsPlatformObserver` (`platform/windows/mod.rs`)
- Pattern: Long-lived struct started once at app init; pushes Intents via static `STATE_TX`

**`Intent` enum:**
- Purpose: All mutations to `AppState` are expressed as Intent variants — no direct field access from outside `StateActor`
- Location: `src-tauri/src/state/mod.rs:118`
- Used by: IPC layer, platform observers (via static channel), hotkey callbacks

**`MacroConfig` / `ActionSequence` / `ActionStep`:**
- Purpose: The core data model shared between frontend (TypeScript mirror), StateActor, Scheduler, and persistence
- `ActionSequence` contains `Vec<ActionStep>` where each step is either `SustainedHold` or `InterleavedInterval`
- Legacy compatibility: empty `sequence` falls back to single left-click at `interval_ms`

**`SchedulerIntent` / `ActionReady`:**
- Purpose: Message types for the two-way channel between StateActor and Scheduler
- `SchedulerIntent`: StateActor → Scheduler (start/stop/update macros)
- `ActionReady`: Scheduler → StateActor (fire this action now)

## Entry Points

**Rust binary entry:**
- Location: `src-tauri/src/main.rs:4`
- Triggers: OS process launch
- Responsibilities: Calls `automux_lib::run()` with Windows subsystem annotation

**Tauri app setup (`lib.rs::run`):**
- Location: `src-tauri/src/lib.rs:19`
- Triggers: Called from `main()`
- Responsibilities: Creates all mpsc channels, spawns Scheduler and StateActor tasks, initializes ProfileManager, starts platform observer, registers IPC handlers

**Frontend entry:**
- Location: `src/index.tsx`
- Triggers: WebView load
- Responsibilities: Mounts `<App />` SolidJS component into `#root`

**IPC commands (16 registered):**
- Location: `src-tauri/src/ipc/mod.rs`
- Triggers: `invoke()` calls from frontend
- Commands: `add_macro`, `remove_macro`, `set_macro_enabled`, `set_macro_target_app`, `get_state`, `get_active_app`, `bind_hotkey`, `unbind_hotkey`, `toggle_engine`, `request_accessibility`, `check_accessibility`, `set_macro_sequence`, `update_step_interval`, `save_profile`, `load_profile`, `delete_profile`, `list_profiles`

## Architectural Constraints

- **Threading:** Two long-lived Tokio async tasks (StateActor, Scheduler) communicate via bounded mpsc channels. Platform observer runs on a separate OS thread (CGEventTap requires CFRunLoop). Input injection is always synchronous in the StateActor task.
- **Global state:** Static `OnceLock` singletons in `platform/macos/observer.rs` — `STATE_TX`, `TAP_INITIALIZED`, `REGISTRY`, `ACTIVE_APP`, `HOTKEY_BINDINGS`. These are initialized once and never mutated structurally after init.
- **Circular imports:** None detected. Dependency direction is: `ipc` → `state` ← `scheduler` ← `state`; `platform` is a leaf depended on by `state`.
- **Input injection site:** The ONLY place input is injected is `StateActor::inject_input()` (`state/mod.rs:241`). The Scheduler never injects directly — this is enforced structurally (Scheduler holds only the `action_tx` sender, never an `InputProvider`).
- **Backpressure:** `action_tx.try_send()` is used in the Scheduler (non-blocking); overflow is silently dropped rather than blocking the timer loop.

## Anti-Patterns

### Bypassing the Intent channel

**What happens:** Calling `StateActor` fields or methods directly from IPC without going through `StateManager::send_intent()`.
**Why it's wrong:** Breaks the single-owner invariant; introduces data races on `AppState`.
**Do this instead:** All state mutations must be sent as `Intent` variants via `StateManager` (`src-tauri/src/ipc/mod.rs:8`).

### Injecting input from the Scheduler

**What happens:** Giving the `Scheduler` access to an `InputProvider` and calling inject methods directly from `fire_due_actions()`.
**Why it's wrong:** Bypasses the three-gate targeting check in `StateActor::handle_action()`; macros would fire on wrong apps and ignore engine/emergency-stop flags.
**Do this instead:** Scheduler sends `ActionReady`; `StateActor::handle_action()` validates and injects (`src-tauri/src/state/mod.rs:198`).

### Per-macro spawned tasks

**What happens:** Spawning a separate `tokio::spawn` for each active macro's timer loop.
**Why it's wrong:** Unbounded task creation proportional to macro count; significant overhead at scale; was the v1 design that was replaced.
**Do this instead:** Use the single `Scheduler` task with the `BTreeMap` timeline — all timers share one event loop (`src-tauri/src/scheduler/mod.rs:127`).

## Error Handling

**Strategy:** `Result<T, String>` throughout — IPC commands return `Result<_, String>` serialized over the Tauri bridge. Async channel errors are silently ignored with `let _ = sender.send(...).await` on non-critical paths.

**Patterns:**
- IPC commands return `Err(e.to_string())` for all failures; frontend catches with `try/catch` on `invoke()`
- Persistence errors surface as `Err(String)` returned from `ProfileManager` methods
- Platform errors (CGEvent creation failures) are silently dropped with `if let Ok(event) = ...`
- Emergency stop path (`flush_held_inputs`) is best-effort — does not propagate errors

## Cross-Cutting Concerns

**Logging:** `eprintln!()` guarded by `#[cfg(debug_assertions)]` only. No structured logging framework. Prefixed with `[Component]` tag (e.g., `[Startup]`, `[Action]`, `[Persistence]`).
**Validation:** Profile name sanitization in `ProfileManager::sanitize_name()` strips path traversal characters. Input validation for macro config is minimal — left to frontend forms.
**Authentication:** Not applicable (local desktop app, no network auth).

---

*Architecture analysis: 2026-05-15*
