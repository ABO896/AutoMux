# Changelog

All notable changes to this project will be documented in this file.

## [1.1.0] - 2026-05-15

### Added
- **Advanced Triggering (Phase 7)**: Introduced "Latched" triggers, allowing support for both "Pulse" and "Hold" modes for intricate macro handling.
- **Process Detection Parity (Phase 8)**: Deployed system-native, push-based target app detection on macOS using `NSWorkspaceDidActivateApplicationNotification`, mirroring the Windows `SetWinEventHook` mechanism.

### Changed
- **$O(1)$ Hotkeys Lookups**: Extracted the trigger hotkey evaluation hot path into an $O(1)$ HashMap structure, removing mutex locks and reducing latency.
- **Zero Polling Reached**: Completely eliminated interval-based polling routines for application state. Engine stays fully idle at 0% CPU until pushed a context change notification.
- **Memory Safety Polished**: Audited and confirmed safe un-registration of `objc2` pointers preventing any memory leaks on macOS over extended uptimes.
