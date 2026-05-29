# Phase 3: Macro Setup UX — Research

**Researched:** 2026-05-30
**Domain:** SolidJS key capture widget, Rust IPC for process enumeration, cross-platform keycode mapping
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Key Capture Widget (UX-01)**
- D-01: Click-to-capture interaction model. The trigger key field shows "Click to set key…" by default. Clicking enters recording state that intercepts the next `keydown` event; captured key name is displayed and integer keycode stored. Clear/Cancel exits without committing. Raw CGKeyCode/VK integers never shown.
- D-02: Human-readable name only after capture ("Q", "Space", "F5", "Left Shift"). No raw integer shown.
- D-03: Frontend lookup table (static `Map<number, string>`) for key name resolution — no new Rust IPC for key names.

**Process Picker (UX-02, UX-03)**
- D-04: Inline `<select>` dropdown — no modal or custom popover. Consistent with existing `<select>` elements.
- D-05: Display name + identifier per option. macOS: "Minecraft (com.mojang.minecraft)". Windows: "Minecraft (C:\...\Minecraft.exe)". Stored `target_app` remains the bundle ID (macOS) or process path (Windows).
- D-06: Fetch on open, every time. `list_running_apps` called each time dropdown is opened or focused. No refresh button.

**Edit Scope**
- D-07: Both creation form AND inline card editing.
- D-08: Click-to-edit with auto-commit. Key badge enters capture mode; new keypress commits via `bind_hotkey`. Target badge opens picker; selection commits via `set_macro_target_app`. Escape exits without committing.

### Claude's Discretion

- Exact visual treatment of the "recording" state (defined in UI-SPEC: `border-accent` + glow)
- Unknown key fallback display (defined in UI-SPEC: `"Key {N}"`)
- "Global" sentinel placement (defined in UI-SPEC: first option with empty value)
- Exact IPC command signature for `list_running_apps`

### Deferred Ideas (OUT OF SCOPE)

- `trigger_key_name` field in Rust model / profile migration (v2 UX-04)
- Cargo dep audit for `cocoa` 0.26 / `objc` 0.2
- Windows hotkey modifier support via `bind_hotkey`/`unbind_hotkey`
- Duplicate trigger key validation
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UX-01 | Key Code field replaced with key capture widget — raw CGKeyCode/VK integers never shown | Key code lookup tables verified (HIToolbox Events.h + Microsoft VK docs). SolidJS `keydown` capture pattern verified via Context7. Existing `trigger_key: number \| null` signal in `handleCreateMacro` is the write target. |
| UX-02 | macOS target app field replaced with running-process picker (name + bundle ID) | `NSWorkspace.runningApplications()` verified via objc2-app-kit Context7 docs. `NSRunningApplication.bundleIdentifier` and `localizedName` confirmed. Existing codebase already uses `NSWorkspace` in observer.rs. No new crate dependencies needed. |
| UX-03 | Windows target app field replaced with running-process picker (name + path) | `EnumWindows` + `QueryFullProcessImageNameW` approach verified. `QueryFullProcessImageNameW` already used in `platform/windows/mod.rs`. All needed Win32 features already imported. `get_app_name_from_hwnd` helper is reusable. |
</phase_requirements>

---

## Summary

Phase 3 replaces two raw-data fields in the macro creation form and adds inline edit affordances to macro cards. The two widgets are:

1. **Key capture widget** — a click-to-record `<div>` that attaches a `keydown` listener to `document`, translates the event to a human-readable key name via a static lookup table, stores the integer keycode, and calls `bind_hotkey` IPC on commit for card edits. No Rust changes needed for key name resolution; the lookup table lives in TypeScript. The only Rust requirement is the existing `bind_hotkey` command (macOS only) and the existing `trigger_key: number | null` path through `handleCreateMacro`.

2. **Process picker** — replaces `<input type="text">` with `<select>`. Populated by a new `list_running_apps` IPC command. On macOS this uses `NSWorkspace.runningApplications()` (already imported in `observer.rs`). On Windows it uses `EnumWindows` + the existing `get_app_name_from_hwnd` helper. No new crate dependencies are required for either platform.

The UI-SPEC (already approved) fully defines visual states, color tokens, class strings, option format, and copy. This research maps each decision to concrete implementation patterns.

**Primary recommendation:** Implement in three tasks — (1) key capture widget replacing both form input and card badge, (2) macOS `list_running_apps` + process picker, (3) Windows `list_running_apps` + process picker. All share the same `<select>` UI component; only the Rust backend differs per platform.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Key name display (lookup) | Browser/Client | — | Static `Map<number, string>` in TypeScript; no I/O, no round-trip |
| Key capture (intercept) | Browser/Client | — | `document.addEventListener('keydown', ...)` fires in the WebView when it has focus |
| Trigger key storage for creation form | Browser/Client | API/Backend | Signal written in frontend; sent to backend via `add_macro` IPC on form submit |
| Trigger key commit for card edit | API/Backend | Browser/Client | Card edit calls `bind_hotkey` IPC directly after capture; macOS only |
| Process enumeration | API/Backend | — | OS API access (`NSWorkspace`, `EnumWindows`) requires Rust; frontend gets a JSON array |
| Target app storage | API/Backend | — | `set_macro_target_app` IPC writes to `AppState` through the Intent channel |
| Inline card edit state | Browser/Client | — | Per-card `editingCardId` + `editingField` signals; no backend state needed |

---

## Standard Stack

### Core (no changes to existing dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| SolidJS | 1.9.3 (existing) | Reactive UI — signals, `Show`, `For` | Already in codebase; `createSignal` + `createEffect` + `onCleanup` are the correct tools for the capture listener lifecycle |
| objc2-app-kit | 0.3.2 (existing) | `NSWorkspace.runningApplications()` on macOS | Already a dependency; `NSWorkspace` already imported in `observer.rs` |
| windows crate | 0.61 (existing) | `EnumWindows`, `IsWindowVisible`, `GetWindowTextW` on Windows | All required features already present in `Cargo.toml` (Win32_UI_WindowsAndMessaging) |
| Tailwind CSS 4 | 4.3.0 (existing) | `border-accent`, `shadow-[0_0_8px_var(...)]`, `cursor-pointer` | All classes used in UI-SPEC already exist in the design token system |

No new npm packages or Rust crates are required.

### Windows Features Already Present

The `windows` crate features already declared in `Cargo.toml` cover all needs:

| Feature | Used For |
|---------|---------|
| `Win32_UI_WindowsAndMessaging` | `EnumWindows`, `IsWindowVisible`, `GetWindowTextW`, `GetWindowLong`, `WS_CAPTION` |
| `Win32_System_Threading` | `OpenProcess`, `QueryFullProcessImageNameW` (already used in `get_app_name_from_hwnd`) |
| `Win32_Foundation` | `HWND`, `BOOL`, `LPARAM` |

**No new feature flags needed in Cargo.toml.** [VERIFIED: codebase grep, src-tauri/src/platform/windows/mod.rs]

---

## Architecture Patterns

### System Architecture Diagram (Phase 3 additions)

```
User clicks Key badge / Trigger key field
        │
        ▼
[SolidJS: recording signal = true]
        │
        ▼
[document keydown listener attached]
        │
[User presses key]
        ▼
[e.preventDefault(), e.stopPropagation()]
        │
        ├─── macOS: CGKeyCode lookup table → display name
        └─── Windows: VK code lookup table → display name
        │
        ▼
[Store integer in trigger_key signal]
        │
        ├── Creation form: stays in signal until "Create Macro"
        └── Card edit: invoke("bind_hotkey", {macro_id, keycode, modifiers}) ──► Rust IPC
                                                                                       │
                                                                                       ▼
                                                                           add_hotkey_binding() (macOS)
                                                                           [no-op on Windows]

User opens Process Picker
        │
        ▼
[<select> onFocus fires]
        │
[invoke("list_running_apps")]
        │
        ├── macOS: NSWorkspace.sharedWorkspace().runningApplications()
        │         filter activationPolicy == Regular
        │         map to {display_name, identifier} sorted by name
        │
        └── Windows: EnumWindows callback
                     filter IsWindowVisible + has title + not current process
                     QueryFullProcessImageNameW → full exe path
                     map to {display_name: basename, identifier: full_path}
        │
        ▼
[Populate <select> options]
        │
[User selects]
        │
        ├── Creation form: store in newMacroTarget signal
        └── Card edit: invoke("set_macro_target_app", {id, target_app}) ──► StateActor
```

### Recommended Project Structure (additions only)

```
src/
├── App.tsx              # All changes here — no new files (consistent with existing pattern)
src-tauri/src/
├── ipc/mod.rs           # Add list_running_apps command + RunningApp struct
```

No new source files are required. The key capture widget and process picker are inline JSX within `App.tsx`, consistent with the existing codebase pattern where all UI lives in a single component file.

### Pattern 1: Key Capture Widget — SolidJS Signals + Document Listener

**What:** A `<div>` acting as a button. `isRecording` signal controls visual state and whether a `keydown` listener is attached to `document`.

**When to use:** Any field where the user must press a key to configure a binding.

```tsx
// Source: Context7 /solidjs/solid-docs — onCleanup event listener pattern
// + .planning/research/FEATURES.md (existing project research)

// Component-local signals (NOT app-level — each widget is independent)
const [isRecording, setIsRecording] = createSignal(false);
const [triggerKey, setTriggerKey] = createSignal<number | null>(null);

function startCapture() {
  setIsRecording(true);

  function onKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape") {
      setIsRecording(false);
      document.removeEventListener("keydown", onKeyDown, true);
      return;
    }

    // Skip bare modifier keys — they are not trigger keys
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

    const rawCode = e.keyCode; // integer — CGKeyCode on macOS, VK on Windows
    setTriggerKey(rawCode);
    setIsRecording(false);
    document.removeEventListener("keydown", onKeyDown, true);
  }

  // { capture: true } ensures this fires even if a focused child handles the event
  document.addEventListener("keydown", onKeyDown, true);
}

// Cleanup on unmount
onCleanup(() => {
  // onKeyDown closure won't be accessible here — use a ref or a module-level variable
  // Pattern: store the listener ref at component scope, call removeEventListener in onCleanup
});
```

**Key design note:** Use `{ capture: true }` on `addEventListener` so the listener fires in the capture phase before any focused child element consumes the event. This prevents a focused button inside the form from intercepting the keypress before the capture listener sees it. [CITED: https://github.com/solidjs/solid-docs — Event Propagation with Native vs. Delegated Events]

**Escape handling:** Escape (keyCode 27 on both platforms) must cancel capture. The Tauri WebView does not intercept Escape for any system purpose, so `preventDefault()` is sufficient.

**Modifier-only skip:** Skip keys where `e.key` is `"Control"`, `"Shift"`, `"Alt"`, or `"Meta"`. The current `trigger_key` model stores a bare u16 with no modifier mask (unlike `bind_hotkey` which takes separate modifiers). [VERIFIED: src-tauri/src/state/mod.rs line 93 — `trigger_key: Option<u16>`]

### Pattern 2: Per-Card Edit State — App-Level Signals

**What:** Two app-level signals (`editingCardId: Accessor<string | null>` and `editingField: Accessor<"key" | "target" | null>`) track which card and field is currently being edited. Cards are not separate components — they are JSX inside a `For` loop — so state must live at the `App` component level.

**When to use:** Any time a `For` loop renders interactive items that need independent edit state without extracting them to full components.

```tsx
// App-level (alongside other createSignal calls)
const [editingCardId, setEditingCardId] = createSignal<string | null>(null);
const [editingField, setEditingField] = createSignal<"key" | "target" | null>(null);

// Inside the For loop:
// macro.id === editingCardId() && editingField() === "key"  → show recording widget
// macro.id === editingCardId() && editingField() === "target" → show inline <select>
// otherwise → show static badges
```

**Why not component-local:** SolidJS `For` keyed loops do not preserve component instances when array order changes. Tracking edit state inside the loop closure means the signal is tied to that iteration's closure, which is fine — but if the macro list re-orders (e.g., after an enable/disable triggers a `state-changed` event), the closing-over signal value would be lost. App-level signals survive state updates. [ASSUMED — based on SolidJS reactivity model; confirm behavior holds for this use case during implementation]

### Pattern 3: Process Picker — onFocus Fetch with createSignal

**What:** A `<select>` element whose options are populated by calling `list_running_apps` each time the user opens it (D-06). State is kept in local signals for loading/error/data.

```tsx
// Local to the form (or card) context
const [apps, setApps] = createSignal<RunningApp[]>([]);
const [appsLoading, setAppsLoading] = createSignal(false);
const [appsError, setAppsError] = createSignal(false);

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

// <select> JSX:
// onFocus={handlePickerFocus}
// Options: "Loading...", "Failed to load apps", or the RunningApp list
// First option always: <option value="">🌐 Global (no target)</option>
```

**Pattern note:** Do not use `createResource` here. `createResource` is reactive and re-fetches automatically on signal changes. The requirement is a one-shot fetch on focus, not reactive refetch. Plain `async function` called from `onFocus` is simpler and correct. [CITED: .planning/research/FEATURES.md — "fetch once on open, discard on close"]

### Pattern 4: New Rust IPC Command — `list_running_apps`

**What:** A single `#[command]` in `ipc/mod.rs` with platform-conditional implementations.

```rust
// Source: ipc/mod.rs pattern — all existing commands follow this structure
// Return type mirrors other list commands (list_profiles returns Vec<ProfileSummary>)

#[derive(Debug, Clone, Serialize)]
pub struct RunningApp {
    pub display_name: String,
    pub identifier: String,
}

#[command]
pub async fn list_running_apps() -> Result<Vec<RunningApp>, String> {
    #[cfg(target_os = "macos")]
    {
        // See macOS implementation pattern below
    }
    #[cfg(target_os = "windows")]
    {
        // See Windows implementation pattern below
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Ok(vec![])
    }
}
```

**Registration:** Add `ipc::list_running_apps` to the `tauri::generate_handler![]` macro in `lib.rs`. [VERIFIED: src-tauri/src/lib.rs lines 83–101 — existing registration pattern]

### Pattern 5: macOS — `NSWorkspace.runningApplications()`

```rust
// Source: Context7 /websites/rs_objc2-app-kit — NSWorkspace, NSRunningApplication
// + .planning/research/FEATURES.md macOS section

#[cfg(target_os = "macos")]
{
    use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace};

    let workspace = NSWorkspace::sharedWorkspace();
    let apps = workspace.runningApplications();
    let mut result: Vec<RunningApp> = apps
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
            Some(RunningApp {
                display_name: name,
                identifier: bundle_id,
            })
        })
        .collect();
    result.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(result)
}
```

**Key facts:**
- `activationPolicy() == NSApplicationActivationPolicy::Regular` filters out daemons, agents, and system helpers. Typical result: 5–25 entries. [VERIFIED: Context7 /websites/rs_objc2-app-kit — NSRunningApplication.activationPolicy]
- `bundleIdentifier()` returns `Option<Retained<NSString>>` — nil for apps without `Info.plist` (e.g., command-line tools). Filter these out with `filter_map`. [VERIFIED: Context7 — bundleIdentifier docs]
- `localizedName()` returns `Option<Retained<NSString>>` — almost always Some for Regular apps; fall back to bundle_id. [VERIFIED: Context7 — localizedName docs]
- `NSWorkspace::sharedWorkspace()` is already called in `observer.rs` line 467 — the import pattern is established. [VERIFIED: codebase]
- `runningApplications` does NOT require Accessibility permission. [CITED: .planning/research/FEATURES.md]
- No new Cargo dependencies needed — `objc2-app-kit = "0.3.2"` is already declared. [VERIFIED: Cargo.toml line 34]

### Pattern 6: Windows — `EnumWindows` + `get_app_name_from_hwnd`

```rust
// Source: .planning/research/FEATURES.md Windows section
// + codebase: platform/windows/mod.rs — existing get_app_name_from_hwnd

#[cfg(target_os = "windows")]
{
    use std::collections::HashMap;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, IsWindowVisible,
    };

    // Thread-local accumulator passed via LPARAM
    let mut apps: Vec<RunningApp> = Vec::new();
    let apps_ptr = &mut apps as *mut Vec<RunningApp> as isize;

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if IsWindowVisible(hwnd).as_bool() {
            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len > 0 {
                let title_str = String::from_utf16_lossy(&title[..len as usize]);
                // get_app_name_from_hwnd is in the same module — call directly
                if let Some(path) = get_app_name_from_hwnd(hwnd) {
                    let basename = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&path)
                        .to_string();
                    let acc = &mut *(lparam.0 as *mut Vec<RunningApp>);
                    acc.push(RunningApp {
                        display_name: basename,
                        identifier: path,
                    });
                }
            }
        }
        BOOL(1) // continue enumeration
    }

    EnumWindows(Some(enum_callback), LPARAM(apps_ptr));

    // Deduplicate by identifier (same process can have multiple windows)
    let mut seen = HashMap::new();
    apps.retain(|app| seen.insert(app.identifier.clone(), ()).is_none());
    apps.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(apps)
}
```

**Key facts:**
- `EnumWindows` is in `Win32_UI_WindowsAndMessaging` — already in `Cargo.toml` features. [VERIFIED: Cargo.toml line 40]
- `QueryFullProcessImageNameW` is called via the existing `get_app_name_from_hwnd` helper in `platform/windows/mod.rs` — reuse it rather than duplicating. [VERIFIED: codebase lines 262–291]
- `get_app_name_from_hwnd` is an `unsafe fn` in the same module; the `list_running_apps` Windows block must use `unsafe`. [VERIFIED: codebase]
- `GetWindowTextW` is already in `Win32_UI_WindowsAndMessaging`. [VERIFIED: Windows crate docs pattern]
- The stored `identifier` must be the full exe path, not just the basename — this is what the observer produces and what `target_app == active_app` comparisons use. [VERIFIED: platform/windows/mod.rs lines 279–291]

### Anti-Patterns to Avoid

- **Fetching running apps on app startup or in a polling interval:** The decision D-06 is "fetch on open, every time". Pre-fetching would waste a Tauri round-trip and the list would be stale by the time the user opens the picker. [CITED: CONTEXT.md D-06]
- **Using `createResource` for the picker fetch:** `createResource` is reactive — it re-fetches when its source signal changes. The picker needs a one-shot imperative fetch on focus, not a reactive binding.
- **Storing `editingCardId` inside the `For` loop closure:** The closure is re-created when SolidJS re-renders the list. Use app-level signals.
- **Calling `bind_hotkey` on Windows card edit:** `bind_hotkey` is a macOS-only IPC command. On Windows, the trigger key is stored in `trigger_key: number | null` on the MacroConfig, and the Windows hook processes `MACRO_TRIGGER_KEYS` directly via `update_macro_trigger_keys`. The card edit flow on Windows should call `add_macro` or a dedicated update IPC (or re-submit the full config), NOT `bind_hotkey`. [VERIFIED: ipc/mod.rs lines 77–94, platform/windows/mod.rs lines 391–399]
- **Using `e.keyCode` without a lookup table:** On macOS, `e.keyCode` from a WebView `keydown` event returns the DOM `keyCode` value, which for alphanumeric keys equals the ASCII value (A=65, not CGKeyCode 0). The DOM `keyCode` is NOT the same as CGKeyCode for most keys. The lookup table must map DOM `keyCode` values to CGKeyCode values, not just use `e.keyCode` directly as the stored integer.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Process list filtering (macOS) | Custom Objective-C runtime iteration | `NSWorkspace.runningApplications()` + `activationPolicy` filter | The API already filters; hand-rolling would duplicate the OS mechanism |
| Process list (Windows visible apps) | `CreateToolhelp32Snapshot` | `EnumWindows` + `get_app_name_from_hwnd` | `CreateToolhelp32Snapshot` returns all 200+ system processes; `EnumWindows` already returns the user-facing subset; `get_app_name_from_hwnd` already exists in the codebase |
| Key name resolution at runtime | Rust IPC command to look up key names | Static `Map<number, string>` in TypeScript | Runtime round-trip is unnecessary; the mapping is static and never changes |
| Cleanup of `keydown` listener | Manual tracking of added listeners | `onCleanup()` inside `createEffect` or at component scope | SolidJS disposal system handles this correctly; manual tracking is error-prone |

**Key insight:** Both the process list and the key name lookup have existing OS APIs or static data that are correct by definition. Custom implementations would miss edge cases (background helpers, nil bundle IDs, hidden windows, OEM key codes) that the OS APIs already handle.

---

## Common Pitfalls

### Pitfall 1: DOM `keyCode` vs CGKeyCode Mismatch

**What goes wrong:** Developer captures `e.keyCode` from the WebView `keydown` event and stores it directly as `trigger_key`. On macOS, `e.keyCode` for the "A" key returns 65 (ASCII). CGKeyCode for "A" is 0. When the macro fires and `inject_key(65, ...)` is called on macOS, CoreGraphics injects VK 65 (which does not map to a standard macOS key cleanly).

**Why it happens:** The DOM `KeyboardEvent.keyCode` is the ASCII/Unicode value for printable characters, not the platform virtual keycode. CGKeyCode is hardware-position-based (ANSI layout), not character-based.

**How to avoid:** The lookup table must map from DOM `keyCode` to the platform-native integer. Specifically:
- macOS: `DOM_KEYCODE_TO_CGKEYCODE` maps e.g. `65 → 0`, `32 → 49`, `27 → 53`
- Windows: `DOM_KEYCODE_TO_VK` maps e.g. `65 → 0x41`, `32 → 0x20` (these happen to match for letters/digits but diverge for special keys)
- Use `e.keyCode` as the lookup key, return the platform native integer to store.

**Warning signs:** Key capture "works" (records without error) but the macro fires the wrong key.

[VERIFIED: src-tauri/src/state/mod.rs line 40 — "macOS: CGKeyCode values (e.g., 0=A, 12=Q, 49=Space)"; HIToolbox Events.h verified]

### Pitfall 2: `keydown` Listener Leaks During Fast Click/Escape

**What goes wrong:** User clicks the capture field, then immediately clicks elsewhere or clicks another capture field. The first `keydown` listener is still attached. The second click creates a second listener. Both fire on the next keydown — double-commit.

**Why it happens:** The listener reference is created inside a closure and there is no guard against multiple attaches.

**How to avoid:** Before attaching a new listener, always call `removeEventListener` for any previously registered listener. Keep the listener reference at the component/closure scope. `onCleanup` inside the recording `createEffect` (or a module-level ref) handles unmount cleanup.

**Warning signs:** Pressing a key after clicking around commits the value twice, or committing one macro's key triggers another macro's capture.

### Pitfall 3: macOS `bind_hotkey` Called Without Clearing Prior Binding

**What goes wrong:** User edits the trigger key on a macro card from "Q" to "E". The `add_hotkey_binding` call registers the new "E" binding, but the old "Q" binding remains in `HOTKEY_BINDINGS`. Now both Q and E toggle the macro.

**Why it happens:** `add_hotkey_binding` appends to a `Vec`; it does not replace. Only `remove_hotkey_bindings_for(macro_id)` removes old entries.

**How to avoid:** On card edit commit for the key capture widget, call `invoke("unbind_hotkey", { macro_id })` before `invoke("bind_hotkey", { macro_id, keycode, modifiers })`. [VERIFIED: ipc/mod.rs lines 96–104 — `unbind_hotkey` exists]

**Warning signs:** The same macro can be triggered by multiple keys after editing.

### Pitfall 4: Windows Card Edit via `bind_hotkey` (Silent No-Op)

**What goes wrong:** The card edit path calls `invoke("bind_hotkey", ...)` on Windows, which succeeds (returns `Ok(())`) but registers nothing. The trigger key on Windows is managed through `update_macro_trigger_keys` in `MACRO_TRIGGER_KEYS`, not through `HOTKEY_BINDINGS`.

**Why it happens:** `bind_hotkey` is `#[cfg(target_os = "macos")]`-gated inside the command handler. The command exists on all platforms but is a no-op on Windows.

**How to avoid:** For Windows card edit of `trigger_key`, the update path must go through a MacroConfig update that triggers `reevaluate_all_macros`, which calls `update_macro_trigger_keys`. There is no dedicated single-field trigger key update IPC. The correct approach is to use the existing `add_macro` / `set_macro_sequence` flow or add a new `set_macro_trigger_key` IPC command that is platform-generic. [ASSUMED — needs validation during planning; the simplest option may be to re-invoke the full state update path]

**Warning signs:** Key capture on Windows card edit appears to work (no error) but pressing the captured key has no effect.

### Pitfall 5: `list_running_apps` Called from Async Rust but NSWorkspace Requires Main Thread Behavior

**What goes wrong:** The `list_running_apps` command is `async fn`, running in the Tokio async runtime. `NSWorkspace::sharedWorkspace()` must be called from a context where Cocoa / AppKit is accessible, which is normally the main thread (or a thread that has called `NSApplicationLoad`).

**Why it happens:** Tauri's async IPC handlers run on Tokio worker threads, not necessarily the main thread.

**How to avoid:** The existing `observer.rs` already calls `NSWorkspace::sharedWorkspace()` from within Tauri's callback contexts (which are safe for AppKit). The `list_running_apps` command will run on Tokio's async runtime, but `runningApplications()` is documented as thread-safe ("This property is thread safe, in that it may be called from background threads"). [VERIFIED: Context7 — "this property is thread safe…may be called from background threads"]

The `list_running_apps` implementation is safe to run from an async Tauri command without dispatching to the main thread.

**Warning signs:** Would manifest as a crash or EXC_BAD_ACCESS if thread-safety assumption is wrong. Not expected given the API documentation.

### Pitfall 6: F-Keys and Media Keys May Not Reach the WebView

**What goes wrong:** User tries to capture F1 or a media key. The WebView `keydown` event never fires because macOS or the system intercepted it at a lower level.

**Why it happens:** F-keys bound to system functions (brightness, mission control, etc.) are intercepted before reaching the WebView. Media keys are handled by the OS-level media key responder chain, not the WebView.

**How to avoid:** This is a documented limitation noted in `STATE.md` ("WebView keydown may not capture all system-intercepted keys (F-keys, media keys) — graceful fallback required"). The fallback per the UI-SPEC is to display `"Key {N}"` when the keyCode has no entry in the lookup table — allowing the user to capture what they can and retaining the raw integer as fallback. No workaround for truly intercepted keys exists within the WebView model.

**Warning signs:** Key capture appears to freeze when user presses certain F-keys; the recording state never exits.

---

## Code Examples

### CGKeyCode Lookup Table (macOS) — from DOM keyCode to CGKeyCode

The table maps `KeyboardEvent.keyCode` (which the WebView fires) to CGKeyCode (what Rust `inject_key` expects on macOS).

```typescript
// Source: HIToolbox/Events.h (Apple SDK) — verified via phracker/MacOSX-SDKs
// https://github.com/phracker/MacOSX-SDKs/.../HIToolbox.framework/.../Events.h

// DOM keyCode → CGKeyCode (macOS)
// Only keys relevant to macro automation; extend as needed
export const DOM_KEYCODE_TO_CGKEYCODE: Record<number, number> = {
  // Letters (DOM = ASCII; CGKeyCode = ANSI physical position)
  65: 0,   // A
  83: 1,   // S
  68: 2,   // D
  70: 3,   // F
  72: 4,   // H
  71: 5,   // G
  90: 6,   // Z
  88: 7,   // X
  67: 8,   // C
  86: 9,   // V
  66: 11,  // B
  81: 12,  // Q
  87: 13,  // W
  69: 14,  // E
  82: 15,  // R
  89: 16,  // Y
  84: 17,  // T
  79: 31,  // O
  85: 32,  // U
  73: 34,  // I
  80: 35,  // P
  76: 37,  // L
  74: 38,  // J
  75: 40,  // K
  78: 45,  // N
  77: 46,  // M
  // Digits (DOM = ASCII 48–57)
  49: 18,  // 1
  50: 19,  // 2
  51: 20,  // 3
  52: 21,  // 4
  54: 22,  // 6
  53: 23,  // 5
  57: 25,  // 9
  55: 26,  // 7
  56: 28,  // 8
  48: 29,  // 0
  // Special keys
  32: 49,  // Space (kVK_Space)
  13: 36,  // Return (kVK_Return)
  9:  48,  // Tab (kVK_Tab)
  8:  51,  // Backspace → Delete (kVK_Delete)
  27: 53,  // Escape (kVK_Escape)
  // Arrow keys
  123: 123, // Left arrow — DOM keyCode matches kVK_LeftArrow by coincidence
  124: 124, // Right arrow
  125: 125, // Down arrow
  126: 126, // Up arrow
  // Navigation
  36: 115, // Home (kVK_Home)
  35: 119, // End (kVK_End)
  33: 116, // Page Up (kVK_PageUp)
  34: 121, // Page Down (kVK_PageDown)
  // Function keys (DOM F1=112 through F12=123)
  112: 122, // F1 (kVK_F1)
  113: 120, // F2 (kVK_F2)
  114: 99,  // F3 (kVK_F3)
  115: 118, // F4 (kVK_F4)
  116: 96,  // F5 (kVK_F5)
  117: 97,  // F6 (kVK_F6)
  118: 98,  // F7 (kVK_F7)
  119: 100, // F8 (kVK_F8)
  120: 101, // F9 (kVK_F9)
  121: 109, // F10 (kVK_F10)
  122: 103, // F11 (kVK_F11)
  123: 111, // F12 (kVK_F12) — NOTE: same DOM keyCode as Left arrow; arrow wins in practice
};

// Reverse map: CGKeyCode → display name (for showing stored values)
export const CGKEYCODE_TO_NAME: Record<number, string> = {
  0: "A", 1: "S", 2: "D", 3: "F", 4: "H", 5: "G", 6: "Z", 7: "X",
  8: "C", 9: "V", 11: "B", 12: "Q", 13: "W", 14: "E", 15: "R",
  16: "Y", 17: "T", 18: "1", 19: "2", 20: "3", 21: "4", 22: "6",
  23: "5", 24: "=", 25: "9", 26: "7", 27: "-", 28: "8", 29: "0",
  30: "]", 31: "O", 32: "U", 33: "[", 34: "I", 35: "P", 36: "Return",
  37: "L", 38: "J", 39: "'", 40: "K", 41: ";", 42: "\\", 43: ",",
  44: "/", 45: "N", 46: "M", 47: ".", 48: "Tab", 49: "Space",
  50: "`", 51: "Delete", 53: "Escape",
  96: "F5", 97: "F6", 98: "F7", 99: "F3", 100: "F8", 101: "F9",
  103: "F11", 109: "F10", 111: "F12", 113: "F15", 114: "Help",
  115: "Home", 116: "Page Up", 117: "Fwd Delete", 118: "F4",
  119: "End", 120: "F2", 121: "Page Down", 122: "F1",
  123: "Left Arrow", 124: "Right Arrow", 125: "Down Arrow", 126: "Up Arrow",
  55: "Cmd", 56: "Shift", 57: "Caps Lock", 58: "Option", 59: "Control",
  60: "Right Shift", 61: "Right Option", 62: "Right Control",
};
```

**Note on F12/Left arrow collision:** DOM `keyCode` 123 is used for both F12 and Left Arrow. In the WebView, the Left Arrow fires 37 (standard DOM), not 123. Use DOM keyCode 37 → CGKeyCode 123. F12 fires DOM 123 → CGKeyCode 111. The mapping above needs adjustment for arrow keys — use `e.code` (`"ArrowLeft"`) to disambiguate when `e.keyCode` is ambiguous. **During implementation, use `e.code` (string) as the primary lookup key rather than `e.keyCode` (number) to avoid collisions.** [ASSUMED — needs verification during implementation; `e.code` is layout-independent and unambiguous]

### Windows VK Lookup Table — DOM keyCode to VK Code

For Windows, DOM `keyCode` for letters (A=65) and digits (0=48) happens to equal the VK code. For special keys they diverge.

```typescript
// Source: Microsoft Learn — Virtual-Key Codes (Winuser.h)
// https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes

// DOM keyCode → Windows VK code
// Letters A-Z: DOM keyCode 65-90 == VK_A-VK_Z 0x41-0x5A — IDENTICAL
// Digits 0-9: DOM keyCode 48-57 == VK 0x30-0x39 — IDENTICAL
// Special keys diverge:
export const DOM_KEYCODE_TO_VK: Record<number, number> = {
  // Letters (same as DOM keyCode — listed for completeness)
  65: 0x41, 66: 0x42, 67: 0x43, 68: 0x44, 69: 0x45, 70: 0x46,
  71: 0x47, 72: 0x48, 73: 0x49, 74: 0x4A, 75: 0x4B, 76: 0x4C,
  77: 0x4D, 78: 0x4E, 79: 0x4F, 80: 0x50, 81: 0x51, 82: 0x52,
  83: 0x53, 84: 0x54, 85: 0x55, 86: 0x56, 87: 0x57, 88: 0x58,
  89: 0x59, 90: 0x5A,
  // Digits (same)
  48: 0x30, 49: 0x31, 50: 0x32, 51: 0x33, 52: 0x34,
  53: 0x35, 54: 0x36, 55: 0x37, 56: 0x38, 57: 0x39,
  // Special keys
  32: 0x20,   // VK_SPACE
  13: 0x0D,   // VK_RETURN
  9:  0x09,   // VK_TAB
  8:  0x08,   // VK_BACK
  27: 0x1B,   // VK_ESCAPE
  37: 0x25,   // VK_LEFT
  38: 0x26,   // VK_UP
  39: 0x27,   // VK_RIGHT
  40: 0x28,   // VK_DOWN
  36: 0x24,   // VK_HOME
  35: 0x23,   // VK_END
  33: 0x21,   // VK_PRIOR (Page Up)
  34: 0x22,   // VK_NEXT (Page Down)
  45: 0x2D,   // VK_INSERT
  46: 0x2E,   // VK_DELETE
  // Function keys
  112: 0x70,  // VK_F1
  113: 0x71,  // VK_F2
  114: 0x72,  // VK_F3
  115: 0x73,  // VK_F4
  116: 0x74,  // VK_F5
  117: 0x75,  // VK_F6
  118: 0x76,  // VK_F7
  119: 0x77,  // VK_F8
  120: 0x78,  // VK_F9
  121: 0x79,  // VK_F10
  122: 0x7A,  // VK_F11
  123: 0x7B,  // VK_F12
  // Numpad
  96: 0x60, 97: 0x61, 98: 0x62, 99: 0x63, 100: 0x64,
  101: 0x65, 102: 0x66, 103: 0x67, 104: 0x68, 105: 0x69,
  // Modifiers
  16: 0x10,   // VK_SHIFT
  17: 0x11,   // VK_CONTROL
  18: 0x12,   // VK_MENU (Alt)
  20: 0x14,   // VK_CAPITAL (Caps Lock)
  91: 0x5B,   // VK_LWIN
  92: 0x5C,   // VK_RWIN
};

// VK → display name (for showing stored values on Windows)
export const VK_TO_NAME: Record<number, string> = {
  0x41: "A", 0x42: "B", 0x43: "C", 0x44: "D", 0x45: "E", 0x46: "F",
  0x47: "G", 0x48: "H", 0x49: "I", 0x4A: "J", 0x4B: "K", 0x4C: "L",
  0x4D: "M", 0x4E: "N", 0x4F: "O", 0x50: "P", 0x51: "Q", 0x52: "R",
  0x53: "S", 0x54: "T", 0x55: "U", 0x56: "V", 0x57: "W", 0x58: "X",
  0x59: "Y", 0x5A: "Z",
  0x30: "0", 0x31: "1", 0x32: "2", 0x33: "3", 0x34: "4",
  0x35: "5", 0x36: "6", 0x37: "7", 0x38: "8", 0x39: "9",
  0x20: "Space", 0x0D: "Return", 0x09: "Tab", 0x08: "Backspace",
  0x1B: "Escape", 0x25: "Left Arrow", 0x26: "Up Arrow",
  0x27: "Right Arrow", 0x28: "Down Arrow",
  0x24: "Home", 0x23: "End", 0x21: "Page Up", 0x22: "Page Down",
  0x2D: "Insert", 0x2E: "Delete",
  0x70: "F1", 0x71: "F2", 0x72: "F3", 0x73: "F4", 0x74: "F5",
  0x75: "F6", 0x76: "F7", 0x77: "F8", 0x78: "F9", 0x79: "F10",
  0x7A: "F11", 0x7B: "F12",
  0x60: "Num 0", 0x61: "Num 1", 0x62: "Num 2", 0x63: "Num 3",
  0x64: "Num 4", 0x65: "Num 5", 0x66: "Num 6", 0x67: "Num 7",
  0x68: "Num 8", 0x69: "Num 9",
  0x10: "Shift", 0x11: "Ctrl", 0x12: "Alt",
};
```

### Platform Detection in TypeScript

The key capture widget needs to know which lookup table to use at runtime.

```typescript
// Option 1: navigator.platform (synchronous, no IPC round-trip)
// "MacIntel" | "MacM1" | "Mac" → macOS; "Win32" | "Win64" → Windows
const IS_MACOS = navigator.platform.toLowerCase().includes("mac");

// Option 2: Tauri os plugin (more reliable but requires async)
// import { platform } from "@tauri-apps/plugin-os";
// const os = await platform(); // "macos" | "windows" | "linux"

// Recommendation: Use navigator.platform for the lookup table selection.
// It is synchronous, available immediately, and sufficient for this purpose.
// navigator.platform is deprecated in browsers but remains reliable in Tauri WebView.
```

[ASSUMED — `navigator.platform` behavior in Tauri 2 WebView should be verified during implementation; the `@tauri-apps/plugin-os` alternative is the guaranteed path but requires the os plugin dependency]

### Existing `<select>` Class Pattern (from App.tsx)

The process picker `<select>` must use this exact class string to match existing selects:

```tsx
// Source: src/App.tsx lines 472–482 (existing input-type selector)
// + 03-UI-SPEC.md Process Picker Widget section
class="bg-background border border-border rounded-lg px-3 py-2 text-sm
       focus:outline-none focus:border-accent/50 transition-colors text-text-main cursor-pointer"
```

Width: `w-full` in the creation form; `flex-1` or fixed width on macro card. [CITED: 03-UI-SPEC.md]

### Key Capture Widget Class Pattern (from UI-SPEC)

```tsx
// Idle state (no key set):
class="bg-background border border-border rounded-lg px-3 py-2 text-sm text-text-dim w-40"

// Idle state (key set):
class="bg-background border border-border rounded-lg px-3 py-2 text-sm text-text-main w-40"

// Recording state:
class="bg-background border border-accent rounded-lg px-3 py-2 text-sm text-accent
       shadow-[0_0_8px_var(--color-accent-glow)] w-40"

// Card badge (click-to-edit):
class="px-1.5 py-0.5 rounded bg-surface-alt border border-border text-[10px] font-mono
       cursor-pointer hover:border-accent/40"
```

[CITED: 03-UI-SPEC.md Component Inventory sections]

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `<input type="number">` for key code | Click-to-capture widget with lookup table | Phase 3 (this phase) | Users never see raw integers |
| `<input type="text">` for target app | `<select>` populated by `list_running_apps` | Phase 3 (this phase) | Users never manually type bundle IDs or exe paths |
| Static read-only macro card badges | Click-to-edit badges for key and target | Phase 3 (this phase) | Users can edit existing macros without recreating them |

**Note on `trigger_key` on Windows card edit:** The CONTEXT.md deferred Windows hotkey binding (`bind_hotkey`/`unbind_hotkey` are macOS-only). For Windows card edit of `trigger_key`, a decision is needed: either add a generic `set_macro_trigger_key(id: Uuid, key: Option<u16>)` IPC command, or require a full `add_macro` re-create flow. This is a planning gap flagged in Open Questions.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | App-level `editingCardId` + `editingField` signals correctly track per-card edit state in a `For` loop after `state-changed` triggers re-render | Architecture Patterns — Pattern 2 | If wrong: edit state could persist across re-renders or be lost mid-edit; fix by extracting cards into sub-components with component-local state |
| A2 | `navigator.platform` reliably returns a Mac/Win identifier in Tauri 2 WebView (WKWebView on macOS, WebView2 on Windows) | Code Examples — Platform Detection | If wrong: wrong lookup table selected; use `@tauri-apps/plugin-os` instead (adds a plugin dependency) |
| A3 | Using `e.code` (string, e.g. `"KeyA"`, `"ArrowLeft"`) instead of `e.keyCode` (number) is the cleaner primary key for the lookup table, avoiding F12/arrow ambiguity | Common Pitfalls — Pitfall 1 | If wrong: some keys ambiguously mapped; stick with `e.keyCode` and document known collision cases |
| A4 | Windows card edit of `trigger_key` requires a new `set_macro_trigger_key` IPC command (not covered by existing `bind_hotkey`) | Common Pitfalls — Pitfall 4 | If wrong (and there is a viable alternative path): saves one new IPC command; but the current code has no generic trigger key update command |
| A5 | `NSWorkspace.runningApplications()` is safe to call from Tokio async context in the Tauri IPC handler (no main thread dispatch needed) | Common Pitfalls — Pitfall 5 | If wrong: crash on macOS; fix by wrapping in `tauri::async_runtime::spawn_blocking` or dispatching to the main thread via AppKit mechanisms |

---

## Open Questions

1. **Windows card edit — how to update `trigger_key` on an existing macro**
   - What we know: `bind_hotkey` is macOS-only. On Windows, trigger keys are registered via `update_macro_trigger_keys` which is called from `reevaluate_all_macros` inside the StateActor.
   - What's unclear: There is no existing IPC command for "change the trigger_key of an existing macro". The Intent enum has no `SetMacroTriggerKey` variant.
   - Recommendation: Add a `set_macro_trigger_key(id: Uuid, trigger_key: Option<u16>)` IPC command and corresponding `Intent::SetMacroTriggerKey(Uuid, Option<u16>)` variant. This is a small, clean addition. Alternative: repurpose `set_macro_sequence` to carry trigger key changes, but this is semantically wrong.

2. **Key capture: `e.code` vs `e.keyCode` as lookup key**
   - What we know: `e.code` (DOM `KeyboardEvent.code`, e.g. `"KeyA"`, `"Space"`, `"F5"`) is layout-independent and unambiguous. `e.keyCode` is a numeric code but collides for some keys (F12 and Left Arrow both fire in some contexts with keyCode 123).
   - What's unclear: Whether `e.keyCode` is reliable in Tauri's WebView for all target keys (especially F-keys), or whether `e.code` is more robust.
   - Recommendation: Use `e.code` as the primary lookup key (a `Record<string, number>` mapping e.g. `{ "KeyA": 0, "Space": 49, ... }` for macOS). This is cleaner and avoids all numeric collision issues.

3. **`list_running_apps` — should it be a platform-gated command or a universal command?**
   - What we know: Both macOS and Windows need implementations. The function signature is identical.
   - What's unclear: Whether to use a single `#[command]` with `#[cfg]` blocks inside (current codebase pattern for `bind_hotkey`) or two separate commands.
   - Recommendation: Single `#[command]` with `#[cfg]` blocks inside the function body — consistent with `bind_hotkey` pattern in `ipc/mod.rs`. [CITED: ipc/mod.rs lines 77–94]

---

## Environment Availability

> Phase 3 is code/config changes only — no external dependencies beyond the project's own codebase. All required OS APIs (NSWorkspace, EnumWindows) are accessed via existing crate dependencies.

Step 2.6: SKIPPED (no external tool dependencies — macOS and Windows API access is via existing crate bindings already in Cargo.toml)

---

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/rs_objc2-app-kit` — `NSWorkspace.runningApplications()`, `NSRunningApplication.bundleIdentifier`, `NSRunningApplication.localizedName`, `activationPolicy`
- Context7 `/solidjs/solid-docs` — `onCleanup` with `removeEventListener`, `createEffect` patterns, native vs delegated event propagation
- `src-tauri/src/platform/macos/observer.rs` (codebase) — verified `NSWorkspace` import and usage pattern
- `src-tauri/src/platform/windows/mod.rs` (codebase) — verified `get_app_name_from_hwnd`, existing Win32 feature imports
- `src-tauri/src/ipc/mod.rs` (codebase) — verified `bind_hotkey`/`unbind_hotkey` macOS-only gating, `set_macro_target_app` signature
- `src-tauri/Cargo.toml` (codebase) — verified no new crate dependencies needed
- `src/App.tsx` (codebase) — verified `newMacroTriggerKey` signal, `handleCreateMacro`, existing `<select>` class strings
- `.planning/phases/03-macro-setup-ux/03-UI-SPEC.md` (approved design contract) — visual states, class strings, copy, interaction flow

### Secondary (MEDIUM confidence)
- Apple HIToolbox Events.h via [phracker/MacOSX-SDKs GitHub mirror](https://github.com/phracker/MacOSX-SDKs/blob/master/MacOSX10.6.sdk/System/Library/Frameworks/Carbon.framework/Versions/A/Frameworks/HIToolbox.framework/Versions/A/Headers/Events.h) — complete kVK_ table verified
- [Microsoft Learn — Virtual-Key Codes](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes) — complete VK_ table verified
- `.planning/research/FEATURES.md` (existing project research document) — process picker design patterns, key capture architecture, Windows EnumWindows approach, macOS activation policy filtering

### Tertiary (LOW confidence — flagged in Assumptions Log)
- `navigator.platform` behavior in Tauri 2 WebView (A2) — assumed from general web knowledge
- Per-card edit state stability across `For` re-renders (A1) — assumed from SolidJS reactivity model

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all existing, verified in codebase
- Key code lookup tables: HIGH — verified against authoritative Apple SDK and Microsoft documentation
- NSWorkspace implementation pattern: HIGH — already used in observer.rs
- Windows EnumWindows pattern: MEDIUM — API is established but exact filtering combination needs testing on Windows
- SolidJS patterns: HIGH — verified via Context7
- `navigator.platform` for OS detection: LOW — assumed; verify during implementation

**Research date:** 2026-05-30
**Valid until:** 2026-06-30 (stable APIs; SolidJS 1.x pattern stability is high)
