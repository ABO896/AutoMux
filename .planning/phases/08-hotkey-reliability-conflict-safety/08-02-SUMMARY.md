---
phase: 08-hotkey-reliability-conflict-safety
plan: 02
subsystem: state-actor
tags: [rust, state-actor, conflict-detection, ux-11, ux-12, ipc, intent-handlers]

# Dependency graph
requires:
  - phase: 08-hotkey-reliability-conflict-safety
    plan: 01
    provides: "MacroConfig.trigger_modifiers, AppState.conflicts, InputConflict struct, tuple-keyed MACRO_TRIGGER_KEYS"
provides:
  - "check_trigger_key_conflict free function + StateActor wrapper (UX-11 trigger-key conflict detection)"
  - "recompute_conflicts free function + StateActor wrapper (UX-12 derived conflict-graph computation)"
  - "recompute_conflicts wired into 7 additional state-mutating intent handlers (RemoveMacro, SetMacroEnabled, SetMacroTargetApp, ResetEmergencyStop, ToggleMacroHotkey, UpdateSequence, LoadProfile)"
  - "Intent::SetMacroTriggerKey 3-tuple form carrying Option<u64> modifiers (CGEventFlags on macOS, MOD_* on Windows)"
  - "set_macro_trigger_key IPC command gained modifiers: Option<u64> parameter"
  - "Drop-on-conflict defense-in-depth in AddMacro and SetMacroTriggerKey (full Result<(), String> error path ships in plan 08-03)"
  - "Inline #[cfg(test)] mod tests with 4 unit tests pinning helper behavior"
affects:
  - "08-03 (Intent::BindHotkey / Intent::UnbindHotkey + new Windows HOTKEY_BINDINGS — will layer the explicit Result<(), String> error path on top of the drop-on-conflict groundwork from this plan)"
  - "08-04 (Frontend computeModifiers / hotkey IPC threading — the new modifiers: Option<u64> IPC parameter must be threaded through)"
  - "08-05 (UI surfaces for UX-12 conflicts — relies on AppState.conflicts being fresh on every state-changed event)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Free-function helpers in state module + thin StateActor-method wrappers — unit tests bypass Tauri AppHandle by calling free functions directly"
    - "Legacy-fallback expansion mirror in recompute_conflicts (empty sequence → single Left-click input, matching scheduler/mod.rs:257-270 exactly)"
    - "HashMap<InputEvent, Vec<Uuid>> accumulation → filter ≥2 → sort_unstable for deterministic frontend rendering"
    - "Drop-on-conflict defense-in-depth pattern for pre-Result-IPC error reporting (T-08-12 interim acceptance)"

key-files:
  created: []
  modified:
    - src-tauri/src/state/mod.rs
    - src-tauri/src/ipc/mod.rs

key-decisions:
  - "Free-function pattern: check_trigger_key_conflict(&AppState, ...) and recompute_conflicts(&mut AppState) are pub(crate) free functions; StateActor methods are 1-line wrappers. This matches the persistence test style (no StateActor/AppHandle needed in tests) and the plan's minimum-surface alternative."
  - "Used sort_unstable on Uuid::Ord for InputConflict.macros (NOT on the input bytes). Uuid::Ord sorts differently from insertion order — test assertion was relaxed to 'both ids present' instead of 'this specific order' to match the actual sort semantics."
  - "Drop-on-conflict as defense-in-depth (T-08-12): AddMacro and SetMacroTriggerKey drop the trigger (set to None/0) on conflict rather than failing the intent. The full Result<(), String> error path ships in plan 08-03's Intent::BindHotkey. The drop is a stopgap that keeps the user-facing flows working (AddMacro still creates the macro, just without a hotkey)."
  - "SetMacroTriggerKey: TriggerEmergencyStop is NOT in the recompute list (per plan). TriggerEmergencyStop has no reevaluate (engine is shutting down) so the conflict field can be left as-is — when the user resets, ResetEmergencyStop will recompute."

patterns-established:
  - "Pure-helper + StateActor-wrapper pattern: prefer free functions in the state module so unit tests can call them without a Tauri AppHandle; expose them as `impl StateActor` methods for intent handlers via 1-line delegation"
  - "Always recompute derived state immediately after the source-of-truth mutation, in the same intent handler (single-writer invariant)"

requirements-completed: [UX-11, UX-12]

# Metrics
duration: 5min
completed: 2026-06-30
---

# Phase 8 Plan 2: Conflict Detection Helpers & Wiring Summary

**UX-11 trigger-key conflict pre-checks and UX-12 derived `AppState.conflicts` field wired into all 9 state-mutating intent handlers, with `Intent::SetMacroTriggerKey` extended to carry `Option<u64>` modifiers for end-to-end hotkey+modifier matching.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-30T21:13:25Z
- **Completed:** 2026-06-30T21:17:51Z
- **Tasks:** 3
- **Files modified:** 2 (state/mod.rs, ipc/mod.rs)

## Accomplishments

- `check_trigger_key_conflict` and `recompute_conflicts` exist as `pub(crate)` free functions in `state/mod.rs` with 1-line `StateActor`-method wrappers — unit-testable without a Tauri AppHandle
- `recompute_conflicts` mirrors the scheduler's legacy-fallback expansion exactly (`scheduler/mod.rs:257-270`) so the derived conflict graph is consistent with what actually fires
- `Intent::AddMacro` and `Intent::SetMacroTriggerKey` pre-check the trigger key against other enabled macros before mutating; on conflict the trigger is dropped (interim, defense-in-depth per T-08-12 — full `Result<(), String>` error path lands in plan 08-03's `Intent::BindHotkey`)
- `Intent::SetMacroTriggerKey` signature changed from `(Uuid, Option<u16>)` to `(Uuid, Option<u16>, Option<u64>)` — `ipc/mod.rs::set_macro_trigger_key` gained the `modifiers: Option<u64>` parameter (CGEventFlags on macOS, MOD_* on Windows)
- `recompute_conflicts()` is now called in **9** state-mutating intent handlers: AddMacro, SetMacroTriggerKey (Task 2) + RemoveMacro, SetMacroEnabled, SetMacroTargetApp, ResetEmergencyStop, ToggleMacroHotkey, UpdateSequence, LoadProfile (Task 3)
- 4 new unit tests cover: self-rebind allowed, bind conflict rejected, conflict detection overlap, conflict disappears on disable
- Lib test count: 8/8 pass (4 new + 4 prior scheduler/persistence)

## Task Commits

1. **Task 1: Add `check_trigger_key_conflict` and `recompute_conflicts` helpers; add inline test module with 4 unit tests** — `e02a212` (feat)
2. **Task 2: Wire `check_trigger_key_conflict` and `recompute_conflicts` into `Intent::AddMacro` and `Intent::SetMacroTriggerKey`; extend `SetMacroTriggerKey` to carry `Option<u64>` modifiers** — `9b540c8` (feat)
3. **Task 3: Wire `recompute_conflicts()` into `Intent::LoadProfile` and 6 other state-mutating intent handlers (UX-12 coverage)** — `4ae5f2e` (feat)

**Plan metadata:** (this SUMMARY commit)

## Files Created/Modified

- `src-tauri/src/state/mod.rs` — Added `pub(crate) fn check_trigger_key_conflict(&AppState, ...)` and `pub(crate) fn recompute_conflicts(&mut AppState)` free functions; thin `StateActor` method wrappers; `Intent::SetMacroTriggerKey` signature change to 3-tuple form; conflict pre-checks in `AddMacro` and `SetMacroTriggerKey` handlers; `recompute_conflicts()` calls in 9 intent handlers total; inline `#[cfg(test)] mod tests` with 4 unit tests
- `src-tauri/src/ipc/mod.rs` — `set_macro_trigger_key` command gained `modifiers: Option<u64>` parameter and forwards to the 3-tuple `Intent::SetMacroTriggerKey`

## Decisions Made

- **Free-function + StateActor-wrapper pattern:** Both helpers are `pub(crate) free fn`s in `state/mod.rs`; the `StateActor` impl methods are 1-line delegations (`check_trigger_key_conflict(&self.state, ...)` and `recompute_conflicts(&mut self.state)`). This matches the persistence test style (`#[cfg(test)] mod tests` in `persistence.rs:251-319`) — no `StateActor`/`AppHandle`/channel plumbing needed in unit tests, just a default `AppState` and the free functions. Plan-endorsed as the "minimum-surface" alternative to the `StateActor::new_for_test` constructor approach.

- **`sort_unstable` on `Uuid::Ord` (not insertion order):** `recompute_conflicts` uses `macros.sort_unstable()` after the `HashMap<InputEvent, Vec<Uuid>>` accumulation. `Uuid`'s `Ord` impl is not the same as the byte ordering produced by `Uuid::new_v4()` sequential generation — the test assertion was relaxed to a set-membership check (`got == want` after sorting both) rather than a fixed-order comparison, matching the actual semantics.

- **Drop-on-conflict as defense-in-depth (T-08-12):** Both `AddMacro` and `SetMacroTriggerKey` silently drop the conflicting trigger (set to `None` / `0`) rather than failing the intent. The IPC channels for these commands don't currently return `Result<(), String>` for the conflict case — the existing `add_macro` returns `Result<Uuid, String>` where the `String` is the transport error, and `set_macro_trigger_key` returns `Result<(), String>` for transport only. Plan 08-03 ships `Intent::BindHotkey` with a `Sender<Result<(), String>>` that surfaces the explicit error to the frontend. Until that lands, the silent drop is the only path that doesn't break the existing flows; it is intentionally accepted (T-08-12 disposition = `accept (interim)`).

- **`TriggerEmergencyStop` not in the recompute list:** Per the plan, the engine-shutdown path has no `reevaluate_all_macros` (the engine is going down) and so `recompute_conflicts` is also not called there. The `ResetEmergencyStop` handler DOES recompute — when the user comes back from an emergency stop, the conflict field must reflect the new enabled/disabled state.

- **`ActiveAppChanged` not in the recompute list:** The conflict graph is computed on enabled macros regardless of target app. App changes don't touch the `enabled` flag, so the conflict field is still correct without recomputation. The plan explicitly carves this out.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test assertion order-dependence on Uuid sort order**
- **Found during:** Task 1 (first test run of `conflict_detection_overlap`)
- **Issue:** Plan example used `assert_eq!(state.conflicts[0].macros, vec![id_a, id_b])` but `recompute_conflicts` uses `sort_unstable` which sorts by `Uuid::Ord` (which differs from insertion order — Uuid's Ord is `(_, _, _, _)` field-by-field, not the byte order).
- **Fix:** Test assertion was relaxed: build `got` and `want` Vecs, sort both, then `assert_eq!`. The contract is "both ids present" not "this specific order" — matches the actual sort semantics. The 4 required test names (`self_rebind_allowed`, `bind_conflict_rejected`, `conflict_detection_overlap`, `conflict_disappear_on_disable`) are preserved verbatim.
- **Files modified:** src-tauri/src/state/mod.rs (test module)
- **Verification:** `cargo test -- conflict_detection_overlap` exits 0
- **Committed in:** `e02a212`

---

**Total deviations:** 1 auto-fixed (1 test-design bug)
**Impact on plan:** Test assertion change only — the helper's actual behavior (sort_unstable on Uuid::Ord) is correct and what the frontend needs. No production-code change required.

## Issues Encountered

None — all 9 intent-handler call sites, the IPC signature change, and the test module landed on the first compile attempt (after the test-order fix above). The `cargo test` count went from 4 → 8 (added 4 new tests; no prior tests broken). The only `cargo clippy` warning in the codebase is the pre-existing `unneeded return statement` in `src/ipc/mod.rs:126` flagged in plan 08-01's deviations — not introduced by this plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 08-03 (Intent::BindHotkey / Intent::UnbindHotkey + new Windows HOTKEY_BINDINGS) can proceed:
- `check_trigger_key_conflict` and `recompute_conflicts` are in place and unit-tested
- `Intent::SetMacroTriggerKey` is the 3-tuple form with the `Option<u64>` modifiers payload
- `set_macro_trigger_key` IPC accepts the `modifiers: Option<u64>` parameter
- The frontend's existing `set_macro_trigger_key` callers (App.tsx:376, 378) don't yet pass `modifiers` — Tauri 2 deserializes the missing parameter to `None` for `Option<u64>`, preserving the v2.0 default-profile behavior
- `recompute_conflicts` is called in all 9 state-mutating intent handlers, so the derived `conflicts` field is fresh on every `state-changed` event

Plan 08-03 will layer the explicit `Result<(), String>` error path on top of the drop-on-conflict groundwork (T-08-12 mitigation: the silent drop remains as defense-in-depth for callers that don't check the reply).

---
*Phase: 08-hotkey-reliability-conflict-safety*
*Completed: 2026-06-30*
