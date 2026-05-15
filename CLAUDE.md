<!-- GSD:project-start source:PROJECT.md -->
## Project

**AutoMux**

AutoMux is a cross-platform desktop auto-clicker and macro automation tool for macOS and Windows, built with Tauri 2, Rust, and SolidJS. It lets users define multi-step macros (clicks, keypresses, timing, optional process targeting) and run them system-wide or scoped to a specific application. It is aimed at users who need reliable, configurable input automation — gamers, productivity power users, and anyone who needs to automate repetitive mouse/keyboard tasks.

**Core Value:** A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.

### Constraints

- **Tech stack:** Tauri 2 + Rust + SolidJS — no framework changes; improvements must work within this architecture
- **Compatibility:** Must maintain working builds for both macOS (arm64 + x86_64) and Windows (x64)
- **Permissions model:** macOS Accessibility permission handling is OS-enforced; the fix must work within what Tauri and CGEvent allow
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- TypeScript ~5.6.2 - Frontend UI (`src/`)
- Rust 2021 edition - Backend/core engine (`src-tauri/src/`)
- CSS - UI styling (`src/App.css`)
- HTML - App shell (`index.html`)
## Runtime
- Node.js 24 (CI-pinned via `actions/setup-node@v4`)
- Rust stable toolchain (managed via `dtolnay/rust-toolchain@stable`)
- npm
- Lockfile: `package-lock.json` present
## Frameworks
- Tauri 2 - Desktop app shell and IPC bridge; ties Rust backend to web frontend (`src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`)
- SolidJS 1.9.3 - Reactive UI framework (`src/`)
- Vite 6.0.3 - Frontend bundler and dev server (`vite.config.ts`)
- vite-plugin-solid 2.11.0 - SolidJS Vite integration (`vite.config.ts`)
- Tailwind CSS 4.3.0 via `@tailwindcss/vite` plugin (`vite.config.ts`)
- tauri-build 2 - Rust build script for Tauri (`src-tauri/Cargo.toml` `[build-dependencies]`)
- No frontend test framework detected
- Rust unit tests in `src-tauri/src/persistence.rs` (inline `#[cfg(test)]` module, standard `cargo test`)
## Key Dependencies
- `@tauri-apps/api` ^2 - Frontend ↔ Rust IPC (`invoke`, `listen` for events)
- `@tauri-apps/plugin-opener` ^2 - Shell/URL open capability
- `tauri-plugin-opener` 2 - Rust-side opener plugin
- `tokio` 1 (rt-multi-thread, sync, time, macros) - Async runtime powering the scheduler and state actor
- `serde` + `serde_json` 1 - JSON serialization for IPC data types and profile persistence
- `uuid` 1 (v4, serde) - Unique IDs for macro configs
- `core-graphics` 0.24.0 - CGEvent tap for input injection
- `core-foundation` 0.10.0 / `core-foundation-sys` 0.8.0 - macOS CF API bindings
- `objc` 0.2.7 / `objc2` 0.6.4 - Objective-C runtime interop
- `objc2-app-kit` 0.3.2 / `objc2-foundation` 0.3.2 - AppKit/Foundation bindings
- `cocoa` 0.26.1 - Higher-level Cocoa bindings
- `block2` 0.6.2 - Objective-C block support
- `windows` 0.61 with features: `Win32_UI_Input_KeyboardAndMouse`, `Win32_UI_WindowsAndMessaging`, `Win32_Foundation`, `Win32_System_Threading`, `Win32_UI_Accessibility` - Win32 API for input injection and window tracking
## Configuration
- `TAURI_DEV_HOST` env var: optional; enables remote device dev mode in `vite.config.ts`
- `GITHUB_TOKEN` secret: used in CI release workflow (`release.yml`) — not an app runtime secret
- `vite.config.ts` - Frontend build config; dev server on port 1420 (strict)
- `tsconfig.json` - TypeScript compiler; strict mode enabled, `jsxImportSource: solid-js`
- `tsconfig.node.json` - TypeScript config for Vite/Node tooling
- `src-tauri/tauri.conf.json` - Tauri app config (window dimensions, bundle targets, CSP, macOS entitlements)
- `src-tauri/Cargo.toml` - Rust crate manifest with platform-conditional dependencies
## Platform Requirements
- Node.js 24+
- Rust stable toolchain
- macOS: Accessibility permissions required at runtime (checked via `AXIsProcessTrusted`)
- macOS minimum deployment: 12.0 (Monterey)
- Bundle targets: `dmg` (macOS), `nsis` installer (Windows)
- macOS: signed builds require `signingIdentity` in `tauri.conf.json` (currently `null` — unsigned)
- Windows: code signing requires `certificateThumbprint` (currently `null` — unsigned); timestamp URL set to `http://timestamp.digicert.com`
- App identifier: `com.alvaro.automux`
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Overview
## Rust Conventions (`src-tauri/src/`)
### Naming Patterns
- `snake_case` for all module files: `persistence.rs`, `scheduler/mod.rs`, `state/mod.rs`
- Platform-specific code lives in sub-modules: `platform/macos/`, `platform/windows/`
- `PascalCase` for structs, enums, and traits: `MacroConfig`, `ActionStep`, `InputProvider`, `StateActor`
- Enum variants: `PascalCase` — `TriggerMode::Pulse`, `TriggerMode::Hold`, `ActionType::HoldStart`
- `snake_case` for all functions: `handle_intent`, `start_macro`, `inject_input`, `sanitize_name`
- `snake_case` for all local vars and struct fields: `interval_ms`, `macro_id`, `action_tx`
- `UPPER_SNAKE_CASE` for statics/constants (pattern from observer.rs)
### Module Organization
### Error Handling
#[cfg(debug_assertions)]
### Documentation Comments
- `@architect:` — structural intent
- `@performance-tuner:` — perf constraints or notes
- `@safety-officer:` — safety-critical invariants
- `// @safety-officer:` — marks invariants that must not be broken
- `// @scheduler-agent:` — marks scheduling constraints
- These are convention, not tooling — they are searched by code review.
### Derives
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
### Async Design
- All IPC handlers are `async` functions.
- Internal business logic (StateActor, Scheduler) runs as single long-lived Tokio tasks
- No per-macro spawns. The Scheduler is a single task with a `tokio::select!` loop.
- Channel backpressure: `try_send` for fire-and-forget from timer hot paths; `.await`
## TypeScript / SolidJS Conventions (`src/`)
### Naming Patterns
- `PascalCase.tsx` for components: `App.tsx`
- `camelCase.ts` for non-component modules (if any)
- `PascalCase` for interfaces and type aliases: `MacroConfig`, `AppState`, `TriggerMode`
- Union type aliases for string literals: `type Tab = "dashboard" | "profiles";`
- `camelCase` for helpers: `formatInputEvent`, `formatStep`
- `camelCase` prefixed with `handle` for event handlers: `handleCreateMacro`,
- `camelCase` prefixed with `refresh` for data-refresh helpers: `refreshProfiles`
- Signal getter functions follow SolidJS convention — `camelCase()` to read,
- Boolean signals use positive framing: `showNewMacro`, `profileLoading`, `engineActive`
### SolidJS Patterns
- `createSignal` for all local reactive state.
- `createEffect` for side effects (data fetching, event listeners, polling).
- `onCleanup` always called inside `createEffect` for subscriptions and intervals.
- `Show` for conditional rendering (never ternary JSX with non-trivial subtrees).
- `For` for list rendering over arrays.
- Non-signal derived state uses plain functions (not `createMemo`):
### Import Organization
### Error Handling (TypeScript)
### Styling
- All styling via inline `class=` Tailwind utility strings.
- No `className` (SolidJS uses `class`).
- Custom design tokens referenced with CSS variable syntax: `bg-accent`,
- Conditional class strings use template literals: `` `flex-1 ${condition ? "a" : "b"}` ``
### TypeScript Config
- `strict: true` — all strict checks enabled.
- `noUnusedLocals: true`, `noUnusedParameters: true` — no dead code allowed.
- `noFallthroughCasesInSwitch: true`.
- `jsx: "preserve"`, `jsxImportSource: "solid-js"` — SolidJS JSX transform.
- Target: `ES2020`, module: `ESNext`.
## Cross-Language Conventions
### Type Mirroring
### IPC Naming
- Rust `#[command]` function names use `snake_case`: `add_macro`, `set_macro_enabled`
- TypeScript `invoke` call strings use the same `snake_case` names verbatim
### Section Separators
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## System Overview
```text
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
- Single `StateActor` async task owns all mutable state — no shared mutexes on the hot path
- Single `Scheduler` async task owns all timers — O(s) memory where s = active interval steps
- Two-phase dispatch: Scheduler fires `ActionReady` → StateActor validates targeting → injects
- Platform abstraction via `InputProvider` and `PlatformObserver` traits with per-OS implementations
- Frontend receives state via Tauri events (`state-changed`); all mutations go through `invoke()`
## Layers
- Purpose: User interface only — renders state, calls backend commands
- Location: `src/`
- Contains: Single `App.tsx` component with all UI, SolidJS signals for reactive state
- Depends on: Tauri JS API (`@tauri-apps/api/core` invoke, `@tauri-apps/api/event` listen)
- Used by: End user via Tauri WebView
- Purpose: Thin translation from Tauri `#[command]` invocations to `Intent` enum messages
- Location: `src-tauri/src/ipc/mod.rs`
- Contains: One async function per IPC command, all delegating to `StateManager::send_intent()`
- Depends on: `StateManager`, `state::Intent`, `persistence::ProfileManager`
- Used by: Tauri `invoke_handler!` macro registration in `lib.rs`
- Purpose: Authoritative runtime state, Intent processing, input injection gating
- Location: `src-tauri/src/state/mod.rs`
- Contains: `AppState`, `MacroConfig`, `ActionSequence`, `Intent` enum, `StateActor`, `StateManager`
- Depends on: `scheduler::SchedulerIntent`, `platform::InputProvider`, `tauri::Emitter`
- Used by: IPC layer (writes via Intent), Scheduler (reads ActionReady back to it)
- Purpose: Pure timer management — fires `ActionReady` signals, never injects input
- Location: `src-tauri/src/scheduler/mod.rs`
- Contains: `Scheduler`, `SchedulerIntent`, `ActionReady`, `ActionType`, `IntervalTask`
- Depends on: `state::ActionStep`, `state::MacroConfig`
- Used by: StateActor (sends `SchedulerIntent`, receives `ActionReady`)
- Purpose: OS-specific input injection and active-app observation
- Location: `src-tauri/src/platform/`
- Contains: `InputProvider` trait, `PlatformObserver` trait, macOS impl, Windows impl
- Depends on: CoreGraphics (macOS), Win32 (Windows)
- Used by: StateActor (injection), `lib.rs` (observer startup)
- Purpose: Save/load named macro profiles as JSON files
- Location: `src-tauri/src/persistence.rs`
- Contains: `ProfileManager`, `ProfileData`, `ProfileSummary`
- Depends on: `state::MacroConfig`, `serde_json`, `tokio::fs`
- Used by: IPC layer (profile commands), `lib.rs` (startup restoration)
## Data Flow
### Primary: User Creates a Macro
### Primary: Macro Fires an Action (Two-Phase Dispatch)
### Platform Event: Hotkey Pressed
### Platform Event: Active App Changed
### Startup Restoration
- All runtime state lives in `StateActor.state: AppState` (single-owner, no shared mutexes on hot path)
- Static `OnceLock<Mutex<...>>` used in platform observer for cross-thread registries (hotkey bindings, active app, registry of held inputs)
- Frontend state is a read-only projection: StateActor pushes full `AppState` on every change
## Key Abstractions
- Purpose: Platform-agnostic interface for injecting OS input events
- Implementations: `MacInputProvider` (`platform/macos/input.rs`), `WindowsInputProvider` (`platform/windows/mod.rs`)
- Methods: `inject_key`, `inject_mouse_click`, `inject_mouse_move`, `inject_mouse_button_raw`, `flush_held_inputs`
- Pattern: Stateless struct; new `CGEventSource` created per call on macOS for thread safety
- Purpose: Platform-agnostic interface for observing active app changes
- Implementations: `MacPlatformObserver` (`platform/macos/observer.rs`), `WindowsPlatformObserver` (`platform/windows/mod.rs`)
- Pattern: Long-lived struct started once at app init; pushes Intents via static `STATE_TX`
- Purpose: All mutations to `AppState` are expressed as Intent variants — no direct field access from outside `StateActor`
- Location: `src-tauri/src/state/mod.rs:118`
- Used by: IPC layer, platform observers (via static channel), hotkey callbacks
- Purpose: The core data model shared between frontend (TypeScript mirror), StateActor, Scheduler, and persistence
- `ActionSequence` contains `Vec<ActionStep>` where each step is either `SustainedHold` or `InterleavedInterval`
- Legacy compatibility: empty `sequence` falls back to single left-click at `interval_ms`
- Purpose: Message types for the two-way channel between StateActor and Scheduler
- `SchedulerIntent`: StateActor → Scheduler (start/stop/update macros)
- `ActionReady`: Scheduler → StateActor (fire this action now)
## Entry Points
- Location: `src-tauri/src/main.rs:4`
- Triggers: OS process launch
- Responsibilities: Calls `automux_lib::run()` with Windows subsystem annotation
- Location: `src-tauri/src/lib.rs:19`
- Triggers: Called from `main()`
- Responsibilities: Creates all mpsc channels, spawns Scheduler and StateActor tasks, initializes ProfileManager, starts platform observer, registers IPC handlers
- Location: `src/index.tsx`
- Triggers: WebView load
- Responsibilities: Mounts `<App />` SolidJS component into `#root`
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
### Injecting input from the Scheduler
### Per-macro spawned tasks
## Error Handling
- IPC commands return `Err(e.to_string())` for all failures; frontend catches with `try/catch` on `invoke()`
- Persistence errors surface as `Err(String)` returned from `ProfileManager` methods
- Platform errors (CGEvent creation failures) are silently dropped with `if let Ok(event) = ...`
- Emergency stop path (`flush_held_inputs`) is best-effort — does not propagate errors
## Cross-Cutting Concerns
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
