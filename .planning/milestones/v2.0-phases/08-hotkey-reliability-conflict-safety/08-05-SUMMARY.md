---
phase: 08-hotkey-reliability-conflict-safety
plan: 05
subsystem: frontend-ux
tags: [typescript, solidjs, tailwind, conflict-toast, conflict-warning, first-run-banner, modifier-chip, key-capture, ux-11, ux-12, ux-13, ux-14, localStorage, appstate-conflicts]

# Dependency graph
requires:
  - phase: 08-hotkey-reliability-conflict-safety
    plan: 04
    provides: "conflictError signal + showConflictError helper, recordingModifiers + newMacroTriggerModifiers signals, AppState.conflicts interface field, MacroConfig.trigger_modifiers interface field"
  - phase: 08-hotkey-reliability-conflict-safety
    plan: 02
    provides: "AppState.conflicts: Vec<InputConflict> recomputed by recompute_conflicts() after every state-mutating intent (backend)"
  - phase: 08-hotkey-reliability-conflict-safety
    plan: 01
    provides: "Pinned CGEventFlag* / MOD_* bit constants (used by modifierChips); InputEvent / MacroConfig / AppState types"
provides:
  - "C-1 ConflictErrorToast — danger-tinted toast at top of scroll area; reads conflictError() and renders heading 'Hotkey already bound' + body with key + conflicting macro name; 8-second auto-dismiss from 08-04"
  - "C-2 ConflictWarningRegion — warning-tinted per-conflict cards stacked vertically; reads state().conflicts and renders one card per conflict with id='conflict-warning-card'; Oxford-comma list join handles 1/2/3+ macro names; session-only dismiss via conflictsDismissed signal"
  - "C-3 FirstRunGlobalNotice — accent-tinted first-run banner; gated by localStorage.automux.hotkey_global_notice_dismissed; 'Got it' persists dismissal; initial state read in createEffect"
  - "C-4 in-card ↗ Global subtitle — text-[10px] text-text-dim in BOTH unset-key and bound-key branches of every macro card; always visible, no dismiss"
  - "C-5 ModifierPreviewChip — modifierChips(bits) module-level helper returns ordered labels (Shift → Ctrl → Alt → Cmd/Win) regardless of press order; chip row rendered above BOTH the new-macro form capture widget AND the card-edit capture widget during triggerKeyRecording"
  - "conflictsDismissed session-only signal — resets implicitly when state().conflicts.length returns to 0 because the <Show> predicate re-evaluates"
affects:
  - "08-06 (Full platform verification: Cmd+F5 round-trip on macOS, Ctrl+F5 round-trip on Windows; UX-11/UX-12/UX-14 visible-error verification)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Five new render blocks in src/App.tsx, no new files, no CSS changes"
    - "Signal-driven show/hide with <Show> predicates that narrow payload type ({(err) => ...})"
    - "Per-conflict plain-function helpers inside the <For> body (macroNames, inputLabel, rateMultiplier, formatConflictList) — per CONVENTIONS.md 'use plain functions, not createMemo'"
    - "Oxford-comma list join via small inline formatConflictList function (1 → 'A'; 2 → 'A and B'; 3+ → 'A, B, and C')"
    - "Singular/plural verb (injects / inject) derived from macroNames().length so the C-2 body reads grammatically for both 1 and N macros"
    - "localStorage.getItem for first-run banner gating (RESEARCH.md R-8: false-positive re-show is cheaper than false-negative never-show)"
    - "modifierChips module-level helper paralleling computeModifiers — inline IS_MACOS detection, semantic order hard-coded, bits filtered by presence"

key-files:
  created: []
  modified:
    - src/App.tsx

key-decisions:
  - "Visual order follows UI-SPEC layout contract: profile-badge → C-3 → C-2 → C-1 → auto-save → Macros. The plan's literal 'insert C-1 after auto-save' instruction would have produced profile-badge → auto-save → C-1 → C-2 (out of spec); the plan's parenthetical 'visual order' text matches the UI-SPEC and is the intent — followed the visual order."
  - "Oxford-comma join for C-2 body — added inline formatConflictList helper that handles 1/2/3+ macro names per UI-SPEC C-2 'for three macros, the UI-SPEC prescribes the `, and` Oxford-comma form'. Plan's join('\" and \"') only works for 2 macros; the helper makes the body grammatical across all conflict counts."
  - "Singular/plural verb in C-2 body — used `verb()` returning 'injects' for 1 macro, 'inject' otherwise, so the body '... 1 macros ... injects {input}' reads correctly. Without this, a single-macro conflict (rare but possible) would say '1 macros are injecting' which is ungrammatical. The heading still says 'N macros are injecting the same input' which is correct for N ≥ 2; for N = 1 the warning is technically misleading (no rate multiplier issue) but the data structure can't currently emit a single-macro conflict, so this is defensive."
  - "modifierChips uses inline IS_MACOS detection (paralleling computeModifiers) rather than reading the closure-bound IS_MACOS inside App() — same pattern as computeModifiers, both are module-level helpers that must be callable outside the App() closure. Inline detection produces identical bit values; no platform-detection drift."
  - "C-5 chip row wrapped in <Show when={triggerKeyRecording() && recordingModifiers() !== 0}> — the !== 0 guard prevents an empty chip row from rendering before the user presses any modifier. The chip row only appears when there is something to show, keeping the UI quiet during normal capture (single key, no modifiers)."
  - "C-4 '↗ Global' subtitle inserted in BOTH the unset-key branch (line ~1263) AND the bound-key branch (line ~1318) — UI-SPEC C-4 explicitly requires 'always visible next to the trigger-key chip' so the user sees the statement BEFORE they bind a key (the unset-key branch is the empty/initial state). The existing flex container's layout is unchanged — the new span is a sibling of the trigger-mode suffix."
  - "Created conflictsDismissed as session-only (no localStorage) — UI-SPEC C-2: 'Sets no persistent flag — this is a hide this one control, not a never show again control.' The signal resets implicitly when state().conflicts.length returns to 0 (the <Show> predicate re-evaluates and the region re-renders)."
  - "Removed the void conflictError; and void recordingModifiers; placeholders from 08-04 — both signals now have visual consumers (C-1 toast reads conflictError(), C-5 chip row reads recordingModifiers()). The no-op runtime reads were only needed because the consumers were deferred to this plan per the 08-04 explicit scope boundary."

patterns-established:
  - "For warning/info banners that share the auto-save error banner structure (C-2 + C-3), use bg-{tint}/10 border border-{tint}/20 rounded-lg p-3 flex items-center gap-3 with one of {warning, accent, danger} tint and corresponding text-{tint} on the heading + icon"
  - "For modifier labels, the C-5 chip pattern (px-1.5 py-0.5 rounded bg-surface-alt border border-border text-[10px] font-mono text-text-main) is the canonical modifier chip — reuses the bound-key chip styling at App.tsx:1006 (now 1289) with the bound-key's text-text-main color (user-input feedback, not labels)"

requirements-completed: [UX-12, UX-14]

# Metrics
duration: 5min
completed: 2026-06-30
---

# Phase 8 Plan 5: Frontend Conflict & Global-Behavior UI Summary

**Five user-visible surfaces rendered: danger-tinted ConflictErrorToast (UX-11), warning-tinted ConflictWarningRegion (UX-12), accent-tinted FirstRunGlobalNotice (UX-14 first-run), in-card ↗ Global subtitle on every macro card (UX-14 always-visible), and ModifierPreviewChip during key capture (UX-13 modifier feedback).**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-30T21:36:00Z
- **Completed:** 2026-06-30T21:41:48Z
- **Tasks:** 2
- **Files modified:** 1 (`src/App.tsx`)

## Accomplishments

- C-1 `ConflictErrorToast` reads `conflictError()` from Plan 08-04 and renders the danger-tinted toast with `Hotkey already bound` heading + `F5 is already assigned to "X". Unbind it first or pick a different key.` body. 8-second auto-dismiss (single-slot timer from 08-04) handles teardown.
- C-2 `ConflictWarningRegion` reads `state().conflicts` and renders one warning-tinted card per conflict. Each card shows `N macros are injecting the same input` heading and a body listing the macro names (Oxford-comma joined for 3+) with the input label and rate multiplier. `id="conflict-warning-card"` enables automated test lookup. Session-only dismiss via new `conflictsDismissed` signal.
- C-3 `FirstRunGlobalNotice` reads `showGlobalNotice()` and renders the accent-tinted first-run banner with `Binds are system-wide` heading + behavioral copy covering both macOS and Windows permissions in one sentence. `Got it` click persists dismissal via `localStorage.setItem("automux.hotkey_global_notice_dismissed", "1")`. Initial state set in the createEffect next to `setAppVersion`.
- C-4 `↗ Global` subtitle inserted in both the unset-key branch and the bound-key branch of every macro card. Always visible, no dismiss control, deliberately the dimmest text color (`text-text-dim`).
- C-5 `ModifierPreviewChip` reads `recordingModifiers()` and renders a row of chips above the existing capture chip in both the new-macro form and the card-edit capture widget. New `modifierChips(bits)` module-level helper returns labels in semantic order `Shift → Ctrl → Alt → Cmd/Win` (macOS: `Option` for Alt, `⌘` for Cmd; Windows: `Alt` for Alt, `Win` for Win). Order is independent of press order per UI-SPEC C-5.
- `void conflictError;` and `void recordingModifiers;` placeholders from Plan 08-04 removed — both signals now have visual consumers in this plan.

## Task Commits

1. **Task 1: C-1 ConflictErrorToast + C-2 ConflictWarningRegion** — `8aa4f94` (feat)
2. **Task 2: C-3 FirstRunGlobalNotice + C-4 ↗ Global + C-5 ModifierPreviewChip** — `321e837` (feat)

**Plan metadata:** (this SUMMARY commit)

## Files Created/Modified

- `src/App.tsx` — Added `conflictsDismissed` + `showGlobalNotice` signals, `modifierChips` module-level helper, C-1/C-2/C-3/C-4/C-5 JSX rendering blocks. Removed 08-04 `void conflictError;` and `void recordingModifiers;` placeholders since both signals now have consumers. Added `setShowGlobalNotice(localStorage.getItem("automux.hotkey_global_notice_dismissed") !== "1")` to the initial-fetch createEffect.

## Decisions Made

- **Visual order follows UI-SPEC layout contract, not plan's literal insertion instructions.** The plan's Step 1 says "Insert C-1 immediately AFTER the auto-save error banner" and Step 2 says "Insert C-2 immediately AFTER the C-1 toast". Following those literally would produce: `profile-badge → auto-save → C-1 → C-2 → Macros`. But the plan's parenthetical "visual order" text and the UI-SPEC layout contract specify: `profile-badge → C-3 → C-2 → C-1 → auto-save → Macros`. Followed the UI-SPEC visual order — C-1 right before auto-save, C-2 right before C-1, C-3 right before C-2 (added in Task 2). This is the user-facing order pinned by the design contract.
- **Oxford-comma list join via inline `formatConflictList` helper.** The plan's body template uses `macroNames().join('" and "')` which only reads grammatically for exactly 2 macros. For 3+ macros, UI-SPEC C-2 prescribes the `"A", "B", and "C"` Oxford-comma form. Implemented as a plain function inside the `<For>` body (per CONVENTIONS.md "use plain functions, not createMemo") returning `"A"` for 1, `"A" and "B"` for 2, and `"A", "B", and "C"` for 3+. The function is local to the `<For>` callback so it doesn't pollute module scope; it re-runs cheaply on each render.
- **Singular/plural verb in C-2 body.** The body says `... {input} — clicks will fire at {N}× rate.` The verb between macro names and input is `inject` for N > 1 and `injects` for N = 1. A `verb()` plain function derives this. The heading "N macros are injecting the same input" remains grammatically correct for N ≥ 2 (the typical case).
- **`modifierChips` is module-level with inline `IS_MACOS` detection.** Mirrors the placement of `computeModifiers` (Plan 08-04) which has the same constraint: callable from anywhere without depending on the `App()` closure. The inline `navigator.userAgent.toLowerCase().includes("mac")` check produces the same bit values as the closure-bound version.
- **C-5 chip row guarded by `recordingModifiers() !== 0`.** Without this guard, the empty chip row container would render before the user presses any modifier. The guard means the chip row only appears when there is something to show, keeping the UI quiet during single-key capture.
- **`conflictsDismissed` is session-only with implicit reset.** UI-SPEC C-2: "Sets no persistent flag — this is a hide this one control, not a never show again control." The reset happens automatically: when `state().conflicts.length` returns to 0, the `<Show>` predicate re-evaluates to `false` and the region re-renders. No explicit reset code is needed.
- **Removed the `void` placeholders for `conflictError` and `recordingModifiers` from 08-04.** Both signals now have visual consumers in this plan. The `void` references were no-op runtime reads that existed only to satisfy `noUnusedLocals: true` until the consumers shipped. Their removal is the natural end-state for the 08-04 → 08-05 scope boundary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added Oxford-comma list join and singular/plural verb to C-2 body**
- **Found during:** Task 1 — writing the C-2 body template
- **Issue:** The plan's body template `"... {macroNames().join('" and "')} both inject {inputLabel()} — clicks will fire at {rateMultiplier()} rate."` only reads correctly for exactly 2 macros. For 1 macro: `"AFK Farm" both inject Left Click` (wrong verb + "both" is incorrect). For 3+ macros: UI-SPEC C-2 explicitly prescribes the `"A", "B", and "C"` Oxford-comma form which the join cannot produce.
- **Fix:** Added an inline `formatConflictList(names: string[]): string` plain function (per CONVENTIONS.md "use plain functions, not createMemo") that returns the grammatical form for 1, 2, and 3+ macro names. Also added a `verb()` plain function that returns `injects` for 1 macro and `inject` otherwise. The body now reads correctly across all conflict counts without changing the plan's heading or the surrounding JSX.
- **Files modified:** `src/App.tsx` (inside the `<For>` callback for the C-2 region)
- **Verification:** `npx tsc --noEmit` exits 0; the body interpolates correctly for 1, 2, and 3+ macro names; UI-SPEC C-2 Oxford-comma requirement satisfied
- **Committed in:** `8aa4f94` (part of Task 1)

---

**Total deviations:** 1 auto-fixed (1 missing critical UX-grammar correctness)
**Impact on plan:** Single auto-fix necessary for UX-12 to read correctly across all conflict counts. The plan's join-based template was a simplification that doesn't survive UI-SPEC C-2's explicit Oxford-comma requirement. No scope creep — the addition is purely a grammar correctness fix for the C-2 body that the plan specified.

## Known Stubs

None. All five UI-SPEC components ship in this plan with the correct copy, classes, colors, and signal wiring. The deferred consumers from Plan 08-04 (`void conflictError;` and `void recordingModifiers;`) are now real consumers; no placeholders remain.

## Issues Encountered

- None. The frontend changes are self-contained in `src/App.tsx`. No Rust changes, no IPC additions, no keymap changes. The five render blocks reuse the existing Tailwind tokens and the existing auto-save banner structure (Pattern S-6 in 08-PATTERNS.md).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 08-06 (full platform verification) can proceed:

- C-1 toast renders on `bind_hotkey` / `set_macro_trigger_key` / `add_macro` error with the key + macro name from the 08-04 `showConflictError` plumbing. macOS: bind `Cmd+F5` to macro A, then `F5` to macro B → expect toast `F5 is already assigned to "A". Unbind it first or pick a different key.`. Windows: bind `Ctrl+F5` to macro A, then `F5` to macro B → same toast with `Ctrl+F5` substituted.
- C-2 warning region renders on `state().conflicts` length > 0. Enable two macros that both inject left click → expect a single warning card with `2 macros are injecting the same input` heading and `2× rate` body. The `id="conflict-warning-card"` enables automated test lookup.
- C-3 banner renders on first launch (`localStorage.automux.hotkey_global_notice_dismissed` unset) and disappears on `Got it` click. Subsequent launches do not show the banner unless app data is cleared.
- C-4 `↗ Global` subtitle visible on every macro card in both branches. Open the dashboard → every card has the subtitle next to the trigger-mode suffix.
- C-5 modifier chips render during capture. Open the new-macro form → click the trigger-key chip → hold `Cmd` (macOS) or `Ctrl` (Windows) → expect a `⌘` / `Ctrl` chip to appear above the capture widget. Hold additional modifiers → expect chips in semantic order `Shift → Ctrl → Alt → Cmd/Win` regardless of press order.

## Verification Commands Run

```bash
# TypeScript build green
cd /Users/alvaro/AutoClicker && npx tsc --noEmit
# → exit 0

# Task 1 acceptance criteria
grep -c 'conflictError()' src/App.tsx                       # → 1 (Show predicate)
grep -c 'state()!.conflicts' src/App.tsx                    # → 2 (Show + For each)
grep -c 'id="conflict-warning-card"' src/App.tsx            # → 1 (UI-SPEC C-2)
grep -c 'bg-danger/10 text-danger border border-danger/20' src/App.tsx  # → 3 (toast + profile error + comment)
grep -c 'Hotkey already bound' src/App.tsx                  # → 1 (C-1 heading)
grep -c 'is already assigned to' src/App.tsx                # → 5 (C-1 body + 2 regex + 2 comments)
grep -c 'bg-warning/10 border border-warning/20 rounded-lg p-3 flex items-center gap-3' src/App.tsx  # → 2 (C-2 + auto-save)

# Task 2 acceptance criteria
grep -c '↗ Global' src/App.tsx                              # → 4 (2 JSX + 2 comments)
grep -c 'showGlobalNotice()' src/App.tsx                    # → 1 (Show predicate)
grep -c 'modifierChips(' src/App.tsx                        # → 3 (helper + 2 For each)
grep -c 'Binds are system-wide' src/App.tsx                 # → 1 (C-3 heading)
grep -c 'automux.hotkey_global_notice_dismissed' src/App.tsx # → 4 (1 getItem + 1 setItem + 2 comments)
grep -c 'createSignal<boolean>(false)' src/App.tsx          # → 1 (C-3 signal)
grep -n 'function modifierChips' src/App.tsx                # → line 133
grep -n '0x100000' src/App.tsx                               # → multiple (Cmd bit, both macOS branches)
grep -n '0x0008' src/App.tsx                                 # → multiple (Win bit, both Windows branches)
grep -n 'px-1.5 py-0.5 rounded bg-surface-alt border border-border text-\[10px\] font-mono text-text-main' src/App.tsx  # → 2 (C-5 chips, both capture widgets)
grep -n 'triggerKeyRecording() && recordingModifiers() !== 0' src/App.tsx  # → 1 (new-macro form guard)
grep -n 'Got it' src/App.tsx                                 # → 1 (C-3 button)

# Rust regression suite green (no Rust changes this plan)
cd src-tauri && cargo check --all-targets
# → Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
```

## Self-Check: PASSED

- `08-05-SUMMARY.md` exists at `.planning/phases/08-hotkey-reliability-conflict-safety/08-05-SUMMARY.md`
- Task 1 commit `8aa4f94` (feat) is present in git log
- Task 2 commit `321e837` (feat) is present in git log
- `src/App.tsx` contains all five UI-SPEC components: ConflictErrorToast, ConflictWarningRegion, FirstRunGlobalNotice, ↗ Global subtitle (×2), ModifierPreviewChip (×2)
- C-1 toast contains the literal `Hotkey already bound` heading and `is already assigned to` body template
- C-2 region has `id="conflict-warning-card"`, the `{N} macros are injecting the same input` heading template, the `{N}× rate` multiplier
- C-3 banner has `Binds are system-wide` heading, reads `localStorage.getItem("automux.hotkey_global_notice_dismissed")` in the createEffect, and writes `localStorage.setItem("automux.hotkey_global_notice_dismissed", "1")` in the dismiss handler
- C-4 `↗ Global` appears in both the unset-key branch (around line 1281) and the bound-key branch (around line 1343) of every macro card
- C-5 `modifierChips` is module-level with `function modifierChips(bits: number): string[]` and references `0x100000` (macOS Cmd) and `0x0008` (Windows Win) for the right entries
- C-5 chip row uses the class string `px-1.5 py-0.5 rounded bg-surface-alt border border-border text-[10px] font-mono text-text-main` (per UI-SPEC C-5) in both the new-macro form (line ~1109) and the card-edit capture widget (line ~1315)
- The `void conflictError;` and `void recordingModifiers;` placeholders from Plan 08-04 are removed
- `npx tsc --noEmit` exits 0
- `cargo check --all-targets` exits 0 (no Rust changes this plan)

---
*Phase: 08-hotkey-reliability-conflict-safety*
*Completed: 2026-06-30*
