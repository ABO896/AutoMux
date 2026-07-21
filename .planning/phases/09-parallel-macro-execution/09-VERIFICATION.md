---
phase: 09-parallel-macro-execution
verified: 2026-07-21T20:15:00Z
status: human_needed
score: 8/9 must-haves verified
behavior_unverified: 1
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 7/9
  gaps_closed:
    - "Scheduler::start_macro's HoldStart send (SustainedHold branch, scheduler/mod.rs 300-325) now uses guaranteed self.action_tx.send(...).await delivery instead of fire-and-forget try_send, mirroring release_holds/StopAll. Closed by plan 09-06, independently re-confirmed against current source (scheduler/mod.rs:315-325) — not merely trusted from 09-06-SUMMARY.md's claims."
  gaps_remaining: []
  regressions: []
gaps: []
deferred: []
behavior_unverified_items:
  - truth: "On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals (ROADMAP SC1)"
    test: "On a real macOS host with Accessibility + Input Monitoring granted: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A, wait ~1s, enable B; observe both macro cards show a pulsing firing dot simultaneously and neither's click rate changes when the other starts/stops"
    expected: "Both macros visibly fire concurrently at their own configured rates on real macOS input injection (CGEvent) — documented as this file's Section 5, Test T9.1, still unexecuted (checkbox unchecked)"
    why_human: "Requires a real macOS device with Accessibility/Input Monitoring permissions and live observation of CGEvent injection timing and the per-card UI dots in real time; the scheduler-level unit tests (re-run and confirmed passing during this verification, 13/13) prove the platform-agnostic timeline/delivery logic but not real-device CGEvent injection behavior"
  - truth: "On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A (ROADMAP SC2)"
    test: "On a real Windows host: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A, wait ~1s, enable B; observe both fire concurrently via SendInput"
    expected: "Documented as this file's Section 6, Test 6.1, still unexecuted (checkbox unchecked)"
    why_human: "Requires a real Windows host with the Win32 SendInput injection path and Win32 hook observer running live; not exercisable from this (macOS) verification environment"
  - truth: "Stopping one running macro does not affect any other concurrently running macro (ROADMAP SC3)"
    test: "On real macOS and Windows hosts: with A and B both firing, disable A; observe B's dot keeps pulsing and B's fire rate is unaffected"
    expected: "Documented as this file's Sections 5/6, Tests T9.2/6.2, still unexecuted (checkboxes unchecked)"
    why_human: "Device-level, real-time observation required; scheduler-level unit test (parallel_stop_one_keeps_other, re-run and confirmed passing) proves the underlying gating logic between two DIFFERENT macros but not real-device confirmation"
human_verification:
  - test: "Run T9.1-T9.5 on a real macOS host per this file's Section 5 (concurrent firing, stop-one-keeps-other, same-input concurrent + conflict warning, held-not-firing indicator, Hold-under-load)"
    expected: "All tests pass; both macro cards show independent pulsing firing dots; stopping one does not affect the other; same-input pair both fire while the Phase 8 conflict warning also displays; a Hold-mode macro shows the static held dot and genuinely holds the input under load"
    why_human: "Real macOS device, live CGEvent injection, live UI observation — cannot be verified by static analysis or from this (non-macOS-GUI) verification session"
  - test: "Run 6.1-6.3 on a real Windows host per this file's Section 6"
    expected: "All 3 tests pass via SendInput injection and the Win32 hook observer"
    why_human: "Requires a real Windows device — not available in this verification environment"
---

# Phase 9: Parallel Macro Execution Verification Report

**Phase Goal:** Multiple macros can run simultaneously on both macOS and Windows — triggering a second macro never blocks, queues, or cancels a running one
**Verified:** 2026-07-21T20:15:00Z
**Status:** human_needed
**Re-verification:** Yes — after plan 09-06 (start-side HoldStart guaranteed-delivery fix, CR-01) landed, closing the prior verification's sole remaining Blocker gap

## Re-verification Summary

The prior verification pass (2026-07-21T09:40:00Z) recorded `status: gaps_found`, `score: 7/9`, with one Blocker-severity gap: `Scheduler::start_macro`'s `SustainedHold` branch sent `HoldStart` via fire-and-forget `try_send`, and `holds.push(*input)` ran unconditionally regardless of send success — asymmetric with the already-hardened `release_holds` path (fixed in plan 09-05). Under action-channel saturation this could produce a "phantom held" state: `active_holds`/`computeRunningState` report a Hold-mode macro as "held" while nothing was ever injected.

**Plan 09-06 closed that gap.** I independently re-read the current `src-tauri/src/scheduler/mod.rs` at HEAD (not 09-06-SUMMARY.md's claims) and confirmed: `start_macro`'s `SustainedHold` arm (lines 315-325) now sends `HoldStart` via `self.action_tx.send(...).await` — guaranteed delivery, byte-for-byte mirroring `release_holds` (381-393) and `StopAll` (214-234), with a matching `@safety-officer` doc comment citing CR-01/09-VERIFICATION.md/09-REVIEW.md. `holds.push(*input)` is retained immediately after the awaited send, exactly like `release_holds`'s post-send flow — no desync between recording and delivery.

I independently verified the RED→GREEN claim rather than trusting it: using a disposable `git worktree` checked out at the RED commit (`79d39f8`, test-added-but-fix-not-yet-applied), I ran `cargo test --lib scheduler::tests::start_macro_hold_start_delivered_under_saturation -- --exact` and confirmed it genuinely FAILS at that commit with `left: 1, right: 2` (the exact discriminator the plan and SUMMARY document) — the second macro's `HoldStart` is dropped by the saturated channel. I then confirmed the same test passes at current HEAD (post-fix commit `5e37063`). The diff between the two commits (`git diff 79d39f8 5e37063`) exactly matches the plan's described change: `try_send`+drop-count block replaced with `.send(...).await`, no other logic touched.

On current HEAD I independently re-ran the full backend regression surface (not trusting any SUMMARY-reported numbers): `cargo test --lib` → 13/13 pass (12 pre-existing + this new test); `cargo clippy --all-targets -- -D warnings` → exit 0, clean; `cargo build --release` → exit 0. I confirmed exactly one `ACTION_DROP_COUNT.fetch_add(1` call site remains in the file (the `fire_due_actions` Interval path) — the HoldStart-site counter was correctly removed as obsolete. I confirmed the release side (`release_holds`/`StopAll`) and the periodic Interval `try_send` path are both unchanged by this plan, satisfying the plan's stated prohibitions (no retry path added for HoldStart; Interval fire still non-blocking `try_send`; no desync between `active_holds` recording and delivery).

**A fresh, independent code-review pass performed after 09-06 landed** (`09-REVIEW.md`, this session, `status: issues_found`, 2 critical) confirms the prior CR-01 (asymmetric HoldStart) is now closed, and surfaces two new/carried-forward Critical issues that are **out of Phase 9's scope** per the task framing given for this verification and independently confirmed here:
- A new CR-01 in that review (renumbered, distinct from the now-closed HoldStart gap): `Scheduler.running_configs` goes stale after `SchedulerIntent::UpdateInterval`, which could reintroduce CR-04's Hold-macro restart flicker for macros combining `SustainedHold` + live-tuned `InterleavedInterval` steps. I confirmed by direct source read (scheduler/mod.rs:235-247) that `UpdateInterval` never touches `running_configs`. However, I also confirmed by grep that `update_step_interval` (the only caller path into `SchedulerIntent::UpdateInterval`) has **zero call sites in `src/App.tsx`** — the IPC command exists but nothing in the shipped frontend invokes it, so this defect is real but currently dormant/unreachable through the product's UI. It predates Phase 9 (the `CR-04` cache and `update_step_interval` command are v1.0-era) and does not touch parallel-execution behavior. Noted for backlog; does not gate this phase's goal.
- CR-02 (macOS hotkey rebind destroys the existing binding before the replacement is confirmed, `src/App.tsx:563-586`): confirmed still open, pre-existing from Phase 8 (UX-11 hotkey work). Unrelated to parallel macro execution. Noted for backlog; does not gate this phase's goal.

Both are recorded in Anti-Patterns below for completeness but are excluded from this phase's gap/blocker determination, since neither is caused by, or affects, this phase's parallel-execution/hold-lifecycle-reliability scope.

**Overall status moves from `gaps_found` to `human_needed`**: all automatable truths are now verified; the only remaining open items are the ROADMAP SC1/SC2/SC3 real-device manual tests (macOS/Windows), which were already tracked as behavior-unverified in the prior pass and remain unexecuted (checkboxes unchecked) — unaffected by this plan and explicitly out of its scope per 09-06-PLAN.md.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Two macros at different intervals both fire concurrently in the scheduler, neither blocking the other (EXEC-01/EXEC-02, plan 09-01) | ✓ VERIFIED | `cargo test --lib` re-run independently: `scheduler::tests::parallel_two_macros_concurrent` passes (part of 13/13 suite). |
| 2 | Stopping one running macro does not affect another concurrently running macro (EXEC-02, plan 09-01) | ✓ VERIFIED | `scheduler::tests::parallel_stop_one_keeps_other` re-run independently: passes. |
| 3 | action_tx channel capacity is 1024 to avoid silent overflow under parallel fire rates | ✓ VERIFIED | `grep 'mpsc::channel::<scheduler::ActionReady>(1024)' src-tauri/src/lib.rs` confirmed present, unchanged. |
| 4 | Debug-only ACTION_DROP_COUNT increments on try_send failure, readable via IPC, absent from release builds; the counter now lives at exactly the one remaining legitimate try_send site | ✓ VERIFIED | `grep -c 'fetch_add(1' src-tauri/src/scheduler/mod.rs` → 1 (down from 2 in the prior pass — the HoldStart-site counter was correctly removed as obsolete once delivery became guaranteed). Site is `fire_due_actions`'s Interval path, `#[cfg(debug_assertions)]`-gated. `cargo build --release` compiles clean. |
| 5 | Per-macro card shows firing/waiting/held/combined/disabled derived from computeRunningState mirroring the 3 backend injection gates (D-01, plan 09-02) | ✓ VERIFIED | Unchanged since prior pass — direct read confirms computeRunningState (src/App.tsx ~90-116) still matches StateActor::handle_action gates. Not touched by 09-06 (frontend files unchanged, confirmed via `git diff 56bb0f8..HEAD -- src/App.tsx` = empty). |
| 6 | Hold-mode macros created through the app's only creation path show a distinct "held" (static accent) indicator, not "firing"; Pulse-mode macros are unaffected (D-02, CR-01 fix, plan 09-04) | ✓ VERIFIED | Unchanged since prior pass — `trigger_mode === "Hold"` branch (App.tsx:105) confirmed present and correctly ordered; file byte-for-byte unchanged since prior pass. |
| 7 | Stopping a macro (release path) reliably releases its own held input even under action-channel saturation — no physical key/mouse button is left permanently stuck down (Core Value, plan 09-05 gap closure) | ✓ VERIFIED | Unchanged since prior pass — `release_holds` (381-393) still uses `self.action_tx.send(...).await`; `stop_macro_release_delivered_under_saturation` re-run independently: passes. |
| 8 | Starting a Hold-mode macro under action-channel saturation reliably delivers HoldStart before recording the input as held — no "phantom held" state (mirror-image of truth #7, on the start side) | ✓ VERIFIED | **Previously FAILED (the Blocker in the 7/9 pass); now closed by plan 09-06.** Direct read of scheduler/mod.rs:315-325 confirms `start_macro`'s SustainedHold arm now sends `HoldStart` via `self.action_tx.send(...).await` (guaranteed), matching `release_holds`/`StopAll`, with `holds.push(*input)` correctly ordered after the awaited send. The new `start_macro_hold_start_delivered_under_saturation` regression test passes on current HEAD (13/13 suite). I independently reproduced RED at commit `79d39f8` via a disposable `git worktree` (test genuinely fails there with `left: 1, right: 2`) and confirmed GREEN at HEAD — not merely trusted from 09-06-SUMMARY.md. |
| 9 | ROADMAP SC1/SC2/SC3 — concurrent firing and independent stop hold true on real macOS and Windows devices | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Scheduler-level unit tests (truths 1-2, 7-8) prove the platform-agnostic scheduling/gating/delivery logic these criteria depend on, but the 6 device-level manual tests (T9.1-T9.5, 6.1-6.3) documented in this file's Sections 5/6 are still unexecuted — all checkboxes remain unchecked. No real macOS or Windows device confirmation has been performed for this phase. |

**Score:** 8/9 truths verified (1 present-behavior-unverified). No Blocker-severity gap remains — status moves to `human_needed` pending real-device confirmation.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/scheduler/mod.rs` | 2 concurrency-proof tests + 2 saturation regression tests (release + start side) + ACTION_DROP_COUNT counter (1 remaining site) + guaranteed-delivery on both hold-lifecycle paths | ✓ VERIFIED | All tests present and passing (13/13 total). Both `release_holds` (09-05) and `start_macro`'s HoldStart (09-06) now use guaranteed `.await` delivery — fully symmetric. `start_macro_hold_start_delivered_under_saturation` (967-1055) added; obsolete HoldStart-site drop-counter removed. |
| `src-tauri/src/lib.rs` | action_tx capacity 1024, debug-gated IPC registration | ✓ VERIFIED | Unchanged, confirmed by direct grep. |
| `src-tauri/src/ipc/mod.rs` | get_debug_action_drop_count debug-only command | ✓ VERIFIED | Unchanged, calls the real accessor, cfg-gated. |
| `src/App.tsx` | computeRunningState + RunningState type, wired card rendering, Hold-mode branch | ✓ VERIFIED | Byte-for-byte unchanged since prior pass (confirmed via `git diff 56bb0f8..HEAD -- src/App.tsx`); the data-integrity risk noted in the prior pass (trigger_mode-only "held" derivation with no injection confirmation) is now resolved at the source — HoldStart delivery is guaranteed, so a recorded hold is now always a genuinely delivered one. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `get_debug_action_drop_count` (ipc/mod.rs) | `crate::scheduler::get_action_drop_count()` | direct call | ✓ WIRED | Unchanged, confirmed. |
| Macro card status dot | `runningState()` thunk → `computeRunningState(macro, state())` | reactive thunk | ✓ WIRED | Unchanged, confirmed — thunk form preserved, reactivity intact. |
| `release_holds` (scheduler/mod.rs) | `action_tx` (HoldRelease delivery) | `.await` (guaranteed) | ✓ WIRED, HARDENED | Fixed by 09-05; confirmed unchanged and current. |
| `start_macro`'s SustainedHold branch (scheduler/mod.rs) | `action_tx` (HoldStart delivery) | `.await` (guaranteed) | ✓ WIRED, HARDENED | **Fixed by 09-06.** Now symmetric with `release_holds`/`StopAll`. `holds.push(*input)` runs only after the awaited send, so `active_holds`/`computeRunningState`'s "held" derivation can no longer diverge from actual delivery outside a harmless receiver-shutdown race. Behaviorally proven (not just presence-checked) by the passing `start_macro_hold_start_delivered_under_saturation` test, independently RED/GREEN-verified via `git worktree`. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full lib test suite (re-run independently, not trusted from any SUMMARY) | `cd src-tauri && cargo test --lib` | 13 passed; 0 failed | ✓ PASS |
| Clippy clean, no new warnings | `cd src-tauri && cargo clippy --all-targets -- -D warnings` | exit 0, zero warnings | ✓ PASS |
| Release build compiles | `cd src-tauri && cargo build --release` | exit 0 | ✓ PASS |
| `start_macro`'s HoldStart send uses guaranteed `.await` delivery, matching the now-fixed release side | direct source read of scheduler/mod.rs:315-325 | Confirmed: `self.action_tx.send(...).await`, `holds.push` after | ✓ PASS |
| RED (pre-fix) genuinely fails; GREEN (post-fix) genuinely passes — reproduced independently, not trusted from SUMMARY | `git worktree` checkout at commit `79d39f8`; `cargo test --lib scheduler::tests::start_macro_hold_start_delivered_under_saturation -- --exact` | RED: FAILED, `left: 1, right: 2` (exact match to plan's documented discriminator). Same test passes at HEAD. | ✓ PASS |
| Exactly one `ACTION_DROP_COUNT.fetch_add(1` site remains (HoldStart-site counter removed as obsolete) | `grep -c 'fetch_add(1' src-tauri/src/scheduler/mod.rs` | 1 | ✓ PASS |
| Release-side hardening (`release_holds`/`StopAll`) and the periodic Interval `try_send` path are unchanged by 09-06 | direct source read | Confirmed both unchanged | ✓ PASS |
| No TBD/FIXME/XXX debt markers in phase-9-touched files | `grep -n -E "TBD\|FIXME\|XXX"` across scheduler/mod.rs, lib.rs, ipc/mod.rs, App.tsx | No matches | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| EXEC-01 | 09-01..09-06 | Multiple macros can run simultaneously on macOS | ⚠️ PARTIAL (scheduler-proof complete; on-device confirmation pending) | Scheduler-level proof complete and symmetric on both hold-lifecycle paths (unit tests, independently re-run, 13/13). All Blocker-severity reliability gaps for this phase (release-side in 09-05, start-side in 09-06) are closed. On-device macOS confirmation (T9.1-T9.5) still not performed — checkboxes unchecked. REQUIREMENTS.md marks this "Complete" ([x] EXEC-01); this remains formally correct at the scheduler-logic level but the device-level manual tests should still be run before full closure confidence. |
| EXEC-02 | 09-01..09-06 | Multiple macros can run simultaneously on Windows | ⚠️ PARTIAL (scheduler-proof complete; on-device confirmation pending) | Same disposition as EXEC-01 — scheduler logic is platform-agnostic (identical Scheduler code on both OSes), so the closed release-side and start-side reliability gaps apply equally to Windows. No Windows-device confirmation has been performed. |

No orphaned requirements found — REQUIREMENTS.md Phase 9 row maps exactly to EXEC-01/EXEC-02, both declared in plan frontmatter across all 6 plans (09-01 through 09-06).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src-tauri/src/scheduler/mod.rs | 235-247 (`SchedulerIntent::UpdateInterval` handler) | `running_configs` cache is never updated when a step's interval is live-tuned, which can cause a later `start_macro` call to see a false config mismatch and force an unwanted stop/restart (reintroducing CR-04's Hold-macro flicker) | ℹ️ Info (out of Phase 9 scope) | Real defect, confirmed by direct source read, newly surfaced by 09-REVIEW.md (this session) as its own CR-01 (distinct from the now-closed Phase-9 CR-01). Confirmed dormant: `update_step_interval` (the only path into this handler) has zero call sites in `src/App.tsx` — unreachable through the shipped UI today. Pre-existing v1.0-era `CR-04` cache issue; does not touch parallel-execution behavior added by this phase. Noted for backlog awareness; does not gate Phase 9. |
| src/App.tsx | 563-586 (`handleCardSetTriggerKey`, macOS path) | `unbind_hotkey` runs and mutates persisted state before the replacement `bind_hotkey` is confirmed to succeed; on conflict, the user's prior hotkey binding is permanently lost | ℹ️ Info (out of Phase 9 scope) | Real defect (09-REVIEW.md CR-02, carried forward unchanged across multiple review passes), but `handleCardSetTriggerKey` was not touched by any Phase 9 plan — pre-existing Phase 8 (hotkey rebind, UX-11) issue. Noted for backlog awareness; does not gate Phase 9. |
| src-tauri/src/lib.rs, src/App.tsx | various | Remaining Warning/Info items from 09-REVIEW.md (interval validation, error-message mislabeling, editing-state cleanup, toast timer cleanup, profile name sanitization asymmetry, startup load silent failure, Promise.all fragility, unhandled-rejection risk, dead loading signal, IPC naming/platform-detection/UUID-discard quality items, and WR-09's noted-but-accepted widening of the scheduler's single-task stall radius from HoldStart now also being an awaited send) | ⚠️ Warning / ℹ️ Info | Real but unrelated to Phase 9's core parallel-execution scope (pre-existing UX/Phase-8/persistence-layer/quality concerns, or an already-accepted, documented trade-off); listed for completeness only, not gating this phase. |

No TBD/FIXME/XXX debt markers found in phase-9-modified files.

## 5. Manual macOS Device Test (EXEC-01)

*Unchanged from the prior verification pass — restored verbatim from plan 09-03's original artifact (commit `e1cecde`). Still pending human execution.*

Pre-flight:

- [ ] Accessibility granted to AutoMux (System Settings → Privacy & Security → Accessibility).
- [ ] Input Monitoring granted to AutoMux (System Settings → Privacy & Security → Input Monitoring).
- [ ] App is running via `npm run tauri dev` or a release build.

### Test T9.1 — Enabling macro B while macro A is firing does not affect A (maps to ROADMAP SC1)

1. Create macro A: "Click A" with action = Left Click, interval = 200ms.
2. Create macro B: "Click B" with action = Right Click, interval = 300ms.
3. Enable A via its toggle. **Expected:** A's card shows a pulsing green firing dot.
4. Wait ~1 second (A fires ~5 times).
5. Enable B via its toggle. **Expected:** B's card also shows a pulsing firing dot, AND A's card keeps its pulsing dot and keeps firing at its own 200ms rate.
6. **Expected outcome:** both macros fire concurrently at their own configured intervals.

### Test T9.2 — Stopping macro A does not affect macro B (maps to ROADMAP SC3)

1. Continue from T9.1 with both A and B enabled and firing.
2. Disable A via its toggle. **Expected:** A's card dot goes dim/disabled.
3. **Expected:** B's card keeps its pulsing dot and continues firing at 300ms.

### Test T9.3 — Two macros with the same input both fire concurrently, and the Phase 8 conflict warning shows (maps to ROADMAP SC1 + Phase 8 conflict detection)

1. Create macro A and macro B, both as Left Click actions (same input), each enabled.
2. **Expected:** both cards show pulsing firing dots AND the Phase 8 same-input conflict warning region appears.

### Test T9.4 — Hold-mode macro shows "held", not "firing"

1. Create a Hold-mode macro, enable it with a matching/Global target and the engine on.
2. **Expected:** the card shows the static accent "held" dot, NOT the pulsing green "firing" dot.

### Test T9.5 — Hold-mode macro under simulated load actually holds the input

1. With several other macros firing at short intervals (e.g., 3 macros at 5-10ms each) to pressure `action_tx`, enable a Hold-mode macro.
2. **Expected:** the card shows "held" AND the configured mouse button/key is genuinely, physically held down in a test text field or game. This is the device-level confirmation that the phantom-held gap (closed by 09-06 at the unit-test level) also holds on a real device under genuine OS-scheduling pressure.

## 6. Manual Windows Device Test (EXEC-02)

*Unchanged from the prior verification pass. Still pending human execution.*

Pre-flight:

- [ ] The app is running (release build or `npm run tauri dev` from a Windows host).
- [ ] If Windows SmartScreen or UAC blocks input injection, run as Administrator or add an exception in the antivirus.

### Test 6.1 — Enabling macro B while macro A is firing does not affect A (maps to ROADMAP SC2)

1. Create macro A: "Click A" with action = Left Click, interval = 200ms.
2. Create macro B: "Click B" with action = Right Click, interval = 300ms.
3. Enable A. **Expected:** pulsing green firing dot.
4. Wait ~1s. Enable B. **Expected:** B also pulses; A keeps firing at 200ms unaffected.

### Test 6.2 — Stopping macro A does not affect macro B (maps to ROADMAP SC3)

1. Continue from 6.1 with both firing. Disable A. **Expected:** A dims; B keeps pulsing at 300ms.

### Test 6.3 — Two macros with the same input both fire concurrently, and the conflict warning shows (maps to ROADMAP SC2 + Phase 8 conflict detection)

1. Create macro A and macro B, both Left Click, each enabled. **Expected:** both pulse; conflict warning appears.

### Human Verification Required

### 1. macOS device tests (T9.1-T9.5)

**Test:** Run the documented macOS manual tests on a real macOS host with Accessibility + Input Monitoring granted.
**Expected:** Both macro cards visibly pulse concurrently at independent rates; disabling one leaves the other unaffected; same-input pair both fire while the Phase 8 conflict warning also displays; a Hold-mode macro shows the static held dot and genuinely holds the input under load.
**Why human:** Requires live CGEvent injection and real-time UI observation on physical/virtual macOS hardware.

### 2. Windows device tests (6.1-6.3)

**Test:** Run the 3 documented Windows manual tests on a real Windows host.
**Expected:** Same concurrent-firing and independent-stop behavior via SendInput.
**Why human:** Requires a real Windows device.

### Gaps Summary

**No gaps remain.** The one Blocker-severity gap carried forward from the previous verification pass — `Scheduler::start_macro`'s `HoldStart` send being fire-and-forget `try_send`, asymmetric with the already-hardened `release_holds` path — is now closed by plan 09-06. I independently confirmed this against current source (not 09-06-SUMMARY.md's claims alone): the `.await` send is present at the correct site, `holds.push` is correctly ordered after it, the `@safety-officer` comment cites the correct gap, the RED test genuinely fails at the pre-fix commit (independently reproduced via a disposable `git worktree`) and genuinely passes at HEAD, and the full 13-test suite, clippy, and release build are all clean on independent re-run.

Two Critical issues were surfaced by a fresh code-review pass performed this session (`09-REVIEW.md`): a new `running_configs` staleness bug (dormant — no frontend call site reaches it) and the carried-forward macOS hotkey-rebind-ordering bug (pre-existing from Phase 8). Both are real and worth tracking, but neither touches this phase's parallel-execution or hold-lifecycle-reliability scope, so neither blocks Phase 9's goal achievement per this verification's scope.

The three ROADMAP success criteria (SC1, SC2, SC3) still require real-device confirmation on macOS and Windows that has not yet been performed — the manual test scripts exist (T9.1-T9.5, 6.1-6.3) but their checkboxes remain unchecked. This is why overall status is `human_needed` rather than `passed`.

**Recommended next step:** Execute the device-level manual tests (T9.1-T9.5 on macOS, 6.1-6.3 on Windows) to close out the ROADMAP success criteria with real-device evidence. Separately, consider filing the two out-of-scope Critical issues from `09-REVIEW.md` (running_configs staleness, hotkey-rebind ordering) as tracked backlog items — neither needs to block Phase 9 from being considered functionally complete at the scheduler-reliability level, since both predate this phase and neither is reachable/relevant through the parallel-execution paths this phase modifies.

---

_Verified: 2026-07-21T20:15:00Z_
_Verifier: Claude (gsd-verifier)_
