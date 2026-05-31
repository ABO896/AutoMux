# Phase 3: Macro Setup UX - Pattern Map

**Mapped:** 2026-05-30
**Files analyzed:** 6 (2 new, 4 modified)
**Analogs found:** 6 / 6

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/App.tsx` (key capture widget) | component | event-driven | `src/App.tsx` lines 98–148 (createEffect + onCleanup listener lifecycle) | exact |
| `src/App.tsx` (process picker) | component | request-response | `src/App.tsx` lines 472–482 (existing `<select>` + invoke pattern) | exact |
| `src/App.tsx` (card inline edit state) | component | event-driven | `src/App.tsx` lines 559–620 (For loop + macro card state) | exact |
| `src/keymap.ts` (new static lookup) | utility | transform | none — pure static data, no runtime analog | no analog |
| `src-tauri/src/ipc/mod.rs` (`list_running_apps`) | controller | request-response | `src-tauri/src/ipc/mod.rs` lines 264–269 (`list_profiles`) | exact |
| `src-tauri/src/state/mod.rs` (`SetMacroTriggerKey` Intent + handler) | model | CRUD | `src-tauri/src/state/mod.rs` lines 125–148 (Intent enum + `SetMacroTargetApp`) | exact |
| `src-tauri/src/platform/macos/observer.rs` (runningApplications impl) | service | request-response | `src-tauri/src/platform/macos/observer.rs` lines 465–525 (NSWorkspace usage) | exact |
| `src-tauri/src/platform/windows/mod.rs` (EnumWindows impl) | service | request-response | `src-tauri/src/platform/windows/mod.rs` lines 262–291 (`get_app_name_from_hwnd`) | exact |
| `src-tauri/src/lib.rs` (handler registration) | config | request-response | `src-tauri/src/lib.rs` lines 83–101 (existing registration block) | exact |

---

## Pattern Assignments

### `src/App.tsx` — Key Capture Widget (event-driven component)

**Analog:** `src/App.tsx` lines 98–148 (createEffect + onCleanup event listener lifecycle)

**Signal declaration pattern** (lines 88–94 — new macro form state block):
```tsx
// Existing signal block in App() — add new capture state alongside these:
const [newMacroTriggerKey, setNewMacroTriggerKey] = createSignal("");
// New additions follow same pattern:
const [newMacroTriggerKeyCode, setNewMacroTriggerKeyCode] = createSignal<number | null>(null);
const [triggerKeyRecording, setTriggerKeyRecording] = createSignal(false);
// For card inline editing (app-level, not inside For loop):
const [editingCardId, setEditingCardId] = createSignal<string | null>(null);
const [editingField, setEditingField] = createSignal<"key" | "target" | null>(null);
```

**createEffect + onCleanup listener lifecycle pattern** (lines 127–148):
```tsx
// How the existing state-changed listener is structured — copy this for keydown capture:
createEffect(() => {
  const unlisten = listen<AppState>("state-changed", (event) => {
    setState(event.payload);
    setActiveApp(event.payload.active_app);
  });
  onCleanup(() => {
    unlisten.then((fn) => fn());
  });
});

// Poll pattern (lines 138–148) — shows setInterval + onCleanup:
createEffect(() => {
  const interval = setInterval(async () => {
    try {
      const ok = await invoke<boolean>("check_accessibility");
      setAccessibility(ok);
    } catch (_) {
      /* ignore */
    }
  }, 3000);
  onCleanup(() => clearInterval(interval));
});
```

**Key capture widget: startCapture function** — no existing exact analog; copy `document.addEventListener` pattern with `{ capture: true }` and explicit listener ref for cleanup:
```tsx
// Store listener ref at component/closure scope for onCleanup access
let _keyCaptureListener: ((e: KeyboardEvent) => void) | null = null;

function startCapture(onCommit: (code: number) => void) {
  // Remove any prior stale listener before attaching a new one (Pitfall 2)
  if (_keyCaptureListener) {
    document.removeEventListener("keydown", _keyCaptureListener, true);
    _keyCaptureListener = null;
  }
  setTriggerKeyRecording(true);

  function onKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      setTriggerKeyRecording(false);
      document.removeEventListener("keydown", onKeyDown, true);
      _keyCaptureListener = null;
      return;
    }
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
    // Use e.code as lookup key to avoid F12/arrow collision — see keymap.ts
    onCommit(domKeycodeToNative(e.code));
    setTriggerKeyRecording(false);
    document.removeEventListener("keydown", onKeyDown, true);
    _keyCaptureListener = null;
  }
  _keyCaptureListener = onKeyDown;
  document.addEventListener("keydown", onKeyDown, true);
}
// onCleanup call at component scope (inside App() function):
onCleanup(() => {
  if (_keyCaptureListener) {
    document.removeEventListener("keydown", _keyCaptureListener, true);
  }
});
```

**Key capture JSX — visual states** (replacing lines 515–523):
```tsx
// Idle state classes (from 03-UI-SPEC.md):
// No key: "bg-background border border-border rounded-lg px-3 py-2 text-sm text-text-dim w-40"
// Key set: "bg-background border border-border rounded-lg px-3 py-2 text-sm text-text-main w-40"
// Recording: "bg-background border border-accent rounded-lg px-3 py-2 text-sm text-accent shadow-[0_0_8px_var(--color-accent-glow)] w-40"

// Replaces lines 515–523 (in the flex gap-2 div alongside trigger mode select):
<div
  class={`rounded-lg px-3 py-2 text-sm w-40 cursor-pointer flex items-center justify-between
    ${triggerKeyRecording()
      ? "bg-background border border-accent text-accent shadow-[0_0_8px_var(--color-accent-glow)]"
      : newMacroTriggerKeyCode() !== null
        ? "bg-background border border-border text-text-main"
        : "bg-background border border-border text-text-dim"
    }`}
  onClick={() => startCapture((code) => setNewMacroTriggerKeyCode(code))}
>
  <span>
    {triggerKeyRecording()
      ? "Press a key…"
      : newMacroTriggerKeyCode() !== null
        ? resolveKeyName(newMacroTriggerKeyCode()!)
        : "Click to set key…"
    }
  </span>
  <Show when={triggerKeyRecording()}>
    <span
      class="text-text-dim hover:text-text-main ml-2"
      onClick={(e) => { e.stopPropagation(); setTriggerKeyRecording(false); }}
    >✕</span>
  </Show>
</div>
```

**handleCreateMacro integration** (lines 176–208 — update the trigger key parse):
```tsx
// OLD (lines 180–181):
const parsedKey = parseInt(newMacroTriggerKey());
const triggerKey = isNaN(parsedKey) ? null : parsedKey;

// NEW: reads from integer signal directly
const triggerKey = newMacroTriggerKeyCode();
// Also reset on submit (line 205):
setNewMacroTriggerKeyCode(null);
setTriggerKeyRecording(false);
```

---

### `src/App.tsx` — Process Picker `<select>` (request-response component)

**Analog:** `src/App.tsx` lines 472–482 (existing input-type `<select>`) and `handleToggleMacro` / `handleSaveProfile` for invoke pattern

**Existing `<select>` class pattern** (lines 505–514 — trigger mode selector, exact class string to copy):
```tsx
class="flex-1 bg-background border border-border rounded-lg px-3 py-2 text-sm
       focus:outline-none focus:border-accent/50 transition-colors text-text-main cursor-pointer"
```

**invoke + try/catch pattern** (lines 197–208):
```tsx
try {
  await invoke<string>("add_macro", { config });
  setShowNewMacro(false);
  // ...reset signals...
} catch (e) {
  console.error("Failed to create macro:", e);
}
```

**Process picker state signals** (add alongside existing signal block, lines 88–94):
```tsx
// Local to the picker context (or app-level if shared between form + cards)
const [apps, setApps] = createSignal<RunningApp[]>([]);
const [appsLoading, setAppsLoading] = createSignal(false);
const [appsError, setAppsError] = createSignal(false);
```

**handlePickerFocus — onFocus fetch pattern** (mirrors `refreshProfiles` at lines 226–233):
```tsx
// refreshProfiles (lines 226–233) is the canonical pattern for fetch-on-demand:
async function refreshProfiles() {
  try {
    const list = await invoke<ProfileSummary[]>("list_profiles");
    setProfiles(list);
  } catch (e) {
    console.error("Failed to refresh profiles:", e);
  }
}

// Copy this for process picker fetch:
async function handlePickerFocus() {
  setAppsLoading(true);
  setAppsError(false);
  try {
    const result = await invoke<RunningApp[]>("list_running_apps");
    setApps(result);
  } catch (_) {
    setAppsError(true);
  } finally {
    setAppsLoading(false);
  }
}
```

**Process picker JSX** (replaces lines 493–501 — target app `<input>`):
```tsx
// Mirror class string from lines 505–514 (trigger mode select):
<select
  id="select-macro-target"
  value={newMacroTarget()}
  onChange={(e) => setNewMacroTarget(e.currentTarget.value)}
  onFocus={handlePickerFocus}
  class="w-full bg-background border border-border rounded-lg px-3 py-2 text-sm
         focus:outline-none focus:border-accent/50 transition-colors text-text-main cursor-pointer"
>
  <option value="">🌐 Global (no target)</option>
  <Show when={appsLoading()}>
    <option disabled>Loading…</option>
  </Show>
  <Show when={appsError()}>
    <option disabled>Failed to load apps</option>
  </Show>
  <For each={apps()}>
    {(app) => (
      <option value={app.identifier}>
        {app.display_name} ({app.identifier})
      </option>
    )}
  </For>
</select>
```

**RunningApp type** (add to type section at top of App.tsx, alongside existing interfaces):
```tsx
// Add after ProfileData interface (line 46):
interface RunningApp {
  display_name: string;
  identifier: string;
}
```

---

### `src/App.tsx` — Macro Card Inline Edit (card badge click-to-edit)

**Analog:** `src/App.tsx` lines 559–620 (For loop macro card rendering), lines 573–580 (`data-active` toggle pattern for interactive card elements)

**Toggle pattern with data-active** (lines 573–580 — how existing card interactivity is done):
```tsx
<div
  class="toggle-track"
  data-active={macro.enabled}
  onClick={() => handleToggleMacro(macro.id, macro.enabled)}
  style={{ transform: "scale(0.8)" }}
>
  <div class="toggle-thumb" />
</div>
```

**Key badge (lines 595–598) — becomes clickable**:
```tsx
// CURRENT (lines 593–602):
<Show when={macro.trigger_key !== null}>
  <div class="flex items-center gap-1">
    <span class="px-1.5 py-0.5 rounded bg-surface-alt border border-border text-[10px] font-mono">
      Key {macro.trigger_key}
    </span>
    <span class="text-[10px] text-text-muted">({macro.trigger_mode})</span>
  </div>
</Show>

// NEW — add cursor-pointer, hover state, Show for edit mode:
// (editingCardId + editingField are app-level signals)
<Show when={macro.trigger_key !== null}>
  <div class="flex items-center gap-1">
    <Show
      when={editingCardId() === macro.id && editingField() === "key"}
      fallback={
        <span
          class="px-1.5 py-0.5 rounded bg-surface-alt border border-border text-[10px] font-mono cursor-pointer hover:border-accent/40"
          onClick={() => {
            setEditingCardId(macro.id);
            setEditingField("key");
            // startCapture will attach the keydown listener
          }}
        >
          {resolveKeyName(macro.trigger_key!)}
        </span>
      }
    >
      {/* inline recording widget */}
    </Show>
    <span class="text-[10px] text-text-muted">({macro.trigger_mode})</span>
  </div>
</Show>
```

**Target app inline edit** (lines 586–591 — Global/target display):
```tsx
// CURRENT (lines 586–591):
<div class="flex items-center gap-2">
  <span>🎯</span>
  <span class="font-mono">{macro.target_app || "Global"}</span>
</div>

// NEW — clickable target badge, inline <select> when editing:
<div class="flex items-center gap-2">
  <span>🎯</span>
  <Show
    when={editingCardId() === macro.id && editingField() === "target"}
    fallback={
      <span
        class="font-mono cursor-pointer hover:border-accent/40"
        onClick={() => { setEditingCardId(macro.id); setEditingField("target"); }}
      >
        {macro.target_app || "Global"}
      </span>
    }
  >
    {/* inline process picker <select> — same onFocus fetch pattern */}
  </Show>
</div>
```

**Card edit auto-commit for target app** (mirrors `handleToggleMacro` single-field invoke pattern, lines 211–216):
```tsx
async function handleCardSetTargetApp(id: string, target_app: string | null) {
  try {
    await invoke("set_macro_target_app", { id, target_app: target_app || null });
    setEditingCardId(null);
    setEditingField(null);
  } catch (e) {
    console.error("Card target update failed:", e);
  }
}
```

**Card edit auto-commit for hotkey** — macOS: unbind then rebind (Pitfall 3 in RESEARCH.md):
```tsx
async function handleCardSetTriggerKey(id: string, keycode: number) {
  try {
    await invoke("unbind_hotkey", { macro_id: id });         // clear old binding
    await invoke("bind_hotkey", { macro_id: id, keycode, modifiers: 0 });
    setEditingCardId(null);
    setEditingField(null);
  } catch (e) {
    console.error("Card hotkey update failed:", e);
  }
}
// Windows: call set_macro_trigger_key IPC (new command — see state/mod.rs section below)
```

---

### `src/keymap.ts` (new utility — no analog)

**Role:** utility, transform. **Data flow:** pure static lookup — no I/O, no async.

No analog exists in the codebase. Use RESEARCH.md Code Examples section directly.

**File structure** (follow `camelCase.ts` naming per CONVENTIONS.md):
```typescript
// src/keymap.ts
// Platform detection (synchronous, Tauri WebView safe):
const IS_MACOS = navigator.platform.toLowerCase().includes("mac");

// Two exported lookup maps — copy from RESEARCH.md Code Examples verbatim:
// DOM_KEYCODE_TO_CGKEYCODE: Record<number, number>  — DOM keyCode → CGKeyCode
// CGKEYCODE_TO_NAME: Record<number, string>          — CGKeyCode → display name
// DOM_KEYCODE_TO_VK: Record<number, number>           — DOM keyCode → VK code
// VK_TO_NAME: Record<number, string>                  — VK code → display name

// Single exported resolver used by App.tsx:
export function resolveKeyName(nativeCode: number): string {
  const nameMap = IS_MACOS ? CGKEYCODE_TO_NAME : VK_TO_NAME;
  return nameMap[nativeCode] ?? `Key ${nativeCode}`;
}

export function domKeycodeToNative(domKeyCode: number): number {
  const map = IS_MACOS ? DOM_KEYCODE_TO_CGKEYCODE : DOM_KEYCODE_TO_VK;
  return map[domKeyCode] ?? domKeyCode;
}
```

**Import in App.tsx** (follows import organization convention — third-party then local):
```tsx
// Add after line 5 (./App.css import):
import { resolveKeyName, domKeycodeToNative } from "./keymap";
```

---

### `src-tauri/src/ipc/mod.rs` — `list_running_apps` command

**Analog:** `src-tauri/src/ipc/mod.rs` lines 264–269 (`list_profiles`) for the no-State command shape; lines 77–93 (`bind_hotkey`) for the platform-conditional `#[cfg]` pattern.

**`list_profiles` shape** (lines 264–269 — closest match: no StateManager needed, returns Vec):
```rust
#[command]
pub async fn list_profiles(
    profile_mgr: State<'_, Arc<crate::persistence::ProfileManager>>,
) -> Result<Vec<crate::persistence::ProfileSummary>, String> {
    profile_mgr.list_profiles().await
}
```

**`bind_hotkey` platform-conditional pattern** (lines 77–93 — copy for `list_running_apps`):
```rust
#[command]
pub async fn bind_hotkey(
    _state: State<'_, StateManager>,
    _macro_id: Uuid,
    _keycode: u16,
    _modifiers: u64,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use crate::platform::macos::observer::{add_hotkey_binding, HotkeyAction, HotkeyBinding};
        add_hotkey_binding(HotkeyBinding { ... });
    }
    Ok(())
}
```

**New `list_running_apps` command** (add after `unbind_hotkey` at line 104):
```rust
// ── RunningApp type (serializable, no State dependency) ──────────
#[derive(Debug, Clone, Serialize)]
pub struct RunningApp {
    pub display_name: String,
    pub identifier: String,
}

/// List all user-facing running applications for the process picker.
/// Returns name + bundle ID on macOS; name + exe path on Windows.
#[command]
pub async fn list_running_apps() -> Result<Vec<RunningApp>, String> {
    #[cfg(target_os = "macos")]
    {
        // Implementation: NSWorkspace.runningApplications() — see macos/observer.rs section
    }
    #[cfg(target_os = "windows")]
    {
        // Implementation: EnumWindows + get_app_name_from_hwnd — see windows/mod.rs section
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Ok(vec![])
    }
}
```

**Imports to add at top of ipc/mod.rs** (mirrors existing serde import usage — Serialize already available via state types; add explicit use if needed):
```rust
// Already present via the crate: serde::Serialize is available
// No new imports needed — RunningApp uses the same derive path as existing types
```

---

### `src-tauri/src/state/mod.rs` — `SetMacroTriggerKey` Intent + handler

**Analog:** `src-tauri/src/state/mod.rs` lines 131 (`SetMacroTargetApp`) for the Intent variant; the StateActor handler for `SetMacroTargetApp` for the processing logic.

**Intent enum addition** (insert after line 131 `SetMacroTargetApp`):
```rust
// CURRENT line 131:
SetMacroTargetApp(Uuid, Option<String>),
// ADD after it:
SetMacroTriggerKey(Uuid, Option<u16>),
```

**IPC command in ipc/mod.rs** (mirrors `set_macro_target_app` exactly, lines 41–51):
```rust
// ANALOG (lines 41–51):
#[command]
pub async fn set_macro_target_app(
    state: State<'_, StateManager>,
    id: Uuid,
    target_app: Option<String>,
) -> Result<(), String> {
    state
        .send_intent(Intent::SetMacroTargetApp(id, target_app))
        .await
        .map_err(|e| e.to_string())
}

// NEW — identical structure:
#[command]
pub async fn set_macro_trigger_key(
    state: State<'_, StateManager>,
    id: Uuid,
    trigger_key: Option<u16>,
) -> Result<(), String> {
    state
        .send_intent(Intent::SetMacroTriggerKey(id, trigger_key))
        .await
        .map_err(|e| e.to_string())
}
```

**StateActor match arm** (find the `Intent::SetMacroTargetApp` match arm in `state/mod.rs` — its handler is the template):
```rust
// Pattern to copy (find SetMacroTargetApp handler in StateActor::run()):
Intent::SetMacroTargetApp(id, target_app) => {
    if let Some(m) = self.state.macros.get_mut(&id) {
        m.target_app = target_app;
    }
    self.emit_state_changed();
    self.auto_save().await;
}

// New handler — same shape:
Intent::SetMacroTriggerKey(id, trigger_key) => {
    if let Some(m) = self.state.macros.get_mut(&id) {
        m.trigger_key = trigger_key;
    }
    self.emit_state_changed();
    self.auto_save().await;
    // Windows: also call update_macro_trigger_keys to refresh MACRO_TRIGGER_KEYS registry
    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::update_macro_trigger_keys(&self.state.macros);
    }
}
```

---

### `src-tauri/src/platform/macos/observer.rs` — `list_running_apps` macOS impl

**Analog:** Lines 465–525 (NSWorkspace usage in `start_observing`) for the exact NSWorkspace API call pattern.

**NSWorkspace import pattern** (line 26 — already present):
```rust
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidActivateApplicationNotification};
```

**Add for `list_running_apps`** (add to the existing import at line 26):
```rust
use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace, NSWorkspaceDidActivateApplicationNotification};
```

**runningApplications() pattern** (lines 472–487 show how `NSWorkspace::sharedWorkspace()` is called — copy for `list_running_apps`):
```rust
// From start_observing (lines 472–478):
let ws = NSWorkspace::sharedWorkspace();
if let Some(app) = ws.frontmostApplication() {
    if let Some(bundle_id) = app.bundleIdentifier() {
        let id_string = bundle_id.to_string();
        // ...
    }
}

// For list_running_apps macOS block in ipc/mod.rs:
#[cfg(target_os = "macos")]
{
    use crate::platform::macos::observer::list_running_apps_impl;
    list_running_apps_impl()
}

// In observer.rs — new pub fn:
pub fn list_running_apps_impl() -> Result<Vec<crate::ipc::RunningApp>, String> {
    use objc2_app_kit::NSApplicationActivationPolicy;
    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let apps = workspace.runningApplications();
    let mut result: Vec<crate::ipc::RunningApp> = apps
        .iter()
        .filter(|app| {
            app.activationPolicy() == NSApplicationActivationPolicy::Regular
        })
        .filter_map(|app| {
            let bundle_id = app.bundleIdentifier()?.to_string();
            let name = app
                .localizedName()
                .map(|n| n.to_string())
                .unwrap_or_else(|| bundle_id.clone());
            Some(crate::ipc::RunningApp {
                display_name: name,
                identifier: bundle_id,
            })
        })
        .collect();
    result.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(result)
}
```

---

### `src-tauri/src/platform/windows/mod.rs` — `list_running_apps` Windows impl

**Analog:** Lines 262–291 (`get_app_name_from_hwnd`) — the existing unsafe helper that this implementation calls.

**`get_app_name_from_hwnd` signature** (lines 262–291 — must remain callable from new EnumWindows callback):
```rust
#[cfg(target_os = "windows")]
unsafe fn get_app_name_from_hwnd(hwnd: HWND) -> Option<String> {
    // ... uses OpenProcess + QueryFullProcessImageNameW ...
    Some(String::from_utf16_lossy(&buf[..len as usize]))
}
```

**Additional Win32 imports needed** (add to the existing `#[cfg(target_os = "windows")]` import block at lines 17–18):
```rust
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, IsWindowVisible,
    GetForegroundWindow, GetWindowThreadProcessId,  // already present
};
```

**New `list_running_apps_impl` function** (add after `get_app_name_from_hwnd`, before `impl WindowsInputProvider`):
```rust
#[cfg(target_os = "windows")]
pub fn list_running_apps_impl() -> Result<Vec<crate::ipc::RunningApp>, String> {
    use std::collections::HashMap;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};

    let mut apps: Vec<crate::ipc::RunningApp> = Vec::new();
    let apps_ptr = &mut apps as *mut Vec<crate::ipc::RunningApp> as isize;

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextW, IsWindowVisible};
        if IsWindowVisible(hwnd).as_bool() {
            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len > 0 {
                if let Some(path) = get_app_name_from_hwnd(hwnd) {
                    let basename = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&path)
                        .to_string();
                    let acc = &mut *(lparam.0 as *mut Vec<crate::ipc::RunningApp>);
                    acc.push(crate::ipc::RunningApp {
                        display_name: basename,
                        identifier: path,
                    });
                }
            }
        }
        BOOL(1)
    }

    unsafe {
        EnumWindows(Some(enum_callback), LPARAM(apps_ptr))
            .map_err(|e| e.to_string())?;
    }

    let mut seen = HashMap::new();
    apps.retain(|app| seen.insert(app.identifier.clone(), ()).is_none());
    apps.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(apps)
}
```

---

### `src-tauri/src/lib.rs` — Handler Registration

**Analog:** Lines 83–101 (existing `generate_handler!` block — exact pattern to extend).

**Registration pattern** (lines 83–101):
```rust
.invoke_handler(tauri::generate_handler![
    ipc::add_macro,
    ipc::remove_macro,
    ipc::set_macro_enabled,
    ipc::set_macro_target_app,
    // ... existing ...
    ipc::list_profiles,
    // ADD these two:
    ipc::list_running_apps,
    ipc::set_macro_trigger_key,
])
```

---

## Shared Patterns

### SolidJS Signal Declaration
**Source:** `src/App.tsx` lines 69–94 (all signal declarations at top of `App()` function body)
**Apply to:** All new signals in App.tsx — `editingCardId`, `editingField`, `apps`, `appsLoading`, `appsError`, `triggerKeyRecording`, `newMacroTriggerKeyCode`

Pattern: All signals declared at the top of the `App()` function before any `createEffect` calls. Boolean signals use positive framing (`triggerKeyRecording` not `isNotRecording`).

### createEffect + onCleanup Lifecycle
**Source:** `src/App.tsx` lines 127–148
**Apply to:** Any keydown listener attachment; any async fetch that needs cleanup on unmount.

Pattern: Every `addEventListener` inside App must have a corresponding `removeEventListener` in `onCleanup`. Every async effect uses a `cancelled` flag guard (lines 98–124 pattern).

### invoke() + try/catch Error Handling
**Source:** `src/App.tsx` lines 197–208 (`handleCreateMacro`) and lines 226–233 (`refreshProfiles`)
**Apply to:** `handlePickerFocus`, `handleCardSetTargetApp`, `handleCardSetTriggerKey`

Pattern:
```tsx
try {
  await invoke<ReturnType>("command_name", { param });
  // on success: update signals, reset edit state
} catch (e) {
  console.error("descriptive message:", e);
}
```

### Rust IPC Command Shape (no State dependency)
**Source:** `src-tauri/src/ipc/mod.rs` lines 264–269 (`list_profiles`)
**Apply to:** `list_running_apps`

Pattern: `#[command] pub async fn name() -> Result<Vec<T>, String>` with no `State<'_, _>` parameter when the command does not interact with AppState.

### Rust IPC Command Shape (with StateManager)
**Source:** `src-tauri/src/ipc/mod.rs` lines 41–51 (`set_macro_target_app`)
**Apply to:** `set_macro_trigger_key`

Pattern: `State<'_, StateManager>` as first parameter; `send_intent(Intent::Variant(...)).await.map_err(|e| e.to_string())`.

### Platform-Conditional Compilation
**Source:** `src-tauri/src/ipc/mod.rs` lines 77–93 (`bind_hotkey`)
**Apply to:** `list_running_apps` function body

Pattern: Single `#[command]` function with `#[cfg(target_os = "macos")]`, `#[cfg(target_os = "windows")]`, and `#[cfg(not(any(...)))]` blocks inside the body. The fallback block returns `Ok(Default)`.

### Rust Struct for IPC Data Transfer
**Source:** `src-tauri/src/ipc/mod.rs` — `ProfileSummary` in `persistence.rs` (used at line 268)
**Apply to:** `RunningApp` struct in `ipc/mod.rs`

Pattern:
```rust
#[derive(Debug, Clone, Serialize)]
pub struct RunningApp {
    pub display_name: String,
    pub identifier: String,
}
```
Only `Serialize` needed (frontend reads; no deserialization from frontend).

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/keymap.ts` | utility | transform | No static key mapping files exist in the codebase; pure data module with no runtime analog |

---

## Metadata

**Analog search scope:** `src/`, `src-tauri/src/ipc/`, `src-tauri/src/state/`, `src-tauri/src/platform/`
**Files read:** 8 source files
**Pattern extraction date:** 2026-05-30
