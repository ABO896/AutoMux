# AutoMux Implementation Plan

## Phase 1: Native Input Layer (macOS)
*(Completed)*

## Phase 2: Cross-Platform Architecture & Process Targeting
*(Completed)*

### Objective
Transition the project to a fully cross-platform architecture (macOS `.app` and Windows `.exe`). Define platform-agnostic traits (`PlatformObserver`, `InputProvider`) to unify targeting rules and input injection. Implement precise background execution by tracking the active application and exposing global hotkeys, leveraging the new trait-based architecture.

### Tasks

- [x] **2.1 Trait-First Architecture**
- [x] **2.2 macOS Platform Implementations**
- [x] **2.3 Windows Placeholder Implementations**
- [x] **2.4 Process Detection (Push-based)**
- [x] **2.5 Targeting Rules in StateActor**
- [x] **2.6 Global Hotkeys**
- [x] **2.7 Accessibility Verification & Dynamic Request**
  - Implemented `check_accessibility_permissions(prompt)` via `AXIsProcessTrustedWithOptions`.
  - Extracted `initialize_tap()` with `AtomicBool` CAS guard (idempotent, no thread leaks).
  - Added `request_accessibility` and `check_accessibility` Tauri commands.
  - `CGEventTap` is NOT created without permissions; hot-started after grant.
- [x] **2.8 UI Command Preparation**

---

## Phase 3: The High-Performance Scheduler

### Objective
Replace the current placeholder scheduler with a production-grade, deterministic timing engine. The scheduler must support sub-millisecond jitter control, adaptive timing modes (fixed interval, random jitter, human-like), and efficient multi-macro concurrency — all within the strict resource budget (< 60MB RAM, 0% idle CPU).

### Tasks

- [x] **3.1 Scheduler Architecture Design**
  - **Actor**: `@scheduler-agent`
  - **Details**: Define the scheduler's internal architecture. Use a single `tokio::select!` loop driven by a sorted timer wheel (or `BTreeMap<Instant, TaskId>`) to avoid per-task threads. Each macro maps to a `SchedulerTask` with its own timing config. The scheduler receives `SchedulerIntent`s (Start, Stop, UpdateConfig) from the StateActor and manages task lifecycles.

- [x] **3.2 Multi-Track Action Sequences**
  - **Actor**: `@scheduler-agent` + `@architect`
  - **Details**: Implemented `ActionSequence`, `ActionStep` (SustainedHold / InterleavedInterval), and `InputEvent` data model. Scheduler uses composite `StepId(macro_id, step_index)` keys for independent per-step timers. SustainedHolds fire HoldStart/HoldRelease immediately on macro start/stop. All types are `Serialize`/`Deserialize` for config persistence.
    1. **Fixed Interval**: Exact `Duration` between actions (e.g., click every 50ms).
    2. **Random Jitter**: Base interval ± configurable random offset (uniform distribution).
    3. **Human-like**: Variable delay using a Gaussian/Poisson distribution to simulate natural input patterns. Parameterized by mean and standard deviation.
  - All strategies must be deterministic when seeded (for testing/reproducibility).

- [x] **3.3 Action Dispatch**
  - **Actor**: `@scheduler-agent` + `@state-manager`
  - **Details**: When a timer fires, the scheduler sends an `ActionIntent` to the StateActor, which validates targeting rules (is the correct app focused? is the engine active?) before dispatching the actual input event via the `InputProvider` trait. This two-phase dispatch ensures the scheduler never bypasses state checks.

- [x] **3.4 Sub-millisecond Precision Audit**
  - **Actor**: `@scheduler-agent` + `@safety-officer`
  - **Details**: Profile `tokio::time::sleep` accuracy on macOS and Windows. If jitter exceeds 1ms at high frequencies (>100 actions/sec), implement a hybrid approach: `sleep` for the bulk of the interval, then busy-spin for the final sub-millisecond portion. Document the precision guarantees and tradeoffs.

- [x] **3.5 Multi-Macro Concurrency**
  - **Actor**: `@scheduler-agent`
  - **Details**: The scheduler must support N concurrent macros, each with independent timing. Use a single async task with a priority queue of next-fire times. On each iteration, `tokio::time::sleep_until(next_fire)`, execute the action, compute the next fire time, and re-insert. This avoids spawning N tasks.

- [x] **3.6 Pause/Resume & Drift Correction**
  - **Actor**: `@scheduler-agent`
  - **Details**: When a macro is paused (target app lost focus or engine toggled off), its timer is suspended. On resume, recalculate the next fire time relative to `Instant::now()` to avoid "catching up" with a burst of missed actions. This prevents the scheduler from flooding the input queue after a long pause.

- [x] **3.7 Scheduler IPC Commands**
  - **Actor**: `@ui-agent`
  - **Details**: Expose Tauri commands for the frontend to configure timing parameters per-macro: `set_timing_mode(macro_id, mode)`, `set_interval(macro_id, ms)`, `set_jitter_range(macro_id, min_ms, max_ms)`. Ensure all updates flow through the StateActor → Scheduler intent channel.

- [x] **3.8 Integration Testing**
  - **Actor**: `@scheduler-agent` + `@safety-officer`
  - **Details**: Write deterministic tests that verify: (a) actions fire at the correct intervals, (b) paused macros produce zero actions, (c) human-like mode stays within ±3σ of configured mean, (d) emergency stop halts all scheduled actions within 1 tick.

### Performance Constraints
- **Target**: < 60MB RAM usage.
- **CPU**: 0% usage while idle (no spinning, no polling).
- **Latency**: < 1ms jitter at 100 actions/sec for Fixed Interval mode.
- **Concurrency**: Support up to 32 concurrent macros on a single async task.

---

## Phase 4: UI Shell & Windows Muscle
*(Completed)*

### Objective
Implement the SolidJS frontend dashboard and the Windows-specific input injection backend.

### Tasks
- [x] **4.1 SolidJS Scaffolding**: Setup Vite + Tailwind + SolidJS. Build status dashboard.
- [x] **4.2 Windows InputProvider**: Implement `SendInput` logic in `platform/windows`.
- [x] **4.3 Binary Packaging**: Configure `tauri.conf.json` for production builds (DMG/NSIS).
- [x] **4.4 Safety Audit**: Ensure Windows user-mode input injection does not conflict with security software.

---

## Phase 5: Persistence & Profile Management
*(Completed - v1.0.0 Ready)*

### Objective
Implement a robust persistence layer to save macro configurations across sessions and provide a UI to manage these profiles.

### Tasks
- [x] **5.1 File System Bridge**: `ProfileManager` created in `src-tauri/src/persistence.rs` using Tauri's path resolver to save/load JSON configs.
- [x] **5.2 UI Config Manager**: "Profiles" tab added to SolidJS dashboard to save, load, and delete named profiles.
- [x] **5.3 Startup Restoration**: Engine automatically loads the "Default" profile on launch and replays macros into the `StateActor`.
- [x] **5.4 Final Performance Audit**: Confirmed via `cargo test` that a 1000-macro JSON config is < 500KB and deserializes cleanly without breaching the 60MB RAM limit.

---

## Phase 6: Public Release & CI/CD Deployment
*(Completed - v1.0.0 Public Release)*

### Objective
Ensure the repository is fully hardened, document the architecture for end-users, prepare professional branding assets, and establish an automated CI/CD pipeline for binary distribution.

### Tasks
- [x] **6.1 Repository Hardening**: `@safety-officer` audited the repository. Confirmed 0 hardcoded paths and verified the `bundle.identifier` (`com.alvaro.automux`) in `tauri.conf.json`.
- [x] **6.2 GitHub Actions Setup**: Configured `.github/workflows/release.yml` using `tauri-action` for automated `macos-latest` (DMG) and `windows-latest` (NSIS) builds on version tags.
- [x] **6.3 Documentation**: Co-authored a high-end `README.md` containing core features, installation steps, and a dedicated "Cybersecurity Audit" section detailing the `CGEventTap` and `SendInput` mechanisms.
- [x] **6.4 Branding**: Generated the full suite of Tauri application icons (macOS `.icns`, Windows `.ico`, and `.png` fallbacks) via `tauri icon`.
