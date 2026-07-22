---
phase: 09-parallel-macro-execution
reviewed: 2026-07-22T14:43:30Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/platform/macos/observer.rs
  - src-tauri/src/scheduler/mod.rs
  - src/App.tsx
findings:
  critical: 4
  warning: 13
  info: 8
  total: 25
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-07-22T14:43:30Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

This review supersedes the prior `09-REVIEW.md` (dated `2026-07-21T19:56:19Z`) and adds
`src-tauri/src/platform/macos/observer.rs` to the reviewed set for the first time (touched by the
`09-07`/`09-08` gap-closure commits for UAT gaps G-09-1a/b/c). `git diff` between the prior review and
current `HEAD` confirms `src-tauri/src/ipc/mod.rs` and `src-tauri/src/scheduler/mod.rs` are unchanged in
the regions the prior review flagged; `src/App.tsx` gained a delete-macro control (G-09-1b) and a
card-edit target-select value binding (G-09-1c) but the previously-flagged code paths themselves are
otherwise untouched. **Every finding from the prior review is still open** and is carried forward below
(re-verified against current line numbers) so this document remains the single authoritative record.

**The most significant new finding in this pass** is a systemic IPC argument-naming defect that the prior
review's own IN-01 ("macro_id vs id — minor, avoidable naming inconsistency") underestimated: cross-checking
the command signatures in `ipc/mod.rs` against the actual `invoke()` call sites in `App.tsx`, and against the
`tauri`/`tauri-macros` 2.11.1/2.6.1 crates actually pinned in `Cargo.lock`, shows this is not cosmetic —
Tauri's default `ArgumentCase` is `Camel` (confirmed directly against
`tauri-macros-2.6.1/src/command/wrapper.rs:51` and the runtime key lookup in
`tauri-2.11.1/src/ipc/command.rs:97,140-143`), and no command in this file carries
`#[tauri::command(rename_all = "snake_case")]`. Every `invoke()` call that sends a snake_case key for a
parameter whose Rust name contains an underscore (`macro_id`, `target_app`, `trigger_key`) either hard-fails
(non-`Option` params) or **silently discards the value** (`Option<T>` params deserialize to `None` when the
expected camelCase key is absent). This directly breaks per-macro process targeting and per-macro hotkey
editing from the card UI — see CR-01/CR-02. It also means the prior review's CR-02 (macOS hotkey-rebind
ordering) is currently *unreachable in practice*, because the very first call in that sequence
(`unbind_hotkey`) now fails immediately — see the note under CR-04 for how the two findings relate and why
both still need fixing together.

`platform/macos/observer.rs` (new to this review pass) is generally solid — the `TAP_INITIALIZED`/
`TAP_STARTING` double-guard and the CR-01-style guaranteed-drain-then-post pattern in `flush_held_inputs`
are correct and well-documented — but the hotkey-modifier matcher has a real logic bug (WR-01) and the
emergency-stop path uses a signal-y exit code (WR-03).

## Critical Issues

### CR-01: `bind_hotkey` / `unbind_hotkey` always fail — `macro_id` argument name mismatch (new)

**File:** `src-tauri/src/ipc/mod.rs:83`, `src-tauri/src/ipc/mod.rs:102`
**File:** `src/App.tsx:575-576`

**Issue:** `bind_hotkey(state, macro_id: Uuid, keycode: u16, modifiers: u64)` and
`unbind_hotkey(state, macro_id: Uuid)` use Tauri's default argument case (`Camel` — no
`#[tauri::command(rename_all = "snake_case")]` appears anywhere in this file; `grep -rn "rename_all"
src-tauri/src` returns zero hits). Tauri's proc-macro therefore emits a lookup key of `macroId` for this
parameter (`tauri-macros-2.6.1/src/command/wrapper.rs:505-512`, default `ArgumentCase::Camel` at line 51).

`macro_id` is `Uuid` — **not** `Option<Uuid>` — so when the JSON payload lacks a `macroId` key,
`CommandItem::deserialize_json` (`tauri-2.11.1/src/ipc/command.rs:97-103`) returns
`Err("command bind_hotkey missing required key macroId")`.

The frontend calls these with the literal snake_case key:
```ts
// src/App.tsx:575-577
await invoke("unbind_hotkey", { macro_id: id });
await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers });
await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
```
`unbind_hotkey` throws immediately — before `bind_hotkey` or `set_macro_trigger_key` ever run — because
there is no `try/catch` between the three sequential `await`s inside `handleCardSetTriggerKey`. On macOS,
**every attempt to set or change a macro's hotkey from the card UI fails**, and the outer `catch`
mislabels the real cause as a "Hotkey already bound" conflict toast (its regex doesn't match the actual
"missing required key macroId" error string, so it falls back to the generic "another macro" message) —
actively misleading the user about why the action failed.

**Fix:** Add `rename_all = "snake_case"` to both commands:
```rust
#[command(rename_all = "snake_case")]
pub async fn bind_hotkey(
    state: State<'_, StateManager>,
    macro_id: Uuid,
    keycode: u16,
    modifiers: u64,
) -> Result<(), String> { ... }

#[command(rename_all = "snake_case")]
pub async fn unbind_hotkey(
    state: State<'_, StateManager>,
    macro_id: Uuid,
) -> Result<(), String> { ... }
```
(This also resolves prior-review IN-01, which had classified this `macro_id`/`id` naming difference as
purely cosmetic — it is the direct cause of this defect, not an unrelated nit.)

### CR-02: `set_macro_target_app` / `set_macro_trigger_key` silently discard their value — `Option<T>` argument name mismatch (new)

**File:** `src-tauri/src/ipc/mod.rs:46` (`target_app: Option<String>`), `src-tauri/src/ipc/mod.rs:148` (`trigger_key: Option<u16>`)
**File:** `src/App.tsx:577`, `src/App.tsx:579`, `src/App.tsx:615`

**Issue:** Same root cause as CR-01, but these parameters are `Option<T>`. When the expected camelCase key
(`targetApp`, `triggerKey`) is absent, Tauri's `deserialize_option` does **not** error — it calls
`visitor.visit_none()` (`tauri-2.11.1/src/ipc/command.rs:140-143`). Since the frontend sends
`target_app`/`trigger_key` (snake_case), the lookup for `targetApp`/`triggerKey` always misses, and the
value is **silently coerced to `None`** — no error is thrown, no toast shown, the call reports success.

```ts
// src/App.tsx:613-621 — handleCardSetTargetApp
await invoke("set_macro_target_app", { id, target_app: targetApp || null });
// target_app is *always* deserialized as None on the Rust side — the macro's
// target_app is unconditionally cleared to Global, regardless of the dropdown selection.
```
```ts
// src/App.tsx:576-579 — handleCardSetTriggerKey (both macOS and Windows paths)
await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode, modifiers });
// trigger_key is *always* deserialized as None — the card-edit path can never
// actually persist a trigger key, on either platform.
```
Editing an existing macro's target-app or trigger-key from the card UI is a silent no-op that always
resets the field to unset — a direct violation of this project's stated core value ("A macro that was set
up must fire reliably"). It fully explains why editing an existing macro's target app appears to do
nothing, independent of the separately-diagnosed `active_app`-tracking root cause recorded for G-09-1c in
`09-UAT.md` — even with perfectly-tracked `active_app`, this call path can never write a non-null
`target_app`. The Windows hotkey-rebind path (`set_macro_trigger_key` only, no `bind_hotkey`/
`unbind_hotkey`) is *entirely* broken by this bug and was never exercised — Windows UAT was marked
`blocked_by: physical-device` in `09-UAT.md`.

**Fix:** Add `rename_all = "snake_case"` to both commands:
```rust
#[command(rename_all = "snake_case")]
pub async fn set_macro_target_app(
    state: State<'_, StateManager>,
    id: Uuid,
    target_app: Option<String>,
) -> Result<(), String> { ... }

#[command(rename_all = "snake_case")]
pub async fn set_macro_trigger_key(
    state: State<'_, StateManager>,
    id: Uuid,
    trigger_key: Option<u16>,
    modifiers: Option<u64>,
) -> Result<(), String> { ... }
```
Also audit `update_step_interval` (`step_index`, `interval_ms`) and `set_macro_sequence` for the same
class of bug before either is wired up to the frontend (currently unused — see IN-06).

### CR-03: `running_configs` cache goes stale after `UpdateInterval`, defeating CR-04's (scheduler) no-op-restart guarantee and reintroducing Hold-macro flicker (carried forward, still open)

**File:** `src-tauri/src/scheduler/mod.rs:235-248` (stale-write site) and `:258-267` (stale-read / comparison site)

**Issue:** `start_macro` skips a restart only when the macro's current config exactly equals the cached
`RunningConfig` in `self.running_configs` (lines 263-267):
```rust
if let Some(existing) = self.running_configs.get(&macro_id) {
    if existing == &new_running {
        return;
    }
}
```
`RunningConfig` includes `steps: Vec<ActionStep>` (line 128), which for an `InterleavedInterval` step
includes `interval_ms`. This cache is written *only* inside `start_macro` (line 345:
`self.running_configs.insert(macro_id, new_running)`).

`SchedulerIntent::UpdateInterval` (lines 235-248) — the handler backing the `update_step_interval` IPC
command, whose documented purpose is to let the frontend "live-tune timing without restarting the macro"
(ipc/mod.rs:280-281) — mutates `interval_tasks`/`timeline` directly and never touches `running_configs`:
```rust
SchedulerIntent::UpdateInterval(macro_id, step_index, new_ms) => {
    let step_id = StepId { macro_id, step_index };
    if let Some(task) = self.interval_tasks.get_mut(&step_id) {
        // ... updates task.interval / timeline only ...
    }
    // running_configs is never touched here.
}
```
Once a step's interval has been live-tuned this way, `self.running_configs[macro_id]` permanently
disagrees with the macro's actual current `sequence.steps`. The next `start_macro` call for that macro for
*any* unrelated reason (the code's own `CR-04` doc comment at scheduler/mod.rs:255-257 names
"active-app switch" via `reevaluate_all_macros` as a real trigger) fails the equality check purely because
of the stale cached `interval_ms`, falls through to `self.stop_macro(&macro_id).await` (line 270), and
rebuilds the macro from scratch. For a macro combining a `SustainedHold` step with an `InterleavedInterval`
step (exactly the shape the codebase's own `afk_farm_stress_test` exercises), this sends a real
`HoldRelease` followed by a real `HoldStart` — physically releasing and re-pressing the held input. This is
precisely the flicker `CR-04` (scheduler) was written to prevent, reachable through the supported,
documented live-tuning feature. Currently unreachable from the UI today only because `update_step_interval`
is never invoked from `App.tsx` (see IN-06) — but it is fully wired on the backend and has zero test
coverage for this interaction.

**Fix:** Keep `running_configs` in sync inside the `UpdateInterval` handler:
```rust
SchedulerIntent::UpdateInterval(macro_id, step_index, new_ms) => {
    let step_id = StepId { macro_id, step_index };
    if let Some(task) = self.interval_tasks.get_mut(&step_id) {
        let old_fire = task.next_fire;
        task.interval = Duration::from_millis(new_ms.max(5));
        task.next_fire = Instant::now() + task.interval;
        let new_fire = task.next_fire;
        self.remove_from_timeline(&step_id, old_fire);
        self.timeline.entry(new_fire).or_default().push(step_id);
    }
    if let Some(running) = self.running_configs.get_mut(&macro_id) {
        if let Some(ActionStep::InterleavedInterval { interval_ms, .. }) =
            running.steps.get_mut(step_index)
        {
            *interval_ms = new_ms;
        }
    }
}
```
Add a regression test: start a combined Hold+Interval macro, send `UpdateInterval`, then send `StartMacro`
again with the updated interval, and assert `hold_starts == 1` / `hold_releases == 0`.

### CR-04: macOS hotkey rebind destroys the existing binding before the replacement is confirmed — currently masked by CR-01, must be fixed together (carried forward, still open)

**File:** `src/App.tsx:572-595`
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
macro's trigger and rebuilds the platform hotkey registry immediately. If the subsequent `bind_hotkey` call
then fails (the documented conflict-check path, UX-11), the `catch` block only shows a toast — it never
re-establishes the macro's previous binding, leaving the user's macro with **no hotkey at all**.

**Relationship to CR-01:** today this specific failure mode cannot actually surface, because CR-01 makes
`unbind_hotkey` fail on its own *first* line for an unrelated reason (the `macroId` key mismatch) before a
real conflict check is ever reached. Fixing CR-01 alone will make this ordering bug live again — it must be
fixed as part of the same change, not treated as resolved once CR-01 lands.

**Fix:** Do not destroy the existing binding until the new one is confirmed — attempt the new bind first,
and only clear the old key on success:
```ts
async function handleCardSetTriggerKey(id: string, nativeCode: number, modifiers: number) {
    try {
      if (IS_MACOS) {
        await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers });
        await invoke("unbind_hotkey", { macro_id: id }); // release the OLD binding only after the new one succeeds
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
(Adjust to whatever the backend's conflict-check actually allows for a macro rebinding over its own prior
key — may instead need an explicit rollback on failure.)

## Warnings

### WR-01: `HotkeyBinding::matches` uses a subset bit-test instead of exact modifier equality (new)

**File:** `src-tauri/src/platform/macos/observer.rs:50-52`
**Issue:**
```rust
fn matches(&self, keycode: u16, flags: CGEventFlags) -> bool {
    keycode == self.keycode && (flags.bits() & self.modifiers) == self.modifiers
}
```
This only checks that all of `self.modifiers` are *present* in the held flags — it does not reject *extra*
held modifiers. Two concrete failure modes: (1) a binding with `modifiers == 0` matches **any** modifier
combination held down, because `(flags.bits() & 0) == 0` is trivially true regardless of `flags` — a macro
bound to plain `F5` also fires when the user presses `Cmd+Shift+F5` for something unrelated; (2) two
bindings sharing a keycode where one's modifiers are a strict subset of another's (`Cmd+5` vs `Cmd+Shift+5`)
can both `matches()` a single keypress, and the loop fires whichever is iterated first — arbitrary from the
user's perspective.
**Fix:** Compare against exact equality of the tracked modifier bits:
```rust
const TRACKED_MODS: u64 = 0x20000 | 0x40000 | 0x80000 | 0x100000; // Shift|Control|Alt|Cmd
fn matches(&self, keycode: u16, flags: CGEventFlags) -> bool {
    keycode == self.keycode && (flags.bits() & TRACKED_MODS) == self.modifiers
}
```

### WR-02: `computeRunningState` renders "waiting" with the same base color as "firing" (new)

**File:** `src/App.tsx:1256-1268`
**Issue:** The status-dot color switch maps both `"firing"` and `"waiting"` to `bg-success` (green) — the
only difference is the `animate-pulse` class on `"firing"`. A macro that is enabled but not currently
injecting because its target app isn't focused (`"waiting"`) is a static green dot, the same base color
used to mean "actively running." A user glancing at the card list can mistake a paused, target-scoped
macro for one that is actually firing.
**Fix:**
```tsx
case "waiting":
  return "w-2 h-2 rounded-full bg-text-dim";
```

### WR-03: Emergency stop exits with code 1 instead of 0 (new)

**File:** `src-tauri/src/platform/macos/observer.rs:412`
**Issue:** `process::exit(1)` is used for the user-triggered, intentional Cmd+Shift+Q emergency-stop-and-quit
path. Exit code `1` conventionally signals abnormal/error termination; depending on how the app is launched,
this can surface a "quit unexpectedly" dialog or trigger a supervisor restart policy for what is actually a
clean, deliberate shutdown.
**Fix:** Use `process::exit(0)` for this intentional-termination path.

### WR-04: Windows `PlatformObserver` is not retained via `app.manage`, unlike the macOS observer (new)

**File:** `src-tauri/src/lib.rs:73-79`
**Issue:**
```rust
#[cfg(target_os = "windows")]
{
    platform::windows::set_state_tx(state_tx);
    let mut observer = platform::windows::WindowsPlatformObserver::new();
    observer.start_observing();
    // Observer is long-lived; it does not implement Drop.
}
```
`observer` is dropped at the end of this block. The macOS branch immediately above explicitly avoids this
via `app.manage(observer)` (commented "Observer is owned by Tauri managed state for full app lifetime
(SAFE-02)"). The Windows branch instead relies on the invariant "`WindowsPlatformObserver` does not
currently implement `Drop`" holding forever — a future change adding cleanup logic to `Drop` (e.g.
unhooking the Win32 hook, mirroring macOS's `stop_observing`) would silently and immediately undo
`start_observing()`, breaking all Windows hotkeys/observation with no compile-time signal.
**Fix:** `app.manage(observer);` for symmetry with the macOS branch and future-proofing.

### WR-05: `handleCreateMacro` and `handleCardSetTriggerKey` mislabel every failure as a hotkey conflict (carried forward)

**File:** `src/App.tsx:494-508`, `src/App.tsx:583-594`
**Issue:** Both `catch` blocks unconditionally render the "Hotkey already bound" conflict toast for *any*
thrown error from `invoke()` — the regex match result is used only to pick the macro name (hardcoded
fallback "another macro"), but the toast fires regardless of whether the regex matched at all. Any other
failure (a malformed field causing an IPC deserialization error — see CR-01/CR-02 above for concrete new
examples of this — the state channel being closed, or a future validation rule) shows a fabricated
conflict message even when there is no conflict and possibly no hotkey involved.
**Fix:**
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

### WR-06: Card trigger-key edit UI is left stuck after a failed bind (carried forward)

**File:** `src/App.tsx:572-595`, `1352-1429`
**Issue:** `handleCardSetTriggerKey`'s `catch` block never resets `editingCardId`/`editingField` — cleared
only on the success path. The card view (lines 1352-1429) renders the "Press…" chip whenever
`editingCardId() === macro.id && editingField() === "key"`, independent of `triggerKeyRecording()`. A
failed bind (e.g. the CR-04 conflict scenario) leaves the card frozen showing "Press…" with an
active-looking accent border even though key capture already ended.
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

### WR-07: No client-side validation prevents negative, zero, or invalid macro intervals (carried forward)

**File:** `src/App.tsx:1104-1112`, `src/App.tsx:456`
**Issue:** The interval `<input type="number">` has no `min` attribute, and
`const interval = parseInt(newMacroInterval()) || 100;` only guards `NaN` — and, because `||` treats `0` as
falsy too, it silently overwrites a deliberately-entered `"0"` with `100`, inconsistent with the backend's
own clamp (`IntervalTask::new`, scheduler/mod.rs:81: `interval_ms.max(5)`) which would otherwise have
produced `5ms`. Negative values (`-50`) are not caught here at all and are sent straight through, failing
JSON→Rust `u64` deserialization with an opaque error that — per WR-05 — gets mis-displayed as a hotkey
conflict.
**Fix:**
```tsx
<input id="input-macro-interval" type="number" min="1" ... />
```
```ts
const parsed = parseInt(newMacroInterval());
const interval = Number.isFinite(parsed) && parsed >= 1 ? parsed : 100;
```

### WR-08: Toast auto-dismiss timers are not cleared on component unmount (carried forward)

**File:** `src/App.tsx:236-248` (`_conflictErrorTimer` / `showConflictError`), `625-628` (`showProfileMsg`)
**Issue:** The file is otherwise careful about clearing pending timers on unmount —
`clearPending()`/`clearImPending()` are both invoked from the top-level `onCleanup` (lines 380-387).
`_conflictErrorTimer` and the anonymous `setTimeout` inside `showProfileMsg` have no equivalent cleanup —
only cleared when a new toast of the same kind supersedes the old one, never on teardown. If the component
unmounts while one is pending (e.g. during HMR), the callback still fires and calls a signal setter on a
disposed reactive scope.
**Fix:** Track both timer handles at component scope and clear them in the existing `onCleanup`.

### WR-09: `delete_profile` / `load_profile` do not sanitize the profile name before handing it to the persistence layer (carried forward)

**File:** `src-tauri/src/ipc/mod.rs:343-354` (`load_profile`), `:360-369` (`delete_profile`)
**Issue:** `save_profile` explicitly sanitizes the incoming `name` up front (documented under CR-03,
lines 301-318) specifically so the on-disk filename always matches the stored name. `delete_profile` and
`load_profile` pass the raw, unsanitized `name` straight through with no equivalent guard in this file —
`delete_profile` only special-cases the literal string `"default"`. `persistence.rs` is out of scope for
this review pass, so it's possible `ProfileManager` sanitizes internally — but that can't be confirmed from
these five files, and the asymmetry with `save_profile`'s explicit, documented sanitization is worth a
follow-up check for a path-traversal vector (e.g. a crafted `name` containing `../`).
**Fix:** Apply `ProfileManager::sanitize_name` (or equivalent) to `name` in `delete_profile`/`load_profile`
before it reaches the persistence layer, or confirm/document that `ProfileManager`'s internal methods
already do this unconditionally.

### WR-10: Startup profile restoration failure is completely silent (carried forward)

**File:** `src-tauri/src/lib.rs:32-44`
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
The oneshot receiver is dropped (`_rx`) and the `.send(...).await` result is discarded via `let _ =`. If
the default profile fails to load at startup (corrupted JSON, permissions error, etc.), the user launches
into an empty macro list with **zero indication anything went wrong** — no toast, no banner, no
release-build log line. The existing "Auto-Save Error Banner" (`App.tsx:1026-1045`) covers *save* failures
only.
**Fix:** Await the startup load's oneshot response and, on failure, emit an event the frontend can surface
(e.g. an `auto-load-error` event feeding a banner analogous to the existing auto-save-error one).

### WR-11: Startup data fetch uses `Promise.all` — one failing call discards all successful results (carried forward)

**File:** `src/App.tsx:291-329`
**Issue:** The initial effect fetches seven independent pieces of state via `Promise.all`. If any single
call rejects, none of the setters run — including `setState(stateData)`, which populates the macro list.
The `catch` only logs to `console.error`; there is no user-visible error state and no retry, so a single
transient IPC hiccup leaves the whole dashboard showing its null/default state indefinitely.
**Fix:** Use `Promise.allSettled` and apply whichever results succeeded, or surface a visible error banner
(mirroring the existing `saveError` pattern) when any startup call fails.

### WR-12: Event-listener promises lack a `.catch`, risking an unhandled rejection (carried forward)

**File:** `src/App.tsx:332-340` (`state-changed`), `345-352` (`auto-save-error`)
**Issue:**
```ts
const unlisten = listen<AppState>("state-changed", (event) => { ... });
onCleanup(() => {
  unlisten.then((fn) => fn());
});
```
If `listen(...)` ever rejects (e.g. the IPC layer is torn down mid-registration), `unlisten.then(...)` has
no rejection handler, producing an unhandled promise rejection.
**Fix:** `unlisten.then((fn) => fn()).catch(() => {});`.

### WR-13: Guaranteed `.await` delivery for `HoldStart` widens the Scheduler's single-task stall radius to *all* macros (carried forward)

**File:** `src-tauri/src/scheduler/mod.rs:296-325`
**Issue:** A knowing extension of an already-accepted trade-off (comments at :300-314, :372-380), not filed
as a defect in the fix itself — worth calling out for a phase whose purpose is *parallel* macro execution.
`Scheduler::run()` is a single task by design — no per-macro spawns. Every `.await`-blocking send inside
`handle_intent` (`HoldStart` in `start_macro`, plus the pre-existing `HoldRelease` in
`release_holds`/`StopAll`) blocks the entire `run()` loop, including every other macro's interval timers,
until `action_tx` has a free slot. With many concurrently running macros contending for the 1024-capacity
`action_tx` channel (lib.rs:26), one Hold-mode macro's `start_macro` can now delay every other macro's
interval fires for as long as the send stays blocked.
**Fix:** No change required if the trade-off is accepted. Otherwise, bound the worst case with a short
`try_send` retry/backoff loop (with a hard timeout) instead of an unbounded `.await`.

## Info

### IN-01: Inconsistent IPC argument naming for the macro identifier — upgraded from cosmetic to functional (carried forward, see CR-01)

**File:** `src-tauri/src/ipc/mod.rs:80-108`
**Issue:** `bind_hotkey`/`unbind_hotkey` take `macro_id: Uuid` while every other per-macro command takes
`id: Uuid`. The prior review filed this as a purely cosmetic inconsistency; this pass shows it is the
literal parameter name feeding the `Camel`-case argument key Tauri derives, and is the direct cause of
CR-01 above. Retained here as a residual naming-consistency note once CR-01 is fixed.
**Fix:** Rename `macro_id` → `id` in both commands' signatures (in addition to, not instead of, adding
`rename_all = "snake_case"` per CR-01) for consistency with the rest of the IPC surface.

### IN-02: macOS platform detection is duplicated three times in `App.tsx` (carried forward)

**File:** `src/App.tsx:145`, `173`, `287`
**Issue:** `navigator.userAgent.toLowerCase().includes("mac")` is independently re-declared as `IS_MACOS`
inside `computeModifiers`, inside `modifierChips`, and again inside `App()`.
**Fix:** Hoist a single module-level `const IS_MACOS = ...` and reference it from all three sites.

### IN-03: `add_macro`'s returned UUID is fetched but discarded (carried forward)

**File:** `src/App.tsx:484`
**Issue:** `await invoke<string>("add_macro", { config });` discards the backend-generated UUID, relying
solely on the subsequent `state-changed` event to populate the real macro ID into `state()`. Fragile
implicit dependency — if that event were dropped or delayed, the frontend has no fallback identifier.
**Fix:** No functional change required; a short comment noting the reliance on `state-changed` would make
the dependency explicit.

### IN-04: `loading` signal is written but never read — dead state (carried forward)

**File:** `src/App.tsx:214`, `326`
**Issue:** `const [, setLoading] = createSignal(true);` — the getter is discarded, `setLoading(false)` is
called once, and nothing in the component ever reads a `loading()` value.
**Fix:** Remove the signal entirely, or wire it into the render (e.g. a loading skeleton) so it has a
consumer.

### IN-05: `get_active_app` performs a full `GetState` round-trip just to extract one field (carried forward)

**File:** `src-tauri/src/ipc/mod.rs:64-73`
**Issue:** `get_active_app` sends `Intent::GetState(tx)` (same intent `get_state` uses) and discards
everything except `app_state.active_app` — serializing the entire `AppState`, including the full macro map,
to read one `Option<String>`.
**Fix:** Consider a dedicated lightweight `Intent::GetActiveApp` variant, or document that the trade-off is
deliberate.

### IN-06: `set_macro_sequence` and `update_step_interval` IPC commands are never invoked from the frontend (new)

**File:** `src-tauri/src/ipc/mod.rs:268-294`
**Issue:** Both commands are fully implemented and registered in `invoke_handler!` (`lib.rs:95-96`), but
`src/App.tsx` contains no call site for either. Multi-step sequence editing and per-step live interval
tuning have no UI path to reach them today (this is also why CR-03's scheduler cache-staleness bug is
currently unreachable from the shipped UI).
**Fix:** Either wire these into the frontend or explicitly note near the command definitions that they are
intentionally deferred, to avoid the appearance of dead backend surface.

### IN-07: `handleDeleteProfile` has no confirmation prompt, unlike `handleRemoveMacro` (new)

**File:** `src/App.tsx:674-689` (compare `handleRemoveMacro`, `src/App.tsx:519-526`)
**Issue:** `handleRemoveMacro` guards its destructive action with `window.confirm(...)`.
`handleDeleteProfile` deletes immediately on click with no confirmation, despite being an equally
destructive, irreversible action (unlinking a saved profile file).
**Fix:**
```ts
async function handleDeleteProfile(name: string) {
  if (name === "Default") return;
  if (!window.confirm(`Delete profile "${name}"?`)) return;
  ...
}
```

### IN-08: `list_running_apps` macOS branch omits the explicit `return` present in the Windows branch (new)

**File:** `src-tauri/src/ipc/mod.rs:126-137`
**Issue:** The Windows arm uses `return crate::platform::windows::list_running_apps_impl();` while the
macOS arm relies on being the function's tail expression. Functionally equivalent today only because the
`#[cfg(...)]` blocks are mutually exclusive per build target — a minor stylistic trap for future edits.
**Fix:** Use `return ...;` consistently in both platform arms.

---

_Reviewed: 2026-07-22T14:43:30Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
