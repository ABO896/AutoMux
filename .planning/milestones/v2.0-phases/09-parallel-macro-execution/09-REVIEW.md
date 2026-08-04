---
phase: 09-parallel-macro-execution
reviewed: 2026-07-22T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/platform/macos/observer.rs
  - src-tauri/src/platform/windows/mod.rs
  - src-tauri/src/scheduler/mod.rs
  - src-tauri/src/state/mod.rs
  - src/App.tsx
findings:
  critical: 2
  warning: 9
  info: 10
  total: 21
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-07-22T00:00:00Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

This review supersedes the previous `09-REVIEW.md` in this directory. That prior pass found
two Critical gaps — `HoldRelease` actions being gated identically to new-input actions
(stuck-input risk) and a hotkey-rebind flow that destroyed a macro's working binding via an
unconditional `unbind_hotkey` pre-step. Both are **confirmed fixed** in the current source:
`action_should_inject` (`state/mod.rs:281-309`) now unconditionally bypasses Gate 1/2/3 for
`HoldRelease`, backed by a dedicated regression test
(`hold_release_bypasses_gates`, `state/mod.rs:1291-1367`), and `handleCardSetTriggerKey`
(`App.tsx:578-606`) no longer calls `unbind_hotkey` before `bind_hotkey`.

Re-reviewing the current code surfaced two **new** Critical issues not covered by any
existing test:

1. `Intent::LoadProfile`'s error path clears `state.macros` unconditionally *before* the disk
   read, and never restores it on failure — a failed profile load (bad name, missing/corrupt
   file, transient I/O error) silently wipes the user's current macros both in memory and on
   disk (the trailing unconditional `auto_save_default()` persists the now-empty state to
   `default.json`), and the UI visibly loses the macro list because `broadcast_state()` fires
   unconditionally after every intent.
2. The Windows low-level keyboard hook (`hook_callback`) never checks the `LLKHF_INJECTED`
   flag on incoming key events, unlike the macOS tap (which explicitly gates on
   `EVENT_SOURCE_USER_DATA == LLMHF_INJECTED`). A macro's own injected keystrokes on Windows
   can therefore match a configured hotkey or the hardcoded Ctrl+Shift+Q emergency-stop combo
   and self-trigger macro toggles or emergency stop.

Several Warning- and Info-level issues remain from the fixed-gap surface area (redundant
double registry rebuilds, a cross-platform modifier-matching asymmetry, dead code, an
overly-broad frontend error handler) — listed below.

## Critical Issues

### CR-01: `LoadProfile` failure wipes current macros in memory and on disk

**File:** `src-tauri/src/state/mod.rs:800-845`

**Issue:** The `Intent::LoadProfile` handler unconditionally clears `state.macros` and sends
`StopAll` **before** attempting the disk read:

```rust
Intent::LoadProfile(name, reply) => {
    // 1. Clear existing macros and stop all scheduler tasks
    self.state.macros.clear();
    let _ = self.scheduler_tx.send(SchedulerIntent::StopAll).await;
    self.state.loading_profile = true;
    match self.profile_mgr.load_profile(&name).await {
        Ok(profile) => { /* repopulate state.macros ... */ }
        Err(e) => {
            let _ = self.app_handle.emit("auto-save-error", e.to_string());
            let _ = reply.send(Err(e));
            // state.macros is NEVER restored here
        }
    }
    self.state.loading_profile = false;
    self.auto_save_default().await;   // <-- always runs
}
```

If `profile_mgr.load_profile(&name)` returns `Err` (missing/renamed/corrupted profile file,
a stale entry in the profiles list, a transient disk I/O error, etc.), the `Err` branch never
restores `state.macros`. Execution then falls through to the unconditional
`self.auto_save_default().await` at the end, which serializes the now-**empty**
`state.macros` and overwrites `default.json` on disk. Because the outer `run()` loop
(`state/mod.rs:431-451`) calls `self.broadcast_state()` unconditionally after every
`handle_intent`, the frontend also immediately receives and renders the emptied `AppState` —
a single failed "Load Profile" action visibly destroys every configured macro, and the
on-disk autosave snapshot is destroyed too, with only a transient "Load failed: ..." toast as
the surviving evidence anything went wrong.

**Fix:** Only mutate `state.macros` / send `StopAll` after the profile has been read
successfully; leave existing state untouched on `Err`:

```rust
Intent::LoadProfile(name, reply) => {
    self.state.loading_profile = true;
    match self.profile_mgr.load_profile(&name).await {
        Ok(profile) => {
            self.state.macros.clear();
            let _ = self.scheduler_tx.send(SchedulerIntent::StopAll).await;
            for (_, config) in profile.macros.clone() {
                self.state.macros.insert(config.id, config);
            }
            self.state.engine_active = profile.engine_active;
            self.reevaluate_all_macros().await;
            self.recompute_conflicts();
            let _ = reply.send(Ok(profile));
        }
        Err(e) => {
            use tauri::Emitter;
            let _ = self.app_handle.emit("auto-save-error", e.to_string());
            let _ = reply.send(Err(e));
        }
    }
    self.state.loading_profile = false;
    self.auto_save_default().await;
}
```

### CR-02: Windows keyboard hook does not filter injected events — self-triggering hotkeys / emergency stop

**File:** `src-tauri/src/platform/windows/mod.rs:493-540`

**Issue:** `hook_callback` reads `kb_struct.vkCode` but never inspects `kb_struct.flags` for
the `LLKHF_INJECTED` bit that Windows sets on `SendInput`-originated `WM_KEYDOWN` events
observed by a `WH_KEYBOARD_LL` hook:

```rust
unsafe extern "system" fn hook_callback(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode >= 0 {
        let kb_struct = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let msg_id = wparam.0 as u32;
        if msg_id == WM_KEYDOWN || msg_id == WM_SYSKEYDOWN {
            let keycode = kb_struct.vkCode as u16;
            // Emergency stop check: Ctrl + Shift + Q  — no injected-event guard
            ...
            // CONFIGURABLE HOTKEYS — no injected-event guard
            if let Ok(bindings) = get_hotkey_bindings().try_lock() { ... }
        }
    }
    CallNextHookEx(None, ncode, wparam, lparam)
}
```

Compare with the macOS tap (`platform/macos/observer.rs:279-315`), which explicitly checks
`event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA) == LLMHF_INJECTED` and
returns early — before both the emergency-stop check and the hotkey-matching loop — for any
event AutoMux itself injected.

On Windows, if a macro's action sequence presses a key whose (keycode, modifier) pair matches
a *different* macro's configured trigger hotkey (or even its own), the macro's own
`SendInput`-injected keystroke passes through this hook indistinguishably from a real
keypress and will toggle that other macro (`ToggleMacroHotkey`) or, combined with a
genuinely-held Ctrl/Shift, could fire the hardcoded Ctrl+Shift+Q emergency stop. This
self-triggering feedback loop has no equivalent on macOS and directly undermines the
project's stated core value that "execution must be accurate."

**Fix:** Check the injected flag and skip both the emergency-stop check and hotkey matching
for injected events, mirroring the macOS gate:

```rust
if msg_id == WM_KEYDOWN || msg_id == WM_SYSKEYDOWN {
    const LLKHF_INJECTED: u32 = 0x10;
    if kb_struct.flags.0 & LLKHF_INJECTED != 0 {
        return CallNextHookEx(None, ncode, wparam, lparam);
    }
    let keycode = kb_struct.vkCode as u16;
    ...
}
```
(Adjust the exact field access to match the `windows` crate's `KBDLLHOOKSTRUCT.flags` type —
it may require `.0` or a `.contains(...)` call depending on crate version.)

## Warnings

### WR-01: Cross-platform hotkey modifier-matching asymmetry

**File:** `src-tauri/src/platform/macos/observer.rs:48-53` vs
`src-tauri/src/platform/windows/mod.rs:524-536`

**Issue:** macOS matches a hotkey binding with a bitwise subset check — extra, unrelated
modifiers held at the same time are ignored:

```rust
fn matches(&self, keycode: u16, flags: CGEventFlags) -> bool {
    keycode == self.keycode && (flags.bits() & self.modifiers) == self.modifiers
}
```

Windows requires an exact bitmask match:

```rust
if binding.keycode == keycode && binding.modifiers == mod_mask {
```

A macro bound to `Shift+F5` fires on macOS even if the user is also holding `Ctrl` (extra
bits ignored), but will **not** fire on Windows under the same physical key combination
(`mod_mask` includes the extra `Ctrl` bit, breaking exact equality). The same configured
hotkey behaves differently across the two supported platforms.

**Fix:** Pick one semantics and apply it on both platforms — the exact-match (Windows)
behavior is likely the intended one (avoids accidental cross-triggering); change the macOS
`matches()` to `flags.bits() == self.modifiers` (after masking off any non-modifier bits
`CGEventFlags` may carry), or explicitly document/test the asymmetry if it is deliberate.

### WR-02: `AddMacro` silently drops a conflicting trigger key with no user feedback

**File:** `src-tauri/src/state/mod.rs:540-547`

**Issue:** `Intent::AddMacro` pre-checks the new macro's `trigger_key` against existing
bindings and, on conflict, silently clears it (`config.trigger_key = None;
config.trigger_modifiers = 0;`) before inserting — the reply oneshot always sends
`Ok(new_id)`. This is exactly the "silent coerce-to-None/0" anti-pattern the codebase
explicitly diagnosed and fixed for `BindHotkey`/`SetMacroTriggerKey` (see those handlers'
doc comments: "the old behavior silently coerced a conflicting key to None/0 with no reply —
this destroyed a working binding with zero user feedback"). Creating a new macro with a
colliding hotkey has the same silent-loss behavior today: the macro is created but the
hotkey the user just set is dropped, and the `ConflictErrorToast` never fires for this path.

**Fix:** Route `AddMacro`'s trigger-key conflict through the same `Result`-carrying pattern
as `BindHotkey`/`SetMacroTriggerKey` — either reject the whole `add_macro` call with a
conflict message, or create the macro without the trigger key and return a warning string
alongside the id so the frontend can surface the same toast used for `bind_hotkey` conflicts.

### WR-03: `RemoveMacro` does not rebuild the platform hotkey-bindings registry

**File:** `src-tauri/src/state/mod.rs:559-570`

**Issue:** Every other state-mutating `Intent` handler calls `self.reevaluate_all_macros()`,
which unconditionally rebuilds the platform `HOTKEY_BINDINGS` registry via
`build_hotkey_bindings_vec(&self.state.macros)`. `Intent::RemoveMacro` does not — it only
removes the macro from `state.macros` and sends `SchedulerIntent::StopMacro`. If the removed
macro had a bound trigger key, that binding remains registered in the platform static
registry (referencing a now-nonexistent macro id) until the next unrelated mutation happens
to rebuild it. Pressing that stale hotkey in the meantime results in a harmless no-op
`ToggleMacroHotkey` dispatch, but it breaks the "every mutating intent refreshes the
registry" invariant the rest of the file establishes.

**Fix:** Add `self.reevaluate_all_macros().await;` to the `RemoveMacro` handler, consistent
with every other mutating intent.

### WR-04: `TriggerEmergencyStop` leaves `state.conflicts` stale

**File:** `src-tauri/src/state/mod.rs:716-727`

**Issue:** `Intent::TriggerEmergencyStop` disables every macro (`mac.enabled = false` for
all) but does not call `self.recompute_conflicts()`, unlike `Intent::ResetEmergencyStop`
just below it, which explicitly does so ("the conflict field must reflect the new
enabled/disabled state"). Since `recompute_conflicts` only considers `enabled` macros, every
conflict group should disappear after an emergency stop — but because it is never
recomputed, `state.conflicts` (and the frontend's "N macros are injecting the same input"
warning card) keeps showing the pre-stop conflict set even though the engine and every macro
are now off.

**Fix:** Add `self.recompute_conflicts();` to `TriggerEmergencyStop`, mirroring
`ResetEmergencyStop`.

### WR-05: `handleCreateMacro` mislabels every `add_macro` failure as a hotkey conflict

**File:** `src/App.tsx:494-508`

**Issue:**
```js
} catch (e) {
  const msg = String(e);
  const macroMatch = msg.match(/is already assigned to "([^"]+)"/);
  const macroName = macroMatch ? macroMatch[1] : "another macro";
  const keyLabel = triggerKey !== null ? resolveKeyName(triggerKey) : "Key";
  showConflictError(keyLabel, macroName);
  console.error("Failed to create macro:", e);
}
```
Unlike `handleCardSetTriggerKey` (`App.tsx:588-600`), which correctly only shows the conflict
toast `if (macroMatch)`, `handleCreateMacro` calls `showConflictError` unconditionally for
*any* thrown error from `add_macro` — including unrelated failures (closed IPC channel, a
`serde` deserialization error, etc.) — falling back to a generic `"another macro"` conflict
message that is actively misleading for non-conflict failures.

**Fix:** Mirror the guard already used in `handleCardSetTriggerKey`:
```js
} catch (e) {
  const msg = String(e);
  const macroMatch = msg.match(/is already assigned to "([^"]+)"/);
  if (macroMatch) {
    const keyLabel = triggerKey !== null ? resolveKeyName(triggerKey) : "Key";
    showConflictError(keyLabel, macroMatch[1]);
  } else {
    console.error("Failed to create macro:", e);
  }
}
```

### WR-06: Redundant, racy double IPC round-trip when setting a card's trigger key on macOS

**File:** `src/App.tsx:578-586`

**Issue:**
```js
if (IS_MACOS) {
  await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers });
  await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
} else {
  await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
}
```
Both `bind_hotkey` (`Intent::BindHotkey`) and `set_macro_trigger_key`
(`Intent::SetMacroTriggerKey`) perform the same conflict-check-and-mutate operation on the
same macro's trigger fields, each independently triggering `reevaluate_all_macros`,
`recompute_conflicts`, `auto_save_default`, and `broadcast_state`. Beyond the redundant work
(double disk write, double state-changed event), there is a race window between the two
awaited calls: if a third party binds the same key to a different macro in between them, the
first call (`bind_hotkey`) succeeds and mutates state, but the second call
(`set_macro_trigger_key`) then fails with a conflict — the `catch` block shows a "hotkey
already bound" toast even though the bind from the first call already took effect, leaving
the toast inconsistent with the actual backend state.

**Fix:** Use a single call on macOS — `bind_hotkey` alone already performs the full
conflict-checked mutation and persists `trigger_key`/`trigger_modifiers`
(`state/mod.rs:669-672`); drop the trailing `set_macro_trigger_key` call.

### WR-07: `HoldRelease`'s unconditional gate bypass can inject a release with no matching injected press

**File:** `src-tauri/src/state/mod.rs:281-309`, `src-tauri/src/scheduler/mod.rs:315-346`

**Issue:** `Scheduler::start_macro` records a `SustainedHold` input into `active_holds` as
soon as the `HoldStart` `ActionReady` message is *sent* to `action_tx` (line 316-324),
regardless of whether `StateActor::action_should_inject` ultimately allows the injection when
the message is later processed — Gate 1/2/3 are re-evaluated at delivery time, not at send
time. If engine state or target-app matching changes in the window between the Scheduler
enqueuing `HoldStart` and the StateActor processing it, the physical key/button press can be
gated out (never injected) while `active_holds` still records the macro as "held." When that
macro later stops, `release_holds`/`stop_macro` sends `HoldRelease`, which — per the
intentional, documented bypass in `action_should_inject` (lines 288-291) — is *always*
injected unconditionally. The result is a physically-injected key-up/mouse-up event with no
preceding physically-injected key-down for that cycle: the mirror-image gap of the
stuck-input bug the release bypass exists to fix.

**Fix:** Either (a) have the StateActor record a hold as "physically down" (and thus eligible
for the unconditional release bypass) only at the moment `HoldStart` actually clears the
gates — moving `active_holds`-equivalent bookkeeping into the StateActor rather than the
Scheduler — or (b) explicitly document this residual edge case next to the
`action_should_inject` doc comment so it is not mistaken for full coverage of the hold
lifecycle.

### WR-08: Redundant hotkey-registry rebuild in `BindHotkey`/`UnbindHotkey` handlers

**File:** `src-tauri/src/state/mod.rs:673-686, 701-710`

**Issue:** `Intent::BindHotkey` and `Intent::UnbindHotkey` both explicitly call
`build_hotkey_bindings_vec(&self.state.macros)` / `update_hotkey_bindings(...)` and then
immediately call `self.reevaluate_all_macros().await`, which — per its own doc comment
("this is the sole registry-refresh site... independent of engine on/off state",
`state/mod.rs:849-857`) — unconditionally performs the exact same rebuild again at the top of
its body. Every bind/unbind rebuilds and replaces the platform registry twice for no
behavioral difference.

**Fix:** Since `reevaluate_all_macros` is documented as the sole registry-refresh site,
remove the now-redundant explicit rebuild block from both handlers and rely solely on the
call to `reevaluate_all_macros()`.

### WR-09: Profile-name sanitization asymmetry between `save_profile` and `load_profile`/`delete_profile`

**File:** `src-tauri/src/ipc/mod.rs:312-341` vs `:349-375`

**Issue:** `save_profile` explicitly calls `ProfileManager::sanitize_name(&name)` before use
(CR-03: "the stored `ProfileData.name` always matches the filename stem"). `load_profile` and
`delete_profile` forward the raw `name` parameter straight to `profile_mgr.load_profile(&name)`
/ `profile_mgr.delete_profile(&name)` with no local sanitization. `persistence.rs` is outside
this review's file set, so it cannot be confirmed here whether those methods sanitize
internally, but the asymmetry visible in this file is itself a maintainability and
defense-in-depth gap — a reader cannot tell from `ipc/mod.rs` alone whether a value like
`load_profile("../../../Library/Preferences/foo")` is safe.

**Fix:** Either apply `ProfileManager::sanitize_name` consistently to all three commands in
`ipc/mod.rs`, or add a comment at `load_profile`/`delete_profile` pointing to where
`persistence.rs` performs the equivalent sanitization, so the trust boundary is explicit and
auditable from this file.

## Info

### IN-01: Interval `"0"` silently becomes 100ms instead of the backend's 5ms floor

**File:** `src/App.tsx:456`

**Issue:** `const interval = parseInt(newMacroInterval()) || 100;` — because `0` is falsy in
JS, a user who explicitly enters `0` gets `100` instead (the Rust scheduler would otherwise
clamp `0` to its 5ms floor via `interval_ms.max(5)`), silently substituting a different value
with no indication to the user.

**Fix:** Use `Number.isNaN(parsed) ? 100 : parsed` instead of `||` so a legitimate `0` is
preserved and the backend's `.max(5)` floor applies.

### IN-02: `inject_mouse_click` appears unused by the actual injection path

**File:** `src-tauri/src/platform/windows/mod.rs:218-244`

**Issue:** `StateActor::inject_input` (`state/mod.rs:496-524`) only ever calls
`inject_mouse_button_raw` (for both interval clicks and sustained holds) — it never calls
`InputProvider::inject_mouse_click`. Within the reviewed files this method has no caller.

**Fix:** Confirm whether it is genuinely unused crate-wide and either wire it into a real
call site or remove it.

### IN-03: Stray blank lines / missing `Default` impl in `WindowsInputProvider`

**File:** `src-tauri/src/platform/windows/mod.rs:59-65`

**Issue:** `WindowsInputProvider::new()` is followed by two blank lines before
`flush_all_held_inputs`, suggestive of removed code left behind. `new()` also takes no
arguments (`clippy::new_without_default` pattern) with no accompanying `Default` impl.

**Fix:** Remove the stray blank lines; optionally add `impl Default for WindowsInputProvider`.

### IN-04: Stale doc comment on `AddMacro`'s trigger-key pre-check

**File:** `src-tauri/src/state/mod.rs:533-539`

**Issue:** The comment says "The full `Result<(), String>` error path is shipped in plan
08-03 (`Intent::BindHotkey`) — this drop-on-conflict is the interim defense-in-depth," but
`AddMacro` was never migrated off that interim behavior (see WR-02). The comment reads as
though this is temporary, but it is currently the permanent behavior.

**Fix:** Update or remove the comment once WR-02 is resolved.

### IN-05: Dead `HotkeyAction::ToggleEngine` variant

**File:** `src-tauri/src/platform/macos/observer.rs:34-38, 415-417`

**Issue:** `HotkeyAction::ToggleEngine` and its `match` arm in the tap callback are never
reachable — `build_hotkey_bindings_vec` (`state/mod.rs:367-382`) only ever constructs
`HotkeyAction::ToggleMacro(mac.id)`. No code path in the reviewed files lets a user bind a
hotkey to the global engine toggle.

**Fix:** Either wire up an IPC/UI path to register a `ToggleEngine` binding, or remove the
dead variant/arm until that feature exists.

### IN-06: Unused `add_hotkey_binding` / `remove_hotkey_bindings_for` functions

**File:** `src-tauri/src/platform/macos/observer.rs:163-173`

**Issue:** Both public functions are unused within the reviewed file set — every registry
mutation goes through the wholesale `update_hotkey_bindings` replace. No caller exists in
`ipc/mod.rs`, `lib.rs`, or `state/mod.rs`.

**Fix:** Remove if genuinely superseded, or note where they're still used if outside this
file set.

### IN-07: `Intent::ResetEmergencyStop` has no caller

**File:** `src-tauri/src/state/mod.rs:181, 728-736`

**Issue:** No IPC command or platform hotkey handler in the reviewed files ever constructs
`Intent::ResetEmergencyStop`. Both platform emergency-stop paths
(`observer.rs:401`, `windows/mod.rs:514`) call `process::exit(1)` unconditionally right after
flushing held inputs, so there is currently no reachable in-process path that would ever need
this reset.

**Fix:** Either this is intentionally vestigial (add a comment explaining the intended future
use) or remove it.

### IN-08: Dead frontend `loading` signal

**File:** `src/App.tsx:214`

**Issue:** `const [, setLoading] = createSignal(true);` discards the getter.
`setLoading(false)` is called in the initial-fetch effect's `finally` block (line 326), but
the value is never read anywhere in the render tree.

**Fix:** Wire the getter into the UI (e.g. a loading gate before the first `get_state`
resolves) or remove the signal.

### IN-09: Non-Windows-gated `uuid::Uuid` import and split `use` blocks in `windows/mod.rs`

**File:** `src-tauri/src/platform/windows/mod.rs:407-409`

**Issue:** `use uuid::Uuid;` at line 409 is not itself `cfg`-gated, but its only consumer in
this file (`WindowsHotkeyBinding { ..., macro_id: Uuid }`, lines 36-42) is gated behind
`#[cfg(target_os = "windows")]`. On a non-Windows compilation of this module, this produces
an `unused_imports` warning. Relatedly, this second `use` block (lines 407-425) appears well
after several struct/impl definitions rather than being consolidated with the imports at the
top of the file (lines 10-25), reading as if two previously-separate files were concatenated.

**Fix:** Gate the import (`#[cfg(target_os = "windows")] use uuid::Uuid;`) and consolidate
all `use` statements at the top of the file.

### IN-10: Repeated `navigator.userAgent` mac-detection

**File:** `src/App.tsx:145, 173, 287`

**Issue:** `computeModifiers`, `modifierChips`, and the component-scoped `IS_MACOS` constant
each independently recompute `navigator.userAgent.toLowerCase().includes("mac")`. The
docstring at lines 141-142 acknowledges the duplication but doesn't eliminate it.

**Fix:** Hoist a single module-level `const IS_MACOS = ...` and reference it from all three
sites.

---

_Reviewed: 2026-07-22T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
