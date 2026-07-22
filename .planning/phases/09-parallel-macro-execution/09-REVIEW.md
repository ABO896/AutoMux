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
  warning: 5
  info: 6
  total: 13
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-07-22T00:00:00Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

This is a fresh full-phase review of all 7 files as they stand after plan 09-10 (CR-01
hotkey double-dispatch consolidation). Note: this supersedes the prior `09-REVIEW.md` in
this directory, which reviewed 5 files pre-09-10 and found a dual-registry
(`HOTKEY_BINDINGS` + `MACRO_TRIGGER_KEYS`) double-dispatch bug. That specific bug is
confirmed fixed here — the current `observer.rs`/`windows/mod.rs` each maintain exactly one
`HOTKEY_BINDINGS` registry, `build_hotkey_bindings_vec` is the sole builder, and the
regression test `hotkey_registry_has_single_binding_per_trigger_macro`
(`state/mod.rs:1110-1146`) correctly pins the "one binding per trigger macro" invariant.

However, tracing the two-phase dispatch path (Scheduler → `action_tx` →
`StateActor::handle_action`) specifically for **releases** (as opposed to new-input starts)
surfaces a new, serious gap: the same three gates that correctly govern whether *new* input
should be injected (engine active, macro enabled, target-app match) are also applied,
unconditionally, to `HoldRelease` actions. Because several state mutations (disable macro,
toggle engine off, switch active app away from a macro's target, load/switch profile) flip
the very state the gates check *before* the scheduler's guaranteed-delivery `HoldRelease`
message is actually processed, the release is silently swallowed by the gate and the
physical key/mouse button injected by the earlier `HoldStart` is left stuck down. This
directly undermines the CR-01/CR-02 "guaranteed delivery" fixes documented in
`scheduler/mod.rs` — channel delivery of the release is guaranteed, but injection of the
release is not.

A second, unrelated but similarly serious issue was found in the hotkey rebind flow
spanning `App.tsx` and `state/mod.rs`: attempting to rebind an existing macro's card hotkey
to a key already used by another macro destroys the macro's previously-working binding with
no rollback, and on Windows this happens completely silently (no error surfaced to the user
at all).

Several smaller warnings and dead-code findings are listed below.

## Critical Issues

### CR-01: `HoldRelease` actions are gated identically to new-input actions — held inputs can get stuck

**File:** `src-tauri/src/state/mod.rs:373-413`

**Issue:** `StateActor::handle_action` applies the same three gates to every `ActionType`:

```rust
fn handle_action(&self, action: crate::scheduler::ActionReady) {
    // Gate 1: Engine must be active
    if !self.state.engine_active || self.state.emergency_stop_active { return; }
    // Gate 2: Macro must exist and be enabled
    let mac = match self.state.macros.get(&action.macro_id) {
        Some(m) if m.enabled => m,
        _ => return,
    };
    // Gate 3: Target app must match (or be Global)
    ...
    match &action.action_type {
        ActionType::Interval(input) => { ... }
        ActionType::HoldStart(input) => { ... }
        ActionType::HoldRelease(input) => { self.inject_input(input, false); }
    }
}
```

The Scheduler's `stop_macro`/`release_holds`/`StopAll` paths were hardened (CR-01/CR-02,
`scheduler/mod.rs:214-234, 300-325, 371-394`) to use `.await` sends instead of `try_send`
specifically so `HoldRelease` messages can never be dropped by channel backpressure. But
channel delivery is not the same as injection: by the time the `HoldRelease` action reaches
`handle_action`, the very state mutation that triggered the stop has usually already
applied, so the gates reject it:

- **Disable a Hold-mode macro:** `Intent::SetMacroEnabled(id, false)` (`state/mod.rs:491-500`)
  sets `mac.enabled = false` *before* `reevaluate_all_macros()` sends `StopMacro`. When the
  resulting `HoldRelease` arrives, Gate 2's `Some(m) if m.enabled => m, _ => return` discards
  it because `enabled` is already `false`.
- **Toggle engine off:** `Intent::ToggleEngineHotkey` (`state/mod.rs:661-671`) sets
  `self.state.engine_active = false` *before* sending `SchedulerIntent::StopAll`. Gate 1
  (`!self.state.engine_active`) discards every `HoldRelease` produced by the
  guaranteed-delivery `StopAll` release loop.
- **Switch away from target app:** `Intent::ActiveAppChanged` (`state/mod.rs:643-649`) sets
  `self.state.active_app = app` *before* `reevaluate_all_macros()` stops the now-out-of-target
  macro. Gate 3 discards the resulting `HoldRelease` because `active_app` no longer matches
  `target_app`. This is the most likely real-world trigger (e.g. alt-tabbing away from a game
  while a Hold-mode right-click macro is engaged for an AFK farm — exactly the scenario the
  stress tests exercise).
- **Load/switch profile:** `Intent::LoadProfile` (`state/mod.rs:706-751`) calls
  `self.state.macros.clear()` *before* sending `StopAll`. Gate 2 discards the `HoldRelease`
  for the old macro id because it no longer exists in `state.macros` at all.

In every one of these paths the physical input injected by the earlier `HoldStart`
(`self.input_provider.inject_mouse_button_raw(btn, true)` / `inject_key(keycode, true)`) is
never followed by the corresponding "up" injection. The only recovery path left is the
hardcoded Emergency Stop hotkey, which bypasses this gate system entirely via
`self.input_provider.flush_held_inputs()` (`state/mod.rs:632`) — a normal disable/engine-toggle
/app-switch/profile-load does not call this.

**Fix:** `HoldRelease` is a cleanup event, not a new-input request — it should never be
blocked by the gates that exist to prevent unwanted new input:

```rust
fn handle_action(&self, action: crate::scheduler::ActionReady) {
    use crate::scheduler::ActionType;

    // A release must always be honored — a dropped release leaves a
    // physically stuck input with no cleanup path short of Emergency Stop.
    if let ActionType::HoldRelease(input) = &action.action_type {
        self.inject_input(input, false);
        return;
    }

    // Gate 1: Engine must be active
    if !self.state.engine_active || self.state.emergency_stop_active {
        return;
    }
    // ... existing Gate 2 / Gate 3 / Interval / HoldStart handling unchanged
}
```

### CR-02: Conflicting hotkey rebind silently destroys the macro's previously-working trigger key

**File:** `src/App.tsx:572-601`, `src-tauri/src/state/mod.rs:514-541`

**Issue:** Rebinding an existing macro's hotkey from its card has no rollback on failure, on
either platform:

- **macOS** (`App.tsx:574-577`): `handleCardSetTriggerKey` first calls `unbind_hotkey`
  (unconditionally clearing the macro's current trigger key), *then* `bind_hotkey` with the
  new key. If the new key conflicts with another macro, `bind_hotkey` rejects with
  `Err(msg)` — but the `unbind_hotkey` call has already succeeded, so the macro is left with
  **no** hotkey at all. The user only sees a "Hotkey already bound" toast; nothing indicates
  their previously-working binding was just deleted. There is also no reason for the
  `unbind_hotkey` step to exist at all — `Intent::BindHotkey` (`state/mod.rs:575-578`)
  already overwrites `mac.trigger_key`/`trigger_modifiers` unconditionally on success, so
  calling `bind_hotkey` alone (without unbinding first) would both fix the data-loss bug and
  remove an unnecessary round trip.
- **Windows** (`App.tsx:579`, `state/mod.rs:514-541`): the card path calls
  `set_macro_trigger_key` directly, which maps to `Intent::SetMacroTriggerKey`. On conflict
  this handler silently coerces `new_key`/`new_mods` to `None`/`0` and applies that
  (`state/mod.rs:522-531`) — no `Result`/oneshot error path exists for this intent, so
  `ipc::set_macro_trigger_key` always returns `Ok(())` (`ipc/mod.rs:291-295`). The net effect
  is identical to the macOS case (previously-working key is destroyed) but *without even a
  toast* — the user gets zero feedback that their rebind attempt failed.

**Fix:** For macOS, drop the leading `unbind_hotkey` call entirely and rely on `bind_hotkey`
alone (it already overwrites in place on success and returns `Err` untouched on conflict, so
the old binding survives a failed rebind):

```ts
if (IS_MACOS) {
  await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers });
} else {
  await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
}
```

For Windows, give `Intent::SetMacroTriggerKey` the same `Result`-carrying oneshot pattern
`Intent::BindHotkey` already uses, so a conflict is rejected (old binding preserved) and
surfaced to the same conflict toast instead of being silently coerced to `None`.

## Warnings

### WR-01: Cross-platform hotkey modifier-matching asymmetry

**File:** `src-tauri/src/platform/macos/observer.rs:48-52` vs
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

A macro bound to `Shift+F5` will fire on macOS even if the user is also holding `Ctrl`
(extra bits ignored), but will **not** fire on Windows under the same physical key
combination (`mod_mask` includes the extra `Ctrl` bit, breaking exact equality). The same
configured hotkey behaves differently across the two supported platforms.

**Fix:** Pick one semantics and apply it on both platforms — most likely the exact-match
(Windows) behavior is the intended one (avoids accidental cross-triggering), so change the
macOS `matches()` to `flags.bits() == self.modifiers` (after masking off any non-modifier
bits CGEventFlags may carry), or explicitly document/test the intentional asymmetry if it's
a deliberate platform difference.

### WR-02: Redundant hotkey-registry rebuild in `BindHotkey`/`UnbindHotkey` handlers

**File:** `src-tauri/src/state/mod.rs:579-592, 607-616, 758-772`

**Issue:** `Intent::BindHotkey` and `Intent::UnbindHotkey` both explicitly call
`build_hotkey_bindings_vec(&self.state.macros)` and `update_hotkey_bindings(...)` (lines
583-591 / 607-615), then immediately call `self.reevaluate_all_macros().await` (line 592 /
616), which — per the CR-01 gap-closure comment at line 758 — *unconditionally* performs the
exact same rebuild again at the top of its body (lines 764-772). Every bind/unbind now
rebuilds and replaces the platform registry twice for no behavioral difference (the second
replace is idempotent with the first).

**Fix:** Since `reevaluate_all_macros` is documented as "the sole registry-refresh site,"
remove the now-redundant explicit rebuild block from the `BindHotkey`/`UnbindHotkey`
handlers and rely solely on the call to `reevaluate_all_macros()`.

### WR-03: macOS card hotkey-rebind issues 3 redundant IPC round trips (and 3 auto-saves) for one logical change

**File:** `src/App.tsx:572-601`

**Issue:** Beyond the data-loss bug in CR-02, the macOS branch of
`handleCardSetTriggerKey` performs three sequential `invoke()` calls (`unbind_hotkey`,
`bind_hotkey`, `set_macro_trigger_key`) for what is logically a single field update. Each of
the three corresponding `Intent` handlers ends with `self.auto_save_default().await`
(`state/mod.rs:596, 620, 540`), so a single hotkey rebind on macOS triggers three separate
profile-JSON disk writes.

**Fix:** Once CR-02's fix removes the leading `unbind_hotkey` call, also drop the trailing
`set_macro_trigger_key` call — `bind_hotkey`'s `Intent::BindHotkey` handler already persists
`trigger_key`/`trigger_modifiers` on the macro (`state/mod.rs:575-578`), making that third
call fully redundant.

### WR-04: `delete_profile`/`load_profile` forward unsanitized profile names while `save_profile` sanitizes first

**File:** `src-tauri/src/ipc/mod.rs:344-370`

**Issue:** `save_profile` explicitly calls `ProfileManager::sanitize_name(&name)` before use,
with a comment (`CR-03`) explaining why: so the stored name always matches the on-disk
filename stem. `load_profile` and `delete_profile`, by contrast, forward the raw `name`
parameter straight to `profile_mgr.load_profile(&name)` / `profile_mgr.delete_profile(&name)`
with no local sanitization. `persistence.rs` is out of this review's file set, so it's not
verified here whether those methods sanitize internally — but the asymmetry in the one file
that is in scope is itself a maintainability/defense-in-depth gap: a reader cannot tell from
`ipc/mod.rs` alone whether `load_profile("../../../Library/Preferences/foo")` is safe.

**Fix:** Either apply `ProfileManager::sanitize_name` consistently to all three commands in
`ipc/mod.rs`, or add a comment at `load_profile`/`delete_profile` pointing to where
`persistence.rs` performs the equivalent sanitization, so the trust boundary is explicit and
auditable from this file.

### WR-05: Unused `uuid::Uuid` import on non-Windows targets

**File:** `src-tauri/src/platform/windows/mod.rs:409` (used only by the
`#[cfg(target_os = "windows")]`-gated struct at lines 36-42)

**Issue:** `use uuid::Uuid;` at line 409 is not itself `cfg`-gated, but its only consumer in
this file, `WindowsHotkeyBinding { ..., macro_id: Uuid }`, is gated behind
`#[cfg(target_os = "windows")]`. On any non-Windows compilation of this module (e.g.
macOS/Linux dev builds or CI matrix checks that compile all platform modules), this produces
an `unused_imports` warning.

**Fix:** Gate the import: `#[cfg(target_os = "windows")] use uuid::Uuid;`.

## Info

### IN-01: Dead `HotkeyAction::ToggleEngine` variant

**File:** `src-tauri/src/platform/macos/observer.rs:34-38, 415-417`

**Issue:** `HotkeyAction::ToggleEngine` and its `match` arm in the tap callback are never
reachable — `build_hotkey_bindings_vec` (`state/mod.rs:284-299`) only ever constructs
`HotkeyAction::ToggleMacro(mac.id)`. There is no code path anywhere in the reviewed files
that lets a user bind a hotkey to the global engine toggle; `toggle_engine` is only invoked
via the dashboard button (`App.tsx:445-451`).

**Fix:** Either wire up a UI/IPC path to actually register a `ToggleEngine` binding, or
remove the dead variant/arm until that feature exists.

### IN-02: Unused `add_hotkey_binding` / `remove_hotkey_bindings_for` functions

**File:** `src-tauri/src/platform/macos/observer.rs:163-173`

**Issue:** Both public functions are unused within the reviewed file set — every registry
mutation goes through the wholesale `update_hotkey_bindings` replace (matching the "replace
is cheap" design noted at `state/mod.rs:283`). No caller of either function exists in
`ipc/mod.rs`, `lib.rs`, or `state/mod.rs`.

**Fix:** Remove if genuinely superseded, or note where they're still used if that's outside
this file set.

### IN-03: `Intent::ResetEmergencyStop` has no caller

**File:** `src-tauri/src/state/mod.rs:170, 634-642`

**Issue:** No IPC command or platform hotkey handler in the reviewed files ever constructs
`Intent::ResetEmergencyStop`. Combined with both platform emergency-stop paths
(`observer.rs:401`, `windows/mod.rs:514`) calling `process::exit(1)` unconditionally right
after flushing held inputs, there is currently no reachable in-process path that would ever
need this reset — the process is already gone by the time any "resume" UI could act.

**Fix:** Either this is intentionally vestigial (in which case a comment explaining the
intended future use would help) or it can be removed.

### IN-04: Dead frontend `loading` signal

**File:** `src/App.tsx:214`

**Issue:** `const [, setLoading] = createSignal(true);` discards the getter.
`setLoading(false)` is called in the initial-fetch effect's `finally` block
(`App.tsx:326`), but the `loading` value is never read anywhere in the render tree — there
is no loading spinner or gate on it.

**Fix:** Either wire the getter into the UI (e.g. a loading state before the first
`get_state` resolves) or remove the signal.

### IN-05: `windows/mod.rs` mixes two apparent source groupings in one file

**File:** `src-tauri/src/platform/windows/mod.rs:1-25` and `:407-425`

**Issue:** A second block of `use` statements (`AtomicBool`, `OnceLock`, `Uuid`, Win32
hook/accessibility imports) appears at line 407, well after several struct/impl definitions,
rather than being consolidated with the imports at the top of the file. This reads as if two
previously-separate files (input-injection code and hotkey/hook code) were concatenated
without merging their preludes.

**Fix:** Consolidate all `use` statements at the top of the file for readability/greppability.

### IN-06: Repeated `navigator.userAgent` mac-detection

**File:** `src/App.tsx:145, 173, 287`

**Issue:** `computeModifiers`, `modifierChips`, and the component-scoped `IS_MACOS` constant
each independently recompute `navigator.userAgent.toLowerCase().includes("mac")`. The
module-level docstring at line 141-142 already acknowledges this duplication ("same
detection, different scope, identical bit output") but doesn't eliminate it.

**Fix:** Hoist a single module-level `const IS_MACOS = ...` and reference it from all three
sites.

---

_Reviewed: 2026-07-22T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
