---
phase: 03-macro-setup-ux
verified: 2026-05-30T00:00:00Z
status: gaps_found
score: 8/11 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Macro card key badge is clickable; clicking it enters capture mode for that macro on macOS — card edit auto-commits via unbind_hotkey then bind_hotkey on macOS AND set_macro_trigger_key on Windows"
    status: failed
    reason: "CR-02: handleCardSetTriggerKey on macOS calls unbind_hotkey + bind_hotkey but never calls set_macro_trigger_key. MacroConfig.trigger_key remains stale in StateActor. Consequence: (1) state broadcast emits the old keycode, so the card badge shows the old key after editing; (2) reevaluate_all_macros rebuilds MACRO_TRIGGER_KEYS from the stale trigger_key, leaving the old key as a live ghost hotkey. The fix requires a third invoke('set_macro_trigger_key', ...) call in the IS_MACOS branch."
    artifacts:
      - path: "src/App.tsx"
        issue: "handleCardSetTriggerKey (lines 280-289) IS_MACOS branch omits invoke('set_macro_trigger_key'). PLAN 02 must_have explicitly states: 'Card key edit auto-commits via unbind_hotkey then bind_hotkey on macOS; set_macro_trigger_key on Windows' — the 'on Windows' constraint is met, the macOS trigger_key persistence is missing."
    missing:
      - "Add invoke('set_macro_trigger_key', { id, trigger_key: nativeCode }) after invoke('bind_hotkey') in the IS_MACOS branch of handleCardSetTriggerKey"

  - truth: "domKeycodeToNative returns 0 for unmapped keys — callers commit nativeCode=0 as trigger key (silently maps to CGKeyCode A on macOS)"
    status: failed
    reason: "CR-03: domKeycodeToNative returns 0 on a cache miss (keymap.ts line 182). In startCapture, the returned nativeCode is passed directly to onCommit with no null/zero guard. Any key not in the lookup table (numpad, PrintScreen, international keys) silently assigns CGKeyCode 0 (letter A) or VK 0 (undefined) as the trigger key. The user sees resolveKeyName(0) = 'A' with no indication capture failed."
    artifacts:
      - path: "src/keymap.ts"
        issue: "domKeycodeToNative returns 0 for unrecognised codes (line 182: map[code] ?? 0). Zero is a valid CGKeyCode (A) on macOS, making failure indistinguishable from success."
      - path: "src/App.tsx"
        issue: "startCapture (lines 258-277): const nativeCode = domKeycodeToNative(e.code); onCommit(nativeCode); — no guard for nativeCode === 0."
    missing:
      - "Change domKeycodeToNative return type to number | null, returning null on miss"
      - "Add guard in startCapture: if (nativeCode === null || nativeCode === 0) return; before onCommit(nativeCode)"

  - truth: "No UI path to add a trigger key to an existing key-less macro via the card (WR-02)"
    status: failed
    reason: "The card key badge section is wrapped in <Show when={macro.trigger_key !== null}> (App.tsx line 779). A macro created without a trigger key has no interactive element in the card to assign one. The only path is to delete and recreate the macro. PLAN 02 must_have states the badge is clickable — but only when trigger_key is already set. This is a functional gap against the UX-01 requirement that users can configure key bindings via the UI."
    artifacts:
      - path: "src/App.tsx"
        issue: "Line 779: <Show when={macro.trigger_key !== null}> gates the entire key editing area. No fallback renders a clickable placeholder when trigger_key is null."
    missing:
      - "Add a fallback to the Show that renders a clickable 'Set key…' placeholder when trigger_key is null, allowing users to assign a key to an existing macro without recreating it"
---

# Phase 3: Macro Setup UX — Verification Report

**Phase Goal:** Users can configure key bindings and target applications without knowing raw system codes or exact process name strings
**Verified:** 2026-05-30
**Status:** gaps_found (3 blockers)
**Re-verification:** No — initial verification

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
| 10 | Macro card key badge is clickable; card edit auto-commits — macOS: trigger_key persisted | FAILED | handleCardSetTriggerKey (lines 280-289): IS_MACOS branch calls unbind_hotkey + bind_hotkey but NOT set_macro_trigger_key. trigger_key in MacroConfig is never updated; state broadcast carries old key; MACRO_TRIGGER_KEYS retains ghost trigger (CR-02) |
| 11 | domKeycodeToNative does not silently assign a valid key for unrecognised e.code strings | FAILED | keymap.ts line 182 returns 0 on miss; startCapture has no guard; CGKeyCode 0 = letter A on macOS; unmapped key silently becomes trigger A (CR-03) |

**Score:** 9/11 truths verified (truths 10 and 11 FAILED)

Note: A third gap is present beyond these two truths — WR-02 (no UI path to assign a trigger key to an existing key-less macro card) — which constitutes a functional gap against UX-01 requirement coverage even though it is not expressed as a separate truth in the PLAN frontmatter.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/keymap.ts` | DOM keycode → native int mapping + display name resolution | VERIFIED | File exists, 197 lines, four lookup tables, two exported functions, platform detection |
| `src-tauri/src/ipc/mod.rs` | RunningApp struct, list_running_apps, set_macro_trigger_key | VERIFIED | All three present (lines 113, 123, 142); #[command] decorated |
| `src-tauri/src/state/mod.rs` | SetMacroTriggerKey Intent variant and handler | VERIFIED | Variant at line 132; handler at lines 323-329 |
| `src-tauri/src/lib.rs` | Updated invoke_handler | VERIFIED | Lines 101-102 register both new commands |
| `src/App.tsx` | Key capture widget, card badge click-to-edit, process picker | PARTIAL | Widget and picker fully implemented; card key edit has CR-02 (missing set_macro_trigger_key on macOS) and WR-02 (no path to add key to key-less macro) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| ipc/mod.rs list_running_apps | platform/macos/observer.rs list_running_apps_impl | cfg block | WIRED | ipc/mod.rs line 126: crate::platform::macos::observer::list_running_apps_impl() |
| ipc/mod.rs set_macro_trigger_key | state/mod.rs Intent::SetMacroTriggerKey | send_intent | WIRED | ipc/mod.rs line 148; state/mod.rs handler lines 323-329 |
| state/mod.rs SetMacroTriggerKey handler | platform/windows/update_macro_trigger_keys | reevaluate_all_macros | WIRED | reevaluate_all_macros lines 466-474 builds trigger_keys HashMap and calls update_macro_trigger_keys on both platforms |
| App.tsx key capture widget onClick | startCapture function | closure | WIRED | App.tsx line 646: startCapture((nativeCode) => setNewMacroTriggerKeyCode(nativeCode)) |
| App.tsx card key badge onClick | unbind_hotkey + bind_hotkey (macOS only) | handleCardSetTriggerKey | PARTIAL | macOS branch wired to bind_hotkey but NOT to set_macro_trigger_key — ghost trigger bug (CR-02) |
| App.tsx startCapture | domKeycodeToNative from keymap.ts | import | WIRED | Line 5 imports domKeycodeToNative; used at line 269 in startCapture |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| App.tsx key capture widget | newMacroTriggerKeyCode | domKeycodeToNative(e.code) in startCapture | Partially — returns 0 for unmapped keys, which is a valid CGKeyCode | STATIC for unmapped keys (CR-03) |
| App.tsx process picker | apps | invoke("list_running_apps") in handlePickerFocus | Real — NSWorkspace.runningApplications() / EnumWindows backed | FLOWING |
| App.tsx card key badge display | macro.trigger_key | StateActor broadcast via state-changed event | Stale on macOS card-edit (never updated by handleCardSetTriggerKey on macOS) | HOLLOW for macOS card-edit (CR-02) |

### Behavioral Spot-Checks

Step 7b: SKIPPED — no runnable Tauri app without the full build environment. TypeScript and Rust compilation checks serve as the available automated gate.

### Probe Execution

Step 7c: No probe scripts found in scripts/*/tests/probe-*.sh. SKIPPED.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| UX-01 | 03-01, 03-02 | Key Code field replaced with key capture widget — raw CGKeyCode/VK integers never shown | PARTIAL | Widget present; resolveKeyName used for display. BLOCKED by CR-02 (macOS card edit leaves stale trigger_key) and WR-02 (no path to assign key to key-less macro). BLOCKED by CR-03 (unmapped key silently assigns CGKeyCode 0 = A). |
| UX-02 | 03-01, 03-03 | macOS target app field replaced with running-process picker showing app name + bundle ID | VERIFIED | select#select-macro-target populated by list_running_apps; NSWorkspace impl returns display_name + bundleIdentifier |
| UX-03 | 03-01, 03-03 | Windows target app field replaced with running-process picker showing process name and path | VERIFIED | Same picker; EnumWindows impl returns basename as display_name + exe path as identifier |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src/App.tsx | 280-289 | IS_MACOS branch calls bind_hotkey but not set_macro_trigger_key | BLOCKER | Ghost trigger hotkey on macOS; stale UI badge after card-edit (CR-02) |
| src/keymap.ts | 182 | Returns 0 for unrecognised e.code — valid CGKeyCode on macOS | BLOCKER | Unmapped keys silently assigned as trigger A; user receives no feedback (CR-03) |
| src/App.tsx | 779 | `<Show when={macro.trigger_key !== null}>` gates entire key edit area with no fallback | WARNING | No UI path to assign a trigger key to an existing key-less macro (WR-02) |
| src-tauri/src/platform/windows/mod.rs | 279 | OpenProcess HANDLE never closed via CloseHandle | WARNING | Continuous resource leak on every foreground-window change and every list_running_apps call (CR-01 — Windows only, does not block macOS builds or the primary UX flow) |
| src-tauri/src/platform/windows/mod.rs | 511-515 | GetMessageW infinite loop on Win32 error (BOOL(-1) treated as continue) | WARNING | CPU spin on message pump error; Windows only (CR-04) |

No TBD / FIXME / XXX debt markers found in phase-modified files.

### Human Verification Required

#### 1. macOS Key Capture End-to-End

**Test:** On macOS, create a new macro, click the key capture widget, press F5, verify the badge shows "F5" (not "Key 96"), submit the form, then reopen the app and verify the trigger key is persisted correctly.
**Expected:** Badge shows "F5"; trigger fires correctly after restart.
**Why human:** Requires running the Tauri app on macOS hardware.

#### 2. macOS Card-Edit Trigger Key (After CR-02 Fix)

**Test:** Once CR-02 is fixed, click the key badge on an existing macro card on macOS, press a key, verify: (a) badge immediately updates to the new key name; (b) the old key no longer triggers the macro; (c) after app restart the new key is still bound.
**Expected:** Badge updates, old ghost trigger gone, binding persists.
**Why human:** Requires macOS runtime; verifies both HOTKEY_BINDINGS and MacroConfig.trigger_key are in sync.

#### 3. Process Picker Populates on Focus

**Test:** Open the creation form, click into the target-app dropdown. Verify it shows running apps by display name and no bundle IDs or exe paths appear as placeholder text.
**Expected:** Dropdown shows app names (e.g., "Safari", "Finder"); each option's identifier is in parentheses.
**Why human:** Requires running app with live NSWorkspace / EnumWindows data.

### Gaps Summary

Three gaps block the phase goal:

**BLOCKER 1 — CR-02 (macOS card-edit trigger key not persisted):** `handleCardSetTriggerKey` on macOS registers the hotkey in `HOTKEY_BINDINGS` but never calls `set_macro_trigger_key`. `MacroConfig.trigger_key` remains stale in the StateActor, so: (a) the state broadcast carries the old keycode, meaning the card badge shows the wrong key after editing; (b) `reevaluate_all_macros` re-registers the old key as the active trigger, leaving a ghost hotkey the user cannot see or remove. Fix: add a third `invoke("set_macro_trigger_key", ...)` call in the IS_MACOS branch of `handleCardSetTriggerKey`.

**BLOCKER 2 — CR-03 (domKeycodeToNative returns 0 for unrecognised keys):** Any key not in the lookup table returns `0`. On macOS `CGKeyCode 0 = A`, so the user pressing an international key, numpad key, or PrintScreen silently sets the trigger to the letter A. There is no feedback. Fix: return `null` from `domKeycodeToNative` on a miss and add a guard in `startCapture` to skip `onCommit` when `nativeCode` is null or 0.

**BLOCKER 3 — WR-02 (no UI path to assign a trigger key to an existing key-less macro):** The card key area is entirely hidden when `macro.trigger_key === null`. A macro created without a trigger key can only have one assigned by deleting and recreating it. The UX-01 requirement states users can configure key bindings; this gap makes it impossible for most existing macros (created before Phase 3). Fix: render a clickable placeholder in the `<Show>` fallback.

The two Windows-specific issues (CR-01 HANDLE leak, CR-04 GetMessageW loop) are pre-existing bugs in the observer infrastructure that affect macOS builds only superficially (they are cfg-gated). They are WARNING severity for this verification and do not block the macOS-primary UX goal from being verified, but they must be fixed before a Windows release.

---

_Verified: 2026-05-30_
_Verifier: Claude (gsd-verifier)_
