---
phase: 10-ui-redesign-macro-management
plan: 01
subsystem: ui
tags: [tauri, rust, solidjs, intent-model, css-translucency, backdrop-filter]

# Dependency graph
requires:
  - phase: 09-parallel-macro-execution
    provides: StateActor Intent model (SetMacroTriggerKey/resolve_trigger_key_update conflict-check pattern), auto_save_default/reevaluate_all_macros/recompute_conflicts mutation pipeline
provides:
  - "Intent::UpdateMacro + update_macro IPC command (registered), the sole backend surface UX-09's full edit form (plan 10-05) will build on"
  - "Inline card name-edit affordance (✎ button, editingField='name' sibling state) proving the D-13 expand-in-place pattern end-to-end"
  - "Real translucent .glass-card (color-mix background + functioning backdrop-filter) as the D-03 foundation later plans (10-02+) extend to the sidebar/tokens"
affects: ["10-02 (theme tokens + sidebar translucency)", "10-04 (Input/Mode selectors)", "10-05 (full MacroForm edit generalization)", "10-06 (final polish/verification)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Atomic multi-field Intent update: resolve_trigger_key_update() conflict pre-check, mutate ALL fields together only on Ok, single auto_save_default() write per edit"
    - "editingField union extended with a new state value (name) as a sibling to existing key/target inline editors, still gated on the single centralized editingCardId signal for mutual exclusion"

key-files:
  created: []
  modified:
    - src-tauri/src/state/mod.rs (Intent::UpdateMacro variant + handler + 3 unit tests)
    - src-tauri/src/ipc/mod.rs (update_macro #[command])
    - src-tauri/src/lib.rs (update_macro registered in generate_handler!)
    - src/App.tsx (editingField gains "name"; ✎ button; inline name editor; handleStartEditMacroName/handleSaveMacroName/handleCancelEditMacroName)
    - src/App.css (.glass-card real translucency)

key-decisions:
  - "editingField gained a third value ('name') rather than gating the full-card name editor on editingCardId alone — prevents the name editor and the existing key/target inline editors from rendering simultaneously on the same card while still sharing the single centralized editingCardId signal for cross-card mutual exclusion"
  - "Persistence round-trip test uses a direct serde_json round-trip through ProfileData (no ProfileManager/AppHandle) — mirrors the existing profile_backwards_compat test's style since this test module has no way to construct a real tauri::AppHandle headlessly"
  - "requirements mark-complete was NOT run for UX-09/UI-01 — gsd-tools requirements ready-ids confirmed both are still 'blocked' (span 5 more plans in this phase); marking them complete now would be premature"

requirements-completed: []

coverage:
  - id: D1
    description: "Intent::UpdateMacro + update_macro IPC atomically updates name/sequence/trigger_mode/target_app/trigger_key together, rejecting on trigger-key conflict without mutating any field"
    requirement: "UX-09"
    verification:
      - kind: unit
        ref: "src-tauri/src/state/mod.rs#update_macro_applies_all_fields"
        status: pass
      - kind: unit
        ref: "src-tauri/src/state/mod.rs#update_macro_conflict_no_partial_mutation"
        status: pass
      - kind: unit
        ref: "src-tauri/src/state/mod.rs#update_macro_persists_across_round_trip"
        status: pass
      - kind: other
        ref: "cd src-tauri && cargo build (exit 0)"
        status: pass
  - id: D2
    description: "Inline ✎ edit affordance on a macro card: opens a name editor pre-filled with the current name, Save Changes calls update_macro, card collapses on success; a hotkey conflict keeps the editor open via the existing ConflictErrorToast"
    requirement: "UX-09"
    verification:
      - kind: other
        ref: "npx tsc --noEmit (exit 0, strict noUnusedLocals/noUnusedParameters)"
        status: pass
    human_judgment: true
    rationale: "End-to-end click-through (open editor, edit name, Save, verify card shows new name, restart app, verify persistence) requires a running Tauri desktop app with a display — no frontend test framework exists in this project (per RESEARCH.md's Validation Architecture) and this plan has no checkpoint task, so the automated gate (tsc clean) is the executor-side proof; the interactive walkthrough is deferred to human UAT."
  - id: D3
    description: ".glass-card renders genuine translucency — color-mix background + functioning backdrop-filter blur, replacing the prior opaque-gradient/inert-blur combination"
    requirement: "UI-01"
    verification:
      - kind: other
        ref: "grep -c color-mix src/App.css (>=1) + backdrop-filter present in .glass-card rule"
        status: pass
    human_judgment: true
    rationale: "Visual translucency (content behind a card visibly blurred, not a flat gradient) and the UI-04 idle CPU/GPU perf delta both require a running app on a real display to judge — this is explicitly called out as manual-only in RESEARCH.md's Validation Architecture (no automated perf-profiling tooling exists in this project's CI)."

duration: 25min
completed: 2026-07-24
status: complete
---

# Phase 10 Plan 01: Macro-Edit Tracer + Translucency Foundation Summary

**Intent::UpdateMacro/update_macro IPC (atomic, conflict-safe) plus an inline card name-edit affordance over a genuinely translucent `.glass-card` (`color-mix` + `backdrop-filter: blur(20px) saturate(150%)`)**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-07-24
- **Tasks:** 3/3
- **Files modified:** 5 (`src-tauri/src/state/mod.rs`, `src-tauri/src/ipc/mod.rs`, `src-tauri/src/lib.rs`, `src/App.tsx`, `src/App.css`)

## Accomplishments

- Added `Intent::UpdateMacro` — a single atomic backend mutation that reuses `resolve_trigger_key_update` for the trigger-key conflict check (not reimplemented) and mutates `name`/`sequence`/`trigger_mode`/`target_app`/`trigger_key`/`trigger_modifiers` together only on success, then calls `reevaluate_all_macros()` + `recompute_conflicts()` + `auto_save_default()` exactly once
- Registered the `update_macro` `#[command]` (mirrors `bind_hotkey`'s oneshot + double-unwrap shape) in `ipc/mod.rs` and `lib.rs`'s `generate_handler!`
- Added 3 backend unit tests proving atomic all-field apply, conflict-rejection-without-mutation, and a serde persistence round-trip
- Wired a thin, production-quality inline name-edit slice on the macro card: a new `✎` button (C-E1 neutral-tinted classes) opens a name `<input>` + `Save Changes`/`Cancel`, gated on the existing centralized `editingCardId` signal (with `editingField` extended to a `"name"` value for mutual exclusion with the target/key inline editors)
- Replaced `.glass-card`'s opaque `linear-gradient` background with `color-mix(in srgb, var(--color-surface) 72%, transparent)`, making the existing (previously inert) `backdrop-filter` visually real for the first time — interim per D-03, finalized/extended (tokens, `.sidebar-glass`) in plan 10-02

## Task Commits

Each task was committed atomically:

1. **Task 1: Backend Intent::UpdateMacro + update_macro IPC + handler registration** - `6dd8abb` (feat)
2. **Task 2: Backend unit tests — atomic apply, conflict-no-mutation, persistence round-trip** - `3fd2d12` (test)
3. **Task 3: Frontend inline name-edit slice + translucent .glass-card** - `667e19b` (feat)

## Files Created/Modified

- `src-tauri/src/state/mod.rs` - `Intent::UpdateMacro` enum variant + handler arm (mirrors `SetMacroTriggerKey`); 3 new unit tests (`update_macro_applies_all_fields`, `update_macro_conflict_no_partial_mutation`, `update_macro_persists_across_round_trip`)
- `src-tauri/src/ipc/mod.rs` - `update_macro` `#[command(rename_all = "snake_case")]`, oneshot channel to `Intent::UpdateMacro`
- `src-tauri/src/lib.rs` - `ipc::update_macro` added to `generate_handler!`
- `src/App.tsx` - `editingField` type gains `"name"`; new `editingMacroName` signal; `handleStartEditMacroName`/`handleSaveMacroName`/`handleCancelEditMacroName`; `✎` button + inline name editor in the macro card header
- `src/App.css` - `.glass-card` background/`backdrop-filter` values replaced per D-03

## Decisions Made

- **editingField extended with `"name"` (not a bare `editingCardId`-only gate):** the plan's literal instruction only mentions `editingCardId(macro.id)`, but gating the name editor purely on `editingCardId() === macro.id` would render it simultaneously with the existing target/key inline editors (both also gated on `editingCardId() === macro.id && editingField() === "..."`) if a user opened one then the other on the same card. Adding `"name"` to the `editingField` union keeps the single centralized signal (mutual exclusion across cards, per RESEARCH.md's explicit pitfall) while preventing same-card editor overlap — a Rule 1 (bug-prevention) refinement, not a scope change.
- **Persistence round-trip test uses direct `serde_json` round-trip, not `ProfileManager`:** `state/mod.rs`'s test module cannot construct a real `tauri::AppHandle` headlessly (documented precedent: `persistence.rs`'s own test-module comment on why its `StateActor`-level round-trip tests are infeasible there too). Mirrored the existing `profile_backwards_compat` test's approach — serialize/deserialize a hand-built `ProfileData` and assert the edited macro's fields survive — which is the same underlying on-disk shape `auto_save_default` produces.
- **`requirements mark-complete` was skipped for UX-09/UI-01:** `gsd-tools query requirements ready-ids` was run against this plan's frontmatter IDs and returned both as `"blocked"` (they span plans 10-02 through 10-06). Marking them complete after only this plan would misrepresent phase progress in `REQUIREMENTS.md`.

## Deviations from Plan

None - plan executed exactly as written. The one interpretive choice (extending `editingField` with `"name"` instead of a bare `editingCardId`-only gate) is documented above as a decision, not a deviation — it implements the plan's own stated goal ("mutual exclusion") more precisely than a literal reading would have.

## Issues Encountered

None. `cargo build`, `cargo test` (24/24 passing, including the 3 new tests), and `npx tsc --noEmit` are all clean.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The backend edit surface (`Intent::UpdateMacro`/`update_macro`) is proven end-to-end for one field and ready for plan 10-05 to generalize to the full field set (Input/Mode selectors, key-capture, target-app) without any further backend changes.
- `.glass-card`'s translucency mechanism is proven and ready for plan 10-02 to extend to the sidebar (`.sidebar-glass`) and finalize the token restructure (`@theme`/`[data-theme]`).
- **Outstanding manual verification (human UAT, not blocking this plan's completion):** click through the app — open `✎` on a macro, edit the name, click `Save Changes`, confirm the card shows the new name, restart the app, confirm the name persisted; and visually confirm `.glass-card` shows genuine blur of content behind it. No frontend test framework and no CI perf-profiling tooling exist in this project (per RESEARCH.md), so this remains a human-only check, consistent with how prior phases (8, 9) have deferred real-device/manual verification items.
- The UI-04 idle CPU/GPU perf measurement (Activity Monitor/Task Manager vs. the v1.2.0 baseline) called for in RESEARCH.md/UI-SPEC has NOT been run in this plan — it is scoped as an early-wave gate per RESEARCH.md's Pitfall 3 guidance, and only sidebar/card blur exists so far (no sidebar yet — that lands in 10-02). Recommend running the measurement once 10-02's sidebar translucency lands, while the fallback (reduce blur layer count) can still be applied cheaply.

---
*Phase: 10-ui-redesign-macro-management*
*Completed: 2026-07-24*
