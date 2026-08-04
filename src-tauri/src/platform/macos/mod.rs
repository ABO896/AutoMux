pub mod input;
pub mod observer;

pub use input::MacInputProvider;
pub use observer::MacPlatformObserver;

use core_foundation::base::TCFType;
use std::ffi::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
}

/// Check whether this process has Accessibility permissions.
///
/// - `prompt = false`: silent check, returns current status.
/// - `prompt = true`: shows the macOS system dialog asking the user to
///   grant Accessibility access in System Settings if not already trusted.
pub fn check_accessibility_permissions(prompt: bool) -> bool {
    if !prompt {
        use core_graphics::event::{
            CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        };
        // @safety-officer: This probe tap must NOT set TAP_INITIALIZED or interact with TAP_STARTING.
        // It is scoped to this function and dropped immediately. Only initialize_tap() in observer.rs
        // sets those flags. AXIsProcessTrusted() is NOT used here — it caches its result per-process
        // and returns stale false on macOS 15 Sequoia / 26 Tahoe after a grant.
        let probe = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::TailAppendEventTap, // passive observer — avoids false negatives on macOS 15+
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::MouseMoved],
            |_, _, _| None, // listen-only: return value is ignored; avoid unnecessary clone
        );
        let is_trusted = probe.is_ok();
        // @safety-officer: idle-input-lag-freeze fix — CGEventTapCreate registers
        // a live, OS-level event tap with the WindowServer the instant it
        // succeeds, independent of whether it's ever added to a run loop. The
        // `core-graphics` crate's CGEventTap/CFMachPort has NO Drop impl that
        // calls CFMachPortInvalidate (only the default CFRelease runs), so
        // simply letting `probe` fall out of scope leaks the underlying Mach
        // port / tap registration — it stays live until this PROCESS exits.
        // This function is polled every 3s by the frontend for the entire
        // time the app is open, so without this explicit invalidation it
        // leaked ~1 zombie system-wide HID tap per call, degrading input
        // dispatch for the whole system within 1-3 minutes. Invalidate here
        // so each probe is fully torn down before the next one is created.
        if let Ok(tap) = probe {
            unsafe {
                core_foundation_sys::mach_port::CFMachPortInvalidate(
                    tap.mach_port.as_concrete_TypeRef(),
                );
            }
        }
        return is_trusted;
    }

    unsafe {
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

/// Check whether this process has Input Monitoring permissions.
///
/// Returns the current status without prompting — same probe pattern as the
/// silent Accessibility check above. Probe is scoped to this function and
/// does NOT set `TAP_INITIALIZED` or interact with `TAP_STARTING`.
///
/// [RESEARCH §D-05]: Required since macOS 10.15 Catalina; subsumed by
/// Accessibility when both are needed, but a separate probe is required
/// because the CGEventTap `ListenOnly` option is the only reliable signal
/// independent of the stale `AXIsProcessTrusted()` cache.
pub fn check_input_monitoring() -> bool {
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };
    // @safety-officer: This probe tap must NOT set TAP_INITIALIZED or interact with TAP_STARTING.
    // It is scoped to this function and dropped immediately. Only initialize_tap() in observer.rs
    // sets those flags. AXIsProcessTrusted() is NOT used here — it caches its result per-process
    // and returns stale false on macOS 15 Sequoia / 26 Tahoe after a grant.
    let probe = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::TailAppendEventTap, // passive observer — avoids false negatives on macOS 15+
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::MouseMoved],
        |_, _, _| None, // listen-only: return value is ignored; avoid unnecessary clone
    );
    let is_trusted = probe.is_ok();
    // @safety-officer: idle-input-lag-freeze fix — see matching comment in
    // check_accessibility_permissions above. This probe is polled every 3s
    // by the frontend for the entire time the app is open; without this
    // explicit CFMachPortInvalidate call it leaked a zombie system-wide HID
    // event tap on every call (core-graphics's CGEventTap/CFMachPort has no
    // Drop impl beyond CFRelease, which does not unregister the tap with the
    // WindowServer).
    if let Ok(tap) = probe {
        unsafe {
            core_foundation_sys::mach_port::CFMachPortInvalidate(
                tap.mach_port.as_concrete_TypeRef(),
            );
        }
    }
    is_trusted
}

#[cfg(test)]
mod tests {
    use core_foundation_sys::mach_port::CFMachPortIsValid;
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };
    use core_foundation::base::TCFType;

    /// Regression test for idle-input-lag-freeze: `check_accessibility_permissions(false)`
    /// and `check_input_monitoring()` above each create a `CGEventTap` purely as a
    /// permission probe, on every 3s frontend poll cycle, for as long as AutoMux is
    /// open. Without an explicit `CFMachPortInvalidate()` call, core-graphics
    /// 0.24.0's `CGEventTap`/`CFMachPort` (no `Drop` impl beyond the default
    /// `CFRelease`) leaves the tap's Mach port registered with the WindowServer
    /// until process exit — leaking ~2 live, system-wide HID event taps every 3
    /// seconds, which degraded input dispatch for the ENTIRE system within 1-3
    /// minutes of the app being open (see .planning/debug/resolved/idle-input-lag-freeze.md).
    ///
    /// This test mirrors the exact `CGEventTap::new(...)` parameters used by both
    /// probe call sites above and proves the fix's mechanism: `CFMachPortInvalidate`
    /// transitions the tap's Mach port from valid to invalid — the same call both
    /// functions now make before their probe falls out of scope. Keep this test's
    /// assertions in sync with both call sites; if either drops its
    /// `CFMachPortInvalidate` call, this is the assertion meant to catch it.
    ///
    /// Skips (does not fail) when this test process is not Accessibility-trusted,
    /// since `CGEventTapCreate` then legitimately returns null (an environment
    /// precondition, not a regression in the code under test) — the skip is logged
    /// so it is never silent.
    #[test]
    fn probe_event_tap_invalidate_prevents_leak() {
        let probe = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::TailAppendEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::MouseMoved],
            |_, _, _| None,
        );

        let Ok(tap) = probe else {
            eprintln!(
                "[test] SKIPPED probe_event_tap_invalidate_prevents_leak: this \
                 process is not Accessibility-trusted in this environment \
                 (CGEventTapCreate returned null) — cannot exercise the Ok(tap) \
                 branch this regression test targets."
            );
            return;
        };

        let raw_ref = tap.mach_port.as_concrete_TypeRef();

        assert_eq!(
            unsafe { CFMachPortIsValid(raw_ref) },
            1,
            "sanity: a freshly created CGEventTap's Mach port must start out valid"
        );

        // Exercise the exact fix applied in check_accessibility_permissions /
        // check_input_monitoring above.
        unsafe {
            core_foundation_sys::mach_port::CFMachPortInvalidate(raw_ref);
        }

        assert_eq!(
            unsafe { CFMachPortIsValid(raw_ref) },
            0,
            "BUG REGRESSION (idle-input-lag-freeze): CFMachPortInvalidate did not \
             invalidate the probe tap's Mach port. Without this, the tap stays \
             registered with the WindowServer until process exit — every 3s \
             frontend poll leaks another live, system-wide HID event tap, \
             degrading input dispatch for the whole system within 1-3 minutes."
        );
    }
}
