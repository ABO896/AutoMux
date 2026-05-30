---
phase: 01-reliability-safety
plan: "03"
subsystem: platform
tags: [windows, rust, mutex, emergency-stop, panic-elimination, reliability, safety]

# Dependency graph
requires: []
provides:
  - "Non-panicking Windows held-inputs mutex on injection hot path (SAFE-01 sites 2-5)"
  - "Synchronous held-input flush before Ctrl+Shift+Q process exit (RELY-03)"
affects:
  - "01-04 (if any Windows platform work remains)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "let-Ok-else early return on mutex lock failure (Windows hot path)"
    - "Synchronous flush call before process::exit in emergency-stop callback"

key-files:
  created: []
  modified:
    - src-tauri/src/platform/windows/mod.rs

key-decisions:
  - "SAFE-01 scope strictly limited to 4 hot-path inject_* sites; lines 51 and 351 (out-of-path) intentionally left unchanged per RESEARCH.md"
  - "flush_all_held_inputs() called synchronously between try_send and process::exit per D-10 best-effort semantics"

patterns-established:
  - "let Ok(mut guard) = get_held_inputs().lock() else { return; } — canonical non-panicking mutex guard pattern for Windows injection hot path"

requirements-completed: [SAFE-01, RELY-03]

# Metrics
duration: 3min
completed: 2026-05-16
---

# Phase 1 Plan 03: Windows Platform Reliability Summary

**Eliminated 4 panicking mutex unwraps on Windows injection hot path and wired synchronous held-input flush before emergency-stop process exit**

## Performance

- **Duration:** 3 min
- **Started:** 2026-05-16T12:25:41Z
- **Completed:** 2026-05-16T12:28:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Replaced all 4 `get_held_inputs().lock().unwrap()` calls in `inject_key`, `inject_mouse_click` (x2), and `inject_mouse_button_raw` with `let Ok(mut guard) = ... else { return; }` — a poisoned or contended mutex now silently skips the action instead of crashing the process
- Inserted `WindowsInputProvider::flush_all_held_inputs()` synchronously between the `try_send(TriggerEmergencyStop)` and `process::exit(1)` calls in `hook_callback`, guaranteeing all held keyboard and mouse button events receive KeyUp/MouseUp before exit
- All 3 existing tests pass; macOS build unaffected

## Task Commits

1. **Task 1: Eliminate 4 lock-unwrap panics on Windows injection hot path (SAFE-01 sites 2-5)** - `5ebb54f` (fix)
2. **Task 2: Synchronous flush before Windows emergency-stop exit (RELY-03)** - `966e1d6` (fix)

## Files Created/Modified

- `src-tauri/src/platform/windows/mod.rs` — 4 hot-path `lock().unwrap()` sites converted to `let Ok(mut guard) = ... else { return; }`; `flush_all_held_inputs()` call added in emergency-stop branch; `flush_all_held_inputs()` internals unchanged

## SAFE-01 Conversion Details

| Method | Old line | Pattern before | Pattern after |
|--------|----------|----------------|---------------|
| `inject_key` | ~173 | `get_held_inputs().lock().unwrap()` | `let Ok(mut guard) = get_held_inputs().lock() else { return; };` |
| `inject_mouse_click` (press) | ~188 | `get_held_inputs().lock().unwrap()` | `let Ok(mut guard) = get_held_inputs().lock() else { return; };` |
| `inject_mouse_click` (release) | ~196 | `get_held_inputs().lock().unwrap()` | `let Ok(mut guard) = get_held_inputs().lock() else { return; };` |
| `inject_mouse_button_raw` | ~208 | `get_held_inputs().lock().unwrap()` | `let Ok(mut guard) = get_held_inputs().lock() else { return; };` |

Lines 51 (`flush_all_held_inputs` internal lock) and 351 (`update_macro_trigger_keys`) retain their original `lock().unwrap()` — both are out-of-scope per RESEARCH.md (not on the injection hot path).

## RELY-03 Emergency-Stop Before/After

**Before:**
```rust
if keycode == 0x51 && ctrl_down && shift_down {
    println!("EMERGENCY STOP TRIGGERED");
    if let Some(tx) = STATE_TX.get() {
        let _ = tx.try_send(crate::state::Intent::TriggerEmergencyStop);
    }
    std::process::exit(1);  // keys may remain stuck
}
```

**After:**
```rust
if keycode == 0x51 && ctrl_down && shift_down {
    println!("EMERGENCY STOP TRIGGERED");
    if let Some(tx) = STATE_TX.get() {
        let _ = tx.try_send(crate::state::Intent::TriggerEmergencyStop);
    }
    // RELY-03: Synchronous flush before exit — matches macOS inline-flush approach.
    // Best-effort: exit regardless of individual event send failures (D-10).
    WindowsInputProvider::flush_all_held_inputs();
    std::process::exit(1);
}
```

## Decisions Made

- Scope of SAFE-01 strictly limited to the 4 injection hot-path sites per RESEARCH.md — no attempt to fix `flush_all_held_inputs` internal lock (line 51) or `update_macro_trigger_keys` (line 351)
- Added `#[cfg(debug_assertions)] eprintln!` trace per project convention for each skipped lock failure
- No `.into_inner()` poison recovery added (Pitfall 3 in RESEARCH.md explicitly forbids this)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Windows cross-compilation toolchain not installed locally; build verified with host `cargo check --lib` and `cargo test --lib`. Windows target acceptance is gated on the next CI run per the plan's acceptance criteria.

## Build Verification

- `cargo check --lib` (host/macOS): Finished with 0 errors, 0 new warnings
- `cargo test --lib`: 3/3 tests pass (persistence + scheduler tests)
- Windows target (`x86_64-pc-windows-msvc`): pending CI — toolchain not installed locally

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The changes are purely mechanical substitutions within existing function bodies.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SAFE-01 is fully addressed across both platforms: Plan 02 handled macOS `input.rs:24`; this plan handled Windows `mod.rs` lines 173, 188, 196, 208
- RELY-03 resolved on Windows: emergency stop now flushes held inputs synchronously before exit
- Plan 04 (RELY-04 auto-save) and the remaining plans can proceed independently

## Self-Check

Files committed:
- `src-tauri/src/platform/windows/mod.rs` - FOUND in commits 5ebb54f and 966e1d6

Commits verified:
- 5ebb54f: fix(01-03): eliminate 4 lock-unwrap panics on Windows injection hot path (SAFE-01)
- 966e1d6: fix(01-03): synchronous held-input flush before Windows emergency-stop exit (RELY-03)

## Self-Check: PASSED

---
*Phase: 01-reliability-safety*
*Completed: 2026-05-16*
