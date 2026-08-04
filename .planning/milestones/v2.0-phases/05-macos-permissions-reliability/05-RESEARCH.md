# Phase 5: macOS Permissions & Reliability — Research

**Researched:** 2026-06-01
**Domain:** macOS Accessibility API, CGEventTap threading, SolidJS reactive state, Rust Cargo dependency graph
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** After "Request Access" is clicked, show a "Pending approval…" indicator instead of false "not granted". The backend returns `false` before the user acts on the OS dialog — that `false` must not propagate to the UI.
- **D-02:** Pending state clears when polling returns `true` OR after a 30-second timeout. Timeout reverts UI to "not granted".
- **D-03:** `check_accessibility` backend handler is responsible for arming the tap on post-launch grant. Before returning `true`, it checks whether the CGEventTap is initialized; if not, it calls `initialize_tap()`. Covers both the 3s polling path and any direct `check_accessibility` calls.
- **D-04:** Tap arming post-grant is silent — the permission indicator updates to "granted" and macros start working. No toast or banner.
- **D-05:** Fix BUILD-02 by removing `cocoa = "0.26.1"` from `Cargo.toml`. No direct `use cocoa::...` imports exist. `block2 = "0.6.2"` remains.

### Claude's Discretion
- Exact wording/styling of the "Pending approval…" indicator (chip, italic text, spinner — whatever fits existing permission UI).
- Whether `initialize_tap()` is called synchronously inside `check_accessibility` IPC handler or dispatched to a background task (implementation choice based on CGEventTap threading constraints — see Research Question 1 below).

### Deferred Ideas (OUT OF SCOPE)
- `objc 0.2.7` legacy cleanup
- broader `cocoa` → `objc2` migration audit
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PERM-01 | macOS app correctly detects accessibility permission as granted after user clicks Request Access and approves the OS prompt — no false "not granted" state persists after approval | D-01/D-02 implement pending state; polling path (D-03) fires `initialize_tap()` when `check_accessibility` returns true |
| RELY-06 | Granting accessibility in System Settings directly (without clicking Request Access first) arms CGEventTap and activates macros without requiring an app restart | D-03: `check_accessibility` handler calls `initialize_tap()` if not yet initialized when it returns true |
| BUILD-02 | `block v0.1.6` macOS deprecation resolved | D-05: removing `cocoa = "0.26.1"` is the sole source of `block v0.1.6` per `cargo tree --invert block` |
</phase_requirements>

---

## Summary

Phase 5 makes three focused, self-contained changes: (1) a frontend state machine that adds a "pending" window between clicking "Request Access" and the OS dialog resolving; (2) a backend change to `check_accessibility` so it arms the CGEventTap whenever it detects a granted permission and the tap is not yet running; and (3) removing the unused `cocoa = "0.26.1"` Cargo dependency which is the sole source of `block v0.1.6`.

The CGEventTap threading model is the deepest concern. `initialize_tap()` spawns a new OS thread internally, so calling it from a Tokio async handler context is safe — no `CFRunLoop` is needed on the calling thread. The `TAP_INITIALIZED` AtomicBool CAS guard makes the call idempotent and concurrent-safe. The one subtlety is that `initialize_tap()` also resets `TAP_INITIALIZED` to `false` on failure (tap creation failure at line 457 of observer.rs), which means a post-launch retry from the polling path will work correctly even if the startup attempt failed.

The frontend change adds one new signal (`accessibilityPending`) and wires it into the existing 3s polling `createEffect` and the `handleRequestAccess` handler. No new `createEffect` blocks are needed — the pending flag is set in the click handler and cleared by the existing polling effect.

**Primary recommendation:** Implement D-03 as a synchronous call inside `check_accessibility` IPC handler (not a background task dispatch). `initialize_tap()` returns immediately after spawning the background OS thread — it does not block the Tokio executor.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Permission status display | Frontend (SolidJS) | — | Read-only projection of signals |
| "Pending" state machine | Frontend (SolidJS) | — | Local UI state, no backend representation needed |
| 30-second timeout | Frontend (SolidJS) | — | Timeout governs the UI indicator, not backend logic |
| Accessibility check (silent) | Backend (Rust IPC) | — | `AXIsProcessTrusted()` must run in process; frontend polls via `check_accessibility` |
| OS permission dialog | Backend (Rust IPC) | macOS OS | `AXIsProcessTrustedWithOptions` with prompt=true |
| CGEventTap arming | Backend (Rust platform layer) | — | Requires `CFRunLoop` background thread; backend-only |
| Cargo dependency cleanup | Build (Cargo.toml) | — | Remove `cocoa` entry; no code change needed |

---

## Standard Stack

All changes stay within the existing stack — no new libraries are introduced.

### Core (existing, no changes)
| Library | Current Version | Role in this phase |
|---------|----------------|-------------------|
| SolidJS | 1.9.3 | `createSignal`, `createEffect`, `onCleanup`, `Show` for pending state |
| Tauri `invoke` | ^2 | Frontend ↔ Rust IPC for `request_accessibility` and `check_accessibility` |
| `core-graphics` | 0.24.0 | `CGEventTap::new()` in `initialize_tap()` |
| `core-foundation` | 0.10.0 | `CFRunLoop` management inside tap thread |
| `block2` | 0.6.2 | Already used in observer.rs; remains after `cocoa` removal |

### Removed
| Library | Version | Reason |
|---------|---------|--------|
| `cocoa` | 0.26.1 | Unused direct dependency; sole source of `block v0.1.6` |

---

## Architecture Patterns

### System Architecture: Permission Grant Flow

```
User clicks "Grant Access"
         │
         ▼
  handleRequestAccess()          [Frontend: App.tsx]
  setAccessibilityPending(true)
  startPendingTimeout(30s)
         │
         ▼
  invoke("request_accessibility") ──► request_accessibility IPC
                                        check_accessibility_permissions(true)
                                        → shows OS dialog
                                        → returns current state (false if not yet approved)
                                        ◄── returns false
         │
         ▼
  setAccessibility(false)         [Frontend stays in pending state]
         │
         ▼ (3s poll tick)
  invoke("check_accessibility") ──► check_accessibility IPC          [D-03]
                                        check_accessibility_permissions(false)
                                        → AXIsProcessTrusted()
                                        if granted && !TAP_INITIALIZED:
                                          initialize_tap()             [spawns OS thread]
                                        returns true
                                        ◄── returns true
         │
         ▼
  setAccessibility(true)
  setAccessibilityPending(false)  [clear pending, cancel timeout]
  clearPendingTimeout()
         │
         ▼
  UI shows "Granted" indicator
```

### Recommended Project Structure

No structural changes. All edits are in existing files:
```
src/
└── App.tsx               — add accessibilityPending signal + UI branch

src-tauri/src/
├── ipc/mod.rs            — update check_accessibility handler (D-03)
└── Cargo.toml            — remove cocoa dependency (D-05)
```

### Pattern 1: Idempotent tap initialization (existing, verified)

```rust
// Source: src-tauri/src/platform/macos/observer.rs:217–262
pub fn initialize_tap() -> bool {
    // AtomicBool CAS: only one thread proceeds.
    if TAP_INITIALIZED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return true; // Already initialized — safe no-op.
    }
    thread::spawn(|| {
        // CGEventTap setup + CFRunLoop on its own OS thread.
        // On failure: TAP_INITIALIZED.store(false, ...) — enables retry.
    });
    true // Returns immediately; tap runs on the spawned thread.
}
```

**[VERIFIED: read src-tauri/src/platform/macos/observer.rs]**

Key property: `initialize_tap()` returns immediately after spawning. The CFRunLoop blocks only on the spawned OS thread, not the caller. Calling this from a Tokio async handler (`check_accessibility`) does not block the executor.

### Pattern 2: D-03 implementation in check_accessibility

```rust
// Source: src-tauri/src/ipc/mod.rs:189 (modified per D-03)
#[command]
pub async fn check_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let granted = crate::platform::macos::check_accessibility_permissions(false);
        if granted {
            crate::platform::macos::observer::initialize_tap(); // idempotent
        }
        Ok(granted)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}
```

This is safe because:
- `check_accessibility_permissions(false)` calls `AXIsProcessTrusted()` which is documented as thread-safe [ASSUMED: Apple documentation — not fetched in this session].
- `initialize_tap()` either spawns a thread (fast) or is a no-op CAS (instant).
- No blocking occurs on the Tokio thread.

### Pattern 3: accessibilityPending signal in SolidJS

```typescript
// Source: src/App.tsx — new signal alongside existing accessibility()
const [accessibility, setAccessibility] = createSignal<boolean | null>(null);
const [accessibilityPending, setAccessibilityPending] = createSignal(false);

// Timeout handle stored in a closure variable (not a signal — not reactive)
let pendingTimeoutId: ReturnType<typeof setTimeout> | null = null;

function clearPending() {
  setAccessibilityPending(false);
  if (pendingTimeoutId !== null) {
    clearTimeout(pendingTimeoutId);
    pendingTimeoutId = null;
  }
}

async function handleRequestAccess() {
  setAccessibilityPending(true);
  pendingTimeoutId = setTimeout(() => {
    setAccessibilityPending(false); // D-02: timeout → revert to "not granted"
    pendingTimeoutId = null;
  }, 30_000);
  try {
    const granted = await invoke<boolean>("request_accessibility");
    setAccessibility(granted);
    if (granted) clearPending();
    // If false, stay pending — polling will clear when granted
  } catch (e) {
    console.error("Accessibility request failed:", e);
    clearPending(); // Error path: revert
  }
}
```

Polling effect update (existing createEffect at line 160):
```typescript
createEffect(() => {
  const interval = setInterval(async () => {
    try {
      const ok = await invoke<boolean>("check_accessibility");
      setAccessibility(ok);
      if (ok) clearPending(); // D-02: clear pending when granted
    } catch (_) { /* ignore */ }
  }, 3000);
  onCleanup(() => clearInterval(interval));
});
```

**[ASSUMED: SolidJS signal pattern — consistent with existing codebase conventions in App.tsx]**

### Anti-Patterns to Avoid

- **Calling `CFRunLoop::run_current()` on the Tokio thread:** This would block the entire async executor indefinitely. `initialize_tap()` is safe precisely because the CFRunLoop call is inside `thread::spawn`, not on the caller's thread.
- **Storing `pendingTimeoutId` as a `createSignal`:** The timeout handle is not reactive data and does not need to trigger re-renders. A plain module-level or closure variable is appropriate (consistent with `_keyCaptureListener` pattern in App.tsx line 75).
- **Adding a second `createEffect` for pending state:** Hook into the existing polling effect — less complexity, same behavior.
- **Using `accessibility() === null` as the pending indicator:** `null` already means "not yet loaded" (initial state). A separate `accessibilityPending` signal is needed per D-01.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Idempotent tap init | Custom mutex/flag | `TAP_INITIALIZED` AtomicBool CAS (already exists) | SeqCst CAS guarantees exactly-once across concurrent calls |
| Accessibility check | Direct P/Invoke from TS | `AXIsProcessTrusted()` via `check_accessibility` IPC | AX API is process-context dependent; must run in Rust process |
| Pending timeout | Reactive signal with a timer | Plain `setTimeout` + close-over handle | No re-render needed; matches `_keyCaptureListener` pattern |

---

## Common Pitfalls

### Pitfall 1: `initialize_tap()` fails silently when permissions not yet granted at startup
**What goes wrong:** `start_observing()` in observer.rs only calls `initialize_tap()` if `check_accessibility_permissions(false)` returns true at startup. If the app launches without permission, the tap is never armed. Existing code handles this correctly by resetting `TAP_INITIALIZED` to `false` on failure, enabling post-launch retry.
**Why it happens:** CGEventTap creation fails with an error when Accessibility is denied — the `Err(_)` branch resets the AtomicBool.
**How to avoid:** D-03 precisely covers the post-launch grant path. No additional handling needed in `start_observing`.
**Warning signs:** After granting via System Settings, macros never activate and the tap is not in `ps -ax` output.

**[VERIFIED: read src-tauri/src/platform/macos/observer.rs:433–459]**

### Pitfall 2: Pending state not cleared on error path in handleRequestAccess
**What goes wrong:** If the `invoke("request_accessibility")` call throws (network error, Tauri panic), the pending indicator stays forever and the 30s timeout becomes the only cleanup.
**Why it happens:** The existing `handleRequestAccess` only has a `console.error` in the catch block.
**How to avoid:** Call `clearPending()` in the catch block of `handleRequestAccess`.

### Pitfall 3: `accessibility() === false` shows "Denied" AND "Grant Access" button during pending
**What goes wrong:** After clicking "Grant Access", `request_accessibility` returns `false` immediately. Without the pending signal, the UI immediately shows "Denied" again — the bug PERM-01 is trying to fix.
**Why it happens:** The backend returns current state, which is `false` before the user acts on the OS dialog.
**How to avoid:** The UI rendering must check `accessibilityPending()` first. When pending, show the pending indicator instead of the "Denied" + "Grant Access" UI.
**Render logic precedence:** `accessibilityPending() === true` → show pending; else `accessibility() === true` → show granted; else → show denied + button.

### Pitfall 4: `cocoa` removal might break transitive users
**What goes wrong:** If any other crate in the dependency graph that AutoMux uses requires `cocoa` as a direct dep, removing it from `Cargo.toml` only removes the root-level declaration. The dep would still compile (as a transitive dep of that other crate).
**Why it matters:** If cocoa is ONLY in Cargo.toml as a direct dep of automux, removing it eliminates the dep entirely.
**Verification:** `cargo tree --invert block` shows `block v0.1.6` → `cocoa v0.26.1` → `automux`. The only user of `block v0.1.6` is `cocoa`, and the only user of `cocoa` is automux (our root crate). Removing `cocoa` from Cargo.toml eliminates the entire chain.
**[VERIFIED: ran `cargo tree --invert block` and `cargo tree -p cocoa` in src-tauri/]**

---

## Code Examples

### Current `check_accessibility` IPC handler (before D-03)

```rust
// Source: src-tauri/src/ipc/mod.rs:189
#[command]
pub async fn check_accessibility() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(crate::platform::macos::check_accessibility_permissions(false))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}
```

### Current `request_accessibility` IPC handler (already has `initialize_tap()`)

```rust
// Source: src-tauri/src/ipc/mod.rs:171
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
    // ...
}
```

Note: `request_accessibility` already arms the tap if permission is granted at the moment of the dialog call. D-03 adds the same logic to `check_accessibility` so the 3s polling path also arms the tap.

### Existing accessibility section in App.tsx (lines 446–481, simplified)

```tsx
// Source: src/App.tsx:451–480
<Show
  when={accessibility() === true}
  fallback={
    <div>  {/* "Denied" indicator + "Grant Access" button */}
      <Show when={accessibility() === false}>
        <button onClick={handleRequestAccess}>Grant Access</button>
      </Show>
    </div>
  }
>
  <div> {/* "Granted" indicator */} </div>
</Show>
```

The pending indicator must be inserted as a third branch between "Granted" and "Denied" — the existing `Show` / fallback structure will need to be extended.

---

## Research Question Answers

### Q1: Is `initialize_tap()` safe to call post-launch from the IPC handler context?

**Yes — confirmed safe.** `initialize_tap()` returns immediately after `thread::spawn(...)`. The CFRunLoop that requires an OS thread runs entirely inside the spawned thread. The Tokio async executor is not blocked.

The `TAP_INITIALIZED` AtomicBool CAS (`compare_exchange(false, true, SeqCst, SeqCst)`) guarantees exactly one thread is ever spawned, even under concurrent calls from multiple polling ticks. Subsequent calls are instant no-ops returning `true`.

On failure (permission denied, CFMachPort creation error), the spawned thread resets `TAP_INITIALIZED` to `false`, allowing the next poll tick to retry. This retry path is precisely what D-03 relies on.

**[VERIFIED: read src-tauri/src/platform/macos/observer.rs:217–462]**

### Q2: What is the async/thread context of `check_accessibility`? Can it call `initialize_tap()` directly?

`check_accessibility` is a `#[command] pub async fn`. Tauri 2 dispatches IPC handlers on the Tokio multi-thread executor. `initialize_tap()` does:
1. AtomicBool CAS — instant
2. `thread::spawn(...)` — spawns an OS thread, returns handle immediately (the handle is dropped, thread detaches)
3. Returns `true`

None of these steps block the Tokio thread. Direct call is correct. No `spawn_blocking` or channel dispatch is needed.

**[VERIFIED: read src-tauri/src/ipc/mod.rs (request_accessibility already calls initialize_tap() directly in async context)]**

### Q3: How does the 3s polling effect work, and how does accessibilityPending integrate?

The existing effect (App.tsx:160–170) runs `setInterval` every 3 seconds, calls `check_accessibility`, and calls `setAccessibility(ok)`. The pending state integrates by:
1. `handleRequestAccess` sets `accessibilityPending(true)` and starts a 30s timeout.
2. When polling returns `true`, call `clearPending()` (clears signal + cancels timeout).
3. The rendering branch checks `accessibilityPending()` first to suppress the "Denied" indicator.
4. On 30s timeout, `setAccessibilityPending(false)` fires — UI reverts to normal (non-pending, not-granted) state.

No new `createEffect` needed — the pending cleanup is a side effect of the existing polling logic.

**[VERIFIED: read src/App.tsx:159–170, 182–189]**

### Q4: Does removing `cocoa = "0.26.1"` actually eliminate `block v0.1.6`?

**Yes — confirmed.** Cargo tree analysis:

```
block v0.1.6
├── cocoa v0.26.1
│   └── automux v1.2.0   ← ROOT CRATE (our Cargo.toml)
└── cocoa-foundation v0.2.1
    └── cocoa v0.26.1 (*)
```

The `cocoa` crate is the only package in the dependency graph that depends on `block v0.1.6`. The only consumer of `cocoa` is `automux` (the root crate). `cocoa-foundation` also uses `block`, but `cocoa-foundation` is only used by `cocoa`. Removing `cocoa` from Cargo.toml removes the entire chain.

No source file in `src-tauri/src/` contains `use cocoa::` — confirmed by grep. `block2 = "0.6.2"` depends only on `objc2` and is unaffected.

**[VERIFIED: ran `cargo tree --invert block`, `cargo tree -p cocoa`, and `grep -r "use cocoa"` in session]**

### Q5: Other callers of `initialize_tap()` or `check_accessibility_permissions()`

**All callers:**

`initialize_tap()`:
- `src-tauri/src/platform/macos/observer.rs:555` — inside `start_observing()`, called at app startup only if permission already granted
- `src-tauri/src/ipc/mod.rs:176` — inside `request_accessibility`, already arms tap if granted at dialog time

`check_accessibility_permissions()`:
- `src-tauri/src/platform/macos/mod.rs:20` — definition
- `src-tauri/src/platform/macos/observer.rs:554` — `start_observing()` uses it as a guard
- `src-tauri/src/ipc/mod.rs:174` — `request_accessibility` (prompt=true)
- `src-tauri/src/ipc/mod.rs:192` — `check_accessibility` (prompt=false) — **this is the D-03 change site**

The plan must modify only `ipc/mod.rs:check_accessibility`. All other callsites are unaffected.

**[VERIFIED: grep -rn across src-tauri/src/]**

---

## Don't Hand-Roll (Supplemental)

The `block v0.1.6` deprecation warning is a build warning, not a compilation failure or security issue in this context. The fix is purely declarative (Cargo.toml edit) — no code migration, no API replacement.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `AXIsProcessTrusted()` is thread-safe (can be called from Tokio thread) | Research Q2 | Low risk — Apple's AX API is called from any thread in standard macOS apps; alternative is `spawn_blocking` which is safe but unnecessary overhead |
| A2 | SolidJS `createSignal` for `accessibilityPending` with a plain closure variable for the timeout handle is idiomatic | Pattern 3 | Low risk — consistent with `_keyCaptureListener` (line 75) pattern in same file |

---

## Open Questions (RESOLVED)

1. **Should `request_accessibility` also clear pending on the backend side?**
   - What we know: `request_accessibility` returns `false` before the user acts on the dialog. The frontend sets pending immediately on click, before the `await` resolves.
   - What's unclear: If the user approves the dialog instantly (fast-click), `request_accessibility` would return `true` and `handleRequestAccess` would call `clearPending()` directly — the 3s poll would never need to run.
   - Recommendation: `handleRequestAccess` should call `clearPending()` when `granted === true` (instant approval path). This is already shown in Pattern 3.

2. **Style of "Pending approval…" indicator**
   - What we know: Claude has discretion on exact styling. The existing permission card uses `bg-danger`/`text-danger` for denied and `bg-success`/`text-success` for granted.
   - Recommendation: Use `text-warning` (or `text-text-dim`) with an italic or muted "Pending approval…" label. A small spinner is optional but aligns with the `status-pulse` CSS class already present.

---

## Environment Availability

This phase is code and config changes only. No external tool dependencies beyond the existing Rust toolchain and Node.js.

| Dependency | Required By | Available | Version |
|------------|------------|-----------|---------|
| Rust stable | Cargo.toml change + rebuild | Assumed available | — |
| cargo tree | Dependency verification | Confirmed working | — |

---

## Sources

### Primary (HIGH confidence)
- `[VERIFIED: read src-tauri/src/platform/macos/observer.rs]` — `initialize_tap()`, `TAP_INITIALIZED`, `start_observing()`, failure reset logic
- `[VERIFIED: read src-tauri/src/platform/macos/mod.rs]` — `check_accessibility_permissions(prompt)` implementation
- `[VERIFIED: read src-tauri/src/ipc/mod.rs]` — `request_accessibility`, `check_accessibility` handlers and all callsite enumeration
- `[VERIFIED: read src/App.tsx]` — `accessibility()` signal, `handleRequestAccess()`, 3s polling effect, existing permissions UI
- `[VERIFIED: read src-tauri/Cargo.toml]` — `cocoa = "0.26.1"`, `block2 = "0.6.2"` confirmed present
- `[VERIFIED: ran `cargo tree -p cocoa` and `cargo tree --invert block`]` — confirmed `block v0.1.6` origin and sole parent chain

### Tertiary (LOW confidence — assumed)
- `[ASSUMED]` `AXIsProcessTrusted()` thread safety — training knowledge, consistent with common macOS practice
- `[ASSUMED]` SolidJS plain closure variable for timeout handle is idiomatic — consistent with existing codebase pattern

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all changes are in verified existing code
- Architecture: HIGH — threading model confirmed from source read
- Pitfalls: HIGH — verified from source and cargo tree output
- Cargo dependency fix: HIGH — verified by cargo tree --invert

**Research date:** 2026-06-01
**Valid until:** 60 days (stable macOS API, no fast-moving library churn)
