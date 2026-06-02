---
phase: 05-macos-permissions-reliability
plan: "01"
subsystem: platform/macos
tags: [macos, permissions, cgeventtap, build-quality, cargo]
dependency_graph:
  requires: []
  provides: [RELY-06-implementation, BUILD-02-fix]
  affects: [src-tauri/src/ipc/mod.rs, src-tauri/Cargo.toml]
tech_stack:
  added: []
  patterns: [idempotent-tap-init-via-atombool-cas, remove-unused-transitive-dep]
key_files:
  created: []
  modified:
    - src-tauri/src/ipc/mod.rs
    - src-tauri/Cargo.toml
    - src-tauri/Cargo.lock
decisions:
  - "Mirror request_accessibility tap-arming pattern into check_accessibility; no new mechanism needed — initialize_tap() idempotency via TAP_INITIALIZED AtomicBool CAS already handles the repeated-call case"
  - "Remove cocoa = 0.26.1 as the sole source of block v0.1.6 deprecation; block2 = 0.6.2 is retained as it is directly used in observer.rs"
metrics:
  duration: ~15m
  completed: "2026-06-02"
  tasks_completed: 2
  tasks_total: 2
---

# Phase 5 Plan 01: CGEventTap Post-Grant Arming + Cocoa Dep Removal Summary

**One-liner:** Armed CGEventTap in check_accessibility via initialize_tap() call (RELY-06) and eliminated block v0.1.6 deprecation by removing the unused cocoa = 0.26.1 dependency (BUILD-02).

## Tasks Completed

| Task | Name | Commit | Files Modified |
|------|------|--------|----------------|
| 1 | Arm CGEventTap in check_accessibility (RELY-06 / D-03) | 4595944 | src-tauri/src/ipc/mod.rs |
| 2 | Remove cocoa dependency (BUILD-02 / D-05) | aa947b3 | src-tauri/Cargo.toml, src-tauri/Cargo.lock |

## What Was Built

### Task 1: RELY-06 — Post-grant tap arming

`check_accessibility` in `src-tauri/src/ipc/mod.rs` previously called `AXIsProcessTrusted()` and returned without doing anything with the result. A user who granted accessibility directly in System Settings (without clicking the in-app "Request Access" button) would need to restart the app before macros became active.

The fix mirrors the existing `request_accessibility` pattern: capture the granted boolean, and if `true`, call `crate::platform::macos::observer::initialize_tap()`. The `initialize_tap()` function is idempotent — it uses a `TAP_INITIALIZED` AtomicBool CAS guard that makes repeated calls from the 3s frontend poll loop instant no-ops once the tap is already running.

The `#[cfg(not(target_os = "macos"))]` branch returning `Ok(true)` is preserved unchanged.

### Task 2: BUILD-02 — block v0.1.6 deprecation eliminated

`cocoa = "0.26.1"` was the sole direct dependency that pulled in `block v0.1.6`, which emits deprecation warnings on recent Rust toolchains. The cocoa crate was not used directly in any source file — it was a leftover from earlier development. Removing it from the `[target.'cfg(target_os = "macos")'.dependencies]` section of `Cargo.toml` eliminates the warning at the source. `block2 = "0.6.2"` is retained as it is used directly in `observer.rs`.

Post-removal `cargo build` output: only `Compiling block2 v0.6.2` appears (block2 being compiled as a fresh dep); no deprecation or error lines.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None — changes are internal Rust layer only. No new network endpoints, auth paths, or trust boundary crossings introduced. `initialize_tap()` call in `check_accessibility` is behind the existing `#[cfg(target_os = "macos")]` guard and is idempotent via the TAP_INITIALIZED CAS guard (T-05-01 and T-05-02 from the plan's threat model both remain accepted, no new surface).

## Self-Check: PASSED

- `src-tauri/src/ipc/mod.rs`: modified (contains initialize_tap() call in check_accessibility) — verified via edit
- `src-tauri/Cargo.toml`: modified (cocoa entry removed, block2 retained) — verified via grep
- Commit 4595944: exists
- Commit aa947b3: exists
- `cargo build` exit 0, no deprecation/error lines related to block, cocoa, or deprecat
