---
phase: 09-parallel-macro-execution
plan: 11
subsystem: input-reliability
tags: [rust, tauri, tokio, statemachine, hotkeys, cgevent, sendinput, solidjs]

# Dependency graph
requires:
  - phase: 09-parallel-macro-execution (plans 01-10)
    provides: StateActor two-phase dispatch, guaranteed-delivery HoldRelease .await sends (plan 05/06), single HOTKEY_BINDINGS registry (plan 10), BindHotkey Result-oneshot conflict pattern (Phase 8)
provides:
  - "action_should_inject: pure gate-decision helper — HoldRelease always bypasses Gate 1/2/3; new-input actions stay gated"
  - "resolve_trigger_key_update: pure Ok/Err resolver for trigger-key rebinds — no silent coercion"
  - "Intent::SetMacroTriggerKey Result-carrying oneshot reply, mirroring Intent::BindHotkey"
  - "macOS card-edit rebind flow with no pre-unbind — relies on bind_hotkey's overwrite-on-success/preserve-on-conflict semantics"
affects: [09-VERIFICATION.md re-verification, EXEC-01, EXEC-02, any future Windows hotkey-rebind or HoldRelease-path work]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Free-function + StateActor-wrapper for testability (P-5): action_should_inject and resolve_trigger_key_update are pure fns over &AppState, unit-testable without a Tauri AppHandle"
    - "Result-carrying oneshot reply on conflict (mirrors BindHotkey): reject-and-report instead of silently coerce-and-apply"

key-files:
  created: []
  modified:
    - src-tauri/src/state/mod.rs
    - src-tauri/src/ipc/mod.rs
    - src/App.tsx

key-decisions:
  - "action_should_inject placed as a module-scope free fn (not a StateActor method) so the four confirmed trigger-path scenarios can be driven directly in unit tests without constructing a StateActor (which needs a Tauri AppHandle)"
  - "resolve_trigger_key_update called directly (no StateActor wrapper) since it already takes &AppState by reference — the P-5 wrapper pattern is only needed when call sites want self.foo() ergonomics, and this handler is the sole caller"
  - "debug-only eprintln in handle_action re-derives the macro name via a defensive lookup (falls back to '<removed>') since the refactor no longer binds `mac` before the match — required because the profile-load trigger path removes the macro before its release arrives"

patterns-established:
  - "Guaranteed-delivery release + unconditional-injection gate: guaranteed .await delivery (plans 05/06) is necessary but not sufficient — the receiving gate must also refuse to drop a release. Both halves are now closed for HoldRelease."

requirements-completed: [EXEC-01, EXEC-02]

coverage:
  - id: D1
    description: "HoldRelease reaching handle_action always injects (bypasses Gate 1/2/3), regardless of engine/enabled/target/absent-macro state; new-input actions (Interval/HoldStart) remain gated"
    requirement: "EXEC-01, EXEC-02"
    verification:
      - kind: unit
        ref: "src-tauri/src/state/mod.rs#hold_release_bypasses_gates"
        status: pass
    human_judgment: false
  - id: D2
    description: "A conflicting Windows hotkey rebind (Intent::SetMacroTriggerKey) is rejected via Err reply instead of silently coerced to None/0; the existing binding survives"
    requirement: "EXEC-02"
    verification:
      - kind: unit
        ref: "src-tauri/src/state/mod.rs#set_trigger_key_rejects_conflict_without_coercion"
        status: pass
    human_judgment: false
  - id: D3
    description: "macOS card-edit hotkey rebind no longer pre-unbinds before bind_hotkey — a conflict-rejected rebind leaves the old binding intact"
    requirement: "EXEC-01"
    verification:
      - kind: unit
        ref: "grep -c invoke(\"unbind_hotkey\" src/App.tsx == 0"
        status: pass
    human_judgment: true
    rationale: "The static grep + tsc checks prove the code shape, but confirming the actual UX (toast shown, binding preserved) after a real conflicting rebind requires interacting with the running app on-device — real-device SC1/SC2 confirmation remains human_needed per this plan's flagged assumptions."

# Metrics
duration: 10min
completed: 2026-07-22
status: complete
---

# Phase 09 Plan 11: Gap Closure — HoldRelease Bypass + Hotkey Rebind Rollback Summary

**Unconditional HoldRelease injection bypass in StateActor::handle_action plus reject-on-conflict hotkey rebinds on both platforms — closes the two confirmed Blocker gaps from 09-VERIFICATION.md.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-07-22T18:00:00Z (approx)
- **Completed:** 2026-07-22T18:10:24Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments
- `handle_action` now consults a pure `action_should_inject` gate helper whose first check is an unconditional bypass for `ActionType::HoldRelease` — a held key/mouse-button is released even if the macro was disabled, the engine was toggled off, the active app changed away from the macro's target, or the macro was removed by a profile load, all before the release arrived. New-input actions (`Interval`/`HoldStart`) remain fully gated.
- `Intent::SetMacroTriggerKey` now carries a `tokio::sync::oneshot::Sender<Result<(), String>>` (mirroring `BindHotkey`) and rejects a genuinely conflicting rebind with an `Err` message instead of silently coercing the key to `None`/`0` — the old binding is preserved and `ipc::set_macro_trigger_key` now propagates the `Err` to the frontend.
- `handleCardSetTriggerKey`'s macOS branch no longer calls `unbind_hotkey` before `bind_hotkey` — a conflict-rejected rebind leaves the macro's previously-working hotkey intact, relying on `bind_hotkey`'s existing overwrite-on-success / preserve-on-conflict semantics.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add an unconditional HoldRelease bypass to handle_action (Gap 1)** - `ef4546f` (feat)
2. **Task 2: Reject conflicting hotkey rebinds on Windows via a Result-carrying oneshot (Gap 2 — backend)** - `986fc32` (feat)
3. **Task 3: Stop the macOS rebind from pre-unbinding the old binding (Gap 2 — frontend)** - `8a7a66d` (fix)

**Plan metadata:** (pending — this commit)

## Files Created/Modified
- `src-tauri/src/state/mod.rs` - Added `action_should_inject` (pure gate-decision helper with HoldRelease bypass) and `resolve_trigger_key_update` (pure Ok/Err resolver); refactored `handle_action` to consult the former; rewrote `Intent::SetMacroTriggerKey` to a Result-carrying oneshot handler using the latter; added `hold_release_bypasses_gates` and `set_trigger_key_rejects_conflict_without_coercion` regression tests
- `src-tauri/src/ipc/mod.rs` - `set_macro_trigger_key` rewired to the oneshot Result reply pattern (mirrors `bind_hotkey`)
- `src/App.tsx` - `handleCardSetTriggerKey`'s macOS branch no longer pre-unbinds before `bind_hotkey`; refreshed the stale `@architect` comment

## Decisions Made
- `action_should_inject` and `resolve_trigger_key_update` are module-scope free functions taking `&AppState` directly (P-5 pattern) rather than `StateActor` methods, so all four Gap-1 trigger-path scenarios and all four Gap-2 conflict/clear/self-rebind scenarios are unit-testable without constructing a `StateActor` (which requires a Tauri `AppHandle`).
- The debug-only `eprintln!` in `handle_action` re-derives the macro name via a defensive `self.state.macros.get(&action.macro_id)` lookup (falling back to `"<removed>"`) since the refactor no longer binds `mac` unconditionally before the match — necessary because the profile-load trigger path removes the macro entirely before its release action arrives, and the old code would have panicked on `mac.name` in that case.

## Deviations from Plan

None - plan executed exactly as written. All three tasks, their `<action>` prescriptions, and their regression tests were implemented per the plan's explicit code-level instructions.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `cargo test --manifest-path src-tauri/Cargo.toml --lib` — 16/16 tests pass (14 pre-existing + `hold_release_bypasses_gates` + `set_trigger_key_rejects_conflict_without_coercion`).
- `cargo build --manifest-path src-tauri/Cargo.toml` — compiles clean, zero warnings.
- `npx tsc --noEmit` — clean.
- `grep -c 'invoke("unbind_hotkey"' src/App.tsx` — returns 0.
- Both confirmed Blocker gaps from 09-VERIFICATION.md are closed at the source level. Per this plan's flagged assumptions, real-device concurrent-firing confirmation (ROADMAP SC1 macOS / SC2 Windows) and the REQUIREMENTS.md EXEC-01/EXEC-02 status-row reconciliation remain deliberately deferred to the post-fix re-verification pass (`09-VERIFICATION.md` NOTE) — this plan does not edit REQUIREMENTS.md.
- Ready for `/gsd-execute-phase 09` re-verification or a fresh `09-VERIFICATION.md` pass to confirm no further gaps.

---
*Phase: 09-parallel-macro-execution*
*Completed: 2026-07-22*
