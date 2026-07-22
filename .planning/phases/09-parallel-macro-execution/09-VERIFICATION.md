---
phase: 09-parallel-macro-execution
verified: 2026-07-22T20:00:00Z
status: gaps_found
score: 12/16 must-haves verified
behavior_unverified: 2
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 11/13
  gaps_closed:
    - "Prior gap (CR-01, hotkey double-dispatch — a keypress dispatched Intent::ToggleMacroHotkey twice via two independently-populated registries, HOTKEY_BINDINGS + MACRO_TRIGGER_KEYS, silently self-cancelling the toggle): CLOSED. Independently re-confirmed by direct read of src-tauri/src/state/mod.rs — reevaluate_all_macros (lines 755-796) now refreshes HOTKEY_BINDINGS unconditionally at the top of the method, before the engine-active early-return, and the second-registry population is gone. `grep -riq 'macro_trigger_keys' src-tauri/src; echo $?` returns 1 (zero matches anywhere in the crate, code and comments). `grep -n 'ToggleMacroHotkey' src-tauri/src/platform/macos/observer.rs` and the Windows equivalent each show exactly one dispatch site. `cargo test --lib` (14/14, including the new `hotkey_registry_has_single_binding_per_trigger_macro` regression test) re-run independently and passes. 09-REVIEW.md (this session's fresh code review) independently corroborates the same conclusion."
  gaps_remaining:
    - "ROADMAP SC1 (macOS real-device concurrent-firing confirmation) — unchanged, still human_needed."
    - "ROADMAP SC2 (Windows real-device concurrent-firing confirmation) — unchanged, still human_needed, explicitly blocked_by physical-device."
  regressions: []
gaps:
  - truth: "Disabling a Hold-mode macro, toggling the engine off, switching the active app away from a macro's target, or loading a new profile reliably releases any physically-held input (key/mouse-button) from that macro — no stuck input is left behind"
    status: failed
    reason: "NEW finding this session (09-REVIEW.md CR-01, review's own numbering — distinct from the prior verification round's now-fixed CR-01 double-dispatch bug). Independently confirmed by my own direct read of src-tauri/src/state/mod.rs, not merely trusted from the review's prose. StateActor::handle_action (lines 373-413) applies the SAME three gates (engine active, macro enabled, target-app match) to every ActionType including HoldRelease (lines 409-411) — there is no early-return/bypass for releases. Traced all four claimed trigger paths against current source and confirmed each: (1) Intent::SetMacroEnabled(id, false) (lines 491-500) sets `mac.enabled = false` at line 492-494, BEFORE calling `reevaluate_all_macros().await` at line 495 (which sends the StopMacro that produces the guaranteed-delivery HoldRelease) — so when that HoldRelease reaches handle_action, Gate 2's `Some(m) if m.enabled => m, _ => return` (line 383) discards it because enabled is already false. (2) Intent::ToggleEngineHotkey (lines 661-671) sets `self.state.engine_active = !self.state.engine_active` at line 662 BEFORE sending SchedulerIntent::StopAll at lines 664-667 when turning off — Gate 1 (`!self.state.engine_active`, line 377) then discards every HoldRelease the guaranteed-delivery StopAll release loop produces. (3) Intent::ActiveAppChanged (lines 643-649) sets `self.state.active_app = app` at line 644 BEFORE `reevaluate_all_macros().await` at line 645 — Gate 3 (lines 387-394) discards the HoldRelease because active_app no longer matches target_app by the time it arrives. (4) Intent::LoadProfile (lines 706-751) calls `self.state.macros.clear()` at line 708 BEFORE sending StopAll at lines 709-712 — Gate 2 discards the HoldRelease because the macro no longer exists in state.macros at all. In every path the physical 'down' injected by the earlier HoldStart is never followed by a corresponding 'up' — the only recovery is the hardcoded Emergency Stop hotkey (`flush_held_inputs`, line 632), which a normal disable/engine-toggle/app-switch/profile-load does not invoke. This directly undermines the guaranteed-delivery CR-01/CR-02 scheduler hardening from plans 09-05/09-06 documented in scheduler/mod.rs — channel delivery of the release is guaranteed, but gate-passage (and therefore actual injection) of that release is not. This is a Blocker: it is a real, reachable stuck-input bug (most likely trigger: alt-tabbing away from a target app while a Hold-mode macro is engaged — exactly the AFK-farm scenario this phase's own stress tests exercise) that directly contradicts the project's stated Core Value ('a macro that was set up must fire reliably') and the phase goal's own language about triggering/cancelling behavior around the two-phase dispatch mechanism this phase owns."
    artifacts:
      - path: "src-tauri/src/state/mod.rs"
        issue: "handle_action (lines 373-413) applies Gate 1/2/3 uniformly to ActionType::HoldRelease; no bypass exists for release-type actions, so a HoldRelease arriving after the state mutation that triggered the stop is silently discarded and the physical input stays down."
    missing:
      - "Add an unconditional bypass at the top of handle_action: if the action is ActionType::HoldRelease, inject the release immediately and return, before any of the three gates run (a release is a cleanup event, not a new-input request, and must never be blocked by gates designed to prevent unwanted new input)."
      - "A regression test that drives one of the four confirmed trigger paths (e.g. disable a Hold-mode macro while its HoldStart has already fired) and asserts the corresponding release is actually injected, not just that the channel delivers the message."
  - truth: "Rebinding an existing macro's hotkey to a key already used by another macro either succeeds or leaves the macro's previously-working binding intact — it never destroys a working binding on a rejected rebind"
    status: failed
    reason: "NEW finding this session (09-REVIEW.md CR-02). Independently confirmed by direct read of src/App.tsx:572-601 and src-tauri/src/state/mod.rs:514-541 (Intent::SetMacroTriggerKey) and ipc/mod.rs:145-156 (set_macro_trigger_key command). macOS: handleCardSetTriggerKey (App.tsx:574-577) calls `unbind_hotkey` unconditionally FIRST (clearing the macro's current trigger_key/trigger_modifiers via Intent::UnbindHotkey, state/mod.rs:599-621), THEN calls `bind_hotkey` with the new key. If the new key conflicts with another macro, Intent::BindHotkey's conflict pre-check (state/mod.rs:550-569) sends Err(msg) WITHOUT mutating state — but by then the preceding unbind_hotkey call has already cleared the macro's old binding, so the macro is left with no hotkey at all. The user sees only a 'Hotkey already bound' toast; nothing indicates their previously-working binding was just deleted. Windows: the card path calls `set_macro_trigger_key` directly (App.tsx:579), which maps to Intent::SetMacroTriggerKey (state/mod.rs:514-541). On conflict this handler silently coerces new_key/new_mods to None/0 (lines 528-529) and applies that unconditionally (lines 532-535) — there is no Result-carrying oneshot for this intent (unlike Intent::BindHotkey), so ipc::set_macro_trigger_key (ipc/mod.rs:145-156) always returns Ok(()) regardless of whether a conflict occurred. The net effect on Windows is identical data loss to macOS, but with zero user feedback at all — not even a toast. Confirmed exactly as the review describes; this is a Blocker because it means the card-edit hotkey-rebind flow — the primary UI path for configuring a macro's trigger, which this phase's own EXEC-01/EXEC-02 reliability language depends on — can silently strand a previously-working macro with no way to trigger it, discoverable only by the user noticing their hotkey stopped working."
    artifacts:
      - path: "src/App.tsx"
        issue: "handleCardSetTriggerKey (lines 572-601) calls unbind_hotkey unconditionally before bind_hotkey on macOS, with no rollback if the subsequent bind_hotkey rejects due to conflict."
      - path: "src-tauri/src/state/mod.rs"
        issue: "Intent::SetMacroTriggerKey (lines 514-541) silently coerces to None/0 on conflict instead of rejecting via an Err oneshot reply, so the Windows card-edit path always returns Ok(()) even when the rebind was actually rejected and the old binding destroyed."
    missing:
      - "macOS: drop the leading unbind_hotkey call in handleCardSetTriggerKey and rely on bind_hotkey alone — Intent::BindHotkey already overwrites trigger_key/trigger_modifiers unconditionally on success and returns Err untouched on conflict, so the old binding survives a failed rebind without any extra round trip."
      - "Windows: give Intent::SetMacroTriggerKey the same Result-carrying oneshot pattern Intent::BindHotkey already uses, so a conflict is rejected (old binding preserved) and surfaced to the user via the same conflict toast, instead of being silently coerced to None."
  - truth: "On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals (ROADMAP SC1)"
    status: human_needed
    reason: "Carried forward unchanged from the prior verification pass — not part of this round's re-verified scope. See behavior_unverified_items."
  - truth: "On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A (ROADMAP SC2)"
    status: human_needed
    reason: "Carried forward unchanged from the prior verification pass — not part of this round's re-verified scope. See behavior_unverified_items."
deferred: []
behavior_unverified_items:
  - truth: "On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals (ROADMAP SC1)"
    test: "On a real macOS host with Accessibility + Input Monitoring granted: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A via the dashboard toggle (not a hotkey), wait ~1s, enable B the same way; observe both macro cards show a pulsing firing dot simultaneously and neither's click rate changes when the other starts/stops"
    expected: "Both macros visibly fire concurrently at their own configured rates on real macOS input injection (CGEvent)"
    why_human: "Unchanged since the prior pass — scheduler-level unit tests (re-run and confirmed passing, 14/14) prove platform-agnostic timeline/delivery logic but not real-device CGEvent injection timing with two simultaneous macros."
  - truth: "On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A (ROADMAP SC2)"
    test: "On a real Windows host: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A, wait ~1s, enable B; observe both fire concurrently via SendInput"
    expected: "Both macros fire concurrently at their own configured rates via SendInput"
    why_human: "Unchanged since the prior pass — 09-UAT.md explicitly recorded this as blocked_by: physical-device. This remains a deliberate skip, not a passing result."
human_verification:
  - test: "Confirm CR-01 (HoldRelease gating) at the device level: enable a Hold-mode macro targeted at a specific app, let the HoldStart fire (input held down), then switch focus away from the target app (or disable the macro, or toggle the engine off, or load a different profile) and observe whether the physically-held input is released."
    expected: "The held key/mouse-button is released immediately when the macro stops for any of these reasons. If the input stays down (confirmed only recoverable via Emergency Stop), CR-01 is confirmed at the device level, matching the source-level trace in this report."
    why_human: "Requires live CGEvent/SendInput injection and observation of actual OS-level key/button state — cannot be triggered or observed through static analysis alone, though the source-level defect is already conclusively confirmed by direct code read."
  - test: "Confirm CR-02 (hotkey rebind rollback) at the device level on both macOS and Windows: give a macro a working hotkey, then attempt to rebind it via the card-edit UI to a key already used by another macro; after the rejected rebind, press the macro's ORIGINAL hotkey and see if it still works."
    expected: "The original hotkey should still toggle the macro after a rejected rebind. Per the confirmed source-level defect, it will NOT — the macro will have no working hotkey at all (macOS: user sees a conflict toast; Windows: no feedback whatsoever)."
    why_human: "Requires live UI interaction and a real CGEventTap/Win32 hook keypress to observe the end-to-end user-facing consequence, though the source-level defect is already conclusively confirmed by direct code read."
  - test: "Run T9.1-T9.7 on a real macOS host (concurrent firing, stop-one-keeps-other, same-input concurrent + conflict warning, held-not-firing indicator, Hold-under-load, responsiveness-after-stop, creation-time targeting) using the dashboard toggle (not a hotkey) to enable/disable macros."
    expected: "All tests pass; both macro cards show independent pulsing firing dots; stopping one does not affect the other; system responsiveness returns to baseline after stopping a fast-interval macro; a macro created with a target app fires only when that app is frontmost."
    why_human: "Real macOS device, live CGEvent injection, live UI observation, live NSWorkspace notifications — cannot be verified by static analysis."
  - test: "Run the 3 documented Windows manual tests (6.1-6.3) on a real Windows host — this has never actually been executed across any verification pass for this phase (09-UAT.md recorded it as blocked_by: physical-device, not as a genuine pass)."
    expected: "All 3 tests pass via SendInput injection and the Win32 hook observer."
    why_human: "Requires a real Windows device."
---

# Phase 9: Parallel Macro Execution Verification Report

**Phase Goal:** Multiple macros can run simultaneously on both macOS and Windows — triggering a second macro never blocks, queues, or cancels a running one
**Verified:** 2026-07-22T20:00:00Z
**Status:** gaps_found
**Re-verification:** Yes — fresh verification pass after gap-closure plan 09-10 (CR-01 hotkey double-dispatch consolidation) executed, and after a fresh code review (09-REVIEW.md) surfaced 2 new Critical findings not present in the prior verification round.

## Re-verification Summary

The prior verification pass (2026-07-22T18:05:00Z) recorded `status: gaps_found`, `score: 11/13`, with one Blocker-severity gap discovered during that pass itself: a dual hotkey-registry (`HOTKEY_BINDINGS` + `MACRO_TRIGGER_KEYS`) double-dispatch defect on macOS, where a single keypress fired `Intent::ToggleMacroHotkey` twice and silently self-cancelled the toggle. Plan 09-10 was executed to close it.

**I independently re-verified plan 09-10's claims against current source, not SUMMARY.md's or 09-REVIEW.md's claims alone:**

- Read `src-tauri/src/state/mod.rs` in full myself. `reevaluate_all_macros` (lines 755-796) now refreshes `HOTKEY_BINDINGS` unconditionally at the very top of the method (lines 764-772), before the emergency-stop/engine-active early-return (line 774) — exactly matching the fix's intent (a hotkey bind is now observable independent of engine state, and refreshed on every trigger_key-mutating handler that calls this method).
- Ran `grep -riq 'macro_trigger_keys' src-tauri/src; echo $?` myself: returned `1` — zero matches anywhere in the crate (code and comments), confirming the redundant registry is fully gone.
- Ran `grep -n 'ToggleMacroHotkey' src-tauri/src/platform/macos/observer.rs` and the Windows equivalent myself: exactly one dispatch site remains on each platform.
- Re-ran `cargo test --lib` myself (not trusting reported numbers): 14 passed, 0 failed — including the new `hotkey_registry_has_single_binding_per_trigger_macro` regression test, which asserts the registry-build function emits exactly one binding per trigger-key macro.
- The fresh code review (09-REVIEW.md, written this session, `status: issues_found`) independently corroborates the same conclusion in its Summary: "That specific bug is confirmed fixed here."

**The prior round's hotkey double-dispatch gap is genuinely closed.**

**However, per this task's explicit instruction, I read `src-tauri/src/state/mod.rs` myself to confirm or refute the two NEW Critical findings 09-REVIEW.md surfaced this session (that review's own CR-01 and CR-02 — distinct bugs from the now-fixed prior-round CR-01), rather than trusting the review's prose. Both are confirmed real:**

### CR-01 (review's numbering) — HoldRelease actions gated identically to new-input actions: CONFIRMED, real, Blocker

Direct read of `handle_action` (`state/mod.rs:373-413`) confirms it applies the *same* three gates (engine active, macro enabled/exists, target-app match) uniformly to every `ActionType`, including `ActionType::HoldRelease` (line 409-411) — there is no bypass for release-type actions. I traced all four claimed trigger paths against current source line-by-line and confirmed each independently:

1. **Disable a Hold-mode macro** — `Intent::SetMacroEnabled(id, false)` (lines 491-500) sets `mac.enabled = false` (line 492-494) *before* calling `reevaluate_all_macros().await` (line 495), which is what sends the `StopMacro` that produces the guaranteed-delivery `HoldRelease`. By the time that `HoldRelease` reaches `handle_action`, Gate 2 (`Some(m) if m.enabled => m, _ => return`, line 383) discards it.
2. **Toggle engine off** — `Intent::ToggleEngineHotkey` (lines 661-671) sets `self.state.engine_active = !self.state.engine_active` (line 662) *before* sending `SchedulerIntent::StopAll` (lines 664-667) when turning off. Gate 1 (`!self.state.engine_active`, line 377) discards every `HoldRelease` the `StopAll` release loop produces.
3. **Switch active app away from target** — `Intent::ActiveAppChanged` (lines 643-649) sets `self.state.active_app = app` (line 644) *before* `reevaluate_all_macros().await` (line 645). Gate 3 (lines 387-394) discards the resulting `HoldRelease` because `active_app` no longer matches `target_app` by the time it arrives — this is the most likely real-world trigger (alt-tabbing away from a game while a Hold-mode macro is engaged for an AFK farm, exactly the scenario this phase's own stress tests exercise).
4. **Load/switch profile** — `Intent::LoadProfile` (lines 706-751) calls `self.state.macros.clear()` (line 708) *before* sending `StopAll` (lines 709-712). Gate 2 discards the `HoldRelease` because the macro no longer exists in `state.macros` at all.

In every path, the physical "down" injected by the earlier `HoldStart` is never followed by a corresponding "up" — the only recovery is the hardcoded Emergency Stop hotkey (`flush_held_inputs`, line 632), which none of these four normal paths invoke. This directly undermines the guaranteed-delivery scheduler hardening from plans 09-05/09-06 (documented in `scheduler/mod.rs`): channel *delivery* of the release is guaranteed, but gate-*passage* (and therefore actual injection) of that release is not. **This is a confirmed, reachable stuck-input bug that touches the project's stated Core Value ("a macro that was set up must fire reliably") directly, in the exact two-phase dispatch mechanism this phase owns.**

### CR-02 (review's numbering) — conflicting hotkey rebind destroys the old binding with no rollback: CONFIRMED, real, Blocker

Direct read of `src/App.tsx:572-601` confirms `handleCardSetTriggerKey`'s macOS branch calls `unbind_hotkey` (line 575) *unconditionally first* — clearing the macro's current trigger key via `Intent::UnbindHotkey` (`state/mod.rs:599-621`) — and only *then* calls `bind_hotkey` (line 576) with the new key. If the new key conflicts with another macro, `Intent::BindHotkey`'s conflict pre-check (`state/mod.rs:550-569`) correctly sends `Err(msg)` without mutating state — but the preceding `unbind_hotkey` has already destroyed the old binding, so the macro ends up with **no hotkey at all**, with only a generic conflict toast shown (nothing indicates the working binding was just deleted).

Direct read of `state/mod.rs:514-541` (`Intent::SetMacroTriggerKey`, the Windows card-edit path) confirms it silently coerces `new_key`/`new_mods` to `None`/`0` on conflict (lines 528-529) and applies that unconditionally (lines 532-535) — there is no `Result`-carrying oneshot for this intent (unlike `Intent::BindHotkey`), so `ipc::set_macro_trigger_key` (`ipc/mod.rs:145-156`) always returns `Ok(())` regardless of whether a conflict actually occurred. The Windows outcome is the same data loss as macOS, but with **zero user feedback at all** — not even a toast.

**Both findings are confirmed exactly as 09-REVIEW.md describes them, independently verified via my own direct source read rather than trusted from the review's prose. Both are Blockers: they undermine the reliability of the exact mechanisms (Two-Phase Dispatch release delivery; card-edit hotkey configuration) that this phase's EXEC-01/EXEC-02 requirements and the project's Core Value depend on.**

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Two macros at different intervals both fire concurrently in the scheduler, neither blocking the other (EXEC-01/EXEC-02, plan 09-01) | VERIFIED | `cargo test --lib` re-run independently: `scheduler::tests::parallel_two_macros_concurrent` passes (14/14 suite). |
| 2 | Stopping one running macro does not affect another concurrently running macro (EXEC-02, plan 09-01) | VERIFIED | `scheduler::tests::parallel_stop_one_keeps_other` re-run independently: passes. |
| 3 | action_tx channel capacity is 1024 | VERIFIED | `grep 'mpsc::channel::<scheduler::ActionReady>(1024)' src-tauri/src/lib.rs` confirmed present (unchanged file). |
| 4 | HoldStart/HoldRelease both use symmetric guaranteed-delivery `.await` at the channel level (plans 09-05/09-06) | VERIFIED | `start_macro_hold_start_delivered_under_saturation` and `stop_macro_release_delivered_under_saturation` both re-run independently: pass. NOTE: this verifies channel *delivery*, not gate-*passage* at `handle_action` — see new gap below, which is a separate mechanism. |
| 5 | Per-macro card shows firing/waiting/held/combined/disabled derived from computeRunningState (plan 09-02/09-04) | VERIFIED | Unchanged this session; not touched by 09-10 (files modified: state/mod.rs, macos/observer.rs, windows/mod.rs, ipc/mod.rs only). |
| 6 | G-09-1a: CGEventTap event mask narrowed to only the 8 consumed event types | VERIFIED (source) | Unchanged this session. |
| 7 | G-09-1b: A user can delete a macro from within the app | VERIFIED | Unchanged this session. |
| 8 | G-09-1c primary: active_app updated from NSWorkspace notification's own userInfo, with logging | VERIFIED (source) | Unchanged this session. |
| 9 | G-09-1c secondary: card-edit target-app `<select>` is controlled and pre-selects the macro's current target | VERIFIED | Unchanged this session. |
| 10 | A user can change an existing macro's target app via the card-edit dropdown and have the IPC layer persist the change | VERIFIED (source) | `rename_all = "snake_case"` confirmed present at `ipc/mod.rs:42` above `set_macro_target_app`. Device-level "takes effect" confirmation still pending — see human_verification. |
| 11 | A user can change an existing macro's hotkey via the card-edit UI on macOS and have the IPC layer accept the change with correct error classification | VERIFIED (source) | `rename_all = "snake_case"` confirmed present above `bind_hotkey`/`unbind_hotkey` (`ipc/mod.rs:80-81, 99-100`). Catch-block mislabeling fixed (`App.tsx:583-600`). Device-level confirmation still pending. |
| 12 | On macOS, a card-edit-assigned hotkey dispatches Intent::ToggleMacroHotkey exactly once per keypress (prior round's CR-01, dual-registry double-dispatch) | VERIFIED | CLOSED this round by plan 09-10 — independently re-confirmed via direct source read (see Re-verification Summary above): single HOTKEY_BINDINGS registry, MACRO_TRIGGER_KEYS fully absent (`grep` returns 0 matches), single dispatch site per platform, new regression test passes. |
| 13 | Disabling/toggling-off/app-switching-away/profile-loading reliably releases a Hold-mode macro's physically-held input — no stuck key/button (NEW this round — 09-REVIEW.md CR-01) | FAILED | Confirmed via direct read of `state/mod.rs:373-413` (handle_action) and all four claimed trigger sites — see Re-verification Summary and Gaps below. Real, reachable, Blocker-severity. |
| 14 | Rebinding an existing macro's hotkey to a conflicting key preserves the old binding on rejection — never silently destroys it (NEW this round — 09-REVIEW.md CR-02) | FAILED | Confirmed via direct read of `App.tsx:572-601`, `state/mod.rs:514-541`, `ipc/mod.rs:145-156` — see Re-verification Summary and Gaps below. Real, reachable, Blocker-severity. |
| 15 | ROADMAP SC1 — concurrent firing holds on a real macOS device | PRESENT_BEHAVIOR_UNVERIFIED | Carried forward unchanged; see behavior_unverified_items. |
| 16 | ROADMAP SC2 — concurrent firing holds on a real Windows device | PRESENT_BEHAVIOR_UNVERIFIED | Carried forward unchanged; 09-UAT.md explicitly marked this `blocked_by: physical-device`. |

**Score:** 12/16 truths verified (2 present-behavior-unverified, 2 newly-confirmed failed). Net change from the prior round: +1 truth moved from failed to verified (prior CR-01 closed by plan 09-10), +2 new failed truths (this session's fresh code review, independently confirmed against source).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/scheduler/mod.rs` | Symmetric guaranteed-delivery hold lifecycle + parallel-execution proof tests | VERIFIED | Unchanged this session; 14/14 tests pass (channel-delivery guarantee holds; does not by itself guarantee gate-passage at handle_action — see gap). |
| `src-tauri/src/state/mod.rs` | Two-phase dispatch gating (handle_action); single hotkey registry (reevaluate_all_macros) | VERIFIED (single-registry consolidation) / NEW GAP (HoldRelease gating) | The 09-10 hotkey-registry consolidation is correctly and completely implemented. Independently of that fix, handle_action's uniform gating of HoldRelease actions is a confirmed, pre-existing (not introduced by any Phase 9 plan) defect — see gap above. |
| `src/App.tsx` | Macro-card delete control, controlled target-app select, corrected catch-block error classification, safe hotkey-rebind flow | VERIFIED (delete, select, catch-block) / NEW GAP (rebind flow) | Delete, target-select, and catch-block fixes all confirmed correct and unaffected by the new findings. The unbind-before-bind ordering in handleCardSetTriggerKey (lines 574-577) is a confirmed data-loss defect on conflict — see gap above. |
| `src-tauri/src/ipc/mod.rs` | Commands whose frontend-facing argument names correctly round-trip; accurate doc comments | VERIFIED | All 5 previously-affected commands carry `rename_all = "snake_case"`; the `set_macro_trigger_key` doc comment now correctly describes the single-registry design. The command's silent-coerce-on-conflict behavior (no Result-carrying reply) is the CR-02 gap, not a documentation issue. |
| `src-tauri/src/platform/macos/observer.rs`, `src-tauri/src/platform/windows/mod.rs` | Single hotkey dispatch path per platform | VERIFIED | Redundant MACRO_TRIGGER_KEYS registry and its lookup fully removed from both files; exactly one ToggleMacroHotkey dispatch site remains per platform. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| Scheduler `StopMacro`/`StopAll` → guaranteed-delivery `HoldRelease` | `action_tx.send(...).await` | Channel-level `.await` send, never `try_send`, for release paths (plans 09-05/09-06) | WIRED (channel delivery only) | Channel delivery is confirmed guaranteed and unaffected. |
| `HoldRelease` action → `handle_action` gates → `inject_input` | `Gate 1 (engine_active) / Gate 2 (macro.enabled) / Gate 3 (target match)` | `state/mod.rs:373-413`, applied uniformly to all `ActionType` variants | NOT_WIRED (new gap) | The gates that correctly govern *new*-input dispatch are also applied to release-type actions, which must never be blocked — confirmed via direct source trace of all four trigger paths. |
| Card-edit hotkey capture (macOS) | `unbind_hotkey` → `bind_hotkey` | Sequential, unconditional `invoke()` calls, `App.tsx:574-576` | NOT_WIRED (new gap) | `unbind_hotkey` runs and mutates state before `bind_hotkey`'s outcome is known; a conflict rejection leaves the macro with no binding at all. |
| Card-edit hotkey capture (Windows) | `set_macro_trigger_key` → `Intent::SetMacroTriggerKey` | Single `invoke()` call, `App.tsx:579`, no Result-carrying reply | NOT_WIRED (new gap) | Conflict is silently coerced to `None`/`0` and applied; `ipc::set_macro_trigger_key` always returns `Ok(())`, so the frontend has no way to detect the rebind was actually rejected. |
| CGEventTap/Win32 hook keydown → `HOTKEY_BINDINGS` lookup → `Intent::ToggleMacroHotkey` | Single dispatch site per platform | `observer.rs`/`windows/mod.rs` keydown/hook callbacks | WIRED (fixed this round) | Confirmed single dispatch site on both platforms; the prior round's double-dispatch defect is closed. |

### Data-Flow Trace (Level 4)

Not applicable in the traditional sense (no dashboard/data-fetch component in this phase's scope) — the equivalent trace performed here is the source-level control-flow trace of `handle_action`'s gating logic against the four claimed trigger paths, documented in full under Re-verification Summary and the Gaps section.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full backend regression suite (re-run independently) | `cargo test --lib` (from `src-tauri/`) | 14 passed; 0 failed | PASS |
| Prior-round hotkey double-dispatch fix confirmed | `grep -riq 'macro_trigger_keys' src-tauri/src; echo $?` | returns `1` (zero matches) | PASS |
| Single ToggleMacroHotkey dispatch site per platform | `grep -n 'ToggleMacroHotkey' src-tauri/src/platform/macos/observer.rs` / windows equivalent | exactly one hit in each file | PASS |
| No debt markers in phase-touched files | `grep -n 'TBD\|FIXME\|XXX'` across state/mod.rs, ipc/mod.rs, observer.rs, windows/mod.rs, App.tsx | no matches | PASS |
| CR-01 (HoldRelease gating) mechanism — direct source trace | Read `state/mod.rs:373-413` + 4 trigger-site reads (lines 491-500, 643-649, 661-671, 706-751) | Confirmed: uniform gating applies to HoldRelease; all 4 trigger paths mutate gating state before the guaranteed-delivery release arrives | FAIL (confirms the new gap) |
| CR-02 (hotkey rebind rollback) mechanism — direct source trace | Read `App.tsx:572-601` + `state/mod.rs:514-541` + `ipc/mod.rs:145-156` | Confirmed: macOS unbind-before-bind with no rollback; Windows silent coerce-to-None with no Result reply | FAIL (confirms the new gap) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| EXEC-01 | 09-01..09-10 | Multiple macros can run simultaneously on macOS | BLOCKED — scheduler-level parallel-execution proof remains solid and unaffected; the prior round's hotkey double-dispatch defect is genuinely closed; but two NEW confirmed Blocker-severity defects (HoldRelease gating stuck-input, hotkey-rebind data loss) directly undermine the reliability guarantee this requirement depends on. REQUIREMENTS.md currently marks this `[x] Complete` — that status is not supported by this verification pass and should be revisited. | Both new defects independently confirmed via direct source read this session (see Gaps). Real-device confirmation of the core concurrency claim (T9.1-T9.7) also still not genuinely performed. |
| EXEC-02 | 09-01..09-10 | Multiple macros can run simultaneously on Windows | BLOCKED — same disposition; the HoldRelease gating defect is platform-agnostic (applies identically on Windows since handle_action is shared code); the hotkey-rebind defect has an equally severe (arguably worse — zero user feedback) Windows-specific manifestation. Real-device Windows confirmation was never genuinely performed for this phase, across all verification passes. | Both defects independently confirmed via direct source read, applicable to Windows equally or more severely. |

REQUIREMENTS.md (lines 17-18, 103-104) currently marks both EXEC-01 and EXEC-02 as `[x] Complete`, citing only the resolved prior-round hotkey double-dispatch gap and pending on-device human verification. That marking does not yet reflect the two new Blocker-severity findings confirmed in this verification pass and should be updated once a gap-closure plan resolves them (or an explicit override is recorded).

No orphaned requirements found — REQUIREMENTS.md's Phase 9 row maps exactly to EXEC-01/EXEC-02, declared in plan frontmatter across all 10 plans (09-01 through 09-10).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src-tauri/src/state/mod.rs | 373-413 | `handle_action` applies the same engine/enabled/target gates to `ActionType::HoldRelease` as to new-input actions, with no bypass | Blocker | Confirmed real, reachable stuck-input bug via direct source trace of 4 independent trigger paths (disable macro, engine-toggle-off, active-app-switch, profile-load). Directly contradicts the project's Core Value. |
| src/App.tsx | 574-577 | `unbind_hotkey` runs and mutates state before the replacement `bind_hotkey`'s success/failure is known | Blocker | Confirmed real data-loss bug: a rejected rebind (key conflict) leaves the macro with no hotkey at all, on both platforms. Windows manifestation has zero user-facing error. |
| src-tauri/src/state/mod.rs | 514-541 | `Intent::SetMacroTriggerKey` silently coerces to `None`/`0` on conflict with no error-reply channel | Blocker (same defect, Windows root cause) | Same as above — `set_macro_trigger_key` always returns `Ok(())`, masking rebind failures from the frontend entirely. |
| src-tauri/src/platform/macos/observer.rs, src-tauri/src/state/mod.rs | (prior round's dual-registry sites) | (RESOLVED this round) Dual hotkey-registry double-dispatch | Closed | Confirmed fixed — single `HOTKEY_BINDINGS` registry, `MACRO_TRIGGER_KEYS` fully removed. No longer an anti-pattern. |
| src-tauri/src/platform/macos/observer.rs | 48-52 vs windows/mod.rs:524-536 | Cross-platform hotkey modifier-matching asymmetry (subset match on macOS, exact match on Windows) | Warning (carried forward from 09-REVIEW.md WR-01, not independently re-verified beyond the review's own analysis) | A macro bound to a hotkey can behave differently across platforms under extra held modifiers. Not gating for this pass; listed for completeness. |
| Various | — | Remaining Warning/Info items from 09-REVIEW.md (WR-02 through WR-05, IN-01 through IN-06) | Warning / Info | Real but pre-existing/unrelated to the core parallel-execution scope; not independently re-verified in this pass beyond the two Critical findings explicitly assigned. Listed for completeness, not gating. |

No TBD/FIXME/XXX debt markers found in phase-9-touched files (`state/mod.rs`, `observer.rs`, `windows/mod.rs`, `ipc/mod.rs`, `App.tsx`).

## Human Verification Required

### 1. CR-01 device confirmation (HoldRelease stuck-input, new this round)
**Test:** Enable a Hold-mode macro targeted at a specific app; let its HoldStart fire (input held down); then switch focus away from the target app (or disable the macro, or toggle the engine off, or load a different profile) and observe whether the held input is released.
**Expected:** The held key/mouse-button is released immediately. Per the confirmed source trace, it will NOT be — the only recovery is Emergency Stop.
**Why human:** Requires live CGEvent/SendInput injection and observation of real OS-level input state.

### 2. CR-02 device confirmation (hotkey rebind data loss, new this round)
**Test:** Give a macro a working hotkey; attempt to rebind it via the card-edit UI to a key already used by another macro; after the rejection, press the macro's original hotkey.
**Expected:** The original hotkey should still work. Per the confirmed source trace, it will NOT — the macro will have no working hotkey.
**Why human:** Requires live UI interaction and a real keypress to observe the end-to-end consequence.

### 3. macOS device tests (T9.1-T9.7)
**Test:** Run the documented macOS manual tests on a real macOS host with Accessibility + Input Monitoring granted, using the dashboard toggle (not a hotkey) to avoid the CR-01/CR-02 confounds.
**Expected:** All pass; concurrent firing and independent stop both hold.
**Why human:** Requires live CGEvent injection, live NSWorkspace notifications, and real-time UI/system observation.

### 4. Windows device tests (6.1-6.3)
**Test:** Run the 3 documented Windows manual tests on an actual Windows host — this has never happened across any verification pass for this phase.
**Expected:** Same concurrent-firing and independent-stop behavior via SendInput.
**Why human:** Requires a real Windows device; the "pass for now" note in 09-UAT.md is an explicit skip, not evidence.

## Gaps Summary

**Plan 09-10 successfully closed the prior round's sole Blocker-severity gap:** the dual hotkey-registry (`HOTKEY_BINDINGS` + `MACRO_TRIGGER_KEYS`) double-dispatch defect. Independently re-confirmed via direct source read (not merely trusted from SUMMARY.md or 09-REVIEW.md's prose): the redundant registry is fully removed (`grep` confirms zero matches crate-wide), exactly one dispatch site remains per platform, and the new regression test passes alongside the full 14/14 suite. No regressions were introduced.

**However, per this verification's explicit task instructions, I read `src-tauri/src/state/mod.rs` myself and confirmed two NEW Critical-severity findings that a fresh code review (09-REVIEW.md, written this session) surfaced — both are real, reachable Blockers, not false positives:**

1. **CR-01 (HoldRelease gating — 09-REVIEW.md's own numbering, distinct from the now-closed prior-round CR-01):** `handle_action` applies the same engine/enabled/target gates to `HoldRelease` actions as to new-input actions. Four independent, real-world-reachable trigger paths (disable a Hold-mode macro, toggle the engine off, switch the active app away from target, load a new profile) each mutate the gating state *before* the guaranteed-delivery release arrives, causing the gate to silently discard it and leave a physical key/mouse-button stuck down. This directly contradicts the project's Core Value ("a macro that was set up must fire reliably") and undermines the very two-phase dispatch mechanism this phase's plans 09-05/09-06 hardened for channel delivery.

2. **CR-02 (hotkey rebind rollback):** Rebinding an existing macro's hotkey to a conflicting key destroys the macro's previously-working binding with no rollback, on both platforms — silently on Windows (zero user feedback), with a misleading toast on macOS (nothing indicates the old binding was just deleted).

**Neither finding is a regression introduced by plan 09-10** — both are pre-existing defects in code that 09-10 did not touch (`handle_action`, the App.tsx hotkey-rebind sequence, `Intent::SetMacroTriggerKey`). Both are recorded as new gaps rather than silently absorbed into a clean pass, because they are confirmed (not merely suspected) and directly bear on the reliability guarantee EXEC-01/EXEC-02 and the phase's Core Value depend on.

ROADMAP SC1/SC2 real-device confirmation also still has not been genuinely completed — unchanged from the prior pass, still `human_needed`.

**Recommended next step:** Open a new gap-closure plan to (1) add an unconditional `HoldRelease` bypass at the top of `handle_action` so a release is always injected regardless of gate state, with a regression test driving one of the four confirmed trigger paths; and (2) fix the hotkey-rebind flow — drop the leading `unbind_hotkey` call on macOS (relying on `bind_hotkey`'s existing overwrite-on-success/preserve-on-conflict semantics) and give `Intent::SetMacroTriggerKey` the same `Result`-carrying oneshot pattern `Intent::BindHotkey` already uses on Windows. REQUIREMENTS.md's current `[x] Complete` marking for EXEC-01/EXEC-02 should be revisited once these are resolved (or an explicit override is recorded).

---

_Verified: 2026-07-22T20:00:00Z_
_Verifier: Claude (gsd-verifier)_
