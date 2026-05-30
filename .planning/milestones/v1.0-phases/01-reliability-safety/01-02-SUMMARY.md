---
phase: 01-reliability-safety
plan: 02
subsystem: platform
tags: [macos, cgevent, cgeventtap, panic-elimination, reliability, cgraphics, rust]

requires: []
provides:
  - Non-panicking CGEventSource construction in MacInputProvider (Option<CGEventSource> with let-Some guard at each caller)
  - CGEventTap inline re-enable on OS timeout (TapDisabledByTimeout/TapDisabledByUserInput detection with CGEventTapEnable call)
affects: [platform-macos, reliability, safety]

tech-stack:
  added: []
  patterns:
    - "Option guard at hot path: fn source() -> Option<T> with let Some(x) = ... else { return; } callers"
    - "CGEventTap timeout recovery: first closure statement matches TapDisabled* and calls CGEventTapEnable via direct FFI"
    - "Local extern C declaration for unexported FFI: declare CGEventTapEnable directly when crate does not re-export"

key-files:
  created: []
  modified:
    - src-tauri/src/platform/macos/input.rs
    - src-tauri/src/platform/macos/observer.rs

key-decisions:
  - "SAFE-01: Changed source() return type to Option<CGEventSource> with .ok() instead of .expect() — construction failure now drops one action silently rather than crashing the process"
  - "RELY-02: Used a local extern C declaration for CGEventTapEnable because core-graphics 0.24.0 does not re-export it; casting CGEventTapProxy (*const c_void) to CFMachPortRef is correct per Apple's CGEventTapCreate docs"
  - "RELY-02: The plan assumed the closure proxy had a .enable() method — it does not (CGEventTapProxy = *const c_void). Fixed by calling CGEventTapEnable via FFI cast instead"

patterns-established:
  - "Option-returning helper with let-Some guard: preferred pattern for fallible resource acquisition on hot paths"
  - "Direct extern C for missing crate FFI: declare locally when an upstream crate does not re-export needed OS functions"

requirements-completed: [SAFE-01, RELY-02]

duration: 10min
completed: 2026-05-16
---

# Phase 1 Plan 02: macOS Platform Reliability (SAFE-01 + RELY-02) Summary

**Eliminated CGEventSource panic on macOS injection hot path and added inline CGEventTap re-enable on OS-issued timeout, using Option-guard pattern and direct FFI casting**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-05-16T12:19:00Z
- **Completed:** 2026-05-16T12:29:52Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `MacInputProvider::source()` now returns `Option<CGEventSource>` — a construction failure under resource pressure drops one action silently instead of panicking the process (SAFE-01, D-08)
- All four `inject_*` methods in `macos/input.rs` guard on `let Some(source) = Self::source() else { return; }` — the existing `if let Ok(event) = ...` per-event guard is preserved unchanged
- CGEventTap callback in `observer.rs` now detects `CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput` as its first statement and calls `CGEventTapEnable` inline, restoring hotkey handling for long sessions without user notification (RELY-02, D-09)
- `cargo build --target aarch64-apple-darwin` and `cargo clippy` pass with zero new warnings
- `cargo test --target aarch64-apple-darwin` passes — 3 existing tests pass, none regressed

## Task Commits

1. **Task 1: Eliminate macOS CGEventSource panic (SAFE-01, D-08)** - `ca59a4f` (fix)
2. **Task 2: Re-enable CGEventTap inline on timeout (RELY-02, D-09)** - `919bfc9` (fix)

## Files Created/Modified

- `src-tauri/src/platform/macos/input.rs` — Changed `source()` from `fn source() -> CGEventSource` (panicking `.expect()`) to `fn source() -> Option<CGEventSource>` (`.ok()`); updated all four `inject_*` callers from `let source = Self::source();` to `let Some(source) = Self::source() else { return; };`
- `src-tauri/src/platform/macos/observer.rs` — Added `extern "C" { fn CGEventTapEnable(...) }` declaration; renamed closure parameter `_proxy` to `tap_proxy`; inserted tap-disabled detection branch as the first statement in the CGEventTap callback

## Decisions Made

- Used `.ok()` instead of `.expect()` for `CGEventSource::new()` — aligns with the existing `if let Ok(event) = CGEvent::new_*()` pattern already used throughout the same file
- Added optional `#[cfg(debug_assertions)] eprintln!` traces per project convention (from `state/mod.rs` and `persistence.rs`)
- For RELY-02: declared `CGEventTapEnable` via a local `extern "C"` block because `core-graphics 0.24.0` does not re-export it publicly; cast `CGEventTapProxy (*const c_void)` to `CFMachPortRef` — this cast is correct per Apple's documentation that the tap callback proxy IS the same CFMachPort the tap was created with

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan's proxy.enable() method does not exist on CGEventTapProxy**

- **Found during:** Task 2 (CGEventTap timeout re-enable)
- **Issue:** The plan stated `tap_proxy.enable()` is callable on the closure's first parameter. In practice, `CGEventTapProxy = *const c_void` (a raw pointer type) has no `.enable()` method. The `.enable()` precedent at line 369 (`tap.enable()`) is on the `CGEventTap` struct, not the callback proxy type. The Rust compiler confirmed with `E0599: no method named 'enable' found for raw pointer '*const c_void'`.
- **Fix:** Declared `CGEventTapEnable(tap: CFMachPortRef, enable: bool)` in a local `extern "C"` block (the function exists in Apple's CoreGraphics framework but is not re-exported by `core-graphics 0.24.0`). Cast the proxy to `CFMachPortRef` and called `unsafe { CGEventTapEnable(tap_proxy as CFMachPortRef, true) }`. This is the correct approach — Apple's `CGEventTapCreate` documentation confirms the proxy in the callback is the same object as the tap's CFMachPortRef.
- **Files modified:** `src-tauri/src/platform/macos/observer.rs` (added `use core_foundation_sys::mach_port::CFMachPortRef;` import and `extern "C"` block)
- **Verification:** `cargo build --target aarch64-apple-darwin` exits 0 with no warnings; `cargo clippy` clean
- **Committed in:** `919bfc9` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug in plan's proxy type assumption)
**Impact on plan:** The fix achieves identical runtime behavior to what the plan intended. The functional result (tap re-enabled on timeout, no user notification) is fully correct. The proxy rename from `_proxy` to `tap_proxy` was retained as planned since the parameter IS used.

## Issues Encountered

The `CGEventTapProxy` type alias (`= *const c_void`) in core-graphics does not expose any methods. The plan assumed the proxy had a `.enable()` method by analogy with `CGEventTap::enable()` — the two are different types. The correct mechanism (direct `CGEventTapEnable` FFI via cast) was applied per Rule 1.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The added `extern "C"` block declares an existing Apple OS function — no new trust boundary. The `unsafe { CGEventTapEnable(...) }` call is bounded to the tap-disabled event path, consistent with the existing `tap.enable()` call at line 396.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- SAFE-01 macOS site (1 of 5) eliminated; remaining 4 Windows sites handled by Plan 03
- RELY-02 macOS long-session hotkey retention complete
- `cargo build` and `cargo test` clean — Plan 03 and Plan 04 can proceed without blockers from this plan

---
*Phase: 01-reliability-safety*
*Completed: 2026-05-16*
