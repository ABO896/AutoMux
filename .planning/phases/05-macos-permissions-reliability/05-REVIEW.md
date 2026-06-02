---
phase: 05-macos-permissions-reliability
reviewed: 2026-06-02T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src-tauri/Cargo.toml
  - src-tauri/src/ipc/mod.rs
  - src/App.tsx
findings:
  critical: 2
  warning: 3
  info: 2
  total: 7
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-06-02  
**Depth:** standard  
**Files Reviewed:** 3  
**Status:** issues_found

## Summary

Phase 05 introduces three changes: (1) `check_accessibility` in `ipc/mod.rs` now calls `initialize_tap()` when accessibility is granted — mirroring `request_accessibility`; (2) the `cocoa` crate is removed from `Cargo.toml`; (3) `App.tsx` gains an `accessibilityPending` signal, a `clearPending()` helper, a 3-branch permission UI, a 30-second timeout, and `onCleanup` cancellation.

The tap-initialization path contains a race window that can spawn a second thread before the first thread's failure resets the flag. The frontend `accessibilityPending` state is never cleared by the 3-second polling loop when accessibility transitions from `null` (unknown) to `false` (denied), creating a stuck "Pending…" indicator when the OS denies the dialog immediately. Additional issues include the timeout not being cancelled on component unmount in one code path, and a `navigator.platform` API that is deprecated and unreliable.

---

## Critical Issues

### CR-01: Race between `initialize_tap()` CAS success and thread startup allows double-tap spawn

**File:** `src-tauri/src/platform/macos/observer.rs:219-460`  
**Issue:** `initialize_tap()` uses `compare_exchange(false, true, …)` to set `TAP_INITIALIZED` to `true` before the spawned thread even calls `CGEventTap::new`. If `CGEventTap::new` fails (permissions not yet granted), the spawned thread resets `TAP_INITIALIZED` back to `false` — but this reset happens on the OS thread, asynchronously. If `check_accessibility` is called again (by the 3-second poll) while the first thread is still inside `CGEventTap::new` and has not yet reset the flag, the CAS will see `true` and return early (correct behavior). However, if the first thread fails and resets to `false` *just before* the second poll fires the CAS, the CAS succeeds again and a second thread is spawned. This is a TOCTOU window, not a theoretical one: the 3-second poll fires while `CGEventTap::new` runs, and the total time to create a tap when permissions are absent is non-deterministic.

The consequence is two concurrent `CFRunLoop::run_current()` invocations on two different OS threads, each with their own tap and `REGISTRY` event stream. This can silently double-count injected inputs and corrupt the `REGISTRY` set.

**Fix:** Keep `TAP_INITIALIZED` true while any attempt is in-flight. Introduce a second `AtomicBool` (`TAP_STARTING`) as a "spin-lock" during thread startup, or move the reset to a Tokio channel acknowledgment from the spawned thread once the tap either succeeds or definitively fails:

```rust
// In the spawned thread, only reset on definitive failure:
match tap_result {
    Ok(tap) => {
        // ... run loop ...
        // Only reset AFTER run loop exits (tap died at runtime):
        TAP_INITIALIZED.store(false, Ordering::SeqCst);
    }
    Err(_) => {
        eprintln!("Failed to create CGEventTap.");
        TAP_INITIALIZED.store(false, Ordering::SeqCst);
        // ↑ This is fine here, but the window is between CAS and this line.
    }
}
// Fix: add a TAP_STARTING AtomicBool that is set true before spawn and
// cleared by the thread itself; initialize_tap() returns early if TAP_STARTING is true.
static TAP_STARTING: AtomicBool = AtomicBool::new(false);

pub fn initialize_tap() -> bool {
    if TAP_INITIALIZED.load(Ordering::SeqCst) { return true; }
    // Guard against concurrent startup attempts:
    if TAP_STARTING.swap(true, Ordering::SeqCst) { return false; }
    // CAS is now safe — only one caller passes the swap guard.
    TAP_INITIALIZED.store(true, Ordering::SeqCst);
    thread::spawn(|| {
        // ... tap setup ...
        TAP_STARTING.store(false, Ordering::SeqCst); // clear on exit
    });
    true
}
```

---

### CR-02: `accessibilityPending` not cleared when accessibility is definitively `false` after the dialog is dismissed

**File:** `src/App.tsx:163-174`  
**Issue:** The 3-second polling loop calls `clearPending()` only when `ok === true`:

```ts
const ok = await invoke<boolean>("check_accessibility");
setAccessibility(ok);
if (ok) clearPending();
```

When the user clicks "Deny" in the macOS system dialog, `request_accessibility` returns `false`, and the next poll will set `accessibility` to `false`. However, `clearPending()` is never called for the `false` case. The UI will remain stuck in the "Pending…" branch indefinitely (or until the 30-second timeout fires). Users who deny the dialog and then look at the UI will see "Pending…" rather than "Denied", and the "Grant Access" button will not be shown. This masks the error state.

The timeout at line 197-200 will eventually fire and call `setAccessibilityPending(false)`, but 30 seconds is far too long for a user who has already dismissed the dialog; the UI should reflect the deny immediately.

**Fix:** Clear pending on any definitive non-null response:

```ts
const ok = await invoke<boolean>("check_accessibility");
setAccessibility(ok);
clearPending(); // clear on any definitive response, not just granted
```

Or more explicitly, only remain pending while `accessibility()` is still `null`:

```ts
setAccessibility(ok);
if (ok !== null) clearPending();
```

---

## Warnings

### WR-01: `_pendingTimeoutId` is a module-level mutable — not cleared on component re-mount, causing a stale timeout to fire `setAccessibilityPending` on the new instance

**File:** `src/App.tsx:77`  
**Issue:** `_pendingTimeoutId` is declared at module level (line 77), outside `App()`. If the `App` component is ever unmounted and remounted (e.g., during hot-module replacement in development or if Tauri reloads the page), the old timeout handle is lost. The `onCleanup` at line 177 calls `clearPending()` which does clear `_pendingTimeoutId`, but only if `clearPending()` is available on the closure captured at unmount time. More concretely: when the component is destroyed, `clearPending` captures the `setAccessibilityPending` setter from the first mount. If the module reloads and a new `App` instance is created, `_pendingTimeoutId` holds a handle from the previous instance's `setTimeout`. The module-level variable persists across component re-mounts — the first mount's `onCleanup` will run `clearTimeout(_pendingTimeoutId)` correctly, but in HMR scenarios the cleanup does not always fire before the new mount initializes.

This is a lower-severity concern in production (Tauri webviews don't do HMR there), but it is an architectural smell that should be addressed: the timeout should be a `let` variable inside `App()`, managed entirely within the component scope.

**Fix:** Move `_pendingTimeoutId` inside `App()` alongside `_keyCaptureListener`:

```ts
function App() {
  let _pendingTimeoutId: ReturnType<typeof setTimeout> | null = null;
  // ... rest of component
}
```

---

### WR-02: `navigator.platform` is deprecated and unreliable for platform detection

**File:** `src/App.tsx:119`  
**Issue:**
```ts
const IS_MACOS = navigator.platform.toLowerCase().includes("mac");
```
`navigator.platform` is deprecated in all major browser engines (MDN: "deprecated since 2021"). In some Chromium versions embedded in Tauri, it returns an empty string or a generic value depending on the OS/WebView version. If this returns an empty string or `"Win32"` on a macOS build (possible in some Tauri 2 WebView configurations), `IS_MACOS` will be `false`, and the `bind_hotkey`/`unbind_hotkey` path in `handleCardSetTriggerKey` will be skipped on macOS, meaning hotkeys will not be registered after a trigger key is set on a card. This is a silent behavior gap — no error, no warning, wrong code path.

The correct approach for a Tauri app is to use a Tauri API or a backend IPC command that returns the platform string, since the backend is compiled per-platform and is authoritative.

**Fix:**
```ts
// Option 1: use @tauri-apps/plugin-os (already available in Tauri 2 ecosystem)
import { platform } from "@tauri-apps/plugin-os";
const IS_MACOS = (await platform()) === "macos";

// Option 2: add a minimal `get_platform` IPC command that returns
// a compile-time constant from the Rust side.
// cfg!(target_os = "macos") is authoritative.

// Option 3 (minimal change, avoids the deprecated API):
const IS_MACOS = navigator.userAgent.includes("Mac");
// userAgent is not deprecated and is more consistently populated in WebView.
```

---

### WR-03: `handleRequestAccess` leaves `accessibilityPending` as `true` if `invoke` hangs and the 30-second timeout fires concurrently with a successful `invoke` response

**File:** `src/App.tsx:195-209`  
**Issue:** There is a race between the 30-second `setTimeout` callback and the resolution of the `invoke<boolean>("request_accessibility")` promise:

1. User clicks "Grant Access" — `setAccessibilityPending(true)` set, 30s timer started.
2. User takes 29s to click "Allow" in System Settings.
3. At t=30s, the timeout fires: `setAccessibilityPending(false)`, `_pendingTimeoutId = null`.
4. At t=30.1s (or even t=29.9s if the invoke resolves just after timeout), `invoke` resolves with `granted = true`.
5. `setAccessibility(true)` is called — correct.
6. `clearPending()` is called — `_pendingTimeoutId` is already `null` so this is a no-op, but `setAccessibilityPending(false)` is called again redundantly.

The race itself is benign in the `granted = true` path. However in the `granted = false` path (deny occurred after a long delay, then the timeout fires, then `invoke` resolves):
- Timeout fires → `setAccessibilityPending(false)`.
- `invoke` resolves with `false` → `clearPending()` → `setAccessibilityPending(false)` (redundant, benign).

The more dangerous scenario: the user grants access after the timeout, but the component has already been destroyed (navigated away). The `invoke` promise still resolves and calls `setAccessibilityPending` and `setAccessibility` on a dead signal. The `cancelled` flag pattern from the initial fetch (line 122-148) is not applied here.

**Fix:** Add a `cancelled` guard to `handleRequestAccess` using the same pattern as the initial fetch:

```ts
async function handleRequestAccess() {
  let cancelled = false;
  setAccessibilityPending(true);
  _pendingTimeoutId = setTimeout(() => {
    if (!cancelled) setAccessibilityPending(false);
    _pendingTimeoutId = null;
  }, 30_000);
  try {
    const granted = await invoke<boolean>("request_accessibility");
    if (!cancelled) {
      setAccessibility(granted);
      if (granted) clearPending();
    }
  } catch (e) {
    if (!cancelled) {
      console.error("Accessibility request failed:", e);
      clearPending();
    }
  }
  onCleanup(() => { cancelled = true; clearPending(); });
}
```

Note: `onCleanup` inside a non-reactive function won't auto-register in SolidJS unless it is called from within a reactive tracking scope. A simpler approach is to check `cancelled` using a closure variable scoped to the `handleRequestAccess` call, as shown above, and store the cancel flag in a signal or ref readable by `onCleanup`.

---

## Info

### IN-01: `cocoa` removal is correct, but `Cargo.lock` should be committed to verify no transitive pin breakage

**File:** `src-tauri/Cargo.toml`  
**Issue:** The `cocoa = "0.26.1"` removal is the right call (it pulls in deprecated `block 0.1.6` alongside `block2 0.6.2`). However, `Cargo.lock` shows as modified in the git status but was not submitted for review. Removing `cocoa` may change the resolved version of shared transitive dependencies (e.g., `core-foundation`, `objc`, `objc2`). The compiled output should be verified to confirm no version regressions were introduced by the dependency graph change.

**Fix:** Run `cargo tree --duplicates` after the removal and confirm no unexpected duplicate versions of `objc`, `core-foundation`, or `core-graphics` appear. Commit the updated `Cargo.lock`.

---

### IN-02: `println!` left in hot CGEventTap callback path (emergency stop trigger)

**File:** `src-tauri/src/platform/macos/observer.rs:318`  
**Issue:**
```rust
println!("EMERGENCY STOP TRIGGERED");
```
This `println!` executes in the CGEventTap callback, which runs on the CFRunLoop OS thread. `println!` acquires a stdout lock and can block the callback. While this is in the emergency-stop branch (which then calls `process::exit(1)`), any lock contention delays the exit and delays the release of held inputs. This is pre-existing code, not introduced in phase 05, but it sits in a file that was read as part of cross-file analysis.

**Fix:** Replace with `eprintln!` wrapped in `#[cfg(debug_assertions)]` to be consistent with the pattern already used in the tap-disabled branch (line 260), or remove it entirely since `process::exit(1)` follows immediately:

```rust
#[cfg(debug_assertions)]
eprintln!("[Observer] Emergency stop triggered");
```

---

_Reviewed: 2026-06-02_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
