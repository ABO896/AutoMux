---
phase: 09-parallel-macro-execution
reviewed: 2026-07-21T19:56:19Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/scheduler/mod.rs
  - src/App.tsx
findings:
  critical: 2
  warning: 9
  info: 5
  total: 16
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-07-21T19:56:19Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

This review supersedes the prior `09-REVIEW.md` (commit `56bb0f8`). Since that review was written, only
`src-tauri/src/scheduler/mod.rs` has changed (commits `79d39f8` test + `5e37063` fix, both `09-06`) —
`git diff 56bb0f8..HEAD` confirms `src/App.tsx`, `src-tauri/src/ipc/mod.rs`, and `src-tauri/src/lib.rs`
are byte-for-byte unchanged. Accordingly:

**CR-01 (prior review) is verified CLOSED.** `Scheduler::start_macro`'s `SustainedHold` branch
(scheduler/mod.rs:315-325) now sends `HoldStart` via `self.action_tx.send(...).await`, mirroring the
guaranteed-delivery pattern already used by `release_holds` (:381-394) and `StopAll` (:214-234). The
now-obsolete `try_send`/drop-count/`eprintln` at the `HoldStart` site was removed. Two saturation-forcing
regression tests (`stop_macro_release_delivered_under_saturation`, `start_macro_hold_start_delivered_under_saturation`,
scheduler/mod.rs:889-1055) exercise both the release and start paths under a capacity-1 channel and pass,
consistent with the fix actually taking effect.

**Every other finding from the prior review is still open** — none of the files those findings live in
(`App.tsx`, `ipc/mod.rs`, `lib.rs`) were touched by the `09-06` fix. They are carried forward below
(re-verified against the current file contents, same line numbers) so this document remains the single
authoritative record of outstanding Phase 9 issues. **CR-02 (macOS hotkey rebind destroys the existing
binding before the replacement is confirmed) is still Critical and still unfixed** — it should not be
considered resolved just because CR-01 was.

**New finding in this pass (CR-01, renumbered — distinct from the now-closed prior CR-01):** tracing the
consequences of the `09-06` fix together with the pre-existing `CR-04` no-op-restart cache surfaced a
previously-undetected regression: `Scheduler.running_configs` (the cache `CR-04` introduced specifically
to prevent unwanted Hold-mode restart flicker) goes silently stale whenever a step's interval is
live-tuned via `update_step_interval`, because the `SchedulerIntent::UpdateInterval` handler never
updates it. Any later `start_macro` call for that macro (which the codebase's own `CR-04` comment says
happens on "unrelated state changes... reevaluate_all_macros") will then see a false config mismatch and
force a full stop/restart — reintroducing the exact Hold-macro flicker `CR-04` was built to eliminate, for
any macro that combines a `SustainedHold` step with a live-tuned `InterleavedInterval` step (the
codebase's own "AFK Fish Farm" stress-test shape). Details below.

A handful of additional frontend warnings/info items (startup-fetch error handling, an unhandled-rejection
risk, and dead state) round out this pass.

## Critical Issues

### CR-01: `running_configs` cache goes stale after `UpdateInterval`, defeating CR-04's no-op-restart guarantee and reintroducing Hold-macro flicker

**File:** `src-tauri/src/scheduler/mod.rs:235-248` (stale-write site) and `:258-267` (stale-read / comparison site)

**Issue:**
`start_macro` skips a restart only when the macro's current config exactly equals the cached
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

But `SchedulerIntent::UpdateInterval` (lines 235-248) — the handler backing the `update_step_interval` IPC
command, whose entire documented purpose is to let the frontend "live-tune timing without restarting the
macro" (ipc/mod.rs:280-281) — mutates `interval_tasks`/`timeline` directly and never touches
`running_configs`:
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
disagrees with the macro's actual current `sequence.steps` (the Scheduler's *own* cache is what is wrong —
not the source-of-truth `AppState`). The next time `start_macro` is invoked for that macro for *any*
unrelated reason — the `CR-04` doc comment (scheduler/mod.rs:255-257) explicitly names "unrelated state
changes (e.g. active-app switch) trigger reevaluate_all_macros" as a real, common trigger — the equality
check at line 264 fails purely because of the stale cached `interval_ms`, so `start_macro` falls through
to `self.stop_macro(&macro_id).await` (line 270) and rebuilds the macro from scratch.

For a macro that combines a `SustainedHold` step with an `InterleavedInterval` step (exactly the shape
the codebase's own `afk_farm_stress_test` exercises: Hold Right-Click + Interval Left-Click every 50ms,
`trigger_mode: Pulse`), this stop/restart sends a real `HoldRelease` followed by a real `HoldStart` for
the held input — i.e. it physically releases and re-presses the held mouse button/key. This is precisely
the flicker `CR-04` was written to prevent, reachable through the *supported, documented* live-tuning
feature the phase shipped. No existing test exercises `UpdateInterval` together with a subsequent
`start_macro` call, so this regression has zero test coverage.

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

    // Keep the CR-04 restart-avoidance cache in sync so a later
    // start_macro() call (e.g. via reevaluate_all_macros) does not see a
    // false mismatch and force an unwanted stop/restart.
    if let Some(running) = self.running_configs.get_mut(&macro_id) {
        if let Some(ActionStep::InterleavedInterval { interval_ms, .. }) =
            running.steps.get_mut(step_index)
        {
            *interval_ms = new_ms;
        }
    }
}
```
Add a regression test that: starts a combined Hold+Interval macro, sends `UpdateInterval` for the
interval step, then sends `StartMacro` again with a config reflecting the new interval, and asserts
`hold_starts == 1` / `hold_releases == 0` (no restart flicker).

### CR-02: macOS hotkey rebind destroys the existing binding before the replacement is confirmed (carried forward — still open)

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
Re-verified unchanged against current source. On macOS, `unbind_hotkey` runs first and — per its own doc
comment in `ipc/mod.rs:95-98` — clears the macro's trigger and rebuilds the platform hotkey registry
immediately (mutates persisted state). If the subsequent `bind_hotkey` call then fails (the documented
conflict-check path, UX-11), the `catch` block only shows a toast — it never re-establishes the macro's
previous binding. A user attempting to change an existing, working hotkey to one that happens to conflict
with another macro's binding ends up with **no hotkey at all** on the macro they were editing. This
directly undermines the project's stated Core Value ("A macro that was set up must fire reliably") — an
unrelated user action (editing one macro's hotkey) can silently disable another, previously-working
macro's ability to be triggered.

**Fix:** Do not destroy the existing binding until the new one is confirmed — attempt the new bind first,
and only clear the old key on success:
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
(Adjust to whatever the backend's conflict-check actually allows — e.g. if `bind_hotkey` refuses to
register a second binding for the same macro without an explicit unbind first, the fix instead needs an
explicit rollback: on `bind_hotkey` failure, re-issue `bind_hotkey`/`set_macro_trigger_key` with the
macro's previous `trigger_key`/`trigger_modifiers` before surfacing the conflict.)

## Warnings

### WR-01: `handleCreateMacro` and `handleCardSetTriggerKey` mislabel every failure as a hotkey conflict (carried forward)

**File:** `src/App.tsx:494-508`, `src/App.tsx:574-585`
**Issue:** Both `catch` blocks unconditionally render the "Hotkey already bound" conflict toast for *any*
thrown error from `invoke()`, not just actual conflict errors — the regex match result is used only to
pick the macro name, with a hardcoded fallback ("another macro"), but the toast fires regardless of
whether the regex matched at all. If `add_macro` (or `bind_hotkey`/`unbind_hotkey`/`set_macro_trigger_key`)
fails for any other reason (a malformed numeric field causing an IPC deserialization error — see WR-05
below — the state channel being closed, or any future backend validation rule), the user is shown a
fabricated "Hotkey already bound … is already assigned to 'another macro'" message even when there is no
conflict and possibly no hotkey involved at all.
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

### WR-02: Card trigger-key edit UI is left stuck after a failed bind (carried forward)

**File:** `src/App.tsx:563-586`, `1332-1401`
**Issue:** `handleCardSetTriggerKey`'s `catch` block never resets `editingCardId`/`editingField` — they
are only cleared on the success path. The card view (lines 1348-1401) renders the "Press…" chip whenever
`editingCardId() === macro.id && editingField() === "key"`, independent of `triggerKeyRecording()`. A
failed bind (e.g. the CR-02 conflict scenario above) leaves the card frozen showing "Press…" with an
active-looking accent border even though key capture already ended — the `keydown` listener was already
removed inside `startCapture`'s `onKeyDown` before `onCommit` (i.e. `handleCardSetTriggerKey`) was even
invoked. The card is only recoverable if the user notices the small "✕" inside that stale chip.
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

### WR-03: No client-side validation prevents negative, zero, or invalid macro intervals (carried forward)

**File:** `src/App.tsx:1095-1103`, `src/App.tsx:456`
**Issue:** The interval `<input type="number">` has no `min` attribute, and
`const interval = parseInt(newMacroInterval()) || 100;` only guards `NaN`-producing input — and, because
`||` treats `0` as falsy too, it also silently overwrites a deliberately-entered `"0"` with `100`, which
is inconsistent with the backend's own clamp (`IntervalTask::new`, scheduler/mod.rs:81:
`interval_ms.max(5)`) that would otherwise have produced `5ms`. Negative values (e.g. `-50`) are not
caught by this check at all and are sent straight through: `interval_ms: -50` reaches the Rust
`u64` field and fails at JSON→Rust deserialization with an opaque error, which — per WR-01 — then gets
mis-displayed as a "Hotkey already bound" conflict.
**Fix:**
```tsx
<input id="input-macro-interval" type="number" min="1" ... />
```
```ts
const parsed = parseInt(newMacroInterval());
const interval = Number.isFinite(parsed) && parsed >= 1 ? parsed : 100;
```

### WR-04: Toast auto-dismiss timers are not cleared on component unmount (carried forward)

**File:** `src/App.tsx:241-248` (`_conflictErrorTimer` / `showConflictError`), `616-619` (`showProfileMsg`)
**Issue:** The file is otherwise careful about clearing pending timers on unmount —
`clearPending()`/`clearImPending()` are both invoked from the top-level `onCleanup` (lines 380-387).
`_conflictErrorTimer` and the anonymous `setTimeout` inside `showProfileMsg` have no equivalent cleanup —
they are only cleared when a new toast of the same kind supersedes the old one, never on component
teardown. If the component unmounts while one of these timers is pending (e.g. during HMR), the timer
callback still fires and calls a signal setter (`setConflictError`/`setProfileMessage`) on a disposed
reactive scope.
**Fix:** Track both timer handles at component scope and clear them in the existing `onCleanup` alongside
`clearPending()`.

### WR-05: `delete_profile` / `load_profile` do not sanitize the profile name before handing it to the persistence layer (carried forward)

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

### WR-06: Startup profile restoration failure is completely silent (carried forward)

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
(`App.tsx:1017-1036`, driven by the `auto-save-error` event) covers *save* failures but has no equivalent
for *load* failures on startup.
**Fix:** Await the startup load's oneshot response and, on failure, emit an event the frontend can
surface (e.g. an `auto-load-error` event feeding a banner analogous to the existing auto-save-error one)
instead of discarding both the send result and the oneshot response.

### WR-07: Startup data fetch uses `Promise.all` — one failing call discards all successful results (new)

**File:** `src/App.tsx:295-329`
**Issue:** The initial effect fetches seven independent pieces of state (`get_state`,
`check_accessibility`, `check_input_monitoring`, `get_tcc_identity_status`, `get_active_app`,
`list_profiles`, `getVersion`) via `Promise.all`. If any single call rejects, none of the setters run —
including `setState(stateData)`, the call that actually populates the macro list. The `catch` only logs to
`console.error` (line 324); there is no user-visible error state and no retry, so a single transient IPC
hiccup leaves the whole dashboard showing its null/default state indefinitely with no indication anything
went wrong.
**Fix:** Use `Promise.allSettled` and apply whichever results succeeded, or at minimum surface a visible
error banner (mirroring the existing `saveError` banner pattern) when any of the startup calls fail, so
partial success is not silently downgraded to a permanently-empty UI.

### WR-08: Event-listener promises lack a `.catch`, risking an unhandled rejection (new)

**File:** `src/App.tsx:333-339` (`state-changed`) and `:346-352` (`auto-save-error`)
**Issue:**
```ts
const unlisten = listen<AppState>("state-changed", (event) => { ... });
onCleanup(() => {
  unlisten.then((fn) => fn());
});
```
If the `listen(...)` promise ever rejects (e.g. the IPC layer is torn down mid-registration),
`unlisten.then(...)` has no rejection handler, producing an unhandled promise rejection. Unlikely in
steady state, but a cheap, cheap-to-fix defensive gap.
**Fix:** Add a `.catch(() => {})` (or log) to both `.then()` chains, e.g.
`unlisten.then((fn) => fn()).catch(() => {});`.

### WR-09: Guaranteed `.await` delivery for `HoldStart` widens the Scheduler's single-task stall radius to *all* macros, not just the affected one (new)

**File:** `src-tauri/src/scheduler/mod.rs:296-325`
**Issue:** This is a knowing extension of an already-accepted trade-off (see the comments at :300-314 and
:372-380), so it is not filed as a defect in the `09-06` fix itself — but it is worth calling out
explicitly for a phase whose entire purpose is *parallel* macro execution. `Scheduler::run()` is a single
task by design (`@scheduler-agent` contract, :112-119) — no per-macro spawns. Every `.await`-blocking send
inside `handle_intent` (now: `HoldStart` in `start_macro`, plus the pre-existing `HoldRelease` in
`release_holds`/`StopAll`) blocks the *entire* `run()` loop, including `tokio::time::sleep_until` firing
for every other macro's `InterleavedInterval` timers, until `action_tx` has a free slot. Before `09-06`,
only the release/stop-all paths could stall the loop; now the start path can too. With many concurrently
running macros contending for the 1024-capacity `action_tx` channel (lib.rs:26), a single Hold-mode
macro's `start_macro` call can now delay every other macro's interval fires for as long as the send stays
blocked. The production channel capacity makes this unlikely in practice, but it is a real, measurable
widening of blast radius and should be tracked rather than only implicitly accepted via comment.
**Fix:** No code change required if the trade-off is accepted as-is. Otherwise, consider bounding the
worst case with a short `try_send` retry/backoff loop (with a hard timeout) instead of an unbounded
`.await`, so one saturated send cannot indefinitely starve unrelated macros' timers.

## Info

### IN-01: Inconsistent IPC argument naming for the macro identifier (carried forward)

**File:** `src-tauri/src/ipc/mod.rs:80-108`
**Issue:** `bind_hotkey` and `unbind_hotkey` take `macro_id: Uuid`, while every other per-macro command
(`remove_macro`, `set_macro_enabled`, `set_macro_target_app`, `set_macro_trigger_key`,
`set_macro_sequence`, `update_step_interval`) takes `id: Uuid`. Minor, avoidable naming inconsistency
across an otherwise uniform IPC surface.
**Fix:** Rename `macro_id` → `id` in `bind_hotkey`/`unbind_hotkey` (requires updating the matching
`App.tsx` call sites that currently pass `{ macro_id: id, ... }`).

### IN-02: macOS platform detection is duplicated three times in `App.tsx` (carried forward)

**File:** `src/App.tsx:145`, `173`, `287`
**Issue:** `navigator.userAgent.toLowerCase().includes("mac")` is independently re-declared as `IS_MACOS`
inside `computeModifiers`, inside `modifierChips`, and again inside `App()`. Any future change to the
detection heuristic requires updating three sites in lockstep.
**Fix:** Hoist a single module-level `const IS_MACOS = navigator.userAgent.toLowerCase().includes("mac");`
and reference it from all three call sites (or pass it as a parameter into the two free functions).

### IN-03: `add_macro`'s returned UUID is fetched but discarded (carried forward)

**File:** `src/App.tsx:483-484`
**Issue:** `await invoke<string>("add_macro", { config });` discards the backend-generated UUID entirely,
relying solely on the subsequent `state-changed` event to populate the real macro ID into `state()`.
Functionally fine today, but a fragile implicit dependency — if that event were ever dropped or delayed,
the frontend has no fallback identifier for the macro it just created.
**Fix:** No functional change required; at minimum, a short comment noting the reliance on
`state-changed` (matching the `WR-06`-style fallback pattern used in `handleLoadProfile`) would make the
dependency explicit for future readers.

### IN-04: `loading` signal is written but never read — dead state (new)

**File:** `src/App.tsx:214`
**Issue:**
```ts
const [, setLoading] = createSignal(true);
```
The getter is discarded via the empty destructuring slot, and `setLoading(false)` is called at line 326,
but nothing in the component ever reads a `loading()` value — there is no gated loading UI. The signal
exists purely to be written to and never consumed, so it currently has no observable effect.
**Fix:** Either remove the signal entirely (and the corresponding `setLoading` calls), or wire it into the
render (e.g. a loading skeleton for the dashboard while the initial `Promise.all` is in flight) so the
tracked state has an actual consumer.

### IN-05: `get_active_app` performs a full `GetState` round-trip just to extract one field (new)

**File:** `src-tauri/src/ipc/mod.rs:64-73`
**Issue:** `get_active_app` sends `Intent::GetState(tx)` — the same intent `get_state` uses — and then
discards everything except `app_state.active_app`. This duplicates `get_state`'s round-trip cost and
serializes the entire `AppState` (including the full macro map) just to read one `Option<String>`. Not a
functional bug, but avoidable duplication.
**Fix:** Consider a dedicated lightweight `Intent::GetActiveApp` variant if this command is called on a
frequent poll path, or leave as-is with a comment noting the trade-off was deliberate (currently there is
no such comment).

---

_Reviewed: 2026-07-21T19:56:19Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
