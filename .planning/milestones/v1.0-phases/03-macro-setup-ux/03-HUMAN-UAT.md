---
status: partial
phase: 03-macro-setup-ux
source: [03-VERIFICATION.md]
started: 2026-05-30T15:00:00Z
updated: 2026-05-30T15:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. macOS Key Capture End-to-End
expected: Create a macro, press F5 in the key capture widget, verify "F5" badge appears, confirm the trigger key persists after app restart
result: [pending]

### 2. macOS Card-Edit Badge Update (CR-02)
expected: On an existing macro card, click the key badge to enter edit mode, press a new key, verify the old ghost trigger is gone and the new key badge persists after app restart
result: [pending]

### 3. "Set key…" Fallback on Key-less Card (WR-02)
expected: Create a macro without a trigger key, find the dashed "Set key…" placeholder on the card, click it, press a key, verify the badge updates with the new key
result: [pending]

### 4. Process Picker Populates on Focus
expected: Click into the target-app dropdown, verify the list shows running apps in display-name format (no raw bundle IDs as placeholder text)
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
