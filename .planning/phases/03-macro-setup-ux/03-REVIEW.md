---
phase: 03-macro-setup-ux
reviewed: 2026-05-30T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/platform/macos/observer.rs
  - src-tauri/src/platform/windows/mod.rs
  - src-tauri/src/state/mod.rs
  - src/App.tsx
  - src/keymap.ts
findings:
  critical: 4
  warning: 5
  info: 2
  total: 11
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-05-30T00:00:00Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

Phase 3 added a key-capture widget, a process picker (`<select>` replacing free-text input), `list_running_apps` IPC, `set_macro_trigger_key` IPC, and `keymap.ts` lookup tables. The scope is correct and the overall architecture is sound. However, four blockers were found.

The most serious defect is a Windows `HANDLE` leak in `get_app_name_from_hwnd` — the function is called on every foreground-window change (continuous leak) and on every `list_running_apps` call (per-window leak). The second critical defect is on macOS: the card-edit trigger-key path calls `bind_hotkey` (updating `HOTKEY_BINDINGS`) but never calls `set_macro_trigger_key`, leaving `MacroConfig.trigger_key` stale, which causes (a) the old key to remain a live trigger in `MACRO_TRIGGER_KEYS`, and (b) the UI badge to show the wrong key after editing. Two additional correctness issues are flagged below.

---

## Critical Issues

### CR-01: Windows HANDLE leak in `get_app_name_from_hwnd` — continuous resource drain

**File:** `src-tauri/src/platform/windows/mod.rs:279`

**Issue:** `OpenProcess` returns a raw `HANDLE` (a newtype over `*mut c_void`) that the `windows` crate does NOT auto-close on drop. `CloseHandle` is never called. This function is called:
- From `win_event_hook_callback` (line 466) on every foreground-window change — leaking one handle per app-switch for the lifetime of the process.
- From `enum_callback` inside `list_running_apps_impl` (line 311) — leaking one handle per visible window per picker open.

**Fix:**
```rust
use windows::Win32::Foundation::CloseHandle;

let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
let mut buf = [0u16; 260];
let mut len = buf.len() as u32;
let result = QueryFullProcessImageNameW(
    handle,
    PROCESS_NAME_FORMAT(0),
    windows::core::PWSTR(buf.as_mut_ptr()),
    &mut len,
);
unsafe { let _ = CloseHandle(handle); }
result.ok()?;
Some(String::from_utf16_lossy(&buf[..len as usize]))
```

---

### CR-02: macOS — card-edit trigger key leaves ghost trigger and shows stale UI badge

**File:** `src/App.tsx:280–289`, `src-tauri/src/platform/macos/observer.rs:192–200`

**Issue:** `handleCardSetTriggerKey` on macOS calls `unbind_hotkey` + `bind_hotkey` (updating `HOTKEY_BINDINGS`) but never calls `set_macro_trigger_key`. As a result:

1. `MacroConfig.trigger_key` is never updated in the `StateActor`. The state broadcast carries the old value, so the key badge in the UI shows the **old key** even after a successful edit.
2. `reevaluate_all_macros` re-populates `MACRO_TRIGGER_KEYS` from `MacroConfig.trigger_key` (still the old key), so the old key remains an active macro trigger — a **ghost hotkey** the user cannot see or remove.

**Fix:** On macOS, after `bind_hotkey`, also call `set_macro_trigger_key` to synchronise the persisted `trigger_key` field:
```typescript
async function handleCardSetTriggerKey(id: string, nativeCode: number) {
  try {
    if (IS_MACOS) {
      await invoke("unbind_hotkey", { macro_id: id });
      await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers: 0 });
      // Sync MacroConfig.trigger_key so the state broadcast reflects the new key
      // and MACRO_TRIGGER_KEYS stops firing the old key.
      await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode });
    } else {
      await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode });
    }
    ...
```

---

### CR-03: `domKeycodeToNative` returns 0 for unmapped keys, silently setting trigger to "A" (macOS)

**File:** `src/keymap.ts:182`

**Issue:** The fallback return value is `0`. On macOS, `CGKeyCode` `0` is the letter `A`. Pressing any key not in `DOM_KEYCODE_TO_CGKEYCODE` (e.g., a numpad key, PrintScreen, or an international key) silently assigns the macro's trigger key as `A`. The user sees `resolveKeyName(0)` display `"A"` and has no indication that capture failed. On Windows the fallback is also `0`, which is not a VK code but is passed to the backend regardless.

**Fix:** Return `null` for unknown codes and filter before committing:
```typescript
export function domKeycodeToNative(code: string): number | null {
  const map = IS_MACOS ? DOM_KEYCODE_TO_CGKEYCODE : DOM_KEYCODE_TO_VK;
  return map[code] ?? null;
}
```
In `startCapture`, guard the commit:
```typescript
const nativeCode = domKeycodeToNative(e.code);
if (nativeCode === null) return; // unmapped key — ignore
onCommit(nativeCode);
```

---

### CR-04: Windows `GetMessageW` infinite loop on Win32 error

**File:** `src-tauri/src/platform/windows/mod.rs:511–515`

**Issue:** `GetMessageW` returns `BOOL(-1)` (i.e. a non-zero value) on error, which `.into()` converts to `true`. The `while` loop therefore spins indefinitely on any message-pump error, burning a full CPU core with no recovery path.

```rust
// Current — loops forever on GetMessageW error:
while GetMessageW(&mut msg, None, 0, 0).into() {
```

**Fix:** Check the raw value and break on error:
```rust
loop {
    let ret = GetMessageW(&mut msg, None, 0, 0);
    if ret.0 == 0 { break; }      // WM_QUIT
    if ret.0 == -1 { break; }     // error — exit the pump
    TranslateMessage(&msg);
    DispatchMessageW(&msg);
}
```

---

## Warnings

### WR-01: Card target `<select>` has no `value` prop — always renders "Global" regardless of current target

**File:** `src/App.tsx:751–776`

**Issue:** The inline-edit `<select>` for the card target app has no `value=` attribute. The browser always selects the first option ("Global") as the visual default, even if `macro.target_app` is already set. A user opening the dropdown to inspect the current target sees "Global" highlighted, and if they click it to "confirm" the existing global setting, `onChange` fires with `""` and `handleCardSetTargetApp` sends `null` — **erasing the actual target silently**.

**Fix:**
```tsx
<select
  value={macro.target_app ?? ""}
  onFocus={handlePickerFocus}
  onChange={(e) => handleCardSetTargetApp(macro.id, e.currentTarget.value || null)}
  ...
>
```

---

### WR-02: No UI path to add a trigger key to an existing key-less macro via card

**File:** `src/App.tsx:779`

**Issue:** The key badge section is wrapped in `<Show when={macro.trigger_key !== null}>`. If a macro was created without a trigger key (or had its key cleared), there is no interactive element in the card to assign one — the only path is to delete and recreate the macro. This is a functional gap introduced in Phase 3.

**Fix:** Always render the key area, showing a placeholder when `trigger_key` is null:
```tsx
<div class="flex items-center gap-1">
  <Show
    when={macro.trigger_key !== null}
    fallback={
      <span
        class="px-1.5 py-0.5 rounded bg-surface-alt border border-dashed border-border text-[10px] font-mono text-text-dim cursor-pointer hover:border-accent/40"
        onClick={() => {
          setEditingCardId(macro.id);
          setEditingField("key");
          startCapture((nativeCode) => handleCardSetTriggerKey(macro.id, nativeCode));
        }}
      >
        + key
      </span>
    }
  >
    {/* existing badge markup */}
  </Show>
</div>
```

---

### WR-03: `initialize_tap()` always returns `true` even when tap creation fails asynchronously

**File:** `src-tauri/src/platform/macos/observer.rs:217–462`

**Issue:** The outer `initialize_tap()` function returns `true` unconditionally at line 462, regardless of whether the CGEventTap creation in the spawned thread succeeds or fails. The `TAP_INITIALIZED` flag is reset to `false` inside the thread on failure, but by then the caller (e.g. `start_observing`) has already observed `true` and logged "tap may be inactive" only when `initialize_tap()` returns `false` — which it never does on first call. Callers cannot distinguish "tap is running" from "tap thread was spawned but immediately failed". The `WR-04` log guard at line 555 in `start_observing` is therefore dead code.

**Fix:** Return `true` from the outer function remains correct for the idempotency contract, but the log message should be moved inside the spawned thread so failures are always surfaced regardless of the caller:
```rust
Err(_) => {
    eprintln!("[Observer] CGEventTap creation failed — Accessibility permissions may not be granted");
    TAP_INITIALIZED.store(false, Ordering::SeqCst);
}
```
The calling site should not rely on the return value to infer tap health.

---

### WR-04: `HMODULE` imported but unused in Windows module

**File:** `src-tauri/src/platform/windows/mod.rs:388`

**Issue:** `HMODULE` is imported from `windows::Win32::Foundation` but never referenced. Rust will emit an unused import warning which should be kept clean per the project's `noUnusedLocals` policy (applied to TypeScript) and analogous Rust hygiene.

**Fix:** Remove it:
```rust
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
```

---

### WR-05: macOS `list_running_apps_impl` called from async Tokio task — may violate AppKit main-thread requirement

**File:** `src-tauri/src/ipc/mod.rs:123–136`, `src-tauri/src/platform/macos/observer.rs:170`

**Issue:** The `list_running_apps` IPC command is `async` and is dispatched on Tauri's Tokio thread pool. `list_running_apps_impl` calls `NSWorkspace::sharedWorkspace()` and iterates `runningApplications`. Apple's documentation states `NSWorkspace` itself is thread-safe and `runningApplications` is documented as safe to call from any thread, so this is likely safe in practice. However, the comment in the source says "Thread-safe" without citing the specific AppKit documentation. If Apple changes this guarantee in a future OS version, this call will silently cause undefined behavior.

The `start_observing` function also calls `NSWorkspace::sharedWorkspace()` and `notificationCenter` from the thread that `MacPlatformObserver` is set up on, which in the current Tauri setup is the main thread — that is correct. But `list_running_apps_impl` diverges from that pattern.

**Fix:** Wrap the AppKit call in a `dispatch_sync` on the main queue, or document with a citation to the specific Apple thread-safety guarantee so it can be audited on OS upgrades:
```rust
/// Thread-safe per Apple documentation:
/// - NSWorkspace.runningApplications: thread-safe per NSWorkspace.h header annotation
/// - No UI operations performed (read-only property access)
```

---

## Info

### IN-01: `navigator.platform` is deprecated and defined at two separate call sites

**File:** `src/keymap.ts:3`, `src/App.tsx:116`

**Issue:** `navigator.platform` is deprecated in the Web Platform specification and may be removed from future WebView implementations. It is also duplicated — `keymap.ts` computes `IS_MACOS` at module scope, and `App.tsx` recomputes it inside the `App` function. They use identical logic so cannot diverge currently, but the duplication is fragile.

**Fix:** Export `IS_MACOS` from `keymap.ts` and import it in `App.tsx`. Consider migrating to `navigator.userAgent.includes("Mac")` or a Tauri OS detection API (`@tauri-apps/plugin-os`) for a non-deprecated signal.

---

### IN-02: macOS CGKeyCode table missing several common keys

**File:** `src/keymap.ts:10–78`

**Issue:** The `DOM_KEYCODE_TO_CGKEYCODE` table omits several common keys users may want as trigger keys: `CapsLock` (57), `Delete`/`ForwardDelete` (117), numpad keys (none of Numpad0–9 are present), and punctuation keys like `BracketLeft` (33), `BracketRight` (30), `Semicolon` (41), `Quote` (39), `Backquote` (50), `Backslash` (42), `Comma` (43), `Period` (47), `Slash` (44), `Minus` (27), `Equal` (24). Pressing these falls through to the `?? 0` default, silently mapping to `KeyA` (CR-03 above).

**Fix:** Add the missing entries. The CGKeyCode values are already present in `CGKEYCODE_TO_NAME` (reverse map) — they just need to be wired into `DOM_KEYCODE_TO_CGKEYCODE` with the correct `e.code` string keys.

---

_Reviewed: 2026-05-30T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
