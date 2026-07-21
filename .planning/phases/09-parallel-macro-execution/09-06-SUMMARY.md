---
phase: 09-parallel-macro-execution
plan: 6
subsystem: scheduler
tags: [rust, tokio, mpsc, scheduler, reliability, tdd]

# Dependency graph
requires:
  - phase: 09-parallel-macro-execution
    provides: "Plan 09-05's release-side CR-01 fix (release_holds/StopAll guaranteed .await HoldRelease delivery) — this plan mirrors that pattern on the start side."
provides:
  - "Guaranteed-delivery HoldStart on the start_macro SustainedHold path, closing the last Blocker-severity gap (CR-01) for Phase 9"
  - "Saturation regression test proving no macro can be recorded as 'held' in active_holds without HoldStart actually being delivered"
affects: [scheduler-reliability, hold-mode-macros]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Guaranteed .await delivery for all hold-lifecycle transitions (HoldStart, HoldRelease, StopAll) — try_send reserved only for the periodic Interval fire path, where a dropped tick self-corrects on the next interval"

key-files:
  created: []
  modified:
    - src-tauri/src/scheduler/mod.rs

key-decisions:
  - "Converted start_macro's SustainedHold HoldStart send from try_send to guaranteed .await, mirroring release_holds/StopAll — symmetric hold-lifecycle delivery guarantee"
  - "Removed the now-obsolete HoldStart-site ACTION_DROP_COUNT increment/eprintln; the fire_due_actions Interval site remains the sole legitimate try_send/drop-count path"
  - "Added @safety-officer doc comment on the SustainedHold arm documenting the CR-01 rationale and the accepted select!-loop blocking trade-off, explicitly distinguishing it from the Interval path's self-correcting drop tolerance"

patterns-established:
  - "Hold-lifecycle sends (HoldStart/HoldRelease/StopAll) always use guaranteed .await; only the self-correcting periodic Interval tick uses try_send with debug-only drop counting"

requirements-completed: [EXEC-01, EXEC-02]

coverage:
  - id: D1
    description: "start_macro's SustainedHold branch delivers HoldStart via guaranteed .await instead of fire-and-forget try_send, closing the phantom-held gap (CR-01)"
    requirement: "EXEC-01"
    verification:
      - kind: unit
        ref: "src-tauri/src/scheduler/mod.rs#start_macro_hold_start_delivered_under_saturation"
        status: pass
    human_judgment: false
  - id: D2
    description: "Full backend regression surface (13 lib tests, clippy -D warnings, release build) remains green with no collateral regression to the already-hardened release side"
    requirement: "EXEC-02"
    verification:
      - kind: unit
        ref: "cargo test --lib (13 passed)"
        status: pass
      - kind: other
        ref: "cargo clippy --all-targets -- -D warnings"
        status: pass
      - kind: other
        ref: "cargo build --release"
        status: pass
    human_judgment: false

# Metrics
duration: 4min
completed: 2026-07-21
status: complete
---

# Phase 9 Plan 6: HoldStart Guaranteed-Delivery Fix (CR-01) Summary

**Converted `start_macro`'s SustainedHold branch from fire-and-forget `try_send` to guaranteed `.await` HoldStart delivery, closing the last Blocker-severity CR-01 gap with a RED→GREEN saturation regression test.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-07-21T21:43:00+02:00 (approx, first commit 21:44:11+02:00)
- **Completed:** 2026-07-21T21:45:43+02:00
- **Tasks:** 2 completed (Task 1: TDD RED→GREEN fix; Task 2: regression sweep, verification-only)
- **Files modified:** 1 (`src-tauri/src/scheduler/mod.rs`)

## Accomplishments

- Closed the last remaining Blocker-severity Phase 9 gap (09-VERIFICATION.md truth #8 / 09-REVIEW.md CR-01): `start_macro`'s `SustainedHold` arm now sends `HoldStart` via `self.action_tx.send(...).await` (guaranteed delivery), symmetric with the already-hardened `release_holds`/`StopAll` paths shipped in plan 09-05.
- Added `start_macro_hold_start_delivered_under_saturation`, a capacity-1-channel regression test proving both macros' `HoldStart` (and subsequent `HoldRelease`) are delivered even when `action_tx` is saturated at start time. RED confirmed the bug (`hold_starts` observed 1, expected 2) before the fix; GREEN confirmed the fix (`hold_starts` == 2, `hold_releases` == 2).
- Removed the now-obsolete HoldStart-site `ACTION_DROP_COUNT` increment/`eprintln!` — the `fire_due_actions` Interval-fire path remains the sole legitimate `try_send`/drop-counting site (self-correcting ticks, unlike a hold transition).
- Full backend regression surface confirmed green: 13/13 lib tests pass (12 pre-existing + 1 new), `cargo clippy --all-targets -- -D warnings` clean, `cargo build --release` clean.

## Task Commits

Each task was committed atomically:

1. **Task 1 (RED): add failing saturation test** - `79d39f8` (test)
2. **Task 1 (GREEN): guarantee HoldStart delivery** - `5e37063` (fix)

Task 2 (full regression sweep and symmetry guard) produced no code changes — it is a verification-only task confirming the Task 1 fix left no collateral regressions. No separate commit was needed for Task 2.

**Plan metadata:** see final commit below.

## Files Created/Modified

- `src-tauri/src/scheduler/mod.rs` — `start_macro`'s `SustainedHold` arm converted to guaranteed `.await` HoldStart delivery with a new `@safety-officer:` doc comment citing CR-01; new `start_macro_hold_start_delivered_under_saturation` `#[tokio::test]` added to `#[cfg(test)] mod tests`; obsolete HoldStart-site `ACTION_DROP_COUNT` increment removed.

## Decisions Made

- Followed the plan's exact mirror pattern: `self.action_tx.send(ActionReady { macro_id, action_type: ActionType::HoldStart(*input), fired_at: Instant::now() }).await` replacing the `try_send`+drop-count block, with `holds.push(*input)` retained immediately after — matching `release_holds`'s post-send flow.
- Kept the RED test's assertion message and structure closely mirroring `stop_macro_release_delivered_under_saturation` for consistency and future auditability.
- No architectural changes — this was a pure internal-channel-delivery hardening fix within an already-established two-phase-dispatch pattern.

## Deviations from Plan

### Minor — verification command literal-match nuance (not a code defect)

The plan's Task 2 automated verify command includes `grep -c 'ACTION_DROP_COUNT.fetch_add(1' src/scheduler/mod.rs` expecting exactly `1`. On this codebase's pre-existing rustfmt formatting, the one remaining call site (in `fire_due_actions`) wraps across two lines:
```rust
ACTION_DROP_COUNT
    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
```
BSD `grep` (macOS default, no `-P`/PCRE multi-line support) cannot match this literal single-line pattern across the wrap, so the literal command returns `0`, not `1`. This line-wrap is pre-existing formatting (confirmed unchanged from `HEAD~1`, i.e. it existed before this plan's edits and was never touched by Task 1). The semantic property the acceptance criteria actually cares about — "the `fetch_add(1` call exists exactly once in the file, at the `fire_due_actions` Interval site" — was verified with `grep -c 'fetch_add(1' src/scheduler/mod.rs` → `1`, which is the equivalent-intent check. No code change was made or needed; this is a note for future verification-script authors that BSD grep's single-line matching does not span rustfmt's wrapped `.fetch_add(...)` call chains.

**Total deviations:** 0 code deviations; 1 documentation note on a verification-command matching nuance.
**Impact on plan:** None on scope or correctness — all acceptance criteria are met by source and confirmed by an equivalent grep check.

## Issues Encountered

None — the RED→GREEN cycle worked exactly as specified on the first attempt: RED reproduced the documented failure (`hold_starts` left=1, right=2, matching the plan's exact expected discriminator), and GREEN passed immediately after the mirrored fix.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The last Blocker-severity reliability gap for Phase 9 (CR-01, asymmetric HoldStart) is closed. `active_holds`/`computeRunningState` can no longer report a Hold-mode macro as "held" without the underlying `HoldStart` having been genuinely delivered, outside of harmless receiver-shutdown races.
- Phase 9's three ROADMAP success criteria (SC1/SC2/SC3) still require human device-level verification on real macOS and Windows hosts — explicitly out of scope for this plan and unaffected by it, per 09-VERIFICATION.md Sections 5/6.
- No frontend or unrelated backend files were touched; `src/App.tsx`, `src-tauri/src/lib.rs`, and `src-tauri/src/ipc/mod.rs` are unchanged.

---
*Phase: 09-parallel-macro-execution*
*Completed: 2026-07-21*
