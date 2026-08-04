# Phase 7: carry-work-platform-ci-safety - Context

**Gathered:** 2026-06-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Clean up and harden the nine requirements deferred from v1.2.0, plus two macOS 26 permission follow-ups surfaced in Phase 6. No new features, no UI redesign, no architectural changes beyond what each fix requires. The work divides into four largely-parallel tracks: Windows platform cleanup (BUILD-01, MEM-01), CI hardening (CI-03, CI-04, CI-05), safety & error surface (SAFE-04, ERR-01), and macOS 26 permission follow-ups (COMPAT-04, COMPAT-05).

</domain>

<decisions>
## Implementation Decisions

### Input Monitoring detection (COMPAT-04)

- **D-01:** Check for `kTCCServiceListenEvent` (Input Monitoring) **at startup, alongside the Accessibility check** — using the same polling pattern established in Phase 5. User sees the full picture on launch.
- **D-02:** Replace the solo Accessibility indicator with a **combined "Permissions" section** that lists both Accessibility and Input Monitoring in a unified block. Makes room for both grants without duplicating UI patterns.
- **D-03:** The "Grant" button for Input Monitoring **opens System Settings → Privacy → Input Monitoring directly** via the opener plugin — same pattern as the existing Accessibility "Grant" button. No step-by-step modal.
- **D-04:** **Extend the existing 3s polling loop** (currently Accessibility-only) to also check Input Monitoring. No new IPC commands required; the same polling `createEffect` in App.tsx covers both.
- **D-05:** Whether Input Monitoring check is macOS 26-only or applies to all supported macOS versions — **Claude's discretion** (researcher determines whether `kTCCServiceListenEvent` was actually required on macOS 12–15 for AutoMux's use case).
- **D-06:** If Input Monitoring is denied, **warn in the UI but do not block the engine**. Mouse click macros triggered via UI toggle still work. Only global hotkey activation is affected. The warning makes this clear.

### TCC identity change (COMPAT-05)

- **D-07:** **Persist a 'was-ever-granted' flag** to `app_data_dir()` (via Tauri's path API, e.g., `~/Library/Application Support/com.alvaro.automux/`) the first time Accessibility is successfully granted. The flag survives app reinstalls and upgrades.
- **D-08:** On startup, if any permission is denied **and** the flag exists → treat as a likely TCC identity change (app upgrade invalidated the grant). This is the detection signal.
- **D-09:** When identity-change is detected, **replace the generic "not granted" copy** in the Permissions section with a specific message: e.g., "AutoMux was updated — Accessibility needs to be re-added in System Settings." Same "Grant" button, same flow — only the copy changes.
- **D-10:** **Unified flag**: covers both Accessibility and Input Monitoring. TCC invalidates all grants when an app's signing identity changes, so both permissions would be lost simultaneously. One flag file, one detection path.

### Auto-save error UI (ERR-01)

- **D-11:** Add a `listen('auto-save-error', ...)` listener in App.tsx, cleaned up via `onCleanup` per SolidJS convention.
- **D-12:** On a save error event, show a **persistent inline banner at the top of the macro list** that stays visible until explicitly dismissed.
- **D-13:** **User-friendly copy only** — e.g., "Failed to save changes — check available disk space." Do not surface the raw Rust error string from the event payload.
- **D-14:** **Dismiss-and-clear behavior**: a dismiss button removes the current banner. The banner re-appears if another `auto-save-error` event fires later. No permanent suppression.

### CI hardening

- **D-15 (CI-03):** **Remove the updater signature upload step from `release.yml` entirely**. AutoMux uses ad-hoc signing — no private key, so no `.sig` file is ever generated. Add a comment: `# Updater signature upload removed — re-add when real signing is configured (v3 scope)`. Eliminates the misleading "Signature not found" skip.
- **D-16 (CI-04):** Replace `npm install` with **`npm ci`** in the release workflow for reproducible, lockfile-gated dependency installs.
- **D-17 (CI-05):** Use **tauri-action's official output variable** (`${{ steps.tauri.outputs.artifactPaths }}`) for artifact discovery in upload steps, instead of hardcoded glob paths. Follows the artifact wherever tauri-action places it.

### Platform cleanup (clear-cut — no discussion needed)

- **BUILD-01:** Remove unused Win32 imports in `src-tauri/src/platform/windows/mod.rs`: `GetWindowTextW`, `IsWindowVisible`, `HMODULE`, `HHOOK`. Handle the `TranslateMessage` unused-bool warning.
- **MEM-01:** Add `CloseHandle(handle)` after each `OpenProcess` call in `list_running_apps_impl` in `platform/windows/mod.rs`. Every opened handle must be closed before the function returns.

### Safety (clear-cut — no discussion needed)

- **SAFE-04:** In `flush_held_inputs`, collect the list of held inputs **while holding the REGISTRY lock**, then **release the lock**, then **post the CGEvents**. Lock must not be held during CGEvent dispatch. This eliminates the potential deadlock on macOS emergency stop.

### Claude's Discretion

- Whether the Input Monitoring check applies only on macOS 26+ or on all supported macOS versions (Monterey–Sequoia included) — researcher determines if `kTCCServiceListenEvent` was required before macOS 26 for global hotkeys to work.
- Exact copy for the "AutoMux was updated" identity-change message — planner picks wording that fits the existing permission UI style.
- Exact copy for the auto-save error banner — planner picks user-friendly message that explains the likely cause without exposing the raw error.
- Whether `check_accessibility_permissions` in `platform/macos/mod.rs` is extended to also return Input Monitoring status, or a separate `check_input_monitoring` backend function is added — planner decides based on the existing function signature.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### macOS platform (SAFE-04, COMPAT-04, COMPAT-05)
- `src-tauri/src/platform/macos/mod.rs` — `check_accessibility_permissions(prompt: bool)` — must be extended to also check `kTCCServiceListenEvent` for Input Monitoring (COMPAT-04); 'was-ever-granted' flag write logic may also live near here
- `src-tauri/src/platform/macos/observer.rs` — `flush_held_inputs()` — SAFE-04 fix lands here (release REGISTRY lock before dispatching CGEvents); also `initialize_tap()` and `TAP_INITIALIZED: AtomicBool` — must remain re-entrant safe after COMPAT-04 changes

### IPC layer (COMPAT-04)
- `src-tauri/src/ipc/mod.rs` — `check_accessibility` handler — will be extended or a new `check_input_monitoring` command added; the frontend polling loop must be able to check both permissions

### Windows platform (BUILD-01, MEM-01)
- `src-tauri/src/platform/windows/mod.rs` — `list_running_apps_impl` (MEM-01: add CloseHandle); unused Win32 imports section (BUILD-01: remove unused imports); zero-warning target is `x86_64-pc-windows-msvc`

### Frontend (COMPAT-04, COMPAT-05, ERR-01)
- `src/App.tsx` — `handleRequestAccess()` and the 3s polling `createEffect` — must be extended to cover Input Monitoring alongside Accessibility; `accessibility()` and `accessibilityPending()` signals as the pattern for new `inputMonitoring()` and `inputMonitoringPending()` signals; `auto-save-error` listener to be added for ERR-01

### Persistence (ERR-01, COMPAT-05)
- `src-tauri/src/persistence.rs` — auto-save error is emitted from here (D-11 wires the frontend listener); the data directory used here is the candidate location for the 'was-ever-granted' flag (D-07)

### CI workflow (CI-03, CI-04, CI-05)
- `.github/workflows/release.yml` — updater signature step to be removed (D-15); `npm install` to be replaced with `npm ci` (D-16); artifact upload steps to use `${{ steps.tauri.outputs.artifactPaths }}` (D-17)

### Requirements
- `.planning/REQUIREMENTS.md` — acceptance criteria for COMPAT-04, COMPAT-05, BUILD-01, MEM-01, CI-03, CI-04, CI-05, SAFE-04, ERR-01

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **3s polling `createEffect`** in App.tsx — already in place for Accessibility; extend to call `check_input_monitoring` (or an extended `check_accessibility`) on the same interval. No new effect needed.
- **`handleRequestAccess()` pattern** in App.tsx — "Grant" button → open System Settings pane via opener plugin. Replicate for Input Monitoring's "Grant" button with a different Settings URL.
- **`accessibilityPending()` signal** established in Phase 5 — same pattern applies for `inputMonitoringPending()` to cover the post-grant detection window.
- **`TAP_INITIALIZED: AtomicBool` guard** in observer.rs — any backend changes for COMPAT-04 must not break re-entrant safety of `initialize_tap()`; the guard is already there.

### Established Patterns
- **Phase 5 pattern: fix lives in the backend handler** — `check_accessibility` backend handler arms the tap and returns correct state; frontend polls and reacts. Same discipline applies for Input Monitoring.
- **Phase 5 pending state**: `accessibilityPending()` boolean signal that clears on grant or 30s timeout. Extend same pattern for Input Monitoring if a "Grant" tap is added.
- **`if let Ok(event) = CGEvent::new(...)` — silent drop** — platform errors are silently dropped. SAFE-04 fix must not introduce error propagation; CGEvent failures stay silent.
- **SolidJS `onCleanup` convention** — any `listen()` call (Tauri event subscription) in App.tsx must be paired with `onCleanup(() => unlisten())`. ERR-01's `auto-save-error` listener must follow this pattern.

### Integration Points
- The combined "Permissions" section (D-02) changes App.tsx layout where the solo Accessibility indicator currently lives. Planner must check whether Phase 8 (hotkey reliability UI) also touches this area to avoid conflicts.
- SAFE-04 fix in `flush_held_inputs` is isolated to observer.rs; no IPC or frontend changes needed.
- CI changes are isolated to `.github/workflows/release.yml`; no Rust or TypeScript changes.
- BUILD-01 and MEM-01 are isolated to `platform/windows/mod.rs`; no other files affected.

</code_context>

<specifics>
## Specific Ideas

No specific UI references from discussion — open to standard Tailwind/SolidJS patterns, consistent with the existing permission indicator style (Phase 5).

ERR-01 banner copy example (Claude's discretion for exact wording): "Failed to save changes — check available disk space."

COMPAT-05 identity-change copy example (Claude's discretion for exact wording): "AutoMux was updated — Accessibility needs to be re-added in System Settings."

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 7-carry-work-platform-ci-safety*
*Context gathered: 2026-06-17*
