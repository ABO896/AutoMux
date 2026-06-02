# Changelog

All notable changes to this project will be documented in this file.

## [1.2.0] - 2026-06-02

### Added
- **Accessibility Permission State Machine (Phase 5)**: Three-branch permission UI — Granted / Pending / Denied — with a 3-second polling loop that transitions state accurately after the system dialog resolves, preventing the UI from showing "Pending…" indefinitely after a deny.

### Fixed
- **CGEventTap Double-Spawn Race (CR-01)**: Added `TAP_STARTING` in-flight `AtomicBool` guard to `initialize_tap()`, preventing a second CGEventTap thread from being spawned during the reset window between a failed spawn and `TAP_INITIALIZED` clearing.
- **CGEventTap Armed on Permission Grant (RELY-06)**: `check_accessibility` IPC handler now arms the CGEventTap immediately when Accessibility is granted, eliminating the need for an app restart.
- **Profile Name Collision (CR-03)**: Profile names are now sanitized before being stored in `ProfileData`, so two names that resolve to the same filename no longer silently overwrite each other; empty sanitized names are rejected.
- **macOS Detection Regression (WR-02)**: Replaced deprecated `navigator.platform` (returns empty string in some Tauri WebView builds) with `navigator.userAgent` for reliable platform detection; prevents `bind_hotkey` from being silently skipped on macOS.
- **Accessibility UI Stuck on "Pending" (CR-02)**: `accessibilityPending` flag is now cleared unconditionally on any definitive poll response, not only on grant, so the "Grant Access" button reappears immediately after a deny.
- **Post-Unmount State Updates (WR-03)**: `handleRequestAccess` is guarded by a `cancelled` flag set in `onCleanup`, preventing stale state updates after component unmount.
- **cocoa Dependency Removed (BUILD-02)**: Removed the `cocoa` crate to eliminate the deprecated `block v0.1.6` transitive dependency and unblock clean Rust builds.
- **CI Universal Binary Verification**: Release workflow now checks the raw universal binary at `release/automux` instead of the cleaned `.app` bundle path, which is removed by `tauri-action` before the verification step runs.

## [1.1.0] - 2026-05-15

### Added
- **Advanced Triggering (Phase 7)**: Introduced "Latched" triggers, allowing support for both "Pulse" and "Hold" modes for intricate macro handling.
- **Process Detection Parity (Phase 8)**: Deployed system-native, push-based target app detection on macOS using `NSWorkspaceDidActivateApplicationNotification`, mirroring the Windows `SetWinEventHook` mechanism.

### Changed
- **$O(1)$ Hotkeys Lookups**: Extracted the trigger hotkey evaluation hot path into an $O(1)$ HashMap structure, removing mutex locks and reducing latency.
- **Zero Polling Reached**: Completely eliminated interval-based polling routines for application state. Engine stays fully idle at 0% CPU until pushed a context change notification.
- **Memory Safety Polished**: Audited and confirmed safe un-registration of `objc2` pointers preventing any memory leaks on macOS over extended uptimes.
