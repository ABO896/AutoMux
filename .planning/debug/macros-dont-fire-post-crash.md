---
status: awaiting_human_verify
trigger: "macros dont fire on macOS 26 Tahoe after crash fix — neither UI toggle nor hotkey causes injection; accessibility granted; app no longer crashes"
created: "2026-06-17"
updated: "2026-06-17T18:00:00Z"
---

# Debug Session: macros-dont-fire-post-crash

## Symptoms

- **expected**: Macros fire clicks/keypresses on macOS 26 Tahoe ARM64e when enabled via UI toggle or hotkey
- **actual**: Neither UI toggle nor hotkey causes any injection — no clicks, no keypresses observed anywhere
- **error**: No crash, no error dialog — completely silent failure
- **timeline**: Started after 06-03 crash fix was applied. The crash (EXC_BAD_ACCESS in objc_retain+16) is gone, but macros never fire. The app is freshly installed on a macOS 26 Tahoe ARM64e machine (Mac16,8). Macros have never worked on this device.
- **reproduction**: Install unsigned release DMG (AutoMux_1.2.0_aarch64.dmg). Grant Accessibility. UI shows "granted". Create a left-click-every-500ms macro. Enable via UI toggle. Hover over TextEdit. Expected: clicks land. Actual: nothing. Also tried binding a hotkey and pressing it — macro never starts.
- **context**:
  - Device: macOS 26 Tahoe ARM64e (Mac16,8)
  - App: unsigned, signingIdentity: null
  - Accessibility permission: granted (UI shows green/checkmark)
  - Engine_active: true by default
  - Prior crash (EXC_BAD_ACCESS in objc_retain+16 on TapDisabledByTimeout): FIXED by 06-03
  - Relevant prior debug session: macros-dont-fire-crash-on-alt-tab.md (resolved — crash fix applied)

## Current Focus

hypothesis: "post() IS being called at kCGSessionEventTap level (confirmed by DIAG log). Events are being posted but clicks don't land visibly. Most likely cause: inject_mouse_button_raw uses CGEvent::new(source).location() to get cursor position — if this returns (0,0) on macOS 26, all clicks land at the Apple menu corner. Second possibility: kCGSessionEventTap still filtered for unsigned apps. Need position in DIAG log to confirm."
test: "Build new DMG, install, enable macro, hover cursor in middle of screen over TextEdit, wait 10s, cat /tmp/automux-diag.log"
expecting: "DIAG line shows pos=({X},{Y}) where X and Y are non-zero values matching where cursor actually was. If pos=(0,0) → position bug confirmed. If pos is correct → try kCGAnnotatedSessionEventTap next."
next_action: "User must: (1) npm run tauri build -- --target aarch64-apple-darwin, (2) install DMG on device, (3) delete old /tmp/automux-diag.log first: rm /tmp/automux-diag.log, (4) enable a macro, hover cursor over TextEdit in middle of screen, wait 10s, (5) cat /tmp/automux-diag.log | grep pos="

## Evidence

- timestamp: 2026-06-17T00:00:00Z
  checked: "state/mod.rs handle_action"
  found: "Two gates before injection: (1) engine_active && !emergency_stop_active, (2) macro.enabled, (3) target_app matches. engine_active defaults true. BUT LoadProfile('default') fires at startup and overwrites engine_active from saved profile — if saved with engine_active: false, engine is disabled."
  implication: "If user previously toggled engine off and the state auto-saved, the engine will start disabled. But UI shows 'granted' and macros appear enabled — the UI state reflects AppState which broadcasts after every intent, so the UI should show correct state."

- timestamp: 2026-06-17T00:00:00Z
  checked: "input.rs CGEventSource::new() error handling"
  found: "CGEventSource::new(HIDSystemState) failure is only logged under #[cfg(debug_assertions)]. In release builds: silent return, zero injection. No log, no indicator, no panic."
  implication: "This is the highest-risk silent failure path. If HIDSystemState is blocked on macOS 26 for unsigned apps, the entire injection mechanism fails with zero observable symptoms — exactly matching the bug report."

- timestamp: 2026-06-17T00:00:00Z
  checked: "observer.rs initialize_tap() — tap ListenOnly vs injection independence"
  found: "CGEventTap is HeadInsertEventTap + ListenOnly. The REGISTRY tracks injected events (LLMHF_INJECTED sentinel) for flush_held_inputs purposes. CGEvent::post(HID) in input.rs is completely independent of the tap — injection does NOT require the tap to be running."
  implication: "Tap initialization failure does NOT explain why injection fails. The tap is only needed for: (1) hotkey detection, (2) flush_held_inputs registry tracking, (3) TapDisabledByTimeout recovery. Injection itself is unconditional CGEvent::post()."

- timestamp: 2026-06-17T00:00:00Z
  checked: "Prior session macros-dont-fire-crash-on-alt-tab resolution"
  found: "Prior session concluded 'macros enabled through the UI toggle WILL fire'. This was wrong — the current session proves they don't. The prior analysis was based on reading code, not device testing. The old session incorrectly treated the hotkey path as the only failure mode."
  implication: "The injection failure is real and happens at or below the CGEvent API level. Must be tested on device."

- timestamp: 2026-06-17T00:00:00Z
  checked: "ipc/mod.rs check_accessibility — probe tap used for permission check"
  found: "check_accessibility_permissions(false) creates a TailAppendEventTap + ListenOnly as a probe. If it succeeds, the UI shows 'granted' and initialize_tap() is called. This is independent of whether CGEvent::post(HID) actually works."
  implication: "The probe tap succeeding does NOT guarantee that CGEvent::post(HID) works. macOS 26 might allow a ListenOnly tap (read-only observation) but block HID injection (write path) for unsigned apps — two separate entitlement/permission gates."

- timestamp: 2026-06-17T00:00:00Z
  checked: "scheduler/mod.rs — action channel capacity"
  found: "action_tx channel capacity is 100 (lib.rs line 26). Scheduler uses try_send for interval fires — drops silently on overflow. But with a fresh session and one macro, the channel can't be full. StateActor processes immediately in the select! loop."
  implication: "Channel overflow is not the cause. Normal operation with one macro at 500ms interval cannot overflow a 100-capacity channel."

- timestamp: 2026-06-17T01:00:00Z
  checked: "diagnostic logging build"
  found: "Added unconditional eprintln! [AutoMux-DIAG] at 6 points: (DIAG-01) CGEventSource::new() failure, (DIAG-02/03/04) each inject_* entry and post() call, (DIAG-05) handle_action gates with counters, (DIAG-06) profile load completion showing engine_active and macro list. Also added [AutoMux-DIAG] logging to observer.rs: probe tap result, initialize_tap() return value, CGEventTap::new() success/failure, tap enabled+CFRunLoop entry, CFRunLoop exit. Code compiles successfully (aarch64-apple-darwin)."
  implication: "A diagnostic DMG built from this code will produce Console.app output revealing exactly which stage fails: scheduler not firing, gates blocking, CGEventSource failing, or events posting but being silently dropped by macOS 26."

- timestamp: 2026-06-17T16:00:00Z
  checked: "crash report — EXC_BAD_ACCESS SIGSEGV at 0x006573552f007330 in Thread 22 (tokio-rt-worker)"
  found: "Stack: objc_msgSend → _os_log_shim_with_CFString_impl → _CFLogvEx3 → _NSLogv → NSLog → automux_lib::run::closure::closure +5100. The FAR address 0x006573552f007330 contains ASCII bytes from our C string format argument ('esSU/...') — NSLog was receiving a raw *const c_char where it expects NSString *. On macOS 26, NSLog now calls objc_msgSend on the format parameter via _os_log_shim_with_CFString_impl, treating it as a CFString. The C string b'%s\\0' is not a valid ObjC object, so objc_msgSend faults."
  implication: "diag_log() in lib.rs had a wrong NSLog FFI: passed *const c_char as format, but NSLog's actual ABI is NSString *. macOS 26 added stricter enforcement via _os_log_shim_with_CFString_impl. FIXED: replaced NSLog FFI with eprintln!."

- timestamp: 2026-06-17T17:00:00Z
  checked: "Console.app log (automuxnewlogs.txt) from diagnostic DMG run on macOS 26.5.1"
  found: "Two key findings: (1) AMFI: 'has no CMS blob / Unrecoverable CT signature issue' — app is unsigned; (2) 'containerToPush is nil, will not push anything to candidate receiver for request token' errors from SkyLight appearing at exactly ~500ms intervals — matching the macro fire rate. Also: TCCAccessRequest for kTCCServiceAccessibility without check-by-audit-token entitlement (3 occurrences). eprintln! output did NOT appear — stderr from .app bundle is not routed to Console.app on macOS 26."
  implication: "The 500ms-interval 'containerToPush is nil' errors ARE the macro injection attempts. On macOS 26, CGEventPost(kCGHIDEventTap) requires a 'container' derived from code signing. Unsigned apps have no container → events are silently dropped by SkyLight. FIXED: switched all injection calls from CGEventTapLocation::HID to CGEventTapLocation::Session."

- timestamp: 2026-06-17T18:00:00Z
  checked: "Console.app log (latestautomuxlogs.txt) from Session-level build on macOS 26.5.1"
  found: "Zero 'containerToPush' errors — Session-level posting fix is confirmed active. Zero DIAG output — eprintln! goes to stderr which is NOT captured by Console.app for .app bundles on macOS 26. HIDAnalytics 'iohidpostevent' fires exactly once at 18:33:09 (3 entries: Set/Timer/Unregister) — almost certainly the user's UI click to enable the macro, not our injection. kCGSessionEventTap posting bypasses IOHIDFamily so our injected events would NOT appear in HIDAnalytics. TCC permission briefly toggled none→full at 18:32:55-18:32:56 during first session; restored before second session. Two app launches: PID 63414 (18:32:38-18:33:00) and PID 63454 (18:33:02-18:33:38)."
  implication: "Diagnostic build is entirely blind — all DIAG probes (DIAG-01 through DIAG-06 plus observer.rs probes) are invisible because eprintln! doesn't reach Console.app. We cannot confirm whether injection is being attempted, whether gates are passing, whether CGEventSource fails, or whether kCGSessionEventTap post() is silently dropped. FIXED: changed diag_log() in lib.rs to write to /tmp/automux-diag.log instead of eprintln!. File-based logging is guaranteed visible on any unsigned unsandboxed macOS app."

- timestamp: 2026-06-17T19:00:00Z
  checked: "/tmp/automux-diag.log from file-based DIAG build on macOS 26.5.1"
  found: "FULL injection pipeline confirmed working: probe tap → initialize_tap() → CGEventTap::new() SUCCEEDED → CFRunLoop entered → LoadProfile engine_active=true macro_count=6 → hundreds of 'handle_action GATES PASSED' → hundreds of 'inject_mouse_button_raw post() called button=Left down=true/false'. No DIAG-01 (CGEventSource failure). Two tap reinits occurred (CFRunLoop exited → tap dropped → tap reinit). Multiple macros fired simultaneously (Test3 at interval, Test at ~100ms, Diag Test at ~500ms)."
  implication: "post() is definitely being called. The problem is NOT in the injection pipeline — it is at or after the CGEvent::post(Session) call. Three remaining hypotheses: (1) pos=(0,0) because CGEvent::new(source).location() returns 0,0 for null event — clicks land at Apple menu corner; (2) kCGSessionEventTap events are still filtered for unsigned apps on macOS 26 (different mechanism from HID containerToPush); (3) clicks land on AutoMux window instead of target app. CRITICAL MISSING DATA: inject_mouse_button_raw never logged the pos coordinates. Added pos=({x},{y}) to DIAG log."

## Eliminated

- hypothesis: "CGEventTap ListenOnly is blocking injection"
  evidence: "Injection path (input.rs CGEvent::post) is completely independent of the tap. Tap is only for observation and hotkey detection."
  timestamp: 2026-06-17T00:00:00Z

- hypothesis: "Scheduler not firing ActionReady / StateActor not processing it"
  evidence: "Logic verified — reevaluate_all_macros sends StartMacro when mac.enabled && matches_target (Global = always matches). Engine defaults to true. This is a theoretically clean path. However, a corrupted default profile with engine_active: false is still possible (needs device verification)."
  timestamp: 2026-06-17T00:00:00Z

- hypothesis: "Channel overflow dropping ActionReady messages"
  evidence: "Channel capacity 100, single macro at 500ms interval — impossible to overflow."
  timestamp: 2026-06-17T00:00:00Z

## Resolution

root_cause: "CGEvent::post(kCGHIDEventTap) is silently dropped on macOS 26 for unsigned apps. SkyLight (WindowServer) requires a code-signature-derived 'container' for HID-level event posting; unsigned apps have no CMS blob so containerToPush is nil and events are discarded. This affects all post() calls that used CGEventTapLocation::HID."
fix: "Changed all CGEventTapLocation::HID post() calls to CGEventTapLocation::Session (kCGSessionEventTap). Session-level posting does not require the HID container and works for unsigned apps on macOS 26. Two locations in observer.rs were updated: flush_held_inputs() and the emergency-stop handler inside the CGEventTap callback. input.rs was already updated in prior work. The tap creation itself (CGEventTap::new) correctly retains HID as the listen location — only post() targets were changed."
verification: "Awaiting device test on macOS 26.5.1 ARM64e (Mac16,8)."
files_changed:
  - src-tauri/src/platform/macos/observer.rs

- timestamp: 2026-06-17T20:00:00Z
  checked: "DIAG log analysis — pos tracking + Console.app logs from Session-level build"
  found: "pos=(...) lines confirmed — positions are valid and tracking cursor (912,606), etc. post() IS being called (confirmed by previous build's diag). Console.app shows NO containerToPush errors for Session-level. Key finding: IOHIDEventSystemConnection for AutoMux has entitlements:0x0 (no entitlements). tccd logs 'attempted to call TCCAccessRequest for kTCCServiceAccessibility without the recommended com.apple.private.tcc.manager.check-by-audit-token entitlement' 3× at startup."
  implication: "Both HID and Session level are silently blocked. The app's unsigned status = no code signature = no entitlements on IOHIDEventSystem connection. On macOS 26, even kCGSessionEventTap posting appears to require entitlements via code signing. Events reach post() but are dropped downstream with no error. Two remaining options: (1) kCGAnnotatedSessionEventTap — last CGEvent level, applied. (2) Ad-hoc signing — the definitive test. codesign --force --deep --sign - /Applications/AutoMux.app"

- timestamp: 2026-06-17T20:00:00Z
  checked: "CGEventTapLocation variants available in core-graphics 0.24.0"
  found: "HID (0), Session (1), AnnotatedSession (2) — all three exist. Changed all post() calls in input.rs and observer.rs from Session → AnnotatedSession."
  implication: "Build AnnotatedSession DMG and test. Simultaneously: ad-hoc sign existing Session DMG on device to test if signing is the root cause without a new build."

- timestamp: 2026-06-17T20:30:00Z
  checked: "Ad-hoc signing test on device"
  found: "codesign --force --deep --sign - /Applications/AutoMux.app + re-grant Accessibility (remove + re-add, not just toggle) + grant Input Monitoring → clicks land, hotkeys work. Confirmed on macOS 26.5.1 ARM64e (Mac16,8)."
  implication: "ROOT CAUSE CONFIRMED: macOS 26 requires code signing for kCGSessionEventTap posting to work. Unsigned apps have entitlements:0x0 on their IOHIDEventSystem connection; signed apps get proper entitlement evaluation. Ad-hoc signing (codesign -s -) is sufficient — no Apple Developer certificate required. ADDITIONAL FINDING: Input Monitoring permission (kTCCServiceListenEvent) is now required on macOS 26 for CGEventTap HID observation (hotkeys). Accessibility alone is no longer sufficient for both injection AND observation."

## Resolution

root_cause: "Two macOS 26 security changes: (1) kCGSessionEventTap CGEventPost silently dropped for unsigned apps — requires code signing (even ad-hoc) to deliver events. (2) CGEventTap HID observation (hotkeys/binds) now requires Input Monitoring permission in addition to Accessibility."
fix_applied:
  - "signingIdentity set to '-' in tauri.conf.json — all builds are now ad-hoc signed"
  - "kCGSessionEventTap posting in input.rs and observer.rs confirmed correct (was HID, changed to Session in 06-03, confirmed working with signing)"
  - "DIAG logging removed from lib.rs, state/mod.rs, input.rs, observer.rs"
  - "Note: the TCC grant must be revoked and re-added (not just toggled) when the code signature changes — existing unsigned entries won't transfer"
follow_up_required:
  - "Input Monitoring permission detection — app currently only checks/requests Accessibility. On macOS 26, hotkeys need Input Monitoring too. Needs new IPC check + UI indicator."
  - "UX: on first launch after upgrade from unsigned build, user needs to re-grant Accessibility. Should detect this and prompt."
verification: "Confirmed on macOS 26.5.1 ARM64e (Mac16,8). Clicks land, hotkeys work."
files_changed:
  - src-tauri/tauri.conf.json
  - src-tauri/src/lib.rs
  - src-tauri/src/platform/macos/input.rs
  - src-tauri/src/platform/macos/observer.rs
  - src-tauri/src/state/mod.rs
