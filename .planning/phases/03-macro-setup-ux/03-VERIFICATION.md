---
phase: 03-macro-setup-ux
verified: 2026-05-30T15:00:00Z
status: human_needed
score: 11/11 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 8/11
  gaps_closed:
    - "CR-02: handleCardSetTriggerKey IS_MACOS branch now calls invoke('set_macro_trigger_key') after bind_hotkey — MacroConfig.trigger_key is persisted to StateActor"
    - "CR-03: domKeycodeToNative returns number | null (not 0 on miss); startCapture guards nativeCode === null before onCommit — unmapped keys no longer silently assign CGKeyCode 0"
    - "WR-02: outer <Show when={macro.trigger_key !== null}> now has a fallback= prop rendering a clickable 'Set key...' placeholder for key-less macro cards"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "On macOS, create a new macro, click the key capture widget, press F5, verify badge shows 'F5' (not 'Key 96'), submit the form, reopen the app, verify trigger key persisted"
    expected: "Badge shows 'F5'; trigger fires correctly after restart"
    why_human: "Requires running the Tauri app on macOS hardware"
  - test: "Click the key badge on an existing macro card on macOS, press a key, verify: (a) badge immediately updates to new key name; (b) old key no longer triggers the macro; (c) after app restart the new key is still bound"
    expected: "Badge updates, old ghost trigger gone, binding persists across restart"
    why_human: "Requires macOS runtime; verifies both HOTKEY_BINDINGS and MacroConfig.trigger_key are in sync"
  - test: "Create a macro without setting a trigger key. Verify that a dashed 'Set key...' placeholder appears on the macro card. Click it, press a key, verify the badge updates to show the new key name."
    expected: "Dashed placeholder visible; capture works; badge shows key name after commit"
    why_human: "Requires running Tauri app to verify reactive Show/fallback re-evaluation after trigger_key changes"
  - test: "Open the creation form, click into the target-app dropdown. Verify it shows running apps by display name; no bundle IDs or exe paths appear as placeholder text."
    expected: "Dropdown shows app names (e.g., 'Safari', 'Finder'); each option's identifier is in parentheses"
    why_human: "Requires running app with live NSWorkspace / EnumWindows data"
---

# Phase 3: Macro Setup UX — Re-Verification Report (after gap-closure plan 03-04)

**Phase Goal:** Deliver a polished key-binding and app-targeting UX so users can configure macros without touching the terminal.
**Verified:** 2026-05-30
**Status:** human_needed — all automated checks pass; runtime verification on macOS hardware required
**Re-verification:** Yes — after gap-closure plan 03-04 closed CR-02, CR-03, WR-02

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | src/keymap.ts exports domKeycodeToNative and resolveKeyName functions used by App.tsx | VERIFIED | File exists at src/keymap.ts; both functions exported (lines 180, 193); App.tsx imports both at line 5 |
| 2 | list_running_apps IPC command returns RunningApp[] on macOS and Windows | VERIFIED | ipc/mod.rs lines 123-136; cfg-gated macOS (observer::list_running_apps_impl) and Windows (windows::list_running_apps_impl) implementations present |
| 3 | set_macro_trigger_key IPC command updates trigger_key on MacroConfig via Intent channel | VERIFIED | ipc/mod.rs lines 142-151; Intent::SetMacroTriggerKey handler in state/mod.rs lines 323-329 mutates mac.trigger_key and calls reevaluate_all_macros |
| 4 | Both new commands are registered in lib.rs invoke_handler | VERIFIED | lib.rs lines 101-102: ipc::list_running_apps and ipc::set_macro_trigger_key present in generate_handler! |
| 5 | Creation form trigger key field is a click-to-capture widget, not an input type=number | VERIFIED | App.tsx lines 630-670: role="button" div with triggerKeyRecording signal, accent border/glow in recording state, resolveKeyName display, cancel affordance |
| 6 | Captured key shows human-readable name; Escape exits without committing | VERIFIED | startCapture (lines 250-277): resolveKeyName called in render; Escape branch at line 261 calls setTriggerKeyRecording(false) and removes listener without calling onCommit |
| 7 | Creation form target app field is a select dropdown populated from list_running_apps | VERIFIED | App.tsx lines 595-616: select#select-macro-target with onFocus={handlePickerFocus}, For loop over apps(), loading/error Show blocks, Global sentinel option |
| 8 | Process picker shows Global sentinel + display name + identifier format; fetches on open with in-progress guard | VERIFIED | handlePickerFocus (lines 296-308): guard if (appsLoading()) return; re-fetches every open; option format "{display_name} ({identifier})" in For loop |
| 9 | Macro card target display is clickable; inline picker auto-commits; Escape collapses | VERIFIED | App.tsx lines 736-776: clickable span sets editingCardId/editingField + calls handlePickerFocus; inline select onChange calls handleCardSetTargetApp; onKeyDown Escape handler |
| 10 | Pressing an unmapped key during capture does not commit any key — capture stays active | VERIFIED | keymap.ts line 180: signature `number \| null`; line 182: `map[code] ?? null`; App.tsx line 270: `if (nativeCode === null) return;` — guard returns without calling onCommit or removing listener, so capture stays alive |
| 11 | macOS card-edit key badge commits trigger_key to StateActor AND updates hotkey binding; key-less macro cards show clickable 'Set key...' placeholder | VERIFIED | handleCardSetTriggerKey IS_MACOS branch (App.tsx lines 283-286): three sequential awaited invokes — unbind_hotkey, bind_hotkey, set_macro_trigger_key; card Show fallback (lines 781-793): fallback= prop with dashed-border span containing onClick that calls startCapture |

**Score:** 11/11 truths verified

### Deviation from Plan Acceptance Criteria — Acceptable

The PLAN for 03-04 specified the guard `if (nativeCode === null || nativeCode === 0) return;`. The implementation uses only `if (nativeCode === null) return;`. This is not a defect: the original fear was that `0` came from a miss, but after changing `domKeycodeToNative` to return `null` on miss, `0` is only returned for `"KeyA"` (a valid CGKeyCode). Blocking `0` would prevent the user from assigning the letter A as a trigger key. The implementation is strictly correct; the plan's acceptance criterion was overconstrained. TypeScript compiles cleanly (`npx tsc --noEmit` → exit 0).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/keymap.ts` | domKeycodeToNative returns `number \| null`; null on miss | VERIFIED | Line 180: `export function domKeycodeToNative(code: string): number \| null`; line 182: `return map[code] ?? null;` |
| `src/App.tsx` | null guard in startCapture; three-invoke IS_MACOS branch; Show fallback with 'Set key...' | VERIFIED | Line 270: null guard; lines 283-286: three invokes; line 790: 'Set key...' fallback span |
| `src-tauri/src/ipc/mod.rs` | RunningApp struct, list_running_apps, set_macro_trigger_key | VERIFIED | All three present (lines 113, 123, 142); #[command] decorated |
| `src-tauri/src/state/mod.rs` | SetMacroTriggerKey Intent variant and handler | VERIFIED | Variant at line 132; handler at lines 323-329 |
| `src-tauri/src/lib.rs` | Updated invoke_handler | VERIFIED | Lines 101-102 register both new commands |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| App.tsx startCapture | domKeycodeToNative | null guard before onCommit | WIRED | Line 269: `const nativeCode = domKeycodeToNative(e.code);` → line 270: `if (nativeCode === null) return;` → line 271: `onCommit(nativeCode)` |
| App.tsx handleCardSetTriggerKey IS_MACOS branch | invoke set_macro_trigger_key | third invoke after bind_hotkey | WIRED | Line 286: `await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode })` inside `if (IS_MACOS)` block |
| App.tsx card Show fallback | startCapture + handleCardSetTriggerKey | onClick on 'Set key...' span | WIRED | Line 785-789: onClick calls setEditingCardId, setEditingField("key"), startCapture with handleCardSetTriggerKey callback |
| ipc/mod.rs list_running_apps | platform/macos/observer.rs list_running_apps_impl | cfg block | WIRED | ipc/mod.rs line 126: crate::platform::macos::observer::list_running_apps_impl() |
| ipc/mod.rs set_macro_trigger_key | state/mod.rs Intent::SetMacroTriggerKey | send_intent | WIRED | ipc/mod.rs line 148; state/mod.rs handler lines 323-329 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| App.tsx key capture widget | nativeCode | domKeycodeToNative(e.code) in startCapture | Yes — null on miss; valid keycode or null only; null-gated before onCommit | FLOWING |
| App.tsx process picker | apps | invoke("list_running_apps") in handlePickerFocus | Real — NSWorkspace.runningApplications() / EnumWindows backed | FLOWING |
| App.tsx card key badge display | macro.trigger_key | StateActor broadcast via state-changed event; set_macro_trigger_key now called on macOS card-edit | Now flows correctly — set_macro_trigger_key in IS_MACOS branch ensures StateActor emits new keycode | FLOWING |
| App.tsx card Show fallback | macro.trigger_key === null | Reactive derivation from state-changed event data | Reactive — Show re-evaluates when trigger_key changes post-commit | FLOWING |

### Behavioral Spot-Checks

Step 7b: SKIPPED — no runnable Tauri app without the full build environment. TypeScript type-check (`npx tsc --noEmit` → exit 0) is the available automated gate and passes.

### Probe Execution

Step 7c: No probe scripts found in scripts/*/tests/probe-*.sh. SKIPPED.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| UX-01 | 03-01, 03-02, 03-04 | Key Code field replaced with key capture widget — raw CGKeyCode/VK integers never shown to user | VERIFIED (code) | Widget present; resolveKeyName used for display; null guard prevents unmapped-key commits; macOS card-edit persists trigger_key via three-invoke path; key-less cards have 'Set key...' fallback. Human verification required for runtime confirmation. |
| UX-02 | 03-01, 03-03 | macOS target app field replaced with running-process picker showing app name + bundle ID | VERIFIED | select#select-macro-target populated by list_running_apps; NSWorkspace impl returns display_name + bundleIdentifier |
| UX-03 | 03-01, 03-03 | Windows target app field replaced with running-process picker showing process name and path | VERIFIED | Same picker; EnumWindows impl returns basename as display_name + exe path as identifier |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src-tauri/src/platform/windows/mod.rs | 279 | OpenProcess HANDLE never closed via CloseHandle | WARNING | Continuous resource leak on every foreground-window change and every list_running_apps call (CR-01 — Windows only, does not block macOS builds or the primary UX flow). Pre-existing; not introduced by 03-04. |
| src-tauri/src/platform/windows/mod.rs | 511-515 | GetMessageW infinite loop on Win32 error (BOOL(-1) treated as continue) | WARNING | CPU spin on message pump error; Windows only (CR-04). Pre-existing; not introduced by 03-04. |

No TBD / FIXME / XXX debt markers found in phase-modified files.

### Human Verification Required

#### 1. macOS Key Capture End-to-End

**Test:** On macOS, create a new macro, click the key capture widget, press F5, verify the badge shows "F5" (not "Key 96"), submit the form, then reopen the app and verify the trigger key is persisted correctly.
**Expected:** Badge shows "F5"; trigger fires correctly after restart.
**Why human:** Requires running the Tauri app on macOS hardware.

#### 2. macOS Card-Edit Trigger Key (CR-02 Fix Confirmation)

**Test:** Click the key badge on an existing macro card on macOS, press a key, verify: (a) badge immediately updates to the new key name; (b) the old key no longer triggers the macro; (c) after app restart the new key is still bound.
**Expected:** Badge updates, old ghost trigger gone, binding persists.
**Why human:** Requires macOS runtime; verifies both HOTKEY_BINDINGS and MacroConfig.trigger_key are in sync.

#### 3. 'Set key...' Fallback on Key-less Macro Card (WR-02 Fix Confirmation)

**Test:** Create a macro without setting a trigger key. Verify that a dashed-border "Set key..." placeholder appears on the macro card. Click it, press a key, verify the badge updates to the new key name.
**Expected:** Dashed placeholder visible; capture commits; badge shows the assigned key name.
**Why human:** Requires running Tauri app to observe reactive Show/fallback re-evaluation after trigger_key changes from null to a value.

#### 4. Process Picker Populates on Focus

**Test:** Open the creation form, click into the target-app dropdown. Verify it shows running apps by display name; no raw bundle IDs or exe paths appear as placeholder text.
**Expected:** Dropdown shows app names (e.g., "Safari", "Finder"); each option's identifier appears in parentheses.
**Why human:** Requires running app with live NSWorkspace / EnumWindows data.

### Gaps Summary

No automated gaps remain. All three blockers from the initial verification (CR-02, CR-03, WR-02) are confirmed closed in the codebase:

- **CR-02 closed:** `handleCardSetTriggerKey` IS_MACOS branch now executes three sequential invokes — `unbind_hotkey`, `bind_hotkey`, `set_macro_trigger_key` — ensuring MacroConfig.trigger_key in StateActor is updated and the subsequent state broadcast carries the new keycode.
- **CR-03 closed:** `domKeycodeToNative` returns `number | null` with `map[code] ?? null`; `startCapture` guards `if (nativeCode === null) return;` before `onCommit`. Note: the guard omits `|| nativeCode === 0` relative to the plan's acceptance criteria — this is intentionally correct because `0` now exclusively maps to `"KeyA"` (CGKeyCode A is valid); blocking it would prevent A from being set as a trigger.
- **WR-02 closed:** The outer `<Show when={macro.trigger_key !== null}>` has a `fallback=` prop (lines 781-793) rendering a dashed-border "Set key..." span wired to `startCapture` + `handleCardSetTriggerKey`.

The two pre-existing Windows-specific warnings (CR-01 HANDLE leak, CR-04 GetMessageW loop) remain open but are out of scope for Phase 3 UX verification and do not block the phase goal on macOS.

Status is `human_needed` because runtime confirmation on macOS hardware is required to close the loop on the three behavioral checks above.

---

_Verified: 2026-05-30_
_Verifier: Claude (gsd-verifier)_
