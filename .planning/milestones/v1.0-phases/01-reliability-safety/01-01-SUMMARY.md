---
phase: 01-reliability-safety
plan: "01"
subsystem: reliability
tags: [accessibility, scheduler, interval-clamp, poll, frontend, rust]

# Dependency graph
requires: []
provides:
  - Accessibility permission grant detected within ~3 seconds via frontend setInterval poll (RELY-01)
  - Scheduler enforces 5ms minimum interval floor at both IntervalTask::new and UpdateInterval handler (SAFE-03)
affects: [02-reliability-safety, 03-reliability-safety, 04-reliability-safety]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Silent clamp pattern: scheduler-layer-only floor enforcement with no error/log/event on clamp (D-12)"
    - "Poll-only accessibility detection: setInterval with silent catch, no UI re-check button (D-02, D-03)"

key-files:
  created: []
  modified:
    - src/App.tsx
    - src-tauri/src/scheduler/mod.rs

key-decisions:
  - "Clamp at scheduler layer only (not IPC or frontend) — D-12 explicitly forbids frontend validation in Phase 1"
  - "Silent clamp: no error return, no log, no Tauri event on sub-5ms input — D-11 requirement"
  - "Poll-only accessibility detection: no Re-check button added — D-02 requirement"

patterns-established:
  - "Scheduler floor: interval_ms.max(5) / new_ms.max(5) at both IntervalTask::new and UpdateInterval handler"
  - "Accessibility poll: setInterval at 3000ms with silent catch inside createEffect with onCleanup clearInterval"

requirements-completed: [RELY-01, SAFE-03]

# Metrics
duration: 8min
completed: 2026-05-16
---

# Phase 01 Plan 01: Reliability & Safety — Accessibility Poll + Scheduler Floor Summary

**Accessibility grant detected within ~3 seconds via frontend poll fix (10s->3s), and 1-4ms intervals silently clamped to 5ms at both scheduler sites to prevent OS event queue saturation**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-16T12:18:00Z
- **Completed:** 2026-05-16T12:26:40Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Frontend accessibility poll shortened from 10000ms to 3000ms — users see permission grant reflected within ~3 seconds of returning to AutoMux without restarting (RELY-01, D-01)
- Scheduler `IntervalTask::new` changed from `interval_ms.max(1)` to `interval_ms.max(5)` — sub-5ms initial intervals clamped silently (SAFE-03, D-11)
- Scheduler `SchedulerIntent::UpdateInterval` handler changed from `new_ms.max(1)` to `new_ms.max(5)` — runtime interval updates also clamped (SAFE-03, D-11)
- All 3 existing scheduler unit tests (`jitter_audit_10ms_interval`, `afk_farm_stress_test`, `large_config_memory_check`) pass with the new floor — no test modifications needed

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix accessibility poll interval (RELY-01)** - `81e8f6c` (fix)
2. **Task 2: Enforce 5ms scheduler interval floor (SAFE-03)** - `406feb6` (fix)

## Files Created/Modified

- `src/App.tsx` — Changed `setInterval(..., 10000)` to `setInterval(..., 3000)` in the check_accessibility createEffect (line 133). One numeric literal change; no new imports, signals, or JSX added.
- `src-tauri/src/scheduler/mod.rs` — Changed `interval_ms.max(1)` to `interval_ms.max(5)` at line 66 (IntervalTask::new) and `new_ms.max(1)` to `new_ms.max(5)` at line 184 (SchedulerIntent::UpdateInterval handler). Two numeric literal changes; no new functions, error returns, or event emissions.

## Decisions Made

None beyond plan specification — plan provided complete implementation instructions. Both changes were exactly as specified (D-01 through D-04, D-11, D-12).

## Deviations from Plan

None — plan executed exactly as written. Both edits were single-token literal changes with no surrounding code modifications.

## Issues Encountered

None — `npx tsc --noEmit` passed with zero errors; `cargo build` and `cargo test` passed on first attempt.

## Verification Results

- `npx tsc --noEmit`: exit 0, no TypeScript errors
- `cargo build --manifest-path src-tauri/Cargo.toml`: exit 0
- `cargo test --manifest-path src-tauri/Cargo.toml`: 3 passed, 0 failed
- `grep -n "setInterval" src/App.tsx` confirms `}, 3000);` at line 133
- `grep -n "\.max(" src-tauri/src/scheduler/mod.rs` shows exactly two `.max(5)` occurrences, zero `.max(1)`

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- RELY-01 satisfied: accessibility poll at 3s, permission grant reflected without app restart
- SAFE-03 satisfied: sub-5ms scheduler intervals impossible, OS event queue protected
- Ready for Plan 02 (next reliability/safety plan)

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. T-01-01 (DoS via 1ms intervals) is now mitigated. T-01-02 (info disclosure via poll frequency) was pre-assessed as accept — shortening poll interval does not change data crossing the boundary.

## Self-Check: PASSED

- `src/App.tsx` exists and contains `}, 3000);` at line 133 (confirmed)
- `src-tauri/src/scheduler/mod.rs` exists with `.max(5)` at lines 66 and 184 (confirmed)
- Commit `81e8f6c` exists (fix(01-01): shorten accessibility poll interval from 10s to 3s)
- Commit `406feb6` exists (fix(01-01): enforce 5ms minimum interval floor in scheduler)

---
*Phase: 01-reliability-safety*
*Completed: 2026-05-16*
