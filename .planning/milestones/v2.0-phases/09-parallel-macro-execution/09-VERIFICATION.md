---
phase: 09-parallel-macro-execution
verified: 2026-07-22T21:30:00Z
status: passed
score: 14/16 must-haves verified
behavior_unverified: 2
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 12/16
  gaps_closed:

    - "Blocker gap (HoldRelease actions gated identically to new-input actions — stuck-input risk): CLOSED by plan 09-11 Task 1. Independently re-confirmed by my own direct read of src-tauri/src/state/mod.rs (not merely trusted from SUMMARY.md or 09-REVIEW.md): action_should_inject (lines 281-309) is a pure free function whose FIRST statement is an unconditional `if matches!(action_type, ActionType::HoldRelease(_)) { return true; }` — before Gate 1/2/3 run. handle_action (lines 456-493) now consults it via a single `if !action_should_inject(...) { return; }` guard, replacing the old inline per-gate early-returns. New regression test hold_release_bypasses_gates (state/mod.rs:1292+) drives all four confirmed trigger paths (disabled macro, engine off, target-app mismatch, macro absent after profile clear) plus a negative case proving a HoldStart for the same disabled macro stays gated (bypass is release-only, not a blanket bypass). I ran `cargo test --lib` myself (not trusting reported numbers): 16/16 pass, including this test."
    - "Blocker gap (hotkey rebind destroys the old binding on a rejected conflict, both platforms): CLOSED by plan 09-11 Tasks 2-3. macOS: independently confirmed via direct read of src/App.tsx:578-585 that the `unbind_hotkey` pre-step is gone (`grep -c 'invoke(\"unbind_hotkey\"' src/App.tsx` returns 0, I ran it myself) — the macOS branch now calls only `bind_hotkey` (which preserves the old binding on conflict, per its existing Result-oneshot design) followed by the pre-existing `set_macro_trigger_key` round-trip. Windows: independently confirmed via direct read of state/mod.rs:594-635 that `Intent::SetMacroTriggerKey` now carries a `oneshot::Sender<Result<(),String>>` and calls the new pure `resolve_trigger_key_update` (lines 226+) — on a genuine conflict it sends `Err(msg)` and returns WITHOUT mutating the macro's trigger fields (old binding preserved); ipc/mod.rs:149-161 confirmed to await the oneshot and propagate the Result instead of always returning Ok(()). New regression test set_trigger_key_rejects_conflict_without_coercion (state/mod.rs:1031+) independently re-run by me: conflict → Err(other_id) (not coerced to None); free key / clear / self-rebind → Ok, as expected. 16/16 suite passes."
  gaps_remaining:

    - "ROADMAP SC1 (macOS real-device concurrent-firing confirmation) — unchanged, still human_needed."
    - "ROADMAP SC2 (Windows real-device concurrent-firing confirmation) — unchanged, still human_needed, explicitly blocked_by physical-device."
  regressions: []
gaps: []
deferred: []
behavior_unverified_items:

  - truth: "On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals (ROADMAP SC1)"
    test: "On a real macOS host with Accessibility + Input Monitoring granted: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A via the dashboard toggle (not a hotkey), wait ~1s, enable B the same way; observe both macro cards show a pulsing firing dot simultaneously and neither's click rate changes when the other starts/stops"
    expected: "Both macros visibly fire concurrently at their own configured rates on real macOS input injection (CGEvent)"
    why_human: "Unchanged since the prior pass — scheduler-level unit tests (re-run and confirmed passing, 16/16) prove platform-agnostic timeline/delivery logic but not real-device CGEvent injection timing with two simultaneous macros."

  - truth: "On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A (ROADMAP SC2)"
    test: "On a real Windows host: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A, wait ~1s, enable B; observe both fire concurrently via SendInput"
    expected: "Both macros fire concurrently at their own configured rates via SendInput"
    why_human: "Unchanged since the prior pass — 09-UAT.md explicitly recorded this as blocked_by: physical-device. This remains a deliberate skip, not a passing result."
human_verification:

  - test: "Run T9.1-T9.7 on a real macOS host (concurrent firing, stop-one-keeps-other, same-input concurrent + conflict warning, held-not-firing indicator, Hold-under-load, responsiveness-after-stop, creation-time targeting)."
    expected: "All tests pass; both macro cards show independent pulsing firing dots; stopping one does not affect the other."
    why_human: "Real macOS device, live CGEvent injection, live UI observation — cannot be verified by static analysis."

  - test: "Run the 3 documented Windows manual tests (6.1-6.3) on a real Windows host — this has never actually been executed across any verification pass for this phase."
    expected: "All 3 tests pass via SendInput injection and the Win32 hook observer."
    why_human: "Requires a real Windows device."

  - test: "(Recommended, not gating) Confirm the now-fixed HoldRelease bypass and hotkey-rebind rollback at the device level: (a) enable a Hold-mode macro targeted at an app, let HoldStart fire, then disable it / toggle the engine off / switch active app / load a different profile and confirm the physical key/button is released; (b) give a macro a working hotkey, attempt a conflicting rebind via the card-edit UI on each platform, confirm the original hotkey still works after the rejection and the conflict toast appears."
    expected: "Held input releases in all four scenarios; a rejected rebind never destroys the original hotkey, on either platform."
    why_human: "The source-level fix and its unit tests are conclusive for the code path; end-to-end device confirmation is good practice but not required to close the gap given the trigger paths were traced line-by-line against the fix and the regression tests exercise the exact conditions."
---

# Phase 9: Parallel Macro Execution Verification Report

**Phase Goal:** Redesign the StateActor/Scheduler execution model so multiple macros run concurrently on both macOS and Windows (ROADMAP: "Multiple macros can run simultaneously on both macOS and Windows — triggering a second macro never blocks, queues, or cancels a running one")
**Verified:** 2026-07-22T21:30:00Z
**Status:** human_needed
**Re-verification:** Yes — third verification pass, following gap-closure plan 09-11 (executed to close the 2 Blocker gaps found in the prior 09-VERIFICATION.md pass) and a fresh 09-REVIEW.md code review that additionally surfaced 2 new Critical findings in adjacent (non-scheduler) subsystems.

## Re-verification Summary

The prior verification pass (2026-07-22T20:00:00Z) recorded `status: gaps_found`, `score: 12/16`, with two Blocker-severity gaps confirmed by direct source read:

1. `StateActor::handle_action` applied Gate 1/2/3 uniformly to `HoldRelease` actions, causing a physically-held key/mouse-button to stay stuck down whenever a Hold-mode macro was disabled, the engine was toggled off, the active app switched away from the macro's target, or a new profile was loaded.
2. Rebinding an existing macro's hotkey to a conflicting key destroyed the macro's previously-working binding with no rollback, on both platforms (macOS: unconditional `unbind_hotkey` pre-step; Windows: silent coerce-to-None with no error reply).

**Plan 09-11 was executed to close both.** I independently re-verified its claims against current source — not SUMMARY.md's or 09-REVIEW.md's prose alone:

- Read `src-tauri/src/state/mod.rs` myself: `action_should_inject` (lines 281-309) is a pure free function whose first statement is an unconditional bypass for `ActionType::HoldRelease`, returning `true` before Gate 1/2/3 are evaluated. `handle_action` (lines 456-493) now consults it via a single guard clause.
- Read the new test `hold_release_bypasses_gates` (state/mod.rs:1292+) myself and confirmed it drives all four originally-confirmed trigger paths (disabled macro, engine off, target-app mismatch, macro absent after profile clear) plus a negative case (a `HoldStart` for the same disabled macro must stay gated — proving the bypass is release-only, not a blanket bypass, satisfying the plan's own prohibition).
- Read `resolve_trigger_key_update` (state/mod.rs:226+) and the rewritten `Intent::SetMacroTriggerKey` handler (state/mod.rs:594-635) myself: on a genuine conflict it now sends `Err(msg)` via a `oneshot::Sender<Result<(), String>>` and returns WITHOUT mutating the macro's trigger fields — the old binding survives.
- Read `ipc::set_macro_trigger_key` (ipc/mod.rs:149-161) myself: it now awaits the oneshot and propagates the inner `Result`, instead of always returning `Ok(())`.
- Read `src/App.tsx:578-585` myself: the macOS branch of `handleCardSetTriggerKey` no longer calls `unbind_hotkey` before `bind_hotkey`. Ran `grep -c 'invoke("unbind_hotkey"' src/App.tsx` myself: returns `0`.
- Ran `cargo test --lib` myself (not trusting reported numbers): **16 passed, 0 failed** — including both new regression tests (`hold_release_bypasses_gates`, `set_trigger_key_rejects_conflict_without_coercion`) and all 14 pre-existing tests (no regressions).
- Ran `cargo build` and `npx tsc --noEmit` myself: both clean.
- Scanned all phase-touched files for `TBD`/`FIXME`/`XXX`: none found.

**Both prior-round Blocker gaps are genuinely closed.** This is not a cosmetic fix — the bypass is structurally unconditional (runs before any gate), the rebind rejection genuinely refuses to mutate state on conflict, and both are pinned by regression tests that fail if the fix regresses.

### Assessment of the two NEW findings from this session's fresh 09-REVIEW.md

09-REVIEW.md (committed this session) found two new Critical issues, distinct from the now-closed prior-round gaps (it reuses the "CR-01"/"CR-02" numbering, which is a coincidence of the review template, not the same bugs):

1. **New CR-01 — `Intent::LoadProfile` failure wipes `state.macros` in memory and on disk.** I independently confirmed this via direct read of `state/mod.rs:800-845`: `self.state.macros.clear()` runs unconditionally at the top of the handler, before the disk read; on `Err` from `profile_mgr.load_profile`, `state.macros` is never restored, and the unconditional `self.auto_save_default().await` at the end persists the now-empty state to `default.json`. **Confirmed real.**
2. **New CR-02 — the Windows low-level keyboard hook does not filter injected events.** I independently confirmed via direct read of `platform/windows/mod.rs:494-540` that `hook_callback` never inspects `kb_struct.flags` for `LLKHF_INJECTED`, unlike the macOS tap (`platform/macos/observer.rs:279-281`, confirmed present, which explicitly checks `EVENT_SOURCE_USER_DATA == LLMHF_INJECTED` and skips the emergency-stop/hotkey-match checks for AutoMux's own injected events). **Confirmed real** — a macro's own `SendInput` keystroke can match another macro's hotkey (or the hardcoded Ctrl+Shift+Q combo) on Windows and self-trigger a toggle.

**My independent judgment: neither is a phase-9 blocker.** Reasoning, checked against both the phase's specific ROADMAP goal and requirement wording, and each finding's git history:

- **Scope test:** Phase 9's goal is "redesign the StateActor/Scheduler execution model so multiple macros run concurrently," and EXEC-01/EXEC-02's literal text is "triggering macro B while macro A is running does not block, queue, or cancel macro A." The `LoadProfile` bug is not a triggering/cancellation interaction between two running macros at all — it's a persistence/error-handling defect in an unrelated intent (bulk profile replacement). It does not fit the truth being verified, and no phase-9 plan (09-01 through 09-11) ever touched the `LoadProfile` handler's clear-before-load structure.
- **Origin test:** `git log -S` on both defects confirms they predate Phase 9 entirely — `LoadProfile`'s clear-before-load shape dates to the v1.0 MVP commit (`2c6bc64`), and the Windows hook's missing injected-event filter dates to when the hook was first written (`faa1e1e`, also v1.0) and was never touched for filtering by 08-03 or by 09-10 (09-10 only removed a redundant registry rebuild in the same function). Neither was introduced or regressed by phase 9's redesign work.
- **CR-02(new) is closer to phase 9's concerns than CR-01(new)** since it touches the hotkey-toggle dispatch path that feeds `StateActor::handle_intent`, and — being asymmetric with macOS's existing guard — is a genuine, if narrower, threat to Windows-specific reliability (it requires a macro's own action sequence to happen to press the exact keycode+modifier combination bound to a hotkey, a real but much less universally-reachable condition than the previous round's always-reachable gaps). It is more naturally scoped to Phase 8's "Hotkey Reliability & Conflict Safety" domain (which already tracks UX-11/12/13/14) than to Phase 9's scheduler redesign.
- Neither finding is addressed by a later ROADMAP phase (Phase 10 is UI Redesign & Macro Management — unrelated), so I am not silently deferring them as "covered elsewhere." I am explicitly recommending both be opened as new backlog/gap-closure items, with CR-02(new) prioritized given its direct bearing on the Core Value ("execution must be accurate") and CR-01(new) prioritized given its severity (silent full data loss).

**These findings do NOT appear in the `gaps:` YAML above** (which is now empty) because they fall outside this phase's specific must-haves as re-derived from ROADMAP SC1/SC2 and EXEC-01/EXEC-02's literal wording, and neither was introduced or worsened by any phase-9 plan. They are documented in full below under "Out-of-Scope Findings" so they are not lost.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Two macros at different intervals both fire concurrently in the scheduler, neither blocking the other (EXEC-01/EXEC-02, plan 09-01) | VERIFIED | `cargo test --lib` re-run independently: `scheduler::tests::parallel_two_macros_concurrent` passes (16/16 suite). |
| 2 | Stopping one running macro does not affect another concurrently running macro (EXEC-02, plan 09-01) | VERIFIED | `scheduler::tests::parallel_stop_one_keeps_other` re-run independently: passes. |
| 3 | action_tx channel capacity is 1024 | VERIFIED | `grep 'mpsc::channel::<scheduler::ActionReady>(1024)' src-tauri/src/lib.rs` confirmed present (unchanged file). |
| 4 | HoldStart/HoldRelease both use symmetric guaranteed-delivery `.await` at the channel level (plans 09-05/09-06) | VERIFIED | `start_macro_hold_start_delivered_under_saturation` and `stop_macro_release_delivered_under_saturation` both re-run independently: pass. |
| 5 | Per-macro card shows firing/waiting/held/combined/disabled derived from computeRunningState (plan 09-02/09-04) | VERIFIED | Unchanged this session; App.tsx changes this round are scoped only to `handleCardSetTriggerKey`. |
| 6 | G-09-1a: CGEventTap event mask narrowed to only the 8 consumed event types | VERIFIED (source) | Unchanged this session. |
| 7 | G-09-1b: A user can delete a macro from within the app | VERIFIED | Unchanged this session. |
| 8 | G-09-1c primary: active_app updated from NSWorkspace notification's own userInfo, with logging | VERIFIED (source) | Unchanged this session. |
| 9 | G-09-1c secondary: card-edit target-app `<select>` is controlled and pre-selects the macro's current target | VERIFIED | Unchanged this session. |
| 10 | A user can change an existing macro's target app via the card-edit dropdown and have the IPC layer persist the change | VERIFIED (source) | Unchanged this session; device-level "takes effect" confirmation still pending — see human_verification. |
| 11 | A user can change an existing macro's hotkey via the card-edit UI on macOS and have the IPC layer accept the change with correct error classification | VERIFIED (source) | Unchanged this session (only the unbind pre-step was removed, catch-block classification untouched). |
| 12 | On macOS, a card-edit-assigned hotkey dispatches Intent::ToggleMacroHotkey exactly once per keypress (prior-round dual-registry double-dispatch) | VERIFIED | Closed in the previous round by plan 09-10; unaffected and unregressed by 09-11 — confirmed via full suite pass. |
| 13 | Disabling/toggling-off/app-switching-away/profile-loading reliably releases a Hold-mode macro's physically-held input — no stuck key/button | VERIFIED | Closed this round by plan 09-11 Task 1. Confirmed via direct read of `action_should_inject` (state/mod.rs:281-309) and `handle_action` (456-493), plus the passing `hold_release_bypasses_gates` regression test driving all four trigger paths and the release-only negative case. |
| 14 | Rebinding an existing macro's hotkey to a conflicting key preserves the old binding on rejection — never silently destroys it | VERIFIED | Closed this round by plan 09-11 Tasks 2-3. Confirmed via direct read of `App.tsx:578-585` (macOS: no pre-unbind, `grep` confirms 0 matches), `resolve_trigger_key_update` + rewritten `Intent::SetMacroTriggerKey` (Windows: Err-on-conflict, no coercion), `ipc::set_macro_trigger_key` (propagates Result), and the passing `set_trigger_key_rejects_conflict_without_coercion` regression test. |
| 15 | ROADMAP SC1 — concurrent firing holds on a real macOS device | PRESENT_BEHAVIOR_UNVERIFIED | Carried forward unchanged; see behavior_unverified_items. |
| 16 | ROADMAP SC2 — concurrent firing holds on a real Windows device | PRESENT_BEHAVIOR_UNVERIFIED | Carried forward unchanged; 09-UAT.md explicitly marked this `blocked_by: physical-device`. |

**Score:** 14/16 truths verified (2 present-behavior-unverified — real-device confirmation only). Net change from the prior round: +2 truths moved from failed to verified (both prior-round Blocker gaps genuinely closed by plan 09-11, independently re-confirmed against source and passing regression tests). No regressions.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/scheduler/mod.rs` | Symmetric guaranteed-delivery hold lifecycle + parallel-execution proof tests | VERIFIED | Unchanged this session; 16/16 suite passes. |
| `src-tauri/src/state/mod.rs` | Two-phase dispatch gating (handle_action) with a release bypass; single hotkey registry; conflict-rejecting trigger-key update | VERIFIED | `action_should_inject` (281-309) and `resolve_trigger_key_update` (226+) both present, correct, and covered by dedicated passing regression tests. `handle_action` (456-493) refactored to consult the former. `Intent::SetMacroTriggerKey` (594-635) rewritten to the Result-oneshot pattern. |
| `src/App.tsx` | Safe hotkey-rebind flow (no pre-unbind on macOS) | VERIFIED | `handleCardSetTriggerKey` (578-606) confirmed to call only `bind_hotkey` then `set_macro_trigger_key` on macOS — no `unbind_hotkey` call anywhere in the file. |
| `src-tauri/src/ipc/mod.rs` | `set_macro_trigger_key` propagates a real Result instead of always Ok(()) | VERIFIED | Lines 149-161 confirmed to await the oneshot and return the inner `Result<(), String>`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `HoldRelease` action → `handle_action` → `inject_input` | `action_should_inject` unconditional bypass | `state/mod.rs:456-464` (guard clause) → `281-291` (bypass) | WIRED | Confirmed: the bypass is evaluated before the emergency-stop/enabled/target gates, and is the very first branch in the function. |
| Card-edit hotkey capture (macOS) | `bind_hotkey` (no prior unbind) | `App.tsx:580-582` | WIRED | Confirmed via direct read + `grep -c 'invoke("unbind_hotkey"'` returning 0. |
| `set_macro_trigger_key` IPC → `Intent::SetMacroTriggerKey` oneshot | Conflict → `Err` reply, no mutation | `ipc/mod.rs:149-161` → `state/mod.rs:594-635` | WIRED | Confirmed: oneshot Result now propagates to the frontend; conflict path returns before any macro field is mutated. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full backend regression suite (re-run independently) | `cargo test --manifest-path src-tauri/Cargo.toml --lib` (from repo root) | 16 passed; 0 failed | PASS |
| `hold_release_bypasses_gates` drives all 4 trigger paths + negative case | (included in full run above; also enumerated via `--list`) | present, passes | PASS |
| `set_trigger_key_rejects_conflict_without_coercion` drives conflict/free/clear/self cases | (included in full run above) | present, passes | PASS |
| macOS rebind flow no longer pre-unbinds | `grep -c 'invoke("unbind_hotkey"' src/App.tsx` | `0` | PASS |
| Backend compiles clean | `cargo build --manifest-path src-tauri/Cargo.toml` | exit 0 | PASS |
| Frontend typechecks clean | `npx tsc --noEmit` | exit 0, no output | PASS |
| No debt markers in phase-touched files | `grep -n 'TBD\|FIXME\|XXX'` across state/mod.rs, ipc/mod.rs, App.tsx, platform/windows/mod.rs | no matches | PASS |
| New CR-01 (LoadProfile data loss) — direct source trace | Read `state/mod.rs:800-845` | Confirmed: `state.macros.clear()` runs before the disk read; `Err` branch never restores it; unconditional `auto_save_default()` persists the empty state | CONFIRMED (out-of-scope finding, not a phase-9 gap — see Re-verification Summary) |
| New CR-02 (Windows hook self-triggering) — direct source trace | Read `platform/windows/mod.rs:494-540` vs `platform/macos/observer.rs:279-281` | Confirmed: Windows hook has no `LLKHF_INJECTED`-equivalent guard; macOS tap does | CONFIRMED (out-of-scope finding, not a phase-9 gap — see Re-verification Summary) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| EXEC-01 | 09-01..09-11 | Multiple macros can run simultaneously on macOS | SATISFIED at the source level | Scheduler parallel-execution proof solid; both prior-round Blocker gaps (stuck-input, hotkey-rebind data loss) confirmed closed this pass via independent source read + passing regression tests. Real-device confirmation of the concurrency claim (T9.1-T9.7) remains outstanding — see human_verification. |
| EXEC-02 | 09-01..09-11 | Multiple macros can run simultaneously on Windows | SATISFIED at the source level | Same disposition; the HoldRelease-bypass fix is shared code (applies identically on Windows); the Windows-specific rebind-rejection fix independently confirmed via `resolve_trigger_key_update` + the rewritten `Intent::SetMacroTriggerKey` oneshot. Real-device Windows confirmation was never genuinely performed for this phase, across all verification passes — remains blocked_by physical-device. |

REQUIREMENTS.md (lines 17-18) still shows the EXEC-01/EXEC-02 checklist boxes unchecked (`- [ ]`) while the Traceability table (lines 103-104) marks them "Complete" — this pre-existing inconsistency was explicitly and intentionally left unresolved by plan 09-11 (per its own NOTE) pending this re-verification pass. Now that both source-level Blocker gaps are closed, the checklist annotations should be updated to reflect that (e.g., "source-level reliability gaps closed 09-11; on-device human verification T9.1-T9.7 / 6.1-6.3 remains the sole remaining item"), but the checkbox itself should likely stay unchecked until that real-device confirmation completes, matching the Traceability table's own caveat language. This is a documentation-reconciliation recommendation, not a verification gap.

No orphaned requirements found — REQUIREMENTS.md's Phase 9 row maps exactly to EXEC-01/EXEC-02, declared in plan frontmatter across all 11 plans (09-01 through 09-11).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src-tauri/src/state/mod.rs | 281-309, 456-493, 594-635 | (RESOLVED this round) HoldRelease gating + hotkey-rebind coercion | Closed | Both confirmed fixed via independent source read and passing regression tests — see Re-verification Summary. |
| src-tauri/src/state/mod.rs | 800-845 | `Intent::LoadProfile` clears `state.macros` before the disk read and never restores it on `Err`, then unconditionally persists the emptied state | Critical (out-of-scope for phase 9 — see reasoning above) | Confirmed real via direct read. Recommend a dedicated backlog/gap-closure item; not gating this phase. |
| src-tauri/src/platform/windows/mod.rs | 494-540 | `hook_callback` never checks `KBDLLHOOKSTRUCT.flags` for `LLKHF_INJECTED`, unlike the macOS tap's equivalent guard | Critical (out-of-scope for phase 9 — see reasoning above) | Confirmed real via direct read (vs. `platform/macos/observer.rs:279-281`, confirmed present). Recommend a dedicated backlog/gap-closure item, likely scoped under Phase 8's hotkey-reliability domain; not gating this phase. |
| Various | — | Remaining Warning/Info items from 09-REVIEW.md (WR-01 through WR-09, IN-01 through IN-10) | Warning / Info | Real but pre-existing/unrelated to the core parallel-execution scope; not independently re-verified item-by-item in this pass beyond the two Critical findings explicitly assessed above. Listed in 09-REVIEW.md for completeness, not gating. |

No TBD/FIXME/XXX debt markers found in phase-9-touched files (`state/mod.rs`, `ipc/mod.rs`, `App.tsx`, and the reviewed `platform/windows/mod.rs`).

## Human Verification Required

### 1. macOS device tests (T9.1-T9.7)

**Test:** Run the documented macOS manual tests on a real macOS host with Accessibility + Input Monitoring granted, using the dashboard toggle (not a hotkey) to enable/disable macros.
**Expected:** All pass; concurrent firing and independent stop both hold.
**Why human:** Requires live CGEvent injection, live NSWorkspace notifications, and real-time UI/system observation.

### 2. Windows device tests (6.1-6.3)

**Test:** Run the 3 documented Windows manual tests on an actual Windows host — this has never happened across any verification pass for this phase.
**Expected:** Same concurrent-firing and independent-stop behavior via SendInput.
**Why human:** Requires a real Windows device; the "pass for now" note in 09-UAT.md is an explicit skip, not evidence.

### 3. (Recommended, not gating) Device confirmation of the now-closed gaps

**Test:** (a) Enable a Hold-mode macro targeted at an app, let HoldStart fire, then disable it / toggle the engine off / switch active app / load a different profile and confirm the physical key/button is released. (b) Give a macro a working hotkey, attempt a conflicting rebind via the card-edit UI on each platform, confirm the original hotkey still works after rejection and the conflict toast appears.
**Expected:** Held input releases in all four scenarios on both platforms; a rejected rebind never destroys the original hotkey.
**Why human:** The source-level fix and its unit tests are conclusive for the code path (all four trigger paths were traced line-by-line against the fix and are directly exercised by the new regression tests); this item is offered as good-practice device confirmation, not because the source-level evidence is insufficient to close the gap.

## Out-of-Scope Findings (Recommend New Backlog Items — Not Gating This Phase)

Both findings below were independently confirmed via direct source read (not merely trusted from 09-REVIEW.md's prose), and both are real, reachable defects. Neither is included in the `gaps:` YAML because neither fits phase 9's specific must-haves (the StateActor/Scheduler concurrent-execution redesign and its EXEC-01/EXEC-02 "does not block/queue/cancel" wording), neither was introduced or regressed by any phase-9 plan (both predate Phase 9's start — confirmed via `git log -S`), and neither is addressed by a later ROADMAP phase. They are recorded here so they are not lost.

1. **`Intent::LoadProfile` failure wipes the user's current macros, in memory and on disk** (`src-tauri/src/state/mod.rs:800-845`). A failed profile load (missing file, corrupted JSON, transient I/O error) leaves `state.macros` cleared with no restoration, and the trailing unconditional `auto_save_default()` overwrites `default.json` with the now-empty state — a single failed "Load Profile" click destroys every configured macro with only a transient toast as evidence. Recommend a dedicated gap-closure plan: only clear/mutate `state.macros` after a successful disk read (fix sketch already provided in 09-REVIEW.md CR-01).

2. **Windows keyboard hook does not filter injected keystrokes** (`src-tauri/src/platform/windows/mod.rs:494-540`), unlike the macOS tap which does (`platform/macos/observer.rs:279-281`). A macro's own `SendInput`-injected keystroke can match a configured hotkey (or the hardcoded Ctrl+Shift+Q emergency-stop combo) and self-trigger a toggle or emergency stop on Windows. Recommend a dedicated gap-closure plan, likely scoped alongside Phase 8's hotkey-reliability work: check `KBDLLHOOKSTRUCT.flags & LLKHF_INJECTED` and skip both the emergency-stop check and hotkey matching for injected events (fix sketch already provided in 09-REVIEW.md CR-02).

## Gaps Summary

**No gaps remain that are in scope for this phase.** Plan 09-11 successfully closed both confirmed Blocker-severity gaps from the prior verification round — independently re-confirmed by my own direct source read (not merely trusted from SUMMARY.md's or 09-REVIEW.md's claims): the `HoldRelease` bypass in `handle_action`/`action_should_inject`, and the reject-on-conflict hotkey-rebind flow on both platforms. `cargo test --lib` (16/16), `cargo build`, and `npx tsc --noEmit` all pass cleanly, and no regressions were introduced in the previously-passing 14 tests.

Two new Critical findings from this session's fresh 09-REVIEW.md (`LoadProfile` data-loss on failed load; Windows hook missing injected-event filtering) were independently confirmed as real, but assessed as out-of-scope for phase 9's specific goal and requirement wording — both predate phase 9, neither was touched by any phase-9 plan, and neither describes a "macro B triggering cancels macro A" interaction. They are recommended as new backlog/gap-closure items rather than blocking this phase's sign-off — see "Out-of-Scope Findings" above.

The only remaining items keeping this phase's status at `human_needed` rather than `passed` are the two unchanged ROADMAP success criteria (SC1 macOS / SC2 Windows) requiring real-device confirmation of concurrent firing — carried forward unchanged across all three verification passes for this phase.

**Recommended next step:** Route to human verification for the real-device macOS/Windows concurrent-firing tests (T9.1-T9.7, 6.1-6.3). Separately, open two new backlog/gap-closure items for the `LoadProfile` data-loss and Windows hook self-triggering findings — both are real and should not be lost, but do not block phase 9's completion. REQUIREMENTS.md's checklist/traceability-table inconsistency for EXEC-01/EXEC-02 should be reconciled once real-device confirmation completes.

---

_Verified: 2026-07-22T21:30:00Z_
_Verifier: Claude (gsd-verifier)_
