# Research Summary — AutoMux

**Synthesized:** 2026-05-15
**Sources:** STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md, PROJECT.md

---

## Executive Summary

AutoMux is a Tauri 2 + Rust + SolidJS macro automation tool for macOS and Windows. The codebase is functionally complete at v1.1.0 with core macro execution, profile persistence, and cross-platform builds shipping. The immediate work is not building new features — it is fixing a permissions-loop bug that breaks the core value proposition ("a macro that was set up must fire reliably"), cleaning up the codebase, and replacing two UX dead-ends (raw key codes, manual process name entry) that make the tool unusable for non-technical users.

The permissions bug is precisely diagnosed: `check_accessibility` (polled every 10 seconds) detects when the user grants Accessibility in System Settings but never calls `initialize_tap()` on that transition. The CGEventTap never starts. The fix is a three-line change in `ipc/mod.rs`. The complementary CGEventTap-disabled-by-timeout gap (no handler for `kCGEventTapDisabledByTimeout` in the callback) is a second reliability gap that causes hotkeys to silently stop working during long sessions.

Beyond the permissions fix, the audit found four issues with immediate impact on user trust: (1) the persistence layer never auto-saves, silently losing all work on app close; (2) Windows emergency stop does not flush held keys before `process::exit(1)`, leaving keys stuck; (3) the minimum interval floor is 1ms instead of the documented 5ms, enabling OS input queue saturation; and (4) the README states MIT License while the actual license is GPL-3.0. These must be fixed before new features are added.

---

## Critical Findings (must fix)

1. **Permissions loop — tap never starts after grant.** `check_accessibility` polls every 10 seconds and detects the false→true transition but does not call `initialize_tap()`. Users who grant Accessibility in System Settings see the app still reporting "access denied" for up to 10 seconds, then clicking "Request Access" again sends them back to System Settings. Fix: modify `check_accessibility` in `ipc/mod.rs` to call `initialize_tap()` when returning `true`. The call is idempotent. No frontend change required.

2. **CGEventTap silently disabled by OS timeout.** The tap callback has no handler for `kCGEventTapDisabledByTimeout` (CGEventType value 0xFFFFFFFE). When the OS disables the tap after a slow callback (contended mutex during emergency stop or high-rate macro execution), all hotkeys and the emergency stop key stop working silently. Fix: add a branch in the callback to call `CGEventTapEnable(tap_ref, true)` on receipt of the tap-disabled event type.

3. **No auto-save — users lose all work on close.** `StateActor::handle_intent` never calls `ProfileManager::save_profile` after mutations. The comment in `persistence.rs` falsely claims auto-save is implemented. Every macro created since the last manual save is lost on app exit. Fix: inject `ProfileManager` into `StateActor` and call `save_profile` asynchronously after each mutating intent.

4. **Windows emergency stop does not flush held inputs.** `windows/mod.rs` calls `try_send(TriggerEmergencyStop)` then immediately `process::exit(1)`. There is no inline flush before exit. Keys held by macros remain stuck after emergency stop on Windows. Fix: add a synchronous `flush_all_held_inputs()` call before `process::exit(1)`, matching the macOS path.

5. **Minimum interval floor is 1ms, not 5ms.** `scheduler/mod.rs:66,184` enforce `interval_ms.max(1)`. At 1ms, OS input event queues saturate and the system becomes unresponsive. Fix: change `max(1)` to `max(5)` in both locations; add frontend validation and a "minimum 5ms" label.

6. **README states MIT License; actual license is GPL-3.0.** `README.md:79` says "MIT License." `LICENSE` file and `PROJECT.md` both say GPL-3.0. This is a legally significant misrepresentation. Fix: replace "MIT License" with "GNU General Public License v3.0 (GPL-3.0)" in README prose; ensure SPDX identifier is `GPL-3.0-only` in `package.json`.

7. **`CGEventTap::new()` failure is silent to the frontend.** `observer.rs:373-379` resets `TAP_INITIALIZED` but does not notify the frontend. The app continues with no tap running — users see no error. Fix: emit a `tap-failed` Tauri event on tap creation failure so the frontend can show a distinct "tap could not start" message.

8. **`update_step_interval` has no bounds check.** The IPC command accepts `step_index: usize` from the frontend without validating it against `steps.len()`. Out-of-bounds access can panic the StateActor. Fix: add bounds validation before forwarding the `SchedulerIntent`.

---

## Stack & Platform

**Core stack (no changes needed):**
- Tauri 2 (IPC bridge, windowing, capability model) — correctly configured for non-sandboxed CGEventTap usage
- Rust backend — StateActor + Scheduler actor pattern, platform input via CGEvent (macOS) and SendInput (Windows)
- SolidJS + TypeScript frontend — Vite 6, Tailwind CSS 4
- `objc2-app-kit` crate v0.3.2 — already used for NSWorkspace; provides `NSRunningApplication` for the process picker at no additional dependency cost

**macOS permissions — the correct re-check pattern:**

The standard pattern used by all macOS automation tools (Raycast, Keyboard Maestro, BetterTouchTool) is:
1. Startup: silent check; if granted, init tap; if not, show permissions UI
2. User clicks "Request Access": call `AXIsProcessTrustedWithOptions(prompt: true)` to show dialog; do NOT expect it to return `true`
3. Poll at 1-2 second intervals while permissions are not granted
4. On poll returning `true`: call `initialize_tap()` and update UI — this is the missing step in AutoMux
5. After granted, reduce polling interval or stop

Additional improvement: open System Settings directly via the `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility` URL scheme rather than relying on the small native dialog.

**Key code strategy — recommended migration path:**

The current `InputEvent::Key(u16)` stores raw CGKeyCodes (macOS) or VK codes (Windows) — different namespaces that overlap numerically but mean different things. The recommended approach is a `NamedKey` enum in Rust that serializes as snake_case strings (e.g., `"space"`, `"f1"`) and resolves to the platform-native keycode at injection time. The frontend maps `KeyboardEvent.code` (DOM standard, layout-independent) to `NamedKey` strings. Old profiles with raw `Key(n)` values remain loadable. Phase 1 key capture can ship without this Rust change by using TypeScript lookup tables.

**Signing and entitlements (required before distribution):**
- Currently unsigned (`signingIdentity: null`, `entitlements: null`)
- Must create `src-tauri/entitlements.plist` without `com.apple.security.app-sandbox` — CGEventTap is incompatible with the App Sandbox
- Required entitlement: `com.apple.security.automation.apple-events`
- Every signing identity change invalidates existing users' TCC grants — establish the identity once and do not change it

---

## Feature Implementation Paths

### Key Capture Widget (pure frontend, no Rust changes needed for Phase 1)

Replace the raw key code number input with a `KeyCaptureInput` SolidJS component:
- Styled button showing the current binding in human-readable form ("F7", "Space", "Ctrl+Q")
- On click: enters capture mode; registers a window-level `keydown` listener with `{ capture: true }`
- First non-modifier key press: records `KeyboardEvent.code` → maps to `NamedKey` string → converts to CGKeyCode/VK via lookup table for IPC
- Escape cancels capture without clearing the existing value

Phase 1 requires two TypeScript lookup tables covering ~80 keys: `DOM_CODE_TO_CGKEYCODE` and `DOM_CODE_TO_VK`. Platform detected at runtime. No Rust changes required — the existing `u16` IPC field is populated from the lookup table. Phase 2 (deferred) migrates to the `NamedKey` enum in Rust.

Risk: some system keys (F-keys, media keys) may not fire `keydown` in the WebView if intercepted by the OS. Fallback: display "Key(N)" for unmapped codes.

### Process Picker (one new Rust IPC command + frontend component)

**macOS:** `NSWorkspace.sharedWorkspace().runningApplications` filtered to `activationPolicy == .regular`. Returns `(bundle_id, display_name)` pairs. The `objc2-app-kit` import pattern is already established in the codebase. Does not require Accessibility permission.

**Windows:** `EnumWindows` filtered with `IsWindowVisible` + window title check. Reuses the existing `get_app_name_from_hwnd` helper. Returns full executable path (matching what `get_active_app` returns) for correct `target_app` matching.

**Single IPC command:** `list_running_apps() -> Vec<RunningApp>` with `#[cfg]`-gated implementations. Called once when the picker opens, not pre-fetched.

**UI pattern:** Search-filtered inline dropdown. "Global" pinned at top. Stores the value the observer produces (bundle ID on macOS, full exe path on Windows). The platform asymmetry is surfaced naturally by the picker.

Risk: Windows UWP apps may appear incorrectly — flag for manual testing during implementation.

---

## Audit Priorities

Ordered by risk to correctness and user trust:

**Priority 1 — Safety and correctness (before any new feature):**
1. `platform/macos/input.rs:24` — `.expect()` on CGEventSource creation panics the injection path on revoked permissions. Return `Err` and propagate.
2. All `.lock().unwrap()` on static mutexes in `observer.rs` and `windows/mod.rs` — poisoned mutex propagates panics to the StateActor. Add poison recovery: `.unwrap_or_else(|e| e.into_inner())`.
3. `update_step_interval` missing bounds check on `step_index` (Critical Finding #8).
4. Minimum interval floor 1ms → 5ms (Critical Finding #5).
5. Windows emergency stop no pre-exit flush (Critical Finding #4).

**Priority 2 — Silent data loss (before shipping features):**
6. No auto-save (Critical Finding #3).
7. Silent channel drops — all three mpsc channels use `try_send` with capacity 100 and drop messages silently. Add drop counters in debug builds.
8. `MacPlatformObserver` dropped silently at end of setup closure. Store in `tauri::Manager::manage()` for stable application lifetime.

**Priority 3 — CI and distribution (before next public release):**
9. Pin `tauri-apps/tauri-action@v0` to a specific release SHA.
10. Verify macOS universal binary build — confirm `tauri-action` receives `--target aarch64-apple-darwin` and `--target x86_64-apple-darwin` as explicit build args.
11. Add `Swatinem/rust-cache@v2` to CI.
12. Create macOS entitlements plist (no sandbox entitlement).
13. Fix Windows timestamp URL from `http://` to `https://`.

**Priority 4 — Repository hygiene (before inviting contributors):**
14. Remove `implementation_plan.md` from repository root (stale, misleading).
15. Add `src-tauri/gen/` to `.gitignore`.
16. Audit `archive/v1-production/` for committed binary artifacts.
17. Fix hardcoded `v1.0.0` in `App.tsx:291` — use `getVersion()` from `@tauri-apps/api/app`.
18. Fix comment/interval mismatch — `App.tsx:124` says "3s" but uses 10000ms.
19. Audit and remove Google Fonts CSP entries if no Google Fonts are loaded.

**Priority 5 — Frontend quality (in parallel with features):**
20. Add `<ErrorBoundary>` at top level of `App.tsx`.
21. Add `aria-pressed` to engine toggle and per-macro enable/disable buttons.
22. Enable `noUnusedLocals: true` in `tsconfig.json`.
23. Remove or wire the discarded `setLoading` signal getter.
24. Add duplicate trigger key conflict detection and user-visible warning.

---

## Watch Out For

1. **Every `cargo build` in dev invalidates the TCC grant for unsigned builds.** macOS TCC anchors the Accessibility grant to the binary hash for unsigned apps. Developers must re-grant after every recompile. Document prominently for contributors.

2. **CGEventTap can be disabled by the OS without any error.** The `kCGEventTapDisabledByTimeout` event type arrives through the same callback channel as real events. If the callback is missing the handler, the tap goes dark silently — indistinguishable from a TCC denial from the frontend's perspective. Keep callback execution time short; avoid holding mutexes inside the callback.

3. **App Sandbox is permanently incompatible with system-wide CGEventTap.** If `com.apple.security.app-sandbox` ever appears in the entitlements file, `CGEventTap::new()` fails silently. This must be a hard guard in the release checklist. App Store distribution is architecturally incompatible without a LaunchAgent helper.

4. **Changing the signing identity breaks existing users' Accessibility grants.** Every user who previously granted AutoMux Accessibility access will silently lose the grant on signing identity change. Establish the signing identity once and do not change it.

5. **macOS target_app is a bundle ID; Windows target_app is a full exe path.** These are not portable between platforms. The process picker will naturally produce the right format, but profiles must be documented as non-portable for the `target_app` field.

6. **`AXIsProcessTrustedWithOptions(prompt: true)` returns current status, not post-grant status.** The call returns `false` immediately even though it just showed the dialog. The correct pattern is: show the dialog, then poll for the grant. Never treat the return value of the prompt call as the grant result.

7. **WebView keydown may not capture all system-intercepted keys.** Function keys and media keys may be intercepted by macOS before they reach the Tauri WebView. Graceful fallback (display "Key(N)", let user retry) is required. Capture mode should have a timeout or manual cancel.

---

## Implementation Order

**Phase 1 — Reliability and correctness (unblock the core value proposition)**
1. Fix permissions loop: modify `check_accessibility` to call `initialize_tap()` on true return (`ipc/mod.rs`, 3-line change)
2. Fix CGEventTap-disabled handler: add `kCGEventTapDisabledByTimeout` branch in callback (`observer.rs`)
3. Fix Windows emergency stop: add synchronous `flush_all_held_inputs()` before `process::exit(1)` (`windows/mod.rs`)
4. Fix minimum interval floor: `max(1)` → `max(5)` in `scheduler/mod.rs` (two locations)
5. Fix auto-save: inject `ProfileManager` into `StateActor`, call save after each mutating intent
6. Fix unsafe panic paths: `.expect()` on CGEventSource → propagate `Err`; add poisoned mutex recovery on all static mutex locks
7. Improve permissions UX: reduce poll interval to 1-2s while not granted; open System Settings directly via URL scheme; update UI messaging ("no restart needed")
8. Surface tap creation failure to frontend: add `TapInitFailed` intent and `tap-failed` Tauri event

**Phase 2 — README and audit cleanup**
9. Fix README license: MIT → GPL-3.0 prose and SPDX badge
10. Codebase hygiene: remove `implementation_plan.md` from root; add `src-tauri/gen/` to `.gitignore`; fix hardcoded version string; fix comment/interval mismatch; audit Google Fonts CSP
11. Add `<ErrorBoundary>` at App.tsx top level; enable `noUnusedLocals` in tsconfig; fix `setLoading` signal
12. Bounds-check `update_step_interval` IPC command; add duplicate trigger key conflict detection

**Phase 3 — UX features**
13. Key capture widget: `KeyCaptureInput` SolidJS component + lookup tables (pure frontend)
14. macOS process picker: `list_running_apps` IPC command + `ProcessPickerInput` frontend component
15. Windows process picker: `EnumWindows`-based enumeration + same frontend component

**Phase 4 — CI, distribution, and signing**
16. Pin `tauri-apps/tauri-action` to specific SHA; verify universal binary build targets
17. Add `Swatinem/rust-cache@v2`
18. Create entitlements plist; configure `tauri.conf.json`
19. Fix Windows timestamp URL (HTTP → HTTPS)

**Deferred to future phases:**
- NamedKey Rust enum schema migration (Phase 1 key capture delivers the UX fix without it)
- Auto-updater / in-app update mechanism (explicitly out of scope per PROJECT.md)
- App Store distribution (architecturally incompatible with CGEventTap without LaunchAgent refactor)
- ARIA accessibility pass and full keyboard navigation audit

---

## Confidence Assessment

| Area | Confidence | Basis |
|------|------------|-------|
| Permissions bug root cause and fix | HIGH | Direct code inspection of ipc/mod.rs, observer.rs, App.tsx; confirmed by STACK.md and PITFALLS.md independently |
| CGEventTap-disabled gap | HIGH | Direct code inspection of observer.rs callback match arms; well-documented Apple platform behavior |
| Auto-save gap | HIGH | Direct code inspection of state/mod.rs; corroborated by CONCERNS.md |
| Windows emergency stop flush gap | HIGH | Direct code inspection of windows/mod.rs vs. macOS path |
| Key capture widget implementation | HIGH | Standard pattern used by all major macOS automation tools; DOM KeyboardEvent.code is well-specified |
| macOS process picker (NSWorkspace) | HIGH | Long-standing public API since macOS 10.6; objc2-app-kit already used in codebase |
| Windows process picker (EnumWindows) | MEDIUM | APIs well-established; exact filtering for UWP/Electron apps needs validation during implementation |
| NamedKey cross-platform strategy | HIGH | Sound approach; straightforward implementation |
| Entitlements / signing behavior | MEDIUM | Cannot verify against live Tauri 2 docs; consistent with known platform behavior |
| CI universal binary verification | MEDIUM | Cannot run CI to verify; identified as a risk requiring manual confirmation |

**Gaps to validate during implementation:**
- Does `tauri-action@v0` require explicit `--target` args or infer from installed toolchain targets? Verify with verbose CI logging.
- Does the `x-apple.systempreferences:` URL scheme work on macOS 12 (the declared minimum)? Test on a macOS 12 machine.
- Windows EnumWindows filter: does `IsWindowVisible` + title check correctly include Electron and UWP apps? Requires manual Windows testing.

---

*Synthesized from: STACK.md (macOS permissions deep-dive, re-check pattern, Tauri 2 entitlements), FEATURES.md (key capture UX, cross-platform key codes, process picker APIs), ARCHITECTURE.md (codebase audit — Rust safety, frontend quality, CI/CD), PITFALLS.md (critical bugs, phase warnings)*
