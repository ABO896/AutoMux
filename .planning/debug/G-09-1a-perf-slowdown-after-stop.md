---
status: diagnosed
trigger: "G-09-1a-perf-slowdown-after-stop: AutoMux (Tauri macro automation app) remains noticeably slowed down system-wide even after all macros have been disabled/stopped, following use of a fast-interval (100ms) global hotkey-triggered macro."
created: 2026-07-22T00:00:00Z
updated: 2026-07-22T00:32:00Z
---

## Current Focus

reasoning_checkpoint:
  hypothesis: "The macOS CGEventTap installed in platform/macos/observer.rs:initialize_tap() subscribes to 5 high-frequency event types (MouseMoved, LeftMouseDragged, RightMouseDragged, OtherMouseDragged, ScrollWheel) that the tap callback never reads or acts on. This tap is a single, app-lifetime singleton (started once in lib.rs setup(), guarded by TAP_INITIALIZED, never torn down or narrowed) that is completely independent of macro enabled/disabled state. Because it is a ListenOnly tap positioned at CGEventTapPlacement::HeadInsertEventTap on the HID event stream, macOS must synchronously relay every matching system-wide event (all mouse movement/drag/scroll, not just AutoMux's own) through this process before the rest of the OS event pipeline proceeds. This creates a permanent, macro-independent system-responsiveness tax that starts the moment the tap initializes and never stops until the app exits — explaining exactly why 'automux in general slowed down even after macros turned off.'"
  confirming_evidence:
    - "observer.rs:254-267 — event mask includes CGEventType::MouseMoved, LeftMouseDragged, RightMouseDragged, OtherMouseDragged, ScrollWheel"
    - "observer.rs:292-321 — the LLMHF_INJECTED branch's match only handles KeyDown/KeyUp/{Left,Right,Other}Mouse{Down,Up} — no MouseMoved/Dragged/ScrollWheel arm exists, so those events do nothing but fall through"
    - "observer.rs:326-447 — the hotkey/emergency-stop/trigger-key logic is gated behind `matches!(event_type, CGEventType::KeyDown)` only — mouse-motion event types never reach any of this logic either"
    - "observer.rs:449 — the ONLY thing done with an unhandled event is `Some(event.clone())` (pass it through unmodified) — confirms MouseMoved/Dragged/ScrollWheel are captured for zero functional purpose"
    - "lib.rs:65-72 — observer.start_observing() (which calls initialize_tap()) runs exactly once at app setup, held in Tauri managed state for the full app lifetime — never re-scoped per macro state"
    - "git blame -L 250,270 observer.rs — this exact event mask (including the 5 unused types) has been present unchanged since the very first commit (0cbbac6, v1.0.0-rc) — confirmed via `git blame`, not modified by Phase 9"
    - "state/mod.rs:317-337 (Scheduler::stop_macro) and StateActor::reevaluate_all_macros correctly stop timers/holds when a macro is disabled — traced end-to-end, no leaked IntervalTask/timeline/active_holds entries found — ruling out the Scheduler as the persistence source"
    - "compounding (not root-cause) factor confirmed: input.rs posts synthetic events at CGEventTapLocation::HID, the same location the tap listens at with HeadInsertEventTap placement, so every interval-fired click loops back through this same callback (REGISTRY lock x2 per fire) — proportional to active-macro rate, explains 'lagged while it ran' but is NOT persistent after stop since it stops firing once StopMacro is processed"
    - "compounding (dev-mode only) factor confirmed: state/mod.rs:240-241 `eprintln!(\"[Action] macro={} {:?}\", ...)` runs under #[cfg(debug_assertions)] on every dispatched action — `npm run tauri dev` is a debug build, so this adds one synchronous stdout write per click (~10/sec at 100ms) during the run — again bounded to active-macro time, not persistent"
  falsification_test: "If the always-on tap were NOT the persistence source, then narrowing the event mask to drop MouseMoved/LeftMouseDragged/RightMouseDragged/OtherMouseDragged/ScrollWheel (keeping only the Key*/*Mouse{Down,Up} types actually consumed) would NOT restore baseline system responsiveness after a macro is stopped. This is directly testable: rebuild with the narrowed mask, repeat the reproduction (100ms global hotkey macro, run, then disable), and confirm system responsiveness returns to baseline post-disable. (Not executed — goal is find_root_cause_only; this is the test a fix-mode session should run to verify.)"
  fix_rationale: "N/A for this diagnose-only session — root cause is a static, macro-independent OS-level resource (the CGEventTap's event subscription mask), not a runtime leak tied to macro start/stop bookkeeping. The eventual fix is to remove the unused event types from the `vec![...]` passed to CGEventTap::new (observer.rs:254-267), since none of them are read anywhere in the callback."
  blind_spots: "Have not run the app under Instruments/`top`/`fs_usage` to directly measure CPU delta with vs without the unused event types — this is inference from macOS CGEventTap semantics (documented, well-known behavior for over-broad HeadInsertEventTap masks) plus code-level proof the events are unused, not a direct profiler measurement. Have not ruled out that the user's 'general slowdown' perception is partly recency/anchoring bias following the visibly-laggy run — but the gap was independently re-confirmed by the user in a follow-up UAT round (see 09-UAT.md commit ac42a7c), so it is being treated as a real, reproducible report, not dismissed as pure perception. Did not test on an actual macOS device in this session (source-only investigation)."

next_action: root cause confirmed — diagnose-only mode, returning ROOT CAUSE FOUND (no fix applied)

## Symptoms

expected: Once all macros are disabled and the engine's input-injection paths are idle, AutoMux's CPU/resource usage returns to baseline and does not continue to degrade overall system responsiveness.
actual: User ran a global-mode macro triggered via hotkey at a 100ms interval. The laptop lagged while it ran (may be expected at 100ms). But after turning the macro off, "automux in general slowed down even after macros turned off" — the slowdown persisted past macro deactivation, which is NOT expected.
errors: None reported by user (no visible crash/error dialog); check app's stdout/stderr logs and Rust backend for leaked resources.
reproduction: In `npm run tauri dev` on macOS - create a macro bound to a global hotkey with a fast interval (~100ms), enable it via the hotkey, let it fire for a while, then disable it (toggle off / press hotkey again) and observe whether CPU usage, event loop responsiveness, or UI stays degraded afterward.
started: Discovered during UAT for Phase 9 (Parallel Macro Execution) - added support for multiple concurrent macros firing via a single Scheduler task and a bounded action_tx (capacity 1024) channel with try_send in the timer hot path and .await sends on Hold start/stop paths.

## Eliminated

- hypothesis: "Scheduler leaves an IntervalTask/timeline entry alive after a macro is disabled (timer keeps firing in the background)."
  evidence: "Traced Scheduler::stop_macro (scheduler/mod.rs:317-337): collects all StepIds for the macro_id, removes each from interval_tasks AND its current timeline slot via remove_from_timeline. reevaluate_all_macros (state/mod.rs:441-475) sends StopMacro for any macro that becomes disabled or target-mismatched. Scheduler::run's main loop (scheduler/mod.rs:159-189) blocks entirely on intent_rx.recv() (0% CPU) once timeline is empty. No orphaned entries found for the disable path."
  timestamp: 2026-07-22T00:15:00Z

- hypothesis: "StateActor fails to clear active_holds / registry bookkeeping when a macro is disabled, causing extra per-action work on every subsequent event."
  evidence: "release_holds (scheduler/mod.rs:340-350) is invoked from stop_macro and removes+drains active_holds for the macro_id before sending HoldRelease. StateActor holds no macro-scoped per-action state of its own beyond `state.macros` (state/mod.rs:152-167) — handle_action (state/mod.rs:216-256) does a fresh HashMap lookup per action, nothing accumulates."
  timestamp: 2026-07-22T00:20:00Z

- hypothesis: "Frontend polling/listener setup (state-changed listener, 3s accessibility poll) grows unbounded or fails to clean up during a fast-interval macro run, causing sustained frontend/webview lag."
  evidence: "broadcast_state() (state/mod.rs:477-480) is only called from the Priority-1 `receiver.recv()` branch of StateActor::run (state/mod.rs:191-211) — i.e. on IPC/hotkey Intents, NOT on the Priority-2 `action_rx.recv()` branch that fires interval clicks. A 100ms macro firing does not trigger state-changed events, so the frontend listener (App.tsx:158-166) does not receive proportional traffic. The 3s accessibility poll (App.tsx:169-182) is correctly cleaned up via onCleanup/clearInterval and unrelated to macro rate."
  timestamp: 2026-07-22T00:25:00Z

## Evidence

- timestamp: 2026-07-22T00:05:00Z
  checked: src-tauri/src/scheduler/mod.rs (full file)
  found: Scheduler is a single Tokio task; stop_macro/StopAll correctly remove IntervalTask entries and timeline slots; idle state (no timers) blocks on intent_rx.recv() only — 0% CPU by design (comment at line 181).
  implication: Scheduler-side timer cleanup is correct; not the source of persistent slowdown.

- timestamp: 2026-07-22T00:08:00Z
  checked: src-tauri/src/state/mod.rs (full file)
  found: handle_action (two-phase dispatch gate) is stateless per call; reevaluate_all_macros sends StartMacro/StopMacro per current enabled+target-match state on every relevant Intent; broadcast_state only fires on the Intent-channel branch, not the action-channel branch.
  implication: StateActor has no accumulating per-macro state and does not amplify frontend traffic proportional to interval rate.

- timestamp: 2026-07-22T00:12:00Z
  checked: src-tauri/src/platform/macos/observer.rs (full file, especially initialize_tap at lines 234-494)
  found: "CGEventTap::new is called with an event mask (lines 254-267) that includes MouseMoved, LeftMouseDragged, RightMouseDragged, OtherMouseDragged, and ScrollWheel — none of which are referenced anywhere in the callback body (checked both the LLMHF_INJECTED registry-tracking switch at 292-321 and the KeyDown-gated hotkey/emergency-stop logic at 326-447). The tap is created exactly once for the app's lifetime (idempotent via TAP_INITIALIZED/TAP_STARTING guards) and is never torn down or reconfigured when macros are disabled."
  implication: "This is a permanent, macro-independent CGEventTap subscription to high-frequency system-wide events with zero functional use. Per macOS CGEventTap semantics (ListenOnly + HeadInsertEventTap on the HID stream), this forces every mouse-movement/drag/scroll event system-wide through a synchronous relay to this process for the entire time the app is running — a known class of system-wide-responsiveness tax that is completely decoupled from whether any macro is enabled. This matches the reported symptom precisely: slowdown persists after all macros are turned off, because disabling a macro never touches this tap."

- timestamp: 2026-07-22T00:16:00Z
  checked: "git blame -L 250,270 src-tauri/src/platform/macos/observer.rs"
  found: "The full event mask (including the 5 unused high-frequency types) traces to commit 0cbbac6 ('v1.0.0-rc'), the project's very first commit — unchanged since. Confirmed not introduced by Phase 9."
  implication: "Pre-existing latent design flaw, not a Phase-9 regression — Phase 9's fast (100ms), continuously-firing, hotkey-toggled global macro is simply the first usage pattern that runs long/fast enough to make this baseline tax clearly noticeable to the user."

- timestamp: 2026-07-22T00:18:00Z
  checked: src-tauri/src/platform/macos/input.rs and how inject_mouse_button_raw/inject_key post events
  found: "All synthetic events are posted via `.post(CGEventTapLocation::HID)` (e.g. input.rs:118-121) — the same location the tap listens at with HeadInsertEventTap. Since the tap intercepts its own re-posted events (this is how LLMHF_INJECTED tagging + REGISTRY tracking works by design), every interval-fired click (down+up) also round-trips through the same callback, taking the REGISTRY mutex twice per fire."
  implication: "Contributing/compounding factor for the WHILE-RUNNING lag (proportional to macro rate, ~10/sec at 100ms) — but this traffic stops the instant the macro's IntervalTask is removed from the Scheduler, so it does not explain the POST-STOP persistence; it explains the 'laptop lagged while it ran' half of the report, which the user themselves suspected might be expected."

- timestamp: 2026-07-22T00:20:00Z
  checked: "src-tauri/src/state/mod.rs:240-241 (debug_assertions eprintln in handle_action)"
  found: "`#[cfg(debug_assertions)] eprintln!(\"[Action] macro={} {:?}\", mac.name, action.action_type);` fires on every dispatched action. The reproduction steps explicitly use `npm run tauri dev`, which is a debug build."
  implication: "Adds one synchronous stdout write per click (~10/sec at 100ms) during active macro runs in dev mode only — another WHILE-RUNNING-only compounding factor, not relevant to release builds or to the post-stop persistence."

- timestamp: 2026-07-22T00:28:00Z
  checked: "src/App.tsx:317-323 (handleCardSetTriggerKey) cross-referenced with observer.rs:417-446 (hotkey dispatch)"
  found: "On macOS, setting a macro's trigger key calls BOTH `bind_hotkey` (registers a HotkeyBinding with modifiers=0 in HOTKEY_BINDINGS) AND `set_macro_trigger_key` (registers the same keycode in MACRO_TRIGGER_KEYS via reevaluate_all_macros -> update_macro_trigger_keys). In the tap callback, both the HOTKEY_BINDINGS loop (lines 417-436) and the MACRO_TRIGGER_KEYS lookup (lines 439-446) run unconditionally (not else-if) for the same KeyDown event, each independently `tx.try_send`-ing a `ToggleMacroHotkey(macro_id)` Intent for the SAME macro. Additionally, `HotkeyBinding::matches` with modifiers=0 (`flags.bits() & 0 == 0`) trivially matches regardless of what modifier keys are actually held."
  implication: "Separate, real double-dispatch/reliability bug (a single physical key press can send two toggle intents, which could cancel out or cause flicker depending on timing) — related to the same hotkey subsystem and worth flagging, but it is a correctness/reliability issue, not the driver of the sustained resource/perf symptom being investigated here. Documented as a related finding, not the root cause of G-09-1a."

- timestamp: 2026-07-22T00:30:00Z
  checked: "git show ac42a7c:.planning/phases/09-parallel-macro-execution/09-UAT.md and 09-REVIEW.md (cross-branch, not on this worktree's HEAD)"
  found: "Confirms gap G-09-1a was independently re-reported by the user in a follow-up UAT round with matching phrasing ('automux in general slowed down even after macros turned off'). 09-REVIEW.md's prior code-review passes never flagged the CGEventTap event mask — no mention of CGEventTap/MouseMoved/event-mask scoping anywhere in that review document."
  implication: "This is a genuinely undiscovered root cause (not something already tracked/dismissed by a prior review pass), and the symptom is a real, reproducible, twice-reported user observation rather than a one-off UAT artifact."

## Resolution

root_cause: "The macOS CGEventTap installed in `initialize_tap()` (src-tauri/src/platform/macos/observer.rs:250-268) subscribes to 5 high-frequency event types — CGEventType::MouseMoved, LeftMouseDragged, RightMouseDragged, OtherMouseDragged, and ScrollWheel — that the tap's callback never reads or acts upon anywhere in its body (verified against both the LLMHF_INJECTED registry-tracking switch at lines 292-321 and the KeyDown-gated hotkey/emergency-stop/trigger-key logic at lines 326-447; the only thing ever done with these events is passing them through unmodified at line 449). This tap is created exactly once, for the entire app lifetime (lib.rs:65-72, guarded idempotent via TAP_INITIALIZED/TAP_STARTING), and is never disabled, torn down, or narrowed when macros are stopped — it is completely decoupled from macro enabled/disabled state. Because it is a `ListenOnly` tap positioned at `CGEventTapPlacement::HeadInsertEventTap` on the HID event stream, macOS must synchronously relay every matching event system-wide (not just AutoMux's own events — ALL mouse movement, dragging, and scrolling on the entire machine) through this process before continuing event delivery. This is a well-known class of macOS system-responsiveness tax for over-broad Quartz Event Tap masks, and it persists for as long as the app process is alive with the tap active — which precisely explains why the user observed AutoMux 'in general slowed down even after macros turned off': disabling a macro stops the Scheduler's timer (verified correct, see Eliminated) but does nothing to this tap's subscription. This event mask has been present unchanged since the project's first commit (0cbbac6, confirmed via git blame) — it is a pre-existing latent flaw, not a Phase 9 regression; Phase 9's fast, continuously-firing, hotkey-toggled global macro is simply the first workload demanding/long-running enough to make the baseline tax clearly noticeable. Two compounding-but-not-causal factors were also found and are worth fixing alongside: (1) synthetic click events are posted at the same CGEventTapLocation::HID the tap listens at, so they loop back through the same callback during active runs (input.rs), and (2) a `#[cfg(debug_assertions)]` eprintln fires per dispatched action (state/mod.rs:240-241), adding dev-build-only overhead — both are strictly proportional to active-macro time and do not explain the post-stop persistence on their own."
fix:
verification:
files_changed: []
