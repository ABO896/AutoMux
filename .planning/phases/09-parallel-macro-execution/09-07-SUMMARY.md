---
phase: 09-parallel-macro-execution
plan: 7
subsystem: infra
tags: [macos, cgeventtap, nsworkspace, objc2, objc2-app-kit, objc2-foundation, performance, targeting]

# Dependency graph
requires:
  - phase: 09-parallel-macro-execution
    provides: parallel execution scheduler/state changes (plans 09-01..09-06) that surfaced these UAT gaps
provides:
  - Narrowed macOS CGEventTap event mask (8 consumed types only)
  - Instrumented + hardened NSWorkspace active-app observer with userInfo-based active-app reads
affects: [09-UAT, macOS device verification]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "NSWorkspace activation notification: read the activated app from the notification's own userInfo payload (NSWorkspaceApplicationKey -> NSRunningApplication) instead of re-querying frontmostApplication() — avoids the re-query race, with frontmostApplication() as fallback only."

key-files:
  created: []
  modified:
    - src-tauri/src/platform/macos/observer.rs

key-decisions:
  - "Kept the raw msg_send! registration for the NSWorkspace observer instead of switching to the typed addObserverForName_object_queue_usingBlock API. Verified block-signature compatibility (block2's DynBlock<F> is a type alias for Block<F>, and RcBlock derefs to Block<F>, so the typed call would compile), but that typed method returns a non-Optional Retained<ProtocolObject<dyn NSObjectProtocol>> per Apple's own API contract — it structurally cannot represent a registration failure, which would defeat G-09-1c's explicit-failure-logging goal. The existing msg_send! + Option<Retained<NSObject>> path gives a real, observable None branch, so it was kept and an explicit eprintln failure branch was added to it."

patterns-established:
  - "Notification-payload-first, re-query-as-fallback: when both a push notification userInfo and a live re-query API can supply the same fact, prefer the payload (avoids TOCTOU races) and fall back to the live query only if the payload lookup misses."

requirements-completed: [EXEC-01, EXEC-02]

coverage:
  - id: D1
    description: "CGEventTap event mask narrowed to the 8 event types the callback consumes (removed MouseMoved, LeftMouseDragged, RightMouseDragged, OtherMouseDragged, ScrollWheel) — eliminates the permanent, macro-independent system-responsiveness tax (G-09-1a)."
    requirement: "EXEC-01"
    verification:
      - kind: other
        ref: "cd src-tauri && cargo build && cargo clippy --all-targets -- -D warnings (both exit 0)"
        status: pass
    human_judgment: true
    rationale: "The behavioral claim (system responsiveness returns to baseline after stopping a fast-interval macro) is an OS-level, real-device perception that cannot be measured or asserted from source/compile-time checks alone — requires the plan's <human-check> on a real macOS host in the next UAT round."
  - id: D2
    description: "NSWorkspace active-app observer hardened: registration-failure branch now logs explicitly, handler reads the activated app from the notification's userInfo (NSWorkspaceApplicationKey -> NSRunningApplication -> bundleIdentifier) with frontmostApplication() fallback, and logs every active-app change under debug_assertions (G-09-1c)."
    requirement: "EXEC-02"
    verification:
      - kind: other
        ref: "cd src-tauri && cargo build && cargo clippy --all-targets -- -D warnings && grep -c 'active-app changed' src/platform/macos/observer.rs (all pass; grep returns 1)"
        status: pass
    human_judgment: true
    rationale: "Whether the NSWorkspace notification actually fires on real app switches, and whether a target-app-scoped macro now fires when its target is focused, can only be confirmed by a human on a real macOS device (the plan's <human-check>) — not by static analysis or a headless build."

# Metrics
duration: 20min
completed: 2026-07-22
status: complete
---

# Phase 09 Plan 7: macOS Observer Gap Closure (G-09-1a, G-09-1c) Summary

**Narrowed the CGEventTap subscription mask to the 8 consumed event types and hardened the NSWorkspace active-app observer with userInfo-based reads and full logging instrumentation, closing two Phase 9 UAT gaps in `src-tauri/src/platform/macos/observer.rs`.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-22
- **Tasks:** 2
- **Files modified:** 1 (`src-tauri/src/platform/macos/observer.rs`)

## Accomplishments
- G-09-1a: Removed 5 unused high-frequency event types (`MouseMoved`, `LeftMouseDragged`, `RightMouseDragged`, `OtherMouseDragged`, `ScrollWheel`) from the CGEventTap's subscription mask in `initialize_tap()`. The tap callback never read any of them; per macOS CGEventTap semantics for a `ListenOnly` `HeadInsertEventTap` on the HID stream, subscribing to them forced macOS to relay all system-wide mouse motion/drag/scroll through this process for the app's entire lifetime — a permanent, macro-independent responsiveness tax. Mask is now exactly the 8 types the callback consumes (Key Down/Up, Left/Right/Other Mouse Down/Up).
- G-09-1c: `MacPlatformObserver::start_observing()`'s NSWorkspace activation handler now (1) logs an explicit failure message if `addObserverForName` returns nil, instead of silently doing nothing; (2) reads the activated app's bundle ID from the notification's own `userInfo` payload (`NSWorkspaceApplicationKey` -> downcast to `NSRunningApplication` -> `bundleIdentifier()`), falling back to `frontmostApplication()` only if that lookup misses; and (3) logs `[Observer] active-app changed -> ...` under `#[cfg(debug_assertions)]` on every resolved change, so the next device UAT can empirically confirm the notification fires on real app switches.

## Task Commits

Each task was committed atomically:

1. **Task 1: Narrow the CGEventTap event mask to the consumed event types (G-09-1a)** - `d6dd9b4` (fix)
2. **Task 2: Instrument and harden the NSWorkspace active-app observer (G-09-1c)** - `d1dfca1` (fix)

**Plan metadata:** (recorded below in state/roadmap update commit)

## Files Created/Modified
- `src-tauri/src/platform/macos/observer.rs` - Narrowed CGEventTap event mask (Task 1); hardened + instrumented NSWorkspace active-app observer registration and handler (Task 2)

## Decisions Made
- Kept the raw `msg_send!` registration for the NSWorkspace observer rather than switching to the typed `addObserverForName_object_queue_usingBlock` API. Confirmed the typed call would have compiled (block-signature compatible: `block2::DynBlock<F>` is a type alias for `Block<F>`, and `RcBlock<F>` derefs to `Block<F>`), but that typed method's return type is a non-Optional `Retained<ProtocolObject<dyn NSObjectProtocol>>` per Apple's own API contract — it cannot represent a nil/failed registration, which would defeat the explicit-failure-logging goal the plan required. The existing `msg_send!` + `Option<Retained<NSObject>>` path preserves a real, observable failure branch, so it was kept with the new `eprintln!` failure log added to its previously-empty else-branch.
- Read the activated app from the notification's `userInfo` with a fallback to `frontmostApplication()` (rather than removing the re-query path entirely) — the plan explicitly required "fall back to the existing `frontmostApplication()` path only if the userInfo lookup yields None," to keep the handler robust against any notification shape that lacks the expected payload key.

## Deviations from Plan

None - plan executed exactly as written. Both tasks matched the plan's `<action>` blocks; the only implementation-time decision (raw `msg_send!` vs. typed API) was explicitly anticipated and permitted by the plan's own fallback clause in Task 2's `<action>`.

## Issues Encountered
- Initial build failed with `E0133: use of extern static is unsafe` when accessing `NSWorkspaceApplicationKey` (an `extern "C" static`) directly inside the handler closure. Fixed by wrapping the access in an `unsafe { NSWorkspaceApplicationKey }` block, mirroring the existing pattern already used for `NSWorkspaceDidActivateApplicationNotification` a few lines above in the same function. Rebuilt clean immediately after.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Both automated gates (`cargo build`, `cargo clippy --all-targets -- -D warnings`) pass with zero warnings, and `cargo test` (13/13) passes unchanged. The `grep -c 'active-app changed'` instrumentation gate returns 1. The behavioral truths for both gaps — baseline responsiveness after stopping a fast-interval macro (G-09-1a), and app-switch logging plus a target-app-scoped macro actually firing when focused (G-09-1c) — require the plan's `<human-check>` on a real macOS device in the next UAT round; this plan does not and cannot claim those closed from source-only verification. No blockers for the rest of Phase 9 (plan 09-08 covers the separate G-09-1b delete-macro UI gap and is independent of this plan's files).

---
*Phase: 09-parallel-macro-execution*
*Completed: 2026-07-22*
