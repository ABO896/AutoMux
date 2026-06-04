pub mod input;
pub mod observer;

pub use input::MacInputProvider;
pub use observer::MacPlatformObserver;

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
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::MouseMoved],
            |_, _, event| Some(event.clone()),
        );
        return probe.is_ok();
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
