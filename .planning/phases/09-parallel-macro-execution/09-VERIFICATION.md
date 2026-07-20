---
phase: 09-parallel-macro-execution
verified: 2026-07-20T18:30:00Z
status: gaps_found
score: 4/7 must-haves verified
behavior_unverified: 3
overrides_applied: 0
gaps:
  - truth: "A macro whose sequence contains only SustainedHold steps shows a distinct held indicator; a macro with both SustainedHold and InterleavedInterval steps shows a single combined indicator (09-02 D-02)"
    status: failed
    reason: "computeRunningState() derives held/combined/firing purely from the persisted macro.sequence.steps shape, but the only currently-wired macro-creation path (handleCreateMacro in src/App.tsx) always stores a single InterleavedInterval step regardless of the selected trigger_mode. The Hold-mode -> SustainedHold conversion happens only ephemerally inside the scheduler (src-tauri/src/scheduler/mod.rs:287-294) at dispatch time and is never written back to persisted AppState (Intent::AddMacro in src-tauri/src/state/mod.rs:448 stores the config exactly as submitted). Net effect: every Hold-mode macro created through the app is injected as a continuous hold at runtime but its card always shows the 'firing' (pulsing green) indicator, never 'held' (accent) — the held/combined branches of computeRunningState are unreachable via the product's only creation path. Independently confirmed by 09-REVIEW.md CR-01 (Critical) and by direct inspection of src/App.tsx:90-107, src/App.tsx:460-472, src-tauri/src/scheduler/mod.rs:274-294, and src-tauri/src/state/mod.rs:448-478 during this verification."
    artifacts:
      - path: "src/App.tsx"
        issue: "computeRunningState() (lines 90-107) does not read macro.trigger_mode; it infers held/firing purely from stored sequence.steps, which never contains SustainedHold for any macro created via the current UI"
      - path: "src-tauri/src/state/mod.rs"
        issue: "Intent::AddMacro (line 448) inserts the submitted MacroConfig verbatim with no Hold-mode step-conversion, so the persisted config never matches what the scheduler actually runs for Hold-mode macros"
    missing:
      - "computeRunningState must also branch on macro.trigger_mode === \"Hold\" (mirroring the scheduler's runtime conversion at scheduler/mod.rs:287-294), OR the Hold->SustainedHold conversion must be persisted back into AppState at macro-creation/update time so the stored steps reflect what actually runs"
deferred: []
behavior_unverified_items:
  - truth: "On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals (ROADMAP SC1)"
    test: "On a real macOS host with Accessibility + Input Monitoring granted: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A, wait ~1s, enable B; observe both macro cards show a pulsing firing dot simultaneously and neither's click rate changes when the other starts/stops"
    expected: "Both macros visibly fire concurrently at their own configured rates on real macOS input injection (CGEvent) — this is documented as 09-VERIFICATION.md (plan-09-03 artifact) Section 5, Test T9.1, still unexecuted (checkbox unchecked)"
    why_human: "Requires a real macOS device with Accessibility/Input Monitoring permissions and live observation of CGEvent injection timing and the per-card UI dots in real time; the scheduler-level unit test (parallel_two_macros_concurrent) proves the platform-agnostic timeline logic but not real-device CGEvent injection behavior"
  - truth: "On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A (ROADMAP SC2)"
    test: "On a real Windows host: create macro A (Left Click, 200ms) and macro B (Right Click, 300ms); enable A, wait ~1s, enable B; observe both fire concurrently via SendInput"
    expected: "Documented as 09-VERIFICATION.md Section 6, Test 6.1, still unexecuted (checkbox unchecked)"
    why_human: "Requires a real Windows device with the Win32 SendInput injection path and Win32 hook observer running live; not exercisable from this (macOS) verification environment"
  - truth: "Stopping one running macro does not affect any other concurrently running macro (ROADMAP SC3)"
    test: "On real macOS and Windows hosts: with A and B both firing, disable A; observe B's dot keeps pulsing and B's fire rate is unaffected"
    expected: "Documented as 09-VERIFICATION.md Sections 5/6, Tests T9.2/6.2, still unexecuted (checkboxes unchecked)"
    why_human: "Same as above — device-level, real-time observation required; scheduler-level unit test (parallel_stop_one_keeps_other) proves the underlying gating logic but not real-device confirmation"
human_verification:
  - test: "Run T9.1-T9.3 on a real macOS host per 09-VERIFICATION.md Section 5 (concurrent firing, stop-one-keeps-other, same-input concurrent + conflict warning)"
    expected: "All 3 tests pass; both macro cards show independent pulsing firing dots; stopping one does not affect the other; same-input pair both fire while the Phase 8 conflict warning also displays"
    why_human: "Real macOS device, live CGEvent injection, live UI observation — cannot be verified by static analysis or from this (non-macOS-GUI) verification session"
  - test: "Run 6.1-6.3 on a real Windows host per 09-VERIFICATION.md Section 6"
    expected: "All 3 tests pass via SendInput injection and the Win32 hook observer"
    why_human: "Requires a real Windows device — not available in this verification environment"
  - test: "After the CR-01 fix lands, create a Hold-mode macro via the UI, enable it, and confirm its card shows the 'held' (accent, non-pulsing) indicator rather than 'firing' (pulsing green)"
    expected: "Hold-mode macros are visually distinguishable from Pulse-mode macros on their card"
    why_human: "Visual confirmation of a UI fix; also gates whether the chosen fix (trigger_mode-based derivation vs. persisting the conversion) produces the intended UX"
---

# Phase 9: Parallel Macro Execution Verification Report

**Phase Goal:** Multiple macros can run simultaneously on both macOS and Windows — triggering a second macro never blocks, queues, or cancels a running one
**Verified:** 2026-07-20T18:30:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Two macros at different intervals both fire concurrently in the scheduler, neither blocking the other (EXEC-01/EXEC-02, plan 09-01) | ✓ VERIFIED | `cargo test parallel_two_macros_concurrent` passes (independently re-run during this verification, exit 0, 1/1 ok as part of full 11/11 suite). Bounded per-macro_id fire-count ranges at src-tauri/src/scheduler/mod.rs:654-758 prove genuine concurrency, not serialization. |
| 2 | Stopping one running macro does not affect another concurrently running macro (EXEC-02, plan 09-01) | ✓ VERIFIED | `cargo test parallel_stop_one_keeps_other` passes (re-run during this verification). Bounded fire-count assertion at src-tauri/src/scheduler/mod.rs:760-869 confirms A's shortened window vs B's full window. |
| 3 | action_tx channel capacity is 1024 (raised from 100) to avoid silent overflow under parallel fire rates | ✓ VERIFIED | `grep 'mpsc::channel::<scheduler::ActionReady>(1024)' src-tauri/src/lib.rs:26` confirmed present; state_tx/sched_tx unchanged at 100. |
| 4 | Debug-only ACTION_DROP_COUNT increments on every try_send failure at all 3 fire sites, readable via get_action_drop_count() and the get_debug_action_drop_count IPC command, absent from release builds | ✓ VERIFIED | Static declared src-tauri/src/scheduler/mod.rs:13, 3 fetch_add sites at lines 313/382/416-ish (grep confirms 4 occurrences: static+3 sites+accessor), `#[cfg(debug_assertions)]` gates confirmed on the static, accessor, and IPC command (src-tauri/src/ipc/mod.rs:227-231); real accessor call confirmed (`crate::scheduler::get_action_drop_count()`, not a literal). |
| 5 | Per-macro card shows firing/waiting/held/combined/disabled derived from computeRunningState mirroring the 3 backend injection gates (D-01, plan 09-02) | ✓ VERIFIED (gates D-01 only) | computeRunningState gates (src/App.tsx:90-107) verified line-for-line against StateActor::handle_action gates (src-tauri/src/state/mod.rs:373-394): engine active + not emergency-stopped, macro enabled, target_app match/Global — logic matches. Wired at 3 call sites in the macro card (App.tsx:1233, 1248, 1253); `npx tsc --noEmit` and `npm run build` both pass. |
| 6 | Held-only macros show a distinct held indicator; hold+interval macros show a combined indicator (D-02, plan 09-02) | ✗ FAILED | computeRunningState never inspects `macro.trigger_mode`; the only wired macro-creation path (handleCreateMacro, App.tsx:460-472) always persists an InterleavedInterval step even when Hold mode is selected. The scheduler's Hold->SustainedHold conversion (scheduler/mod.rs:287-294) is ephemeral and never written back to AppState (state/mod.rs:448 Intent::AddMacro stores the config as submitted). Every Hold-mode macro created via the app shows "firing" instead of "held" — the held/combined branches are unreachable in practice. Independently confirmed by 09-REVIEW.md CR-01 (Critical) and by direct source inspection during this verification. |
| 7 | ROADMAP SC1/SC2/SC3 — concurrent firing and independent stop hold true on real macOS and Windows devices | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Scheduler-level unit tests (truths 1-2 above) prove the platform-agnostic scheduling/gating logic that SC1-3 depend on, but the 6 device-level manual tests (T9.1-T9.3, 6.1-6.3) documented in this same file's prior draft (plan-09-03 deliverable) are still unexecuted — all checkboxes in the original Section 5/6 pre-flight and status table were unchecked/pending. No real macOS or Windows device confirmation has been performed. |

**Score:** 4/7 truths verified (3 present, behavior-unverified; 1 failed)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/scheduler/mod.rs` | 2 new concurrency-proof tests + ACTION_DROP_COUNT counter | ✓ VERIFIED | Both tests present, passing, compiled only under `cfg(test)`. Counter static + 3 fetch_add sites + accessor all present and correctly `#[cfg(debug_assertions)]`-gated. |
| `src-tauri/src/lib.rs` | action_tx capacity 1024, debug-gated IPC registration | ✓ VERIFIED | Confirmed by direct grep. |
| `src-tauri/src/ipc/mod.rs` | get_debug_action_drop_count debug-only command | ✓ VERIFIED | Calls the real accessor, correctly cfg-gated, `cargo build --release` compiles clean (command compiled out). |
| `src/App.tsx` | computeRunningState + RunningState type, wired card rendering | ⚠️ HOLLOW (partial) | Exists, substantive, wired into 3 call sites, builds clean — but the underlying derivation logic has the CR-01 gap (see truth #6). Artifact is present/wired but does not fully deliver its stated D-02 behavior. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `get_debug_action_drop_count` (ipc/mod.rs) | `crate::scheduler::get_action_drop_count()` | direct call | ✓ WIRED | Confirmed — calls the real accessor, not a hardcoded value. |
| Macro card status dot (App.tsx:1233) | `computeRunningState(macro, state())` | switch statement | ✓ WIRED | Dot no longer branches on `macro.enabled` alone; correctly calls computeRunningState. |
| `computeRunningState` | `StateActor::handle_action` gates (state/mod.rs:373-394) | logic mirroring | ⚠️ PARTIAL | Gates 1-3 (engine/enabled/target) mirror correctly. The implicit 4th gate — trigger_mode-driven Hold conversion that the scheduler applies at dispatch (scheduler/mod.rs:287-294) — is NOT mirrored, causing the CR-01 gap. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full lib test suite (re-run independently, not trusted from SUMMARY) | `cd src-tauri && cargo test` | 11 passed; 0 failed (includes both new parallel tests) | ✓ PASS |
| Clippy clean, no new warnings | `cd src-tauri && cargo clippy --all-targets -- -D warnings` | exit 0, zero warnings | ✓ PASS |
| Frontend typecheck | `npx tsc --noEmit -p tsconfig.json` | exit 0, no output | ✓ PASS |
| Frontend production build | `npm run build` | exit 0, bundle built (pre-existing unrelated CSS optimizer warning, not phase-9 code) | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| EXEC-01 | 09-01, 09-02, 09-03 | Multiple macros can run simultaneously on macOS | ⚠️ PARTIAL | Scheduler-level proof verified (unit tests); on-device macOS confirmation not yet performed; UI-visibility half has the CR-01 gap for Hold-mode macros. REQUIREMENTS.md currently marks this "Complete" — that marking is premature given the unresolved gap and un-executed device tests. |
| EXEC-02 | 09-01, 09-02, 09-03 | Multiple macros can run simultaneously on Windows | ⚠️ PARTIAL | Same disposition as EXEC-01 — scheduler logic is platform-agnostic and unit-proven, but no Windows-device confirmation has been performed. REQUIREMENTS.md marks this "Complete" — premature for the same reasons. |

No orphaned requirements found — REQUIREMENTS.md Phase 9 row maps exactly to EXEC-01/EXEC-02, both declared in plan frontmatter.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src/App.tsx | 90-107 | Derivation logic gap (CR-01) — held/combined branch unreachable via the only wired UI creation path | 🛑 Blocker | Directly breaks the D-02 must-have of plan 09-02; misrepresents actual macro runtime behavior to the user |
| src/App.tsx | 1233, 1248, 1253 | `computeRunningState(macro, state()!)` called 3 separate times per card instead of computed once (WR-01 in 09-REVIEW.md) | ⚠️ Warning | Risk of future logic drift between dot color and inline labels; not itself a functional defect today |
| src-tauri/src/scheduler/mod.rs | 654-869 | Both new parallel tests use real wall-clock `sleep` windows with fixed numeric bounds (WR-02 in 09-REVIEW.md) | ⚠️ Warning | Bounded CI-flakiness risk under a starved/throttled runner; not a correctness defect in the current passing run |

No TBD/FIXME/XXX debt markers found in phase-9-modified files.

## 5. Manual macOS Device Test (EXEC-01)

*Restored verbatim from plan 09-03's original artifact (commit `e1cecde`) — the verifier agent's first pass overwrote this file and dropped the step-by-step procedures. Content below is unchanged from what 09-03 produced.*

These steps must be performed on a real macOS host. `parallel_two_macros_concurrent` and `parallel_stop_one_keeps_other` (§1) cover the scheduler-level proof; the device test confirms the end-to-end behavior with real input injection and the plan-09-02 per-card running-state indicators (pulsing green dot = firing; see D-01/D-02 in `09-CONTEXT.md`).

Pre-flight:

- [ ] Accessibility granted to AutoMux (System Settings → Privacy & Security → Accessibility).
- [ ] Input Monitoring granted to AutoMux (System Settings → Privacy & Security → Input Monitoring).
- [ ] App is running via `npm run tauri dev` or a release build.

### Test T9.1 — Enabling macro B while macro A is firing does not affect A (maps to ROADMAP SC1)

> ROADMAP SC1: "On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals."

1. Create macro A: "Click A" with action = Left Click, interval = 200ms.
2. Create macro B: "Click B" with action = Right Click, interval = 300ms.
3. Enable A via its toggle. **Expected:** A's card shows a pulsing green firing dot (plan 09-02 `computeRunningState` "firing" state).
4. Wait ~1 second (A fires ~5 times).
5. Enable B via its toggle. **Expected:** B's card also shows a pulsing firing dot, AND A's card keeps its pulsing dot and keeps firing at its own 200ms rate — A is not paused, delayed, or doubled by B starting.
6. **Expected outcome:** both macros fire concurrently at their own configured intervals.

### Test T9.2 — Stopping macro A does not affect macro B (maps to ROADMAP SC3)

> ROADMAP SC3: "Stopping one running macro does not affect any other concurrently running macro."

1. Continue from T9.1 with both A and B enabled and firing (both cards show pulsing dots).
2. Disable A via its toggle. **Expected:** A's card dot goes dim/disabled (plan 09-02 "disabled" state).
3. **Expected:** B's card keeps its pulsing dot and continues firing at 300ms — stopping A does not pause, delay, or stop B.

### Test T9.3 — Two macros with the same input both fire concurrently, and the Phase 8 conflict warning shows (maps to ROADMAP SC1 + Phase 8 conflict detection)

1. Create macro A and macro B, both as Left Click actions (same input), each enabled.
2. **Expected:** both cards show pulsing firing dots (both actually fire, proving parallel execution is not blocked by input collision) AND the Phase 8 same-input conflict warning region appears (`⚠ 2 macros are injecting the same input…`, unit-covered by Phase 8's `conflict_detection_overlap`).
3. This is the device-level confirmation that parallel firing and the Phase 8 conflict warning coexist correctly — the warning is informational only and does not block either macro from firing.

## 6. Manual Windows Device Test (EXEC-02)

These steps must be performed on a real Windows host. The device test confirms the same concurrent-firing behavior via SendInput injection and the Win32 hook observer.

Pre-flight:

- [ ] The app is running (release build or `npm run tauri dev` from a Windows host).
- [ ] If Windows SmartScreen or UAC blocks input injection, run as Administrator or add an exception in the antivirus.

### Test 6.1 — Enabling macro B while macro A is firing does not affect A (maps to ROADMAP SC2)

> ROADMAP SC2: "On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A."

1. Create macro A: "Click A" with action = Left Click, interval = 200ms.
2. Create macro B: "Click B" with action = Right Click, interval = 300ms.
3. Enable A via its toggle. **Expected:** A's card shows a pulsing green firing dot.
4. Wait ~1 second (A fires ~5 times).
5. Enable B via its toggle. **Expected:** B's card also shows a pulsing firing dot, AND A's card keeps firing at its own 200ms rate, unaffected by B starting.
6. **Expected outcome:** both macros fire concurrently via `SendInput`, at their own configured intervals.

### Test 6.2 — Stopping macro A does not affect macro B (maps to ROADMAP SC3)

1. Continue from Test 6.1 with both A and B enabled and firing.
2. Disable A via its toggle. **Expected:** A's card dot goes dim/disabled.
3. **Expected:** B's card keeps its pulsing dot and continues firing at 300ms — stopping A does not affect B.

### Test 6.3 — Two macros with the same input both fire concurrently, and the conflict warning shows (maps to ROADMAP SC2 + Phase 8 conflict detection)

1. Create macro A and macro B, both as Left Click actions, each enabled.
2. **Expected:** both cards show pulsing firing dots (both actually fire on Windows via `SendInput`) AND the Phase 8 same-input conflict warning region appears.
3. This is the Windows device-level confirmation that parallel firing and the conflict warning coexist correctly.

### Human Verification Required

### 1. macOS device tests (T9.1-T9.3)

**Test:** Run the 3 documented macOS manual tests above (concurrent firing, stop-one-keeps-other, same-input concurrent + conflict warning) on a real macOS host with Accessibility + Input Monitoring granted.
**Expected:** Both macro cards visibly pulse concurrently at independent rates; disabling one leaves the other unaffected; same-input pair both fire while the Phase 8 conflict warning also displays.
**Why human:** Requires live CGEvent injection and real-time UI observation on physical/virtual macOS hardware — not exercisable from this verification session.

### 2. Windows device tests (6.1-6.3)

**Test:** Run the 3 documented Windows manual tests above on a real Windows host.
**Expected:** Same concurrent-firing and independent-stop behavior via SendInput.
**Why human:** Requires a real Windows device.

### 3. Hold-mode UI fix confirmation (post-CR-01 fix)

**Test:** After CR-01 is fixed, create a Hold-mode macro via the UI, enable it, and confirm the card shows the "held" (accent, static) dot rather than "firing" (pulsing green).
**Expected:** Hold-mode macros are visually distinguishable from Pulse-mode macros.
**Why human:** Visual UX confirmation of the fix.

### Gaps Summary

One Blocker gap: `computeRunningState()`'s D-02 "held indicator" deliverable is unreachable for any macro created through the app's current UI, because Hold-mode macros always persist an `InterleavedInterval` step (handleCreateMacro never applies the scheduler's Hold->SustainedHold conversion), so the card always shows "firing" instead of "held" for Hold-mode macros. This was independently found by the code review (09-REVIEW.md CR-01) and confirmed again directly against source during this verification (src/App.tsx:90-107/460-472, src-tauri/src/scheduler/mod.rs:274-294, src-tauri/src/state/mod.rs:448-478). This does not affect the scheduler's underlying concurrency correctness (09-01), which is unit-proven and independently re-verified as passing (11/11 tests, including both new parallel tests). It does affect the phase's UI-visibility deliverable (09-02's own stated must-have D-02) and therefore the phase is not fully done.

Additionally, three ROADMAP success criteria (SC1, SC2, SC3) require real-device confirmation on macOS and Windows that has not yet been performed — the manual test scripts exist (documented in the prior draft of this file, itself a 09-03 deliverable) but their checkboxes remain unchecked. These are captured as human-verification items and do not block the gaps_found status on their own (they would route to human_needed), but combine with the CR-01 blocker to keep the phase at gaps_found overall.

**Recommended next step:** Route the CR-01 fix through `/gsd-plan-phase --gaps` (or a quick follow-up plan) before closing Phase 9. Consider fixing by making `computeRunningState` also branch on `macro.trigger_mode === "Hold"` (mirroring scheduler/mod.rs:287-294), which is the fix already sketched in 09-REVIEW.md's CR-01 section. After the fix lands, re-run this verification and additionally have a human execute the 6 documented device tests before marking EXEC-01/EXEC-02 fully complete in REQUIREMENTS.md.

---

_Verified: 2026-07-20T18:30:00Z_
_Verifier: Claude (gsd-verifier)_
