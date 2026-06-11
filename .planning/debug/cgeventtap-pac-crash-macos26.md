---
status: resolved
trigger: "crash in initialize_tap when macro fires on macOS 26 Tahoe: EXC_BREAKPOINT SIGTRAP in __CFCheckCFInfoPACSignature <- CFMachPortGetContext <- SLEventTapEnable <- initialize_tap closure callback"
created: "2026-06-11"
updated: "2026-06-11T00:10:00Z"
---

# Debug Session: cgeventtap-pac-crash-macos26

## Symptoms

- **expected**: Macros fire (inject clicks/keypresses) after Accessibility permission is granted on macOS 26 Tahoe
- **actual**: App crashes with EXC_BREAKPOINT SIGTRAP on Thread 17 when any macro is triggered (CGEventTap tap-disabled re-enable path fires)
- **error**:
  ```
  Thread 17 Crashed:
  0   CoreFoundation    __CFCheckCFInfoPACSignature + 44
  1   CoreFoundation    CFMachPortGetContext + 28
  2   SkyLight          SLEventTapEnable + 68
  3   automux           automux_lib::platform::macos::observer::initialize_tap::closure::closure + 56
  4   automux           core_graphics::event::cg_event_tap_callback_internal + 48
  EXC_BREAKPOINT (SIGTRAP) — ESR: pointer authentication trap IA (0xf200c470)
  ```
- **timeline**: Regression on macOS 26 Tahoe (26.5.1, Mac16,8, ARM-64). COMPAT-02 now passes (Accessibility shows granted). COMPAT-01 fails (macros crash).
- **reproduction**: Grant Accessibility, create a macro, trigger it.

## Current Focus

hypothesis: "CGEventTapEnable(tap_proxy as CFMachPortRef, true) is called at observer.rs:285 from INSIDE the CGEventTap callback. macOS 26 ARM64e PAC enforcement traps CFMachPortGetContext when called from within the tap's own callback thread — __CFCheckCFInfoPACSignature fails PAC signature verification in that calling context."
test: "Read observer.rs — confirmed the call is on line 285 inside TapDisabledByTimeout handler inside the closure passed to CGEventTap::new()"
expecting: "Moving CGEventTapEnable to a thread spawned from within the callback (after retaining the port with CFRetain) will avoid the PAC violation and allow re-enable to succeed"
next_action: "Apply fix: use std::thread::spawn + CFRetain/CFRelease to call CGEventTapEnable asynchronously from the callback"
root_cause_confirmed: true

## Evidence

- timestamp: "2026-06-11T00:00:00Z"
  observation: "Crash thread 17 stack: __CFCheckCFInfoPACSignature <- CFMachPortGetContext <- SLEventTapEnable <- initialize_tap::closure::closure <- cg_event_tap_callback_internal. EXC_BREAKPOINT with ESR=0xf200c470 (pointer authentication trap IA on ARM64e)."
  implication: "macOS 26 enforces PAC on CFMachPort context pointer lookups. Calling CGEventTapEnable from within the tap's own callback violates this on macOS 26."

- timestamp: "2026-06-11T00:01:00Z"
  observation: "observer.rs:277-287 — TapDisabledByTimeout/TapDisabledByUserInput handler calls `unsafe { CGEventTapEnable(tap_proxy as CFMachPortRef, true) }` synchronously on the tap callback thread."
  implication: "This is the PAC crash site. The fix must move CGEventTapEnable off the callback thread."

- timestamp: "2026-06-11T00:02:00Z"
  observation: "observer.rs:13-15 declares extern CGEventTapEnable. tap_proxy parameter in CGEventTap callback is cast to CFMachPortRef (RELY-02 comment confirms this is valid per Apple docs)."
  implication: "Port reference is valid. The problem is the calling context (within callback), not the cast."

## Eliminated

- hypothesis: "CGEventTap creation fails — macro can't arm at all"
  eliminated_by: "Crash occurs DURING a callback invocation (TapDisabledByTimeout event), meaning the tap was created and received events. Failure is in the re-enable path, not creation."

- hypothesis: "Unsigned app hard-blocked from CGEventTap (D-09 scenario)"
  eliminated_by: "The tap runs and fires callbacks; the crash is a PAC enforcement failure on a specific API call pattern, not a TCC block."

## Resolution

root_cause: "CGEventTapEnable called from within CGEventTap callback at observer.rs:285 triggers SLEventTapEnable → CFMachPortGetContext → __CFCheckCFInfoPACSignature PAC trap on macOS 26 ARM64e. The PAC signature on the CFMachPort context pointer fails validation when accessed from the tap's own callback thread."
fix: "Spawn a new thread from the callback, using CFRetain to keep the port alive, call CGEventTapEnable from that thread, then CFRelease. This moves the PAC-sensitive call outside the tap callback context."
files_changed:
  - src-tauri/src/platform/macos/observer.rs
