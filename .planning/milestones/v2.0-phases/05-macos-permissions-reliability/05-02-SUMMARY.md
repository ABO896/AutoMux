---
phase: 05-macos-permissions-reliability
plan: "02"
subsystem: ui
tags: [solidjs, accessibility, permissions, macos, state-machine]

# Dependency graph
requires: []
provides:
  - accessibilityPending SolidJS signal with 3-branch Show UI (Granted / Pending / Denied)
  - clearPending() helper cancelling in-flight 30s timeout on unmount (T-05-04)
  - Grant Access button hidden while pending state is active
affects: [05-macos-permissions-reliability]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Module-level non-reactive timeout handle (_pendingTimeoutId) alongside reactive signal for pending state"
    - "3-branch nested Show pattern: outer=granted, middle=pending, inner=denied"
    - "clearPending() helper centralises signal reset + timeout cancellation"

key-files:
  created: []
  modified:
    - src/App.tsx

key-decisions:
  - "Store timeout handle as module-level non-reactive variable (not createSignal) — timeout IDs are not reactive data"
  - "Pending state cleared by BOTH polling effect (on grant) and 30s fallback timeout (on no response)"
  - "clearPending() called in onCleanup block (T-05-04) to prevent timeout leak on component unmount"
  - "Warning glow used for pending dot (--color-warning-glow is defined in App.css)"

patterns-established:
  - "Non-reactive module-level handle pattern: use let _handle = null (not createSignal) for OS handles and timeout IDs"

requirements-completed:
  - PERM-01

# Metrics
duration: 15min
completed: 2026-06-02
---

# Phase 5 Plan 02: macOS Permissions & Reliability — Pending UI State Summary

**SolidJS pending-approval state machine: accessibilityPending signal, 30s fallback timeout, 3-branch Granted/Pending/Denied Show UI, Grant Access button hidden while pending**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-06-02T00:00:00Z
- **Completed:** 2026-06-02T00:15:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Added `accessibilityPending` boolean signal (false by default) for the window between Grant Access click and OS dialog resolution
- Added `clearPending()` helper centralising signal reset and in-flight timeout cancellation
- Updated `handleRequestAccess` to enter pending state, start 30s fallback timeout, and call clearPending on instant-grant or error
- Updated 3s polling effect to call `clearPending()` when `check_accessibility` returns true (restart-free grant detection)
- Added `clearPending()` to existing `onCleanup` block — cancels any in-flight 30s timeout on component unmount (T-05-04 mitigation)
- Restructured accessibility Show block to 3-branch nested Show: Granted (top) / Pending (middle, amber dot + "Pending...") / Denied (fallback)
- Grant Access button Show guard updated to `accessibility() === false && !accessibilityPending()` — button hidden during pending window

## Task Commits

1. **Task 1: Add accessibilityPending state machine and 3-branch permission UI (PERM-01)** - `c649a19` (feat)

**Plan metadata:** (committed with this SUMMARY)

## Files Created/Modified
- `src/App.tsx` - Added pending state machine: _pendingTimeoutId, accessibilityPending signal, clearPending helper, updated handleRequestAccess, polling effect, onCleanup, and 3-branch UI Show

## Decisions Made
- `_pendingTimeoutId` stored as a module-level `let` (not `createSignal`) — timeout handles are not reactive data and should not trigger re-renders when assigned
- `clearPending()` is a plain `function` declaration (not `const`) inside `App` — hoisted in function scope, callable before its definition in source order (needed by polling effect and onCleanup which appear earlier in the file)
- `--color-warning-glow` is defined in App.css so the glow shadow was included on the pending dot (matching the danger/success dot pattern)
- 30s fallback timeout chosen per plan spec — gives the user ample time to interact with the OS dialog; reverts to Denied if no approval received

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. The `accessibilityPending` signal is UI-only; the authoritative permission state remains `AXIsProcessTrusted()` via the Rust backend `check_accessibility` command. T-05-04 (timeout leak on unmount) was mitigated as specified.

## Known Stubs

None. The pending state machine is fully wired: signal, 30s fallback, polling clear, onCleanup cancel, 3-branch UI, and button guard are all connected.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness
- PERM-01 UI fix complete: no false "not granted" state shown between Grant Access click and OS dialog approval
- Plan 05-01 (Rust backend) must also complete for the full PERM-01 fix to be end-to-end reliable
- Both plans in Wave 1 are parallel — merge order doesn't matter

---
*Phase: 05-macos-permissions-reliability*
*Completed: 2026-06-02*
