---
phase: 10-ui-redesign-macro-management
plan: 05
subsystem: ui
tags: [solidjs, tailwind-v4, macro-card, inline-edit, delete-confirmation]

# Dependency graph
requires:
  - phase: 10-ui-redesign-macro-management (plan 04)
    provides: "MacroForm.tsx (shared create/edit field set) and KeyCaptureField.tsx (reusable key-capture widget), ready to mount a second time inside the card's inline edit"
  - phase: 10-ui-redesign-macro-management (plan 01)
    provides: "update_macro IPC + Intent::UpdateMacro (full field set, atomic, conflict-checked via resolve_trigger_key_update), editingCardId centralized signal"
provides:
  - "src/components/MacroCard.tsx — normal / inline-edit / inline-delete-confirm display states for one macro, extracted from App.tsx's inline <For> block"
  - "App.tsx: full-field edit-form signals (editMacroName/Input/Mode/Interval/Target/TriggerKeyCode/TriggerModifiers/ActionKeyCode, editFormCapturingSlot) and handlers (deriveEditFormFields, handleStartEditMacro, handleCancelEditMacro, handleSaveMacro)"
  - "App.tsx: confirmingDeleteId + deleteError signals and handlers (handleDeleteStart/Cancel/Confirm) — window.confirm() fully removed"
affects: ["10-06 (final polish/verification — the last plan in this phase)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Fourth component-file extraction from App.tsx (MacroCard.tsx), completing the decomposition started in 10-03/10-04"
    - "The card's per-field micro-editors (target-app click-to-select, trigger-key click-to-capture, name-only tracer edit) are retired and consolidated into ONE full inline expand-in-place edit surface that mounts MacroForm a second time — matching D-13's literal 'reuses the same fields/components as the macro creation form' contract"
    - "Nested Show blocks in MacroCard: outer Show gates isConfirmingDelete() (C-D1 replaces the entire card body), inner Show gates isEditing() (C-E1 replaces the entire card body) — normal display is the shared fallback"

key-files:
  created:
    - src/components/MacroCard.tsx
  modified:
    - src/App.tsx

key-decisions:
  - "Retired the pre-existing per-field micro-editors (click target-app span to open a select; click trigger-key chip to capture; the 10-04 tracer's name-only inline input) entirely, rather than keeping them alongside the new full edit form. UI-SPEC's Component Inventory only documents two new interactive states beyond normal display (C-E1 full edit, C-D1 delete confirm) — no separate quick-edit component — and keeping both would have meant two different mutual-exclusion mechanisms (editingField sub-states vs. the new full edit) competing for the same editingCardId signal. Consolidating to one edit surface is simpler and matches D-13's 'reuses the SAME form' framing literally."
  - "Removed the old handleCardSetTriggerKey's macOS-only bind_hotkey + set_macro_trigger_key double-invoke (and the now-unused IS_MACOS constant) rather than porting it into the new edit-save path. Intent::UpdateMacro's handler already calls reevaluate_all_macros() after every successful edit, which refreshes the platform hotkey registry unconditionally (per the Phase 9 Plan 10 consolidation) — the atomic Save Changes call covers hotkey registration without a separate immediate-commit call, exactly like the New Macro creation form's trigger-key capture already worked (capture just updates local signals; persistence + registry refresh happens on submit)."
  - "The delete-confirm state (C-D1) replaces the CARD's entire body, not just its header row, despite the UI-SPEC prose saying 'replaces the macro card's header row content in place' — the UI-SPEC's own ASCII mockup box shows only the two confirmation lines inside the card, with no meta row or step chips drawn below. Treating it as a full-body replacement (mirroring how the edit state already replaces the full body) is the reading consistent with that mockup and keeps both non-normal states symmetric."
  - "The Delete/Cancel button pair inside the delete-confirmation heading uses a flex row with shrink-0 literal-text spans flanking a truncate+min-w-0 name span, rather than putting `truncate` directly on inline text — a bare inline span has no bounded width to truncate against; the flex-row-with-min-w-0 pattern is the standard fix for truncating one segment of a longer line."

requirements-completed: [UX-08, UX-09]

coverage:
  - id: D1
    description: "MacroCard.tsx extracted with normal / full inline-edit (C-E1) / inline-delete-confirm (C-D1) display states; card header ✎ button (before ✕) opens edit pre-filled with the macro's current name, Input+Mode, key/button, and timing via deriveEditFormFields(); Save Changes persists the full field set through the atomic update_macro; Cancel discards local edits with no IPC"
    requirement: "UX-09"
    verification:
      - kind: unit
        ref: "cargo test update_macro_applies_all_fields, update_macro_conflict_no_partial_mutation, update_macro_persists_across_round_trip (all pass — backend atomicity/persistence proven in 10-01, exercised end-to-end by this plan's frontend)"
        status: pass
      - kind: other
        ref: "grep -c 'const { ' src/components/MacroCard.tsx (0); grep -c 'createSignal' src/components/MacroCard.tsx (0); grep -c 'MacroForm' src/components/MacroCard.tsx (5); grep -c 'Edit macro' src/components/MacroCard.tsx (1); npx tsc --noEmit (exit 0); cargo build (clean)"
        status: pass
    human_judgment: true
    rationale: "Visual confirmation that editing each field independently (name, Input, Mode, key/button, interval, target app) persists across an app restart, and that opening edit on a second card discards the first card's unsaved edit, requires exercising the running app — no frontend test framework exists in this project (per CONVENTIONS.md)."
  - id: D2
    description: "window.confirm() fully removed from the delete path; ✕ opens the C-D1 inline confirmation (`Delete \"{name}\"?` / `This can't be undone.` / Cancel / Delete) with exact UI-SPEC copy and classes; Delete invokes the existing remove_macro IPC unchanged; Cancel reverts with no IPC"
    requirement: "UX-08"
    verification:
      - kind: other
        ref: "grep -cE 'window\\.confirm' src/App.tsx (0); grep -c 'confirmingDeleteId' src/App.tsx (2); grep -c \"This can't be undone.\" src/components/MacroCard.tsx (1); grep -c 'Delete \\\"' src/components/MacroCard.tsx (1); npx tsc --noEmit (exit 0)"
        status: pass
    human_judgment: true
    rationale: "Confirming the inline confirmation renders in place of a native dialog, that Delete/Cancel behave correctly, and that deleting the last macro shows the empty-state card, requires visual/manual QA in the running app — no frontend test framework exists."
  - id: D3
    description: "Delete-failure backstop: a failed remove_macro shows an inline 'Delete failed' / 'Could not delete this macro. Try again.' banner and leaves the card's confirm state open (delete intent is not silently discarded)"
    requirement: "UX-08"
    verification:
      - kind: other
        ref: "grep -c 'Delete failed' src/App.tsx (2 — see Deviations; the new banner's exact copy is present); npx tsc --noEmit (exit 0)"
        status: pass
    human_judgment: true
    rationale: "Triggering an actual remove_macro failure to observe the banner and confirm the confirm-state stays open requires either a simulated backend failure or manual QA — no automated harness exists for this path."

# Metrics
duration: ~16min
completed: 2026-07-24
status: complete
---

# Phase 10 Plan 05: MacroCard Extraction — Inline Edit + Delete Confirmation Summary

**Extracted MacroCard.tsx with three display states — normal, full inline expand-in-place edit (mounting the shared MacroForm, full UX-09 field set via the atomic update_macro), and inline delete confirmation (C-D1, replacing window.confirm() entirely) — completing macro management for Phase 10**

## Performance

- **Duration:** ~16 min
- **Completed:** 2026-07-24
- **Tasks:** 2/2
- **Files modified:** 2 (`src/App.tsx`, plus 1 new file: `src/components/MacroCard.tsx`)

## Accomplishments

- Extracted `src/components/MacroCard.tsx` — one macro card with normal / edit / confirm-delete states, replacing the inline `<For>` block that previously lived directly in `App.tsx`
- Generalized the 10-01 tracer's name-only inline edit into the full UX-09 field set: clicking ✎ opens the card in place, mounting the shared `MacroForm` (10-04) pre-filled via a new `deriveEditFormFields()` helper (reconstructs Input/Mode/interval/action-key from the macro's persisted `sequence.steps`); `Save Changes` calls the atomic `update_macro` IPC with every field; `Cancel` discards local edits with no IPC
- Retired the pre-existing per-field micro-editors (click-to-edit target-app span, click-to-capture trigger-key chip) — all editing now flows through the one unified full-edit surface, matching D-13's "reuses the same fields/components as creation" contract literally
- Replaced `window.confirm()` (`handleRemoveMacro`, previously `App.tsx:576-583`) with the C-D1 inline in-card confirmation: `confirmingDeleteId` (App-owned, alongside `editingCardId`) plus `handleDeleteStart`/`handleDeleteCancel`/`handleDeleteConfirm`; the ✕ button now opens the confirmation instead of the native dialog
- Added a `deleteError` signal + inline "Delete failed" / "Could not delete this macro. Try again." banner (`profileMessage`-style pattern) for a failed `remove_macro` — the card's confirm state stays open on failure, so the delete intent is never silently discarded
- Mutual exclusion preserved and extended: `editingCardId` and `confirmingDeleteId` are both App-owned signals; starting edit on a card clears any in-progress delete-confirm (and vice versa), and opening edit on a second card while another is mid-edit reassigns the shared edit-form signals, discarding the first card's unsaved edit
- Removed the now-dead `handleCardSetTriggerKey`/`handleCardSetTargetApp` micro-editor handlers and the unused `IS_MACOS` constant — `Intent::UpdateMacro`'s backend handler already calls `reevaluate_all_macros()` on every successful save, so the atomic edit path covers hotkey-registry refresh without a separate immediate-commit call
- `npx tsc --noEmit`, `cargo build --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml --lib` (24/24 tests) all clean/pass after all changes

## Task Commits

Each completed task was committed atomically:

1. **Task 1: Extract MacroCard.tsx with normal + inline-edit states (full UX-09)** - `329d3fe` (feat)
2. **Task 2: Inline delete confirmation (C-D1) replacing the native dialog + failure banner** - `f4de1ac` (feat)

## Files Created/Modified

- `src/components/MacroCard.tsx` - New: `MacroCardProps` (accessor+callback pairs for normal/edit/confirm-delete), `runningStateDotClass` helper (plain function, moved from App.tsx's inline switch), nested `Show` blocks for the three display states
- `src/App.tsx` - `editingCardId`/`confirmingDeleteId`/`deleteError` signals; new edit-form signal set (`editMacroName`/`Input`/`Mode`/`Interval`/`Target`/`TriggerKeyCode`/`TriggerModifiers`/`ActionKeyCode`, `editFormCapturingSlot`); `deriveEditFormFields`, `handleStartEditMacro`, `handleCancelEditMacro`, `handleSaveMacro`, `handleDeleteStart`, `handleDeleteCancel`, `handleDeleteConfirm`; exported `MacroConfig`, `RunningState`, `formatStep` for `MacroCard.tsx` to consume; removed `handleRemoveMacro` (window.confirm), `handleCardSetTriggerKey`, `handleCardSetTargetApp`, the tracer's `handleStartEditMacroName`/`handleCancelEditMacroName`/`handleSaveMacroName`, the `editingField`/`editingMacroName` signals, and the unused `IS_MACOS` constant; the macro-list `<For>` body now instantiates `<MacroCard>`

## Decisions Made

- **Retired the per-field micro-editors instead of keeping them alongside the new full edit:** UI-SPEC's Component Inventory only documents C-E1 (full edit) and C-D1 (delete confirm) as new interactive states beyond normal display — no separate quick-edit component is specified. Keeping the old target-app/trigger-key click editors would have meant two competing mutual-exclusion mechanisms sharing `editingCardId`. Consolidating onto one full edit surface is simpler and matches D-13's literal wording.
- **Dropped the macOS bind_hotkey double-invoke from the edit path:** `Intent::UpdateMacro`'s handler already calls `reevaluate_all_macros()` after every successful mutation (confirmed by reading `state/mod.rs:683`), which refreshes the platform hotkey registry unconditionally per the Phase 9 Plan 10 single-registry consolidation. The edit form's trigger-key capture now behaves exactly like the New Macro creation form's — capture just updates local signals, and the atomic Save Changes call handles both persistence and registry refresh.
- **Delete-confirm (C-D1) replaces the whole card body, not just the header row:** UI-SPEC's prose says "replaces the macro card's header row content," but its own ASCII mockup draws only the two confirmation lines inside the card box, with no meta row or step chips below. Treating it as a full-body replacement (symmetric with how the edit state already replaces the full body) is the reading consistent with the mockup.
- **Truncating the quoted macro name inside `Delete "{name}"?`:** used a flex row with `shrink-0` literal-text spans flanking a `truncate min-w-0` name span, since a bare inline span has no bounded width to truncate against inside running text.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Reworded two doc comments that inadvertently matched a literal `window.confirm` grep gate**
- **Found during:** Task 2
- **Issue:** Two explanatory comments (documenting the old behavior being replaced) contained the literal substring `window.confirm()`, which caused the acceptance criteria's `grep -cE 'window\.confirm' src/App.tsx` gate to return 2 instead of the required 0, even though no actual call remained.
- **Fix:** Reworded both comments to describe "the old native dialog" instead of quoting `window.confirm()` verbatim.
- **Files modified:** `src/App.tsx`
- **Verification:** `grep -cE 'window\.confirm' src/App.tsx` → 0
- **Committed in:** `f4de1ac` (Task 2)

### Noted, Not Fixed

**2. [Documentation-only] `grep -c 'Delete failed' src/App.tsx` returns 2, not the plan's expected 1**
- **Found during:** Task 2
- **Issue:** The plan's acceptance criteria expected exactly one occurrence of the literal text `Delete failed` in `src/App.tsx`. A second, pre-existing, unrelated occurrence already exists at `handleDeleteProfile`'s catch block (`showProfileMsg(\`Delete failed: ${e}\`, "error")`), which predates this plan and covers *profile* deletion, not macro deletion.
- **Why not fixed:** The plan's own copy contract (UI-SPEC's Copywriting section) mandates the exact string `Delete failed` for the new macro-delete banner heading — changing it would violate the design contract. Rewording the pre-existing, unrelated profile-delete message is out of scope for this plan (not in `files_modified`'s intent, and would be an unplanned behavior/copy change to a different feature).
- **Files affected:** `src/App.tsx` (no change made beyond the plan's own edits)
- **Verification:** Manual read of both matching lines confirms they are two distinct, correctly-scoped banners (`src/App.tsx:812` pre-existing profile-delete failure, `src/App.tsx:1162` new macro-delete failure added by this plan).
- **Committed in:** `f4de1ac` (Task 2) — no additional commit needed; this is a documentation note, not a code change.

---

**Total deviations:** 1 auto-fixed (comment wording), 1 noted-not-fixed (pre-existing unrelated string collision)
**Impact on plan:** Both are documentation-level; no functional behavior differs from what the plan specified. No scope creep — no files outside this plan's declared `files_modified` (`src/App.tsx`, `src/components/MacroCard.tsx`) were touched.

## Issues Encountered

None blocking. Both tasks are `type="auto"` with no checkpoints in this plan. Per the plan's own `<verification>` section, the "Manual: ..." acceptance-criteria rows (full-field edit persists across restart, single-card mutual exclusion, inline delete confirm + failure banner + empty-state on last delete) are recommended manual QA passes for a future session (e.g. plan 10-06's final phase verification), consistent with the precedent set by 10-04.

Addressed the `known_context` hint about the Wave 3 checkpoint bug report directly: `handleRemoveMacro` (the function containing the `window.confirm()` call at the old `App.tsx:577`) is fully removed, not reskinned around — `grep -cE 'window\.confirm' src/App.tsx` returns 0, and the delete path now runs entirely through `confirmingDeleteId`/`handleDeleteStart`/`handleDeleteConfirm` → the existing `remove_macro` IPC. This genuinely fixes the underlying WKWebView `window.confirm()` unreliability, since no call to it remains anywhere in the delete path.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 10-05 is complete.** Both tasks done, no checkpoints. `requirements mark-complete` WAS run for `UX-08` and `UX-09` — unlike 10-01/10-02/10-04's precedent of deferring, both requirements' full described behavior (edit every field with persistence; delete via in-app confirmation, no native dialog) is now fully implemented end-to-end by this plan.
- Plan 10-06 (final polish/verification, per the phase's 6-plan wave structure) can proceed with the full UI-SPEC Verification Checklist — including the UI-04 perf gate follow-up noted as a blocker in Phase 10 Plan 03 (STATE.md Blockers section) and the manual QA items this plan's own tasks flagged (full-field edit persistence across restart, mutual exclusion, delete-confirm/failure-banner/empty-state visual checks).

## Self-Check: PASSED

`src/components/MacroCard.tsx` verified present; both commit hashes (`329d3fe`, `f4de1ac`) verified present in `git log`.

---
*Phase: 10-ui-redesign-macro-management*
*Completed: 2026-07-24*
