# Feature UX Research

**Project:** AutoMux
**Researched:** 2026-05-15
**Scope:** Key capture UX, cross-platform key code strategy, running process picker

---

## Key Capture Patterns

### How Popular Tools Handle This

**AutoHotkey (Windows)**
- Key assignment uses a "Press a key to capture" modal. The UI shows a button labeled something like "Click to set hotkey", which puts a listener in capture mode. Any key pressed while in capture mode is recorded and its human-readable name displayed ("Ctrl+Shift+F5", "F7", "NumPad0"). Raw VK codes never surface to the user.

**Keyboard Maestro (macOS)**
- Uses a dedicated hotkey recorder widget: a styled button that says "Click to record shortcut". On click, it renders as a focused capture target. The user presses the desired key combination; Keyboard Maestro displays the symbol representation ("⌘⇧K", "F9", "Space"). Internally it stores CGKeyCode + modifier mask.

**BetterTouchTool (macOS)**
- Virtually identical pattern: a button that enters capture mode, shows live preview of what is being pressed (including modifier-only states), and confirms on release. It differentiates between "single key" and "combo" capture modes.

**xdotool GUIs (Linux, e.g., KeyboardRepeater)**
- Same pattern; the field changes border color (e.g., orange outline) when in capture mode and fills with the key label when a key is released.

**Common Pattern Across All Tools:**
1. A styled `<button>` or `<div>` that reads "Press a key..." or shows the current binding in human-readable form.
2. On click, the element enters **capture mode**: it registers a one-time `keydown` listener and possibly changes visual state (different background, animated border).
3. The first non-modifier key press (or a key+modifier combo) is captured and stored as (keycode, modifiers).
4. The displayed value is immediately replaced with a human-readable label ("F7", "Space", "Ctrl+Q").
5. Escape cancels capture without clearing the existing value.

### Recommended Approach for Tauri 2 / SolidJS

**Architecture:** The key capture widget lives entirely in the frontend. No Tauri IPC is needed for the capture itself because `keydown` events fire in the WebView when the WebView has focus.

**Mechanism:**
```
1. User clicks the capture button
2. SolidJS sets a `capturing` signal to true
3. A window-level `keydown` listener is registered (via addEventListener)
4. First non-modifier key fires:
   - Record event.code (for display) + event.key + platform keycode
   - Store the raw u16 (CGKeyCode / VK code) as the internal value
   - Display the human-readable label
   - Remove the listener, set capturing = false
5. Escape fires: remove listener, set capturing = false (no change)
```

**SolidJS component sketch:**
```tsx
function KeyCaptureInput(props: {
  value: number | null;   // stored as raw u16
  onChange: (code: number | null, label: string) => void;
}) {
  const [capturing, setCapturing] = createSignal(false);

  function startCapture() {
    setCapturing(true);
    function onKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      if (e.key === "Escape") {
        setCapturing(false);
        window.removeEventListener("keydown", onKeyDown);
        return;
      }
      // Skip bare modifier keys
      if (["Control","Shift","Alt","Meta"].includes(e.key)) return;

      const code = keyEventToRawCode(e);   // see Cross-Platform section
      const label = keyEventToLabel(e);
      props.onChange(code, label);
      setCapturing(false);
      window.removeEventListener("keydown", onKeyDown);
    }
    window.addEventListener("keydown", onKeyDown);
  }

  return (
    <button
      class={capturing() ? "capture-mode" : "normal"}
      onClick={startCapture}
    >
      {capturing() ? "Press a key..." : (props.value ? labelForCode(props.value) : "Click to set")}
    </button>
  );
}
```

**WebView focus caveat:** The Tauri WebView must be focused for `keydown` to fire inside it. Since AutoMux's macro form is shown in the app window, this is naturally satisfied when the user is interacting with the form.

**Modifier capture for trigger keys:** AutoMux's `trigger_key` field (used for `ToggleMacroHotkey`) is a bare `u16` with no modifier storage. For the immediate UX fix, capture single keys without modifiers — matching what the CGEventTap already handles in `get_macro_trigger_keys()`. The existing `bind_hotkey` IPC command stores modifiers separately for global hotkeys; the trigger key feature is a simpler path.

---

## Key Code Cross-Platform Strategy

### The Fundamental Problem

The internal type is `InputEvent::Key(u16)`. On macOS this is a **CGKeyCode** (hardware-layout-independent virtual key). On Windows it is a **Win32 Virtual Key (VK_*) code**. These two namespaces overlap numerically but mean different things:

- macOS CGKeyCode 0 = A key (ANSI position 0)
- Windows VK 0x00 = undefined

Some codes coincidentally match (Space: macOS 49, Windows VK_SPACE 0x20 = 32 — different). They cannot be used interchangeably.

### Recommended Strategy: Named Key Enum + Per-Platform Mapping

**Option A: Named enum in Rust, serialized as string tag (RECOMMENDED)**

Define a `NamedKey` enum that covers the ~80 keys users actually assign in automation tools. The frontend works exclusively with `NamedKey` strings. Each platform layer maps `NamedKey → u16` at injection time.

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamedKey {
    A, B, C, ..., Z,
    F1, F2, ..., F12,
    Space, Enter, Escape, Tab,
    Backspace, Delete,
    Left, Right, Up, Down,
    Home, End, PageUp, PageDown,
    Insert,
    Num0, Num1, ..., Num9,
    Numpad0, ..., Numpad9,
    NumpadPlus, NumpadMinus, NumpadMul, NumpadDiv,
    Semicolon, Equals, Comma, Minus, Period, Slash,
    Grave, LeftBracket, Backslash, RightBracket, Quote,
}
```

The `InputEvent::Key(u16)` variant remains for backward compatibility (loading old saved profiles). New macros created via the UI store `InputEvent::NamedKey(NamedKey)`. The Rust platform layer resolves at injection:

```rust
// macos/input.rs
fn named_key_to_cgkeycode(k: NamedKey) -> u16 {
    match k {
        NamedKey::A => 0,
        NamedKey::Space => 49,
        // ...
    }
}

// windows/mod.rs
fn named_key_to_vk(k: NamedKey) -> u16 {
    match k {
        NamedKey::A => 0x41,
        NamedKey::Space => 0x20,
        // ...
    }
}
```

**Serialization:** `NamedKey::Space` serializes as `"space"` in JSON profiles — human-readable, portable, and survives re-loading on a different OS.

**Frontend:** The SolidJS `KeyCaptureInput` maps `KeyboardEvent.code` (a DOM standard, layout-independent) to `NamedKey`:

```ts
const DOM_CODE_TO_NAMED_KEY: Record<string, string> = {
  "KeyA": "a", "KeyB": "b", ...,
  "Space": "space", "Enter": "enter",
  "F1": "f1", ...
};
```

`KeyboardEvent.code` is the best choice because it is layout-independent (matches physical key positions, same as CGKeyCode philosophy on macOS).

**Option B: Keep raw u16, add a display-name lookup table in the frontend (NOT RECOMMENDED)**

Store codes as-is but ship a JS object mapping known CGKeyCode/VK values to names. This is fragile: the same stored value means different keys on the two platforms, so a profile saved on macOS would misfire on Windows if ever shared. Avoid.

**Migration path for existing profiles:**
- Old profiles contain `{ "Key": 49 }` (raw u16).
- Add a Rust migration helper: when loading a profile, if `InputEvent::Key(n)` is encountered, attempt to convert to the nearest `NamedKey` using the macOS CGKeyCode table (since macOS is the primary dev platform). Unmappable codes stay as `Key(n)` and are displayed as "Key(49)" in the UI — same as today, but only for legacy entries.

### Key Label Display

For immediate display in the macro list (`formatInputEvent` in `App.tsx`), add a lookup map in TypeScript:

```ts
const NAMED_KEY_LABELS: Record<string, string> = {
  "space": "Space", "enter": "Enter", "escape": "Esc",
  "a": "A", "b": "B", ...
  "f1": "F1", ...
  "left": "←", "right": "→", "up": "↑", "down": "↓",
};

function formatInputEvent(ev: InputEvent): string {
  if ("NamedKey" in ev) return `⌨ ${NAMED_KEY_LABELS[ev.NamedKey] ?? ev.NamedKey}`;
  if ("Key" in ev)      return `⌨ Key(${ev.Key})`;   // legacy fallback
  if ("MouseButton" in ev) return `🖱 ${ev.MouseButton}`;
  return "?";
}
```

---

## Process Picker — macOS

### Available API

`NSWorkspace.sharedWorkspace().runningApplications` returns an `NSArray<NSRunningApplication>`. Each entry exposes:
- `bundleIdentifier: NSString?` — the bundle ID (e.g. `"com.apple.finder"`)
- `localizedName: NSString?` — human-readable app name (e.g. `"Finder"`)
- `activationPolicy: NSApplicationActivationPolicy` — whether the app is a regular UI app (`.regular`), an agent (`.accessory`), or a background daemon (`.prohibited`)

**Filtering to user-facing apps:** Filter to `activationPolicy == .regular`. This excludes background helpers, daemons, and system services. Result is typically 5–25 entries on a typical macOS system.

### Rust/Tauri Implementation

The `objc2-app-kit` crate (already a dependency at version 0.3.2) exposes `NSWorkspace` and `NSRunningApplication`. A new IPC command can be added:

```rust
// In ipc/mod.rs
#[derive(Debug, Serialize)]
pub struct RunningApp {
    pub bundle_id: String,
    pub name: String,
}

#[command]
pub async fn list_running_apps() -> Result<Vec<RunningApp>, String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSWorkspace, NSApplicationActivationPolicy};
        let workspace = NSWorkspace::sharedWorkspace();
        let apps = workspace.runningApplications();
        let mut result = Vec::new();
        for app in apps.iter() {
            if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
                continue;
            }
            let Some(bundle_id) = app.bundleIdentifier() else { continue };
            let name = app.localizedName()
                .map(|n| n.to_string())
                .unwrap_or_else(|| bundle_id.to_string());
            result.push(RunningApp {
                bundle_id: bundle_id.to_string(),
                name,
            });
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(vec![])
    }
}
```

**Confidence:** HIGH — `NSWorkspace.runningApplications` is a long-standing public API (available since macOS 10.6). The `objc2-app-kit` crate already uses `NSWorkspace` in `observer.rs` (for `NSWorkspaceDidActivateApplicationNotification`), so the import pattern is already established in the codebase.

**Permission note:** Reading `runningApplications` does not require Accessibility permission — it is a standard public API. This means the list can be shown even before the user has granted Accessibility access.

### UX Pattern Recommendation for macOS

A **search-filtered inline dropdown** attached to the target app field:

1. Replace the raw text input with a composite widget: a text input + a "Browse..." button.
2. Clicking "Browse..." (or focusing the text input) opens an inline popover/dropdown showing the sorted list of running apps.
3. Each row shows: `[app icon if available] AppName  (com.bundle.id)` — name large, bundle ID small/dimmed.
4. The user can type to filter by name or bundle ID.
5. Selecting a row fills the field with the bundle ID and closes the dropdown.
6. The "Global" option (no target app) is pinned at the top of the list.

This pattern is used by Alfred, Raycast, and Keyboard Maestro for application selection. It is familiar, fast, and avoids requiring the user to know the bundle ID.

**Refresh strategy:** Call `list_running_apps` when the picker opens (not on a timer). Apps change infrequently; a stale list is fine for the picker's lifetime. A "Refresh" icon button in the dropdown header handles the rare case where the user needs to launch an app and pick it.

---

## Process Picker — Windows

### Available API

**EnumWindows + GetWindowText + GetWindowThreadProcessId** is the standard approach for listing user-facing windows (and their owning processes). The existing `get_app_name_from_hwnd` in `windows/mod.rs` already uses `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`.

For a process picker, the recommended Win32 approach is:

1. `EnumWindows` iterates all top-level windows.
2. For each window, filter with `IsWindowVisible` + `GetWindowLong(GWL_STYLE) & WS_CAPTION` to exclude invisible/frameless processes.
3. `GetWindowText` fetches the window title (for display).
4. `GetWindowThreadProcessId` gets the owning process ID.
5. `QueryFullProcessImageNameW` (already used) gets the executable path.
6. The executable filename (e.g. `"chrome.exe"`) becomes the displayed name and stored identifier for Windows.

**Alternative: CreateToolhelp32Snapshot** — enumerates all processes (including background ones). This is less useful for the picker because it includes hundreds of system processes with no window. EnumWindows produces a more relevant, user-facing list.

**Filtering:** Skip windows where `IsWindowVisible` is false, skip windows without a title, and skip windows owned by the current AutoMux process itself.

**What identifier to store on Windows:** The current observer code stores the full path from `QueryFullProcessImageNameW` (e.g. `C:\Program Files\Google\Chrome\Application\chrome.exe`). For matching against `active_app`, the same full path is used. The picker should therefore return the same format the observer produces — not just the filename — to ensure `target_app == active_app` comparisons work.

### Rust/Tauri Implementation

```rust
// ipc/mod.rs — Windows side of list_running_apps
#[cfg(target_os = "windows")]
{
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, IsWindowVisible, WNDENUMPROC,
    };
    use windows::Win32::Foundation::{HWND, LPARAM, BOOL};
    // Collect visible top-level windows, resolve process paths
    // using get_app_name_from_hwnd (already implemented)
    // Deduplicate by path, sort by display name
}
```

The existing `get_app_name_from_hwnd` helper in `windows/mod.rs` can be reused directly; it is already pub-accessible within the crate.

**Confidence:** MEDIUM — the Win32 APIs are well-established, but the exact filtering combination (IsWindowVisible + WS_CAPTION check) needs testing to get the right subset on different Windows versions and DPI configurations. Some apps (e.g. UWP) have non-standard window structures. Flag for validation during implementation.

### UX Pattern Recommendation for Windows

Same search-filtered inline dropdown as macOS, with these adaptations:
- Display the executable filename (e.g. `chrome.exe`) as the primary name, with the full path as a tooltip.
- Pin "Global" at the top.
- The stored value for `target_app` must be the full executable path (matching what `get_active_app` returns from the observer) — not just the basename.

**Label asymmetry to document:** On macOS, `target_app` is a bundle ID (`com.google.Chrome`). On Windows, it is a full exe path (`C:\...\chrome.exe`). This asymmetry is already baked into the existing codebase. Profiles are not portable across platforms (a macOS profile referencing `com.google.Chrome` will not match on Windows). The picker UI should make this implicit without confusing users.

---

## Recommended Implementation Approach

### Key Capture — Phased Approach

**Phase 1 (immediate, no Rust changes):** Add the `KeyCaptureInput` SolidJS component. It intercepts `keydown` in the WebView and maps `KeyboardEvent.code` to `NamedKey` string names. Store these names as the new `trigger_key` representation in the frontend. For the IPC call, provide a separate `name_to_cgkeycode` mapping in TypeScript so the existing `u16` field can be populated. This avoids any Rust schema change and ships the UX fix immediately.

**Phase 2 (schema migration):** Add `NamedKey` enum to Rust's `InputEvent`. Add `NamedKey` → platform keycode resolvers in both `macos/input.rs` and `windows/mod.rs`. Update profile serialization. Provides correct cross-platform key identity in stored profiles.

For Phase 1, the TypeScript lookup tables needed are:
- `DOM_CODE_TO_CGKEYCODE: Record<string, number>` — ~80 entries covering letters, digits, F-keys, navigation keys, and symbols (authoritative values from Apple's HIToolbox.framework)
- `DOM_CODE_TO_VK: Record<string, number>` — same keys mapped to Win32 VK_ codes
- Select the right table at runtime using `navigator.platform` or a `#[cfg]`-gated Tauri command that returns the current OS.

### Process Picker — Single IPC Command, Shared UX

Add one new IPC command `list_running_apps() -> Vec<RunningApp>` with platform-specific implementations. The `RunningApp` struct carries both `bundle_id` (or exe path on Windows) and `display_name`. The frontend uses `display_name` for rendering and `bundle_id`/path for the stored `target_app` value.

**Component:** Replace the `<input id="input-macro-target">` raw text field with a `<ProcessPickerInput>` component that:
1. Shows the current value as a badge: `[Finder (com.apple.finder)]` or `[Global]`.
2. Has a clear button (×) to reset to Global.
3. Has a "Browse running apps..." button that opens a combobox/listbox.
4. The listbox filters as the user types (filter on both name and bundle ID/path).
5. On selection, closes and sets the value.

**State management:** The running apps list is fetched once when the picker opens (not pre-fetched on app load) and discarded when the picker closes. No persistent signal needed — a `createResource` with manual refetch is appropriate in SolidJS.

### Priority Order

1. **Key capture widget** — pure frontend change, no Rust needed, highest user-visible pain reduction.
2. **macOS process picker** — one new Rust IPC command + frontend component; `NSWorkspace.runningApplications` is already used in the codebase so the import pattern is established.
3. **Windows process picker** — `EnumWindows`-based enumeration requires more careful filtering; implement after macOS is validated.
4. **NamedKey schema migration** — deferred to a dedicated milestone; Phase 1 key capture already delivers the UX fix.

### Risks and Flags

| Risk | Severity | Mitigation |
|------|----------|------------|
| WebView `keydown` capture may not fire for all system keys (e.g., F-keys, media keys intercepted by OS) | MEDIUM | Graceful fallback: if `KeyboardEvent.code` is unmapped, display "Key(N)" and store raw keycode; user can retry |
| macOS: `NSRunningApplication.bundleIdentifier` is nil for some sandboxed helper processes | LOW | Filter out nil bundle IDs in Rust (already handled in the sketch above) |
| Windows: Full exe path as target_app identifier changes if the app is updated/reinstalled | MEDIUM | Document limitation; consider storing basename separately as display name while storing full path for matching |
| SolidJS window-level keydown listener: if another element captures the event first (e.g., a focused button), the capture may not fire | LOW | Use `{ capture: true }` option on `addEventListener` to capture during the event's capture phase |
| Windows EnumWindows: some UWP/Electron apps may appear multiple times (multiple processes per window) | LOW | Deduplicate by process path in the Rust enumeration |
| Profile cross-platform portability: macOS bundle IDs vs Windows exe paths in `target_app` | MEDIUM | Document this as a known limitation; do not attempt to solve in this milestone |
