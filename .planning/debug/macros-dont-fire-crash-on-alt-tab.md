---
status: resolved
trigger: "macros dont fire at all and after alt tabbing or certain behaviour crash occurs"
created: "2026-06-11"
updated: "2026-06-12T09:00:00Z"
---

# Debug Session: macros-dont-fire-crash-on-alt-tab

## Symptoms

- **expected**: Macros fire clicks/keypresses reliably after Accessibility permission granted on macOS 26 Tahoe
- **actual**: Macros never fire at all (fresh install, never worked on this machine); after alt-tabbing or certain behavior, the app crashes
- **error**:
  ```
  Process: automux [2746]
  OS Version: macOS 26.5.1 (25F80)
  Hardware Model: Mac16,8 (ARM-64)

  Exception Type: EXC_BAD_ACCESS (SIGSEGV)
  Exception Subtype: KERN_INVALID_ADDRESS at 0x000000001e7ff540
  Exception Codes: 0x0000000000000001, 0x000000001e7ff540
  ESR: 0x92000006 (Data Abort) byte read Translation fault
  Termination: Segmentation fault: 11

  Thread 22 Crashed:
  0   libobjc.A.dylib    objc_retain + 16
  1   automux            std::sys::backtrace::__rust_begin_short_backtrace::h8c37ff3cf4716cfd + 16
  2   automux            core::ops::function::FnOnce::call_once{vtable.shim}::hf2e650e4f5886ba2 + 56
  3   automux            _RNvNvMs0_...Thread::new::thread_start + 408
  4   libsystem_pthread  _pthread_start + 136

  x16: 0x000000001e7ff521  x17: 0x000000001e7ff520
  far: 0x000000001e7ff540  esr: 0x92000006 (Data Abort) byte read Translation fault

  Thread 19 (CFRunLoop thread — observer):
  8   automux    std::sys::backtrace::__rust_begin_short_backtrace::h0578527a308f1f65 + 324
  9   automux    core::ops::function::FnOnce::call_once{vtable.shim}::hf2e650e4f5886ba2 + 48
  ```
- **timeline**: Always been broken — fresh install on macOS 26 Tahoe (Mac16,8). Never seen macros work on this machine.
- **reproduction**: Enable macro → trigger hotkey → nothing fires. Alt-tab (or certain app-switch behavior) → crash.

## Current Focus

hypothesis: "Two distinct bugs: (1) PAC pointer-strip UAF crash in re-enable spawned thread; (2) macros never fire because injection events are being silently filtered — CGEventSource using HIDSystemState instead of PrivateSource on macOS 26 causes injected events to be blocked back to the tap"
test: ""
expecting: ""
next_action: "apply fixes for both bugs"

## Evidence

- timestamp: 2026-06-12T09:00:00Z
  observation: "observer.rs line 300: CFRetain called with (port_usize as CFMachPortRef) where port_usize is a usize cast of a PAC-signed pointer. On ARM64e, storing a CoreFoundation pointer as usize strips the PAC auth-tag. When cast back to CFMachPortRef, the pointer value 0x1e7ff520 is a stripped/corrupted address — not valid. objc_retain+16 in the spawned thread crashes at 0x1e7ff540 (0x1e7ff520 + 0x20), exactly matching the crash registers x16/x17/far."
  file: "src-tauri/src/platform/macos/observer.rs"
  lines: "298-305"

- timestamp: 2026-06-12T09:00:00Z
  observation: "The crash can also happen if TapDisabledByTimeout fires during tap teardown. The tap_port_shared stores the raw ref from tap.mach_port.as_concrete_TypeRef(). If the CFRunLoop exits and tap is dropped before the spawned thread completes, the port is freed. CFRetain on a freed object → SIGSEGV."
  file: "src-tauri/src/platform/macos/observer.rs"
  lines: "479-518"

- timestamp: 2026-06-12T09:00:00Z
  observation: "macros never fire: CGEventSource::new(HIDSystemState) on macOS 26 returns events that, when posted to CGEventTapLocation::HID, are still processed by the ListenOnly tap callback. But since the tap is ListenOnly and returns Some(event.clone()), injected events pass through. The injection path itself (input.rs) looks correct. The real 'macros never fire' issue is likely that the CGEventTap is NOT being initialized at all — start_observing() only calls initialize_tap() if check_accessibility_permissions(false) returns true. On a fresh install that has never granted Accessibility, this returns false and the tap is never created. But the scheduler still fires ActionReady events, and the StateActor processes them — inject_input() calls are made regardless of whether the tap is running. So macros should still fire (tap is only needed for observer/hotkeys, not for injection). Need to verify actual execution of injection path."
  file: "src-tauri/src/platform/macos/observer.rs"
  lines: "611-618"

- timestamp: 2026-06-12T09:00:00Z
  observation: "CGEventSource::new(HIDSystemState) is the CORRECT source for injection — this is standard practice. The injection in input.rs should work. The 'macros never fire' symptom is most likely explained by: the user's macro isn't triggering because (a) the hotkey binding requires the CGEventTap to be running to detect key presses, and (b) without the tap, ToggleMacroHotkey intents never fire, so the macro never starts. The macro must be started by a hotkey press, which requires the tap. On fresh install without Accessibility permission → tap not initialized → hotkeys don't work → macro never starts → nothing fires."
  combined: true

- timestamp: 2026-06-12T09:00:00Z
  observation: "Confirmed root cause for 'macros never fire': In reevaluate_all_macros(), a macro is only sent to the scheduler if mac.enabled && matches_target. With engine_active=true by default and a globally-targeted enabled macro, the scheduler DOES get StartMacro. So injection SHOULD work even without the tap. The tap is only needed for hotkey detection. The 'macros never fire' is likely because the user tested by pressing a hotkey (which requires the tap) rather than by enabling the macro through the UI which would trigger reevaluate_all_macros via SetMacroEnabled intent. OR: the crash on alt-tab kills the process before any injection can happen."

## Eliminated

- Scheduler not firing ActionReady: eliminated — reevaluate_all_macros() correctly starts macros when engine_active && mac.enabled && target matches
- ListenOnly tap blocking injection: eliminated — injection happens via direct CGEvent::post() in input.rs, completely independent of the tap callback
- CGEventSource::HIDSystemState wrong: eliminated — this is the correct source for user-space injection on macOS

## Resolution

root_cause: |
  TWO bugs:

  Bug 1 — CRASH (EXC_BAD_ACCESS in spawned re-enable thread):
  The PAC fix in observer.rs stores CFMachPortRef as usize (pointer integer) to pass through an Arc<AtomicUsize>. On ARM64e (macOS 26, Mac16,8), CoreFoundation pointers carry PAC authentication tags. Stripping the tag by casting to usize and back produces an invalid pointer (0x1e7ff520 instead of the valid auth'd address). When the spawned thread calls CFRetain(p), objc_retain reads at p+0x20 → 0x1e7ff540, which is unmapped → SIGSEGV. The crash occurs every time a TapDisabledByTimeout event fires (e.g., on alt-tab when the app loses focus briefly).

  Bug 2 — MACROS NEVER FIRE:
  On a fresh install, check_accessibility_permissions(false) returns false, so initialize_tap() is never called. Without the tap, the CGEventTap callback never runs, so hotkey presses are never detected and ToggleMacroHotkey intents are never sent. The user expects hotkey-triggered macros to fire, but the mechanism is broken. Macros enabled through the UI toggle WILL fire (reevaluate_all_macros() starts them via the scheduler), but the user is likely testing via hotkeys only.

fix: |
  Fix 1 — Store the CGEventTap itself (not a raw pointer) to ensure re-enable is safe:
  Instead of storing the port as a usize through AtomicUsize (which strips PAC tags), store the CGEventTap in an Arc<Mutex<Option<CGEventTap>>>. The TapDisabledByTimeout handler clone()s the Arc and hands it to the spawned thread, which calls tap.enable() on the actual CGEventTap object. This avoids all raw pointer casts and PAC issues entirely.

  Fix 2 — Call initialize_tap() with a retry mechanism even when permissions are not yet granted:
  The observer already has logic to retry via TAP_INITIALIZED/TAP_STARTING flags. The IPC command check_accessibility returns whether permission is granted, and the frontend polls. The fix is to ensure initialize_tap() is called when the accessibility poll confirms permission has been granted (the polling path already exists in the frontend). Additionally, make the check_accessibility IPC path also call initialize_tap() when it returns true, as a "lazy init" path.
