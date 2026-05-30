---
status: partial
phase: 01-reliability-safety
source: [01-VERIFICATION.md]
started: 2026-05-16T12:45:00Z
updated: 2026-05-16T12:45:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Accessibility grant detected without restart
expected: On macOS, open System Settings → Privacy & Security → Accessibility and grant permission to AutoMux while the app is running. Within ~3 seconds the permissions prompt in the UI clears and the engine becomes usable — no app restart required.
result: [pending]

### 2. 30-minute CGEventTap hotkey survival
expected: Start AutoMux on macOS, bind a hotkey, and leave the app running for at least 30 minutes. After 30 minutes the hotkey still fires (CGEventTap has not silently deactivated due to timeout or user-input disable events).
result: [pending]

### 3. Windows emergency-stop key release (Ctrl+Shift+Q)
expected: On Windows, start a macro with a sustained key/button hold active. Press Ctrl+Shift+Q. All held keys and mouse buttons receive release events before the process exits — no keys remain physically stuck after the app closes.
result: [pending]

### 4. Auto-save roundtrip
expected: Create or edit one or more macros (rename, enable/disable, change interval, add/remove steps). Close AutoMux without clicking any explicit Save button. Reopen the app. All changes are present exactly as they were when the app was closed.
result: [pending]

### 5. Engine-active state restore
expected: Toggle the engine off in the UI. Close and reopen AutoMux. The engine remains off — it does not silently reset to on (no one-way ratchet in profile restore).
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
