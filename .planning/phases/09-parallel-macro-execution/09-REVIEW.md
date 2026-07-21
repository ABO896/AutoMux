---
phase: 09-parallel-macro-execution
reviewed: 2026-07-21T01:03:07Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/scheduler/mod.rs
  - src/App.tsx
findings:
  critical: 2
  warning: 6
  info: 3
  total: 11
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-07-21T01:03:07Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

This is a fresh, independent re-review of `src-tauri/src/ipc/mod.rs`, `src-tauri/src/lib.rs`,
`src-tauri/src/scheduler/mod.rs`, and `src/App.tsx` against their current state on disk. Every finding
below was re-derived by reading the current source directly — nothing is copy-pasted from the prior
pass — but where a re-derived finding happens to match something the prior review already flagged, that
is noted so the record stays accurate about what plan 09-05 did and did not fix.

**Confirmed fixed:** plan 09-05's change to `Scheduler::release_holds` and the `StopAll` intent handler
(scheduler/mod.rs:214-234, 377-390) — both now use `self.action_tx.send(...).await` instead of
`try_send`, so a `HoldRelease` can no longer be silently dropped under `action_tx` saturation on the
stop paths. The `stop_macro_release_delivered_under_saturation` test (scheduler/mod.rs:884-944) exercises
this with a capacity-1 channel and demonstrates the fix works.

**Not fixed — asymmetric gap (new Critical, CR-01 below):** the fix was applied only to the *release*
side of the hold lifecycle. `Scheduler::start_macro`'s `HoldStart` send (scheduler/mod.rs:296-321) is
still `try_send`, and `holds.push(*input)` runs unconditionally regardless of whether the send succeeded.
This reproduces the exact class of bug the 09-05 fix targeted, just on the opposite end — a macro can be
recorded (and rendered in the UI) as actively holding an input that was never actually pressed.

**Not fixed — carried forward (Critical, CR-02 below):** `App.tsx`'s `handleCardSetTriggerKey` still
unbinds the macro's existing hotkey before confirming the replacement bind succeeds, on the macOS path.
A conflicting new key permanently loses the old, working binding.

Several other issues (interval validation, error-message mislabeling, editing-state cleanup after a
failed bind, timer cleanup on unmount, missing name sanitization on two profile IPC commands, and a few
naming/dead-value quality items) are detailed below. No hardcoded secrets, `eval`, `innerHTML`/
`dangerouslySetInnerHTML`, or injection patterns were found in any of the four files.

## Critical Issues

### CR-01: `HoldStart` is still fire-and-forget (`try_send`) while `HoldRelease` was fixed to guaranteed delivery

**File:** `src-tauri/src/scheduler/mod.rs:296-321`
**Issue:**
In `Scheduler::start_macro`, the `SustainedHold` branch fires `ActionType::HoldStart` via
`self.action_tx.try_send(...)` and unconditionally does `holds.push(*input)` regardless of whether the
send succeeded:

```rust
ActionStep::SustainedHold { input } => {
    // Fire HoldStart immediately.
    if self
        .action_tx
        .try_send(ActionReady {
            macro_id,
            action_type: ActionType::HoldStart(*input),
            fired_at: Instant::now(),
        })
        .is_err()
    {
        #[cfg(debug_assertions)]
        { ACTION_DROP_COUNT.fetch_add(1, ...); eprintln!(...); }
    }
    holds.push(*input);   // <-- recorded as "held" even if the send above failed
}
```

Compare this to `release_holds` (scheduler/mod.rs:377-390), fixed in 09-05 to use
`self.action_tx.send(...).await` specifically so a `HoldRelease` can never be silently dropped under
`action_tx` saturation. The `HoldStart` path was left on `try_send`.

Under `action_tx` backpressure (the same 1024-slot channel the D-09/D-10 debug drop-counter exists to
diagnose — a scenario this codebase already treats as realistically reachable, especially with many
concurrent macros, which is the entire point of this phase), a `HoldStart` can be dropped. Because
`holds.push(*input)` runs unconditionally, `active_holds` still records the macro as holding the input,
`running_configs` still marks it started, and the frontend's `computeRunningState`
(src/App.tsx:90-116) will render the macro's status dot as "held" (blue) purely from
`trigger_mode === "Hold"`, with no confirmation the StateActor ever actually injected the input. The
scheduler and UI both believe the macro is actively holding a button/key; nothing is physically pressed;
there is no retry. The only recovery is toggling the macro off/on, which can hit the same drop again
under sustained load.

**Fix:** Mirror the guaranteed-delivery pattern already used in `release_holds`/`StopAll`:

```rust
ActionStep::SustainedHold { input } => {
    // Guaranteed delivery — mirrors release_holds so a HoldStart cannot be
    // silently dropped while active_holds still records the macro as held.
    let _ = self
        .action_tx
        .send(ActionReady {
            macro_id,
            action_type: ActionType::HoldStart(*input),
            fired_at: Instant::now(),
        })
        .await;
    holds.push(*input);
}
```

This reintroduces the same trade-off already accepted for `release_holds`/`StopAll` (an `.await` here
can block the Scheduler's main `tokio::select!` loop until the channel frees a slot) — consistent with
the codebase's existing correctness-over-responsiveness stance for hold lifecycle events, as distinct
from the periodic `Interval` fires in `fire_due_actions`, which intentionally keep `try_send` because a
dropped tick self-corrects on the next interval.

### CR-02: macOS hotkey rebind destroys the existing binding before the replacement is confirmed

**File:** `src/App.tsx:563-586`
**Issue:**
```ts
async function handleCardSetTriggerKey(id: string, nativeCode: number, modifiers: number) {
    try {
      if (IS_MACOS) {
        await invoke("unbind_hotkey", { macro_id: id });
        await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers });
        await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
      } else {
        await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
      }
      ...
```
On macOS, `unbind_hotkey` runs first and — per its own doc comment in `ipc/mod.rs:95-98` — clears the
macro's trigger and rebuilds the platform hotkey registry immediately (mutates persisted state). If the
subsequent `bind_hotkey` call then fails (the documented conflict-check path, UX-11), the `catch` block
only shows a toast — it never re-establishes the macro's previous binding. A user attempting to change an
existing, working hotkey to one that happens to conflict with another macro's binding ends up with **no
hotkey at all** on the macro they were editing.

This directly undermines the project's stated Core Value ("A macro that was set up must fire reliably")
— an unrelated user action (editing one macro's hotkey) can silently disable another, previously-working
macro's ability to be triggered.

**Fix:** Do not destroy the existing binding until the new one is confirmed — attempt the new bind
first, and only clear the old key on success:
```ts
async function handleCardSetTriggerKey(id: string, nativeCode: number, modifiers: number) {
    try {
      if (IS_MACOS) {
        // Attempt the new binding first; only clear the old one once accepted.
        await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers });
        await invoke("unbind_hotkey", { macro_id: id }); // release the OLD platform binding, if any prior key existed
        await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
      } else {
        await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
      }
      setEditingCardId(null);
      setEditingField(null);
    } catch (e) {
      // ... existing conflict toast handling ...
    }
}
```
(Adjust to whatever the backend's conflict-check actually allows — e.g. if `bind_hotkey` itself refuses
to register a second binding for the same macro without an explicit unbind first, the fix instead needs
an explicit rollback: on `bind_hotkey` failure, re-issue `bind_hotkey`/`set_macro_trigger_key` with the
macro's previous `trigger_key`/`trigger_modifiers` before surfacing the conflict.)

## Warnings

### WR-01: `handleCreateMacro` and `handleCardSetTriggerKey` mislabel every failure as a hotkey conflict

**File:** `src/App.tsx:494-508`, `src/App.tsx:574-585`
**Issue:** Both `catch` blocks unconditionally render the "Hotkey already bound" conflict toast for
*any* thrown error from `invoke()`, not just actual conflict errors:
```ts
} catch (e) {
  const msg = String(e);
  const macroMatch = msg.match(/is already assigned to "([^"]+)"/);
  const macroName = macroMatch ? macroMatch[1] : "another macro";
  const keyLabel = triggerKey !== null ? resolveKeyName(triggerKey) : "Key";
  showConflictError(keyLabel, macroName);   // always shown, regardless of cause
  console.error("Failed to create macro:", e);
}
```
If `add_macro` (or `bind_hotkey`/`unbind_hotkey`/`set_macro_trigger_key`) fails for any other reason —
a malformed numeric field causing an IPC deserialization error (see WR-03), the state channel being
closed, or any future backend validation rule — the user is shown a fabricated "Hotkey already bound …
is already assigned to 'another macro'" message even when there is no conflict and possibly no hotkey
involved at all.

**Fix:** Only show the conflict toast when the regex actually matches the known conflict error format;
otherwise surface a generic failure message:
```ts
} catch (e) {
  const msg = String(e);
  const macroMatch = msg.match(/is already assigned to "([^"]+)"/);
  if (macroMatch) {
    const keyLabel = triggerKey !== null ? resolveKeyName(triggerKey) : "Key";
    showConflictError(keyLabel, macroMatch[1]);
  } else {
    console.error("Failed to create macro:", e);
    // TODO: surface a generic error toast instead of silently swallowing it
  }
}
```

### WR-02: Card trigger-key edit UI is left stuck after a failed bind

**File:** `src/App.tsx:563-586`, `1332-1401`
**Issue:** `handleCardSetTriggerKey`'s `catch` block never resets `editingCardId`/`editingField` — they
are only cleared on the success path. The card view (lines 1348-1401) renders the "Press…" chip whenever
`editingCardId() === macro.id && editingField() === "key"`, independent of `triggerKeyRecording()`. A
failed bind (e.g. the CR-02 conflict scenario above) therefore leaves the card frozen showing "Press…"
with an active-looking accent border even though key capture already ended — the `keydown` listener was
already removed inside `startCapture`'s `onKeyDown` before `onCommit` (i.e. `handleCardSetTriggerKey`)
was even invoked. The card is only recoverable if the user notices the small "✕" inside that stale chip.

**Fix:**
```ts
} catch (e) {
  const msg = String(e);
  const macroMatch = msg.match(/is already assigned to "([^"]+)"/);
  const macroName = macroMatch ? macroMatch[1] : "another macro";
  const keyLabel = resolveKeyName(nativeCode);
  showConflictError(keyLabel, macroName);
  setEditingCardId(null);
  setEditingField(null);
  console.error("Card trigger key update failed:", e);
}
```

### WR-03: No client-side validation prevents negative/invalid macro intervals

**File:** `src/App.tsx:1095-1103`, `src/App.tsx:456`
**Issue:** The interval `<input type="number">` has no `min` attribute, and
`const interval = parseInt(newMacroInterval()) || 100;` only guards `NaN`/`0` (both falsy) — not
negative values. Typing `-50` produces `interval_ms: -50` in the `add_macro` payload, but the Rust field
is `interval_ms: u64`, so the call fails at JSON→Rust deserialization with an opaque error rather than a
friendly validation message — and per WR-01, that opaque error is then mis-displayed as a "Hotkey
already bound" conflict.

**Fix:**
```tsx
<input id="input-macro-interval" type="number" min="1" ... />
```
```ts
const parsed = parseInt(newMacroInterval());
const interval = Number.isFinite(parsed) && parsed >= 1 ? parsed : 100;
```

### WR-04: Toast auto-dismiss timers are not cleared on component unmount

**File:** `src/App.tsx:241-248` (`_conflictErrorTimer` / `showConflictError`), `616-619` (`showProfileMsg`)
**Issue:** The file is otherwise careful about clearing pending timers on unmount —
`clearPending()`/`clearImPending()` are both invoked from the top-level `onCleanup` (lines 380-387).
`_conflictErrorTimer` and the anonymous `setTimeout` inside `showProfileMsg` have no equivalent
cleanup — they are only cleared when a new toast of the same kind supersedes the old one, never on
component teardown. If the component unmounts while one of these timers is pending (e.g. during HMR),
the timer callback still fires and calls a signal setter (`setConflictError`/`setProfileMessage`) on a
disposed reactive scope.

**Fix:** Track both timer handles at component scope and clear them in the existing `onCleanup`
alongside `clearPending()`.

### WR-05: `delete_profile` / `load_profile` do not sanitize the profile name before handing it to the persistence layer

**File:** `src-tauri/src/ipc/mod.rs:343-369`
**Issue:** `save_profile` explicitly sanitizes the incoming `name` up front (documented under CR-03,
lines 301-318) specifically so the on-disk filename always matches the stored name. `delete_profile`
(360-369) and `load_profile` (343-355) pass the raw, unsanitized `name` straight through to
`profile_mgr.delete_profile(&name)` / `Intent::LoadProfile(name, tx)` with no equivalent guard in this
file — `delete_profile` only special-cases the literal string `"default"` (case-insensitively), nothing
else. `persistence.rs` is out of scope for this review pass, so it's possible `ProfileManager` sanitizes
internally before building a filesystem path — but that can't be confirmed from these four files, and the
asymmetry with `save_profile`'s explicit, documented sanitization is worth a follow-up check for a
path-traversal vector (e.g. a crafted `name` containing `../` reaching `delete_profile`).

**Fix:** Apply `ProfileManager::sanitize_name` (or equivalent validation) to `name` in
`delete_profile`/`load_profile` before it reaches the persistence layer, or confirm and document that
`ProfileManager`'s internal methods already do this unconditionally regardless of caller.

### WR-06: Startup profile restoration failure is completely silent

**File:** `src-tauri/src/lib.rs:35-44`
**Issue:**
```rust
let startup_tx = state_tx.clone();
tauri::async_runtime::spawn(async move {
    #[cfg(debug_assertions)]
    eprintln!("[Startup] Dispatching LoadProfile(\"default\") intent");
    let (tx, _rx) = tokio::sync::oneshot::channel();
    let _ = startup_tx
        .send(Intent::LoadProfile("default".to_string(), tx))
        .await;
});
```
The oneshot receiver for the startup `LoadProfile` is dropped (`_rx`), and even the outer
`.send(...).await` result is discarded via `let _ =`. If the default profile fails to load at startup
(corrupted JSON, permissions error, etc.), the user launches into an empty macro list with **zero
indication anything went wrong** — no toast, no banner, no release-build log line (the only `eprintln!`
here is gated to the dispatch message, not the outcome). The existing "Auto-Save Error Banner"
(`App.tsx:1017-1036`, driven by the `auto-save-error` event) covers *save* failures but has no
equivalent for *load* failures on startup.

**Fix:** Await the startup load's oneshot response and, on failure, emit an event the frontend can
surface (e.g. an `auto-load-error` event feeding a banner analogous to the existing auto-save-error one)
instead of discarding both the send result and the oneshot response.

## Info

### IN-01: Inconsistent IPC argument naming for the macro identifier

**File:** `src-tauri/src/ipc/mod.rs:80-108`
**Issue:** `bind_hotkey` and `unbind_hotkey` take `macro_id: Uuid`, while every other per-macro command
(`remove_macro`, `set_macro_enabled`, `set_macro_target_app`, `set_macro_trigger_key`,
`set_macro_sequence`, `update_step_interval`) takes `id: Uuid`. Minor, avoidable naming inconsistency
across an otherwise uniform IPC surface.
**Fix:** Rename `macro_id` → `id` in `bind_hotkey`/`unbind_hotkey` (requires updating the matching
`App.tsx` call sites that currently pass `{ macro_id: id, ... }`).

### IN-02: macOS platform detection is duplicated three times in `App.tsx`

**File:** `src/App.tsx:145`, `173`, `287`
**Issue:** `navigator.userAgent.toLowerCase().includes("mac")` is independently re-declared as
`IS_MACOS` inside `computeModifiers`, inside `modifierChips`, and again inside `App()`. Any future
change to the detection heuristic requires updating three sites in lockstep.
**Fix:** Hoist a single module-level `const IS_MACOS = navigator.userAgent.toLowerCase().includes("mac");`
and reference it from all three call sites (or pass it as a parameter into the two free functions).

### IN-03: `add_macro`'s returned UUID is fetched but discarded

**File:** `src/App.tsx:483-484`
**Issue:** `await invoke<string>("add_macro", { config });` discards the backend-generated UUID
entirely, relying solely on the subsequent `state-changed` event to populate the real macro ID into
`state()`. Functionally fine today, but a fragile implicit dependency — if that event were ever dropped
or delayed, the frontend has no fallback identifier for the macro it just created.
**Fix:** No functional change required; at minimum, a short comment noting the reliance on
`state-changed` (matching the `WR-04`-style fallback pattern used in `handleLoadProfile`) would make the
dependency explicit for future readers.

---

_Reviewed: 2026-07-21T01:03:07Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
