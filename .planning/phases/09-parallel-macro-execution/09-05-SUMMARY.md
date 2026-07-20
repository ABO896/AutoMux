---
phase: 09-parallel-macro-execution
plan: 05
subsystem: scheduler
tags: [rust, tokio, mpsc, async, reliability, gap-closure]

# Dependency graph
requires:
  - phase: 09-parallel-macro-execution (plans 01-04)
    provides: parallel-capable Scheduler/StateActor architecture, 09-VERIFICATION.md gap findings, 09-REVIEW.md CR-01 finding
provides:
  - Guaranteed-delivery HoldRelease on the per-macro stop path (release_holds), mirroring the already-shipped StopAll (.await) pattern
  - Saturation regression test proving the fix (stop_macro_release_delivered_under_saturation)
affects: [09-VERIFICATION.md re-verification, future scheduler/reliability work]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Guaranteed-delivery .await send for safety-critical release messages (HoldRelease) vs. best-effort try_send for self-correcting/non-leaking messages (Interval, HoldStart) — same split StopAll already established, now applied consistently to the per-macro stop path"

key-files:
  created: []
  modified:
    - src-tauri/src/scheduler/mod.rs

key-decisions:
  - "release_holds/stop_macro/start_macro converted to async fn; the three call sites (handle_intent StartMacro/StopMacro arms, start_macro's internal CR-04 restart) now .await them — mirrors StopAll's guaranteed-delivery pattern exactly per 09-VERIFICATION.md's explicit fix authorization"
  - "The two remaining try_send sites (fire_due_actions Interval, start_macro HoldStart) are intentionally left untouched per D-09 scope — neither is the stuck-input class this gap addresses"

patterns-established:
  - "Pattern: safety-critical release/cleanup sends use guaranteed .await; hot-path fire sends stay try_send with a debug-only drop counter"

requirements-completed: [EXEC-01, EXEC-02]

coverage:
  - id: D1
    description: "stop_macro/release_holds/start_macro converted to async with guaranteed .await HoldRelease delivery, mirroring StopAll — the per-macro stop path no longer silently drops HoldRelease under action-channel saturation"
    requirement: "EXEC-01"
    verification:
      - kind: unit
        ref: "src-tauri/src/scheduler/mod.rs#scheduler::tests::stop_macro_release_delivered_under_saturation"
        status: pass
      - kind: unit
        ref: "src-tauri/src/scheduler/mod.rs#scheduler::tests::afk_farm_stress_test"
        status: pass
      - kind: unit
        ref: "src-tauri/src/scheduler/mod.rs#scheduler::tests::jitter_audit_10ms_interval"
        status: pass
      - kind: unit
        ref: "src-tauri/src/scheduler/mod.rs#scheduler::tests::parallel_two_macros_concurrent"
        status: pass
      - kind: unit
        ref: "src-tauri/src/scheduler/mod.rs#scheduler::tests::parallel_stop_one_keeps_other"
        status: pass
    human_judgment: false
  - id: D2
    description: "EXEC-01/EXEC-02 device-level manual verification (macOS T9.1-T9.3, Windows 6.1-6.3) — out of scope for this plan, remains pending human execution on real hardware"
    requirement: "EXEC-01"
    verification: []
    human_judgment: true
    rationale: "Physical-device input-injection and permission verification cannot be automated in this environment; explicitly deferred by 09-VERIFICATION.md sections 5/6, unchanged by this gap-closure plan."

duration: 8min
completed: 2026-07-20
status: complete
---

# Phase 09 Plan 05: Guaranteed HoldRelease Delivery on Per-Macro Stop Path Summary

**Converted `release_holds`/`stop_macro`/`start_macro` in the scheduler to guaranteed `.await` delivery for `HoldRelease`, closing the 09-VERIFICATION.md CR-01 stuck-input gap, with a saturation regression test proving RED→GREEN.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-20T19:32:00Z
- **Completed:** 2026-07-20T19:40:48Z
- **Tasks:** 2 completed
- **Files modified:** 1 (`src-tauri/src/scheduler/mod.rs`)

## Accomplishments
- Added `stop_macro_release_delivered_under_saturation`, a regression test using a capacity-1 action channel to force `release_holds`'s send to contend for the single slot; drains via `recv().await` (not `try_recv`) so a blocked `.await` send can make progress. Confirmed RED against the pre-fix code: `hold_releases == 0` (release dropped under saturation).
- Converted `release_holds`, `stop_macro`, and `start_macro` to `async fn`, and switched `release_holds`'s `HoldRelease` send from fire-and-forget `try_send` to guaranteed `self.action_tx.send(...).await`, mirroring the already-shipped `StopAll` (CR-02) pattern exactly. Updated all three call sites (`handle_intent`'s `StartMacro`/`StopMacro` arms, and `start_macro`'s internal CR-04 restart call) to `.await`.
- Confirmed GREEN: the new saturation test now passes (`hold_releases == 1`), and all 4 pre-existing scheduler tests (`afk_farm_stress_test`, `jitter_audit_10ms_interval`, `parallel_two_macros_concurrent`, `parallel_stop_one_keeps_other`) still pass unchanged.
- `cargo clippy --all-targets -- -D warnings` clean; `cargo build --release` compiles.
- The two D-09-scoped diagnostic `try_send` sites (`fire_due_actions` Interval fire, `start_macro` HoldStart) are untouched — still using `try_send` with their `ACTION_DROP_COUNT`/`eprintln` debug-only drop counter.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add a failing saturation regression test proving HoldRelease is delivered when action_tx is full (RED)** - `a2e59b4` (test)
2. **Task 2: Harden the per-macro stop path to guaranteed .await delivery, mirroring StopAll (GREEN)** - `6e470d0` (fix)

**Plan metadata:** (this commit)

## Files Created/Modified
- `src-tauri/src/scheduler/mod.rs` - Added saturation regression test; converted `release_holds`/`stop_macro`/`start_macro` to async with guaranteed `.await` `HoldRelease` delivery, mirroring `StopAll`'s existing CR-02 pattern.

## Decisions Made
- Converted `release_holds`/`stop_macro`/`start_macro` to `async fn` rather than introducing a separate guaranteed-delivery helper — matches the plan's explicit instruction to mirror `StopAll`'s exact form (`let _ = self.action_tx.send(...).await;`) and keeps the change minimal (no new abstractions).
- Removed the `#[cfg(debug_assertions)]` `ACTION_DROP_COUNT`/`eprintln` block only at the `release_holds` site — under guaranteed `.await` delivery there is no backpressure-drop condition left to count at that site (the send only errors on receiver-closed/shutdown, not backpressure). The static, its accessor, and the two other diagnostic sites (`fire_due_actions` Interval, `start_macro` HoldStart) remain unchanged.

## Deviations from Plan

None - plan executed exactly as written. Both tasks' acceptance criteria (structural greps, RED/GREEN test behavior, clippy, release build) were verified and passed.

## Issues Encountered

None. The plan's grep-based acceptance criteria for `try_send` count (expected exactly 2) and `ACTION_DROP_COUNT.fetch_add` count (expected exactly 2) reported different raw counts (7 and 1 respectively) due to grep matching substrings inside doc comments (e.g., "not try_send" in the new `@safety-officer` comment, "try_send" in the new test's doc comment) and a pre-existing multi-line `ACTION_DROP_COUNT\n.fetch_add(...)` formatting split at the `fire_due_actions` site that a single-line grep doesn't match. Manually inspecting the matched lines confirmed the substantive intent is met: exactly 2 real `try_send(ActionReady { ... })` call sites remain (line 304 `start_macro`'s HoldStart, line 406 `fire_due_actions`'s Interval), and both retained `ACTION_DROP_COUNT.fetch_add` call sites are present and unchanged.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The 09-VERIFICATION.md CR-01 Blocker gap (silently-dropped HoldRelease under saturation) is closed at the automated-verification level: `cargo test --lib`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --release` all pass.
- Phase 9's automated gates are now fully green across all 5 plans. The 6 device-level manual tests (macOS T9.1-T9.3, Windows 6.1-6.3) in 09-VERIFICATION.md §5/§6 remain pending human verification on real hardware — unchanged and unaddressed by this plan, consistent with its stated scope.
- EXEC-01/EXEC-02 remain in "Active" status in PROJECT.md pending that device verification; this plan hardens the reliability half of those requirements but does not itself close them.

---
*Phase: 09-parallel-macro-execution*
*Completed: 2026-07-20*
