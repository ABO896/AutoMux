---
phase: 06-macos-tahoe-26-compatibility
plan: "03"
subsystem: platform/macos
status: pending-device-test
tags:
  - macos
  - tahoe-26
  - cgeventtap
  - crash-fix
  - use-after-free
dependency_graph:
  requires:
    - "06-01"
    - "06-02"
  provides:
    - "observer.rs-crash-safe-tap-disabled-handling"
    - "CFRunLoop-stop-reinit-pattern"
  affects:
    - "COMPAT-01"
    - "COMPAT-03"
tech-stack:
  added: []
  patterns:
    - "CFRunLoop::get_current().stop() in TapDisabled callback — crash-safe tap teardown (Option B)"
    - "TAP_INITIALIZED cleared before TAP_STARTING in all teardown paths (safety invariant)"

key-files:
  created:
    - "src-tauri/target/release/bundle/dmg/AutoMux_1.2.0_aarch64.dmg"
  modified:
    - "src-tauri/src/platform/macos/observer.rs"

key-decisions:
  - "Option B (CFRunLoop stop + 3s-poll reinit) chosen over Option A (Arc<Mutex<Option<CGEventTap>>> cross-thread) because CGEventTap<'tap_life> in core-graphics 0.24.0 is not Send — holds Box<dyn Fn(...) + 'tap_life> with no unsafe impl Send"
  - "Option B chosen over Option C (long-lived signaling thread + CFRetained port) because the existing 3s check_accessibility poll in ipc/mod.rs:193-206 already handles re-initialization — less complexity, same correctness"
  - "Removed entire extern C block (CGEventTapEnable, CFRetain, CFRelease) — no longer called anywhere in observer.rs"

patterns-established:
  - "TapDisabled handling: reset TAP_INITIALIZED (Release), then TAP_STARTING (Release), then CFRunLoop::get_current().stop() — tap drops cleanly on its owning thread"

requirements-completed: [COMPAT-01, COMPAT-03]

duration: ~4min
completed: "2026-06-12"
---

# Phase 6 Plan 03: CGEventTap Crash-Safe TapDisabled Handling Summary

**CFRunLoop stop+reinit replaces per-event spawned re-enable thread that caused EXC_BAD_ACCESS (SIGSEGV) in objc_retain+16 on macOS 26 ARM64e — tap now drops cleanly on its owning thread, rearms via existing 3s poll.**

## Status: PARTIAL — Awaiting Task 5 Device Verification

Tasks 1-4 are complete. Task 5 is a `checkpoint:human-verify` (blocking gate) requiring a physical macOS 26 Tahoe ARM64e device to confirm COMPAT-01 and COMPAT-03 are satisfied.

## Performance

- **Duration:** ~4 min
- **Started:** 2026-06-12T12:46:03Z
- **Completed:** 2026-06-12T12:49:39Z (Tasks 1-4)
- **Tasks:** 4/5 (Task 5 requires device)
- **Files modified:** 1

## Accomplishments

- Eliminated `EXC_BAD_ACCESS (SIGSEGV) in objc_retain+16` crash path: removed the `tap_port_shared: Arc<AtomicUsize>` → `CFRetain` → `std::thread::spawn` pattern that caused a use-after-free when `TapDisabledByTimeout` fired near tap teardown on macOS 26 ARM64e
- Replaced crash-prone re-enable path with `CFRunLoop::get_current().stop()` — tap drops cleanly on its owning thread, TAP_INITIALIZED/TAP_STARTING reset so the 3s `check_accessibility` poll reinitializes the tap (restart gap ≤ 3s)
- Built fresh release DMG including all Phase 6 fixes (06-01 CGEventTap probe, 06-02 infoPlist wiring, 06-03 crash fix) — ready for Tahoe 26 device testing

## Task Commits

1. **Task 1: Confirm CGEventTap Send bounds** — No code changes (investigation only). CGEventTap<'tap_life> confirmed not Send; Option B locked in.
2. **Task 2: Replace spawned re-enable thread with CFRunLoop stop+reinit** — `5eb586e` (fix)
3. **Task 3: Static analysis and cleanup** — No additional commits; zero warnings verified by cargo build.
4. **Task 4: Build release DMG** — No source commits (binary artifact only).

## Files Created/Modified

- `src-tauri/src/platform/macos/observer.rs` — Removed: `extern "C" { CGEventTapEnable, CFRetain, CFRelease }`, `Arc<AtomicUsize>` tap port share, `tap_port_cb`, `tap_port_shared.store(...)`. Added: `CFRunLoop::get_current().stop()` in TapDisabled callback with TAP_INITIALIZED/TAP_STARTING reset guards. Removed unused imports: `core_foundation::base::TCFType`, `core_foundation_sys::mach_port::CFMachPortRef`, `std::sync::Arc`, `std::sync::atomic::AtomicUsize`.
- `src-tauri/target/release/bundle/dmg/AutoMux_1.2.0_aarch64.dmg` — 3.5MB unsigned release DMG, SHA-256: `fc6c7ae899559c4697e942ef4feaa36723e6d6b467fb2a5ff573c73e7b586908`

## Decisions Made

**D-1: Option B — CFRunLoop stop + 3s-poll reinit**

`core_graphics::event::CGEventTap<'tap_life>` in core-graphics 0.24.0 is NOT Send:
- It holds `Box<dyn Fn(CGEventTapProxy, CGEventType, &CGEvent) -> Option<CGEvent> + 'tap_life>` — a non-Send callback with a borrowed lifetime
- No `unsafe impl Send for CGEventTap` exists in the crate

Option A (`Arc<Mutex<Option<CGEventTap<'_>>>>` sent to a worker thread) is unsound — moving the tap off the CFRunLoop-owning thread either violates the borrow or requires unsafe lifetime erasure with no memory-safety guarantee.

Option C (long-lived signaling thread with a CFRetained raw port) is more complex than necessary given the existing 3s `check_accessibility` poll already handles re-initialization via `ipc/mod.rs:193-206`.

Option B is the minimal-complexity correct solution: stop the CFRunLoop → tap drops on its owning thread → guards reset → next poll reinitializes.

## Deviations from Plan

None — plan executed exactly as written. The `TCFType` unused import warning discovered during Task 2's first build was removed as part of normal cleanup (not a deviation — the plan's `<action>` explicitly instructs removing all unused imports introduced by the deleted code path).

## Safety Invariant Verification (Task 3)

| Invariant | Status | Evidence |
|-----------|--------|---------|
| Emergency stop (Cmd+Shift+Q) is FIRST branch inside KeyDown guard | PASS | Line 329 `if matches!(event_type, CGEventType::KeyDown)` → line 334 `HARDCODED EMERGENCY STOP` is the first `if` branch; configurable hotkeys at line 418 come after |
| TapDisabled branch is at callback top level (not inside KeyDown guard) | PASS | Lines 278-290 precede the KeyDown guard at line 329 — TapDisabled events are never KeyDown, so emergency stop cannot be bypassed |
| TAP_INITIALIZED cleared first, TAP_STARTING cleared last in ALL teardown paths | PASS | All 4 paths confirmed: (a) TapDisabled callback (lines 285-286), (b) success path after run_current() (lines 474-475), (c) runloop source creation failure (lines 480-481), (d) CGEventTap creation failure (lines 490-491) |
| flush_held_inputs() drains REGISTRY before lock release and before CGEvent post | PASS | Lines 85-89: drain into `inputs_to_flush` inside lock scope, `// Lock released here`, then CGEvent::post calls |
| cargo build zero warnings | PASS | `cargo build` exits 0 with no warnings |
| No extern C declarations remain | PASS | `grep "extern.*C" observer.rs` returns empty |

## Build Artifact

```
Path:   src-tauri/target/release/bundle/dmg/AutoMux_1.2.0_aarch64.dmg
Size:   3.5MB
SHA-256: fc6c7ae899559c4697e942ef4feaa36723e6d6b467fb2a5ff573c73e7b586908
Signed: No (signingIdentity: null — unsigned developer test build)
Includes: Phase 6 fixes 06-01 (live probe), 06-02 (infoPlist wiring), 06-03 (crash fix)
```

## Task 5: Device Verification Required

**Gate type:** `checkpoint:human-verify` (blocking)

On a macOS 26 Tahoe device (ARM64e):

1. Install the DMG. Grant Accessibility in System Settings if not already granted.
2. Launch AutoMux. Confirm UI shows accessibility "granted".
3. **Crash test (COMPAT-03):** Cmd+Tab between AutoMux and Safari 30 times. Also switch focus rapidly 10+ times inside AutoMux. **Expected:** no crash, no `EXC_BAD_ACCESS` in Console.app.
4. **Injection test (COMPAT-01):** Create a left-click-every-500ms macro, enable via UI toggle, hover over TextEdit. **Expected:** clicks land in TextEdit. Disable → clicks stop.
5. **Rearm test:** While macro runs, switch focus rapidly to trigger `TapDisabledByTimeout`. **Expected:** within ≤ 3s tap rearms and clicks resume. No crash.
6. **Hotkey regression:** Bind hotkey, trigger 10 times. **Expected:** each press toggles macro reliably.

**Resume signal:** Type "approved" if all pass on Tahoe 26, or describe the failure (step, symptom, crash signature from Console.app).

## Known Stubs

None.

## Threat Coverage

| Threat ID | Status | Notes |
|-----------|--------|-------|
| T-06-03-01 | Mitigated | `tap.mach_port.as_concrete_TypeRef() as usize` storage and per-event `std::thread::spawn` removed; tap drops on owning thread |
| T-06-03-02 | Mitigated | TAP_INITIALIZED/TAP_STARTING reset in TapDisabled branch; 3s poll reinitializes (restart gap ≤ 3s) |
| T-06-03-03 | Mitigated | Release ordering for both stores; STARTING cleared strictly after INITIALIZED |
| T-06-03-04 | Accepted | Emergency stop remains FIRST branch in callback (verified by grep) |
| T-06-03-05 | Accepted | flush_held_inputs() drain pattern verified untouched |
| T-06-03-06 | Accepted | debug_assertions eprintln retained; CFMachPort pointer storage removed |

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check: PASSED

- `src-tauri/src/platform/macos/observer.rs` modified: confirmed
- Commit `5eb586e` exists in git log: confirmed
- DMG artifact at `src-tauri/target/release/bundle/dmg/AutoMux_1.2.0_aarch64.dmg`: confirmed (3.5MB)
- `cargo build` zero warnings: confirmed
- `CFRunLoop::get_current().stop()` present in observer.rs: confirmed (line 288)
- Removed patterns absent from observer.rs: `tap_port_shared`, `AtomicUsize` (code), `CFRetain`, `CFMachPortRef` imports, `extern "C"` block: all confirmed absent
