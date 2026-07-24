---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Redesign & Platform Excellence
current_phase: 10
current_phase_name: ui-redesign-macro-management
status: executing
stopped_at: "Completed 10-04-PLAN.md (KeyCaptureField.tsx + MacroForm.tsx extraction, Key Press selectable, UX-10 relabeling); no checkpoints in this plan. Next: plan 10-05 (inline card-edit + delete confirmation, D-13/D-14)."
last_updated: "2026-07-24T14:36:49.413Z"
progress:
  total_phases: 6
  completed_phases: 5
  total_plans: 31
  completed_plans: 29
  percent: 83
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-02 — milestone v2.0 started)

**Core value:** A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.
**Current focus:** Phase 10 — ui-redesign-macro-management

## Current Position

Phase: 10 (ui-redesign-macro-management) — EXECUTING
Plan: 4 of 6 (10-04) — Complete. Extracted KeyCaptureField.tsx (reusable key-capture widget) and MacroForm.tsx (shared field set incl. selectable Key Press per D-11, UX-10 two-selector relabeling). No checkpoints in this plan. See 10-04-SUMMARY.md. Next: plan 10-05.
Status: Executing (Phase 10, Plan 05 next)

Phase 9 (parallel-macro-execution) note: source-level work is complete and independently re-verified against source in 09-VERIFICATION.md (2026-07-22T21:30:00Z) — both prior Blocker gaps closed (HoldRelease Gate 1/2/3 bypass; hotkey-rebind rollback safety on both platforms). 16/16 backend tests pass; `cargo build` and `npx tsc --noEmit` are clean. The only remaining work is human real-device verification — macOS tests T9.1-T9.7 and Windows tests 6.1-6.3 (Windows never yet run on a real device across any verification pass for this phase).

Separately, Phase 8 (hotkey-reliability-conflict-safety) remains blocked on human device verification — Sections 5 + 6 of 08-VERIFICATION.md (manual macOS + Windows hotkey tests) are still pending and unrelated to Phase 9's progress.

```
Progress: [█████████░] 94% (plans 25/25 complete in v2.0)
```

## Phase Summary

| Phase | Name | Requirements | Status |
|-------|------|--------------|--------|
| 5 | macOS Permissions & Reliability | (rolled into v1.x) | Complete (prior milestone) |
| 6 | macOS Tahoe 26 Compatibility | COMPAT-01, COMPAT-02, COMPAT-03 | Complete (2026-06-17) |
| 7 | Carry Work — Platform, CI & Safety | BUILD-01, MEM-01, CI-03, CI-04, CI-05, SAFE-04, ERR-01, COMPAT-04, COMPAT-05 | Complete (2026-06-30) |
| 8 | Hotkey Reliability & Conflict Safety | UX-11, UX-12, UX-13, UX-14 | In Progress (6/6 plans done; 4/4 automated gates green, 2/2 manual device test plans documented — awaiting human device verification) |
| 9 | Parallel Macro Execution | EXEC-01, EXEC-02 | Source-level complete / human_needed (11/11 plans done; both Blocker gaps — HoldRelease gating stuck-input, hotkey-rebind data loss — closed by 09-11 and independently re-confirmed against source 2026-07-22T21:30:00Z; 16/16 backend tests pass, cargo build + tsc clean; only real-device tests T9.1-T9.7 (macOS) / 6.1-6.3 (Windows) remain, Windows never yet run on-device) |
| 10 | UI Redesign & Macro Management | UI-01, UI-02, UI-03, UI-04, UX-08, UX-09, UX-10 | Not started — now unblocked (Phase 9's source-level Blocker gaps are closed; Phase 9 real-device confirmation continues in parallel) |

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

Last session: 2026-07-24T14:36:49.405Z
Stopped at: Completed 10-04-PLAN.md (KeyCaptureField.tsx + MacroForm.tsx extraction, Key Press selectable, UX-10 relabeling); no checkpoints in this plan. Next: plan 10-05 (inline card-edit + delete confirmation, D-13/D-14).
Next: Phase 9 routes to human real-device verification only — macOS tests T9.1-T9.7 and Windows tests 6.1-6.3 (Windows never yet run on a real device). No further Phase 9 code work is pending: both Blocker gaps are closed, 16/16 backend tests pass, `cargo build` and `npx tsc --noEmit` are clean. Separately and independently, Phase 8 still awaits its own human device verification (Sections 5+6 of 08-VERIFICATION.md) — unrelated to Phase 9. Phase 10 (UI redesign) may now proceed since Phase 9's source-level Blocker gaps are closed, with Phase 9 real-device confirmation continuing in parallel. Both out-of-scope Critical findings filed as todos are now resolved: LoadProfile data loss via quick task 260723-k9l, and Windows hook injected-event filtering via quick task 260723-krr — both moved to .planning/todos/completed/.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260723-jt3 | Reconcile Phase 9 planning docs: fix stale STATE.md continuity text, reconcile REQUIREMENTS.md EXEC-01/EXEC-02 annotations with 09-VERIFICATION.md, file two new backlog items (LoadProfile data loss, Windows hook injected-keystroke filtering) | 2026-07-23 | 6e76eb6 | [260723-jt3-reconcile-phase-9-planning-docs-fix-stal](./quick/260723-jt3-reconcile-phase-9-planning-docs-fix-stal/) |
| 260723-k9l | Fix LoadProfile failure wiping all macros (memory + disk): reordered Intent::LoadProfile so macros.clear()/auto_save_default() run only on a successful load, Err branch restarts scheduler tasks without persisting; added persistence-layer regression test; closed backlog item 09-REVIEW-CR-01 | 2026-07-23 | 88ee2a7 | [260723-k9l-fix-loadprofile-failure-wiping-all-macro](./quick/260723-k9l-fix-loadprofile-failure-wiping-all-macro/) |
| 260723-krr | Fix Windows keyboard hook not filtering injected keystrokes (CR-02): added flags_indicate_injected predicate + hook_callback gating so a macro's own SendInput-synthesized keystroke can no longer self-trigger the Ctrl+Shift+Q emergency stop or another macro's hotkey binding; closed backlog item | 2026-07-23 | 6ab0b66 | [260723-krr-fix-windows-keyboard-hook-not-filtering-](./quick/260723-krr-fix-windows-keyboard-hook-not-filtering-/) |

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
| Phase 09 P03 | 2min | 2 tasks | 1 files |
| Phase 09 P04 | 6min | 2 tasks | 1 files |
| Phase 09 P05 | 8min | 2 tasks | 1 files |
| Phase 09 P06 | 4min | 2 tasks | 1 files |
| Phase 09 P07 | 20min | 2 tasks | 1 files |
| Phase 09 P08 | 5min | 2 tasks | 1 files |
| Phase 09 P09 | 10min | 2 tasks | 2 files |
| Phase 09 P10 | 5min | 3 tasks | 4 files |
| Phase 09 P11 | 10min | 3 tasks | 3 files |
| Phase 10 P01 | 25min | 3 tasks | 5 files |
| Phase 10 P02 | 10min | 3 tasks | 4 files |
| Phase 10 P04 | 13min | 2 tasks | 3 files |

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
- [Phase ?]: [Phase 9 Plan 3]: Documented that Phase 9 (via plan 09-01) fully resolved the one pre-existing needless_return clippy warning that Phase 8 had documented as pre-existing-but-accepted — cargo clippy now exits 0 with zero warnings on the macOS host, an improvement over Phase 8's disposition, not just parity with it.
- [Phase ?]: [Phase 9 Plan 4] CR-01 gap-closure: added trigger_mode === Hold branch to computeRunningState (Approach A, display-only) and folded in WR-01 by computing the running-state once per macro card via a reactive thunk instead of 3 duplicated calls.
- [Phase ?]: [Phase 9 Plan 5]: CR-01 gap-closure — converted release_holds/stop_macro/start_macro to async guaranteed .await HoldRelease delivery, mirroring StopAll's existing CR-02 pattern; the two D-09-scoped diagnostic try_send sites (fire_due_actions Interval, start_macro HoldStart) intentionally left untouched.
- [Phase ?]: [Phase 9 Plan 6] CR-01 final gap-closure: converted start_macro's SustainedHold HoldStart send from try_send to guaranteed .await, mirroring release_holds/StopAll from plan 09-05 — closes the last Blocker-severity asymmetric-HoldStart gap for Phase 9.
- [Phase ?]: [Phase 9 Plan 7] G-09-1a/G-09-1c gap closure: narrowed CGEventTap event mask to 8 consumed types (removes permanent post-stop responsiveness tax) and hardened NSWorkspace active-app observer (explicit registration-failure logging, userInfo-based active-app reads with frontmostApplication() fallback, debug-instrumented active-app-changed logging). Kept raw msg_send! registration over the typed addObserverForName_object_queue_usingBlock API because the typed method's non-Optional return type cannot represent a registration failure.
- [Phase ?]: [Phase 9 Plan 8] G-09-1b/G-09-1c gap closure: added handleRemoveMacro handler + per-card delete button (mirrors handleDeleteProfile conventions, no manual state refresh — relies on existing state-changed broadcast), and bound value={macro.target_app ?? ""} on the card-edit target select to fix the uncontrolled-dropdown display bug.
- [Phase ?]: Phase 9 Plan 9: Chose backend rename_all direction over frontend camelCase rename for the IPC argument-casing sweep, since the frontend already sends snake_case keys verbatim; closed 09-VERIFICATION.md gaps #10/#11.
- [Phase ?]: [Phase 9 Plan 10] CR-01 final closure: consolidated to single HOTKEY_BINDINGS registry on both platforms (refreshed unconditionally from reevaluate_all_macros, before the engine-active early-return); deleted the redundant MACRO_TRIGGER_KEYS registry and its per-platform keydown/hook lookup block entirely, closing the dual-registry double-dispatch self-cancel bug symmetrically on macOS and Windows.
- [Phase ?]: [Phase 9 Plan 11] Gap-closure: action_should_inject free fn gives HoldRelease an unconditional Gate 1/2/3 bypass in handle_action (stuck-input fix); resolve_trigger_key_update + Result-carrying SetMacroTriggerKey oneshot rejects conflicting Windows rebinds instead of silently coercing to None/0; macOS handleCardSetTriggerKey no longer pre-unbinds before bind_hotkey, relying on its overwrite-on-success/preserve-on-conflict semantics. Closes both Blocker gaps from 09-VERIFICATION.md.
- [Phase ?]: [Quick 260723-k9l]: Gated Intent::LoadProfile's macros.clear()+auto_save_default() behind a successful profile load (Ok branch only); Err branch restarts scheduler tasks via reevaluate_all_macros() without persisting. Closes 09-REVIEW.md CR-01 data-loss bug; regression test added in persistence.rs.
- [Phase ?]: [Quick 260723-krr]: Fixed Windows keyboard hook missing LLKHF_INJECTED filter (09-REVIEW.md CR-02) — added flags_indicate_injected predicate + hook_callback gating so a macro's own SendInput-synthesized keystroke can no longer self-trigger the Ctrl+Shift+Q emergency stop or another macro's hotkey binding, mirroring the macOS CGEventTap's existing LLMHF_INJECTED guard. Predicate + tests relocated to platform/mod.rs mid-execution because pub mod windows; is itself cfg(target_os=windows)-gated, making anything defined inside windows/mod.rs untestable on macOS regardless of its own gating. Closed backlog item.
- [Phase ?]: [Phase 10 Plan 1]: editingField gained a third value ('name') as a sibling to the existing key/target inline editors, still gated on the single centralized editingCardId signal — prevents the new name editor and the target/key editors from rendering simultaneously on the same card.
- [Phase ?]: [Phase 10 Plan 1]: update_macro's persistence round-trip test uses a direct serde_json round-trip through ProfileData (no ProfileManager/AppHandle) — mirrors the existing profile_backwards_compat test's style since state/mod.rs's test module cannot construct a real tauri::AppHandle headlessly.
- [Phase ?]: [Phase 10 Plan 1]: requirements mark-complete was NOT run for UX-09/UI-01 — gsd-tools requirements ready-ids confirmed both are still 'blocked' (span plans 10-02 through 10-06 of this phase); marking them complete after only this plan would misrepresent phase progress in REQUIREMENTS.md.
- [Phase ?]: [Phase 10 Plan 2]: resolveTheme not imported into App.tsx (applyTheme already resolves internally; strict noUnusedLocals would fail an unused import) — themePreference's setter kept live via a bare void reference pending plan 10-03's ThemeToggle consumer, same deferred-consumer precedent as Phase 8 Plan 4.
- [Phase ?]: [Phase 10 Plan 2]: requirements mark-complete NOT run for UI-01/UI-03/UI-04 — this plan only lays the token/translucency foundation; full redesign spans plans 10-03 through 10-06 and the UI-04 perf gate is deferred to plan 10-03.
- [Phase ?]: [Phase 10 Plan 4]: KeyCaptureField.tsx collapses the card's prior 3-branch trigger-key Show tree into one component instance (not-set + committed sub-states shared an identical onClick handler); gets a compact boolean prop for two visual variants (card meta-row chip vs MacroForm select-sized box).
- [Phase ?]: [Phase 10 Plan 4]: Key Press action-key capture renders as its own row below Input+interval (not literally replacing the interval field) so a Key Press + Pulse macro's repeat interval stays editable — chosen over a more literal but functionality-losing reading of UI-SPEC's ambiguous 'renders in the input slot' wording. No modifiers signal created for the action-key capture since InputEvent::Key has no modifiers field.

### Blockers

- Phase 10 Plan 03 Task 3: blocking checkpoint:human-verify — early UI-04 idle CPU/GPU perf gate (all blur surfaces active: .glass-card + .sidebar-glass) plus both-theme visual spot-check on a real macOS Tahoe device. Tasks 1-2 (Sidebar.tsx, ThemeToggle.tsx, window resize to 720x680) are committed (dbdc8ee, 3f5237d); plan 10-03 cannot be marked complete until this checkpoint is approved or the documented blur-reduction fallback is applied and re-measured.
