# Phase 7: carry-work-platform-ci-safety - Pattern Map

**Mapped:** 2026-06-17
**Files analyzed:** 8
**Analogs found:** 8 / 8

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src-tauri/src/platform/macos/mod.rs` (add `check_input_monitoring`) | platform-utility | probe (synchronous CGEventTap) | `check_accessibility_permissions(false)` in same file (lines 19–36) | exact |
| `src-tauri/src/platform/macos/observer.rs` (SAFE-04 verification) | platform-module | flush (drain-then-dispatch) | `flush_held_inputs()` in same file (lines 80–138) | verified-correct |
| `src-tauri/src/platform/windows/mod.rs` (BUILD-01 imports, MEM-01 CloseHandle) | platform-module | resource-management (Win32) | `list_running_apps_impl` and `get_app_name_from_hwnd` in same file (lines 263–291) | exact |
| `src-tauri/src/ipc/mod.rs` (add `check_input_monitoring` command) | ipc-command | request-response | `check_accessibility` (lines 192–206) | exact |
| `src-tauri/src/lib.rs` (register new IPC) | app-bootstrap | command-list | `invoke_handler!` macro block (lines 83–103) | exact |
| `src-tauri/src/persistence.rs` (was_ever_granted flag helpers) | persistence-utility | file-I/O (sentinel file) | `ProfileManager::from_app_handle` (lines 66–80) — `app_data_dir()` resolution | exact |
| `src/App.tsx` (PermissionsCard, polling, banner, IM handler) | component | event-driven (frontend) | Existing accessibility card + `handleRequestAccess` + 3s `setInterval` poll + `state-changed` listener (lines 158–226, 488–535) | exact |
| `.github/workflows/release.yml` (CI-03/04/05) | ci-config | CI-pipeline | The file itself (60 lines, all in one block) | direct-edit |

## Pattern Assignments

### `src-tauri/src/platform/macos/mod.rs` — add `check_input_monitoring()` (COMPAT-04)

**Analog:** Same file, `check_accessibility_permissions(prompt: bool)` (lines 19–68).

**Imports pattern** (lines 1–6): No new imports needed — `core_graphics::event::*` items are imported inside the function body to keep them scoped (same scoping discipline used in the existing probe).

**Probe pattern — copy from `check_accessibility_permissions(false)`** (lines 19–36):
```rust
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
        return probe.is_ok();
    }
    // ... prompt branch uses AXIsProcessTrustedWithOptions via CFDictionary ...
}
```

**New function to add** (mirrors the probe branch; `CGEventTap` with `ListenOnly` exercises `kTCCServiceListenEvent`):
```rust
/// Check whether this process has Input Monitoring permissions.
/// Returns the current status without prompting — same probe pattern as the
/// silent Accessibility check above. Probe is scoped to this function and
/// does NOT set TAP_INITIALIZED.
/// [RESEARCH §D-05]: Required since macOS 10.15 Catalina; subsumed by
/// Accessibility when both are needed, but a separate probe is required
/// because the CGEventTap ListenOnly option is the only reliable signal
/// independent of the stale AXIsProcessTrusted() cache.
pub fn check_input_monitoring() -> bool {
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };
    let probe = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::TailAppendEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::MouseMoved],
        |_, _, _| None,
    );
    probe.is_ok()
}
```

**Critical safety note** (preserve verbatim from existing comment, lines 24–27): The new function MUST NOT touch `TAP_INITIALIZED` or `TAP_STARTING`. Probe is dropped at function return. The pre-existing `// @safety-officer:` comment block documents this discipline and must be replicated.

---

### `src-tauri/src/platform/macos/observer.rs` — SAFE-04 verification only (no code change)

**Analog:** `flush_held_inputs()` (lines 80–138) — the **exact pattern SAFE-04 requires is already in place**.

**Verified-correct pattern** (lines 80–138):
```rust
pub fn flush_held_inputs() {
    // @safety-officer: CR-01 — drain registry into a local Vec BEFORE releasing
    // the lock, then post CGEvents outside the lock.  Posting a CGEvent while
    // holding the REGISTRY mutex causes a re-entrant deadlock: the CGEventTap
    // callback fires synchronously and immediately tries to acquire the same mutex.
    let inputs_to_flush: Vec<ActiveInput> = {
        let mut reg = get_registry().lock().unwrap();
        reg.drain().collect()
    };
    // Lock released here — safe to post CGEvents.
    if let Ok(source) = core_graphics::event_source::CGEventSource::new(
        core_graphics::event_source::CGEventSourceStateID::HIDSystemState,
    ) {
        // ... CGEvent dispatch follows outside the lock ...
    }
}
```

**Same pattern duplicated at emergency-stop** (lines 351–357):
```rust
// @safety-officer: CR-01 — drain registry into a local Vec BEFORE
// releasing the lock.  Posting CGEvents while holding the REGISTRY mutex
// causes a re-entrant deadlock (the tap callback re-acquires the mutex).
let inputs_to_release: Vec<ActiveInput> = {
    let mut reg = get_registry().lock().unwrap();
    reg.drain().collect()
};
// Lock released — safe to post CGEvents.
```

**Verification action for planner:** Open `observer.rs:80–138` and confirm the drain-before-release block is present. The `// @safety-officer: CR-01` comment is the canonical marker. If present → mark SAFE-04 verified, no edit. If absent → scheduling mistake.

**Plan's role for SAFE-04:** Verification-only. No code edits to `flush_held_inputs`. The "implemented during Phase 6" finding in RESEARCH.md §SAFE-04 Status is the rationale.

---

### `src-tauri/src/platform/windows/mod.rs` — BUILD-01 + MEM-01

**Analog (MEM-01):** `get_app_name_from_hwnd` in same file (lines 263–291) — the function being patched.

**Current `get_app_name_from_hwnd` body** (lines 263–291):
```rust
#[cfg(target_os = "windows")]
unsafe fn get_app_name_from_hwnd(hwnd: HWND) -> Option<String> {
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    if hwnd == HWND::default() {
        return None;
    }

    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    if process_id == 0 {
        return None;
    }

    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
    let mut buf = [0u16; 260];
    let mut len = buf.len() as u32;
    QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_FORMAT(0),
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut len,
    )
    .ok()?;

    Some(String::from_utf16_lossy(&buf[..len as usize]))
}
```

**MEM-01 fix — pattern to apply** (CloseHandle on both success and early-return paths):
```rust
use windows::Win32::Foundation::CloseHandle;  // ADD to imports

let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
let mut buf = [0u16; 260];
let mut len = buf.len() as u32;
let query_result = QueryFullProcessImageNameW(
    handle,
    PROCESS_NAME_FORMAT(0),
    windows::core::PWSTR(buf.as_mut_ptr()),
    &mut len,
);
unsafe { CloseHandle(handle); }  // Must close BEFORE the `?` propagates
query_result.ok()?;
Some(String::from_utf16_lossy(&buf[..len as usize]))
```

**Critical pattern note (RESEARCH §Pitfall 3):** `CloseHandle` MUST run before `?` on `QueryFullProcessImageNameW`. The `let query_result =` rebinding delays the `?` so the close can fire first. The two early returns above (HWND default, process_id == 0) never reach `OpenProcess`, so they don't need `CloseHandle`.

**BUILD-01 fix — unused-import scan target:**

Current top-of-file imports (lines 10–21):
```rust
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_TYPE, KEYBDINPUT, KEYEVENTF_KEYUP, MOUSEEVENTF_ABSOLUTE,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT,
};

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
```

Mid-file imports (lines 384–401):
```rust
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use uuid::Uuid;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{
    SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_SHIFT};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
};
```

**Inner `enum_callback` re-imports** (lines 306–307 — DO NOT remove, these are local to the callback):
```rust
unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextW, IsWindowVisible};
    // ...uses both...
}
```

**BUILD-01 disposition by item (from RESEARCH §Pitfall 4 + code audit):**

| Symbol | Decision | Reason |
|--------|----------|--------|
| `HMODULE` (line 389) | **VERIFY with `cargo build --target x86_64-pc-windows-msvc`** | `SetWindowsHookExW` is called with `None` for the module handle (line 487), which may or may not require the import. CONTEXT.md lists it as removable; RESEARCH.md marks this LOW confidence. Compiler is source of truth. |
| `HHOOK` (line 399) | **KEEP** | Used at line 518: `UnhookWindowsHookEx(hook.unwrap())` — `hook` is `Result<HHOOK, _>`. |
| `GetWindowTextW` (line 301, top-level) | **REMOVE from top-level** | Already re-imported inside `enum_callback` (line 307) where it's actually used. Top-level import is redundant. |
| `IsWindowVisible` (line 301, top-level) | **REMOVE from top-level** | Already re-imported inside `enum_callback` (line 307) where it's actually used. Top-level import is redundant. |
| `TranslateMessage` (line 398) | **KEEP, suppress `must_use` warning** | Used at line 514. Wrap the call site: `let _ = TranslateMessage(&msg);` to silence the `must_use` lint. |

**Recommended build-verify command** (RESEARCH §Pitfall 4): `cargo build --target x86_64-pc-windows-msvc 2>&1 | grep "^warning"` — use the actual compiler warning list as the source of truth, not CONTEXT.md's enumeration.

---

### `src-tauri/src/ipc/mod.rs` — add `check_input_monitoring` command (COMPAT-04)

**Analog:** `check_accessibility` command (lines 192–206) — exact pattern, no `prompt` variant needed.

**Existing pattern** (lines 192–206):
```rust
/// Silent check: returns current accessibility status without prompting.
///
/// D-03: arms CGEventTap on post-launch grant (idempotent via TAP_INITIALIZED AtomicBool guard).
/// Called by the frontend 3s poll loop — if the user granted access directly in System Settings
/// (without clicking Request Access), the next poll will arm the tap without requiring a restart.
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

**New command to add — copy the structure:**
```rust
/// Silent check: returns current Input Monitoring status without prompting.
///
/// D-04: paired with the 3s accessibility poll in App.tsx. Same dual-platform
/// stub pattern as `check_accessibility` — returns `true` on non-macOS because
/// the permission has no equivalent on Windows/Linux.
#[command]
pub async fn check_input_monitoring() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(crate::platform::macos::check_input_monitoring())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}
```

**Why no `initialize_tap()` call here:** Input Monitoring is for hotkey observation only. `initialize_tap()` is the Accessibility-tap initializer — `check_input_monitoring` is a passive probe and does not need to arm anything. The frontend treats Input Monitoring as advisory (D-06, UI-SPEC §Polling).

**`request_input_monitoring` not needed:** Per D-03, the frontend opens the System Settings pane via `tauri-plugin-opener`. No `request_*` IPC is required for Input Monitoring (unlike Accessibility, which has `request_accessibility` for the system dialog).

---

### `src-tauri/src/lib.rs` — register new IPC handler

**Analog:** `invoke_handler!` macro block (lines 83–103) — direct append.

**Current block** (lines 83–103):
```rust
.invoke_handler(tauri::generate_handler![
    ipc::add_macro,
    ipc::remove_macro,
    ipc::set_macro_enabled,
    ipc::set_macro_target_app,
    ipc::get_state,
    ipc::get_active_app,
    ipc::bind_hotkey,
    ipc::unbind_hotkey,
    ipc::toggle_engine,
    ipc::request_accessibility,
    ipc::check_accessibility,
    ipc::set_macro_sequence,
    ipc::update_step_interval,
    ipc::save_profile,
    ipc::load_profile,
    ipc::delete_profile,
    ipc::list_profiles,
    ipc::list_running_apps,
    ipc::set_macro_trigger_key,
])
```

**Edit pattern:** Add `ipc::check_input_monitoring,` (and any other new commands) to the macro list. Order is not significant but matches the `ipc/mod.rs` file order convention.

---

### `src-tauri/src/persistence.rs` — was_ever_granted flag helpers (COMPAT-05)

**Analog:** `ProfileManager::from_app_handle` (lines 66–80) — exact `app_data_dir()` resolution pattern.

**Existing app_data_dir pattern** (lines 66–80):
```rust
pub fn from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self, String> {
    use tauri::Manager;
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

    let profiles_dir = app_data.join("profiles");

    // Create directory synchronously during setup (only runs once).
    std::fs::create_dir_all(&profiles_dir)
        .map_err(|e| format!("Failed to create profiles dir: {}", e))?;

    Ok(Self { profiles_dir })
}
```

**New helpers to add** (do not need to be on `ProfileManager`; module-level functions are fine, matching the `crate::platform::macos::observer` style of small free functions):
```rust
// In persistence.rs (module-level, pub(crate) so ipc and lib can call)

use tauri::Manager;  // already in scope within from_app_handle; pull up to module scope

const TCC_GRANTED_FLAG_FILENAME: &str = "tcc_granted.flag";

/// Path of the was-ever-granted sentinel file. Lives directly in app_data_dir
/// (not the `profiles/` subdir) so it survives profile operations and is
/// discoverable alongside other top-level state.
pub fn tcc_granted_flag_path(app_handle: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app_handle
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join(TCC_GRANTED_FLAG_FILENAME))
}

/// Write the sentinel file. Best-effort; failures are silently dropped because
/// the flag is advisory (TCC flag tampering is not a security boundary — see
/// RESEARCH §Security Domain). Called on first successful Accessibility grant.
pub fn write_tcc_granted_flag(app_handle: &tauri::AppHandle) {
    if let Some(path) = tcc_granted_flag_path(app_handle) {
        let _ = std::fs::write(&path, b"1");
    }
}

/// Returns true if the sentinel file exists. Read on startup before
/// `check_accessibility` so the frontend can show the identity-change copy
/// in the same frame as the permission status.
pub fn tcc_granted_flag_exists(app_handle: &tauri::AppHandle) -> bool {
    tcc_granted_flag_path(app_handle)
        .map(|p| p.exists())
        .unwrap_or(false)
}
```

**Note on platform scope:** Per D-10 the flag is unified (covers both Accessibility and Input Monitoring). It is **always written** when Accessibility is first granted — this is sufficient because TCC invalidates both grants together on identity change.

**Where to wire the write** (RESEARCH §Pitfall 6 + §Code Examples): In the `check_accessibility` IPC handler (`ipc/mod.rs:192-206`), after `granted` is true and **before** `Ok(granted)` returns. Order:
1. `let granted = crate::platform::macos::check_accessibility_permissions(false);`
2. `if granted { crate::platform::macos::observer::initialize_tap(); crate::persistence::write_tcc_granted_flag(&app_handle); }` — requires capturing `app_handle` in the `#[command]` signature (Tauri injects via `State` or direct `app.handle()` call). Verify exact signature in RESEARCH §Code Examples.

**Where to wire the read** (`lib.rs` setup hook, around line 67): Read the flag synchronously at app startup so the `get_tcc_identity_status` IPC command can return the right value on the first frontend poll. Pattern: spawn a small init task that reads the flag once into a `OnceLock<bool>` and exposes a getter; the new IPC command reads that getter.

---

### `src/App.tsx` — PermissionsCard, polling, banner, IM handler (COMPAT-04, COMPAT-05, ERR-01)

Three distinct changes share one file. Each change has a tight analog already in the file.

#### 7a. COMPAT-04 — 3s polling extension

**Analog:** Existing `setInterval` poll at lines 168–182.

**Existing pattern** (lines 168–182):
```typescript
// Poll accessibility every 3s
createEffect(() => {
  const interval = setInterval(async () => {
    try {
      const ok = await invoke<boolean>("check_accessibility");
      setAccessibility(ok);
      // Clear pending on any definitive response (granted or denied).
      // Only remain "pending" while the dialog is actually in-flight (null).
      clearPending();
    } catch (_) {
      /* ignore */
    }
  }, 3000);
  onCleanup(() => clearInterval(interval));
});
```

**Extended pattern (UI-SPEC §Polling):**
```typescript
// Poll accessibility AND input monitoring every 3s (D-04: single effect).
createEffect(() => {
  const interval = setInterval(async () => {
    try {
      const [a11y, im] = await Promise.all([
        invoke<boolean>("check_accessibility"),
        invoke<boolean>("check_input_monitoring"),
      ]);
      setAccessibility(a11y);
      setInputMonitoring(im);
      clearPending();
    } catch (_) {
      /* ignore */
    }
  }, 3000);
  onCleanup(() => clearInterval(interval));
});
```

**Initial-fetch extension** (analog: lines 129–155). Add `invoke<boolean>("check_input_monitoring")` and `invoke<boolean>("get_tcc_identity_status")` to the `Promise.all` destructuring and to the initial setter block.

#### 7b. COMPAT-04 — `handleRequestInputMonitoringAccess`

**Analog:** `handleRequestAccess` at lines 208–226.

**Existing pattern** (lines 208–226):
```typescript
async function handleRequestAccess() {
  setAccessibilityPending(true);
  _pendingTimeoutId = setTimeout(() => {
    if (!requestAccessCancelled) setAccessibilityPending(false);
    _pendingTimeoutId = null;
  }, 30_000);
  try {
    const granted = await invoke<boolean>("request_accessibility");
    if (!requestAccessCancelled) {
      setAccessibility(granted);
      clearPending();
    }
  } catch (e) {
    if (!requestAccessCancelled) {
      console.error("Accessibility request failed:", e);
      clearPending();
    }
  }
}
```

**New handler (UI-SPEC §Grant button — uses `tauri-plugin-opener`'s `open()` directly, NOT `invoke`):**
```typescript
import { open } from "@tauri-apps/plugin-opener";  // ADD to imports (line 1-6 area)

async function handleRequestInputMonitoringAccess() {
  setInputMonitoringPending(true);
  // 30s timeout mirrors Accessibility flow
  _imPendingTimeoutId = setTimeout(() => {
    if (!requestAccessCancelled) setInputMonitoringPending(false);
    _imPendingTimeoutId = null;
  }, 30_000);
  try {
    await open("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent");
  } catch (e) {
    if (!requestAccessCancelled) {
      console.error("Failed to open Input Monitoring settings:", e);
      setInputMonitoringPending(false);
      _imPendingTimeoutId = null;
    }
  }
}
```

**Why `open()` from `@tauri-apps/plugin-opener`, not `invoke("open_url", ...)`:** The plugin is already in `package.json` (line 17) and registered in `lib.rs:21` (`.plugin(tauri_plugin_opener::init())`). It's the documented Tauri 2 pattern. No custom IPC is required for opening URLs.

**No `IS_MACOS` gate** needed around the URL itself — on non-macOS the URL is harmless (a no-op system settings link) and the polling returns `true` for Input Monitoring there anyway. But the new "Grant Access" button should be conditionally hidden on non-macOS using the existing `IS_MACOS` constant from line 125.

#### 7c. COMPAT-04 / COMPAT-05 — new PermissionsCard (replaces solo Accessibility card)

**Analog:** Existing solo Accessibility card at lines 488–535.

**Existing structure to REPLACE** (lines 488–535):
```tsx
{/* Accessibility Status */}
<div class="glass-card flex-1 p-4">
  <div class="flex items-center justify-between mb-2">
    <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
      Security
    </span>
    <Show
      when={accessibility() === true}
      fallback={ /* denied or pending dot+label */ }
    >
      <div class="flex items-center gap-1.5">
        <div class="w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)]" />
        <span class="text-[11px] text-success font-medium">Granted</span>
      </div>
    </Show>
  </div>
  <p class="text-xs text-text-dim">Accessibility Permissions</p>
  <Show when={accessibility() === false && !accessibilityPending()}>
    <button
      id="btn-request-access"
      onClick={handleRequestAccess}
      class="mt-3 w-full py-1.5 rounded-lg bg-accent/10 border border-accent/30 text-accent text-xs font-medium
             hover:bg-accent/20 hover:border-accent/50 transition-all duration-200 cursor-pointer"
    >
      Grant Access
    </button>
  </Show>
</div>
```

**Reusable patterns to copy into the new card:**
- **Status dot+label** (lines 493–522): The 3-way `<Show>`/`fallback` for granted/denied/pending is the canonical pattern. The Input Monitoring row mirrors it with `"Warning"` instead of `"Denied"` per UI-SPEC §Input Monitoring soft warning.
- **Grant Access button** (lines 526–533): The `bg-accent/10 border border-accent/30 text-accent` ghost button. The new Input Monitoring row uses the exact same classes.
- **Section header** (line 490): `text-xs font-medium text-text-muted uppercase tracking-wider` — UI-SPEC §Permissions section header uses this same pattern with text "Permissions".
- **`<Show when={...}>`** (line 525): The `false && !accessibilityPending()` guard pattern for the button visibility is the canonical "show when not yet granted" check.

**New card structure (UI-SPEC §Component Inventory):**
```tsx
{/* Permissions — combined Accessibility + Input Monitoring */}
<div class="glass-card flex-1 p-4">
  <div class="flex items-center justify-between mb-3">
    <span class="text-xs font-medium text-text-muted uppercase tracking-wider">
      Permissions
    </span>
  </div>
  <div class="flex flex-col gap-3">
    {/* Accessibility row — copy of the existing pattern, header text changes */}
    {/* Input Monitoring row — new, mirrors Accessibility row, soft warning */}
  </div>
</div>
```

**TCC identity change copy swap (COMPAT-05, UI-SPEC §TCC identity change copy):** The Accessibility row's subtitle swaps from `"Required for input injection"` to `"AutoMux was updated — Accessibility needs to be re-added in System Settings."` when `tccIdentityChanged() === true` AND `accessibility() === false`. Pattern: `<Show when={...} fallback={<DefaultSubtitle/>}><IdentityChangeSubtitle/></Show>`.

#### 7d. ERR-01 — `auto-save-error` listener + banner

**Analog:** Existing `state-changed` listener at lines 158–166.

**Existing pattern** (lines 158–166):
```typescript
// Real-time state listener
createEffect(() => {
  const unlisten = listen<AppState>("state-changed", (event) => {
    setState(event.payload);
    setActiveApp(event.payload.active_app);
  });
  onCleanup(() => {
    unlisten.then((fn) => fn());
  });
});
```

**New listener** (UI-SPEC §Auto-save error banner):
```typescript
// Auto-save error listener (D-11) — shows persistent banner until dismissed
const [saveError, setSaveError] = createSignal(false);

createEffect(() => {
  const unlisten = listen<string>("auto-save-error", (_event) => {
    // D-13: do NOT surface event payload string — show fixed user-friendly copy
    setSaveError(true);
  });
  onCleanup(() => {
    unlisten.then((fn) => fn());
  });
});
```

**Banner JSX** (UI-SPEC §New `AutoSaveErrorBanner`, placed at top of macro list):
```tsx
<Show when={saveError()}>
  <div
    id="auto-save-error-banner"
    class="bg-warning/10 border border-warning/20 rounded-lg p-3 flex items-center gap-3"
  >
    <span class="text-warning text-base">⚠</span>
    <div class="flex-1">
      <p class="text-xs font-medium text-warning">Save failed</p>
      <p class="text-[11px] text-text-dim">
        Your changes are not being saved. Check available disk space and file permissions.
      </p>
    </div>
    <button
      onClick={() => setSaveError(false)}
      class="text-[11px] text-text-muted hover:text-text-main transition-colors duration-200 cursor-pointer"
    >
      Dismiss
    </button>
  </div>
</Show>
```

**Pattern reuse from `state-changed` listener:** The `onCleanup(() => unlisten.then(fn => fn()))` shape is the canonical teardown — copy verbatim. The `listen<T>` typed signature is reused.

**Pattern reuse from existing banners/pills:** The `bg-warning/10 border border-warning/20` palette mirrors the existing warning-status dot color tokens (`--color-warning` / `--color-warning-glow` in `App.css:20-21`).

#### 7e. New signal declarations

**Analog:** Existing accessibility signals at lines 83–84.

**Existing pattern** (lines 83–84):
```typescript
const [accessibility, setAccessibility] = createSignal<boolean | null>(null);
const [accessibilityPending, setAccessibilityPending] = createSignal(false);
```

**Add four new signals** (UI-SPEC §New signals):
```typescript
const [inputMonitoring, setInputMonitoring] = createSignal<boolean | null>(null);
const [inputMonitoringPending, setInputMonitoringPending] = createSignal(false);
const [saveError, setSaveError] = createSignal(false);
const [tccIdentityChanged, setTccIdentityChanged] = createSignal(false);
```

**Plus a timeout ref** to mirror `_pendingTimeoutId` for Input Monitoring's 30s pending clear.

---

### `.github/workflows/release.yml` — CI-03 / CI-04 / CI-05

**No analog — this file is the only release workflow in the repo. All three changes are direct edits.**

**Current full file** (60 lines) — see `release.yml`. Relevant sections:

**CI-03 (D-15) — updater signature step:** Currently absent from the file. The signature-upload step was referenced in the project's prior workflow but was never committed (or was removed in a prior cleanup). **Action: do nothing if the step is not present; if a future scan finds it, remove the entire step block and add a comment:**
```yaml
# Updater signature upload removed — re-add when real signing is configured (v3 scope)
```

**CI-04 (D-16) — line 28, change `npm install` → `npm ci`:**
```yaml
# BEFORE:
      - name: Install Frontend Dependencies
        run: npm install

# AFTER:
      - name: Install Frontend Dependencies
        run: npm ci
```

**CI-05 (D-17) — add `id: tauri` to the tauri-action step and reference the output in upload steps.** The current `release.yml` (lines 35–45) does not use a hardcoded glob for upload (the tauri-action handles uploads internally via `releaseDraft: true`). However, if any explicit upload step exists or is added, use the official output:
```yaml
# Step that needs the id:
      - name: Build and Release Tauri App
        id: tauri          # REQUIRED — output variable name
        uses: tauri-apps/tauri-action@84b9d35b5fc46c1e45415bdb6144030364f7ebc5  # v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          # ...

# Reference in any downstream upload step:
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: automux-${{ matrix.platform }}
          path: ${{ steps.tauri.outputs.artifactPaths }}   # NOT artifact-paths (kebab-case is wrong)
```

**Verification command** (RESEARCH §Pitfall 5): `grep -c "artifact-paths" .github/workflows/release.yml` should return 0. `grep "steps.tauri.outputs.artifactPaths" .github/workflows/release.yml` should return ≥ 1 if any explicit upload step is added.

**Critical pitfall** (RESEARCH §Pitfall 5): The output variable name is `artifactPaths` (camelCase), not `artifact-paths` (kebab-case). The step `id` must be `tauri`.

---

## Shared Patterns

### 1. `#[cfg(target_os = "macos")]` / `#[cfg(not(target_os = "macos"))]` for cross-platform stubs

**Source:** `src-tauri/src/ipc/mod.rs:170-205` (used in `request_accessibility` and `check_accessibility`).
**Apply to:** New `check_input_monitoring` IPC command.

```rust
#[cfg(target_os = "macos")]
{
    Ok(crate::platform::macos::check_input_monitoring())
}
#[cfg(not(target_os = "macos"))]
{
    Ok(true)
}
```

This is the canonical pattern for "macOS-specific probe, no-op true elsewhere" — used twice in the IPC layer already and is the explicit guidance in the RESEARCH D-05 Resolution.

### 2. `app_data_dir()` resolution via `tauri::Manager`

**Source:** `src-tauri/src/persistence.rs:66-71`.
**Apply to:** `tcc_granted_flag_path`, `write_tcc_granted_flag`, `tcc_granted_flag_exists` helpers.

```rust
use tauri::Manager;
let app_data = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;
```

The `app_data_dir()` path is platform-sandboxed (V5 Input Validation per RESEARCH §Security Domain). ProfileManager already uses it; the new flag helpers reuse the same call.

### 3. `if let Ok(event) = ...` — silent drop on platform errors

**Source:** `src-tauri/src/platform/macos/observer.rs:97-105` (CGEvent key up event), `input.rs` (input injection), `src-tauri/src/state/mod.rs:499` (`let _ = self.app_handle.emit(...)`).
**Apply to:** The new flag file write (`let _ = std::fs::write(...)`) and any new CGEvent probes.

```rust
if let Ok(up_event) = core_graphics::event::CGEvent::new_keyboard_event(source.clone(), *k, false) {
    // ...
}
```

CGEvent creation failures, emit failures, and flag write failures are all silently dropped. This is the established convention — the planner must not introduce `unwrap()` or `?` on platform-emitted values.

### 4. SolidJS `onCleanup` paired with `listen()` subscriptions

**Source:** `src/App.tsx:158-166` (`state-changed` listener).
**Apply to:** New `auto-save-error` listener (ERR-01).

```typescript
createEffect(() => {
  const unlisten = listen<T>("event-name", (event) => { /* ... */ });
  onCleanup(() => {
    unlisten.then((fn) => fn());
  });
});
```

The `unlisten.then((fn) => fn())` shape is the canonical teardown for `listen()` in this codebase. NEVER store a `listen()` call in a `createSignal` — it must be inside a `createEffect` so the cleanup fires on HMR remount.

### 5. `setInterval` inside `createEffect` with `onCleanup(clearInterval)`

**Source:** `src/App.tsx:168-182` (3s accessibility poll).
**Apply to:** Extended 3s accessibility+IM poll.

```typescript
createEffect(() => {
  const interval = setInterval(async () => { /* ... */ }, 3000);
  onCleanup(() => clearInterval(interval));
});
```

### 6. `// @safety-officer:` and `// @architect:` documentation comment style

**Source:** Throughout `observer.rs`, `persistence.rs`, `state/mod.rs`, `windows/mod.rs`.
**Apply to:** All new code that introduces lock-ordering, state-mutation, or platform-interaction invariants.

Format: triple-slash `// @safety-officer:` for safety-critical invariants, `// @architect:` for structural intent, `// @performance-tuner:` for perf constraints, `// @scheduler-agent:` for scheduling constraints. See `CONVENTIONS.md` for the full convention.

### 7. CGEventTap probe with `ListenOnly` option

**Source:** `src-tauri/src/platform/macos/mod.rs:21-36`.
**Apply to:** New `check_input_monitoring()` function.

```rust
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
};
let probe = CGEventTap::new(
    CGEventTapLocation::HID,
    CGEventTapPlacement::TailAppendEventTap,
    CGEventTapOptions::ListenOnly,
    vec![CGEventType::MouseMoved],
    |_, _, _| None,
);
probe.is_ok()
```

`ListenOnly` is the key — it requires `kTCCServiceListenEvent` (Input Monitoring) on macOS, making it the correct probe to distinguish IM denial from Accessibility denial. The `TailAppendEventTap` placement matches the existing accessibility probe exactly (consistency discipline).

### 8. Conditional UI render with `<Show when={...} fallback={...}>`

**Source:** `src/App.tsx:493-535` (accessibility status dot+label), `488-558` (status row layout).
**Apply to:** PermissionsCard status dots, Grant button visibility, banner visibility, identity-change copy swap.

```tsx
<Show when={accessibility() === true} fallback={<DeniedOrPending/>}>
  <GrantedDot/>
</Show>
```

The 3-way nested `<Show when={...} fallback={<Show when={pending} fallback={denied}/>}` pattern at lines 493-514 is the canonical "granted / pending / denied" tri-state.

---

## No Analog Found

None. All 8 files in this phase have a strong, file-internal or near-internal analog in the existing codebase. The phase is a hardening pass — every fix reuses an existing pattern.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | — | — | — |

---

## Verification Commands (per requirement)

From RESEARCH §Validation Architecture, the verification commands for the planner to use:

| Req | Command |
|-----|---------|
| BUILD-01 | `cargo build --target x86_64-pc-windows-msvc 2>&1 \| grep "^warning"` (expect zero matches for `unused_imports` / `unused_must_use` on the lines affected) |
| MEM-01 | Code review of `get_app_name_from_hwnd` (lines 263–291) — `CloseHandle(handle)` must appear on every OpenProcess success path |
| CI-03 | `grep -c "sig.*upload\|Signature not found" .github/workflows/release.yml` → 0 |
| CI-04 | `grep "npm ci" .github/workflows/release.yml` → ≥ 1 match |
| CI-05 | `grep "steps.tauri.outputs.artifactPaths" .github/workflows/release.yml` → ≥ 1 match (camelCase, not kebab) |
| SAFE-04 | `grep -A5 "inputs_to_flush\|inputs_to_release" src-tauri/src/platform/macos/observer.rs` → must show drain-block before `let pos = ...` or CGEvent dispatch |
| ERR-01 | Manual smoke: trigger save error, observe banner at top of macro list, click "Dismiss" — banner hides, no console error |
| COMPAT-04 | Manual smoke: revoke Input Monitoring in System Settings, observe "Warning" status in Input Monitoring row of Permissions card, click Grant Access → settings pane opens |
| COMPAT-05 | Manual smoke: `rm ~/Library/Application\ Support/com.alvaro.automux/tcc_granted.flag`; revoke Accessibility; relaunch; observe "AutoMux was updated" subtitle in Accessibility row |

---

## Metadata

**Analog search scope:** `src-tauri/src/`, `src/`, `.github/workflows/`
**Files scanned:** 11 (observer.rs, mod.rs, input.rs for macOS; mod.rs for Windows; mod.rs for ipc; mod.rs + lib.rs for state; persistence.rs; App.tsx; release.yml; package.json; App.css; Cargo.toml)
**Pattern extraction date:** 2026-06-17
**Pre-planner handoff:** All 9 requirements (BUILD-01, MEM-01, CI-03, CI-04, CI-05, SAFE-04, ERR-01, COMPAT-04, COMPAT-05) have actionable pattern assignments. SAFE-04 is verification-only.
