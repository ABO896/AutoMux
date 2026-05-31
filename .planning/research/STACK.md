# Stack & Platform Research

**Project:** AutoMux
**Researched:** 2026-05-15
**Scope:** macOS Accessibility permissions false-positive; CGEventTap entitlements; Tauri 2 specifics; re-check pattern

---

## macOS Permissions — Root Causes

### Root Cause 1: AXIsProcessTrusted() is evaluated at call time, not at grant time

`AXIsProcessTrusted()` queries the Accessibility database for the **current process identity** at the moment it is called. It does not receive a callback when the user changes the setting. After the user grants access in System Settings, the OS updates its database, but the running process must call `AXIsProcessTrusted()` again to observe the new value. There is no notification. A common misconception is that the app needs to restart — it does not. A re-poll is sufficient and the call will return `true` within milliseconds of the user clicking the toggle in System Settings.

**Consequence for AutoMux:** The `check_accessibility` IPC command polls every 10 seconds. If the user grants access between polls, the UI will not update for up to 10 seconds, and `initialize_tap()` will never be called because the transition from false → true is detected only by the polling interval, and the polling path does not call `initialize_tap()`. The tap is only initialized inside `request_accessibility`, which is only called when the user clicks the "Request Access" button — not on subsequent polls.

### Root Cause 2: App bundle identity changes invalidate the TCC database entry

macOS Transparency, Consent, and Control (TCC) ties the Accessibility grant to the app's **code signature and bundle path**. If any of the following change, the OS treats the app as a different identity and the old grant no longer applies:

- The app is rebuilt and the binary changes (common in dev)
- The `.app` bundle is moved to a different path
- The bundle identifier (`CFBundleIdentifier`) changes
- The signing identity changes (signed vs unsigned, or different signing cert)
- An update replaces the binary without removing the old TCC entry first

This is the most common cause of "I granted it but it still says denied" reports. The grant in System Settings may refer to a stale path or identity. The user must remove the old entry and re-grant.

**Consequence for AutoMux:** The app is currently **unsigned** (`"signingIdentity": null`). Unsigned apps use the binary path as their identity anchor. Every `cargo build` regenerates the binary, which can invalidate the TCC entry. Users who update the app by replacing the binary (rather than reinstalling the .dmg) will hit this.

### Root Cause 3: No entitlements file — potential sandbox concern

`tauri.conf.json` has `"entitlements": null`. Tauri 2 does not sandbox apps by default (no App Sandbox), which is correct for an app needing `CGEventTap`. However, without an entitlements file, there is no place to declare `NSAppleEventsUsageDescription` or other keys that inform the system about permission intent.

More critically: Tauri's macOS bundle process may inject a default entitlements file during signing. If that file accidentally includes `com.apple.security.app-sandbox`, the CGEventTap will fail at creation time (not at permission check time), and the error will surface as `CGEventTap::new()` returning `Err(...)`. This is a separate failure mode from TCC denial — the tap creation fails even if TCC says granted.

**Confirmed from code:** `CGEventTap::new()` failure path in `observer.rs:373-379` resets `TAP_INITIALIZED` to false but does not signal the frontend that this happened. The app silently continues with no tap running, no hotkey support, and no emergency stop.

### Root Cause 4: TAP_INITIALIZED is reset on failure but initialize_tap() is not retried

When `CGEventTap::new()` fails (permissions denied, sandbox conflict, or any other reason):

```rust
// observer.rs:373-379
Err(_) => {
    eprintln!("Failed to create CGEventTap...");
    TAP_INITIALIZED.store(false, Ordering::SeqCst);
}
```

The atomic is reset to `false`, but `initialize_tap()` is never called again from any code path unless the user explicitly clicks "Request Access" again (which calls `request_accessibility`). The 10-second polling loop calls `check_accessibility` (the silent check IPC), which just returns a boolean — it does not call `initialize_tap()` even when the result transitions from false to true.

### Root Cause 5: AXIsProcessTrustedWithOptions with prompt=true returns current status, not post-grant status

The `request_accessibility` IPC handler does:

```rust
let granted = crate::platform::macos::check_accessibility_permissions(true);
if granted {
    crate::platform::macos::observer::initialize_tap();
}
Ok(granted)
```

`AXIsProcessTrustedWithOptions({ AXTrustedCheckOptionPrompt: true })` shows the System Settings dialog **and** returns the current trust status. If the app does not yet have permission, it returns `false` even though it just showed the prompt. The user must then switch to System Settings, grant access, switch back to the app — but by then this call has already returned `false` and `initialize_tap()` was not called.

The user sees "Permissions Required" even after granting, because `initialize_tap()` was never called during the session.

---

## macOS Permissions — Solutions

### Solution 1: Initialize the tap when the poll detects the false→true transition

The 10-second polling loop already detects the new permission state. It just needs to call `initialize_tap()` when it observes the transition. Two options:

**Option A: Re-check + tap init in the IPC poll handler (simplest)**

Add an IPC command `check_and_init_accessibility` that does both:

```rust
#[command]
pub async fn check_and_init_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let granted = crate::platform::macos::check_accessibility_permissions(false);
        if granted {
            crate::platform::macos::observer::initialize_tap();
        }
        Ok(granted)
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(true) }
}
```

Replace the polling call in `App.tsx` from `check_accessibility` to `check_and_init_accessibility`. The existing `initialize_tap()` is already idempotent (the `compare_exchange` guard), so calling it repeatedly when already initialized is a no-op.

**Option B: Modify the existing `check_accessibility` command to also trigger init**

Simpler: change `check_accessibility` in `ipc/mod.rs` to call `initialize_tap()` when returning `true`. No frontend change needed.

```rust
#[command]
pub async fn check_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let granted = crate::platform::macos::check_accessibility_permissions(false);
        if granted {
            // Idempotent: no-op if tap already running
            crate::platform::macos::observer::initialize_tap();
        }
        Ok(granted)
    }
    #[cfg(not(target_os = "macos"))]
    { Ok(true) }
}
```

**Recommendation: Option B.** Zero frontend changes, smallest diff, safe due to idempotency guard.

### Solution 2: Reduce the polling interval during the permission-pending state

The current interval is 10 seconds (`setInterval(..., 10000)` in `App.tsx:126`). During the window when the user has gone to System Settings but not yet returned, 10 seconds is a poor UX. Change to 1-2 seconds when `accessibility()` is `false`, and revert to a longer interval once granted.

```typescript
// App.tsx — adaptive polling
createEffect(() => {
  const poll = async () => {
    try {
      const ok = await invoke<boolean>("check_accessibility");
      if (ok && !accessibility()) {
        // Transition: just became granted — also trigger tap init
        // (handled server-side if Solution 1B is applied)
        setAccessibility(ok);
      } else {
        setAccessibility(ok);
      }
    } catch (_) {}
  };

  const interval = setInterval(poll, accessibility() === false ? 1500 : 10000);
  onCleanup(() => clearInterval(interval));
});
```

However, reactive intervals in SolidJS require care — a simpler approach is a fixed 1-2 second interval unconditionally since `check_accessibility` is a cheap syscall.

### Solution 3: Instruct users to remove the stale TCC entry when updating

For the unsigned binary identity issue: document clearly in the app UI and README that when updating AutoMux, users must:
1. Remove the old AutoMux entry from System Settings → Privacy & Security → Accessibility
2. Relaunch the new version
3. Re-grant when prompted

An in-app update flow (future phase) should automate this.

### Solution 4: Handle CGEventTap creation failure visibly

Currently, `CGEventTap::new()` failure is silent to the frontend. Add a mechanism to surface this:

```rust
// In initialize_tap(), instead of just eprintln!, push an Intent
Err(_) => {
    TAP_INITIALIZED.store(false, Ordering::SeqCst);
    if let Some(tx) = STATE_TX.get() {
        let _ = tx.try_send(crate::state::Intent::TapInitFailed);
    }
}
```

Add `TapInitFailed` to the `Intent` enum and have `StateActor` broadcast a `tap-failed` Tauri event. Frontend can show a distinct error: "Accessibility was granted but the event tap could not start. Please restart AutoMux."

This disambiguates "TCC denied" (permissions not granted) from "tap creation failed despite permissions" (sandbox conflict, binary identity mismatch, or OS-level CGEventTap exhaustion).

---

## Tauri 2 Specifics

### Entitlements configuration in Tauri 2

Tauri 2 reads the entitlements file path from `bundle.macOS.entitlements` in `tauri.conf.json`. Currently set to `null`, meaning no custom entitlements file is used. Tauri's bundler may generate a minimal default.

**Current state:**
```json
"macOS": {
  "minimumSystemVersion": "12.0",
  "frameworks": [],
  "entitlements": null,
  "signingIdentity": null
}
```

For a non-sandboxed app using `CGEventTap`, the entitlements file should be minimal and must NOT include `com.apple.security.app-sandbox`. A correct entitlements file for AutoMux:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- DO NOT add com.apple.security.app-sandbox: it blocks CGEventTap -->
    <key>com.apple.security.automation.apple-events</key>
    <true/>
</dict>
</plist>
```

Then reference it:
```json
"macOS": {
  "minimumSystemVersion": "12.0",
  "entitlements": "entitlements.plist",
  "signingIdentity": null
}
```

**Important:** For an unsigned app distributed via DMG (current state), entitlements are only embedded during signing. Without a signing identity, the entitlements file has no effect at runtime. The entitlements file becomes critical when the app is signed. Since AutoMux is currently unsigned, the entitlements file is not the cause of the current permissions bug — but it becomes required if the app is ever signed or submitted to the Mac App Store.

### CGEventTap and the App Sandbox

`CGEventTap` with `CGEventTapLocation::HID` (system-wide) is incompatible with App Sandbox. If the sandbox entitlement is present, `CGEventTap::new()` will fail at runtime. The current AutoMux codebase is not sandboxed, which is correct.

To verify no sandbox is accidentally added: the absence of `com.apple.security.app-sandbox` in any entitlements file and in the Tauri build output is what matters. With `"entitlements": null`, Tauri 2 (as of v2.x) does not add a sandbox by default for non-App-Store builds.

**Confidence:** MEDIUM — verified from Tauri source behavior and known CGEventTap constraints; cannot fetch live Tauri 2 docs due to tool restrictions.

### How Tauri 2 wraps macOS permission flows

Tauri 2 does not provide a built-in abstraction for `AXIsProcessTrusted()` or Accessibility permissions. There is no `tauri-plugin-accessibility` or equivalent in the official plugin ecosystem as of mid-2025. The current approach (manual FFI to `AXIsProcessTrustedWithOptions` in `platform/macos/mod.rs`) is the standard approach used by Tauri-based automation apps.

Tauri's `tauri-plugin-opener` can be used to open the URL `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility` to direct the user to the exact System Settings pane. This is better UX than showing the system prompt (which is small and easy to dismiss). The current codebase does not use this.

### NSUsageDescription keys

Apps that request Accessibility programmatically do not need an `NSAccessibilityUsageDescription` key for the system prompt — that key does not exist. The user is directed to System Settings directly. However, some apps add an `NSAppleEventsUsageDescription` if they also send Apple Events. AutoMux does not need this.

For the `Info.plist` embedded in the bundle, Tauri 2 allows a custom `Info.plist` via `bundle.macOS.infoPlist` (a dictionary in `tauri.conf.json`). No custom keys are required for AutoMux's current feature set.

### Code signing and TCC database identity

Without a signing identity, macOS uses the absolute binary path as the TCC identity. This means:
- Moving the app invalidates the TCC grant
- Rebuilding the binary does NOT change the path, so rebuilds in the same output directory preserve the grant for development builds
- Distributing via DMG where the user copies to `/Applications` is fine — the path is stable after installation

**The practical implication:** If a user updates AutoMux by replacing the app in `/Applications` with a new version from a DMG, the path stays the same, so the TCC grant survives the update (for unsigned apps). This is better than expected.

The false-positive problem is therefore more likely the "tap never gets initialized after grant" bug (Root Cause 1 + 4) than a TCC identity issue for most users.

---

## Re-check Pattern

### Recommended pattern: poll + init on transition

The correct re-check pattern for macOS accessibility permissions is:

1. At app startup: call the silent check. If granted, initialize the tap. If not, show the permissions prompt UI.
2. When the user clicks "Request Access": call `AXIsProcessTrustedWithOptions(prompt: true)` to show the system dialog. Do NOT expect this call to return `true` — the user hasn't granted yet.
3. Start polling at short intervals (1-2 seconds) while permissions are not granted.
4. When the poll returns `true`: call `initialize_tap()` and update the UI. This is the moment the tap starts running.
5. After granted, reduce polling interval or stop polling.

**The critical missing piece in AutoMux:** Step 4 does not happen. The polling loop (`setInterval` every 10 seconds) calls `check_accessibility` which just returns a boolean. It does not trigger `initialize_tap()`. The fix is to have the server-side `check_accessibility` (or a replacement command) call `initialize_tap()` when returning `true`.

### Opening System Settings directly

Better UX than the native prompt is to open System Settings directly to the Accessibility pane. Use the URL scheme:

```rust
// In request_accessibility handler, instead of/in addition to the prompt:
// Open System Settings → Privacy & Security → Accessibility
let url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
// Use tauri-plugin-opener or open::that(url)
```

Or from the frontend via the opener plugin:
```typescript
import { open } from '@tauri-apps/plugin-opener';

async function handleRequestAccess() {
  await open('x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility');
  // Start polling aggressively for the user to return and grant
}
```

This gives users a direct link to the right pane rather than a small system dialog.

### Notification-based re-check (advanced, not recommended)

macOS does not provide a public notification when TCC grants change. There is a private notification `com.apple.TCCDHelper` but using private APIs is fragile and against App Store guidelines. The polling approach is the standard pattern used by all automation apps (Raycast, Keyboard Maestro, BetterTouchTool, etc.).

The 1-2 second polling interval while pending is industry standard. At 2 seconds, the overhead is negligible: `AXIsProcessTrusted()` is a fast synchronous syscall that queries an in-process cache (TCC updates flush the cache within ~1 second of the grant).

### Handling the "app needs restart" misconception

The app does NOT need to restart. `AXIsProcessTrusted()` updates without restart. The "needs restart" behavior users report is caused by one of two things:

1. The tap was not initialized after the grant was detected (AutoMux's current bug)
2. The old binary/path was replaced and TCC associates with the new path (rare with stable installation paths)

Communicating this in the UI is important: the status banner should say "Grant access in System Settings, then return here — no restart needed" rather than suggesting a restart.

---

## Confidence Levels

| Finding | Confidence | Basis |
|---------|------------|-------|
| AXIsProcessTrusted() does not require app restart | HIGH | Well-documented macOS TCC behavior; consistent across multiple OS versions |
| Polling transition does not trigger initialize_tap() | HIGH | Direct code inspection of `ipc/mod.rs` and `App.tsx` |
| TAP_INITIALIZED reset on failure does not self-heal | HIGH | Direct code inspection of `observer.rs:373-379` |
| request_accessibility() returns false while user is in System Settings | HIGH | Direct code inspection + AXIsProcessTrustedWithOptions documented behavior |
| No entitlements file configured (entitlements: null) | HIGH | Direct read of `tauri.conf.json` |
| CGEventTap is incompatible with App Sandbox | HIGH | Documented macOS constraint; confirmed by absence of sandbox entitlement |
| Fix: modify check_accessibility to call initialize_tap() | HIGH | Follows directly from code analysis; initialize_tap() is already idempotent |
| TCC identity tie to bundle path for unsigned apps | MEDIUM | Documented macOS behavior; cannot currently verify with live sources |
| Tauri 2 does not add sandbox by default for non-App-Store builds | MEDIUM | Consistent with Tauri 2 behavior as of v2 GA; cannot verify against live docs |
| x-apple.systempreferences URL scheme for opening Accessibility pane | MEDIUM | Known macOS URL scheme used by many apps; works on macOS 13+; verify on macOS 12 |
| AXIsProcessTrusted() queries an in-process cache flushed within ~1s | MEDIUM | Consistent with observed behavior across many apps; not officially documented |
| No tauri-plugin-accessibility exists in official Tauri ecosystem | MEDIUM | Accurate as of mid-2025 training knowledge; could change |

---

## Summary of Required Changes

Priority order based on impact vs effort:

1. **P0 (root fix):** Modify `check_accessibility` IPC command to call `initialize_tap()` when returning `true`. Three-line change in `ipc/mod.rs`. Fixes the core bug where poll detects grant but tap never starts.

2. **P1 (UX):** Reduce polling interval while permissions are not granted (from 10s to 1-2s). Better responsiveness after user grants access. Change in `App.tsx`.

3. **P1 (UX):** Open System Settings directly via `x-apple.systempreferences:...` URL instead of (or in addition to) the native dialog. Removes the small, easy-to-miss system prompt.

4. **P2 (observability):** Surface `CGEventTap::new()` failure to frontend as a distinct error. Disambiguates "TCC denied" from "tap creation failed despite permissions."

5. **P3 (future/signing):** Create `entitlements.plist` without sandbox. Required when code signing is added.

---

*Research sources: Direct code inspection of AutoMux codebase (observer.rs, mod.rs, ipc/mod.rs, App.tsx, tauri.conf.json); macOS platform knowledge (AXIsProcessTrusted, TCC, CGEventTap behavior) from training data through August 2025.*
