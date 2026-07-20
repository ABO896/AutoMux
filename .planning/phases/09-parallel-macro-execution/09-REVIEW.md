---
phase: 09-parallel-macro-execution
reviewed: 2026-07-20T18:59:11Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/scheduler/mod.rs
  - src/App.tsx
findings:
  critical: 2
  warning: 4
  info: 3
  total: 9
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-07-20T18:59:11Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

This is a full re-review of the phase's current cumulative diff (parallel macro execution: scheduler, IPC surface, app bootstrap, dashboard UI). The prior review's CR-01 (held-indicator mismatch — `computeRunningState` misclassifying Hold-mode macros as "firing") is confirmed fixed: the function now checks `macro.trigger_mode === "Hold"` explicitly and returns `"held"` before ever inspecting the persisted step shape, with a comment explaining why the persisted shape alone is insufficient.

Two new BLOCKER-level issues were found in this pass, both directly relevant to a "parallel macro execution" phase:

1. The Scheduler's per-macro stop path (`stop_macro` → `release_holds`) uses fire-and-forget `try_send` for `HoldRelease` messages, unlike the emergency `StopAll` path, which was hardened (per the existing CR-02 comment in the same file) to use `.await` for guaranteed delivery. Under the exact channel-saturation conditions this phase is designed to stress (many concurrent macros), disabling/stopping an individual held-mode macro can silently and permanently leave a physical key/mouse button held down, with no retry path short of a full emergency stop.
2. `App.tsx`'s `handleCardSetTriggerKey` (macOS path) destroys an existing hotkey binding (`unbind_hotkey`) *before* confirming the new one is accepted (`bind_hotkey`). If the new key conflicts with another macro, the old binding is permanently lost even though the user only intended to attempt a change.

Four Warnings and three Info items round out the findings — mostly UX/error-recovery gaps and minor naming/cleanup inconsistencies. No hardcoded secrets, `eval`, `innerHTML`/`dangerouslySetInnerHTML`, or SQL/command-injection patterns were found in any of the four files.

## Critical Issues

### CR-01: Per-macro stop can silently leave a physical input stuck down

**File:** `src-tauri/src/scheduler/mod.rs:367-391` (via `stop_macro` at 344-365, invoked from `handle_intent`'s `StopMacro` arm at 211-213 and from `start_macro`'s restart path at 270)

**Issue:** `release_holds()` removes the macro's holds from `active_holds` unconditionally, then attempts to notify the StateActor of each `HoldRelease` via `self.action_tx.try_send(...)`. `try_send` is non-blocking and silently drops on backpressure — the failure is only logged, and only in debug builds, via `ACTION_DROP_COUNT`. Because `active_holds.remove(macro_id)` already happened *before* the send attempt, a dropped message is unrecoverable: the Scheduler no longer believes it owns that hold, so nothing will ever re-attempt the release.

Contrast this with `SchedulerIntent::StopAll` (lines 214-234), which was explicitly hardened per the CR-02 comment already in this file to use `.await` (guaranteed delivery, blocking until the channel has room) specifically because dropped `HoldRelease` messages leave real, physically-stuck input. The identical reasoning applies to a single macro's normal stop/disable path — under load (many concurrent macros firing intervals is precisely what this phase adds), the 1024-slot `action_tx` channel can legitimately fill up, and a user disabling one held-mode macro (e.g. a sustained right-click macro for an AFK farm) can be left with that mouse button physically down in-game with no on-screen indication anything went wrong. This gap is invisible in release builds — `ACTION_DROP_COUNT` and its logging are both `#[cfg(debug_assertions)]`-only, so there is no diagnostic signal at all for production users hitting this.

This directly threatens the project's stated Core Value: "A macro that was set up must fire reliably."

**Fix:** Route `release_holds` through the same guaranteed-delivery mechanism as `StopAll`. The simplest correct fix is to make `stop_macro`/`release_holds` `async` and `.await` the send, exactly as `StopAll` already does:

```rust
async fn release_holds(&mut self, macro_id: &Uuid) {
    if let Some(holds) = self.active_holds.remove(macro_id) {
        for input in holds {
            // CR-01: guaranteed delivery — matches the StopAll emergency path.
            // A dropped HoldRelease here means a permanently-stuck physical input.
            let _ = self.action_tx.send(ActionReady {
                macro_id: *macro_id,
                action_type: ActionType::HoldRelease(input),
                fired_at: Instant::now(),
            }).await;
        }
    }
}
```
(`stop_macro` and its callers — `handle_intent`, `start_macro`'s restart path — would need to become `async`/`.await` this call as well.)

---

### CR-02: Hotkey rebind on a card destroys the existing binding before the new one is confirmed

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
On macOS, `unbind_hotkey` runs first. Per its own doc comment in `ipc/mod.rs:95-98`, this "clears the macro's trigger and rebuilds the platform HOTKEY_BINDINGS registry" — i.e. it mutates persisted state immediately. If the subsequent `bind_hotkey` call then fails (the documented conflict-check path, UX-11), the `catch` block only shows a toast (`showConflictError`) — it never re-establishes the macro's previous binding. A user attempting to change an existing hotkey to one that happens to conflict with another macro ends up with **no hotkey at all** on the macro they were editing, instead of keeping the original binding.

**Fix:** Do not destroy the existing binding until the new one is confirmed. Attempt the new bind first, and only clear the old key on success:

```ts
async function handleCardSetTriggerKey(id: string, nativeCode: number, modifiers: number) {
    const macro = state()?.macros[id];
    const hadPrevKey = macro?.trigger_key != null;
    try {
      if (IS_MACOS) {
        // Attempt the new binding first; only clear the old one once accepted.
        await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers });
        if (hadPrevKey) {
          await invoke("unbind_hotkey", { macro_id: id });
        }
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
(Adjust ordering to whatever the backend's conflict-check actually requires — the essential fix is that the destructive `unbind_hotkey` call must not run unconditionally before the replacement binding is guaranteed to succeed.)

## Warnings

### WR-01: Card trigger-key edit UI gets stuck after a conflict error

**File:** `src/App.tsx:563-586`, `1332-1401`

**Issue:** `handleCardSetTriggerKey`'s `catch` block never resets `editingCardId`/`editingField` (they are only cleared on the success path). The card view (`1348-1401`) shows the "Press…" chip whenever `editingCardId() === macro.id && editingField() === "key"`, regardless of `triggerKeyRecording()`. A failed bind therefore leaves the card frozen showing "Press…" with an active-looking accent border even though key capture has already ended (the `keydown` listener was removed inside `startCapture`'s `onKeyDown` before `onCommit` — i.e. `handleCardSetTriggerKey` — was even invoked). The card is only recoverable if the user notices and clicks the small "✕" inside that chip.

**Fix:** Reset the editing state in the `catch` block too:
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

### WR-02: No client-side validation on macro interval allows invalid values to reach the backend with a misleading error

**File:** `src/App.tsx:453-509` (`handleCreateMacro`)

**Issue:** `const interval = parseInt(newMacroInterval()) || 100;` only guards against `NaN`/`0` (both fall back to 100, since `0` is falsy) but does not reject negative values — `parseInt("-50")` is `-50`, which is truthy, so it is sent straight through in the `add_macro` payload as `interval_ms: -50`. The Rust side declares `MacroConfig.interval_ms: u64`, which cannot deserialize a negative number, so `add_macro` fails with a generic serde error. The `catch` block in `handleCreateMacro` assumes every failure is a hotkey conflict (it regex-matches for `is already assigned to "..."`, falling back to a generic "another macro" message), so the user sees a confusing "hotkey conflict" toast for what is actually an invalid-interval input error.

**Fix:** Clamp/validate before sending (mirroring the scheduler's own `.max(5)` floor):
```ts
const parsed = parseInt(newMacroInterval());
const interval = Number.isFinite(parsed) && parsed >= 5 ? parsed : 100;
```
and/or disable the Create button when the interval field holds an invalid value, similar to the existing `disabled={!newMacroName().trim()}` guard.

### WR-03: Toast/message auto-dismiss timers are not cleared on unmount, unlike the rest of the file's cleanup pattern

**File:** `src/App.tsx:241-248` (`_conflictErrorTimer` / `showConflictError`), `616-619` (`showProfileMsg`)

**Issue:** The file is otherwise careful about clearing pending timers on unmount — `clearPending()`/`clearImPending()` are both invoked from the top-level `onCleanup` (lines 380-387), per the documented WR-01/WR-03/WR-08 conventions already in the file. `_conflictErrorTimer` (conflict toast) and the anonymous `setTimeout` inside `showProfileMsg` (profile toast) have no equivalent cleanup — they are only cleared when a new toast supersedes the old one, never on component teardown. If the component unmounts while one of these timers is pending (e.g. during HMR), the timer will still fire and call a signal setter on a torn-down component.

**Fix:** Track both timer handles and clear them in the top-level `onCleanup`:
```ts
onCleanup(() => {
  if (_conflictErrorTimer !== null) clearTimeout(_conflictErrorTimer);
  if (_profileMsgTimer !== null) clearTimeout(_profileMsgTimer);
  ...
});
```

### WR-04: `delete_profile` / `load_profile` do not sanitize the profile name before handing it to the persistence layer

**File:** `src-tauri/src/ipc/mod.rs:343-369`

**Issue:** `save_profile` explicitly sanitizes the incoming `name` up front (documented under CR-03, lines 301-318) specifically so the on-disk filename always matches the stored name. `delete_profile` (360-369) and `load_profile` (343-355) pass the raw, unsanitized `name` straight through to `profile_mgr.delete_profile(&name)` / `Intent::LoadProfile(name, tx)` with no equivalent guard in this file. `persistence.rs` is out of scope for this review pass, so it's possible `ProfileManager` sanitizes internally before building a filesystem path — but that can't be confirmed here, and the asymmetry with `save_profile`'s explicit, documented sanitization is worth a follow-up check for a path-traversal vector (e.g. a crafted `name` containing `../`).

**Fix:** Apply the same `ProfileManager::sanitize_name` (or equivalent validation) to `name` in `delete_profile`/`load_profile` before it reaches the persistence layer, or confirm and document that `ProfileManager`'s internal methods already do this unconditionally.

## Info

### IN-01: Inconsistent IPC argument naming for the macro identifier

**File:** `src-tauri/src/ipc/mod.rs:80-108`

**Issue:** `bind_hotkey` and `unbind_hotkey` take `macro_id: Uuid`, while every other per-macro command (`remove_macro`, `set_macro_enabled`, `set_macro_target_app`, `set_macro_trigger_key`, `set_macro_sequence`, `update_step_interval`) takes `id: Uuid`. This is a minor but avoidable naming inconsistency across an otherwise uniform IPC surface.

**Fix:** Rename `macro_id` → `id` in `bind_hotkey`/`unbind_hotkey` for consistency (this would also require updating the two `App.tsx` call sites that currently pass `{ macro_id: id, ... }`).

### IN-02: Duplicated platform-detection logic between component scope and module scope

**File:** `src/App.tsx:144-161` (`computeModifiers`), `172-191` (`modifierChips`), `287` (component-scoped `IS_MACOS`)

**Issue:** `computeModifiers`/`modifierChips` are module-level functions that each independently recompute `navigator.userAgent.toLowerCase().includes("mac")`, duplicating the `IS_MACOS` constant already computed inside `App()`. This is called out in the existing code comments as intentional (the helpers need to be callable outside the component closure), but it's still a duplication risk — if the detection method ever changes, three call sites must be updated in lockstep rather than one.

**Fix:** Pass `IS_MACOS` as a parameter into `computeModifiers`/`modifierChips` from the call sites inside `App()`, or hoist a single memoized `isMacOS()` module-level helper that both the component and the free functions call.

### IN-03: `add_macro`'s returned UUID is fetched but never used

**File:** `src/App.tsx:483-484`

**Issue:** `await invoke<string>("add_macro", { config });` discards the backend-generated UUID entirely, relying solely on the subsequent `state-changed` event to populate the real macro ID into `state()`. This is functionally fine today (the event does arrive), but if that event were ever dropped or delayed, the frontend would have no fallback identifier for the macro it just created. Not a bug today, but a fragile implicit dependency worth documenting.

**Fix:** No functional change required; consider a one-line comment noting the reliance on `state-changed` for the new macro to appear, matching the existing WR-04 fallback pattern already used for `handleLoadProfile`'s explicit re-fetch.

---

_Reviewed: 2026-07-20T18:59:11Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
