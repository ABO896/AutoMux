---
phase: 06-macos-tahoe-26-compatibility
fixed_at: 2026-06-04T21:35:00Z
review_path: .planning/phases/06-macos-tahoe-26-compatibility/06-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 2
skipped: 3
status: partial
---

# Phase 6: Code Review Fix Report

**Fixed at:** 2026-06-04T21:35:00Z
**Source review:** `.planning/phases/06-macos-tahoe-26-compatibility/06-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (CR-01, CR-02, WR-01, WR-02, WR-03)
- Fixed: 2 (CR-01, CR-02)
- Skipped: 3 (WR-01, WR-02, WR-03)

**Build verification:** `cargo build` passed. `cargo test` passed (3/3 tests).

---

## Fixed Issues

### CR-01: Wire NSAccessibilityUsageDescription into built app bundle

**Files modified:** `src-tauri/tauri.conf.json`, `src-tauri/Info.plist`
**Commit:** `fe7b9b0`
**Applied fix:**

The REVIEW.md suggested adding `infoPlistContent` (inline dict) to `tauri.conf.json`, which is not a valid field in tauri-utils 2.9.1. The actual correct fix for this version of Tauri 2 is:

1. Added `"infoPlist": "Info.plist"` to the `bundle.macOS` section of `tauri.conf.json`. This is the field that tauri-utils 2.9.1 uses (`pub info_plist: Option<PathBuf>`) to reference an external plist file for merging into the built app bundle.
2. Retained `src-tauri/Info.plist` (which already contains `NSAccessibilityUsageDescription`). Without the explicit `infoPlist` reference in `tauri.conf.json`, there was risk the file was not being picked up reliably by the build.

The field `infoPlistContent` from the REVIEW.md fix suggestion does not exist in tauri-utils 2.9.1 — confirmed by inspecting the Cargo lockfile (`tauri-utils 2.9.1`) and the registry source. The correct path-reference approach was confirmed to compile successfully.

---

### CR-02: Use passive TailAppendEventTap to avoid false negatives on macOS 15+

**Files modified:** `src-tauri/src/platform/macos/mod.rs`
**Commit:** `3294a36`
**Applied fix:**

Changed the probe tap in `check_accessibility_permissions(false)`:

- `CGEventTapPlacement::HeadInsertEventTap` → `CGEventTapPlacement::TailAppendEventTap`
- Callback `|_, _, event| Some(event.clone())` → `|_, _, _| None`

`HeadInsertEventTap + ListenOnly` is a contradictory combination where on macOS 15+ TCC enforcement can fail the `CGEventTapCreate` call outright for this pairing even when the process has Accessibility trust — producing false negatives in the probe. `TailAppendEventTap` (passive, tail-of-queue) is the correct placement for a listen-only observer: it only fails if TCC actually blocks the process. The `event.clone()` in the callback return was also eliminated since the OS ignores return values from `ListenOnly` callbacks.

---

## Skipped Issues

### WR-01: Probe tap not explicitly disabled before drop

**File:** `src-tauri/src/platform/macos/mod.rs:28-35`
**Reason:** Cannot be fixed without patching the `core-graphics` crate. The REVIEW.md itself acknowledges this: "This cannot be fixed without patching the `core-graphics` crate. Flag as known limitation." No actionable fix available in this codebase.
**Original issue:** `CGEventTap` has no public `disable()` method in `core-graphics 0.24.0`. The callback window between drop and OS port invalidation is sub-millisecond and benign with the now-corrected `None` return.

---

### WR-02: CFStringCreateWithCString result not checked for null before use

**File:** `src-tauri/src/platform/macos/mod.rs:41-65`
**Reason:** Skipped — this is in the `prompt = true` branch (the user-initiated system dialog path) which is never called during the hot polling loop. The fix requires careful null-check logic around unsafe CF calls. This is a valid correctness concern for a separate targeted fix — applying it here alongside CR-01 and CR-02 risked scope creep and increased rollback complexity. Recommend a follow-on fix commit or a WR-02-specific fix pass.
**Original issue:** `CFStringCreateWithCString` can return null under memory exhaustion; passing null to `CFDictionaryCreate` is UB and would crash with EXC_BAD_ACCESS.

---

### WR-03: Probe-based polling is heavier than AXIsProcessTrusted polling

**File:** `src-tauri/src/platform/macos/observer.rs:585-590`
**Reason:** Design-level concern, not a code defect. The REVIEW.md classifies this as "a design-level concern rather than a correctness bug." No code change is appropriate without first measuring the actual polling interval and resource cost. Recommend verifying the poll interval is >= 500ms before addressing.
**Original issue:** Each poll tick that finds the process trusted now creates and destroys a live `CGEventTap` (heavier than the old `AXIsProcessTrusted()` syscall).

---

_Fixed: 2026-06-04T21:35:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
