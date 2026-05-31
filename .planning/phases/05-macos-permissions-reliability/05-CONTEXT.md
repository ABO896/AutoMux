# Phase 5: macOS Permissions & Reliability - Context

**Gathered:** 2026-05-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix macOS accessibility permission detection and CGEventTap arming so the app correctly reflects permission state and activates macros immediately on any grant path — no false negatives, no restart required. Also remove the deprecated `block v0.1.6` transitive dependency. No new features, no UI redesign, no Windows changes.

</domain>

<decisions>
## Implementation Decisions

### PERM-01: Post-grant UI state

- **D-01:** After the user clicks "Request Access", show a **"Pending approval…" indicator** rather than immediately displaying "not granted". The backend returns `false` before the user has acted on the OS dialog — that false state must not propagate to the UI.
- **D-02:** The pending state clears when polling returns `true` (permission granted) OR after a **30-second timeout**, at which point the UI reverts to "not granted". This covers the case where the user dismisses the OS dialog without approving.

### RELY-06: Tap arming on post-launch grant

- **D-03:** The `check_accessibility` **backend handler** is responsible for arming the tap when permission is detected. Before returning `true`, it checks whether the CGEventTap is initialized; if not, it calls `initialize_tap()`. This covers both the polling path (3s interval from `App.tsx`) and any direct `check_accessibility` calls — no new IPC command, no frontend transition-tracking required.
- **D-04:** When the tap arms post-grant, activation is **silent** — the permission indicator updates to "granted" and macros start working. No toast or banner.

### BUILD-02: block v0.1.6 deprecation

- **D-05:** Fix by **removing `cocoa = "0.26.1"` from `Cargo.toml`**. The CONCERNS.md confirmed no direct `use cocoa::…` imports in the source — it is an unnecessary direct declaration that transitively pulls in `block v0.1.6`. The `block2` crate (already used in observer.rs) remains. The broader `objc 0.2.7` legacy cleanup is **out of scope for Phase 5** — that belongs in a future tech-debt phase.

### Claude's Discretion

- Exact wording / styling of the "Pending approval…" indicator in `App.tsx` (chip, italic text, spinner — whatever fits the existing permission UI).
- Whether `initialize_tap()` is called synchronously inside `check_accessibility` or dispatched to a background task (implementation choice for the planner based on CGEventTap threading constraints).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Permission & tap initialization
- `src-tauri/src/platform/macos/mod.rs` — `check_accessibility_permissions(prompt: bool)` — the function both `check_accessibility` and `request_accessibility` IPC commands call; D-03 fix lives here or in the IPC handler
- `src-tauri/src/platform/macos/observer.rs` — `initialize_tap()` and `TAP_INITIALIZED` AtomicBool — the tap arming function called at startup; must be re-entrant safe for post-launch calls
- `src-tauri/src/ipc/mod.rs` — `request_accessibility` (line 171) and `check_accessibility` (line 189) — the two IPC handlers whose behavior changes under D-01 and D-03
- `src/App.tsx` — `handleRequestAccess()` (line 182) and the polling `createEffect` (line 160) — frontend permission flow that gains the "pending" state under D-01 and D-02

### Build dependency
- `src-tauri/Cargo.toml` — `cocoa = "0.26.1"` to be removed (D-05); verify `block2 = "0.6.2"` remains

### Requirements
- `.planning/REQUIREMENTS.md` — PERM-01, RELY-06, BUILD-02 acceptance criteria

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TAP_INITIALIZED: AtomicBool` in `observer.rs` — already exists; `initialize_tap()` checks it to avoid double-initialization. Post-launch calls to `initialize_tap()` must be safe — the guard is already there (line 219).
- 3s polling `createEffect` in `App.tsx` (line 160) — already in place; the pending state (D-01) can hook into this effect rather than adding a new one.

### Established Patterns
- `check_accessibility_permissions(false)` → `AXIsProcessTrusted()`: the silent check path used by polling. If this returns `true` and tap is not initialized, D-03 triggers `initialize_tap()`.
- `check_accessibility_permissions(true)` → `AXIsProcessTrustedWithOptions` with prompt: shows OS dialog and returns the **current** state (false before approval). This is why D-01 is needed — the return value is not the post-approval state.
- Frontend signals: `accessibility()` signal (boolean) in App.tsx drives the permission indicator. A new `accessibilityPending()` signal can drive the pending state without breaking the existing boolean.

### Integration Points
- `initialize_tap()` is currently called once in `lib.rs` at startup. The post-launch call from `check_accessibility` (D-03) needs to be thread-safe — `initialize_tap()` already has the `TAP_INITIALIZED` AtomicBool guard, but it runs a `CFRunLoop` on a background thread; the planner must verify re-entrant safety.
- The pending state timeout (D-02, 30s) needs to be cleared by the polling effect — when polling returns `true`, clear pending immediately; when 30s elapses with no grant, clear pending and stay on "not granted".

</code_context>

<specifics>
## Specific Ideas

No specific UI references — open to standard approaches within the existing Tailwind / SolidJS patterns.

</specifics>

<deferred>
## Deferred Ideas

- **`objc 0.2.7` legacy cleanup** — CONCERNS.md flagged this as a risk (unmaintained, duplicates `objc2`). Explicitly deferred out of Phase 5 scope. Future tech-debt phase.
- **`cocoa` → `objc2` migration audit** — broader than just removing `cocoa`; deferred.

</deferred>

---

*Phase: 5-macOS Permissions & Reliability*
*Context gathered: 2026-05-31*
