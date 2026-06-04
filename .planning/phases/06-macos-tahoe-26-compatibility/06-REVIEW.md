---
phase: 06-macos-tahoe-26-compatibility
reviewed: 2026-06-04T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - src-tauri/Info.plist
  - src-tauri/src/platform/macos/mod.rs
findings:
  critical: 2
  warning: 3
  info: 1
  total: 6
status: issues_found
---

# Phase 6: Code Review Report

**Reviewed:** 2026-06-04  
**Depth:** standard  
**Files Reviewed:** 2  
**Status:** issues_found

## Summary

This phase replaces the stale `AXIsProcessTrusted()` call with a live `CGEventTap::new()` probe for permission detection, and adds `Info.plist` with `NSAccessibilityUsageDescription` so AutoMux appears in System Settings → Privacy & Security → Accessibility.

The probe approach is architecturally sound: constructing a `ListenOnly` CGEventTap at `HID` placement is the correct way to bypass the per-process TCC result cache that causes `AXIsProcessTrusted` to return stale `false` after a grant on Sequoia / Tahoe 26. The tap is locally scoped, never added to a RunLoop, and the `@safety-officer` comment correctly documents the non-interaction with `TAP_INITIALIZED` / `TAP_STARTING`.

However, two critical defects and three warnings were found, detailed below.

---

## Critical Issues

### CR-01: `Info.plist` is never wired into the Tauri 2 macOS bundle — the key is silently dropped

**File:** `src-tauri/Info.plist:1`  
**Issue:** Tauri 2 does **not** automatically merge a freestanding `src-tauri/Info.plist` into the built `.app` bundle. The Tauri 2 macOS bundle config accepts custom Info.plist keys only through the `bundle.macOS.infoPlistContent` field in `tauri.conf.json` (a TOML/JSON inline dict) — or, for Tauri's Xcode-backed target, through the Xcode project. The file at `src-tauri/Info.plist` exists on disk but is not referenced from `tauri.conf.json` or from any build script. Inspecting `tauri.conf.json` confirms `bundle.macOS` contains no `infoPlistContent` key. As shipped, the built `.app`'s `Info.plist` will contain no `NSAccessibilityUsageDescription` entry, so:

1. The app **will not appear** in System Settings → Privacy & Security → Accessibility on a clean install.
2. On Tahoe 26 (where TCC enforces the description key more strictly), the user cannot grant the permission at all through the normal UI flow.

The file exists but has zero effect.

**Fix:** Add the key directly into `tauri.conf.json` under `bundle.macOS.infoPlistContent`:

```json
"macOS": {
  "minimumSystemVersion": "12.0",
  "frameworks": [],
  "entitlements": null,
  "signingIdentity": null,
  "infoPlistContent": {
    "NSAccessibilityUsageDescription": "AutoMux requires Accessibility access to monitor and inject keyboard and mouse events for macro automation."
  }
}
```

Once this is present, the standalone `src-tauri/Info.plist` file should be deleted (or kept only as documentation), since only the `tauri.conf.json` version is consumed by the build.

---

### CR-02: Probe tap callback captures `event` by cloning it, but the callback fires with `None` return on `ListenOnly` taps — behavior is undefined on some OS versions

**File:** `src-tauri/src/platform/macos/mod.rs:28-34`  
**Issue:** The probe tap is created with `CGEventTapOptions::ListenOnly` but the callback returns `Some(event.clone())`:

```rust
let probe = CGEventTap::new(
    CGEventTapLocation::HID,
    CGEventTapPlacement::HeadInsertEventTap,
    CGEventTapOptions::ListenOnly,
    vec![CGEventType::MouseMoved],
    |_, _, event| Some(event.clone()),  // ← returning Some on a ListenOnly tap
);
```

`HeadInsertEventTap` + `ListenOnly` is contradictory: `HeadInsertEventTap` requests head-of-queue insertion (an active-filter tap placement), but `ListenOnly` removes the ability to modify or suppress events. Apple's `CGEventTapCreate` documentation states that returning a non-null event from a `ListenOnly` callback has no effect — the OS ignores the return value. The combination is not dangerous in itself, but on macOS 15+ TCC enforcement has been observed to **fail the `CGEventTapCreate` call outright** for `HeadInsertEventTap + ListenOnly` combinations when the process does not yet have Accessibility trust, which is precisely the pre-grant scenario this probe is meant to detect. If the call fails for this reason rather than for lack of trust, the probe produces a false negative (reports "not trusted" when the OS would grant a `TailAppendEventTap + ListenOnly` tap).

The correct placement for a passive probe is `CGEventTapPlacement::TailAppendEventTap` with `CGEventTapOptions::ListenOnly`, and the callback should return `None` (returning the event on a listen-only tap is a no-op and wastes a clone):

**Fix:**
```rust
let probe = CGEventTap::new(
    CGEventTapLocation::HID,
    CGEventTapPlacement::TailAppendEventTap,   // passive, not head-insert
    CGEventTapOptions::ListenOnly,
    vec![CGEventType::MouseMoved],
    |_, _, _| None,                            // listen-only: return value is ignored anyway
);
return probe.is_ok();
```

---

## Warnings

### WR-01: The probe tap is dropped immediately but never explicitly disabled — the OS may hold the mach port open briefly

**File:** `src-tauri/src/platform/macos/mod.rs:28-35`  
**Issue:** `CGEventTap::new()` returns a struct that wraps a `CFMachPort`. When the `probe` binding is dropped at the end of the `if !prompt` block, the `CFMachPort` is released. The OS is expected to clean up the tap when the port's refcount reaches zero. However, under busy system conditions (many events in flight) the tap may fire its callback one final time after the `probe` has been dropped but before the run-loop processes the port invalidation. The `core-graphics` crate's `CGEventTap` does not call `CGEventTapEnable(port, false)` before releasing the port in its `Drop` impl. This is a best-effort concern: the window is tiny (sub-millisecond) and the callback (`|_, _, event| Some(event.clone())`) is harmless. But it is worth being aware of in an adversarial review. Adding an explicit disable before drop would be belt-and-suspenders safety:

```rust
// After is_ok() check:
// if let Ok(ref tap) = probe { tap.enable(); /* then immediately */ }
// — but CGEventTap has no public disable() method in core-graphics 0.24.0.
// Dropping is the only available mechanism; this is acceptable as-is.
```

This cannot be fixed without patching the `core-graphics` crate. Flag as known limitation.

---

### WR-02: `AXIsProcessTrustedWithOptions` is called with `prompt = true` path but the `key` CFString is never checked for null before use

**File:** `src-tauri/src/platform/macos/mod.rs:41-65`  
**Issue:** `CFStringCreateWithCString` can return null if memory is exhausted. The code casts the result directly to `*const c_void` and passes it into `CFDictionaryCreate` without a null check:

```rust
let key = core_foundation_sys::string::CFStringCreateWithCString(
    core_foundation_sys::base::kCFAllocatorDefault,
    key_cstr.as_ptr(),
    core_foundation_sys::string::kCFStringEncodingUTF8,
);

// key could be null here ↓
let keys = [key as *const c_void];
```

Passing a null key pointer to `CFDictionaryCreate` is undefined behavior and will crash the process with an EXC_BAD_ACCESS in the CF internals. Similarly, `CFDictionaryCreate` itself could return null under memory pressure, and `CFRelease(dict as *const c_void)` would then pass a null pointer — also undefined behavior (`CFRelease` requires a non-null argument).

**Fix:**
```rust
let key = core_foundation_sys::string::CFStringCreateWithCString(
    core_foundation_sys::base::kCFAllocatorDefault,
    key_cstr.as_ptr(),
    core_foundation_sys::string::kCFStringEncodingUTF8,
);
if key.is_null() {
    return false; // cannot build options dict; treat as no-prompt fallback
}

// ... build dict ...

let dict = core_foundation_sys::dictionary::CFDictionaryCreate( /* ... */ );
if dict.is_null() {
    core_foundation_sys::base::CFRelease(key as *const c_void);
    return false;
}

let result = AXIsProcessTrustedWithOptions(dict as *const c_void);
core_foundation_sys::base::CFRelease(dict as *const c_void);
core_foundation_sys::base::CFRelease(key as *const c_void);
result != 0
```

---

### WR-03: `start_observing` uses the new `check_accessibility_permissions(false)` probe to gate `initialize_tap()`, but a permission grant between the probe and `initialize_tap()` can still be missed

**File:** `src-tauri/src/platform/macos/observer.rs:585-590` (cross-reference)  
**Issue:** The call sequence in `start_observing` is:

```rust
if super::check_accessibility_permissions(false) {
    let ok = initialize_tap();
    ...
}
```

If the probe `CGEventTap::new()` is created and succeeds (permission exists), the probe is dropped, and then `initialize_tap()` creates a second `CGEventTap`. Between the two calls the TCC database is not consulted again — this is fine. However, if the probe *fails* (permission not yet granted), `initialize_tap()` is never called, and the tap is never initialized for the session even if the user grants permission after `start_observing` returns. The existing poll-based permission check elsewhere in the codebase is supposed to catch this via periodic `check_accessibility_permissions(false)` polling and then call `initialize_tap()` again. This architecture is correct in principle, but the new probe introduces an observable asymmetry: the probe being called on every poll tick now creates a live `CGEventTap` and immediately destroys it on every poll cycle where the app is trusted. This is significantly heavier than the old `AXIsProcessTrusted()` syscall. Confirm that the polling interval is not shorter than ~500 ms, or throttle the probe frequency to avoid taxing the TCC daemon.

This is a design-level concern rather than a correctness bug, but it has a real resource cost that should be acknowledged.

---

## Info

### IN-01: `unwrap()` on `CString::new()` can panic on embedded null bytes — low risk given the static string

**File:** `src-tauri/src/platform/macos/mod.rs:40`  
**Issue:** `std::ffi::CString::new("AXTrustedCheckOptionPrompt").unwrap()` panics if the string contains an interior null byte. The literal does not contain one, so this cannot trigger in practice. However, project convention (from CLAUDE.md error handling guidance) is to avoid `unwrap()` in production paths. This is in the `prompt = true` branch (the system dialog path), which is user-initiated, so a panic here would crash the app instead of surfacing an error.

**Fix:** Use `expect()` with a message, or use the `c"AXTrustedCheckOptionPrompt"` C string literal available in Rust 1.77+ instead:

```rust
// Rust 1.77+ (zero-cost, statically verified):
let key_cstr = c"AXTrustedCheckOptionPrompt";
let key = core_foundation_sys::string::CFStringCreateWithCString(
    core_foundation_sys::base::kCFAllocatorDefault,
    key_cstr.as_ptr(),
    core_foundation_sys::string::kCFStringEncodingUTF8,
);
```

---

_Reviewed: 2026-06-04_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
