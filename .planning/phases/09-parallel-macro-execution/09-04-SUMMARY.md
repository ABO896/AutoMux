---
phase: 09-parallel-macro-execution
plan: 04
subsystem: ui
tags: [solidjs, tsx, running-state, derived-ui, gap-closure]

# Dependency graph
requires:
  - phase: 09-parallel-macro-execution (plan 02)
    provides: computeRunningState helper and the per-macro-card RunningState indicator (dot + waiting/combined labels)
provides:
  - Hold-mode macros now display the "held" (static accent) indicator instead of the pulsing "firing" indicator
  - Single reactive `runningState` thunk per macro card, reused at all 3 former inline call sites
affects: [09-VERIFICATION.md (CR-01 blocker), pending macOS/Windows device tests T9.1-T9.3 / 6.1-6.3]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Frontend-only derivation mirrors backend runtime conversion (scheduler force-converts Hold-mode steps to SustainedHold) without persisting that conversion — display stays honest while backend data model stays untouched (D-04)."
    - "Compute-once-per-card reactive thunk pattern: `const x = () => fn(...)` declared once inside a <For> callback, invoked as `x()` at every JSX call site, avoiding both redundant recomputation and reactivity loss (WR-01)."

key-files:
  created: []
  modified:
    - src/App.tsx

key-decisions:
  - "Chose Approach A (branch computeRunningState on trigger_mode) over Approach B (persist the Hold->SustainedHold conversion in Intent::AddMacro) — Approach B would drop interval_ms data and change backend behavior, contradicting D-04."

patterns-established:
  - "Reactive thunk compute-once-per-list-item: declare `const derived = () => expensiveFn(item, signal())` inside a <For> callback body, call `derived()` at each JSX site — prevents drift between multiple derivations of the same value within one render."

requirements-completed: [EXEC-01, EXEC-02]

coverage:
  - id: D1
    description: "computeRunningState returns 'held' for a Hold-mode macro (enabled, engine active, not emergency-stopped, target matches/Global) regardless of persisted step shape, closing CR-01"
    requirement: "EXEC-01"
    verification:
      - kind: unit
        ref: "source assertion — src/App.tsx:90-115 computeRunningState Hold branch placed after target-match gate, before empty-sequence fallback; npx tsc --noEmit -p tsconfig.json"
        status: pass
    human_judgment: true
    rationale: "No frontend test framework exists (per CLAUDE.md); the plan's own verification spec requires a human visual check (create a Hold-mode macro via UI, confirm static accent dot vs pulsing green) plus the still-pending macOS/Windows device tests in 09-VERIFICATION.md §5/§6 that this plan does not itself execute."
  - id: D2
    description: "Hold macro with non-matching target_app still returns 'waiting', not 'held' (target-mismatch gate ordered before the new Hold branch)"
    requirement: "EXEC-01"
    verification:
      - kind: unit
        ref: "source assertion — branch order confirmed: (1) !enabled, (2) !engine_active||emergency_stop, (3) !matchesTarget->waiting, (4) trigger_mode===Hold->held, (5) empty-sequence->firing, (6) hasHold&&hasInterval->combined"
        status: pass
    human_judgment: false
  - id: D3
    description: "Pulse-mode macros unaffected: single InterleavedInterval step still shows 'firing', both-step-kind Pulse macros still show 'combined'"
    verification:
      - kind: unit
        ref: "source assertion — Pulse branch (hasHold/hasInterval tail logic) left fully intact after the new Hold branch"
        status: pass
    human_judgment: false
  - id: D4
    description: "Each macro card computes computeRunningState once as a reactive thunk (runningState) and reuses it at the dot-class switch, 'waiting' Show, and 'combined' Show — resolving WR-01 drift risk"
    verification:
      - kind: unit
        ref: "grep counts on src/App.tsx: 1x thunk definition, 1x total computeRunningState(macro,...) call, >=3x runningState() usages; npx tsc --noEmit && npm run build both exit 0"
        status: pass
    human_judgment: false

# Metrics
duration: 6min
completed: 2026-07-20
status: complete
---

# Phase 09 Plan 04: CR-01 Held-Indicator Gap Closure Summary

**Hold-mode macros now show the static "held" dot instead of the pulsing "firing" dot, by branching `computeRunningState` on `trigger_mode === "Hold"`, and each macro card computes that derivation once via a reactive thunk instead of 3 separate inline calls.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-20T18:48:18Z
- **Completed:** 2026-07-20T18:50:25Z
- **Tasks:** 2 completed
- **Files modified:** 1 (`src/App.tsx`)

## Accomplishments
- Closed CR-01 (BLOCKER in 09-VERIFICATION.md): `computeRunningState` now returns `"held"` for any Hold-mode macro that passes the enabled/engine/emergency-stop/target gates, mirroring the scheduler's runtime Hold->SustainedHold force-conversion (`scheduler/mod.rs:287-294`) and the empty-sequence Hold fallback (`scheduler/mod.rs:279-281`) — without touching backend persistence.
- Folded in WR-01 (non-blocking warning): the macro-card `<For>` callback now computes `computeRunningState` once as a reactive thunk (`runningState`) and reuses it at the dot-class switch and both `<Show>` labels, eliminating the 3x-duplicated-call drift risk.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add trigger_mode === "Hold" branch to computeRunningState (CR-01 fix)** - `a5ac4e8` (fix)
2. **Task 2: Compute running-state once per macro card and reuse it (WR-01 fold-in)** - `2676795` (refactor)

**Plan metadata:** pending (this commit)

## Files Created/Modified
- `src/App.tsx` - `computeRunningState` gained a Hold-mode branch (ordered after the target-match "waiting" gate, before the empty-sequence "firing" fallback); the macro-card `<For>` callback converted from expression-body to block-body to declare and reuse a single `runningState` thunk.

## Decisions Made
- Confirmed the plan's chosen fix (Approach A: branch on `trigger_mode`) over the rejected Approach B (persisting the Hold->SustainedHold conversion in `Intent::AddMacro`), since Approach B would lose `interval_ms` data and change backend behavior — no new decision needed, plan's own analysis was followed as specified.

## Deviations from Plan

None - plan executed exactly as written. Both tasks' acceptance criteria were verified byte-for-byte:
- `grep -c 'trigger_mode === "Hold"' src/App.tsx` = 1 (the new branch only)
- Branch order confirmed via direct read: disabled gates -> waiting gate -> Hold->held -> empty-sequence->firing -> combined/held/firing tail
- `grep -c 'const runningState = () => computeRunningState(macro, state()' src/App.tsx` = 1
- `grep -c 'computeRunningState(macro, state()' src/App.tsx` = 1 (only the thunk definition remains)
- `grep -c 'runningState()' src/App.tsx` = 3 (dot switch, waiting Show, combined Show)
- `git diff --name-only` for both commits shows only `src/App.tsx` — no backend files touched, confirming the display-only approach
- `npx tsc --noEmit -p tsconfig.json` exits 0 after each task
- `npm run build` exits 0 (production bundle builds cleanly; the single CSS optimizer warning about `shadow-[0_0_8px_var(...)]` is pre-existing/unrelated to this plan's changes)

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- CR-01 blocker in 09-VERIFICATION.md is closed at the source-code level; the still-pending macOS device tests (T9.1-T9.3) and Windows device tests (6.1-6.3) in 09-VERIFICATION.md §5/§6 remain outstanding human verification steps — this plan makes the on-screen indicator honest for those tests to observe, but does not itself execute them.
- EXEC-01 and EXEC-02 remain flagged `unresolved` per the plan's own frontmatter (`flagged_assumptions`) — this gap-closure plan repairs UI-visibility only; the requirements themselves close only after the pending device tests pass.
- No new stubs, no new threat surface beyond the already-registered T-09-05 (display-integrity drift risk, mitigated by mirroring the scheduler's Hold-conversion logic exactly).

---
*Phase: 09-parallel-macro-execution*
*Completed: 2026-07-20*

## Self-Check: PASSED

- FOUND: .planning/phases/09-parallel-macro-execution/09-04-SUMMARY.md
- FOUND: src/App.tsx
- FOUND: a5ac4e8 (Task 1 commit)
- FOUND: 2676795 (Task 2 commit)
