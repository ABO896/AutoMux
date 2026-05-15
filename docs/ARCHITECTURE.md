# AutoMux - Desktop Automation Architecture

## 1. Architecture Overview
The application is a Tauri-based desktop utility consisting of a Rust/Tokio backend for heavy lifting (OS hooks, scheduling) and a SolidJS/Tailwind frontend for a minimal, keyboard-driven UI.

**Core Principles:**
- **Zero Polling**: Event-driven architecture using system hooks and Tokio's asynchronous event loop.
- **Message Passing**: Concurrency managed via MPSC channels instead of shared mutable state (`Arc<Mutex<T>>`) wherever possible.
- **O(1) Overhead**: Background tasks must sleep when inactive.

## 2. Scheduler Engine
The scheduler will utilize `tokio::time::interval` wrapped in independent async tasks. 
- **Drift Prevention**: `Interval::set_missed_tick_behavior(MissedTickBehavior::Skip)` to ensure we don't queue up rapid-fire events if the OS suspends the process.
- **Task Lifecycle**: When an automation is enabled, the State Manager spawns a Tokio task. The task holds a `CancellationToken` (from `tokio-util`). When the automation is disabled or modified, the token is cancelled, cleanly terminating the loop.
- **Engine Loop**:
  ```rust
  loop {
      tokio::select! {
          _ = interval.tick() => { execute_action(&action).await; }
          _ = cancel_token.cancelled() => { break; }
      }
  }
  ```

## 3. Concurrency Risks
- **Global Input Hook Deadlocks**: OS-level hooks for keyboard/mouse events block the OS input queue. If our hook callback attempts to acquire a Mutex that is held by a slow operation, the entire OS UI could freeze. 
  *Mitigation*: Input hook callbacks must be lock-free, immediately dispatching events to a Tokio channel (`mpsc::unbounded_channel`) and returning.
- **State Inconsistencies**: The UI enabling a task at the exact millisecond it is deleted.
  *Mitigation*: The backend acts as the single source of truth using an Actor pattern. The UI sends `Intent` messages, and the backend broadcasts `StateSync` updates.

## 4. Performance Bottlenecks
- **Event Injection Loop**: Rapid consecutive inputs (e.g., auto-clicker at 1000cps) can flood the OS event queue.
- **IPC Overhead**: Sending every tick or mouse click over the Tauri IPC to the frontend.
  *Mitigation*: The frontend does NOT need to know about every simulated click. The backend only syncs high-level state (Enabled/Disabled) and statistics (updated at most 1Hz).

## 5. Module Boundaries (Rust Backend)
- `input_hook`: OS-level listeners for global hotkeys and emergency stops.
- `injector`: OS-level simulation of keystrokes and mouse clicks.
- `scheduler`: Spawns and tracks Tokio tasks for active automations.
- `state_manager`: The central Actor managing profiles, configurations, and active task lists.
- `ipc_bridge`: Tauri command handlers and event emitters.
- `process_target`: OS-specific window/process tracking to ensure macros only fire when the correct app is in focus.

## 6. Folder Structure
```text
/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── input_hook/
│   │   ├── injector/
│   │   ├── scheduler/
│   │   ├── state/
│   │   ├── ipc/
│   │   └── process_target/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                # SolidJS
│   ├── index.tsx
│   ├── App.tsx
│   ├── components/     # UI primitives (shadcn-like)
│   ├── features/       # Feature-specific components
│   ├── store/          # SolidJS state/signals
│   ├── lib/            # Utilities (IPC wrappers, hotkeys)
│   └── styles/         # Tailwind CSS
├── package.json
└── RULES.md
```

## 7. Implementation Phases
- **Phase 1: Foundation (Backend)**. Setup Tauri, Tokio, Actor state, and IPC bridge. No automation yet.
- **Phase 2: UI Shell (Frontend)**. SolidJS + Tailwind setup, keyboard navigation (Raycast style), tray icon support.
- **Phase 3: The Engine (Backend)**. Implement `input_hook` and `injector`. Create the Tokio `scheduler`.
- **Phase 4: Integration (Fullstack)**. Connect UI configuration to the backend engine. Implement Start/Stop and the Emergency Stop feature.
- **Phase 5: Advanced Features**. Process targeting, fullscreen detection, profiles, and toggle/hold modes.

## 8. Reviewer Checkpoints
- **Checkpoint 1**: Post-Phase 1. Review state management architecture to ensure it's lock-free and Actor-based.
- **Checkpoint 2**: Post-Phase 3. Review OS-level hook performance to ensure zero OS input lag.
- **Checkpoint 3**: Post-Phase 4. UX Review. Ensure keyboard-only navigation feels like Superhuman/Linear.

## 9. Dangerous Edge Cases
- **The Runaway Macro**: A macro that clicks so fast the user cannot click the UI to stop it. 
  *Fix*: Hardware-level emergency stop hook (e.g., `Ctrl+Alt+Shift+Escape`) processed synchronously at the highest priority.
- **Self-Triggering Hooks**: Our injected simulated keystrokes triggering our own hotkey listeners, causing an infinite loop.
  *Fix*: Event simulation must tag inputs (if supported by OS) or we must logically pause hooks for the thread injecting the event.
- **Modifier Key Sticking**: A macro holds `Shift`, but the process crashes or is stopped before releasing it. The OS is left with `Shift` permanently pressed.
  *Fix*: On macro cancellation or panic, the `scheduler` MUST execute a cleanup routine that dispatches KeyUp events for all currently tracked pressed keys.

## 10. Anti-Patterns to Avoid
- **Polling for Window Focus**: Do not use `loop { get_foreground_window(); sleep(10ms); }`. Use OS event hooks to detect focus changes.
- **React-style Prop Drilling**: With SolidJS, context or a centralized signal store is preferred over passing props down.
- **Overusing `useEffect` (or `createEffect`)**: UI state should be derived, not synchronized. 
- **Electron-style resource usage**: Keep memory usage low. Rely on Tauri's Wry instead of bundling a heavy browser engine.
