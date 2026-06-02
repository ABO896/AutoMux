# Phase 5: macOS Permissions & Reliability — Pattern Map

**Mapped:** 2026-06-01
**Files analyzed:** 3
**Analogs found:** 3 / 3

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src-tauri/src/ipc/mod.rs` | IPC handler (modify) | request-response | `request_accessibility` handler in same file (lines 171–185) | exact |
| `src/App.tsx` | component (modify) | request-response + event-driven | Existing `handleRequestAccess` + 3s polling `createEffect` in same file | exact |
| `src-tauri/Cargo.toml` | config (modify) | — | `block2 = "0.6.2"` line in same file (remove adjacent `cocoa` line) | exact |

---

## Pattern Assignments

### `src-tauri/src/ipc/mod.rs` — `check_accessibility` handler (D-03)

**Change:** Add `initialize_tap()` call when the silent poll returns `true`, mirroring the existing pattern in `request_accessibility`.

**Analog:** `request_accessibility` handler — `src-tauri/src/ipc/mod.rs` lines 171–185.

**Existing analog — full pattern to copy from** (lines 171–185):
```rust
/// Request Accessibility permissions from the OS.
///
/// - Shows the macOS system dialog prompting the user to grant access.
/// - If already trusted, initializes the CGEventTap (idempotent).
/// - Returns `true` if the process currently has Accessibility permissions.
///
/// The SolidJS frontend should call this on launch and display
/// a "Permissions Required" indicator when it returns `false`.
#[command]
pub async fn request_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let granted = crate::platform::macos::check_accessibility_permissions(true);
        if granted {
            crate::platform::macos::observer::initialize_tap();
        }
        Ok(granted)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows/Linux: no accessibility gate
        Ok(true)
    }
}
```

**Target — `check_accessibility` before change** (lines 188–200):
```rust
/// Silent check: returns current accessibility status without prompting.
#[command]
pub async fn check_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(crate::platform::macos::check_accessibility_permissions(
            false,
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}
```

**After change — copy the `if granted { initialize_tap(); }` block from the analog:**
```rust
/// Silent check: returns current accessibility status without prompting.
/// D-03: arms CGEventTap on post-launch grant (idempotent — TAP_INITIALIZED AtomicBool guard).
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

**Why this is safe:** `initialize_tap()` performs an AtomicBool CAS then `thread::spawn(...)` and returns immediately. It does not block the Tokio executor. The `TAP_INITIALIZED` guard makes repeated calls idempotent. This is identical to what `request_accessibility` already does on line 176.

---

### `src/App.tsx` — `accessibilityPending` signal + 3-way permission UI (D-01, D-02)

**Change:** Add one signal, update `handleRequestAccess`, update the 3s polling `createEffect`, and replace the 2-branch accessibility `Show` with a 3-branch version.

#### Signal declaration pattern

**Analog — existing signals block** (lines 78–82):
```tsx
const [state, setState] = createSignal<AppState | null>(null);
const [accessibility, setAccessibility] = createSignal<boolean | null>(null);
const [activeApp, setActiveApp] = createSignal<string | null>(null);
const [, setLoading] = createSignal(true);
const [activeTab, setActiveTab] = createSignal<Tab>("dashboard");
```

**New signal to add directly after `accessibility` signal (line 79):**
```tsx
const [accessibilityPending, setAccessibilityPending] = createSignal(false);
```

**Timeout handle pattern — non-reactive closure variable (analog: line 75):**
```tsx
// Module-level listener ref — survives across renders; prevents double-attach (T-03-08)
let _keyCaptureListener: ((e: KeyboardEvent) => void) | null = null;
```
Copy this pattern for the pending timeout handle — place it alongside `_keyCaptureListener` at module level (outside `App()`):
```tsx
// Pending-approval timeout handle — not reactive; no re-render needed on assignment.
let _pendingTimeoutId: ReturnType<typeof setTimeout> | null = null;
```

#### `clearPending` helper — add inside `App()` near other action helpers

```tsx
function clearPending() {
  setAccessibilityPending(false);
  if (_pendingTimeoutId !== null) {
    clearTimeout(_pendingTimeoutId);
    _pendingTimeoutId = null;
  }
}
```

#### `handleRequestAccess` — update existing function (lines 182–189)

**Before (lines 182–189):**
```tsx
async function handleRequestAccess() {
  try {
    const granted = await invoke<boolean>("request_accessibility");
    setAccessibility(granted);
  } catch (e) {
    console.error("Accessibility request failed:", e);
  }
}
```

**After — add pending state (D-01) + timeout (D-02) + error-path cleanup (pitfall 2):**
```tsx
async function handleRequestAccess() {
  setAccessibilityPending(true);
  _pendingTimeoutId = setTimeout(() => {
    setAccessibilityPending(false); // D-02: 30s timeout — revert to "not granted"
    _pendingTimeoutId = null;
  }, 30_000);
  try {
    const granted = await invoke<boolean>("request_accessibility");
    setAccessibility(granted);
    if (granted) clearPending(); // instant-approval path
    // If false, stay pending — 3s poll clears when granted
  } catch (e) {
    console.error("Accessibility request failed:", e);
    clearPending(); // error path: revert (pitfall 2)
  }
}
```

#### 3s polling `createEffect` — update existing effect (lines 160–170)

**Before (lines 160–170):**
```tsx
// Poll accessibility every 3s
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

**After — add `clearPending()` on grant (D-02):**
```tsx
// Poll accessibility every 3s
createEffect(() => {
  const interval = setInterval(async () => {
    try {
      const ok = await invoke<boolean>("check_accessibility");
      setAccessibility(ok);
      if (ok) clearPending(); // D-02: clear pending state when polling confirms grant
    } catch (_) {
      /* ignore */
    }
  }, 3000);
  onCleanup(() => clearInterval(interval));
});
```

#### Accessibility `Show` block — update existing UI (lines 451–481)

**Analog — existing 2-branch structure (lines 451–481):**
```tsx
<Show
  when={accessibility() === true}
  fallback={
    <div class="flex items-center gap-1.5">
      <div class="w-2 h-2 rounded-full bg-danger status-pulse shadow-[0_0_6px_var(--color-danger-glow)]" />
      <span class="text-[11px] text-danger font-medium">
        Denied
      </span>
    </div>
  }
>
  <div class="flex items-center gap-1.5">
    <div class="w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)]" />
    <span class="text-[11px] text-success font-medium">
      Granted
    </span>
  </div>
</Show>
```

And below it, the conditional "Grant Access" button (lines 471–480):
```tsx
<Show when={accessibility() === false}>
  <button
    id="btn-request-access"
    onClick={handleRequestAccess}
    class="mt-3 w-full py-1.5 rounded-lg bg-accent/10 border border-accent/30 text-accent text-xs font-medium
           hover:bg-accent/20 hover:border-accent/50 transition-all duration-200 cursor-pointer"
  >
    Grant Access
  </button>
</Show>
```

**After — 3-branch rendering (pitfall 3: `accessibilityPending` checked first):**

Replace the `Show` status badge with a 3-way branch:
```tsx
<Show
  when={accessibility() === true}
  fallback={
    <Show
      when={accessibilityPending()}
      fallback={
        <div class="flex items-center gap-1.5">
          <div class="w-2 h-2 rounded-full bg-danger status-pulse shadow-[0_0_6px_var(--color-danger-glow)]" />
          <span class="text-[11px] text-danger font-medium">
            Denied
          </span>
        </div>
      }
    >
      <div class="flex items-center gap-1.5">
        <div class="w-2 h-2 rounded-full bg-warning status-pulse shadow-[0_0_6px_var(--color-warning-glow)]" />
        <span class="text-[11px] text-warning font-medium">
          Pending…
        </span>
      </div>
    </Show>
  }
>
  <div class="flex items-center gap-1.5">
    <div class="w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)]" />
    <span class="text-[11px] text-success font-medium">
      Granted
    </span>
  </div>
</Show>
```

The "Grant Access" button `Show` block is unchanged — it already guards on `accessibility() === false`. When pending, `accessibility()` is still `false` (the backend returned false), so the button remains visible. That is acceptable — the pending indicator above communicates the in-progress state. If the desired behaviour is to hide the button while pending, add `&& !accessibilityPending()` to its `when` condition:
```tsx
<Show when={accessibility() === false && !accessibilityPending()}>
  ...
</Show>
```
This is Claude's discretion per the CONTEXT.md. Either approach is valid.

**Styling note:** `text-warning` and `bg-warning` follow the same color-token convention as `text-danger`/`bg-danger` and `text-success`/`bg-success` seen in lines 455–466. If `--color-warning-glow` does not exist in `App.css`, use `shadow-[0_0_6px_theme(colors.warning/50)]` or omit the glow entirely — the status-pulse animation on the dot already conveys "in progress".

---

### `src-tauri/Cargo.toml` — remove `cocoa = "0.26.1"` (D-05)

**Change:** Delete one line from the `[target.'cfg(target_os = "macos")'.dependencies]` block.

**Existing block (lines 27–37):**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.24.0"
core-foundation = "0.10.0"
core-foundation-sys = "0.8.0"
objc = "0.2.7"
cocoa = "0.26.1"        # <-- REMOVE THIS LINE
objc2 = "0.6.4"
objc2-app-kit = "0.3.2"
objc2-foundation = "0.3.2"
block2 = "0.6.2"        # <-- KEEP: block2 is unrelated to cocoa's block v0.1.6
```

**After:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.24.0"
core-foundation = "0.10.0"
core-foundation-sys = "0.8.0"
objc = "0.2.7"
objc2 = "0.6.4"
objc2-app-kit = "0.3.2"
objc2-foundation = "0.3.2"
block2 = "0.6.2"
```

No source files need updating — `grep -rn "use cocoa::" src-tauri/src/` returns no results (verified in RESEARCH.md Q4).

---

## Shared Patterns

### `cfg(target_os)` platform branching
**Source:** `src-tauri/src/ipc/mod.rs` — every handler that has macOS-specific logic.
**Apply to:** The `check_accessibility` modification.
**Pattern:**
```rust
#[cfg(target_os = "macos")]
{
    // macOS implementation
}
#[cfg(not(target_os = "macos"))]
{
    Ok(true) // Windows/Linux: feature not gated
}
```
All three IPC handlers that touch platform code (`bind_hotkey`, `request_accessibility`, `check_accessibility`) follow this pattern. The D-03 change must preserve the `#[cfg(not(target_os = "macos"))]` branch returning `Ok(true)`.

### SolidJS `Show` for conditional rendering
**Source:** `src/App.tsx` throughout the JSX.
**Apply to:** The 3-branch accessibility status indicator.
**Convention:** Never use ternary JSX with non-trivial subtrees (per CLAUDE.md). Nest `Show` components instead. The pattern is already used for the existing 2-branch status indicator (lines 451–481).

### Non-reactive module-level handle
**Source:** `src/App.tsx` line 75 — `_keyCaptureListener`.
**Apply to:** `_pendingTimeoutId` timeout handle.
**Rule:** Values that are not reactive data (do not need to trigger re-renders) are stored as plain module-level or closure variables, not signals. The naming convention is `_camelCase` with underscore prefix for module-level side-effect handles.

### `onCleanup` inside `createEffect`
**Source:** `src/App.tsx` lines 154–157, 169 — both polling effects call `onCleanup`.
**Apply to:** No new `createEffect` is added in Phase 5, but if one is added, `onCleanup` is mandatory.

---

## No Analog Found

All three files are direct modifications of existing files. No new files are introduced. All patterns are taken from the same files being edited.

---

## Metadata

**Analog search scope:** `src-tauri/src/ipc/mod.rs`, `src/App.tsx`, `src-tauri/Cargo.toml`
**Files scanned:** 3 source files + CONTEXT.md + RESEARCH.md
**Pattern extraction date:** 2026-06-01
