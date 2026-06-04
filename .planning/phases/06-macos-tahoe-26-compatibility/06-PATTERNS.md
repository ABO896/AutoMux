# Phase 6: macOS Tahoe 26 Compatibility - Pattern Map

**Mapped:** 2026-06-04
**Files analyzed:** 2 modified + 1 new
**Analogs found:** 2 / 2 (1 new file has no codebase analog — patterns from RESEARCH.md)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src-tauri/src/platform/macos/mod.rs` | platform-utility | request-response | self (existing function being modified) | exact — modify in place |
| `src-tauri/Info.plist` | config | N/A | none — new file type for this project | no analog |

**Not in scope per RESEARCH.md findings:**
- `src-tauri/tauri.conf.json` — `entitlements` field stays `null` (entitlements require signing; D-05 superseded by Finding 1). No change needed.
- `src-tauri/src/ipc/mod.rs` — `check_accessibility` and `request_accessibility` handlers are correct as-is; they call `check_accessibility_permissions()` and `initialize_tap()` which remain structurally unchanged.
- `src-tauri/src/platform/macos/observer.rs` — `initialize_tap()` is correct and must NOT be modified. Phase 5 re-entrancy guards (`TAP_INITIALIZED`, `TAP_STARTING`) must be preserved intact.

---

## Pattern Assignments

### `src-tauri/src/platform/macos/mod.rs` (platform-utility, request-response)

**Analog:** Self — this is an in-place modification of the existing `check_accessibility_permissions()` function.

**Current full file** (`src-tauri/src/platform/macos/mod.rs` lines 1-55):

```rust
pub mod input;
pub mod observer;

pub use input::MacInputProvider;
pub use observer::MacPlatformObserver;

use std::ffi::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
}

/// Check whether this process has Accessibility permissions.
///
/// - `prompt = false`: silent check, returns current status.
/// - `prompt = true`: shows the macOS system dialog asking the user to
///   grant Accessibility access in System Settings if not already trusted.
pub fn check_accessibility_permissions(prompt: bool) -> bool {
    unsafe {
        if !prompt {
            return AXIsProcessTrusted() != 0;
        }

        // Build CFDictionary { "AXTrustedCheckOptionPrompt": kCFBooleanTrue }
        let key_cstr = std::ffi::CString::new("AXTrustedCheckOptionPrompt").unwrap();
        let key = core_foundation_sys::string::CFStringCreateWithCString(
            core_foundation_sys::base::kCFAllocatorDefault,
            key_cstr.as_ptr(),
            core_foundation_sys::string::kCFStringEncodingUTF8,
        );

        let value = core_foundation_sys::number::kCFBooleanTrue;

        let keys = [key as *const c_void];
        let values = [value as *const c_void];

        let dict = core_foundation_sys::dictionary::CFDictionaryCreate(
            core_foundation_sys::base::kCFAllocatorDefault,
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &core_foundation_sys::dictionary::kCFTypeDictionaryKeyCallBacks,
            &core_foundation_sys::dictionary::kCFTypeDictionaryValueCallBacks,
        );

        let result = AXIsProcessTrustedWithOptions(dict as *const c_void);

        core_foundation_sys::base::CFRelease(dict as *const c_void);
        core_foundation_sys::base::CFRelease(key as *const c_void);

        result != 0
    }
}
```

**The single change point** — replace line 23 (`return AXIsProcessTrusted() != 0;`) with a live CGEventTap probe:

```rust
// BEFORE (line 22-24 — the stale-cache path to replace):
if !prompt {
    return AXIsProcessTrusted() != 0;
}

// AFTER — live TCC probe via CGEventTap::new():
if !prompt {
    // AXIsProcessTrusted() caches its result per-process and does NOT reflect
    // live TCC state after the user grants Accessibility in System Settings.
    // On macOS 15 Sequoia and 26 Tahoe, this cache is never invalidated during
    // the process lifetime. CGEventTap::new() consults the live TCC database
    // on every call — a successful creation proves Accessibility is live-granted.
    //
    // @safety-officer: This probe tap must NOT set TAP_INITIALIZED or interact
    // with the TAP_STARTING guard. It is scoped entirely to this function and
    // dropped immediately. Only initialize_tap() in observer.rs sets those flags.
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };
    let probe = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::MouseMoved],
        |_, _, event| Some(event.clone()),
    );
    // probe drops here — it is a check only; initialize_tap() creates the real tap.
    return probe.is_ok();
}
```

**Imports pattern to copy from** `observer.rs` lines 5-8 (same crate, same import style):

```rust
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
```

The probe in `mod.rs` uses the same import path; `EventField` is not needed for the probe so it can be omitted from the local `use` statement.

**CGEventTap::new() call pattern** to copy from `observer.rs` lines 250-268 (the real tap creation that the probe mimics in minimal form):

```rust
let tap_result = CGEventTap::new(
    CGEventTapLocation::HID,
    CGEventTapPlacement::HeadInsertEventTap,
    CGEventTapOptions::ListenOnly,
    vec![
        CGEventType::KeyDown,
        // ... (real tap uses many event types; probe needs only one)
        CGEventType::MouseMoved,
    ],
    |tap_proxy, event_type, event| {
        // ... callback body
    },
);
```

The probe uses `vec![CGEventType::MouseMoved]` (single type, minimal) and a trivial callback `|_, _, event| Some(event.clone())`. The real tap in `observer.rs` uses a full event list and a complex callback — the probe pattern is intentionally minimal.

**Re-entrancy guard pattern** — DO NOT replicate in `mod.rs`. This pattern lives ONLY in `observer.rs` lines 234-247 and must remain there:

```rust
// observer.rs — guard pattern (reference only — do NOT add to mod.rs):
static TAP_INITIALIZED: AtomicBool = AtomicBool::new(false);
static TAP_STARTING: AtomicBool = AtomicBool::new(false);

pub fn initialize_tap() -> bool {
    if TAP_INITIALIZED.load(Ordering::SeqCst) {
        return true;
    }
    if TAP_STARTING.swap(true, Ordering::SeqCst) {
        return false;
    }
    TAP_INITIALIZED.store(true, Ordering::SeqCst);
    // ... thread spawn follows
}
```

The probe in `check_accessibility_permissions(false)` must never touch `TAP_INITIALIZED` or `TAP_STARTING`.

**Error handling pattern** from `observer.rs` lines 458-490 (match on `tap_result`):

```rust
match tap_result {
    Ok(tap) => { /* live */ }
    Err(_) => {
        eprintln!("Failed to create CGEventTap. Make sure the app has Accessibility permissions.");
        TAP_INITIALIZED.store(false, Ordering::SeqCst);
        TAP_STARTING.store(false, Ordering::SeqCst);
    }
}
```

For the probe in `mod.rs`, the equivalent is just `probe.is_ok()` — no flags to reset, no eprintln needed (silent check path).

---

### `src-tauri/Info.plist` (config, N/A)

**Analog:** No existing `.plist` file in the codebase. Pattern comes from RESEARCH.md Finding 8 and Tauri 2 documentation.

**New file — full content to create:**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
    "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>NSAccessibilityUsageDescription</key>
    <string>AutoMux requires Accessibility access to monitor and inject keyboard and mouse events for macro automation.</string>
</dict>
</plist>
```

**Placement:** `src-tauri/Info.plist` (sibling of `tauri.conf.json`). Tauri CLI auto-merges this file with the generated `Info.plist` at build time. No `tauri.conf.json` change is required for this merge to occur.

**What this achieves:** The `NSAccessibilityUsageDescription` key causes macOS to display a human-readable description in System Settings → Privacy & Security → Accessibility. Its absence may prevent the app from appearing in the Accessibility list on some Tahoe 26 configurations, which would block the user from granting permission entirely.

**What this does NOT achieve:** For unsigned builds (`signingIdentity: null`), this key has no effect on TCC permission behaviour itself — it does not fix `AXIsProcessTrusted()`. The CGEventTap probe fix in `mod.rs` is the authoritative fix for COMPAT-01 and COMPAT-02.

---

## Shared Patterns

### CGEventTap creation style
**Source:** `src-tauri/src/platform/macos/observer.rs` lines 250-268
**Apply to:** The probe in `check_accessibility_permissions(false)` in `mod.rs`

The pattern is `CGEventTap::new(location, placement, options, event_types_vec, callback_closure)`. The probe uses `ListenOnly` options and a single-element `event_types` vec. Match the existing `observer.rs` style exactly — same crate, same enum variants, same argument order.

### `#[cfg(target_os = "macos")]` gating
**Source:** `src-tauri/src/ipc/mod.rs` lines 171-185 and 193-206

The `check_accessibility_permissions()` function in `mod.rs` is already macOS-only (the file is under `platform/macos/`). The probe import block inside the function uses a local `use` statement — no additional `#[cfg]` guard is needed at the function level because the module is already platform-gated at the module declaration level in `src-tauri/src/platform/mod.rs`.

### `unsafe` block scoping
**Source:** `src-tauri/src/platform/macos/mod.rs` lines 21-54

The existing `prompt=true` branch uses an `unsafe` block for the entire CFDictionary construction. The `prompt=false` probe does NOT need `unsafe` — `CGEventTap::new()` is a safe Rust API in `core-graphics 0.24.0`. The planner should restructure the function so the `unsafe` block wraps only the `prompt=true` branch, not the whole function body.

Current structure (wraps everything in unsafe):
```rust
pub fn check_accessibility_permissions(prompt: bool) -> bool {
    unsafe {
        if !prompt {
            return AXIsProcessTrusted() != 0;  // ← this is the only unsafe call in !prompt path
        }
        // ... CFDictionary unsafe calls for prompt=true ...
    }
}
```

After the fix, the `!prompt` early return is safe (no FFI), so the `unsafe` block can be deferred to only the `prompt=true` branch. This is cleaner but either structure compiles — preserve whichever minimizes diff.

### Error propagation style (IPC layer — unchanged)
**Source:** `src-tauri/src/ipc/mod.rs` lines 193-206

```rust
#[command]
pub async fn check_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let granted = crate::platform::macos::check_accessibility_permissions(false);
        if granted {
            crate::platform::macos::observer::initialize_tap();
        }
        Ok(granted)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}
```

This handler is UNCHANGED. The fix is purely in `check_accessibility_permissions(false)`. The IPC handler's structure — call the platform function, gate `initialize_tap()` on result, return `Ok(bool)` — is the established pattern and must not be altered.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src-tauri/Info.plist` | config | N/A | No plist files exist in the repo; Tauri generates Info.plist at build time — this is the developer-override file |

---

## Critical Constraints for Planner

1. **Do NOT modify `observer.rs`** — `initialize_tap()` and the `TAP_INITIALIZED`/`TAP_STARTING` guards from Phase 5 are correct. The Tahoe 26 fix does not touch this file.

2. **Do NOT add entitlements** — `tauri.conf.json`'s `entitlements: null` stays null. Entitlements require signing (deferred to v3). Creating an `Entitlements.plist` and setting the `entitlements` field is CONFIRMED to have no effect on TCC for unsigned apps.

3. **The probe tap must not set `TAP_INITIALIZED`** — it is a probe only. It creates a tap, checks if `Ok`, and drops it. `initialize_tap()` in `observer.rs` creates the real tap and is the only function that sets atomic flags.

4. **`AXIsProcessTrusted()` extern declaration** — the `extern "C"` block at lines 10-13 of `mod.rs` can remain (it is still used in the `prompt=true` path via `AXIsProcessTrustedWithOptions`). Only the `!prompt` branch changes from `AXIsProcessTrusted()` to the CGEventTap probe. The extern declaration for `AXIsProcessTrusted()` itself becomes unused after the fix; it can be removed to avoid a dead-code warning, but this is cosmetic.

5. **CGEventPost fallback is NOT a planned task** — RESEARCH.md Finding 6 confirms `CGEventPost` requires the same Accessibility permission as CGEventTap; it is not a distinct fallback strategy. D-08 is effectively superseded by Finding 6.

---

## Metadata

**Analog search scope:** `src-tauri/src/platform/macos/`
**Files read:** `mod.rs`, `observer.rs`, `ipc/mod.rs`, `tauri.conf.json`, `Cargo.toml`
**Pattern extraction date:** 2026-06-04
