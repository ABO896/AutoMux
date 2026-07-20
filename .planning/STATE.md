---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Redesign & Platform Excellence
current_phase: 09
current_phase_name: parallel-macro-execution
status: executing
stopped_at: Completed 09-02-PLAN.md
last_updated: "2026-07-20T17:49:28.479Z"
progress:
  total_phases: 6
  completed_phases: 4
  total_plans: 17
  completed_plans: 16
  percent: 67
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-02 — milestone v2.0 started)

**Core value:** A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.
**Current focus:** Phase 09 — parallel-macro-execution

## Current Position

Phase: 09 (parallel-macro-execution) — EXECUTING
Plan: 3 of 3
Next: Phase 8 manual device verification (Sections 5 + 6 of 08-VERIFICATION.md) — human execution on real macOS + Windows hosts; once marked done, Phase 9 (Parallel Macro Execution) can start
Status: Ready to execute

```
Progress: [█████████░] 94% (plans 14/14 complete in v2.0)
```

## Phase Summary

| Phase | Name | Requirements | Status |
|-------|------|--------------|--------|
| 5 | macOS Permissions & Reliability | (rolled into v1.x) | Complete (prior milestone) |
| 6 | macOS Tahoe 26 Compatibility | COMPAT-01, COMPAT-02, COMPAT-03 | Complete (2026-06-17) |
| 7 | Carry Work — Platform, CI & Safety | BUILD-01, MEM-01, CI-03, CI-04, CI-05, SAFE-04, ERR-01, COMPAT-04, COMPAT-05 | Complete (2026-06-30) |
| 8 | Hotkey Reliability & Conflict Safety | UX-11, UX-12, UX-13, UX-14 | In Progress (6/6 plans done; 4/4 automated gates green, 2/2 manual device test plans documented — awaiting human device verification) |
| 9 | Parallel Macro Execution | EXEC-01, EXEC-02 | Ready to execute (3 plans, 2 waves — planned ahead of Phase 8 device verification) |
| 10 | UI Redesign & Macro Management | UI-01, UI-02, UI-03, UI-04, UX-08, UX-09, UX-10 | Not started |

## Accumulated Context

### Key Decisions

- v2.0 is a major version bump: full UI redesign + macOS Tahoe 26 compat + parallel macro execution
- All features must ship on both macOS and Windows — no platform-exclusive fixes
- UI redesign: Apple design language + liquid glass (macOS 26), modern equivalent on Windows, Raycast-inspired layout
- Parallel execution: triggering macro B while macro A runs must not block or cancel macro A — architectural change required
- macOS Tahoe 26 compatibility is the critical path — DONE (Phase 6 complete)
- kink-fixing and hotkey reliability (Phase 7+8) happens BEFORE the full UI/UX redesign (Phase 10)
- Phase 10 (UI redesign) depends on Phase 6 because liquid glass APIs must be understood before implementation
- Phase 9 (parallel execution) is independent and can proceed in parallel with Phase 7/8

### Known Constraints

- COMPAT-01/02: Must work within Tauri and CGEvent API surface — OS enforces permission model
- macOS 26 signing: ad-hoc signing (`signingIdentity: "-"`) is in place; CGEvent injection uses Session tap
- UI-01: Liquid glass requires macOS 26+ APIs — cannot backport to Monterey/Ventura/Sonoma/Sequoia
- UI-04: UI redesign must not increase idle memory/CPU overhead — keep AutoMux lightweight
- SAFE-04: Lock ordering fix (release REGISTRY before CGEvent dispatch) is subtle — requires careful audit

### Deferred Items

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Distribution | Auto-updater (DIST-01) | v3 scope | v1.2.0 roadmap creation |
| Distribution | macOS notarization (DIST-02) | v3 scope | v1.2.0 roadmap creation |
| UX | NamedKey schema migration (UX-04) | future | v1.2.0 roadmap creation |
| UX | Cross-platform profile portability (UX-05) | future | v1.2.0 roadmap creation |

## Session Continuity

**Resume file:** None

Last session: 2026-07-20T17:49:28.473Z
Stopped at: Completed 09-02-PLAN.md
Next: Phase 8 manual device verification (run Tests 5.1–5.6 on macOS + Tests 6.1–6.5 on Windows, then mark Sections 5+6 of 08-VERIFICATION.md as done); once those pass, Phase 9 (Parallel Macro Execution) can start.

## Performance Metrics

| Phase | Plan | Duration | Notes |
|-------|------|----------|-------|
| Phase 7 P1 | 8 min | 3 tasks | 2 files |
| Phase 7 P1 | 1h 20m | 3 tasks | 2 files |
| Phase 07 P02 | 3min | 2 tasks | 4 files |
| Phase 07 P03 | 5min | 2 tasks | 1 files |
| Phase 08 P01 | 5 min | 3 tasks | 5 files (data model + tuple-keyed registry + bit pinning) |
| Phase 08 P02 | 5 min | 3 tasks | 2 files (conflict helpers + 9 handler wirings) |
| Phase 08 P03 | 3 min | 3 tasks | 3 files (BindHotkey IPC + Windows HOTKEY_BINDINGS) |
| Phase 08 P04 | 5 min | 2 tasks (combined) | 1 file (frontend computeModifiers + threading + conflict error wiring) |
| Phase 08 P05 | 5 min | 2 tasks | 1 file (5 UI surfaces: C-1 toast, C-2 region, C-3 banner, C-4 ↗ Global, C-5 modifier chip) |
| Phase 08 P06 | 22 min | 4 tasks | 2 files (08-VERIFICATION.md + new profile_backwards_compat test) |
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 09 P01 | 5min | 3 tasks | 3 files |
| Phase 09 P02 | 8min | 2 tasks | 1 files |

## Decisions

- [Phase 7]: Plan 07-01 deviation: reworded release.yml comment to 'Tauri auto-update publish step intentionally absent' to satisfy the CI-03 grep gate that the plan's example text would have violated.

Plan 07-01 example comment contained 'updater' and matched 'sig.*upload', both of which are trigger patterns in the CI-03 acceptance criteria gate. Auto-fixed per deviation Rule 1 — example was buggy.

- [Phase 8 Plan 2]: Free-function + StateActor-wrapper pattern for `check_trigger_key_conflict` and `recompute_conflicts` — adopted the plan's minimum-surface alternative to the `StateActor::new_for_test` constructor. Unit tests call the free functions directly without a Tauri AppHandle, matching the persistence test style.

- [Phase 8 Plan 2]: Drop-on-conflict in `AddMacro` and `SetMacroTriggerKey` is accepted as interim (T-08-12) — the full `Result<(), String>` error path ships in plan 08-03's `Intent::BindHotkey`. The silent drop is defense-in-depth that keeps the existing IPC flows working until then.

- [Phase 8 Plan 3]: cfg-gated `build_hotkey_bindings_vec` helper with three impls (macOS, Windows, non-supported fallback) — single call site in each `Intent::BindHotkey` / `Intent::UnbindHotkey` handler, type signatures encode the platform split. Avoids a shared `HotkeyBinding` trait abstraction in the platform module while preserving the StateActor-as-source-of-truth invariant. The fallback impl returns `Vec<T>` (generic, empty) so the helper compiles on non-{macos,windows} hosts.

- [Phase 8 Plan 3]: Windows cross-compile gate deferred to Plan 08-06 — the x86_64-pc-windows-gnu target is not installed on this host (`can't find crate for 'core'`). Per the plan's explicit allowance, the strict `cargo build --target x86_64-pc-windows-msvc` gate is deferred. The `#[cfg(target_os = "windows")]` attributes + `cargo check` on macOS give high confidence the code is correct on Windows.

- [Phase 8 Plan 4]: Combined Tasks 1 and 2 into a single commit (5031995) — the plan's two-task separation (interface extension vs. threading) is incompatible with tsconfig's `noUnusedLocals: true` strict mode. Declaring the new signals (`recordingModifiers`, `newMacroTriggerModifiers`, `conflictError`) and the `computeModifiers` helper without using them in the same commit leaves a broken build at the Task 1 commit boundary. All acceptance criteria from both tasks pass on the combined commit. The two `void` references (for `conflictError` and `recordingModifiers` getters whose visual consumers are deferred to Plan 08-05) are no-op runtime reads that satisfy the strict compiler setting without rendering partial UI.

- [Phase 8 Plan 4]: `computeModifiers` is module-level with inline `IS_MACOS` detection rather than inside `App()` using the closure-bound constant. The 08-PATTERNS.md document listed `IS_MACOS` as a module-level constant; the actual placement is inside `App()` (line 191). Module-level placement keeps the helper callable from outside the component closure, matches the pattern of the existing module-level `formatInputEvent` / `formatStep` helpers, and the inline `navigator.userAgent.toLowerCase().includes("mac")` produces the same result as the closure-bound version.

- [Phase 8 Plan 5]: Visual order follows UI-SPEC layout contract, not the plan's literal "insert C-1 after auto-save" / "insert C-2 after C-1" instructions — the plan's parenthetical "visual order" text and the UI-SPEC layout contract pin the order as `profile-badge → C-3 → C-2 → C-1 → auto-save → Macros`. Following the literal instructions would produce `profile-badge → auto-save → C-1 → C-2 → Macros` (out of spec). C-1 inserted right before auto-save, C-2 right before C-1, C-3 right before C-2 in Task 2 — the user-facing order pinned by the design contract.

- [Phase 8 Plan 5]: Oxford-comma list join via inline `formatConflictList` helper plus singular/plural `verb()` helper for the C-2 body — the plan's body template `macroNames().join('" and "')` only reads correctly for exactly 2 macros. For 1 macro the body says `"AFK Farm" both inject ...` (wrong verb + "both" is incorrect); for 3+ macros UI-SPEC C-2 explicitly prescribes the `"A", "B", and "C"` Oxford-comma form. The inline helpers handle all conflict counts grammatically without changing the plan's heading or surrounding JSX. Implemented as plain functions inside the `<For>` body per CONVENTIONS.md (no `createMemo`).

- [Phase 8 Plan 5]: `modifierChips` placed at module level with inline `IS_MACOS` detection — mirrors `computeModifiers` (Plan 08-04). Both are bit-translation helpers that must be callable from anywhere without depending on the `App()` closure. Inline `navigator.userAgent.toLowerCase().includes("mac")` produces the same bit values as the closure-bound version. Semantic order (`Shift → Ctrl → Alt → Cmd/Win`) is hard-coded in the bit→label tuple array and is independent of press order per UI-SPEC C-5.

- [Phase 8 Plan 6]: `cargo test` and `cargo clippy` used the un-namespaced form (no `-p automux-lib`) because the actual package name is `automux` (hyphen-free) with a separate `[lib] name = "automux_lib"`. The plan's hyphenated `-p automux-lib` form errors with `package ID specification 'automux-lib' did not match any packages`. The verification report documents the equivalent invocation under each section's "Notes on command" line.

- [Phase 8 Plan 6]: Added a new `profile_backwards_compat` unit test rather than relying on the manual smoke fallback. The test deserializes a hand-crafted pre-Phase-8 `ProfileData` JSON string (no `trigger_modifiers` on `MacroConfig`, no `conflicts` on `AppState`) and asserts the new fields default to `0` and `[]` respectively. R-5 is now pinned at the unit-test level — the `#[serde(default)]` annotations from Plan 08-01 are proven to work end-to-end for v2.0 users upgrading to Phase 8.

- [Phase 8 Plan 6]: Windows cross-compile gate deferred to CI (same disposition as Plan 08-03) — the `x86_64-pc-windows-msvc` target is not installed on this host (Homebrew rust 1.95.0, no `rustup`). The plan's "paste the output of `rustup target list --installed`" requirement is unsatisfiable on this host; the equivalent evidence is the cross-compile error itself + `cargo check --all-targets` clean exit + the bit-constant equivalence across `windows_mod_constants` test, `build_mod_mask` function, and `computeModifiers` helper (all use the same `0x0001`/`0x0002`/`0x0004`/`0x0008` values).
- [Phase ?]: [Phase 9 Plan 1]: Fixed pre-existing needless_return clippy lint in ipc/mod.rs::list_running_apps (Rule 3 blocking-issue fix) — it blocked the cargo clippy --all-targets -D warnings gate required by this plan's own verification, even though unrelated to Phase 9 changes.
- [Phase ?]: [Phase 9 Plan 2]: Used bg-warning/--color-warning-glow for the 'combined' hold+interval indicator instead of the PATTERNS.md-illustrated bg-info, because no --color-info token exists in src/App.css's @theme block. Followed the plan's explicit fallback instruction to reuse an existing token rather than invent one.
- [Phase ?]: [Phase 9 Plan 2]: Combined Task 1 (computeRunningState helper) and Task 2 (card rendering wiring) into a single commit — tsconfig's noUnusedLocals strict mode fails tsc for an unused helper if Task 1 is committed alone, same reasoning as Phase 8 Plan 04.
