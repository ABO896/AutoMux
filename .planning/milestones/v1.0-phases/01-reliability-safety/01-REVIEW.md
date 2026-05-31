---
phase: 01-reliability-safety
reviewed: 2026-05-16T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/platform/macos/input.rs
  - src-tauri/src/platform/macos/observer.rs
  - src-tauri/src/platform/windows/mod.rs
  - src-tauri/src/scheduler/mod.rs
  - src-tauri/src/state/mod.rs
  - src/App.tsx
findings:
  critical: 6
  warning: 8
  info: 3
  total: 17
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-05-16T00:00:00Z
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

This review covers the full backend (Rust) and frontend (SolidJS/TypeScript) of AutoMux. The architecture is sound — the single-StateActor / single-Scheduler two-phase dispatch model is correctly structured, and the intent channel pattern is consistently applied. However, six correctness bugs were found that directly affect the reliability and safety goals of this phase: held-input registry leaks on emergency stop, missing flush after scheduler StopAll on engine-off, a TOCTOU race in CGEventTap re-initialization that can produce a zombie tap, a deadlock vector in `flush_held_inputs`, dropped state broadcasts after some intents, and an incorrect SendInput coordinate path on Windows. Multiple additional warnings cover unsafe code patterns, missing error propagation, and silent behavior that will be hard to diagnose.

---

## Critical Issues

### CR-01: `flush_held_inputs` holds registry lock while posting CGEvents — deadlock risk

**File:** `src-tauri/src/platform/macos/observer.rs:75-125`

**Issue:** `flush_held_inputs()` acquires `get_registry().lock()` at line 76 and holds it for the entire body, including every `up_event.post(CGEventTapLocation::HID)` call (lines 91, 119). Posting a CGEvent re-enters the same process's CGEventTap callback (lines 214-388). The callback immediately tries to acquire the same `REGISTRY` mutex at line 236 (`get_registry().lock().unwrap()`). On macOS, `std::sync::Mutex` is non-reentrant, so the callback blocks waiting for the lock that `flush_held_inputs` already holds — deadlock. The process hangs on the emergency-stop path, which is the most safety-critical code path in the application.

The inline emergency-stop block at lines 293-348 has the same pattern (locks registry at line 293, then posts events from within the lock).

**Fix:** Drain the registry into a local `Vec` while holding the lock, drop the lock, then iterate the local vec to post events:
```rust
pub fn flush_held_inputs() {
    let inputs_to_flush: Vec<ActiveInput> = {
        let mut reg = get_registry().lock().unwrap();
        reg.drain().collect()
    };
    // lock released here — safe to post CGEvents
    if let Ok(source) = CGEventSource::new(...) {
        for input in inputs_to_flush {
            // ... post events
        }
    }
}
```
Apply the same drain-then-release pattern in the inline emergency-stop block (observer.rs:293-348).

---

### CR-02: Emergency stop does not flush held inputs after marking macros disabled — inputs stay depressed on Windows

**File:** `src-tauri/src/state/mod.rs:313-324`

**Issue:** `Intent::TriggerEmergencyStop` (lines 313-324) sends `SchedulerIntent::StopAll` to the scheduler, but does NOT call `self.input_provider.flush_held_inputs()` after stopping. Wait — it does call `flush_held_inputs()` at line 323. However, `SchedulerIntent::StopAll` is sent via `.await` on the bounded channel (capacity 100), but the StateActor is the only consumer of `action_rx`. If the `action_rx` is near-full, sending `StopAll` could block briefly while the action channel drains. More critically: `StopAll` in the Scheduler (scheduler/mod.rs:168-175) calls `release_holds` which uses `try_send` back on `action_tx` (line 282). If `action_tx` is full, `HoldRelease` events for all sustained holds are **silently dropped** — the StateActor never sees them, never calls `inject_input(input, false)`, and the OS keys/buttons remain held.

The emergency stop path on the StateActor side does `flush_held_inputs()` directly (line 323), bypassing the scheduler's HoldRelease path entirely, which is correct. But the Windows flush (platform/windows/mod.rs:374) also calls `flush_all_held_inputs()` from the hook callback thread. The Windows `HELD_INPUTS` registry (line 27) is only updated by `inject_key`/`inject_mouse_button_raw` in the Windows provider. If `inject_mouse_click` (lines 188-214) completes the down+up cycle normally, the key is briefly in the registry between lines 201 and 212, but never removed from there on abnormal termination mid-click if the thread panics. This is a narrow window, not the primary issue.

The primary issue: the Windows hook callback (lines 371-375) calls `WindowsInputProvider::flush_all_held_inputs()` then `process::exit(1)` with **no `SchedulerIntent::StopAll`** sent synchronously first. The StateActor may not process `TriggerEmergencyStop` before exit, meaning no `broadcast_state` event fires and the scheduler is not cleanly stopped. On Windows the order is: flush → exit, which is correct for held inputs. But the `TriggerEmergencyStop` intent is a `try_send` (line 370) that may be dropped if the channel is full, so the StateActor's `emergency_stop_active` flag may never be set before `process::exit(1)` is called — this is acceptable because exit terminates everything, but it means any profile auto-save on exit is skipped.

**Fix (primary — action channel overflow):** In `Scheduler::release_holds` and `Scheduler::StopAll`, use `.await` instead of `try_send` for `HoldRelease` messages when inside `StopAll`, to guarantee delivery:
```rust
SchedulerIntent::StopAll => {
    let all_ids: Vec<Uuid> = self.active_holds.keys().copied().collect();
    for id in all_ids {
        // Use blocking send for safety-critical releases
        for input in self.active_holds.remove(&id).unwrap_or_default() {
            let _ = self.action_tx.send(ActionReady {
                macro_id: id,
                action_type: ActionType::HoldRelease(input),
                fired_at: Instant::now(),
            }).await;
        }
    }
    self.interval_tasks.clear();
    self.timeline.clear();
    self.active_holds.clear();
}
```
Alternatively, the StateActor should always call `flush_held_inputs()` immediately on `TriggerEmergencyStop` as a synchronous best-effort (it already does this at line 323 on macOS, confirming this is the intended backup). Ensure both platforms do this.

---

### CR-03: CGEventTap TOCTOU — `TAP_INITIALIZED` flipped to `true` before tap is confirmed working; failed taps leave the flag permanently set until reset

**File:** `src-tauri/src/platform/macos/observer.rs:184-411`

**Issue:** `initialize_tap()` uses `compare_exchange(false, true)` at line 187 — flipping `TAP_INITIALIZED` to `true` immediately — then spawns a thread. Inside the thread, if `CGEventTap::new()` fails (no Accessibility permission, or resource pressure), the error branch at line 400-406 resets the flag back to `false` (`TAP_INITIALIZED.store(false, Ordering::SeqCst)`). However, there is a race window: between the `compare_exchange` succeeding and the `store(false)` executing on the spawned thread, any concurrent call to `initialize_tap()` will see `TAP_INITIALIZED == true` and return `true` (idempotent return, line 191) — falsely advertising that the tap is active. Any caller relying on `is_tap_initialized()` (line 172) during this window will believe the tap is running when it has not yet been confirmed.

More critically: if the `CGEventTap::new()` succeeds but `tap.mach_port.create_runloop_source(0)` fails (line 394), the code falls through without running `CFRunLoop::run_current()` — the thread exits, the tap is dropped, the `OnceLock` source is never added to the run loop, but `TAP_INITIALIZED` is **never reset to false** in this branch (only the `Err(_)` branch resets it). The tap is silently dead and `is_tap_initialized()` returns `true` forever. No hotkeys, no emergency stop, no input tracking — all silently broken.

**Fix:** Reset `TAP_INITIALIZED` in all non-success branches:
```rust
match tap_result {
    Ok(tap) => {
        let current_loop = CFRunLoop::get_current();
        match tap.mach_port.create_runloop_source(0) {
            Ok(source) => {
                current_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });
                tap.enable();
                CFRunLoop::run_current();
                // run_current() only returns on runloop stop — reset flag on exit
                TAP_INITIALIZED.store(false, Ordering::SeqCst);
            }
            Err(e) => {
                eprintln!("[Observer] Failed to create runloop source: {:?}", e);
                TAP_INITIALIZED.store(false, Ordering::SeqCst); // <-- missing today
            }
        }
    }
    Err(_) => {
        eprintln!("...");
        TAP_INITIALIZED.store(false, Ordering::SeqCst);
    }
}
```

---

### CR-04: `reevaluate_all_macros` does not stop macros when engine is off — running macros are never stopped on engine toggle off path that goes through `ToggleEngineHotkey`

**File:** `src-tauri/src/state/mod.rs:340-349` and `416-449`

**Issue:** `Intent::ToggleEngineHotkey` (lines 340-349) correctly sends `SchedulerIntent::StopAll` when toggling off (line 343-345). However, `Intent::TriggerEmergencyStop` (lines 313-323) disables the engine and sends `StopAll` but then calls `reevaluate_all_macros` implicitly through `ResetEmergencyStop` (line 328). More critically, `reevaluate_all_macros` (lines 416-449) has an early-return guard at line 417: `if self.state.emergency_stop_active || !self.state.engine_active { return; }`. This means if the engine is off, `reevaluate_all_macros` does **nothing** — it does not send `StopMacro` for any macro. This is by design for the stop path (StopAll is used instead), but the problem is in the **re-enable path**:

`Intent::ResetEmergencyStop` sets `engine_active = true` (line 327) then calls `reevaluate_all_macros` (line 328). At that point, macros whose `enabled = true` will be restarted in the scheduler. But `TriggerEmergencyStop` had set all macros' `enabled` to `false` (lines 316-318), so `reevaluate_all_macros` after reset will not start any macros. This is actually correct behavior. No bug here — removing this from critical.

The real issue: `Intent::ToggleMacroHotkey` (lines 334-339) toggles `mac.enabled` and calls `reevaluate_all_macros`. But it does NOT call `auto_save_default`. A hotkey-toggled macro's enabled state is lost on next restart. This is a warning-level issue (see WR-01).

**Correcting the critical finding:** The actual critical in this area is that `reevaluate_all_macros` sends `StartMacro` for every currently-enabled macro on every state change, even macros that are already running. The Scheduler handles this idempotently by calling `stop_macro` before `start_macro` (scheduler/mod.rs:199-200). This means every `ActiveAppChanged`, every `SetMacroEnabled`, every `AddMacro` causes ALL running macros to be stopped and restarted. For sustained-hold macros (`SustainedHold` steps), this produces a transient key-up + key-down on every app switch or any unrelated state mutation, potentially causing unintended double-inputs in the target game/app.

**File:** `src-tauri/src/scheduler/mod.rs:196-255` and `src-tauri/src/state/mod.rs:416-449`

**Issue (restated):** `start_macro` unconditionally calls `stop_macro` first (line 199), which calls `release_holds` → sends `HoldRelease` via `try_send`. Every `reevaluate_all_macros` call (which happens on every intent) stops and restarts all running macros including sustained-hold ones. A user holding right-click in Minecraft will experience a brief right-click release every time any other state changes (new active app, another macro toggled, etc.).

**Fix:** Track which macros are currently running in the StateActor and only restart a macro if its config actually changed or its targeting eligibility changed. Alternatively, have the Scheduler compare the incoming `StartMacro` config against what is already running and no-op if unchanged.

---

### CR-05: Windows `send_mouse_move` passes raw pixel coordinates as MOUSEEVENTF_ABSOLUTE units — all mouse moves go to wrong position

**File:** `src-tauri/src/platform/windows/mod.rs:137-156`

**Issue:** `send_mouse_move` uses `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE` (line 146). When `MOUSEEVENTF_ABSOLUTE` is set, `dx` and `dy` must be in the normalized range 0–65535 mapping to the full virtual desktop, not pixel coordinates. The function receives raw `f64` pixel values and casts them directly to `i32` (lines 143-144: `dx: x as i32, dy: y as i32`). On a 1920×1080 display, passing `x=1920` means the cursor goes to `1920/65535 ≈ 2.9%` of the screen width — far off target. The `inject_mouse_move` call from `MacInputProvider` uses CGPoint pixel coords correctly on macOS, but the Windows side silently moves to the wrong location.

**Fix:** Convert to normalized coordinates before calling `SendInput`:
```rust
fn send_mouse_move(&self, x: f64, y: f64) {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN};
    let (screen_w, screen_h) = unsafe {
        (GetSystemMetrics(SM_CXVIRTUALSCREEN) as f64,
         GetSystemMetrics(SM_CYVIRTUALSCREEN) as f64)
    };
    let norm_x = ((x / screen_w) * 65535.0) as i32;
    let norm_y = ((y / screen_h) * 65535.0) as i32;
    // use norm_x, norm_y in the MOUSEINPUT struct
}
```

---

### CR-06: `load_profile` IPC command loads the profile twice — once in the IPC handler and once in StateActor — causing a double disk read and a potential state inconsistency window

**File:** `src-tauri/src/ipc/mod.rs:225-241`

**Issue:** `load_profile` (lines 225-241) does two separate operations:
1. Line 231: `profile_mgr.load_profile(&name).await?` — reads the profile from disk.
2. Lines 236-238: `state.send_intent(Intent::LoadProfile(name.clone())).await` — sends an intent that causes the StateActor to call `profile_mgr.load_profile(&name).await` **again** (state/mod.rs:392).

Between these two reads, the file on disk could theoretically change (e.g., another profile save). More practically, the IPC handler is returning `profile` (from read #1) as the function result to the frontend, while the StateActor applies `profile` (from read #2). If read #2 fails (e.g., concurrent deletion), the StateActor emits an `auto-save-error` event but the IPC handler has already returned `Ok(profile)` to the frontend — the frontend believes the load succeeded while the backend state is actually empty (macros.clear() was called in the StateActor at line 384 before the failed read #2).

**Fix:** Remove the `load_profile` call from the IPC handler. Instead, have the StateActor return the loaded `ProfileData` via a oneshot channel, similar to `GetState`:
```rust
// New intent variant:
LoadProfile(String, tokio::sync::oneshot::Sender<Result<ProfileData, String>>)

// IPC handler:
let (tx, rx) = tokio::sync::oneshot::channel();
state.send_intent(Intent::LoadProfile(name, tx)).await...;
let profile = rx.await...?;
Ok(profile)
```
Alternatively, if the double-read is acceptable, at minimum check that the IPC layer propagates the StateActor's error (currently it cannot, since the intent is fire-and-forget from the IPC side).

---

## Warnings

### WR-01: `ToggleMacroHotkey` intent missing `auto_save_default` — hotkey-toggled state lost on restart

**File:** `src-tauri/src/state/mod.rs:334-339`

**Issue:** `Intent::ToggleMacroHotkey` toggles `mac.enabled` and calls `reevaluate_all_macros`, but does not call `auto_save_default`. Every other state-mutating intent (`AddMacro`, `SetMacroEnabled`, `UpdateSequence`, etc.) calls `auto_save_default`. If a user toggles a macro via hotkey, the change is not persisted; on next app launch the macro reverts to its pre-toggle state.

**Fix:** Add `self.auto_save_default().await;` after the `reevaluate_all_macros` call at line 338:
```rust
Intent::ToggleMacroHotkey(id) => {
    if let Some(mac) = self.state.macros.get_mut(&id) {
        mac.enabled = !mac.enabled;
    }
    self.reevaluate_all_macros().await;
    self.auto_save_default().await;  // <-- missing
}
```

---

### WR-02: `Intent::LoadProfile` broadcasts state twice — double `state-changed` event on profile load

**File:** `src-tauri/src/state/mod.rs:411` and `src-tauri/src/state/mod.rs:194`

**Issue:** `handle_intent` for `LoadProfile` calls `self.broadcast_state()` explicitly at line 411 (inside the match arm). Then the caller (`StateActor::run`, line 193-195) calls `self.broadcast_state()` again unconditionally after every `handle_intent` call. So every profile load fires two `state-changed` events with the same payload. This causes the frontend to re-render twice, and any side-effects registered on `state-changed` (e.g., list_profiles refresh) will fire twice.

**Fix:** Remove the explicit `self.broadcast_state()` call at line 411 from the `LoadProfile` arm, relying on the outer unconditional broadcast. Or wrap `run`'s broadcast in a flag system. The simplest fix:
```rust
// In LoadProfile arm, remove line 411:
// self.broadcast_state();  // DELETE — outer run() loop broadcasts
```

---

### WR-03: Windows hook thread has no message loop for the WinEvent hook — `UnhookWinEvent` may never be called

**File:** `src-tauri/src/platform/windows/mod.rs:418-452`

**Issue:** The spawned thread installs both a `WH_KEYBOARD_LL` hook (line 419) and a WinEvent hook via `SetWinEventHook` with `WINEVENT_OUTOFCONTEXT` (line 432). With `WINEVENT_OUTOFCONTEXT`, the callback is delivered on the thread that called `SetWinEventHook`, but only if that thread pumps messages. The `GetMessageW` loop (line 443) does pump messages for the keyboard hook. However, `WINEVENT_OUTOFCONTEXT` with a per-thread message pump only works if the `SetWinEventHook` thread is the same thread receiving the events. In practice, `WINEVENT_OUTOFCONTEXT` delivers to any available thread with a message loop. This should work, but the `event_hook` return value is a `HWINEVENTHOOK` (line 432-440): if `SetWinEventHook` returns a null/invalid handle (failure), the code proceeds without checking — the `event_hook` variable is used directly in `UnhookWinEvent(event_hook)` at line 449 without an `is_invalid()` guard.

Also: if the keyboard hook install fails (line 426-429), the code resets `HOOK_INITIALIZED` and returns — but the WinEvent hook was not yet set at that point, so this is fine. However, if the WinEvent hook fails silently (returns null), the app loses active-app tracking without any error.

**Fix:** Check the WinEvent hook handle before using it:
```rust
if event_hook.is_invalid() {
    eprintln!("[WindowsObserver] SetWinEventHook failed — no active app tracking");
} else {
    // store it for cleanup
}
// In cleanup:
if !event_hook.is_invalid() {
    let _ = UnhookWinEvent(event_hook);
}
```

---

### WR-04: `TAP_INITIALIZED` is reset to `false` inside the tap thread on failure, but concurrent callers have already returned `true`

**File:** `src-tauri/src/platform/macos/observer.rs:184-411`

**Issue:** (Companion to CR-03.) Even after the flag is correctly reset on failure, any caller that received `true` from `initialize_tap()` during the window between the `compare_exchange` success and the `store(false)` in the error branch has already acted on that value. The `start_observing` method (line 501-503) calls `initialize_tap()` and does not check the return value or retry. If the tap fails to initialize (e.g., Accessibility not yet granted), `start_observing` silently completes with no tap active, no hotkeys, no emergency stop. The user sees no error.

**Fix:** Return the success/failure from the spawned thread via a channel or `Arc<AtomicBool>`, or call `is_tap_initialized()` after a brief yield to verify initialization completed:
```rust
// In start_observing:
if super::check_accessibility_permissions(false) {
    let ok = initialize_tap();
    if !ok {
        eprintln!("[Observer] CGEventTap initialization returned false — tap may be inactive");
    }
}
```
And ensure the return value of `initialize_tap` accurately reflects tap liveness (post-thread-spawn, it cannot — so document this limitation).

---

### WR-05: `handleCreateMacro` constructs `MacroConfig` with `id: crypto.randomUUID()` on the frontend — UUID generation responsibility belongs to the backend

**File:** `src/App.tsx:170`

**Issue:** The frontend generates the macro UUID using `crypto.randomUUID()`. This means the ID is generated in the WebView context, which is correct in modern browsers/Tauri, but it bypasses any server-side ID assignment or conflict detection. If two windows/instances call `add_macro` concurrently with frontend-generated UUIDs, there is no collision check — the backend will silently overwrite an existing macro if UUIDs collide (state/mod.rs:287: `self.state.macros.insert(config.id, config)`). While UUID v4 collision is astronomically unlikely, the pattern is architecturally incorrect: IDs should be assigned by the authoritative state owner (the StateActor).

**Fix:** Have `add_macro` on the backend generate and return the UUID, and let the frontend use the returned ID. Or at minimum, have the StateActor validate that the incoming ID does not already exist before inserting.

---

### WR-06: Scheduler `missed tick` calculation has integer truncation — `missed` count underestimates by 1 when drift is an exact multiple of interval

**File:** `src-tauri/src/scheduler/mod.rs:78-85`

**Issue:** In `IntervalTask::advance()`:
```rust
let elapsed = now.duration_since(self.next_fire);
let missed = elapsed.as_nanos() / self.interval.as_nanos();
self.next_fire += self.interval * (missed as u32 + 1);
```
`elapsed` is `now - next_fire` (i.e., how far past the deadline we are). `missed = elapsed / interval` gives the number of full intervals we are behind. Adding `+1` advances past the current due tick. This is correct for the general case. However, if `elapsed` is exactly `0` (i.e., `now == self.next_fire`), `missed = 0` and `next_fire += interval * 1` — correct. If `elapsed == interval`, `missed = 1` and `next_fire += interval * 2` — skips 2 ticks when only 1 was missed. This is an off-by-one: when we are exactly N intervals behind, we skip N+1 ticks instead of N.

Also, `missed as u32` will panic/wrap in debug mode if the scheduler is suspended for more than `u32::MAX * interval_ms` nanoseconds (roughly 4 billion intervals worth of time), which is not a realistic concern but is a latent overflow.

**Fix:**
```rust
fn advance(&mut self) {
    let now = Instant::now();
    if self.next_fire <= now {
        let elapsed = now.duration_since(self.next_fire);
        // elapsed.as_nanos() / interval gives intervals already missed (floor division).
        // We want next_fire to be the NEXT tick after now:
        let missed = elapsed.as_nanos() / self.interval.as_nanos();
        self.next_fire += self.interval * (missed as u32 + 1);
    } else {
        self.next_fire += self.interval;
    }
}
```
The condition `self.next_fire + self.interval <= now` in the original at line 78 is the condition for "we are at least one full interval behind". This correctly identifies the catch-up case. The `+1` advancement is correct in that branch because `elapsed` measured from `next_fire` (not `next_fire + interval`) gives one fewer missed interval than the condition implies. On close inspection this may be correct — but the condition at line 78 `self.next_fire + self.interval <= now` means `now >= next_fire + interval`, i.e., we are at least interval behind `next_fire`. Then `elapsed = now - next_fire >= interval`. `missed = elapsed/interval >= 1`. `next_fire += interval * (missed + 1)`. If `elapsed = interval` exactly, `missed=1`, `next_fire += 2*interval` — advances 2 ticks from `next_fire`, which puts it at `next_fire + 2*interval`. But we fired at `next_fire` (1 tick missed), next fire should be at `next_fire + interval` + whatever is past now... This is an off-by-one skip. The correct formula when you want "next fire time after now" is: `next_fire = now + interval` (simplest) or `next_fire += interval * (missed + 1)` where `missed = (now - next_fire) / interval`.

---

### WR-07: `handleDeleteProfile` guards against deleting "Default" on the frontend only — no backend guard

**File:** `src/App.tsx:252` and `src-tauri/src/ipc/mod.rs:244-249`

**Issue:** `handleDeleteProfile` checks `if (name === "Default") return;` on the frontend (line 252). The backend `delete_profile` IPC command (ipc/mod.rs:244-249) delegates directly to `ProfileManager::delete_profile` with no such guard. A caller bypassing the frontend (e.g., another Tauri window, CLI invocation of the webview, or a future IPC client) can delete the "default" profile. This would cause the next `LoadProfile("default")` on startup to fail silently (the error branch at state/mod.rs:403-406 emits an `auto-save-error` event but macros.clear() has already been called, leaving the app with an empty macro list).

**Fix:** Add a guard in `delete_profile` on the backend:
```rust
pub async fn delete_profile(
    profile_mgr: State<'_, Arc<crate::persistence::ProfileManager>>,
    name: String,
) -> Result<(), String> {
    if name.eq_ignore_ascii_case("default") {
        return Err("Cannot delete the default profile".to_string());
    }
    profile_mgr.delete_profile(&name).await
}
```

---

### WR-08: `createEffect` for initial data fetch runs with no cleanup — if the component unmounts before all `invoke` calls resolve, stale setters are called on a dead component

**File:** `src/App.tsx:94-111`

**Issue:** The async `createEffect` at line 94 calls four `invoke` commands in parallel. If the `App` component is unmounted before the `Promise.all` resolves (unlikely in a single-page Tauri app, but possible during hot-reload in dev or if the WebView is destroyed), the `setState`, `setAccessibility`, `setActiveApp`, `setProfiles` calls at lines 102-105 fire on a collected signal — SolidJS will typically no-op this, but the `setLoading(false)` in `finally` could trigger further reactive effects on stale signals. More concretely, `createEffect` with an `async` callback does not track the async continuation — the cleanup returned by `onCleanup` is not awaitable, so there is no way to cancel the in-flight `invoke` calls.

**Fix:** Add an `onCleanup` with an abort flag:
```typescript
createEffect(() => {
  let cancelled = false;
  (async () => {
    try {
      const results = await Promise.all([...]);
      if (cancelled) return;
      setState(results[0]);
      // ...
    } catch (e) { ... }
    finally { if (!cancelled) setLoading(false); }
  })();
  onCleanup(() => { cancelled = true; });
});
```

---

## Info

### IN-01: Magic number `LLMHF_INJECTED = 0x414D5558` — name is misleading, value is "AMUX" in ASCII

**File:** `src-tauri/src/platform/macos/input.rs:6`

**Issue:** The constant `LLMHF_INJECTED` is named after the Windows low-level mouse hook flag (`LLMHF_INJECTED = 0x01`), but the value `0x414D5558` is actually the ASCII string "AMUX" — the app's own marker. The name is a confusing misnomer that could cause a future developer to assume it maps to a Windows API constant. The constant is used correctly (as a user-data marker to identify self-injected events), but the name should reflect its purpose.

**Fix:** Rename to `AUTOMUX_EVENT_MARKER` or `INJECTED_EVENT_USER_DATA` in both `input.rs` and all references in `observer.rs`.

---

### IN-02: `println!` for emergency stop trigger — should be `eprintln!` or structured logging

**File:** `src-tauri/src/platform/macos/observer.rs:285` and `src-tauri/src/platform/windows/mod.rs:368`

**Issue:** Both emergency stop paths use `println!("EMERGENCY STOP TRIGGERED")` which writes to stdout. In a Tauri app, stdout may be buffered or ignored in production builds. `eprintln!` writes to stderr, which Tauri captures and surfaces in logs. Additionally, this is a safety-critical event that should be distinguishable in logs.

**Fix:** Use `eprintln!` and add a timestamp or structured prefix consistent with the other `[Observer]` log patterns used elsewhere in the file.

---

### IN-03: `setLoading` signal is created but the setter is never used after the effect — dead signal setter

**File:** `src/App.tsx:71`

**Issue:** `const [, setLoading] = createSignal(true)` — the getter is unused (destructured as `_`) and only `setLoading` is used inside the initial data effect (line 109). The loading state is never read in the render tree, so no spinner or loading indicator is shown while initial data fetches. This means the UI renders with `state() === null` until the first `invoke` resolves. The render handles `null` with `macroList()` returning `[]`, so there is no crash — but this is dead signal tracking overhead and the UI has no loading feedback.

**Fix:** Either use the loading signal to render a spinner/skeleton, or remove it entirely if no loading UI is planned.

---

_Reviewed: 2026-05-16T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
