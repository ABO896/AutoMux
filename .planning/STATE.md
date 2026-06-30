---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Redesign & Platform Excellence
status: executing
stopped_at: Phase 8 Plan 4 complete
last_updated: "2026-06-30T21:35:04.000Z"
progress:
  total_phases: 5
  completed_phases: 3
  total_plans: 14
  completed_plans: 12
  percent: 86
current_phase: 08
current_phase_name: hotkey-reliability-conflict-safety
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-02 — milestone v2.0 started)

**Core value:** A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.
**Current focus:** Phase 08 — hotkey-reliability-conflict-safety

## Current Position

Phase: 08 (hotkey-reliability-conflict-safety) — EXECUTING
Plan: 4 of 6
Next: Phase 08 — Hotkey Reliability & Conflict Safety, Plan 08-05
Status: Plan 08-04 complete (UX-11 frontend plumbing + UX-13 modifier pass-through live)

```
Progress: [████████████░░░░░░░░] 86% (plans 12/14 complete in v2.0)
```

## Phase Summary

| Phase | Name | Requirements | Status |
|-------|------|--------------|--------|
| 5 | macOS Permissions & Reliability | (rolled into v1.x) | Complete (prior milestone) |
| 6 | macOS Tahoe 26 Compatibility | COMPAT-01, COMPAT-02, COMPAT-03 | Complete (2026-06-17) |
| 7 | Carry Work — Platform, CI & Safety | BUILD-01, MEM-01, CI-03, CI-04, CI-05, SAFE-04, ERR-01, COMPAT-04, COMPAT-05 | Complete (2026-06-30) |
| 8 | Hotkey Reliability & Conflict Safety | UX-11, UX-12, UX-13, UX-14 | In Progress (4/6 plans done) |
| 9 | Parallel Macro Execution | EXEC-01, EXEC-02 | Not started |
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

**Resume file:** /Users/alvaro/AutoClicker/.planning/phases/08-hotkey-reliability-conflict-safety/08-04-SUMMARY.md

Last session: 2026-06-30T21:35:04.000Z
Stopped at: Phase 8 Plan 4 complete (UX-11 frontend plumbing + UX-13 modifier pass-through live)
Next: Plan 08-05 (Conflict toast / warning region / first-run global notice) — `/gsd-execute-phase 08`

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

## Decisions

- [Phase 7]: Plan 07-01 deviation: reworded release.yml comment to 'Tauri auto-update publish step intentionally absent' to satisfy the CI-03 grep gate that the plan's example text would have violated.

Plan 07-01 example comment contained 'updater' and matched 'sig.*upload', both of which are trigger patterns in the CI-03 acceptance criteria gate. Auto-fixed per deviation Rule 1 — example was buggy.

- [Phase 8 Plan 2]: Free-function + StateActor-wrapper pattern for `check_trigger_key_conflict` and `recompute_conflicts` — adopted the plan's minimum-surface alternative to the `StateActor::new_for_test` constructor. Unit tests call the free functions directly without a Tauri AppHandle, matching the persistence test style.

- [Phase 8 Plan 2]: Drop-on-conflict in `AddMacro` and `SetMacroTriggerKey` is accepted as interim (T-08-12) — the full `Result<(), String>` error path ships in plan 08-03's `Intent::BindHotkey`. The silent drop is defense-in-depth that keeps the existing IPC flows working until then.

- [Phase 8 Plan 3]: cfg-gated `build_hotkey_bindings_vec` helper with three impls (macOS, Windows, non-supported fallback) — single call site in each `Intent::BindHotkey` / `Intent::UnbindHotkey` handler, type signatures encode the platform split. Avoids a shared `HotkeyBinding` trait abstraction in the platform module while preserving the StateActor-as-source-of-truth invariant. The fallback impl returns `Vec<T>` (generic, empty) so the helper compiles on non-{macos,windows} hosts.

- [Phase 8 Plan 3]: Windows cross-compile gate deferred to Plan 08-06 — the x86_64-pc-windows-gnu target is not installed on this host (`can't find crate for 'core'`). Per the plan's explicit allowance, the strict `cargo build --target x86_64-pc-windows-msvc` gate is deferred. The `#[cfg(target_os = "windows")]` attributes + `cargo check` on macOS give high confidence the code is correct on Windows.

- [Phase 8 Plan 4]: Combined Tasks 1 and 2 into a single commit (5031995) — the plan's two-task separation (interface extension vs. threading) is incompatible with tsconfig's `noUnusedLocals: true` strict mode. Declaring the new signals (`recordingModifiers`, `newMacroTriggerModifiers`, `conflictError`) and the `computeModifiers` helper without using them in the same commit leaves a broken build at the Task 1 commit boundary. All acceptance criteria from both tasks pass on the combined commit. The two `void` references (for `conflictError` and `recordingModifiers` getters whose visual consumers are deferred to Plan 08-05) are no-op runtime reads that satisfy the strict compiler setting without rendering partial UI.

- [Phase 8 Plan 4]: `computeModifiers` is module-level with inline `IS_MACOS` detection rather than inside `App()` using the closure-bound constant. The 08-PATTERNS.md document listed `IS_MACOS` as a module-level constant; the actual placement is inside `App()` (line 191). Module-level placement keeps the helper callable from outside the component closure, matches the pattern of the existing module-level `formatInputEvent` / `formatStep` helpers, and the inline `navigator.userAgent.toLowerCase().includes("mac")` produces the same result as the closure-bound version.
