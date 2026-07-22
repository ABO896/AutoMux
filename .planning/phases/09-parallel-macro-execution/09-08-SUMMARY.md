---
phase: 09-parallel-macro-execution
plan: 8
subsystem: ui
tags: [solidjs, tauri-ipc, macro-management, gap-closure]

# Dependency graph
requires:
  - phase: 09-parallel-macro-execution
    provides: "remove_macro IPC command fully implemented and registered on the Rust backend (Intent::RemoveMacro, AppState.macros removal, scheduler stop, auto_save_default) — this plan's only gap was the missing frontend call site"
provides:
  - "Per-macro-card delete control (confirm + invoke remove_macro) closing G-09-1b"
  - "Controlled card-edit target-app <select> pre-selecting the macro's current target, closing the secondary G-09-1c UI display bug"
affects: [10-ui-redesign]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Native window.confirm as the minimal interim destructive-action guard where no modal/toast helper exists on a given tab"

key-files:
  created: []
  modified:
    - src/App.tsx

key-decisions:
  - "Used window.confirm rather than building a modal — the dashboard tab has no such helper and Phase 10 owns the full UI redesign (UI-03); minimal interim affordance per plan's explicit prohibition against new dialog components."

patterns-established: []

requirements-completed: [EXEC-01, EXEC-02]

coverage:
  - id: D1
    description: "Delete control on each macro card: click ✕, confirm, macro removed from live list via existing state-changed broadcast and persisted removal (does not reappear on restart)"
    requirement: EXEC-01
    verification:
      - kind: automated_ui
        ref: "tsc --noEmit -p tsconfig.json && npm run build && grep -c 'remove_macro' src/App.tsx"
        status: pass
    human_judgment: true
    rationale: "Behavioral truth (card disappears live, persists across restart, other macros unaffected) requires running the app on a device; no frontend test framework exists in this project (confirmed no test script in package.json). Automated gates only prove the call site compiles and is wired, not the runtime/persistence behavior."
  - id: D2
    description: "Card-edit target-app <select> is controlled and pre-selects the macro's current target (or Global) on entering edit mode"
    requirement: EXEC-02
    verification:
      - kind: automated_ui
        ref: "tsc --noEmit -p tsconfig.json && npm run build"
        status: pass
    human_judgment: true
    rationale: "Visual pre-selection behavior on entering edit mode requires running the app; tsc/build only prove the value= binding compiles under strict mode, not that the dropdown visually reflects the correct option."

duration: 5min
completed: 2026-07-22
status: complete
---

# Phase 09 Plan 8: Macro Delete Control + Controlled Target Select Summary

**Added the missing frontend delete-macro call site (danger-styled ✕ button + confirm guard wired to the already-implemented `remove_macro` IPC command) and made the card-edit target-app `<select>` controlled so it pre-selects the macro's current target.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-07-22T14:25:00Z
- **Completed:** 2026-07-22T14:30:19Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- Added `handleRemoveMacro(id, name)` handler (confirm guard + `invoke("remove_macro", { id })` + console.error on failure, mirroring `handleToggleMacro`'s error handling) and wired a danger-styled ✕ delete button into each macro card header, closing G-09-1b — users can now delete macros from within the app.
- Bound `value={macro.target_app ?? ""}` on the card-edit target-app `<select>`, closing the secondary G-09-1c UI display bug — the dropdown now pre-selects the macro's actual current target (or "🌐 Global (no target)") instead of defaulting to the first option.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add a delete control to each macro card (G-09-1b)** - `948f27a` (feat)
2. **Task 2: Make the card-edit target-app select controlled (G-09-1c secondary)** - `d50f3df` (fix)

**Plan metadata:** pending (docs: complete plan)

## Files Created/Modified
- `src/App.tsx` - Added `handleRemoveMacro` handler + macro card header delete button; added `value=` binding to the card-edit target `<select>`

## Decisions Made
- No manual state refresh added after `remove_macro` — relies on the existing `state-changed` broadcast/listener path (App.tsx:333-334), identical to how `handleToggleMacro` works. Matches the plan's explicit prohibition.
- Did not touch the backend `remove_macro` command, `Intent::RemoveMacro`, or `handleCardSetTargetApp` — both were already correct; this plan was purely the missing frontend call site plus one missing `value=` binding.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- G-09-1b (missing delete UI) and the G-09-1c secondary UI display bug are both closed. The G-09-1c primary root cause (macOS active-app tracking) was closed separately in plan 09-07.
- Device UAT still required for both tasks' `<human-check>` items: delete-and-persist-across-restart, and target-select pre-selection on edit — these are the `human_judgment: true` coverage items above.
- Full macro delete/edit UI redesign remains scoped to Phase 10 (UI-03) — this plan intentionally shipped only the minimal working affordance.

---
*Phase: 09-parallel-macro-execution*
*Completed: 2026-07-22*

## Self-Check: PASSED

- FOUND: src/App.tsx
- FOUND: 948f27a (Task 1 commit)
- FOUND: d50f3df (Task 2 commit)
