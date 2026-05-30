# Phase 1: Reliability & Safety - Context

**Gathered:** 2026-05-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix six concrete reliability and safety gaps in the existing Rust backend and platform layer so macros fire without crashes, silent data loss, or false permission failures. This phase touches no new features — only correctness fixes to existing behavior.

**Requirements in scope:** RELY-01, RELY-02, RELY-03, RELY-04, SAFE-01, SAFE-03

**Not in scope:** README/license fixes (Phase 2), UX key capture / process picker (Phase 3), CI hardening (Phase 4). Do not expand scope.

</domain>

<decisions>
## Implementation Decisions

### Permission Re-check Experience (RELY-01)

- **D-01:** Fix `setInterval` in `src/App.tsx:124–134` from `10000ms` to `3000ms` — the code comment already says "every 3s" but the actual interval is 10s. This is the only change needed.
- **D-02:** No "Re-check" button — polling alone is sufficient.
- **D-03:** On permission grant detected (poll transitions `false → true`): clear the permissions prompt silently. No toast, no modal — the UI state change is the signal.
- **D-04:** On launch with permissions denied: show the existing prompt with "Open Settings" button. No auto-redirect to System Settings.

### Auto-save Profile Target (RELY-04)

- **D-05:** Auto-save always writes to the `default` profile. No "active profile" tracking in `AppState` — named profiles remain explicit-save only via the Profiles tab.
- **D-06:** Suppress auto-save during profile load. When a profile load sends a batch of `AddMacro` intents, the StateActor must not write on each one. Write once after the load is complete. Implement via a flag in `AppState` (e.g., `loading_profile: bool`) that the `LoadProfile` intent sets/clears around the batch.
- **D-07:** Auto-save write failures surface to the frontend as a transient (non-blocking, auto-dismissing) status message. Use the same event mechanism as other StateActor → frontend notifications (emit a Tauri event the UI listens for).

### Failure Recovery Visibility (SAFE-01, RELY-02, RELY-03)

- **D-08:** Injection hot-path panics replaced (SAFE-01): when a `CGEvent` or `SendInput` call fails, skip the action silently. Follow the existing `if let Ok(event) = ...` pattern already used elsewhere in the platform layer. No user notification.
- **D-09:** CGEventTap re-enable after timeout (RELY-02): fully silent recovery. When the tap callback receives `kCGEventTapDisabledByTimeout`, immediately call `CGEventTapEnable` and return. No user notification.
- **D-10:** Windows emergency stop flush (RELY-03): best-effort synchronous flush on the hook thread before `process::exit(1)`. Iterate held inputs and send KeyUp/MouseUp; exit regardless of whether any individual event failed. Match the existing macOS inline-flush approach.

### Interval Floor Enforcement (SAFE-03)

- **D-11:** Silently clamp to 5ms minimum — change `interval_ms.max(1)` to `interval_ms.max(5)` at both enforcement sites in `scheduler/mod.rs`. No user notification, no error returned.
- **D-12:** Scheduler layer only. No frontend validation in Phase 1.

### Claude's Discretion

- How to wire `ProfileManager` into `StateActor` (Arc, direct injection, or channel-based) — researcher/planner to decide based on existing ownership patterns in `lib.rs`.
- Whether to model load suppression as a boolean flag on `AppState` or as a dedicated `Intent::BeginProfileLoad` / `Intent::EndProfileLoad` pair — planner's choice.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — v1 requirements; RELY-01/02/03/04, SAFE-01, SAFE-03 are Phase 1 scope
- `.planning/ROADMAP.md` — Phase 1 success criteria (5 acceptance tests that must pass)

### Core Fix Locations (Rust)
- `src-tauri/src/state/mod.rs` — `StateActor::handle_intent()` (RELY-04 auto-save injection site); `inject_input()` at line 241 (SAFE-01 panic paths)
- `src-tauri/src/scheduler/mod.rs:66,184` — `interval_ms.max(1)` enforcement (SAFE-03 fix sites)
- `src-tauri/src/platform/macos/observer.rs` — CGEventTap callback (`kCGEventTapDisabledByTimeout` handling for RELY-02); `AXIsProcessTrusted` not directly here — the check is in IPC/frontend
- `src-tauri/src/platform/windows/mod.rs` — emergency stop handler (RELY-03 synchronous flush site)
- `src-tauri/src/persistence.rs` — `ProfileManager::save_profile` (to be injected into StateActor for RELY-04)

### Core Fix Location (Frontend)
- `src/App.tsx:124–134` — `setInterval` permissions poll (fix 10000 → 3000ms for RELY-01)

### Codebase Analysis
- `.planning/codebase/CONCERNS.md` — precise line numbers for every issue; read before planning to avoid hunting
- `.planning/codebase/ARCHITECTURE.md` — actor topology, threading model, ownership constraints

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ProfileManager::save_profile()` (`persistence.rs`) — already implemented; only needs to be called from StateActor after mutating intents
- `if let Ok(event) = CGEvent::new(...)` pattern — already used in macOS injection path; SAFE-01 replacements should match this pattern exactly
- Tauri event emission (`app_handle.emit()`) — already used for `state-changed`; auto-save error events should use the same mechanism

### Established Patterns
- `#[cfg(debug_assertions)]` guard — all debug logging uses this; any SAFE-01 debug traces should follow this convention
- `try_send` for fire-and-forget from non-async contexts — used in platform observer; RELY-02 re-enable is synchronous (no channel needed, call directly in tap callback)
- `Intent` enum for all StateActor mutations — any new state for load-suppression must go through an Intent variant, not direct field access

### Integration Points
- `StateActor` needs access to `ProfileManager` — currently `ProfileManager` is in the IPC layer (`ipc/mod.rs`). Researcher should determine whether to pass an `Arc<ProfileManager>` into the StateActor at construction (`lib.rs`) or use a channel-based approach
- CGEventTap callback in `observer.rs` — already has access to the tap reference; `CGEventTapEnable` can be called inline in the callback for RELY-02
- Windows hook thread in `windows/mod.rs` — emergency stop handling already has access to held-inputs registry; RELY-03 flush goes here before `process::exit(1)`
- `lib.rs::run()` — construction site for StateActor; any new dependencies injected here

</code_context>

<specifics>
## Specific Ideas

- The poll frequency fix (D-01) is a one-line change: `10000` → `3000` in `src/App.tsx:132` (the `setInterval` call). The comment at line 124 already says the intended interval.
- The interval floor fix (D-11) is a two-line change: lines 66 and 184 in `scheduler/mod.rs`. Researcher should confirm both sites still exist and whether any other `max(1)` calls are present.
- The 5 `unwrap()`/`expect()` calls on the injection hot path (SAFE-01) — CONCERNS.md identifies these are in `state/mod.rs` and the platform injection layer; researcher must locate exact call sites before planning the replacements.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 1-Reliability & Safety*
*Context gathered: 2026-05-16*
