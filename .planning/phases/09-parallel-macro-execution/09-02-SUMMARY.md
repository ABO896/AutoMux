---
phase: 09-parallel-macro-execution
plan: 02
subsystem: ui
tags: [solidjs, tailwind, tsx, frontend-derivation]

# Dependency graph
requires:
  - phase: 08-hotkey-reliability-conflict-safety
    provides: macro-card layout conventions (↗ Global inline-subtitle pattern, modifier chip technique) that this plan's "Waiting for…" subtitle and "combined" label reuse
provides:
  - Per-macro running-state visibility (firing / held / combined / waiting / disabled) on the macro card, derived purely on the frontend from existing AppState fields
  - computeRunningState pure helper + RunningState union type, mirroring the 3 injection gates in StateActor::handle_action plus a SustainedHold/InterleavedInterval distinction
affects: [09-03-manual-verification]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure top-level derivation function called inline in JSX (no createSignal/createMemo) — mirrors formatInputEvent/formatStep shape, now extended to gate-mirroring derivations of backend state"

key-files:
  created: []
  modified:
    - src/App.tsx

key-decisions:
  - "Used bg-warning/--color-warning-glow for the 'combined' (Hold + Interval) indicator instead of the PATTERNS.md-illustrated bg-info, because no --color-info token exists in src/App.css (@theme block only defines background/surface/border/text/accent/success/danger/warning). Per the plan's explicit fallback instruction ('use the base color token without the glow shadow rather than inventing a token'), warning was the closest available token distinct from accent (held) and success (firing/waiting)."
  - "Combined Task 1 (computeRunningState helper) and Task 2 (card rendering) into a single commit, per the plan's own acceptance-criteria note: tsconfig's noUnusedLocals strict mode would fail Task 1's tsc verification in isolation since the helper is unused until Task 2 wires it in (same reasoning as Phase 8 Plan 04)."

requirements-completed: [EXEC-01, EXEC-02]

coverage:
  - id: D1
    description: "computeRunningState pure helper (+ RunningState union type) mirrors the 3 gates in StateActor::handle_action (engine active + not emergency-stopped, macro enabled, target app match) plus a 4th SustainedHold/InterleavedInterval distinction, reading only existing AppState/MacroConfig fields"
    requirement: "EXEC-01"
    verification:
      - kind: other
        ref: "npx tsc --noEmit -p tsconfig.json (exit 0, strict mode incl. noUnusedLocals)"
        status: pass
      - kind: other
        ref: "grep -c 'function computeRunningState' src/App.tsx == 1; grep -q 'type RunningState' src/App.tsx"
        status: pass
    human_judgment: false
  - id: D2
    description: "Macro card dot switches on computeRunningState (pulsing green=firing, solid green=waiting, accent=held, pulsing warning=combined, dim=disabled), with a 'Waiting for {target_app}' subtitle and an 'Active (Hold + Click)' combined label, per-card only (no global firing/waiting summary added near the engine toggle)"
    requirement: "EXEC-02"
    verification:
      - kind: other
        ref: "npm run build (exit 0, production bundle builds)"
        status: pass
      - kind: other
        ref: "grep checks: computeRunningState(macro, state() usage count >=2, animate-pulse present, 'Waiting for' present inside Show, no computeRunningState reference near App.tsx:740/:799 engine-toggle region"
        status: pass
    human_judgment: false
  - id: D3
    description: "On-device visual confirmation that the firing/waiting/held/combined/disabled states render and transition correctly as engine state, macro enabled state, active app, and macro sequence change"
    verification: []
    human_judgment: true
    rationale: "Visual/interactive states require a human to observe real-time transitions on a running app across macOS and Windows; this is explicitly deferred to plan 09-03's manual device tests per this plan's own <verification> section."

duration: 8min
completed: 2026-07-20
status: complete
---

# Phase 9 Plan 02: Per-Macro Running-State UI Summary

**computeRunningState pure derivation helper + macro-card dot/subtitle rendering that mirrors the backend's 3 injection gates, making parallel firing visible per macro (firing/waiting/held/combined/disabled) with no new backend field or signal.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-20T17:50:00Z
- **Completed:** 2026-07-20T17:58:00Z
- **Tasks:** 2 (combined into 1 commit per strict-mode dependency)
- **Files modified:** 1 (src/App.tsx)

## Accomplishments
- `computeRunningState(macro, state)` — a pure, signal-free top-level helper mirroring the exact 3 gates in `StateActor::handle_action` (`src-tauri/src/state/mod.rs:373-394`): engine active + not emergency-stopped, macro enabled, target app matches (or Global) — plus a 4th SustainedHold/InterleavedInterval check for held vs combined vs firing
- Macro-card status dot no longer branches solely on `macro.enabled`; it now switches on `computeRunningState` to show a pulsing green dot (firing), solid green dot + "Waiting for {target_app}" subtitle (enabled but target app mismatch), accent dot (held-only), pulsing warning dot + "Active (Hold + Click)" label (combined hold+interval), or dim dot (disabled)
- Per-card-only surface: no global "X firing / Y waiting" summary was added anywhere, including the engine-toggle region (`App.tsx:740`, `:799`), consistent with D-03

## Task Commits

Both tasks were committed together (see Decisions Made for why):

1. **Task 1: Add computeRunningState pure helper + RunningState type** — combined into `4f751c0`
2. **Task 2: Render running-state dot, "Waiting for…" subtitle, and combined label** — combined into `4f751c0` (feat)

## Files Created/Modified
- `src/App.tsx` - New `RunningState` union type + `computeRunningState` pure helper (placed after `formatStep`, before the `computeModifiers` doc comment block); macro-card dot switch statement replacing the `macro.enabled` ternary; "Waiting for {target_app}" and "Active (Hold + Click)" conditional `<Show>` spans in the card title row

## Decisions Made
- Used `bg-warning`/`--color-warning-glow` for the "combined" indicator instead of the PATTERNS.md-illustrated `bg-info`, since `src/App.css`'s `@theme` block has no `--color-info` token at all (only background/surface/border/text/accent/success/danger/warning). Followed the plan's explicit fallback instruction to use an existing token rather than invent one.
- Combined Task 1 and Task 2 into a single commit — Task 1's `computeRunningState` helper is unused until Task 2 wires it into JSX, which trips `noUnusedLocals` under `tsc --noEmit` if committed in isolation. The plan itself anticipated and permitted this (citing Phase 8 Plan 04's identical situation).

## Deviations from Plan

None - plan executed exactly as written (color-token substitution and commit combination were both explicitly anticipated and permitted by the plan text itself, not unplanned deviations).

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Plan 09-03 (manual device verification / 09-VERIFICATION.md) can now reference the firing/waiting/held/combined/disabled indicators as the on-screen signal for confirming "both macros fire concurrently" and "enabled-but-target-mismatched macro shows waiting, not firing"
- No blockers

---
*Phase: 09-parallel-macro-execution*
*Completed: 2026-07-20*

## Self-Check: PASSED

All created/modified files exist on disk (src/App.tsx, this SUMMARY.md). Commit 4f751c0 verified present in git log.
