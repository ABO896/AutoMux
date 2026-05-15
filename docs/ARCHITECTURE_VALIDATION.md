# Architecture Validation & Pressure Test Report

## 1. Scheduler Precision & Drift
- **Internal Mechanism**: `tokio::time::interval` relies on the OS timer wheel and Tokio's reactor. Using `MissedTickBehavior::Skip` means if the CPU is bogged down and we miss a deadline, Tokio will fire the timer *once* upon waking, and compute the next deadline relative to the original start time, preventing rapid-fire bursts.
- **Comparisons**:
  - `interval`: Best balance of O(1) memory/CPU efficiency and acceptable accuracy.
  - `sleep_until`: Functions similarly but requires manual loop calculation.
  - `std::thread::sleep` (Dedicated Threads): Extremely precise (spin-sleeping gives microsecond accuracy) but burns CPU and scales poorly (O(N) OS threads).
- **Expected Drift**:
  - *Normal/Minimized*: ~1-3ms drift.
  - *High CPU Load*: Tokio worker threads might delay task execution by 10-15ms.
  - *Many Tasks*: Tokio scales to millions of tasks. Timer resolution is the bottleneck, not task count.
- **Realistic Guarantee**: +/- 5ms on modern OSs, provided the OS multimedia timer is requested (`timeBeginPeriod(1)` on Windows). If absolute <1ms precision is required for competitive gaming macros, a dedicated spin-lock OS thread is necessary, which violates the "low CPU usage" requirement. We will default to Tokio timers.

## 2. Input Injection Reliability
- **Comparisons**:
  - *SendInput (WinAPI) / CGEventPost (Mac)*: Highly reliable for desktop apps. Struggles with direct-input games unless scan codes are formatted perfectly.
  - *Interception / Kernel Drivers*: Bypasses all anti-cheat, but requires complex installation and violates the lightweight utility requirement.
  - *enigo / rdev*: High-level crates. Often abstract away critical flags needed to bypass game engines or track our own injected events (to avoid hook loops).
- **Recommendation**: **Raw OS APIs**. We will implement a minimal `native_input` module using `windows-rs` on Windows and `core-graphics` on macOS. This avoids external unmaintained dependencies and allows us to append tracking flags (like `LLMHF_INJECTED`) to differentiate physical vs simulated input.

## 3. State Architecture Validation
- **Actor Model Design**: A single Tokio task `StateActor` owns the configuration state. It listens on an `mpsc::channel`.
- **Lock Avoidance**: There are zero `Mutex` or `RwLock` primitives wrapping the state.
- **IPC Synchronization**: 
  1. SolidJS sends `toggle_macro(id)`.
  2. Tauri Command sends an `Intent::ToggleMacro(id)` to the Actor channel.
  3. Actor receives intent, mutates state, and emits `StateDiff` to the frontend via Tauri.
- **Race Condition Identifications**:
  - *Double-Toggle*: The UI sends "toggle" and a physical hotkey sends "toggle" simultaneously. The Actor processes them sequentially, resulting in the macro flipping on then immediately off.
  - *Mitigation*: The interface will use explicit commands (`Intent::SetMacroEnabled(id, true)`) rather than idempotent toggles for physical hotkeys, or apply a short debounce.

## 4. Emergency Stop System
- **Risk**: A runaway macro spamming 1000 clicks/sec will freeze the OS input queue. The Tokio reactor might starve.
- **Architecture**:
  - A dedicated raw OS thread (`std::thread::spawn`) is created *outside* the Tokio runtime.
  - It uses a synchronous blocking hook (or raw `GetAsyncKeyState` polling loop at 5ms if hooks are dropped by the OS) exclusively looking for a hardcoded combo (e.g., `Ctrl+Alt+Shift+Escape`).
  - Upon detection, it toggles a global `AtomicBool` `EMERGENCY_STOP` and sends a `CancelToken` to all active macros.
- **Cleanup Guarantees**: The Injector maintains an atomic bitset of currently depressed keys. On emergency stop, the dedicated thread executes a loop sending `KeyUp` events for all active bits to prevent the "stuck modifier" OS bug.

## 5. Resource Usage Modeling
- **Idle RAM**: ~15MB (Rust backend) + ~30-50MB (Tauri WebView) = ~60MB total.
- **Per-Task Overhead**: ~2KB memory per Tokio task.
- **CPU Usage Estimates**:
  - *1 Task (100ms interval)*: < 0.1% CPU.
  - *10 Tasks*: < 0.1% CPU.
  - *100 Tasks*: ~0.5% CPU (dominated by OS timer interrupts and IPC syncing, if UI is visible).
- **Optimization**: Do not send IPC updates for macro ticks. Only sync state when a macro starts/stops or when the user opens the UI to view telemetry.

## 6. Process Detection / Fullscreen Tracking
- **Strategy**: Avoid polling (`GetForegroundWindow` in a loop).
- **OS Hooks**:
  - *Windows*: `SetWinEventHook` listening to `EVENT_SYSTEM_FOREGROUND`. Callback triggers instantly when focus shifts.
  - *macOS*: `NSWorkspaceDidActivateApplicationNotification` listener.
- **Fullscreen Detection Edge Cases**:
  - When the focus changes, the backend retrieves the Window RECT and compares it against the Monitor RECT.
  - *Borderless Fullscreen*: Matches monitor RECT perfectly, handled natively.
  - *Alt-Tab*: Triggers a foreground change event. The state manager evaluates the new process. If it doesn't match the macro's target, running macros are instantly cancelled.

## 7. UI Architecture Validation
- **State Flow**: SolidJS maintains a reactive store. Tauri backend pushes patch events (`diffs`). SolidJS merges diffs.
- **Batching**: IPC messages from Rust are debounced using a 16ms window (approx 60fps) to prevent IPC queue flooding if the backend state changes rapidly.
- **Render Minimization**: SolidJS compiles to direct DOM updates. If Macro A's "active" state changes, only the specific SVG path of its status dot is recalculated, not the list.

## 8. Dependency Audit
We are heavily restricting dependencies to minimize binary bloat and compile times:
- `tauri`, `serde` (Required).
- `tokio` (For safe async scaling).
- `windows-rs` / `core-graphics` (Raw OS APIs, replacing `enigo` and `rdev`).
- `solid-js`, `tailwindcss`.
- **Excluded**: `reqwest`, `regex`, unmaintained cross-platform input crates.

## 9. Reviewer Audit (Self-Critical)
- **Weak Point (OS Hooks)**: Windows silently drops `SetWindowsHookEx` hooks if the callback exceeds `LowLevelHooksTimeout` (default 300ms). Our callbacks MUST be <1ms. They will immediately push to a lock-free queue and return.
- **Risky Assumption (macOS Accessibility)**: Injecting input on Mac requires user permission in System Preferences. If denied, the app fails silently. We must architect a pre-flight permission check on boot.
- **Overengineering Risk**: Using an Actor model for config state is robust but slightly verbose. However, avoiding `Arc<Mutex<T>>` in a highly concurrent, hotkey-driven app eliminates deadlock panics, which are fatal for macro software.
