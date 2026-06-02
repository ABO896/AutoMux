---
status: partial
phase: 05-macos-permissions-reliability
source: [05-VERIFICATION.md]
started: 2026-06-02T04:00:00Z
updated: 2026-06-02T04:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. RELY-06 runtime — Grant via System Settings
expected: Macros activate within 3s of granting Accessibility in System Settings without restarting the app
result: [pending]

### 2. PERM-01 pending flow — Click Grant Access
expected: "Pending…" amber indicator appears immediately after clicking; "Granted" shows within 3s of approving the OS dialog
result: [pending]

### 3. PERM-01 fallback — Dismiss OS dialog
expected: "Pending…" clears after 30 seconds and UI reverts to "Denied" + Grant Access button
result: [pending]

### 4. BUILD-02 build output — macOS clean build
expected: `cargo build` on macOS produces no lines matching block/cocoa/deprecation
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
