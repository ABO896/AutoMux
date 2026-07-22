---
phase: 09-parallel-macro-execution
verified: 2026-07-22T18:05:00Z
status: gaps_found
score: 11/13 must-haves verified
behavior_unverified: 2
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 9/13
  gaps_closed:
    - "Gap #10 (set_macro_target_app silently discarded target_app on card-edit): independently confirmed by direct grep+read of src-tauri/src/ipc/mod.rs — `#[command(rename_all = \"snake_case\")]` is present immediately above `pub async fn set_macro_target_app` (line 42-43). Tauri now looks up the `target_app` key the frontend already sends (App.tsx:621), instead of the nonexistent camelCase `targetApp` key. The downstream Intent::SetMacroTargetApp handler (state/mod.rs:501-513) was already correct — only the deserialization boundary was broken, and that boundary is now fixed."
    - "Gap #11 (bind_hotkey/unbind_hotkey hard-fail on macOS card-edit hotkey change): independently confirmed — `#[command(rename_all = \"snake_case\")]` present above both `pub async fn bind_hotkey` (line 80-81) and `pub async fn unbind_hotkey` (line 99-100). Tauri now looks up the `macro_id` key App.tsx:575-576 already sends. The sibling `set_macro_trigger_key` (line 144-145) and the latent `update_step_interval` (line 283-284) were also swept, per plan scope."
    - "Gap #11 second half (catch-block mislabeling): independently confirmed by direct read of App.tsx:583-600 — the catch block in handleCardSetTriggerKey now branches on `macroMatch` (the conflict-format regex): a genuine conflict shows showConflictError with the true conflicting macro name; a non-conflict failure is console.error'd only. Both branches now call setEditingCardId(null)/setEditingField(null), fixing the previously-unreported frozen-card symptom on failure."
  gaps_remaining: []
  regressions: []
gaps:
  - truth: "Enabling a macro via a card-edit-assigned hotkey on macOS reliably toggles it on/off, exactly once per keypress (touches the phase's Core Value that a macro which was set up must fire reliably when triggered)"
    status: failed
    reason: "NEW finding this session, independently re-derived from source (not merely trusted from 09-REVIEW.md CR-01's prose) — confirmed by direct read of observer.rs:418-445, state/mod.rs:285-299 (build_hotkey_bindings_vec), state/mod.rs:542-591 (Intent::BindHotkey/UnbindHotkey handlers, both of which call self.reevaluate_all_macros().await at their tail), and state/mod.rs:770-789 (reevaluate_all_macros unconditionally calls platform::macos::observer::update_macro_trigger_keys under #[cfg(target_os = \"macos\")]). Net effect: any BindHotkey/UnbindHotkey call populates BOTH the HOTKEY_BINDINGS registry (via build_hotkey_bindings_vec, built from every macro with a trigger_key) AND the MACRO_TRIGGER_KEYS registry (via the reevaluate_all_macros call at the end of the same handler) with an entry for the same (keycode, modifiers) -> macro_id pair. The CGEventTap callback (observer.rs:418-445) checks both registries unconditionally on every keydown and calls tx.try_send(Intent::ToggleMacroHotkey(id)) for each match — a single physical keypress therefore dispatches ToggleMacroHotkey TWICE. Since the handler does `mac.enabled = !mac.enabled`, two flips cancel out: the macro is enabled and then immediately disabled again (or vice versa) before the user perceives any change. This is functionally a silent self-cancellation of the very 'trigger a macro' action the phase goal is about, for any macro whose hotkey was ever set/changed via the card-edit UI on macOS (which also retroactively double-registers every other macro that already had a trigger_key, per 09-REVIEW.md's analysis, independently re-confirmed here)."
    artifacts:
      - path: "src-tauri/src/platform/macos/observer.rs"
        issue: "CGEventTap keydown handler (lines 418-445) checks two independently-populated hotkey registries (HOTKEY_BINDINGS and MACRO_TRIGGER_KEYS) with no de-duplication; a keycode present in both dispatches Intent::ToggleMacroHotkey twice for one keypress."
      - path: "src-tauri/src/state/mod.rs"
        issue: "Intent::BindHotkey and Intent::UnbindHotkey handlers (lines 542-620) both populate HOTKEY_BINDINGS directly AND call self.reevaluate_all_macros().await, which (lines 770-789) unconditionally also populates MACRO_TRIGGER_KEYS on macOS — the two registries are not mutually exclusive as the set_macro_trigger_key doc comment (ipc/mod.rs:98) implies they should be."
    missing:
      - "Pick a single source of truth for the macOS hotkey registry: either remove the HOTKEY_BINDINGS lookup block from the tap callback (observer.rs:418-437) and stop maintaining it on macOS, or stop populating MACRO_TRIGGER_KEYS on macOS from reevaluate_all_macros (remove the #[cfg(target_os = \"macos\")] branch at state/mod.rs:786-787) and rely on HOTKEY_BINDINGS exclusively there."
      - "A regression test or manual check that sets a macro's hotkey via the card-edit UI on macOS, presses that hotkey once, and confirms the macro's enabled state actually flips (not silently no-ops)."
  - truth: "On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals (ROADMAP SC1)"
    status: human_needed
    reason: "Carried forward unchanged from the prior verification pass — not part of this round's re-verified scope. See behavior_unverified_items."
  - truth: "On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A (ROADMAP SC2)"
    status: human_needed
    reason: "Carried forward unchanged from the prior verification pass — not part of this round's re-verified scope. See behavior_unverified_items."
deferred: []
behavior_unverified_items:
  - truth: "On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals (ROADMAP SC1)"
    test: "On a real macOS host with Accessibility + Input Monitoring granted: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A via the dashboard toggle (not a hotkey, to avoid the new CR-01 confound above), wait ~1s, enable B the same way; observe both macro cards show a pulsing firing dot simultaneously and neither's click rate changes when the other starts/stops"
    expected: "Both macros visibly fire concurrently at their own configured rates on real macOS input injection (CGEvent)"
    why_human: "Unchanged since the prior pass — the 09-UAT.md session that did run on a real macOS host reported freeform issues but did not explicitly confirm the two-macro-concurrent-firing scenario in isolation; scheduler-level unit tests (re-run and confirmed passing, 13/13) prove platform-agnostic timeline/delivery logic but not real-device CGEvent injection timing with two simultaneous macros."
  - truth: "On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A (ROADMAP SC2)"
    test: "On a real Windows host: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A, wait ~1s, enable B; observe both fire concurrently via SendInput"
    expected: "Both macros fire concurrently at their own configured rates via SendInput"
    why_human: "Unchanged since the prior pass — 09-UAT.md explicitly recorded this as blocked_by: physical-device; the human tester chose to mark it 'pass for now' without device access. This remains a deliberate skip, not a passing result."
human_verification:
  - test: "Set a macro's hotkey via the card-edit UI on macOS (any macro), then press that hotkey once and observe whether the macro's enabled state actually toggles (new this round — CR-01 double-dispatch)."
    expected: "A single keypress toggles the macro on or off exactly once. If the observed behavior is 'nothing visibly happens' or 'toggles twice as fast as expected', CR-01 is confirmed at the device level, not just at the source-tracing level performed in this pass."
    why_human: "Requires a live CGEventTap on a real macOS host with Accessibility permission granted; cannot be triggered or observed through static analysis."
  - test: "Run T9.1-T9.7 on a real macOS host per this file's Section 5 (concurrent firing, stop-one-keeps-other, same-input concurrent + conflict warning, held-not-firing indicator, Hold-under-load, responsiveness-after-stop, creation-time targeting) using the dashboard toggle (not a hotkey) to enable/disable macros so the CR-01 finding above does not confound the result."
    expected: "All tests pass; both macro cards show independent pulsing firing dots; stopping one does not affect the other; system responsiveness returns to baseline after stopping a fast-interval macro; a macro created with a target app fires only when that app is focused."
    why_human: "Real macOS device, live CGEvent injection, live UI observation, live NSWorkspace notifications — cannot be verified by static analysis or from this (non-macOS-GUI) verification session."
  - test: "Run 6.1-6.3 on a real Windows host per this file's Section 6 — this has never actually been executed (09-UAT.md recorded it as blocked_by: physical-device, not as a genuine pass)."
    expected: "All 3 tests pass via SendInput injection and the Win32 hook observer."
    why_human: "Requires a real Windows device; the 'pass for now' note in 09-UAT.md is an explicit skip, not evidence."
  - test: "On a real macOS host, edit an existing macro's target app via the card-edit dropdown and confirm the new target actually gates firing (fires only when that app is frontmost); separately, edit an existing macro's hotkey via the card-edit UI (using the dashboard toggle, not the newly-edited hotkey, to observe the result) and confirm the change is retained across a state-changed refresh."
    expected: "Both edits persist and take effect — device-level confirmation of the IPC-layer fix (gaps #10/#11) applied this round."
    why_human: "The rename_all fix is confirmed correct at the source/mechanical level (grep, build, clippy, test all green) but on-device runtime confirmation that the edited value actually gates behavior has not yet been performed."
---

# Phase 9: Parallel Macro Execution Verification Report

**Phase Goal:** Multiple macros can run simultaneously on both macOS and Windows — triggering a second macro never blocks, queues, or cancels a running one
**Verified:** 2026-07-22T18:05:00Z
**Status:** gaps_found
**Re-verification:** Yes — after gap-closure plan 09-09 (IPC argument-casing sweep), following the prior `gaps_found` verification pass (score 9/13) that recorded gaps #10 and #11.

## Re-verification Summary

The prior verification pass (2026-07-22T15:10:00Z) recorded `status: gaps_found`, `score: 9/13`, with two Blocker-severity gaps: (#10) card-edit target-app changes silently discarded due to a Tauri IPC argument-casing mismatch, and (#11) card-edit hotkey changes hard-failing on macOS for the same root-cause class of bug. Plan 09-09 was executed this session to close both.

**I independently re-verified all of plan 09-09's claims against current source, not SUMMARY.md's claims:**

- Ran `grep -n "rename_all" src-tauri/src/ipc/mod.rs` myself: 5 hits, at lines 42, 80, 99, 144, 283 — immediately above `set_macro_target_app`, `bind_hotkey`, `unbind_hotkey`, `set_macro_trigger_key`, and `update_step_interval` respectively. This matches exactly the 5 commands named in both 09-VERIFICATION's gaps and 09-09-PLAN.md's task list — no more, no fewer.
- Read `src/App.tsx:572-601` directly: `handleCardSetTriggerKey`'s catch block now branches on `macroMatch` (the `/is already assigned to "([^"]+)"/` regex) — `showConflictError` is called only inside the `if (macroMatch)` branch with the true conflicting macro name; the `else` branch does `console.error` only. Both branches call `setEditingCardId(null)` / `setEditingField(null)`.
- Re-ran the full regression suite myself (not trusting SUMMARY's reported numbers): `cargo build` succeeds, `cargo test --lib` reports 13 passed / 0 failed, `cargo clippy --all-targets -- -D warnings` exits clean, `npx tsc --noEmit -p tsconfig.json` exits 0. No regressions from the prior pass.
- Cross-checked the two task commits (`f154691`, `441af05`) exist in `git log` with diffs matching their claimed scope (`ipc/mod.rs`: 5 insertions/5 deletions; `App.tsx`: 14 insertions/8 deletions) — consistent with a mechanical attribute-add and a catch-block restructure, not a broader change.

**Both gaps #10 and #11 are genuinely closed at the IPC-deserialization-boundary level.** The downstream Intent handlers (`Intent::SetMacroTargetApp`, `Intent::BindHotkey`, `Intent::UnbindHotkey` — all read directly in `state/mod.rs`) were already implemented correctly; the only defect was that the frontend's snake_case JSON keys never reached them. That defect is now fixed. On-device confirmation that an edited target app or hotkey actually persists and gates behavior in a running app is still an outstanding human-verification item (unchanged in kind from the prior pass's T9.7 test), not a code-level gap.

**However, while independently verifying this round's fix, I also independently re-derived — from source, not from 09-REVIEW.md's prose — the CR-01 finding that same fresh code review (09-REVIEW.md, `status: issues_found`, 1 Critical) surfaced this session: a dual hotkey-registry double-dispatch bug on macOS.** This is NOT part of plan 09-09's scope (09-09 touched only `ipc/mod.rs` command attributes and one `App.tsx` catch block; CR-01 lives in `observer.rs`'s CGEventTap callback and `state/mod.rs`'s `BindHotkey`/`UnbindHotkey`/`reevaluate_all_macros` interaction, none of which 09-09 touched) and was not one of the gaps that gap-closure round targeted. It is also not a regression introduced by 09-09 — the dual-registry interaction predates this session's changes.

I traced the mechanism myself:
- `build_hotkey_bindings_vec` (`state/mod.rs:285-299`) builds `HOTKEY_BINDINGS` from every macro with a `trigger_key` set, called from `Intent::BindHotkey`/`Intent::UnbindHotkey` (`state/mod.rs:583, 607`).
- Both of those same handlers *also* call `self.reevaluate_all_macros().await` at their tail (`state/mod.rs:591, ~614` region), and `reevaluate_all_macros` (`state/mod.rs:770-789`) unconditionally populates `MACRO_TRIGGER_KEYS` on macOS too (`#[cfg(target_os = "macos")] crate::platform::macos::observer::update_macro_trigger_keys(...)`).
- The result: any `bind_hotkey`/`unbind_hotkey` call — which is exactly what the card-edit hotkey flow (`App.tsx:574-577`) performs on macOS — leaves the *same* macro's `(keycode, modifiers) -> macro_id` entry present in **both** registries simultaneously.
- The CGEventTap keydown handler (`observer.rs:418-445`) checks both registries unconditionally on every keypress and does `tx.try_send(Intent::ToggleMacroHotkey(id))` for each match, with no de-duplication between the two blocks.
- `Intent::ToggleMacroHotkey`'s handler does `mac.enabled = !mac.enabled`, so two dispatches from one keypress cancel out: the macro silently fails to toggle.

I judge this affects phase-goal verification status: it is a confirmed (not merely suspected), Critical-severity, phase-relevant defect that undermines the reliability of *triggering* a macro via hotkey — directly touching the phase goal's own language ("triggering a second macro never blocks, queues, or cancels a running one") and the project's stated Core Value ("a macro that was set up must fire reliably"). It is recorded as a new gap below rather than deferred as unrelated debt, per the instruction to document it appropriately rather than silently ignore it. It does **not** invalidate the scheduler-level EXEC-01/EXEC-02 mechanism itself (the two-phase dispatch and concurrent-timer proof are untouched and re-confirmed passing), nor does it affect macros enabled via the dashboard toggle switch rather than a hotkey — but it is a real, reproducible-by-source-trace defect that this phase's own touched code (the macOS observer, modified in plan 09-07) exhibits.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Two macros at different intervals both fire concurrently in the scheduler, neither blocking the other (EXEC-01/EXEC-02, plan 09-01) | VERIFIED | `cargo test --lib` re-run independently this session: `scheduler::tests::parallel_two_macros_concurrent` passes (13/13 suite, unchanged). |
| 2 | Stopping one running macro does not affect another concurrently running macro (EXEC-02, plan 09-01) | VERIFIED | `scheduler::tests::parallel_stop_one_keeps_other` re-run independently: passes. |
| 3 | action_tx channel capacity is 1024 | VERIFIED | `grep 'mpsc::channel::<scheduler::ActionReady>(1024)' src-tauri/src/lib.rs` confirmed present (unchanged file). |
| 4 | HoldStart/HoldRelease both use symmetric guaranteed-delivery `.await` (plans 09-05/09-06) | VERIFIED | `start_macro_hold_start_delivered_under_saturation` and `stop_macro_release_delivered_under_saturation` both re-run independently: pass. |
| 5 | Per-macro card shows firing/waiting/held/combined/disabled derived from computeRunningState (plan 09-02/09-04) | VERIFIED | Unchanged this session; not touched by 09-09 (git diff scope confirmed: only `ipc/mod.rs` and `App.tsx`'s `handleCardSetTriggerKey`). |
| 6 | G-09-1a: CGEventTap event mask narrowed to only the 8 consumed event types | VERIFIED (source) | Unchanged this session (`observer.rs` not touched by 09-09); confirmed still present at the previously-cited lines. |
| 7 | G-09-1b: A user can delete a macro from within the app | VERIFIED | Unchanged this session; `handleRemoveMacro`/delete button untouched by 09-09. |
| 8 | G-09-1c primary: active_app updated from NSWorkspace notification's own userInfo, with logging | VERIFIED (source) | Unchanged this session (`observer.rs` not modified by 09-09). |
| 9 | G-09-1c secondary: card-edit target-app `<select>` is controlled and pre-selects the macro's current target | VERIFIED | Unchanged this session. |
| 10 | A user can change an existing macro's target app via the card-edit dropdown and have the change persist (IPC layer) | VERIFIED (source) | `#[command(rename_all = "snake_case")]` confirmed present at `ipc/mod.rs:42`, immediately above `set_macro_target_app` (line 43). `target_app` key now round-trips; downstream `Intent::SetMacroTargetApp` handler logic (`state/mod.rs:501-513`) was already correct. Device-level "takes effect" confirmation still pending — see human_verification. |
| 11 | A user can change an existing macro's hotkey via the card-edit UI on macOS and have the change succeed (IPC layer + error classification) | VERIFIED (source) | `#[command(rename_all = "snake_case")]` confirmed present above both `bind_hotkey` (line 80-81) and `unbind_hotkey` (line 99-100); `macro_id` key now round-trips instead of hard-erroring. Catch-block mislabeling also fixed (`App.tsx:583-600`, `macroMatch` branch confirmed). Device-level confirmation still pending. |
| 12 | ROADMAP SC1 — concurrent firing holds on a real macOS device | PRESENT_BEHAVIOR_UNVERIFIED | Carried forward unchanged; see behavior_unverified_items. |
| 13 | ROADMAP SC2 — concurrent firing holds on a real Windows device | PRESENT_BEHAVIOR_UNVERIFIED | Carried forward unchanged; 09-UAT.md explicitly marked this `blocked_by: physical-device`. |

**Score:** 11/13 truths verified (2 present-behavior-unverified). **Plus 1 new Blocker-severity gap (CR-01, hotkey double-dispatch) discovered this session, outside the original 13-truth accounting — see Gaps below.**

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/scheduler/mod.rs` | Symmetric guaranteed-delivery hold lifecycle + parallel-execution proof tests | VERIFIED | Unchanged this session; 13/13 tests pass. |
| `src-tauri/src/platform/macos/observer.rs` | Narrowed CGEventTap mask + hardened NSWorkspace observer | VERIFIED (for 09-07's scope) / NEW GAP (dual hotkey registry, CR-01) | 09-07's original changes (event mask, NSWorkspace) confirmed still correct. The separately-existing dual-registry hotkey dispatch mechanism (pre-dating 09-07, not modified by any Phase 9 plan) is a confirmed defect — see gap below. |
| `src/App.tsx` | Macro-card delete control, controlled target-app select, corrected catch-block error classification | VERIFIED | All three present and correctly wired: delete works end-to-end; target-select displays and now round-trips the right value; `handleCardSetTriggerKey` catch block no longer mislabels non-conflict failures. |
| `src-tauri/src/ipc/mod.rs` | Commands whose frontend-facing argument names correctly round-trip | VERIFIED | All 5 previously-affected commands (`set_macro_target_app`, `bind_hotkey`, `unbind_hotkey`, `set_macro_trigger_key`, `update_step_interval`) now carry `rename_all = "snake_case"`. `grep -rn "rename_all" src-tauri/src` returns 5 hits (was 0). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| Macro card delete button | `remove_macro` IPC → `Intent::RemoveMacro` | `invoke("remove_macro", { id })` | WIRED | Unchanged; unaffected by the casing bug (`id` has no underscore). |
| Card-edit target `<select onChange>` | `set_macro_target_app` IPC | `invoke("set_macro_target_app", { id, target_app })` | WIRED (fixed this round) | `rename_all` now present; the `target_app` key deserializes correctly. Device-level "takes effect" confirmation still pending (human_verification). |
| Card-edit hotkey capture | `bind_hotkey`/`unbind_hotkey`/`set_macro_trigger_key` IPC (macOS) | `invoke("unbind_hotkey", { macro_id: id })` then `invoke("bind_hotkey", {...})` then `invoke("set_macro_trigger_key", {...})` | WIRED (fixed this round) / NEW GAP downstream | `macro_id`/`trigger_key` keys now deserialize correctly (gap #11 IPC layer closed). However, the very act of running this sequence is what populates BOTH `HOTKEY_BINDINGS` and `MACRO_TRIGGER_KEYS` for the same macro (see CR-01 gap below) — the IPC call itself now succeeds, but the resulting hotkey dispatch at runtime is a confirmed double-dispatch no-op. |
| CGEventTap keydown → `HOTKEY_BINDINGS` lookup AND `MACRO_TRIGGER_KEYS` lookup | `Intent::ToggleMacroHotkey` (dispatched up to twice per keypress) | Two independent, non-mutually-exclusive registry checks in the same callback, `observer.rs:418-445` | NOT_WIRED (double-dispatch defect) | New finding this session (CR-01), independently re-derived from source. A macro whose hotkey was set/changed via the card-edit UI on macOS ends up registered in both registries; a single keypress toggles `mac.enabled` twice, canceling out. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full backend regression suite (re-run independently) | `cargo test --lib` (from `src-tauri/`) | 13 passed; 0 failed | PASS |
| Clippy clean | `cargo clippy --all-targets -- -D warnings` (from `src-tauri/`) | exit 0 | PASS |
| Build clean | `cargo build` (from `src-tauri/`) | Finished, no errors | PASS |
| Frontend type-checks clean | `npx tsc --noEmit -p tsconfig.json` | exit 0 | PASS |
| `rename_all` present on all 5 affected commands | `grep -n "rename_all" src-tauri/src/ipc/mod.rs` | 5 hits at lines 42, 80, 99, 144, 283 — matching exactly `set_macro_target_app`, `bind_hotkey`, `unbind_hotkey`, `set_macro_trigger_key`, `update_step_interval` | PASS |
| `handleCardSetTriggerKey` catch block branches correctly | direct read, `App.tsx:583-600` | `if (macroMatch)` branch present; else branch logs only; both branches reset edit state | PASS |
| Task commits exist and match claimed scope | `git show --stat f154691 441af05` | `f154691`: ipc/mod.rs, 5+/5-; `441af05`: App.tsx, 14+/8- | PASS |
| CR-01 dual-registry mechanism (new, independently re-derived) | direct read, `observer.rs:418-445`, `state/mod.rs:285-299,542-620,770-789` | Confirmed: BindHotkey/UnbindHotkey populate HOTKEY_BINDINGS directly AND trigger reevaluate_all_macros, which populates MACRO_TRIGGER_KEYS on macOS too; tap callback checks both unconditionally | FAIL (confirms the new gap) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| EXEC-01 | 09-01..09-09 | Multiple macros can run simultaneously on macOS | PARTIAL — scheduler-level proof solid; macro-editing IPC reliability now fixed; hotkey-toggle reliability has a newly-confirmed regression-class defect (CR-01) | The core concurrent-scheduling mechanism remains verified at the scheduler/unit-test level, unaffected by any change or finding this round. Card-edit target-app/hotkey persistence (gaps #10/#11) is now fixed at the IPC layer. However, a macro whose hotkey is set via the card-edit UI on macOS can silently fail to toggle on a single keypress due to CR-01 — a confirmed, source-traced defect, not yet device-confirmed. On-device macOS confirmation of the core concurrency claim (T9.1-T9.7) also still not genuinely performed. |
| EXEC-02 | 09-01..09-09 | Multiple macros can run simultaneously on Windows | PARTIAL — same disposition; CR-01 is macOS-specific (Windows never populates HOTKEY_BINDINGS the same way) but Windows device confirmation was explicitly skipped by the human tester | Scheduler logic is identical/platform-agnostic; the IPC casing fix (gaps #10/#11) is platform-independent and benefits Windows equally. CR-01 (dual hotkey registry) is macOS-specific — Windows uses only `MACRO_TRIGGER_KEYS`, per `state/mod.rs:786-789`'s platform-conditional branches. No genuine Windows-device confirmation has ever been performed for this phase, across all verification passes. |

No orphaned requirements found — REQUIREMENTS.md's Phase 9 row (lines 103-104) maps exactly to EXEC-01/EXEC-02, declared in plan frontmatter across all 9 plans (09-01 through 09-09).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src-tauri/src/platform/macos/observer.rs | 418-445 | CGEventTap keydown handler checks two independently-populated hotkey registries (`HOTKEY_BINDINGS`, `MACRO_TRIGGER_KEYS`) with no de-duplication between them | Blocker | NEW finding this session, independently re-derived from source (not merely trusted from 09-REVIEW.md). A macro whose hotkey was set/changed via the card-edit UI on macOS can silently fail to toggle on a keypress — two `Intent::ToggleMacroHotkey` dispatches cancel out. Directly undermines the phase goal's own "triggering... never cancels" language, applied to hotkey-based triggering specifically. |
| src-tauri/src/state/mod.rs | 542-620, 770-789 | `Intent::BindHotkey`/`Intent::UnbindHotkey` populate `HOTKEY_BINDINGS` directly AND unconditionally call `reevaluate_all_macros()`, which also populates `MACRO_TRIGGER_KEYS` on macOS | Blocker (same defect, root cause) | Same as above — the two registries are not mutually exclusive on macOS despite the `set_macro_trigger_key` doc comment implying they should be. |
| src-tauri/src/ipc/mod.rs | 43, 81, 100, 145, 284 | (RESOLVED this round) Snake_case Rust parameter names previously had no `rename_all` attribute | Closed | Confirmed fixed — `rename_all = "snake_case"` now present on all 5 affected commands. No longer an anti-pattern. |
| src-tauri/src/scheduler/mod.rs | 235-247 (`SchedulerIntent::UpdateInterval` handler) | `running_configs` cache never updated on live interval tuning | Info (out of Phase 9 scope, carried forward, unchanged) | Real defect, confirmed dormant — `update_step_interval` has zero call sites in `src/App.tsx`. Pre-existing, unrelated to parallel-execution behavior. |
| src/App.tsx | 574-577 | `unbind_hotkey` runs and mutates state before the replacement `bind_hotkey` is confirmed to succeed | Info (out of Phase 9 scope, carried forward, unchanged — 09-REVIEW.md CR-04) | Ordering issue, separate from CR-01's registry-duplication issue; not addressed by 09-09 (explicitly out of scope per the plan). |
| Various | — | Remaining Warning/Info items from 09-REVIEW.md (WR-02 through WR-05, IN-01 through IN-05) | Warning / Info | Real but pre-existing/unrelated to Phase 9's core parallel-execution scope; listed for completeness, not gating. Not independently re-verified in this pass beyond CR-01 (which was elevated due to explicit task instruction to assess its bearing on phase-goal status). |

No TBD/FIXME/XXX debt markers found in phase-9-modified files (`observer.rs`, `App.tsx`, `scheduler/mod.rs`, `ipc/mod.rs`, `lib.rs`).

## 5. Manual macOS Device Test (EXEC-01)

*Still not executed as a clean, isolated pass. Recommend using the dashboard toggle switch (not a hotkey) to enable/disable macros during T9.1-T9.7, so the newly-confirmed CR-01 hotkey double-dispatch defect does not confound the result.*

Pre-flight:
- [ ] Accessibility granted to AutoMux.
- [ ] Input Monitoring granted to AutoMux.
- [ ] App running via `npm run tauri dev` or a release build.

### Test T9.1 — Enabling macro B while macro A is firing does not affect A (ROADMAP SC1)
1. Create macro A: "Click A", Left Click, 200ms. 2. Create macro B: "Click B", Right Click, 300ms. 3. Enable A via the dashboard toggle — pulsing dot. 4. Wait ~1s. 5. Enable B the same way — both pulse independently at their own rates.

### Test T9.2 — Stopping macro A does not affect macro B (ROADMAP SC3)
Continue from T9.1; disable A; B keeps pulsing at 300ms.

### Test T9.3 — Two same-input macros both fire + Phase 8 conflict warning shows
Both Left Click, both enabled — both pulse, conflict warning appears.

### Test T9.4 — Hold-mode macro shows "held", not "firing"
Static accent dot, not pulsing green.

### Test T9.5 — Hold-mode macro under simulated load actually holds
Several fast macros running; Hold-mode macro genuinely holds the input physically.

### Test T9.6 — G-09-1a: system responsiveness returns to baseline after stopping a fast-interval macro
Run a 50-100ms-interval macro for ~30s, stop it, and confirm overall macOS responsiveness (cursor tracking, other apps) returns to normal within a few seconds.

### Test T9.7 — G-09-1c: a macro CREATED with a target app fires only when that app is focused
Create a new macro (not edit an existing one) with a specific target app set at creation time; confirm it fires only when that app is frontmost.

### Test T9.8 — NEW this round: card-edit hotkey toggle actually toggles (CR-01)
Set a macro's hotkey via the card-edit "Press…" flow, then press that hotkey once. Confirm the macro's enabled state changes exactly once, not zero times (silent no-op) or twice (also a no-op from the user's perspective).

### Test T9.9 — NEW this round: card-edit target-app / hotkey changes persist (gaps #10/#11 device confirmation)
Edit an existing macro's target app via the card dropdown; confirm it now gates firing correctly (fires only when the new target is frontmost). Separately, edit an existing macro's hotkey via the card UI and confirm the new key is retained after a page refresh / state-changed event.

## 6. Manual Windows Device Test (EXEC-02)

*Still never genuinely executed. 09-UAT.md recorded this as `blocked_by: physical-device`.*

### Test 6.1 — Enabling macro B while macro A is firing does not affect A (ROADMAP SC2)
### Test 6.2 — Stopping macro A does not affect macro B (ROADMAP SC3)
### Test 6.3 — Two same-input macros both fire + conflict warning shows

### Human Verification Required

### 1. CR-01 device confirmation (new this round)
**Test:** Set a macro's hotkey via the card-edit UI on macOS, press it once, observe whether the macro's enabled state toggles.
**Expected:** Toggles exactly once per keypress.
**Why human:** Requires a live CGEventTap on a real macOS host; the double-dispatch mechanism is confirmed at the source level but not yet observed live.

### 2. macOS device tests (T9.1-T9.9)
**Test:** Run the documented macOS manual tests, including the two new ones (T9.8 hotkey-toggle, T9.9 edit-persistence) on a real macOS host with Accessibility + Input Monitoring granted.
**Expected:** All pass; concurrent firing and independent stop both hold; card-edit hotkey toggles work exactly once; card-edit target-app/hotkey edits persist and take effect.
**Why human:** Requires live CGEvent injection, live NSWorkspace notifications, and real-time UI/system observation.

### 3. Windows device tests (6.1-6.3)
**Test:** Run the 3 documented Windows manual tests on an actual Windows host — this has never happened across any verification pass for this phase.
**Expected:** Same concurrent-firing and independent-stop behavior via SendInput.
**Why human:** Requires a real Windows device; the "pass for now" note in 09-UAT.md is an explicit skip, not evidence.

## Gaps Summary

**Plan 09-09 successfully closed both Blocker-severity gaps carried forward from the prior verification pass:**

1. **Gap #10 (target-app edit silently discarded)** — CLOSED. `rename_all = "snake_case"` added to `set_macro_target_app`; independently confirmed present via direct grep and source read. The IPC deserialization boundary is fixed; the downstream Intent handler logic was already correct.
2. **Gap #11 (hotkey edit hard-fails / mislabeled error)** — CLOSED. `rename_all = "snake_case"` added to `bind_hotkey`/`unbind_hotkey` (plus the sibling `set_macro_trigger_key` and latent `update_step_interval`, swept per plan scope); the frontend catch block no longer mislabels non-conflict failures as hotkey conflicts, and resets edit state on failure. Both independently confirmed via direct source read.

No regressions were introduced by this round: `cargo test --lib` (13/13), `cargo clippy` (clean), `cargo build` (clean), and `npx tsc --noEmit` (clean) were all re-run independently and match the SUMMARY's claims exactly.

**However, while performing this re-verification, a fresh independent code-review pass (09-REVIEW.md, committed this session) surfaced a NEW Critical-severity finding (CR-01) that I independently re-traced through source rather than trusting the review's prose. This finding is real, is not part of 09-09's scope, is not a regression introduced by 09-09, but is directly relevant to the phase's own goal language ("triggering... never cancels") when a macro is triggered via a card-edit-assigned hotkey on macOS:** a dual hotkey-registry (`HOTKEY_BINDINGS` + `MACRO_TRIGGER_KEYS`) double-dispatches `Intent::ToggleMacroHotkey` for a single keypress, silently canceling the toggle. This is recorded as a new gap (see frontmatter) rather than silently absorbed into a clean pass, per the explicit instruction to document review findings that bear on phase-goal verification status.

ROADMAP SC1/SC2 real-device confirmation also still has not been genuinely completed — unchanged from the prior pass, still `human_needed`.

**Recommended next step:** Open a new gap-closure plan to resolve CR-01 by picking a single source of truth for the macOS hotkey registry (either registry, not both — see the two fix directions documented in 09-REVIEW.md CR-01 and reproduced in this file's gap entry). Separately, complete the still-outstanding macOS (T9.1-T9.9) and Windows (6.1-6.3) device-level manual tests, ideally using the dashboard toggle (not a hotkey) for T9.1-T9.7 so the CR-01 defect does not confound those specific results.

---

_Verified: 2026-07-22T18:05:00Z_
_Verifier: Claude (gsd-verifier)_
