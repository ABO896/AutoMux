# Failure-Mode Analysis

This document outlines the critical failure modes identified for the AutoMux architecture and the exact recovery strategies to ensure system stability.

## 1. Runaway Loops & Event Flooding
- **Scenario:** A macro is configured to click every 1ms without a delay, flooding the OS event queue. The OS becomes unresponsive to normal mouse clicks.
- **Mitigation:**
  - The Injector enforces a hard limit (e.g., minimum 5ms delay between physical simulated inputs).
  - The dedicated hardware-level Emergency Stop thread (listening for `Ctrl+Alt+Shift+Escape`) bypasses the flooded standard event queue and triggers a global halt.

## 2. Scheduler Desync
- **Scenario:** OS goes to sleep, or CPU is starved at 100%. A Tokio timer misses 50 ticks. Upon waking, it attempts to execute 50 clicks instantly.
- **Mitigation:** `tokio::time::interval` is configured with `MissedTickBehavior::Skip`. It will execute exactly once upon waking, discarding the backlog.

## 3. IPC Stalls
- **Scenario:** The backend sends 10,000 state updates to the UI per second, locking the Tauri Webview thread.
- **Mitigation:** The backend never sends raw ticks. State changes (e.g., Active/Inactive) are batched and debounced to a maximum of 60Hz.

## 4. Deadlocks
- **Scenario:** Thread A waits on a Mutex held by Thread B, but Thread B is waiting on an OS hook that Thread A has locked.
- **Mitigation:** The architecture completely forbids `Arc<Mutex<T>>` spanning across OS boundaries or `await` points. The Actor pattern ensures state is managed sequentially by a single owner, with external threads communicating strictly via non-blocking MPSC channels.

## 5. Task Leaks
- **Scenario:** A macro is deleted by the user while it is currently in a "sleep" cycle waiting to execute its next step. The task remains in memory forever.
- **Mitigation:** Every spawned macro task is bound to a `CancellationToken` (via `tokio-util`). When a macro is deleted, modified, or deactivated, the token is explicitly cancelled. The `select!` macro inside the task ensures immediate termination.

## 6. Stuck Inputs (Modifier Keys)
- **Scenario:** A macro executes `KeyDown(Shift)`, but the user triggers the emergency stop before `KeyUp(Shift)` is executed. The user's physical keyboard behaves as if Shift is permanently held down.
- **Mitigation:** The native injector maintains an atomic bitmask of all currently depressed simulated keys. On macro cancellation or panic, a `cleanup_keys()` routine runs, forcing `KeyUp` for all tracked active keys.

## 7. Panic Recovery
- **Scenario:** A boundary parsing error causes the Actor task or a Macro task to panic.
- **Mitigation:** `tokio` catches panics in spawned tasks without crashing the runtime. The supervisor task detects the exit of a macro task and resets the UI state to `Error/Stopped`.

## 8. Hotkey Conflicts
- **Scenario:** The user assigns `Ctrl+C` to a macro, overriding the OS copy function.
- **Mitigation:** The UI will display a prominent warning when binding system-reserved keys. The hotkey hook operates in "passthrough" mode by default unless the hotkey strictly matches an active macro, in which case the OS event is consumed.

## 9. Tray/Background Lifecycle Failures
- **Scenario:** The user closes the UI window, but a silent error occurs preventing the tray icon from spawning. The app is running, macros are active, but there is no way to open the UI.
- **Mitigation:** The UI close button defaults to `hide()`. The tray icon is initialized on boot, *before* the UI is shown. If tray initialization fails, the app forces the UI to stay visible.
