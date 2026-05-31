# Codebase Structure

**Analysis Date:** 2026-05-15

## Directory Layout

```
automux/                         # Project root
├── src/                         # SolidJS frontend (TypeScript)
│   ├── index.tsx                # WebView entry point — mounts <App />
│   ├── App.tsx                  # Entire UI: all components, signals, IPC calls
│   ├── App.css                  # Global CSS variables and component styles
│   ├── vite-env.d.ts            # Vite/TypeScript ambient declarations
│   └── assets/
│       └── logo.svg             # App logo asset
│
├── src-tauri/                   # Rust backend (Tauri app)
│   ├── src/
│   │   ├── main.rs              # Binary entry — calls automux_lib::run()
│   │   ├── lib.rs               # App setup: channels, actors, IPC registration
│   │   ├── ipc/
│   │   │   └── mod.rs           # All 17 Tauri #[command] handlers
│   │   ├── state/
│   │   │   └── mod.rs           # AppState, MacroConfig, Intent, StateActor, StateManager
│   │   ├── scheduler/
│   │   │   └── mod.rs           # Scheduler, SchedulerIntent, ActionReady, IntervalTask
│   │   ├── platform/
│   │   │   ├── mod.rs           # InputProvider + PlatformObserver traits; platform gating
│   │   │   ├── macos/
│   │   │   │   ├── mod.rs       # check_accessibility_permissions(); re-exports
│   │   │   │   ├── input.rs     # MacInputProvider — CGEvent injection
│   │   │   │   └── observer.rs  # CGEventTap, NSWorkspace observer, hotkey registry
│   │   │   └── windows/
│   │   │       └── mod.rs       # WindowsInputProvider + WindowsPlatformObserver (SendInput)
│   │   └── persistence.rs       # ProfileManager, ProfileData, ProfileSummary
│   ├── capabilities/            # Tauri capability permission files
│   └── icons/                   # App icons (macOS, Windows, Android, iOS sizes)
│
├── .planning/                   # GSD planning documents
│   └── codebase/                # Codebase map documents (this file's parent)
│
├── .github/
│   └── workflows/
│       └── release.yml          # CI/CD release workflow
│
├── archive/
│   └── v1-production/           # Archived v1 codebase artifacts (historical only)
│
├── docs/                        # Additional documentation
├── public/                      # Static assets served by Vite
│
├── index.html                   # Vite HTML entry point
├── vite.config.ts               # Vite + SolidJS + Tailwind config; dev server port 1420
├── package.json                 # Frontend dependencies (solid-js, tailwindcss, tauri)
├── package-lock.json            # npm lockfile
├── tsconfig.json                # TypeScript config for src/
└── tsconfig.node.json           # TypeScript config for vite.config.ts
```

## Directory Purposes

**`src/`:**
- Purpose: Entire SolidJS frontend — all UI code lives here
- Contains: Single-component architecture (`App.tsx`), CSS variables, TypeScript types mirroring Rust state
- Key files: `src/App.tsx` (all UI logic), `src/index.tsx` (mount point)

**`src-tauri/src/`:**
- Purpose: All Rust backend logic
- Contains: Five modules: `ipc`, `state`, `scheduler`, `platform`, `persistence`
- Key files: `src-tauri/src/lib.rs` (app wiring), `src-tauri/src/state/mod.rs` (core actor)

**`src-tauri/src/platform/`:**
- Purpose: OS-specific implementations behind shared traits
- Contains: `InputProvider` and `PlatformObserver` implementations per OS
- Pattern: `#[cfg(target_os = "macos")]` / `#[cfg(target_os = "windows")]` gates at module boundaries

**`src-tauri/src/ipc/`:**
- Purpose: Thin IPC translation layer — no business logic, only Intent dispatch
- Contains: One `pub async fn` per registered Tauri command
- Pattern: Every function follows `StateManager::send_intent(Intent::Variant(...)).await.map_err(|e| e.to_string())`

**`archive/v1-production/`:**
- Purpose: Historical v1 artifacts — do not reference or import from active code
- Generated: No
- Committed: Yes (for reference only)

## Key File Locations

**Entry Points:**
- `src-tauri/src/main.rs`: Rust binary entry — calls `automux_lib::run()`
- `src-tauri/src/lib.rs`: App wiring — creates all mpsc channels, spawns actors, registers commands
- `src/index.tsx`: Frontend mount — renders `<App />` into `#root`

**Configuration:**
- `vite.config.ts`: Vite dev server (port 1420), SolidJS plugin, Tailwind plugin
- `package.json`: Frontend dependencies and build scripts
- `src-tauri/capabilities/`: Tauri permission capability files

**Core Logic:**
- `src-tauri/src/state/mod.rs`: `AppState`, `Intent` enum, `StateActor` — the heart of the backend
- `src-tauri/src/scheduler/mod.rs`: `Scheduler` — all timer management
- `src-tauri/src/ipc/mod.rs`: All 17 IPC commands the frontend can call
- `src-tauri/src/persistence.rs`: `ProfileManager` — JSON profile persistence
- `src-tauri/src/platform/mod.rs`: `InputProvider` + `PlatformObserver` traits

**Platform Implementations:**
- `src-tauri/src/platform/macos/input.rs`: CGEvent-based input injection
- `src-tauri/src/platform/macos/observer.rs`: CGEventTap + NSWorkspace observer + hotkey registry
- `src-tauri/src/platform/windows/mod.rs`: Win32 SendInput + Windows hook observer

**Testing:**
- `src-tauri/src/scheduler/mod.rs` (bottom): `afk_farm_stress_test`, `jitter_audit_10ms_interval` — inline `#[tokio::test]`
- `src-tauri/src/persistence.rs` (bottom): `large_config_memory_check` — inline `#[test]`

## Naming Conventions

**Rust files:**
- Module directories use `mod.rs` pattern (e.g., `ipc/mod.rs`, `state/mod.rs`)
- `snake_case` for all file names and module names
- Platform modules nested under `platform/{os}/`

**Rust types:**
- `PascalCase` for structs, enums, traits (e.g., `StateActor`, `MacroConfig`, `InputProvider`)
- `SCREAMING_SNAKE_CASE` for statics (e.g., `STATE_TX`, `TAP_INITIALIZED`, `HOTKEY_BINDINGS`)
- `snake_case` for functions and methods

**TypeScript files:**
- `PascalCase` for component files (`App.tsx`) and interfaces (`MacroConfig`, `AppState`)
- `camelCase` for handler functions (`handleCreateMacro`, `handleToggleEngine`)
- `camelCase` for signal getters/setters (`newMacroName`, `setNewMacroName`)

**Rust enum variants:**
- `PascalCase` for all variants (e.g., `Intent::AddMacro`, `ActionType::Interval`)
- Intent variants named as imperative actions: `AddMacro`, `RemoveMacro`, `SetMacroEnabled`

## Where to Add New Code

**New IPC command:**
1. Add `Intent::NewVariant(...)` to the `Intent` enum in `src-tauri/src/state/mod.rs:118`
2. Add handler arm in `StateActor::handle_intent()` (`src-tauri/src/state/mod.rs:271`)
3. Add `pub async fn new_command(...)` function to `src-tauri/src/ipc/mod.rs`
4. Register in `invoke_handler!` macro in `src-tauri/src/lib.rs:84`
5. Call `invoke("new_command", { ... })` from `src/App.tsx`

**New macro field:**
1. Add field to `MacroConfig` struct (`src-tauri/src/state/mod.rs:77`) with `#[serde(default)]`
2. Mirror in TypeScript `MacroConfig` interface (`src/App.tsx:10`)
3. Handle in `StateActor::handle_intent()` if new intent needed
4. Handle in `Scheduler::start_macro()` if scheduling behavior changes (`src-tauri/src/scheduler/mod.rs:196`)

**New platform (e.g., Linux):**
1. Add `#[cfg(target_os = "linux")] pub mod linux;` in `src-tauri/src/platform/mod.rs`
2. Implement `InputProvider` trait for a new `LinuxInputProvider` struct
3. Implement `PlatformObserver` trait for a new `LinuxPlatformObserver` struct
4. Add `#[cfg(target_os = "linux")]` block in `src-tauri/src/lib.rs` startup setup
5. Add `#[cfg(target_os = "linux")]` field in `StateActor` struct (`state/mod.rs:138`)

**New persistence operation:**
- Add method to `ProfileManager` in `src-tauri/src/persistence.rs`
- Add corresponding IPC command following the pattern above
- No new files needed — persistence is a single module

**New UI section/panel:**
- All UI lives in `src/App.tsx` — add a new `Tab` value and `Show` block
- Follow the `createSignal` + `createEffect` + `async function handleX()` pattern already present
- No separate component files currently — the codebase uses a single-file component model

## Special Directories

**`archive/v1-production/`:**
- Purpose: Archived first-version codebase and build artifacts
- Generated: No
- Committed: Yes — do not delete; do not import from

**`src-tauri/icons/`:**
- Purpose: App icon assets for all target platforms and resolutions
- Generated: Via `tauri icon` command from a source image
- Committed: Yes

**`src-tauri/capabilities/`:**
- Purpose: Tauri v2 capability/permission declarations (JSON)
- Generated: Partially (via `tauri` CLI)
- Committed: Yes — must be committed for production builds

**`.planning/codebase/`:**
- Purpose: GSD codebase map documents consumed by planning and execution agents
- Generated: By `/gsd-map-codebase` command
- Committed: Yes

---

*Structure analysis: 2026-05-15*
