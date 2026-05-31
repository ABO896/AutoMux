# Codebase Concerns

**Analysis Date:** 2026-05-15

## Tech Debt

**Minimum interval floor too low (1ms, not 5ms as documented):**
- Issue: `FAILURE_MODES.md` documents a hard minimum of 5ms to prevent OS event queue flooding. The actual enforcement in the scheduler is `interval_ms.max(1)` — a 1ms floor, not 5ms. A user can configure a 1ms interval and DOS the OS input queue.
- Files: `src-tauri/src/scheduler/mod.rs` (line 66, 184)
- Impact: System unresponsiveness if any macro is set to a very low interval (≤4ms). The documented safety guarantee is broken.
- Fix approach: Change `interval_ms.max(1)` to `interval_ms.max(5)` in both `IntervalTask::new` and `UpdateInterval` handling, matching the spec in `docs/FAILURE_MODES.md`.

**No auto-save on macro changes:**
- Issue: `persistence.rs` header comment states "auto-saved on changes," but the StateActor never calls `save_profile` when macros are added, removed, enabled/disabled, or their sequences are updated. The Default profile is only saved at first creation. Any changes made during a session are lost on next launch unless the user explicitly navigates to the Profiles tab and saves.
- Files: `src-tauri/src/state/mod.rs` (`handle_intent` method), `src-tauri/src/persistence.rs` (line 9 comment)
- Impact: Silent data loss. Users configuring macros and closing the app lose all changes. This contradicts the stated behavior.
- Fix approach: Inject `ProfileManager` into `StateActor` and call `save_profile` after each mutating intent (`AddMacro`, `RemoveMacro`, `SetMacroEnabled`, `SetMacroTargetApp`, `UpdateSequence`, `UpdateStepInterval`).

**UI keyboard key input not exposed in macro creation form:**
- Issue: The backend model supports `InputEvent::Key(u16)` for keyboard keys in `ActionStep`. The new macro creation form (`src/App.tsx` lines 459–467) only offers a dropdown with three mouse button options (Left, Right, Middle). There is no way for users to create a keyboard key macro through the UI. The code path `{ Key: parseInt(inputVal) || 0 }` in `handleCreateMacro` is dead — the `select` element only ever produces `"Left"`, `"Right"`, or `"Middle"`.
- Files: `src/App.tsx` (lines 163–165, 459–467)
- Impact: Core feature advertised in documentation is inaccessible via UI. Users cannot bind keyboard keys to macros from the frontend.
- Fix approach: Add a keyboard key option to the input type selector, or add a separate input mode toggle that shows a keycode number field when `Key` is selected.

**Hardcoded version string in UI:**
- Issue: The titlebar displays `v1.0.0` hardcoded in the JSX at `src/App.tsx` line 291, while `tauri.conf.json` and `package.json` both declare version `1.1.0`. These are already out of sync.
- Files: `src/App.tsx` (line 291), `src-tauri/tauri.conf.json`
- Impact: UI always shows wrong version. Will continue drifting as releases happen.
- Fix approach: Use `getVersion()` from `@tauri-apps/api/app` to read the version at runtime.

**Comment says "Poll every 3s" but interval is 10s:**
- Issue: The accessibility status polling in `src/App.tsx` has a comment "Poll accessibility every 3s" (line 124) but `setInterval` uses 10000ms (10 seconds), not 3000ms.
- Files: `src/App.tsx` (lines 124–134)
- Impact: Minor misleading documentation, but if the intent was 3s polling (for quick feedback after granting permissions) the user must wait up to 10s to see the status change.
- Fix approach: Either fix the comment to say 10s or reduce the interval to 3000ms to match stated intent.

**`implementation_plan.md` left in repo root with unchecked task items:**
- Issue: `implementation_plan.md` at the repo root contains `[ ]` (unchecked) task items — leftover planning artifacts. It describes work that appears to already be implemented (trigger keys, Hold mode), suggesting the plan was never reconciled with the implementation.
- Files: `implementation_plan.md`
- Impact: Misleads future contributors about what is implemented vs pending. The "Open Questions" section about Hold mode semantics is still unresolved in the document even though a toggle-to-toggle behavior was chosen in the implementation.
- Fix approach: Archive to `archive/` and delete from repo root, or update checkboxes to reflect current state.

## Known Bugs

**Duplicate macro trigger key registration:**
- Symptoms: If the same keycode is assigned as `trigger_key` on two different macros, `update_macro_trigger_keys` replaces the entire HashMap each time. The last macro to call `reevaluate_all_macros` wins. The first macro's trigger key is silently dropped with no error.
- Files: `src-tauri/src/state/mod.rs` (`reevaluate_all_macros`, lines 391–397), `src-tauri/src/platform/macos/observer.rs` (`update_macro_trigger_keys`, line 158)
- Trigger: Create two macros with the same trigger key. Only the last one in HashMap iteration order will respond.
- Workaround: None — users must manually ensure no duplicate trigger keys. No UI validation exists.

**`try_send` on the hotkey bindings channel can silently drop hotkey toggles:**
- Symptoms: When channel capacity (100) is saturated, hotkey presses dispatched via `tx.try_send(intent)` in the CGEventTap callback silently fail. The macro toggle is lost with no feedback to the user.
- Files: `src-tauri/src/platform/macos/observer.rs` (lines 341, 354), `src-tauri/src/platform/windows/mod.rs` (lines 350, 359)
- Trigger: Pressing a hotkey rapidly while the StateActor is busy processing many intents.
- Workaround: Not possible from user side.

**NSWorkspace observer token memory leak:**
- Symptoms: `MacPlatformObserver` stores the `NSObject` observer token as a raw `usize` pointer (`_observer_token`) at line 456 of `observer.rs`. `stop_observing` recovers and removes the observer correctly (lines 480–495). However, `lib.rs` (lines 70–72) drops the `MacPlatformObserver` struct at the end of the `setup` closure — calling `stop_observing` is never triggered because neither `Drop` is implemented nor is the observer stored anywhere persistent. The raw pointer inside `_observer_token` leaks.
- Files: `src-tauri/src/platform/macos/observer.rs` (lines 386–496), `src-tauri/src/lib.rs` (lines 70–72)
- Trigger: Every application launch on macOS.
- Workaround: The notification observer continues to function (the NSWorkspace center holds a strong reference), so functionally this is benign, but it means `stop_observing` is dead code and the `Drop` contract is not honored.

**`inject_mouse_click` ignores coordinates on Windows:**
- Symptoms: `WindowsInputProvider::inject_mouse_click` accepts `x: f64, y: f64` parameters (matching the `InputProvider` trait) but ignores them, instead sending button events at the current cursor position.
- Files: `src-tauri/src/platform/windows/mod.rs` (lines 183–199, comment at line 185)
- Trigger: Any future code that calls `inject_mouse_click` expecting click-at-coordinates behavior.
- Workaround: Current usage always uses `inject_mouse_button_raw`, which also clicks at cursor position, so this is not currently exercised.

## Security Considerations

**Emergency stop calls `process::exit(1)` without cleanup:**
- Risk: Both macOS and Windows emergency stop handlers call `process::exit(1)` directly after sending `TriggerEmergencyStop` to the StateActor. Because `try_send` is non-blocking, the StateActor may not have processed the stop intent (and thus not called `flush_held_inputs`) before the process terminates. This creates a race: the held-input flush and the forced exit race against each other, and the exit can win.
- Files: `src-tauri/src/platform/macos/observer.rs` (line 322), `src-tauri/src/platform/windows/mod.rs` (line 352)
- Current mitigation: The emergency stop handler directly iterates the held-inputs registry and posts KeyUp/MouseUp events immediately before `exit(1)` — so the raw flush still happens. But the `StateActor`'s `flush_held_inputs()` may not run.
- Recommendations: Perform the flush synchronously on the calling thread before `exit(1)`, rather than relying on the StateActor channel. The macOS path already does an inline flush in the CGEventTap callback; ensure Windows does the same.

**No input validation on `step_index` in `update_step_interval`:**
- Risk: The `update_step_interval` IPC command accepts a `step_index: usize` from the frontend. The StateActor checks bounds on the sequence steps (`get_mut(step_index)` returns None gracefully), but the `SchedulerIntent::UpdateInterval` is still sent to the scheduler even when the StateActor found no matching step. The scheduler then silently looks up a non-existent `StepId` and does nothing, but an adversarial or buggy frontend could flood the scheduler channel with invalid indices.
- Files: `src-tauri/src/state/mod.rs` (lines 340–358), `src-tauri/src/ipc/mod.rs` (lines 182–191)
- Current mitigation: The Tauri CSP and same-process trust model limit exploit surface.
- Recommendations: Validate `step_index < mac.sequence.steps.len()` before forwarding `UpdateInterval` to the scheduler.

**macOS `signingIdentity: null` and no entitlements file:**
- Risk: The `tauri.conf.json` sets `signingIdentity: null` and `entitlements: null` for macOS bundle. Unsigned builds will trigger Gatekeeper on any macOS installation and cannot be distributed via the App Store. Accessibility APIs (CGEventTap) require specific entitlements (`com.apple.security.automation.apple-events` and the hardened runtime) for notarized distribution.
- Files: `src-tauri/tauri.conf.json`
- Current mitigation: Acceptable for local/development builds.
- Recommendations: Define entitlements file for production distribution; set a signing identity for CI builds via a secret.

## Performance Bottlenecks

**`reevaluate_all_macros` is O(n) on every state mutation:**
- Problem: Every intent that mutates state (add, remove, enable, disable, target change, sequence change, active app change) calls `reevaluate_all_macros`, which iterates all macros and sends `StartMacro`/`StopMacro` to the scheduler for every one of them. With many macros, this creates O(n) scheduler messages on every single state change.
- Files: `src-tauri/src/state/mod.rs` (lines 366–400)
- Cause: Lack of dirty-tracking or incremental re-evaluation.
- Improvement path: For individual macro mutations (`SetMacroEnabled`, `SetMacroTargetApp`, `UpdateSequence`), only re-evaluate the affected macro instead of all macros. Only `ActiveAppChanged` genuinely requires a full sweep.

**`list_profiles` deserializes every profile file to get macro count:**
- Problem: `ProfileManager::list_profiles` reads and fully deserializes every `.json` profile file just to extract `macro_count`. For a large collection of profiles with many macros each, this is an unnecessary O(n × macro_size) I/O cost on every profile tab open and profile save.
- Files: `src-tauri/src/persistence.rs` (lines 152–191, comment at line 166 acknowledges it as "lazy")
- Cause: `ProfileSummary.macro_count` is not stored separately; it is derived from the full `HashMap`.
- Improvement path: Store `macro_count` as a top-level field in the JSON, or maintain a lightweight manifest file alongside profiles.

## Fragile Areas

**`MacPlatformObserver` dropped silently at end of setup closure:**
- Files: `src-tauri/src/lib.rs` (lines 67–73)
- Why fragile: The `observer` local variable is not stored anywhere persistent. It drops at the end of the `setup` closure. The lib.rs comment says "Observer is long-lived; it does not implement Drop, so the token is kept alive natively" — this is partially true (the NSWorkspace observer token is kept alive by the notification center), but if the behavior of `stop_observing` ever needs to be invoked (e.g., for permission re-check flows), it cannot be called.
- Safe modification: Store `MacPlatformObserver` in Tauri's managed state so it lives for the application lifetime and `stop_observing` can be called if needed.
- Test coverage: No tests for observer lifecycle.

**Windows hook thread message loop exits if hook installation fails:**
- Files: `src-tauri/src/platform/windows/mod.rs` (lines 387–430)
- Why fragile: If `SetWindowsHookExW` returns an error, `initialize_hook` resets `HOOK_INITIALIZED` to `false` and returns early. But the thread spawned via `std::thread::spawn` is already running — it just exits silently. No error is propagated to the UI. The `win_event_hook` for foreground app changes (active app tracking) is set up inside the same thread after the keyboard hook, so if the keyboard hook fails, foreground tracking is also broken with no user feedback.
- Safe modification: Return a `Result` from `initialize_hook` and surface the error to the IPC layer so the UI can display a permissions error on Windows.
- Test coverage: No tests for Windows hook initialization failure path.

**Action channel capacity of 100 for `action_rx` (Scheduler → StateActor):**
- Files: `src-tauri/src/lib.rs` (line 25), `src-tauri/src/scheduler/mod.rs` (line 301, comment "@safety-officer: try_send backpressure — drops on overflow")
- Why fragile: The `action_rx` channel has capacity 100. With 32 macros firing at 5ms intervals, that is up to 6400 actions/second. If the StateActor is briefly delayed by a slow `broadcast_state` call (which emits a Tauri event to the WebView), the channel can saturate and actions are silently dropped via `try_send`. The stress test uses a 256-capacity channel which masks the tighter production capacity.
- Safe modification: Increase `action_rx` capacity to match peak throughput, or implement backpressure signaling so the scheduler can detect drops.
- Test coverage: Stress test uses a different channel capacity than production code.

## Scaling Limits

**Channel backpressure is silent:**
- Current capacity: `state_tx` = 100, `sched_tx` = 100, `action_tx` = 100
- Limit: With many macros at fast intervals, all three channels can saturate; all use `try_send` in hot paths, silently dropping messages when full.
- Scaling path: Log dropped messages at least in debug builds; increase capacity or use bounded-with-notification patterns.

## Dependencies at Risk

**`objc` 0.2.7 (legacy, unmaintained):**
- Risk: `objc` 0.2.7 is in the dependency tree alongside `objc2` 0.6.4. The old `objc` crate is effectively unmaintained. The code uses both crates simultaneously, which increases binary size and creates potential for ABI mismatches on future macOS versions.
- Files: `src-tauri/Cargo.toml` (line 31)
- Impact: Potential compilation failures as Apple evolves the Objective-C runtime; security patches won't be applied to `objc` 0.2.x.
- Migration plan: Audit all usage of `objc` 0.2 and replace with `objc2` equivalents; remove `objc` and `cocoa` dependencies.

**`cocoa` 0.26.1 (legacy):**
- Risk: `cocoa` depends on the legacy `objc` crate and is not actively maintained. It is currently listed as a dependency but no direct `use cocoa::...` imports appear in the source. It may be a transitive dependency being explicitly declared unnecessarily.
- Files: `src-tauri/Cargo.toml` (line 32)
- Impact: Bloated dependency tree; security surface.
- Migration plan: Run `cargo tree` to verify if `cocoa` is actually used; if not, remove from `Cargo.toml`.

## Missing Critical Features

**No system tray / background mode:**
- Problem: `docs/FAILURE_MODES.md` (section 9) documents a tray icon that allows the app to keep running after the window is closed. This is not implemented — there is no tray icon setup in `lib.rs`, no window close event handler, and the standard Tauri window close terminates the process. Macros stop when the user closes the window.
- Blocks: Any use case where the app should run in the background (the primary AFK automation use case).

**No configurable hotkeys on Windows:**
- Problem: The `bind_hotkey` and `unbind_hotkey` IPC commands are macOS-only (`#[cfg(target_os = "macos")]` blocks in `src-tauri/src/ipc/mod.rs` lines 76–96). On Windows, both commands succeed silently but register nothing. The Windows hook only processes `trigger_key` fields, not the full `HotkeyBinding` model (with modifiers).
- Blocks: Engine toggle via hotkey on Windows; modifier-key hotkeys on Windows entirely.
- Files: `src-tauri/src/ipc/mod.rs` (lines 69–97), `src-tauri/src/platform/windows/mod.rs`

## Test Coverage Gaps

**No tests for persistence layer (file I/O):**
- What's not tested: `save_profile`, `load_profile`, `delete_profile`, `list_profiles` with actual filesystem I/O. The only test in `persistence.rs` is a pure in-memory serialization/deserialization round-trip.
- Files: `src-tauri/src/persistence.rs` (tests module, lines 208–277)
- Risk: File write failures, permission errors, corrupted JSON, concurrent access — all untested.
- Priority: High — persistence bugs cause silent data loss.

**No integration tests for StateActor + Scheduler pipeline:**
- What's not tested: The full flow from `Intent` → `StateActor` → `SchedulerIntent` → `Scheduler` → `ActionReady` → `StateActor.handle_action`. Scheduler unit tests test the scheduler in isolation; StateActor has no tests at all.
- Files: `src-tauri/src/state/mod.rs` (no test module)
- Risk: Targeting gate bugs, emergency stop races, enable/disable state desync with scheduler.
- Priority: High — the two-phase dispatch safety guarantee is completely untested end-to-end.

**No tests for platform observer (macOS or Windows):**
- What's not tested: CGEventTap initialization, hotkey dispatch, active app change propagation, emergency stop in-callback flush.
- Files: `src-tauri/src/platform/macos/observer.rs`, `src-tauri/src/platform/windows/mod.rs`
- Risk: Platform-specific bugs and regressions ship without detection.
- Priority: Medium — platform code is inherently difficult to unit-test but integration/smoke tests are feasible.

**No frontend tests:**
- What's not tested: All SolidJS component behavior in `src/App.tsx` — macro creation, profile save/load, engine toggle, accessibility status display.
- Files: `src/App.tsx`, `package.json` (no test script defined)
- Risk: UI regressions in user-facing flows are not caught by CI.
- Priority: Medium.

---

*Concerns audit: 2026-05-15*
