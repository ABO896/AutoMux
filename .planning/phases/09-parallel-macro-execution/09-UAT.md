---
status: partial
phase: 09-parallel-macro-execution
source: [09-VERIFICATION.md]
started: 2026-07-21T20:15:00Z
updated: 2026-07-22T10:15:00Z
---

## Current Test

[testing complete]

## Tests

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

total: 2
passed: 0
issues: 1
pending: 0
skipped: 0
blocked: 1

## Gaps

- gap_id: G-09-1a
  truth: "AutoMux's performance returns to baseline (no lag) once all macros are disabled/stopped"
  status: failed
  reason: "User reported: hotkey-triggered global macro at 100ms interval caused laptop lag, and AutoMux remained slowed down even after the macro(s) were turned off"
  severity: major
  test: 1
  artifacts: []
  missing: []

- gap_id: G-09-1b
  truth: "User can delete a macro from within the app"
  status: failed
  reason: "User reported: there is still not a way to delete macros"
  severity: major
  test: 1
  artifacts: []
  missing: []

- gap_id: G-09-1c
  truth: "A macro targeted to a specific process fires when that process is focused (matching the app's process-targeting feature)"
  status: failed
  reason: "User reported: global macro worked (despite lag), but a targetted macro did not seem to work"
  severity: major
  test: 1
  artifacts: []
  missing: []

## Deferred Follow-Ups

- test: 1
  idea: "UI/menus are confusing, unintuitive, and outdated — user explicitly deferred this to a later pass, not this phase or this release."
  deferred_at: 2026-07-22
