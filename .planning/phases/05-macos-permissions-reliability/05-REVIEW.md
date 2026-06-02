---
phase: "05"
phase_name: "macos-permissions-reliability"
reviewed: 2026-06-02T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/Cargo.toml
  - src/App.tsx
findings:
  critical: 3
  warning: 4
  info: 3
  total: 10
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-06-02
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

Phase 05 targeted macOS Accessibility permission detection and IPC reliability. The three reviewed files introduce `check_accessibility` / `request_accessibility` commands that arm the CGEventTap after a permission grant, an `accessibilityPending` UI state machine with a 30-second timeout, and profile-management IPC commands.

Three critical issues were found: a TOCTOU race in `initialize_tap()` can spawn a second CGEventTap thread between the time the CAS succeeds and the spawned thread reaches its failure-reset branch; the accessibility poll loop never clears `accessibilityPending` on a `false` response, locking users in "Pending…" after a denied dialog; and `save_profile` accepts an un-sanitized name from the IPC caller but passes it directly to `ProfileData.name`, letting the unsanitized name be stored in the JSON and used as a display string — while `profile_path()` does sanitize before building the path, a crafted name containing special characters can cause the stored `name` field to diverge from the filename stem, breaking subsequent `load_profile` lookups by name.

---

## Critical Issues

### CR-01: TOCTOU race in `initialize_tap()` — second tap thread can spawn between CAS and failure-reset

**File:** `src-tauri/src/platform/macos/observer.rs:217-463`
**Issue:** `initialize_tap()` does a `compare_exchange(false, true)` to mark initialization, then unconditionally spawns a thread. Inside the spawned thread, if `CGEventTap::new` fails (permissions not granted), the thread resets `TAP_INITIALIZED` back to `false` (line 457). This reset is asynchronous — it runs on the spawned OS thread after the CAS has already returned `true` to the caller.

The 3-second accessibility poll in `App.tsx` (line 164) calls `check_accessibility`, which calls `initialize_tap()` on every successful permission check. If the first spawned thread is inside `CGEventTap::new` and has not yet reset the flag (which can take dozens of milliseconds on a slow system or under permission-denied conditions), the next poll fires the CAS, sees `true` (already set), and returns early — this is the intended behavior. However, if the first thread has already reset `TAP_INITIALIZED` to `false` and then the poll fires, the CAS succeeds again and a second thread is spawned. Two threads then run `CFRunLoop::run_current()` concurrently, each with their own tap source, double-consuming events from `REGISTRY` and causing unpredictable double-injection or corrupted held-input tracking.

The comment at line 212-215 claims "Exactly ONE thread is ever spawned" but the guarantee breaks whenever the spawned thread resets the flag before the next poll cycle.

**Impact:** Silent double-injection of inputs, corrupted `REGISTRY` state, undefined behavior in the held-input emergency-stop path.

**Fix:** Introduce a `TAP_STARTING` `AtomicBool` that is set to `true` before spawning and cleared only by the spawned thread. `initialize_tap()` returns early if either `TAP_INITIALIZED` or `TAP_STARTING` is true:

```rust
static TAP_STARTING: AtomicBool = AtomicBool::new(false);

pub fn initialize_tap() -> bool {
    if TAP_INITIALIZED.load(Ordering::SeqCst) { return true; }
    // Prevent concurrent spawn attempts from racing through the window
    // between CAS and the spawned thread setting TAP_INITIALIZED to false.
    if TAP_STARTING.swap(true, Ordering::SeqCst) { return false; }

    TAP_INITIALIZED.store(true, Ordering::SeqCst);
    thread::spawn(|| {
        // ... existing tap code ...
        // In ALL exit branches (Ok runloop exit, Err runloop source, Err tap):
        TAP_INITIALIZED.store(false, Ordering::SeqCst);
        TAP_STARTING.store(false, Ordering::SeqCst); // always last
    });
    true
}
```

---

### CR-02: Accessibility poll never clears `accessibilityPending` on `false` — UI stuck in "Pending…" after deny

**File:** `src/App.tsx:163-174`
**Issue:** The 3-second poll calls `clearPending()` only when `ok === true`:

```ts
const ok = await invoke<boolean>("check_accessibility");
setAccessibility(ok);
if (ok) clearPending();          // ← false branch never clears
```

When the user dismisses the macOS dialog with "Don't Allow", `request_accessibility` returns `false` and the next poll sets `accessibility` to `false`. However, `accessibilityPending` remains `true`. The UI renders the "Pending…" branch (lines 484-491) instead of the "Denied" branch with the "Grant Access" button — hiding the recovery action from the user for up to 30 seconds. Since the 30-second timeout fires `setAccessibilityPending(false)` without calling `clearTimeout`, there is also a one-time callback fire even after the timeout has been superseded by the poll result.

**Impact:** After the user denies accessibility, the "Grant Access" button is hidden for up to 30 seconds. Users who deny and want to retry cannot do so immediately.

**Fix:**

```ts
const ok = await invoke<boolean>("check_accessibility");
setAccessibility(ok);
// Clear pending on any definitive response (granted or denied).
// Only remain "pending" while the dialog is actually in-flight (null).
clearPending();
```

---

### CR-03: `save_profile` stores unsanitized name in `ProfileData.name` — load-by-name will fail for names with special characters

**File:** `src-tauri/src/ipc/mod.rs:259-279`
**Issue:** `save_profile` takes the raw `name: String` from the IPC caller, puts it verbatim into `ProfileData { name, .. }`, and passes it to `profile_mgr.save_profile()`. Inside `save_profile`, `profile_path()` sanitizes the name before building the filename (stripping anything that is not alphanumeric, `-`, `_`, or space). The JSON file therefore has a sanitized filename but a `name` field containing the raw, unsanitized string.

Example: caller sends `name = "My Profile!!!"`. The file is written to `My Profile.json` (characters stripped). The JSON contains `"name": "My Profile!!!"`. When `list_profiles()` reads this file back, it returns a `ProfileSummary` with `name = "My Profile!!!"`. When the user then calls `load_profile("My Profile!!!")`, `profile_path("My Profile!!!")` sanitizes to `My Profile.json` and finds the file — this lookup accidentally succeeds. However, if the user calls `delete_profile("My Profile!!!")`, `eq_ignore_ascii_case("default")` check passes, and `profile_path` sanitizes to `My Profile.json` and deletes the correct file. So in the current code the roundtrip works by coincidence because `profile_path()` re-sanitizes on every call.

The real breakage occurs when the sanitized form of two different raw names collides: `"My Profile"` and `"My Profile!!!"` both map to `My Profile.json`. The second save overwrites the first silently. The frontend will display two distinct profile names from `list_profiles()`, but both load from and write to the same file, causing silent data loss.

**Impact:** Silent overwrite of one profile by another when their sanitized names collide. Data loss is reproducible with any two names that differ only in stripped characters.

**Fix:** Sanitize the name before storing it in `ProfileData`, so the stored name always matches the filename stem:

```rust
pub async fn save_profile(
    state: State<'_, StateManager>,
    profile_mgr: State<'_, Arc<crate::persistence::ProfileManager>>,
    name: String,
) -> Result<(), String> {
    // Sanitize early so stored name matches filename stem.
    let safe_name = crate::persistence::ProfileManager::sanitize_name_pub(&name);
    if safe_name.is_empty() {
        return Err("Profile name must contain at least one alphanumeric character.".to_string());
    }
    // ... snapshot current state ...
    let profile = crate::persistence::ProfileData {
        name: safe_name,
        macros: app_state.macros,
        engine_active: app_state.engine_active,
    };
    profile_mgr.save_profile(&profile).await
}
```

This also requires exposing `sanitize_name` as `pub fn sanitize_name_pub` (or making it `pub`). Additionally, add a maximum-length guard to prevent arbitrarily long filenames.

---

## Warning Findings

### WR-01: `_pendingTimeoutId` at module level survives HMR remount — dangling setTimeout callback

**File:** `src/App.tsx:77`
**Issue:** `_pendingTimeoutId` is declared at module scope (line 77), outside `App()`. The `onCleanup` at line 177 calls `clearPending()` on unmount, which is correct for normal lifecycle. In development HMR or any scenario where the page is reloaded without a full process restart, the module-level variable persists. If the component is destroyed before the 30-second timeout fires and the timeout handle was lost (e.g., HMR replaced the module), `setAccessibilityPending` will be called on a stale signal setter from the previous `App` instance.

`_keyCaptureListener` is correctly scoped at module level for single-listener enforcement (per the T-03-08 comment), but `_pendingTimeoutId` has no such architectural justification for module scope — it is purely local state.

**Impact:** In development builds, stale `setAccessibilityPending(false)` calls against a dead component instance. In production this is benign but represents an architectural inconsistency.

**Fix:** Move `_pendingTimeoutId` inside `App()`:

```ts
function App() {
  let _pendingTimeoutId: ReturnType<typeof setTimeout> | null = null;
  // ...
}
```

---

### WR-02: `navigator.platform` is deprecated — `IS_MACOS` can be `false` on macOS, silently skipping hotkey bind

**File:** `src/App.tsx:119`
**Issue:**

```ts
const IS_MACOS = navigator.platform.toLowerCase().includes("mac");
```

`navigator.platform` is deprecated (MDN: deprecated since 2021, removed from some contexts). In Chromium-based WebViews used by Tauri, this property returns an empty string in some configurations. If `IS_MACOS` is incorrectly `false` on macOS, `handleCardSetTriggerKey` takes the `else` branch (line 307-308) and only calls `set_macro_trigger_key` — skipping `unbind_hotkey` and `bind_hotkey`. The CGEventTap will not register the new binding, so the hotkey never fires. This is a silent behavioral failure with no error or warning to the user.

**Impact:** On macOS builds where `navigator.platform` returns an empty string or unexpected value, trigger key bindings set from the card UI are silently never registered in the CGEventTap.

**Fix:** Use the Tauri platform API, which reads from the compiled Rust platform constant:

```ts
import { platform } from "@tauri-apps/plugin-os";
// In App(), inside an async init or createEffect:
const IS_MACOS = (await platform()) === "macos";
```

Or expose a `get_platform` command on the Rust side using `cfg!(target_os = "macos")`, which is authoritative.

---

### WR-03: `handleRequestAccess` does not guard against post-unmount state updates — stale setter risk

**File:** `src/App.tsx:195-209`
**Issue:** `handleRequestAccess` is an `async` function that calls `invoke<boolean>("request_accessibility")` and then calls `setAccessibility(granted)` and `clearPending()` on the result. There is no `cancelled` guard. If the component is destroyed while the `invoke` is in-flight (e.g., the Tauri window is closed, or in tests), the promise resolves and fires the setters on a dead signal. The initial data fetch (lines 123-148) correctly uses a `cancelled` flag via `onCleanup`, but this pattern was not applied to `handleRequestAccess`.

Additionally, the race between the 30-second timeout and a late `invoke` resolution is not handled: if the timeout fires and clears `_pendingTimeoutId = null`, then `invoke` resolves with `granted = false`, `clearPending()` is called — `setAccessibilityPending(false)` fires again redundantly (benign but imprecise).

**Impact:** Potential state update on unmounted component. In SolidJS this does not crash but may produce unexpected reactive effects if the signal is tracked elsewhere.

**Fix:** Apply the same `cancelled` closure pattern used in the initial data fetch:

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
      clearPending();
    }
  } catch (e) {
    if (!cancelled) {
      console.error("Accessibility request failed:", e);
      clearPending();
    }
  }
  // Register cleanup at call site in a createEffect if needed,
  // or store cancelled setter in component scope ref.
}
```

Note that `onCleanup` only works inside reactive tracking scopes (i.e., `createEffect`). The correct approach is to store `cancelled` as a `let` in component scope and set it to `true` inside the existing `onCleanup` at line 177.

---

### WR-04: `handleLoadProfile` discards the returned `ProfileData` — frontend state not updated from profile contents

**File:** `src/App.tsx:376-387`
**Issue:**

```ts
async function handleLoadProfile(name: string) {
  setProfileLoading(true);
  try {
    await invoke<ProfileData>("load_profile", { name });  // return value discarded
    setActiveProfile(name);
    showProfileMsg(`Loaded "${name}"`, "success");
  }
  ...
}
```

`load_profile` returns `ProfileData` (the full macro map and `engine_active` flag). The return value is explicitly typed as `invoke<ProfileData>` but is discarded. The frontend instead relies on the backend emitting a `state-changed` event after the profile is applied by the StateActor.

If the `state-changed` event is lost (channel backpressure, event delivery failure, or timing) the UI will show "Loaded" but display stale macros. The frontend has no fallback to re-request state when the event is missed.

**Impact:** If the backend `state-changed` event is dropped, the UI will show the wrong macro list after a profile load. The success toast is shown before the UI has confirmed the new state is reflected.

**Fix:** Either use the returned `ProfileData` to update state directly (eliminating the event dependency for this path), or immediately call `get_state` after a successful load to confirm synchronization:

```ts
const profileData = await invoke<ProfileData>("load_profile", { name });
// Option A: apply the returned data directly
setState(prev => prev ? { ...prev, macros: profileData.macros, engine_active: profileData.engine_active } : prev);
// Option B: explicit re-fetch as fallback
const freshState = await invoke<AppState>("get_state");
setState(freshState);
```

---

## Info

### IN-01: `tokio-util` declared as dependency but not used in any source file

**File:** `src-tauri/Cargo.toml:24`
**Issue:**

```toml
tokio-util = "0.7"
```

No `use tokio_util` or `tokio_util::` references appear anywhere in `src-tauri/src/`. This is an unused dependency that increases compile time and binary size.

**Fix:** Remove the `tokio-util` line from `[dependencies]`.

---

### IN-02: `println!` in CGEventTap callback on the hot path — blocks on stdout lock

**File:** `src-tauri/src/platform/macos/observer.rs:318`
**Issue:**

```rust
println!("EMERGENCY STOP TRIGGERED");
```

This executes in the CGEventTap callback on the CFRunLoop OS thread. `println!` acquires the global stdout lock, which can block if another thread is writing to stdout. This delays the emergency-stop path — specifically, it delays `flush_held_inputs()` and `process::exit(1)`. While emergency stop calls `exit(1)` shortly after, any delay to input release can cause inputs to remain held for an additional scheduler tick, injecting spurious events.

The rest of the observer uses `#[cfg(debug_assertions)]` `eprintln!` for diagnostic output. This `println!` is unconditional and inconsistent with that pattern.

**Fix:**

```rust
#[cfg(debug_assertions)]
eprintln!("[Observer] Emergency stop triggered");
```

Or remove it entirely — `process::exit(1)` follows immediately and is unambiguous in any crash dump.

---

### IN-03: `[, setLoading]` signal — getter is discarded, `loading` state never used in render

**File:** `src/App.tsx:84`
**Issue:**

```ts
const [, setLoading] = createSignal(true);
```

The `loading` getter is discarded with `,`. `setLoading(false)` is called once in the initial fetch `finally` block (line 146), but the value is never read in the render tree. There is no loading spinner, skeleton, or disabled state gated on this signal. The signal therefore has no observable effect on the UI and exists as dead state.

**Impact:** Dead code. With `noUnusedLocals: true` in `tsconfig.json`, the discarded getter pattern avoids the TypeScript lint error, masking the dead code from the compiler.

**Fix:** If a loading state is desired, implement it in the UI. If not, remove both the signal and the `setLoading(false)` call:

```ts
// Remove line 84 entirely and remove:
// if (!cancelled) setLoading(false);  (line 146)
```

---

_Reviewed: 2026-06-02_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
