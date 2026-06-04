---
phase: 06-macos-tahoe-26-compatibility
plan: "01"
subsystem: platform/macos
tags: [macos, permissions, cgeventtap, tahoe-26, compatibility]
dependency_graph:
  requires: []
  provides: [check_accessibility_permissions-live-probe, Info.plist-NSAccessibilityUsageDescription]
  affects: [src-tauri/src/ipc/mod.rs:check_accessibility, AutoMux.app/Contents/Info.plist]
tech_stack:
  added: []
  patterns: [CGEventTap-probe-as-TCC-trust-signal, local-use-import-in-function]
key_files:
  created:
    - src-tauri/Info.plist
  modified:
    - src-tauri/src/platform/macos/mod.rs
decisions:
  - "Use CGEventTap::new(ListenOnly) as the live TCC probe rather than AXIsProcessTrusted() to bypass stale per-process cache on macOS Sequoia and Tahoe 26"
  - "Removed AXIsProcessTrusted() extern declaration entirely (was unused after fix) to avoid dead-code warning"
  - "Moved unsafe block inward to wrap only the prompt=true branch — the probe is safe Rust"
metrics:
  duration: "~15 minutes"
  completed: "2026-06-04T19:17:27Z"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 2
---

# Phase 6 Plan 1: CGEventTap Live Probe + NSAccessibilityUsageDescription Summary

**One-liner:** Live CGEventTap::new() probe replaces stale AXIsProcessTrusted() cache on the !prompt path, fixing false-negative permission detection on macOS Sequoia and Tahoe 26.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Replace AXIsProcessTrusted() with live CGEventTap probe | 403c8ba | src-tauri/src/platform/macos/mod.rs |
| 2 | Create src-tauri/Info.plist with NSAccessibilityUsageDescription | 9656597 | src-tauri/Info.plist |

## What Was Built

**Task 1 — Live TCC Probe (COMPAT-01, COMPAT-02)**

`check_accessibility_permissions(false)` was calling `AXIsProcessTrusted()` which caches its result per-process and never re-reads TCC state after the initial load. On macOS Sequoia (15) and Tahoe (26) this cache is stale after the user grants Accessibility in System Settings, causing the function to return `false` indefinitely and preventing CGEventTap from being armed.

The fix replaces the single stale-cache line with a `CGEventTap::new()` probe using `ListenOnly` and a single event type (`MouseMoved`). The probe is:
- Scoped to the function body (dropped immediately after `is_ok()`)
- Never added to a CFRunLoop (cannot receive or inject events)
- Never setting `TAP_INITIALIZED` or `TAP_STARTING` (those remain owned exclusively by `observer.rs::initialize_tap()`)
- A safe Rust API call (no `unsafe` needed — `unsafe` block moved inward to wrap only the `prompt=true` CFDictionary branch)

**Task 2 — NSAccessibilityUsageDescription (COMPAT-03)**

Created `src-tauri/Info.plist` at the documented Tauri 2 location (`src-tauri/Info.plist` sibling of `tauri.conf.json`). Tauri CLI auto-merges this file into `AutoMux.app/Contents/Info.plist` at build time. The file contains only `NSAccessibilityUsageDescription` — no entitlement keys (those require signing, deferred to v3).

## Verification

All 7 plan verification checks passed:
1. `cargo build` exits 0 with no new errors or warnings
2. `grep "CGEventTap::new"` in mod.rs: 1 match (line 28)
3. `grep "AXIsProcessTrusted() !="` in mod.rs: 0 matches (stale-cache call removed)
4. `grep "TAP_INITIALIZED|TAP_STARTING"` in mod.rs: only in `@safety-officer:` comment, not code
5. `Info.plist` contains `NSAccessibilityUsageDescription`: count 1
6. `Info.plist` contains no `com.apple.security` keys: count 0
7. `plutil -lint src-tauri/Info.plist`: OK

## Deviations from Plan

None — plan executed exactly as written.

The plan noted that the AXIsProcessTrusted extern declaration could be removed (cosmetic, to avoid dead-code warning). This was done: the `fn AXIsProcessTrusted() -> u8;` line was removed from the `extern "C"` block since it became unused after the fix. This is consistent with the plan's note on Task 1 action item 1.

## Threat Coverage

| Threat ID | Status | Notes |
|-----------|--------|-------|
| T-06-01 | Mitigated | CGEventTap probe replaces stale AXIsProcessTrusted() cache |
| T-06-02 | Mitigated | Probe is ListenOnly, dropped immediately, not added to CFRunLoop |
| T-06-03 | Mitigated | mod.rs never reads or writes TAP_INITIALIZED / TAP_STARTING |
| T-06-SC | Accepted | No new packages installed in this phase |

## Known Stubs

None. Both changes are fully wired: the probe is called on every `check_accessibility(false)` IPC invocation via `src-tauri/src/ipc/mod.rs:check_accessibility`, and `Info.plist` is automatically merged by Tauri CLI at build time.

## Next Plan

06-02 (Wave 2): Manual Tahoe 26 verification checkpoint — validate that `check_accessibility_permissions(false)` returns true on a real Tahoe 26 system after granting Accessibility in System Settings, and confirm macros fire correctly.

## Self-Check: PASSED

- `src-tauri/src/platform/macos/mod.rs` — exists and modified (confirmed by grep)
- `src-tauri/Info.plist` — exists (confirmed by ls + plutil lint)
- commit 403c8ba — exists (`git log` confirmed)
- commit 9656597 — exists (`git log` confirmed)
