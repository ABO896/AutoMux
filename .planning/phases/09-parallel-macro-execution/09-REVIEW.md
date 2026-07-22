---
phase: 09-parallel-macro-execution
reviewed: 2026-07-22T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/platform/macos/observer.rs
  - src-tauri/src/scheduler/mod.rs
  - src/App.tsx
findings:
  critical: 1
  warning: 5
  info: 5
  total: 11
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-07-22T00:00:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

This is a fresh adversarial pass over the same five files (not a diff against the prior
09-REVIEW.md). The IPC argument-casing issues the prior review flagged appear to have been
fixed in the codebase as currently checked out: `bind_hotkey`, `unbind_hotkey`,
`set_macro_target_app`, and `set_macro_trigger_key` all use
`#[command(rename_all = "snake_case")]` and the `App.tsx` call sites pass matching
snake_case keys (`macro_id`, `target_app`, `trigger_key`), so that class of bug was not
re-verified as still open here.

The scheduler (`scheduler/mod.rs`) is solid: the two-phase dispatch, the no-op restart
guard (`RunningConfig`), and the guaranteed-delivery `.await` sends on the hold lifecycle
are correctly implemented and are backed by real concurrency tests (`afk_farm_stress_test`,
`parallel_two_macros_concurrent`, the two saturation tests).

The most serious finding in this pass is in the macOS hotkey path: a single keypress for a
macro's trigger key can be dispatched **twice** to the StateActor
(`Intent::ToggleMacroHotkey`) because the CGEventTap callback checks two
independently-populated registries (`HOTKEY_BINDINGS` and `MACRO_TRIGGER_KEYS`) that both
end up containing an entry for the same macro once the frontend's per-card "Set key…" flow
runs on macOS. Since `ToggleMacroHotkey` flips `mac.enabled`, two dispatches cancel each
other out — the hotkey silently does nothing. This directly undermines the project's
stated core value ("a macro that was set up must fire reliably") and was traced across
`observer.rs`, `state/mod.rs`, and `App.tsx`.

Several lower-severity reliability and consistency issues were also found, mostly around
error surfacing, timer cleanup consistency in the frontend, and a few dead-code / hygiene
items on the Rust side.

## Critical Issues

### CR-01: macOS hotkey toggle can silently no-op — dual registry double-dispatch

**File:** `src-tauri/src/platform/macos/observer.rs:418-451` (cross-referenced with
`src-tauri/src/state/mod.rs:542-598, 786-789` and `src/App.tsx:572-601`)

**Issue:**
The CGEventTap keydown handler checks two independent hotkey registries for
every keypress:

```rust
// CONFIGURABLE HOTKEYS — checked AFTER emergency stop
if let Ok(bindings) = get_hotkey_bindings().try_lock() {
    for binding in bindings.iter() {
        if binding.matches(keycode as u16, flags) {
            ... tx.try_send(Intent::ToggleMacroHotkey(*id)) ...
        }
    }
}

// MACRO TRIGGER KEYS (O(1) lookup)
if let Ok(trigger_keys) = get_macro_trigger_keys().try_lock() {
    if let Some(&macro_id) = trigger_keys.get(&(keycode as u16, mod_bits)) {
        tx.try_send(Intent::ToggleMacroHotkey(macro_id));
    }
}
```

`HOTKEY_BINDINGS` is rebuilt from **every** macro with a `trigger_key` set,
whenever `bind_hotkey`/`unbind_hotkey` fires (`build_hotkey_bindings_vec`,
`state/mod.rs:285-299`, called from `Intent::BindHotkey`/`Intent::UnbindHotkey`,
`state/mod.rs:583, 607`). `MACRO_TRIGGER_KEYS` is rebuilt from every macro with a
`trigger_key` set inside `reevaluate_all_macros()`, which is called
unconditionally on macOS too (`state/mod.rs:786-787`,
`#[cfg(target_os = "macos")] crate::platform::macos::observer::update_macro_trigger_keys(...)`)
after essentially every mutating intent (`AddMacro`, `SetMacroTriggerKey`,
`BindHotkey`, `UnbindHotkey`, `ToggleMacroHotkey`, …).

The frontend's card-based trigger-key editor calls **both** paths for macOS in
a single user action (`App.tsx:572-580`):

```ts
if (IS_MACOS) {
  await invoke("unbind_hotkey", { macro_id: id });
  await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers });
  await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
}
```

After this sequence, the macro's `(keycode, modifiers)` pair is registered in
**both** `HOTKEY_BINDINGS` (via `bind_hotkey`) and `MACRO_TRIGGER_KEYS` (via
`set_macro_trigger_key` → `reevaluate_all_macros`). Worse, `bind_hotkey`'s
`build_hotkey_bindings_vec` rebuilds `HOTKEY_BINDINGS` from **all** macros with a
trigger key, so the very first time the user sets any macro's key via this
card UI, every other macro that already has a trigger key (e.g. one set via the
"New Macro" form, which only ever populates `MACRO_TRIGGER_KEYS`) also becomes
double-registered from that point forward.

Once double-registered, a single keydown causes two
`Intent::ToggleMacroHotkey(macro_id)` sends. The handler
(`state/mod.rs:650-660`) does `mac.enabled = !mac.enabled;` — two flips is a
net no-op. The user presses the hotkey and the macro visibly does not toggle,
with no error surfaced anywhere.

This also contradicts the doc comment on `set_macro_trigger_key`
(`ipc/mod.rs:140-143`): *"macOS: the CGEventTap observes keycode via
HOTKEY_BINDINGS (managed separately by bind_hotkey)"* — implying
`MACRO_TRIGGER_KEYS` should be a Windows-only path, but the `#[cfg(target_os =
"macos")]` call in `reevaluate_all_macros()` populates it on macOS anyway, and
the macOS tap callback consumes it anyway.

**Fix:** Pick one source of truth for macOS. Two viable directions:
1. Keep `MACRO_TRIGGER_KEYS` as the single macOS+Windows registry: remove the
   macOS-specific "CONFIGURABLE HOTKEYS" `HOTKEY_BINDINGS` lookup block from the
   tap callback (`observer.rs:418-437`) entirely — `bind_hotkey`/`unbind_hotkey`
   would then only need to run the conflict-check reply, not maintain a second
   registry consumed on macOS.
2. Keep `HOTKEY_BINDINGS` as the single macOS registry: make `AddMacro` and
   `SetMacroTriggerKey` also call `build_hotkey_bindings_vec` +
   `update_hotkey_bindings` on macOS, and remove the
   `#[cfg(target_os = "macos")]` branch at `state/mod.rs:786-787` so
   `update_macro_trigger_keys` is only ever called for Windows.

Either way, `App.tsx:574-577` should also be simplified to call only the single
remaining IPC path instead of three IPC round-trips per key-set action.

## Warnings

### WR-01: Trigger-key conflict on macro creation is silently dropped with no user feedback

**File:** `src-tauri/src/state/mod.rs:453-467` (cross-referenced with `src/App.tsx:483-508`)

**Issue:** `Intent::AddMacro`'s conflict pre-check silently clears
`config.trigger_key`/`trigger_modifiers` to `None`/`0` when the requested key
collides with an existing macro — it never returns an error. `add_macro`'s
`Result<Uuid, String>` can therefore never carry the
`"is already assigned to \"...\""` message. Yet `App.tsx`'s
`handleCreateMacro` catch block (lines 494-508) is written specifically to
parse that message out of a caught error and show a `ConflictErrorToast`. In
practice, a user who creates a new macro with a trigger key that collides with
an existing one gets a macro created successfully with the trigger silently
removed — no toast, no console warning, nothing indicates the key was dropped.

**Fix:** Either (a) have `AddMacro` return `Err(conflict_message)` like
`BindHotkey` does and require the frontend to retry without the key /
surface the toast, or (b) have the `AddMacro` reply include whether the
trigger was dropped so the frontend can show `showConflictError` immediately
after a successful `add_macro` call instead of only in the `catch` branch.

### WR-02: CGEventTap thread has no panic guard — `TAP_INITIALIZED` can get stuck `true` forever

**File:** `src-tauri/src/platform/macos/observer.rs:244-496`

**Issue:** `initialize_tap()` sets `TAP_INITIALIZED = true` (line 242) *before*
spawning the thread, and only resets it back to `false` inside specific
success/failure branches at the end of the closure (lines 283-284, 476-477,
482-483, 490-491). If the spawned thread panics anywhere in between — e.g. a
`.lock().unwrap()` on an already-poisoned mutex, which can itself only happen
after some other panic, but once it does, every subsequent `.lock().unwrap()`
call in this file panics too — the thread dies without ever resetting
`TAP_INITIALIZED`/`TAP_STARTING`. From that point on, `initialize_tap()`'s fast
path (`if TAP_INITIALIZED.load(...) { return true; }`) reports the tap as
initialized even though no thread is running and no input is being observed —
the 3-second `check_accessibility` poll (`ipc/mod.rs:200-215`) that is
supposed to recover the tap can never re-arm it, and this is silent (no user
visible signal).

**Fix:** Wrap the closure body in `std::panic::catch_unwind`, or use a
`scopeguard`-style RAII guard that resets both flags on drop (covering the
panic path as well as the normal-return paths), so a single panic cannot
permanently disable tap recovery.

### WR-03: `try_lock()` on hotkey registries can silently drop a keypress under contention

**File:** `src-tauri/src/platform/macos/observer.rs:418, 440`

**Issue:** Both the `HOTKEY_BINDINGS` and `MACRO_TRIGGER_KEYS` lookups in the
tap callback use `try_lock()` and simply skip dispatch if the lock is held
(e.g., mid-update from `update_hotkey_bindings`/`update_macro_trigger_keys`
being called from the StateActor task). There is no retry, queuing, or
logging when this happens — a keypress that lands exactly during a registry
rebuild is silently swallowed with no diagnostic trace, unlike the
`try_send` overflow path in the scheduler which is at least counted in debug
builds (`scheduler/mod.rs:417-425`).

**Fix:** At minimum, log (debug-only) when `try_lock()` fails here, mirroring
the `ACTION_DROP_COUNT` diagnostic pattern already used in
`scheduler/mod.rs`, so a dropped keypress is diagnosable rather than
invisible.

### WR-04: Frontend timer cleanup is inconsistent — some timers leak past unmount

**File:** `src/App.tsx:236-248, 380-387, 419-425, 631-634`

**Issue:** The component's `onCleanup` (lines 380-387) only calls
`clearPending()` (the Accessibility 30s timeout). It does **not** call
`clearImPending()` (the Input Monitoring 30s timeout, defined at lines
419-425) and does not clear `_conflictErrorTimer` (set in `showConflictError`,
lines 241-248) or the profile-message 3s timeout created inline in
`showProfileMsg` (line 633). These timers will still fire after the component
is torn down (e.g. HMR remount), calling stale signal setters. The pattern
established for `_pendingTimeoutId` (comment: "onCleanup → clearPending()
path handles teardown") was not applied consistently to the other three
timers.

**Fix:** Add `clearImPending()` to the existing `onCleanup`, and track/clear
`_conflictErrorTimer` and the profile-message timeout the same way.

### WR-05: Inconsistent error surfacing — several handlers fail silently to the user

**File:** `src/App.tsx:445-451, 511-517, 519-526, 619-627`

**Issue:** `handleToggleEngine`, `handleToggleMacro`, `handleRemoveMacro`, and
`handleCardSetTargetApp` all catch IPC errors with only `console.error(...)` —
there is no toast, banner, or any user-visible signal that the action failed.
By contrast, profile actions (`handleSaveProfile`/`handleLoadProfile`/
`handleDeleteProfile`) and the trigger-key flow (`handleCardSetTriggerKey`)
do surface failures via `showProfileMsg`/`showConflictError`. A user who
clicks "Delete" on a macro and has the backend call fail (e.g. channel
closed) sees the macro still present with no explanation.

**Fix:** Route these handlers through a shared error-toast mechanism (or at
minimum reuse `showProfileMsg`-style feedback) instead of `console.error`
alone, so failures are visible in the UI.

## Info

### IN-01: Dead code — two macOS observer functions are never called

**File:** `src-tauri/src/platform/macos/observer.rs:167-173, 211-214`

**Issue:** `remove_hotkey_bindings_for(macro_id: &Uuid)` and
`is_tap_initialized()` are `pub fn` but have no callers anywhere in the
codebase (verified via repo-wide grep). They are either leftover from a prior
design or intended for a caller that was never wired up.

**Fix:** Remove if truly unused, or wire them in where intended (e.g.
`is_tap_initialized` seems like a natural fit for a status IPC command).

### IN-02: `get_active_app` fetches the entire `AppState` for one field

**File:** `src-tauri/src/ipc/mod.rs:64-73`

**Issue:** `get_active_app` round-trips a full `Intent::GetState` (which
clones/serializes the entire macro registry) just to read
`app_state.active_app`. `get_state` is already called separately by the
frontend on the same initial-fetch `Promise.all` (`App.tsx:297-305`), so this
is a redundant larger payload for a single string field.

**Fix:** Consider a dedicated lightweight `Intent::GetActiveApp` if this
command is called frequently, or drop it in favor of reading `active_app`
off the existing `get_state`/`state-changed` payload the frontend already
has.

### IN-03: Magic number for the emergency-stop keycode

**File:** `src-tauri/src/platform/macos/observer.rs:340`

**Issue:** `if keycode == 12 && has_cmd && has_shift` hardcodes macOS keycode
12 (the `Q` key) with no named constant, unlike the `CGEventFlags` bit values
above it which are documented and pinned by a unit test.

**Fix:** Extract to a named constant, e.g.
`const EMERGENCY_STOP_KEYCODE: i64 = 12; // 'Q'`.

### IN-04: `set_macro_sequence` / `update_step_interval` IPC commands have no frontend caller

**File:** `src-tauri/src/ipc/mod.rs:268-294` (cross-referenced with `src/App.tsx`)

**Issue:** Both commands are registered in `lib.rs`'s `invoke_handler!` and
implemented, but a repo-wide search of `src/App.tsx` shows neither
`set_macro_sequence` nor `update_step_interval` is invoked from the UI. Either
this is intentionally forward-looking API surface, or it's dead surface that
should be documented as such.

**Fix:** No action required if intentionally unused for now; otherwise wire
up or remove.

### IN-05: `App()` is a single ~900-line function mixing all concerns

**File:** `src/App.tsx:200-1594`

**Issue:** The entire application — signal declarations, effects, ~20 event
handlers, and the full JSX tree for both tabs — lives in one function body.
This is high cyclomatic complexity and makes the file hard to navigate and
review (this review itself had to scan the whole file to trace a handful of
handlers). CLAUDE.md's conventions describe `App.tsx` as "Single `App.tsx`
component with all UI" by design, so this may be an accepted project
convention rather than an oversight — flagged for awareness, not as a
required fix.

**Fix (optional):** Consider extracting the dashboard macro-card renderer and
the profiles-tab list into local sub-components/functions to reduce the
single function's size, if maintainability becomes a pain point.

---

_Reviewed: 2026-07-22T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
