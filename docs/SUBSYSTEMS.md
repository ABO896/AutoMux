# Subsystems Architecture

In order to maintain strict architectural boundaries, the application is divided into independent subsystems. Each subsystem has a specific responsibility, defined interfaces, and concurrency rules.

## 1. Scheduler Engine (`scheduler`)
- **Responsibilities:** Manage Tokio tasks for active macros, handle tick events, coordinate with the native input layer to execute actions. Provide an emergency stop mechanism to cancel all tasks.
- **Interfaces:** `start_macro(id: Uuid, config: MacroConfig)`, `stop_macro(id: Uuid)`, `stop_all()`.
- **Dependencies:** `tokio` (time, sync), `tokio-util` (CancellationToken).
- **Concurrency Concerns:** Receives commands via mpsc channels. Owns the cancellation tokens for all running macros. Must not block the Tokio reactor.
- **Testing Strategy:** Unit tests mocking time (`tokio::time::pause()`) to verify interval accuracy and cancellation speed.

## 2. Native Input Layer (`native_input`)
- **Responsibilities:** Listen for global hotkeys (including the emergency stop). Inject simulated mouse clicks and keyboard events.
- **Interfaces:** `register_hotkey(...)`, `inject_event(event: InputEvent)`.
- **Dependencies:** `rdev` or OS-specific crates (WinAPI, CoreGraphics) for lock-free hooking.
- **Concurrency Concerns:** Callbacks from OS hooks run on OS threads. Must send events to Rust via non-blocking channels (`tokio::sync::mpsc::unbounded_channel`). Must be completely lock-free.
- **Testing Strategy:** Integration tests verifying injection without crashing, and unit tests for event formatting.

## 3. Process Detection (`process_tracker`)
- **Responsibilities:** Detect active foreground window. Ensure macros only fire when the target application is focused. Detect fullscreen mode.
- **Interfaces:** `get_foreground_process() -> ProcessInfo`, `on_process_changed(callback)`.
- **Dependencies:** `sysinfo`, OS-specific APIs.
- **Concurrency Concerns:** Operates via a background listener thread using OS hooks rather than polling, communicating changes via channels.
- **Testing Strategy:** Manual integration testing on each target OS.

## 4. Profiles/State Management (`state_manager`)
- **Responsibilities:** Maintain truth for profiles, macros, settings. Persist state to disk. Sync state to the UI via Tauri IPC.
- **Interfaces:** `load()`, `save()`, `update_macro(...)`, `get_state()`.
- **Dependencies:** `serde`, `tauri` (for state management/IPC).
- **Concurrency Concerns:** Implements the Actor pattern. One Tokio task owns the state, receives mutations via channel, and broadcasts updates. No Mutexes spanning `await` points.
- **Testing Strategy:** Property-based testing for state transitions and serialization.

## 5. UI Layer (`ui`)
- **Responsibilities:** Command palette, macro configuration, status display. Lightweight, keyboard-first.
- **Interfaces:** SolidJS components, Tauri IPC client.
- **Dependencies:** `solid-js`, `tailwindcss`, `@tauri-apps/api`.
- **Concurrency Concerns:** Responds to reactive state updates from Tauri IPC. Derives state instead of syncing it locally where possible.
- **Testing Strategy:** Vitest for logic, Playwright for e2e.

## 6. Tray/Background Management (`tray_manager`)
- **Responsibilities:** Keep app running in the background. System tray icon and menu. Toggle UI visibility.
- **Interfaces:** `init_tray()`, `set_tray_status()`.
- **Dependencies:** `tauri` tray APIs.
- **Concurrency Concerns:** Native OS thread execution for tray APIs.
- **Testing Strategy:** Manual testing.

## 7. Reviewer/Performance Validation (`telemetry`)
- **Responsibilities:** Track memory usage, CPU usage, and IPC event frequency to ensure O(1) overhead.
- **Interfaces:** `log_metric()`, `get_stats()`.
- **Dependencies:** `std::time`, process stats.
- **Concurrency Concerns:** Uses lightweight atomic counters.
- **Testing Strategy:** Load tests simulating 10,000 injected events to measure memory/CPU spikes.
