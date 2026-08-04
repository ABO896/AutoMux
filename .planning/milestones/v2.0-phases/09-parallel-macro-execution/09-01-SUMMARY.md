---
phase: 09-parallel-macro-execution
plan: 01
subsystem: scheduler
tags: [tokio, rust, scheduler, action-channel, atomic, ipc]

# Dependency graph
requires:
  - phase: 08-hotkey-reliability-conflict-safety
    provides: trigger_modifiers field on MacroConfig, StateActor free-function testability pattern
provides:
  - Unit-level proof that the scheduler runs multiple macros concurrently and that stopping one macro does not affect another (EXEC-01/EXEC-02 backstop verification)
  - action_tx channel capacity raised 100 -> 1024 for the higher parallel fire rate
  - Debug-only ACTION_DROP_COUNT counter + get_debug_action_drop_count IPC command for diagnosing action-channel overflow
affects: [09-02-running-state-ui, 09-03-manual-verification]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Debug-only AtomicU64 diagnostic counter gated by #[cfg(debug_assertions)], paired with an eprintln! log line and a debug-only accessor fn — mirrors the existing state/mod.rs:397-398 [Action] logging convention"
    - "Bounded-range per-macro_id fire-count assertions in tokio scheduler tests (never exact counts or timing-based assertions)"

key-files:
  created: []
  modified:
    - src-tauri/src/scheduler/mod.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/ipc/mod.rs

key-decisions:
  - "Fixed a pre-existing needless_return clippy lint in ipc/mod.rs::list_running_apps (Rule 3 blocking-issue fix) because it blocked the cargo clippy --all-targets -D warnings gate required by this plan's own tasks, even though the lint was unrelated to Phase 9 changes"

patterns-established:
  - "Debug-only try_send drop counter pattern: convert `let _ = tx.try_send(...)` to `if tx.try_send(...).is_err() { #[cfg(debug_assertions)] { COUNTER.fetch_add(1, Relaxed); eprintln!(...); } }` — applicable to any other fire-and-forget channel in the codebase that later needs overflow diagnostics"

requirements-completed: [EXEC-01, EXEC-02]

coverage:
  - id: D1
    description: "Two macros at different intervals (50ms, 80ms) fire concurrently in the scheduler, proven by a bounded per-macro_id fire-count assertion that a serialized implementation could not satisfy"
    requirement: "EXEC-01"
    verification:
      - kind: unit
        ref: "src-tauri/src/scheduler/mod.rs#parallel_two_macros_concurrent"
        status: pass
    human_judgment: false
  - id: D2
    description: "Stopping one running macro does not affect a second concurrently running macro"
    requirement: "EXEC-02"
    verification:
      - kind: unit
        ref: "src-tauri/src/scheduler/mod.rs#parallel_stop_one_keeps_other"
        status: pass
    human_judgment: false
  - id: D3
    description: "action_tx channel capacity raised from 100 to 1024 to reduce overflow risk under parallel macro fire rates"
    verification:
      - kind: unit
        ref: "src-tauri/src/lib.rs (mpsc::channel::<scheduler::ActionReady>(1024))"
        status: pass
    human_judgment: false
  - id: D4
    description: "Debug-only ACTION_DROP_COUNT counter increments and logs on try_send failure at all 3 fire sites (HoldStart, HoldRelease, Interval); debug-only IPC command exposes it; both are absent from release builds"
    verification:
      - kind: unit
        ref: "cargo build --release (exit 0, static/accessor/command compiled out cleanly)"
        status: pass
    human_judgment: false

duration: 5min
completed: 2026-07-20
status: complete
---

# Phase 9 Plan 01: Scheduler Parallel-Execution Proof + Action-Channel Hardening Summary

**Two new tokio tests proving concurrent macro execution and independent-stop behavior in the scheduler, action_tx capacity raised 100->1024, and a debug-only AtomicU64 drop counter + IPC accessor for diagnosing overflow.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-07-20T17:40:27Z
- **Completed:** 2026-07-20T17:44:43Z
- **Tasks:** 3
- **Files modified:** 3 (src-tauri/src/scheduler/mod.rs, src-tauri/src/lib.rs, src-tauri/src/ipc/mod.rs)

## Accomplishments
- `parallel_two_macros_concurrent` and `parallel_stop_one_keeps_other` tests prove the single-scheduler BTreeMap timeline genuinely runs multiple macros at independent intervals without serializing, and that stopping one macro leaves another running macro unaffected (EXEC-01/EXEC-02 unit-level proof)
- `action_tx` channel capacity raised from 100 to 1024 in `lib.rs`, scoped only to the channel flagged in `CONCERNS.md` as most likely to overflow under parallel execution (`state_tx`/`sched_tx` left untouched per D-11)
- Debug-only `ACTION_DROP_COUNT` `AtomicU64` instruments all 3 `try_send` fire sites (`start_macro` HoldStart, `release_holds` HoldRelease, `fire_due_actions` Interval) with a Relaxed-ordering increment + `[Scheduler]` eprintln log, and a debug-only `get_action_drop_count()` accessor + `get_debug_action_drop_count` IPC command expose the count — all fully compiled out of release builds

## Task Commits

Each task was committed atomically:

1. **Task 1: Add parallel_two_macros_concurrent + parallel_stop_one_keeps_other tests** - `aa4a185` (test)
2. **Task 2: action_tx capacity 100->1024 + debug-only ACTION_DROP_COUNT counter** - `36c3fd5` (feat)
3. **Task 3: get_debug_action_drop_count debug-only IPC command + registration** - `29d913e` (feat)

## Files Created/Modified
- `src-tauri/src/scheduler/mod.rs` - Two new `#[tokio::test]` functions; `ACTION_DROP_COUNT` static + `get_action_drop_count()` accessor; all 3 `try_send` sites converted to check-and-count-on-failure form
- `src-tauri/src/lib.rs` - `action_tx` channel capacity literal 100 -> 1024; new `#[cfg(debug_assertions)]`-gated `ipc::get_debug_action_drop_count` entry in `generate_handler!`
- `src-tauri/src/ipc/mod.rs` - New debug-only `get_debug_action_drop_count` command; unrelated pre-existing `needless_return` clippy fix in `list_running_apps`

## Decisions Made
- Applied the exact bounded-range assertion style from the existing `afk_farm_stress_test`/`jitter_audit_10ms_interval` tests (D-07) rather than timing-based assertions, per plan and CONTEXT.md D-07's explicit rejection of fragile timing assertions
- Kept both new test scenarios as separate `#[tokio::test]` functions (not combined) per D-06, for isolated failure reporting

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed pre-existing needless_return clippy lint in ipc/mod.rs**
- **Found during:** Task 2 (running `cargo clippy --all-targets -- -D warnings` as this task's verification gate)
- **Issue:** `list_running_apps` (pre-existing code, unrelated to Phase 9) used `return crate::platform::macos::observer::list_running_apps_impl();` inside a `#[cfg(target_os = "macos")]` block, which clippy's `needless_return` lint (implied by `-D warnings`) rejected. This blocked the `cargo clippy --all-targets -- -D warnings` gate required by both Task 2's and Task 3's `<verify>` commands.
- **Fix:** Removed the `return` keyword and trailing semicolon so the block's tail expression is returned implicitly.
- **Files modified:** src-tauri/src/ipc/mod.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` exits 0 after the fix; confirmed via `git stash` that the lint predates this plan's changes.
- **Committed in:** 36c3fd5 (Task 2 commit)

**2. [Rule 1 - Bug] Fixed manual_range_contains clippy lint in own new test code**
- **Found during:** Task 2 (same clippy gate run)
- **Issue:** The two new tests from Task 1 used `a_count >= 5 && a_count <= 16` style bound checks, which clippy's `manual_range_contains` lint flags in favor of `(5..=16).contains(&a_count)`.
- **Fix:** Converted both bound checks in `parallel_two_macros_concurrent` to `RangeInclusive::contains` form. No change to test semantics or the documented bound reasoning.
- **Files modified:** src-tauri/src/scheduler/mod.rs
- **Verification:** `cargo test parallel_` still passes 2/2; clippy clean.
- **Committed in:** 36c3fd5 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking pre-existing lint, 1 lint in own new code)
**Impact on plan:** Both fixes were required strictly to satisfy this plan's own `-D warnings` verification gates. No scope creep — no behavior changed, no files outside the plan's `files_modified` touched beyond the already-planned `ipc/mod.rs`.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 09-02 (running-state UI) can proceed — no dependency on this plan's changes beyond the shared `AppState`/`MacroConfig` shapes, which are unchanged
- Plan 09-03 (manual device verification / 09-VERIFICATION.md) can reference `parallel_two_macros_concurrent` and `parallel_stop_one_keeps_other` as the unit-level backstop for EXEC-01/EXEC-02, plus the new `get_debug_action_drop_count` command for on-device diagnosis if overflow is suspected during manual testing
- No blockers

---
*Phase: 09-parallel-macro-execution*
*Completed: 2026-07-20*

## Self-Check: PASSED

All created/modified files exist on disk (src-tauri/src/scheduler/mod.rs, src-tauri/src/lib.rs, src-tauri/src/ipc/mod.rs, this SUMMARY.md). All 4 commits (aa4a185, 36c3fd5, 29d913e, 2c8a238) verified present in git log.
