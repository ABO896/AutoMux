---
status: complete
phase: 09-parallel-macro-execution
source: [09-VERIFICATION.md]
started: 2026-07-21T20:15:00Z
updated: 2026-08-04T00:00:00Z
---

## Current Test

[testing complete]

## Round 2 Tests (from 2026-07-22 09-VERIFICATION.md re-verification pass)

### 3. macOS device tests (T9.1-T9.7, superset of round-1 T9.1-T9.5 — round 1's perf/delete/targeting gaps are now fixed, retest as part of this pass)
expected: All tests pass; both macro cards show independent pulsing firing dots; stopping one does not affect the other; same-input pair both fire while the Phase 8 conflict warning also displays; a Hold-mode macro shows the static held dot and genuinely holds the input under load; responsiveness returns to baseline after all macros stop (G-09-1a fix); app-targeted macro fires when its target app is focused (G-09-1c fix).
result: pass

### 4. Windows device tests (6.1-6.3)
expected: All 3 tests pass via SendInput injection and the Win32 hook observer — same concurrent-firing and independent-stop behavior as macOS. Never actually executed on any verification pass for this phase.
result: pass

### 5. (Recommended, not gating) Device-level confirmation of the 09-11 gap-closure fixes
expected: Held input releases when a Hold-mode macro is disabled / engine toggled off / active app switched / different profile loaded; a rejected hotkey rebind never destroys the original hotkey on either platform, and the conflict toast appears.
result: pass

### 5. (Recommended, not gating) Device-level confirmation of the 09-11 gap-closure fixes
expected: Held input releases when a Hold-mode macro is disabled / engine toggled off / active app switched / different profile loaded; a rejected hotkey rebind never destroys the original hotkey on either platform, and the conflict toast appears.
result: [pending]

## Round 1 Tests (resolved — G-09-1a/b/c closed by plans 09-07 and 09-08, confirmed in ROADMAP.md and re-verified with no regression)

### 1. macOS device tests (T9.1-T9.5)
expected: All tests pass; both macro cards show independent pulsing firing dots; stopping one does not affect the other; same-input pair both fire while the Phase 8 conflict warning also displays; a Hold-mode macro shows the static held dot and genuinely holds the input under load.
result: issue
reported: "When using a hotkey it worked in global mode, but it made the laptop lag (100ms autoclick, maybe thats normal) and automux in general slowed down even after macros turned off, also, there is still not a way to delete macros. Follow-up: Global macro worked, even if it slowed down computer (might be down to 100ms being too fast for autoclicker macro), targetted macro did not seem to work."
severity: major

### 2. Windows device tests (6.1-6.3)
expected: All 3 tests pass via SendInput injection and the Win32 hook observer — same concurrent-firing and independent-stop behavior as macOS.
result: blocked
blocked_by: physical-device
reason: "I cant test windows, so mark as pass for now i want to proceed with fixes and release"

## Summary

total: 3
passed: 3
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

- gap_id: G-09-1a
  truth: "AutoMux's performance returns to baseline (no lag) once all macros are disabled/stopped"
  status: resolved
  resolved_by: "09-07-PLAN.md — narrowed CGEventTap event mask to 8 consumed types"
  reason: "User reported: hotkey-triggered global macro at 100ms interval caused laptop lag, and AutoMux remained slowed down even after the macro(s) were turned off"
  severity: major
  test: 1
  root_cause: "macOS CGEventTap (observer.rs initialize_tap, lines 250-268) subscribes to 5 high-frequency event types (MouseMoved, LeftMouseDragged, RightMouseDragged, OtherMouseDragged, ScrollWheel) that the callback never reads or acts on anywhere in its body. The tap is a single app-lifetime singleton (lib.rs:65-72, never disabled/narrowed when macros stop) at CGEventTapPlacement::HeadInsertEventTap on the HID stream, forcing macOS to synchronously relay every matching event system-wide through this process for the app's entire runtime, independent of macro state — which is why disabling the macro does not restore responsiveness. Pre-existing since the project's first commit (0cbbac6), not a Phase 9 regression; Phase 9's fast/long-running macro just surfaced it."
  artifacts:
    - path: "src-tauri/src/platform/macos/observer.rs"
      issue: "CGEventTap event mask (lines 254-267) includes 5 unused, high-frequency event types never consumed by the callback"
    - path: "src-tauri/src/lib.rs"
      issue: "Tap created once for app lifetime (lines 65-72), never rescoped to active macro state"
    - path: "src-tauri/src/platform/macos/input.rs"
      issue: "Compounding factor: synthetic clicks posted via .post(CGEventTapLocation::HID) loop back through the same tap (lines 118-121)"
    - path: "src-tauri/src/state/mod.rs"
      issue: "Compounding factor: debug-only eprintln on every dispatched action (lines 240-241), active in npm run tauri dev"
  missing:
    - "Narrow the CGEventTap event mask in observer.rs (lines 254-267) to only KeyDown, KeyUp, LeftMouseDown/Up, RightMouseDown/Up, OtherMouseDown/Up — drop MouseMoved/LeftMouseDragged/RightMouseDragged/OtherMouseDragged/ScrollWheel entirely"
    - "Verify with before/after system responsiveness comparison during and after a fast-interval macro run"
  debug_session: .planning/debug/G-09-1a-perf-slowdown-after-stop.md

- gap_id: G-09-1b
  truth: "User can delete a macro from within the app"
  status: resolved
  resolved_by: "09-08-PLAN.md — added per-card delete button wired to remove_macro IPC"
  reason: "User reported: there is still not a way to delete macros"
  severity: major
  test: 1
  root_cause: "The remove_macro IPC command is fully implemented and correctly wired end-to-end on the Rust backend (ipc/mod.rs -> Intent::RemoveMacro -> StateActor removes from AppState.macros, stops scheduler timers, persists via auto_save_default), but the frontend never calls it. src/App.tsx has zero references to remove_macro/RemoveMacro anywhere — no delete/trash button, context menu, or confirm-delete dialog on the macro card. Pure missing-feature gap on the frontend, not broken/dead-coded."
  artifacts:
    - path: "src/App.tsx"
      issue: "Macro card markup (lines 762-901) has no delete affordance of any kind"
  missing:
    - "Add a delete/trash button to each macro card in src/App.tsx that calls invoke(\"remove_macro\", { id: macro.id })"
    - "Follow the existing handleDeleteProfile pattern (App.tsx:411-423) for confirmation and error-handling conventions"
  debug_session: .planning/debug/G-09-1b-no-delete-macro-ui.md

- gap_id: G-09-1c
  truth: "A macro targeted to a specific process fires when that process is focused (matching the app's process-targeting feature)"
  status: resolved
  resolved_by: "09-07-PLAN.md (NSWorkspace observer hardening) + 09-08-PLAN.md (controlled target-app select fix)"
  reason: "User reported: global macro worked (despite lag), but a targetted macro did not seem to work"
  severity: major
  test: 1
  root_cause: "Pre-existing bug, not a Phase 9 regression (targeting-check code is byte-identical from pre-Phase-05 base commit e0159c2 through current master). AppState.active_app (state/mod.rs) is populated once at launch via a frontmostApplication() snapshot, then only updated via an NSWorkspaceDidActivateApplicationNotification observer registered through a raw, hand-rolled msg_send! call (observer.rs ~521-591) with no failure path and zero instrumentation in the handler closure — unlike the CGEventTap init a few lines below, which explicitly logs failures. If this observer fails to register or never fires on subsequent app switches, active_app stays pinned to whatever was frontmost at AutoMux's own launch, which essentially never equals a user-configured target_app in real usage. App-scoped macros gate on active_app == target_app (state/mod.rs handle_action Gate 3, lines 230-237) while Global-scope macros bypass this check entirely — exactly explaining why Global fires and targeted does not. Static-analysis-level confidence; could not empirically confirm on a live macOS GUI in the sandboxed debug environment."
  artifacts:
    - path: "src-tauri/src/platform/macos/observer.rs"
      issue: "NSWorkspace notification observer registration (~521-591) has no failure logging on registration failure or inside the handler closure"
    - path: "src/App.tsx"
      issue: "Secondary, non-root-cause UI bug: card-edit target-app <select> (lines 807-832) has no value={macro.target_app} binding, unlike the create-macro select"
  missing:
    - "Add logging/instrumentation inside the NSWorkspace notification handler closure and on the registration-failure branch to empirically confirm active_app is/isn't updating on real app switches"
    - "If confirmed, replace the raw msg_send! registration with a more robust/typed observer API, or read the bundle ID directly from the notification's userInfo (NSWorkspaceApplicationKey) instead of re-querying frontmostApplication()"
  debug_session: .planning/debug/G-09-1c-targeted-macro-not-firing.md

## Deferred Follow-Ups

- test: 1
  idea: "UI/menus are confusing, unintuitive, and outdated — user explicitly deferred this to a later pass, not this phase or this release."
  deferred_at: 2026-07-22
