---
phase: 01-reliability-safety
verified: 2026-05-16T14:00:00Z
status: human_needed
score: 6/6 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Grant macOS Accessibility permission while AutoMux is running — confirm prompt clears within ~3s without restart"
    expected: "The accessibility denied banner disappears within approximately 3 seconds of granting permission in System Settings"
    why_human: "Requires a live macOS session with the app running; cannot simulate the OS permission grant programmatically"
  - test: "Run a macro continuously for 30+ minutes on macOS, then press the hotkey"
    expected: "The hotkey still toggles the macro — CGEventTap did not silently deactivate"
    why_human: "Requires a live long-running macOS session; CGEventTap timeout behavior cannot be triggered in static analysis"
  - test: "On Windows: start a macro that holds a key, press Ctrl+Shift+Q, then type into a text field"
    expected: "No keys remain stuck; the text field receives clean keystrokes immediately"
    why_human: "Requires a live Windows session; cannot verify OS-level key state in static analysis"
  - test: "Create a macro, edit its name and interval, close AutoMux without using the Profiles tab, reopen"
    expected: "All changes are present on reopen — macro is there with the modified interval"
    why_human: "Requires live app execution with actual disk writes and restarts"
  - test: "Toggle engine off, wait for auto-save, restart AutoMux"
    expected: "Engine state restored to off on launch — no one-way ratchet"
    why_human: "Requires live app execution to verify engine_active restoration from disk"
---

# Phase 1: Reliability & Safety Verification Report

**Phase Goal:** Macros fire reliably — permissions are detected without restart, hotkeys survive long sessions, no work is lost on close, and the app cannot panic during execution
**Verified:** 2026-05-16T14:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth                                                                                                      | Status     | Evidence                                                                                                               |
|----|------------------------------------------------------------------------------------------------------------|------------|------------------------------------------------------------------------------------------------------------------------|
| T1 | Accessibility grant detected within ~3s without restart (RELY-01)                                          | ✓ VERIFIED | `src/App.tsx:133` — `setInterval(..., 3000)` confirmed; no `10000` remains; `onCleanup` preserved                     |
| T2 | Scheduler silently clamps sub-5ms intervals to 5ms at both sites (SAFE-03)                                 | ✓ VERIFIED | `scheduler/mod.rs:66` — `interval_ms.max(5)`; `scheduler/mod.rs:184` — `new_ms.max(5)`; zero `.max(1)` remain        |
| T3 | macOS CGEventSource construction failure drops the action silently, no panic (SAFE-01 macOS)               | ✓ VERIFIED | `platform/macos/input.rs` — `fn source() -> Option<CGEventSource>` with `.ok()`; 4 callers use `let Some(source) = Self::source() else { return; }` |
| T4 | CGEventTap inline re-enable on OS timeout, fully silent, hotkeys survive long sessions (RELY-02)           | ✓ VERIFIED | `observer.rs:214` — `tap_proxy` param; `observer.rs:222–232` — `TapDisabledByTimeout | TapDisabledByUserInput` branch is FIRST statement; calls `CGEventTapEnable` via FFI cast, returns `None` |
| T5 | Windows mutex lock failure on injection hot path drops action silently, no panic (SAFE-01 Windows)         | ✓ VERIFIED | `platform/windows/mod.rs` — exactly 4 occurrences of `let Ok(mut guard) = get_held_inputs().lock() else { return; }`; zero hot-path `lock().unwrap()` remain (only out-of-scope line 51) |
| T6 | Windows Ctrl+Shift+Q flushes all held inputs synchronously before exit (RELY-03)                          | ✓ VERIFIED | `platform/windows/mod.rs:374` — `WindowsInputProvider::flush_all_held_inputs()` inserted between `try_send(TriggerEmergencyStop)` and `process::exit(1)` |
| T7 | Macro mutations persist to default profile without explicit save (RELY-04)                                 | ✓ VERIFIED | `state/mod.rs:289,297,304,311,356,377` — `auto_save_default().await` appended to AddMacro, RemoveMacro, SetMacroEnabled, SetMacroTargetApp, UpdateSequence, UpdateStepInterval |
| T8 | Startup uses single Intent::LoadProfile; suppresses per-macro writes via loading_profile flag (D-06)       | ✓ VERIFIED | `lib.rs:39-40` — `startup_tx.send(Intent::LoadProfile("default"))` is the only send; `state/mod.rs:109-110` — `#[serde(skip)] pub loading_profile: bool`; `state/mod.rs:390,409` — flag set true/false around batch |
| T9 | IPC load_profile routes through Intent::LoadProfile (one write after batch) (D-05)                        | ✓ VERIFIED | `ipc/mod.rs:236` — `state.send_intent(Intent::LoadProfile(name.clone()))` present; all 4 State annotations updated to `Arc<crate::persistence::ProfileManager>` |
| T10 | auto-save-error Tauri event emitted on save failure (D-07)                                                | ✓ VERIFIED | `state/mod.rs:469-474` — `self.app_handle.emit("auto-save-error", e.to_string())`; also in LoadProfile Err branch at `state/mod.rs:405` |
| T11 | engine_active unconditionally restored from profile — no one-way ratchet (D-06, Pitfall 6)               | ✓ VERIFIED | `state/mod.rs:400` — `self.state.engine_active = profile.engine_active;` unconditional; zero occurrences of `if profile.engine_active {` in file |

**Score:** 6/6 ROADMAP success criteria verified (11/11 plan truths verified)

### Required Artifacts

| Artifact                                          | Expected                                                                | Status     | Details                                                                                                    |
|---------------------------------------------------|-------------------------------------------------------------------------|------------|------------------------------------------------------------------------------------------------------------|
| `src/App.tsx`                                     | Accessibility poll at 3000ms                                            | ✓ VERIFIED | Line 133: `}, 3000);` confirmed; no `10000` present                                                        |
| `src-tauri/src/scheduler/mod.rs`                  | 5ms minimum interval floor                                              | ✓ VERIFIED | Lines 66 and 184: `interval_ms.max(5)` and `new_ms.max(5)`                                                |
| `src-tauri/src/platform/macos/input.rs`           | Non-panicking CGEventSource construction                                | ✓ VERIFIED | `fn source() -> Option<CGEventSource>` at line 25; `.ok()` body; 4 callers with `let Some` guard          |
| `src-tauri/src/platform/macos/observer.rs`        | CGEventTap timeout re-enable in callback                                | ✓ VERIFIED | `tap_proxy` at line 214; `TapDisabledByTimeout | TapDisabledByUserInput` match at lines 222-232            |
| `src-tauri/src/platform/windows/mod.rs`           | Non-panicking held_inputs locks + synchronous emergency-stop flush      | ✓ VERIFIED | 4 `let Ok(mut guard) = get_held_inputs().lock() else { return; }` at lines 174,194,207,224; flush at 374  |
| `src-tauri/src/state/mod.rs`                      | AppState.loading_profile, Intent::LoadProfile, auto_save_default, etc. | ✓ VERIFIED | All fields/variants/methods present and substantive                                                        |
| `src-tauri/src/lib.rs`                            | Arc<ProfileManager> wiring; startup via Intent::LoadProfile             | ✓ VERIFIED | `Arc::new(ProfileManager...)` at line 29; single `Intent::LoadProfile("default")` send at line 40         |
| `src-tauri/src/ipc/mod.rs`                        | Arc<ProfileManager> managed-state; load_profile routes via intent       | ✓ VERIFIED | 4 occurrences of `State<'_, Arc<crate::persistence::ProfileManager>>`; `Intent::LoadProfile` at line 236  |

### Key Link Verification

| From                                                              | To                                  | Via                                       | Status     | Details                                                             |
|-------------------------------------------------------------------|-------------------------------------|-------------------------------------------|------------|---------------------------------------------------------------------|
| `src/App.tsx` createEffect                                        | `invoke('check_accessibility')`     | `setInterval` at 3000ms                   | ✓ WIRED    | Line 133 confirms the 3000ms literal                               |
| `scheduler/mod.rs IntervalTask::new`                              | `Duration::from_millis`             | `.max(5)` clamp                           | ✓ WIRED    | Line 66: `interval_ms.max(5)` directly feeds `Duration::from_millis` |
| `scheduler/mod.rs UpdateInterval handler`                         | `Duration::from_millis`             | `new_ms.max(5)` clamp                     | ✓ WIRED    | Line 184: `new_ms.max(5)` confirmed                                |
| `MacInputProvider::source()`                                      | inject_key/inject_mouse_click/inject_mouse_move/inject_mouse_button_raw | `let Some(source) = Self::source() else { return; }` | ✓ WIRED | All 4 inject methods have the guard |
| `CGEventTap callback` (observer.rs line 214)                      | `CGEventTapEnable(tap_proxy as CFMachPortRef, true)` | `TapDisabledByTimeout | TapDisabledByUserInput` early branch | ✓ WIRED | Branch at lines 222-232 is first statement in closure |
| `WindowsInputProvider inject_*` (lines 174,194,207,224)           | `get_held_inputs().lock()`          | `let Ok(...) else { return; }`            | ✓ WIRED    | 4 sites confirmed with `grep -c`                                   |
| `hook_callback emergency-stop branch`                             | `WindowsInputProvider::flush_all_held_inputs()` | synchronous call before `process::exit`  | ✓ WIRED    | Line 374: call between try_send (370) and exit(1) (375)            |
| `StateActor::handle_intent mutating arms`                         | `ProfileManager::save_profile`      | `self.auto_save_default().await`          | ✓ WIRED    | 6 arms confirmed at lines 289,297,304,311,356,377                  |
| `Intent::LoadProfile handler`                                     | `AppState.loading_profile` guard + `auto_save_default` at end | set true/false bracket | ✓ WIRED | Lines 390 and 409; line 410 has single write |
| `lib.rs startup spawn block`                                      | `Intent::LoadProfile("default")`    | single intent send                        | ✓ WIRED    | Lines 39-40; no remaining `Intent::AddMacro` in startup            |
| `ipc::load_profile`                                               | `Arc<ProfileManager>`               | `State<'_, Arc<...>>`                     | ✓ WIRED    | All 4 IPC profile commands updated; no bare ProfileManager remains  |

### Data-Flow Trace (Level 4)

N/A — this phase delivers behavior changes (not rendering pipelines). The relevant data flows are through Rust async channels and Tauri events, not JSX rendering props. Key data flows verified:

- `auto_save_default()` reads `self.state.macros.clone()` and `self.state.engine_active` (live state, not hardcoded)
- `Intent::LoadProfile` populates `self.state.macros` from `profile_mgr.load_profile().await` result
- `setInterval` poll feeds `setAccessibility(ok)` from `invoke<boolean>("check_accessibility")`

### Behavioral Spot-Checks

Step 7b: SKIPPED — the app is a Tauri desktop application with no runnable standalone entry points. The macOS CGEventTap and Windows hook callbacks require a live desktop session. The Rust scheduler and StateActor require the full Tauri runtime. Static build checks (via `cargo build`) were performed by the executor and recorded in SUMMARY files.

### Probe Execution

No probe scripts found in the project (`find scripts -name "probe-*.sh"` yields nothing). No probes declared in PLAN files.

### Requirements Coverage

| Requirement | Source Plan | Description                                                          | Status       | Evidence                                                                 |
|-------------|------------|----------------------------------------------------------------------|--------------|--------------------------------------------------------------------------|
| RELY-01     | 01-01-PLAN | macOS permissions detected within ~3s without restart                | ✓ SATISFIED  | App.tsx:133 — `setInterval(..., 3000)`                                   |
| RELY-02     | 01-02-PLAN | macOS CGEventTap re-enables on OS timeout                            | ✓ SATISFIED  | observer.rs:222-232 — TapDisabledByTimeout branch with CGEventTapEnable  |
| RELY-03     | 01-03-PLAN | Windows emergency stop flushes held inputs before exit               | ✓ SATISFIED  | windows/mod.rs:374 — synchronous flush before process::exit              |
| RELY-04     | 01-04-PLAN | Macro changes auto-saved; no work lost on restart                    | ✓ SATISFIED  | 6 mutating intents call `auto_save_default`; startup uses LoadProfile    |
| SAFE-01     | 01-02-PLAN (macOS), 01-03-PLAN (Windows) | All 5 panic sites on injection hot path replaced | ✓ SATISFIED | macos/input.rs: Option<CGEventSource>; windows/mod.rs: 4 let-Ok-else sites |
| SAFE-03     | 01-01-PLAN | 5ms minimum interval floor enforced                                  | ✓ SATISFIED  | scheduler/mod.rs:66,184 — `.max(5)` at both sites                        |

**Orphaned requirements from REQUIREMENTS.md mapped to Phase 1:** None — all 6 are covered by the 4 plans.

### Anti-Patterns Found

No TBD, FIXME, or XXX markers in any modified file. No unreferenced debt markers. Scanned all 8 files modified by this phase.

The code review (01-REVIEW.md, already committed) identified several issues found AFTER the phase was executed. These are documented below as they bear on the phase goal assessment:

| File | Finding | Severity | Impact on Phase Goal |
|------|---------|----------|---------------------|
| `observer.rs:75-125` | CR-01: `flush_held_inputs` holds registry lock while posting CGEvents — deadlock risk on macOS emergency stop | Warning for phase goal | Does not block RELY-02 (tap re-enable); affects macOS emergency stop path. The flush_held_inputs deadlock pre-exists Phase 1; Phase 1 did not claim to fix flush_held_inputs internals. |
| `state/mod.rs:334-339` | WR-01: `ToggleMacroHotkey` missing `auto_save_default` — hotkey-toggled state lost on restart | Warning | RELY-04 must-have truth specifies "creates, edits, deletes, enables/disables, retargets, or modifies the sequence/interval" — hotkey-toggle is NOT in this list. This is a gap beyond the stated truth, not a failure of it. |
| `state/mod.rs:411` + `state/mod.rs:194` | WR-02: `Intent::LoadProfile` calls `broadcast_state()` explicitly AND `run()` calls it unconditionally — double event per load | Warning | Does not break RELY-04 correctness; causes double frontend re-render. |
| `windows/mod.rs:137-156` | CR-05: `send_mouse_move` passes raw pixel coords as MOUSEEVENTF_ABSOLUTE units — wrong position | Info (pre-existing) | Not addressed by Phase 1; RELY-03 covers emergency stop, not mouse precision. |
| `ipc/mod.rs:225-241` | CR-06: `load_profile` reads profile twice — potential state inconsistency window | Warning | RELY-04 must-have truth is about auto-save after mutations, not the profile-load error path. The double-read is a quality issue, not a failure of the stated truth. |

**Debt marker gate:** PASSED — zero unresolved TBD/FIXME/XXX markers in modified files.

### Human Verification Required

#### 1. Accessibility Permission Live Grant (RELY-01)

**Test:** On macOS, launch AutoMux without Accessibility permission. See the denied prompt. Go to System Settings > Privacy & Security > Accessibility. Grant AutoMux permission. Return to AutoMux within a few seconds.
**Expected:** The permission denied banner clears within approximately 3 seconds. The user does not need to restart the app. No "Re-check" button is required.
**Why human:** Requires a live macOS session. The OS-level permission grant cannot be simulated programmatically.

#### 2. 30-Minute CGEventTap Survival (RELY-02)

**Test:** On macOS, configure a macro with a global hotkey. Leave AutoMux running for 30+ minutes with no hotkey presses. Then press the hotkey.
**Expected:** The macro toggles on. The hotkey is still live — the CGEventTap did not silently deactivate due to a timeout event.
**Why human:** Requires a sustained live session. The CGEventTap timeout cannot be triggered reliably in a short test.

#### 3. Windows Emergency Stop Key Release (RELY-03)

**Test:** On Windows, configure a macro with a SustainedHold step (e.g., hold right mouse button). Start the macro. While the button is being held, press Ctrl+Shift+Q. After the app exits, type into a text field or check key state.
**Expected:** No keys or mouse buttons remain physically stuck. The OS receives the release events before process exit.
**Why human:** Requires a live Windows session. OS key-state verification requires manual inspection.

#### 4. Auto-Save Roundtrip (RELY-04 — create-edit-close-reopen)

**Test:** Launch AutoMux. Create a new macro. Edit its name or step interval. Close AutoMux without clicking anything in the Profiles tab. Reopen AutoMux.
**Expected:** The macro exists with all edits intact.
**Why human:** Requires live app execution with actual disk writes and process restarts.

#### 5. Engine-Active Restore (RELY-04 — one-way ratchet prevention)

**Test:** Launch AutoMux. Toggle the engine off. Wait a moment (auto-save fires). Quit AutoMux. Reopen AutoMux.
**Expected:** The engine is still off. The engine_active state is restored to false from the saved profile — not coerced to true on load.
**Why human:** Requires live app execution to confirm the disk state is read and applied correctly.

### Gaps Summary

No blockers identified. All 6 ROADMAP success criteria have verified code implementations. The human verification items cover behaviors that cannot be confirmed through static analysis alone — they are execution-time behaviors requiring live OS sessions.

The code review report (01-REVIEW.md) documents post-implementation findings including CR-01 (deadlock risk in macOS flush_held_inputs), WR-01 (ToggleMacroHotkey missing auto-save), CR-06 (double profile read). None of these were in scope for Phase 1's stated must-have truths, and none prevent the phase goal from being met by the implemented changes. They are candidates for Phase 2 or later planning.

---

_Verified: 2026-05-16T14:00:00Z_
_Verifier: Claude (gsd-verifier)_
