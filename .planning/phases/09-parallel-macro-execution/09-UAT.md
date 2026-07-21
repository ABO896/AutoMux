---
status: testing
phase: 09-parallel-macro-execution
source: [09-VERIFICATION.md]
started: 2026-07-21T20:15:00Z
updated: 2026-07-21T20:15:00Z
---

## Current Test

number: 1
name: macOS device tests (T9.1-T9.5)
expected: |
  Run T9.1-T9.5 on a real macOS host per 09-VERIFICATION.md Section 5 (concurrent firing,
  stop-one-keeps-other, same-input concurrent + conflict warning, held-not-firing indicator,
  Hold-under-load). All tests pass; both macro cards show independent pulsing firing dots;
  stopping one does not affect the other; same-input pair both fire while the Phase 8 conflict
  warning also displays; a Hold-mode macro shows the static held dot and genuinely holds the
  input under load.
awaiting: user response

## Tests

### 1. macOS device tests (T9.1-T9.5)
expected: All tests pass; both macro cards show independent pulsing firing dots; stopping one does not affect the other; same-input pair both fire while the Phase 8 conflict warning also displays; a Hold-mode macro shows the static held dot and genuinely holds the input under load.
result: [pending]

### 2. Windows device tests (6.1-6.3)
expected: All 3 tests pass via SendInput injection and the Win32 hook observer — same concurrent-firing and independent-stop behavior as macOS.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
