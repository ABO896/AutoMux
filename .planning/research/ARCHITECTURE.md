# Audit Architecture

**Project:** AutoMux (Tauri 2 + Rust + SolidJS)
**Scope:** Pre-feature-addition codebase audit
**Date:** 2026-05-15

---

## Rust Backend Checklist

### 1. Unsafe Code — Verify All Sites Are Justified

**Check:** Every `unsafe` block must have a comment explaining the safety contract. Unjustified unsafe is a soundness bug waiting to be triggered by a future OS SDK update.

| Location | Current State | Why It Matters |
|---|---|---|
| `platform/macos/mod.rs:21` | `unsafe { }` around `AXTrustedCheckOptionPrompt` CString usage | Justified by FFI contract — verify comment exists |
| `platform/macos/observer.rs:368` | `unsafe { kCFRunLoopCommonModes }` in CF RunLoop setup | Justified by CoreFoundation ABI — verify comment |
| `platform/macos/observer.rs:415,436,482` | `unsafe { NSWorkspaceDidActivateApplicationNotification }`, `NSObject` observer | Needs explicit safety comment: the observer pointer lifetime must outlive the CFRunLoop thread |
| `platform/windows/mod.rs:88,125,153` | `unsafe { }` around Win32 SendInput calls | Justified by Win32 ABI — verify each block is minimal scope |
| `platform/windows/mod.rs:230` | `unsafe fn get_app_name_from_hwnd` | Entire function is unsafe — ensure callers handle null `HWND` correctly (line 263) |
| `platform/windows/mod.rs:335,369` | `unsafe extern "system" fn hook_callback`, `win_event_hook_callback` | Required by Win32 callback ABI — verify no panics can unwind across the FFI boundary (UB) |
| `platform/windows/mod.rs:395` | `std::thread::spawn(|| unsafe { ... })` — entire thread body is unsafe | The broadest unsafe scope in the codebase; every operation inside must be individually justified |

**Action:** For each site, verify: (a) the unsafe is necessary, (b) the invariants are documented in a comment, (c) the scope is as narrow as possible.

---

### 2. Error Handling — `unwrap` / `expect` Outside Tests

**Check:** Any `unwrap()` or `expect()` in non-test production code can panic and crash the app silently. Desktop apps must not panic on input.

| Location | Expression | Risk | Fix |
|---|---|---|---|
| `platform/macos/input.rs:24` | `.expect("Failed to create CGEventSource")` | CGEventSource creation fails if Accessibility permission is revoked mid-run — panics the injection path | Return `Err` and propagate; caller in `StateActor::inject_input` already handles `InputProvider` returning results |
| `platform/macos/mod.rs:27` | `CString::new(...).unwrap()` | Only panics if string contains interior NUL; safe for this literal — add a comment asserting this | Low risk but document |
| `platform/macos/observer.rs:68,135,140,147,159,209,266,408,422,463` | `.lock().unwrap()` on static `OnceLock<Mutex<...>>` | Panics if a thread holding the lock panics (poisoned mutex). Platform observer runs in a background thread where panics are possible. | Use `.lock().unwrap_or_else(\|e\| e.into_inner())` ("poison recovery") or document why poisoning cannot occur |
| `platform/windows/mod.rs:51,173,188,196,208,331` | `.lock().unwrap()` on static Mutex | Same poisoning risk as macOS static mutexes | Same fix |
| `platform/windows/mod.rs:425` | `hook.unwrap()` inside a `stop_observing` path | Called only when `hook.is_err()` branch was NOT taken — logically safe but fragile; replace with `if let Ok(h) = hook` | Low risk; clarity fix |
| `scheduler/mod.rs:378,387,470,476` | `.unwrap()` on channel `send` / task join in tests | Confirm all four are inside `#[tokio::test]` modules — if any are in production code paths, they must be replaced | Verify they are test-only |
| `persistence.rs:262,269` | `.unwrap()` in test module | Confirmed test-only — acceptable; add a `#[cfg(test)]` guard assertion at the top of the block for clarity | Low risk |
| `lib.rs:104` | `.expect("error while running tauri application")` | Last-resort at app entry; Tauri itself documents this pattern as acceptable — acceptable, verify comment explains this | Acceptable |

**Action:** Audit every `.unwrap()` with `rg 'unwrap\(\)' src-tauri/src --no-heading` and classify each as: test-only, justified-literal, poison-recovery-needed, or needs-propagation.

---

### 3. Memory / Resource Leaks in Async Tasks

**Check:** Long-lived tasks and threads must be tracked. If they can exit silently, the app continues running in a degraded state with no feedback.

| Issue | Location | Consequence |
|---|---|---|
| `MacPlatformObserver` dropped at end of setup closure | `lib.rs:70-72` | `_observer_token` (raw `usize` pointer to `NSObject`) is never reclaimed. `stop_observing` is dead code. Benign now; breaks if graceful shutdown is ever needed. **Fix:** Store observer in `tauri::Manager::manage()` so it lives for the application lifetime. |
| `std::thread::spawn` for Windows hook — no join handle stored | `platform/windows/mod.rs:395` | If the hook thread panics, the process continues without any keyboard/mouse observation. No error is surfaced. **Fix:** Store the `JoinHandle`, monitor it, or use `std::thread::Builder::spawn().expect()` with a panic hook. |
| macOS CGEventTap thread — `thread::spawn` at `observer.rs:186` | `platform/macos/observer.rs` | Same — panic in the CGEventTap thread silently stops all hotkey processing. |
| `action_tx` channel capacity 100 with `try_send` | `lib.rs:25`, `scheduler/mod.rs:301` | At 32 macros × 5ms = 6400 actions/s, channel saturates and drops fire silently. **Audit:** Either raise capacity or add a dropped-action counter in debug builds. |
| State, scheduler, and hotkey channels all unbounded-drop | `state_tx`, `sched_tx`, `action_tx` — all capacity 100, all `try_send` | Silent data loss under load. No counter, no log, no backpressure. Add at-least-debug-mode drop logging. |

---

### 4. IPC Command Surface — Privilege Scope Review

**Check:** Tauri 2's capability model restricts which commands the WebView can call. Every command must be in `capabilities/default.json` and must be appropriately scoped. Commands with no side effects are low risk; commands that mutate state, touch the filesystem, or trigger OS-level actions are high risk.

| Command | Risk Level | Current State | Notes |
|---|---|---|---|
| `add_macro`, `remove_macro`, `set_macro_enabled` | Medium | Exposed via `core:default` catch-all | Fine for a local desktop app; no network surface |
| `toggle_engine` | High | Same catch-all | Enables/disables all input injection — most powerful command; acceptable since frontend is same-process |
| `request_accessibility` | High | Same catch-all | Triggers OS permission dialog. Acceptable — only the user can approve it anyway |
| `save_profile`, `load_profile`, `delete_profile` | Medium-High | Same catch-all | Writes to OS app data dir. `ProfileManager::sanitize_name` strips path traversal — **verify this is tested** |
| `update_step_interval` | Low-Medium | Same catch-all | Accepts `step_index: usize` — validate bounds before forwarding `SchedulerIntent` (currently missing; see CONCERNS.md) |
| `bind_hotkey`, `unbind_hotkey` | Medium | macOS-only; Windows silently succeeds | Silent no-op on Windows is misleading but not a security issue |
| `get_state`, `get_active_app` | Low | Read-only | Fine |

**Capability file audit (`src-tauri/capabilities/default.json`):**
- Currently grants `core:default` and `opener:default` to window `main`.
- `core:default` is a broad grant — in Tauri 2, this enables all core plugin commands. Verify which core plugin commands are actually needed and consider explicit allowlisting once the feature set is stable.
- `opener:default` is used for shell/URL open; confirm this is actually called and not a leftover.

**Action:** Run `grep -rn "invoke(" src/ | awk -F'"' '{print $2}' | sort -u` to enumerate every command the frontend actually calls, then cross-reference with `invoke_handler!` in `lib.rs` to ensure no registered commands are unintentionally exposed.

---

### 5. Tauri 2 Security Model Compliance

**Check:** Tauri 2's security model is meaningfully stricter than Tauri 1. The following areas need verification.

| Area | Current State | Required Action |
|---|---|---|
| CSP in `tauri.conf.json` | `default-src 'self'; style-src 'self' 'unsafe-inline' ...` | `'unsafe-inline'` in `style-src` is required for Tailwind CSS 4 (it generates inline styles). This is acceptable for a local app but must be documented as an intentional trade-off. Remove if Tailwind ever moves to class extraction only. |
| `connect-src ipc: http://ipc.localhost` | Current CSP | Correct for Tauri 2 IPC — `ipc:` scheme is the Tauri 2 bridge. Do not change. |
| External font sources in CSP | `fonts.googleapis.com` and `fonts.gstatic.com` | If no Google Fonts are actually loaded, remove these entries to reduce the attack surface. **Audit `App.css` and `index.html` for `@import` or `<link>` to Google Fonts.** |
| macOS entitlements | `"entitlements": null` in `tauri.conf.json` | Unsigned builds are blocked by Gatekeeper on non-developer Macs. For CGEventTap to work in notarized builds, `com.apple.security.automation.apple-events` entitlement is required. Create an entitlements plist file and configure `tauri.conf.json` for production. |
| macOS signing | `"signingIdentity": null` | Expected for development; must be set via CI secret for any real distribution. |
| Windows signing | `"certificateThumbprint": null` | Same — acceptable for dev; required for distribution to avoid SmartScreen warnings. |
| Hardened runtime | Not configured | Required for macOS notarization alongside the CGEventTap entitlement. |

---

## SolidJS Frontend Checklist

### 1. Dead Code and Unused Imports

**Check:** `src/App.tsx` is the entire frontend in a single file. Dead signals or handlers are hard to spot.

| Item | Location | Status |
|---|---|---|
| `setLoading` signal setter | `App.tsx:71` | Signal is `[, setLoading]` — the getter is discarded. The `setLoading(true/false)` calls run but the value is never read. Either expose the getter to show a loading state in the UI, or remove the signal entirely. |
| `Key` branch in `handleCreateMacro` | `App.tsx:163-165` | Dead code path — the `select` element at line 458 only ever produces `"Left"`, `"Right"`, or `"Middle"`, so `{ Key: parseInt(inputVal) \|\| 0 }` is never executed. Must be made reachable when keyboard key support is added. |
| `For` import from `solid-js` | `App.tsx:1` | Verify `<For>` is used in JSX for macro list rendering — if it is, this is fine. Confirm no `Array.map()` pattern is used instead (which bypasses SolidJS's fine-grained reactivity). |

**Action:** Run TypeScript compiler with `noUnusedLocals: true` in `tsconfig.json` — it is currently not set, which means unused variables are silent. Enable it.

---

### 2. State Management Issues

**Check:** SolidJS reactivity rules differ from React. Common mistakes cause stale reads or missed updates.

| Issue | Location | Detail |
|---|---|---|
| `createEffect` with async body does not track reactive reads after first `await` | `App.tsx:94` (initial load effect) | The `await Promise.all([...])` call is inside `createEffect`. Any reactive signals read after the first await are NOT tracked by SolidJS. In this case no signals are read after the await, so it is currently safe — but this is a footgun to document. |
| `listen` Promise not awaited in `onCleanup` | `App.tsx:119-121` | `unlisten.then((fn) => fn())` — this fires the unlisten asynchronously. If the component unmounts and remounts rapidly (unlikely in a desktop app but possible during HMR dev), the old listener might not be cleaned up before a new one is registered. Use `createEffect` with `const unlisten = await listen(...)` and an `onCleanup` that calls `unlisten()` synchronously instead. |
| Full `AppState` object serialized and pushed on every state mutation | `state/mod.rs:404` (Rust) + `App.tsx:116` (frontend) | StateActor calls `app_handle.emit("state-changed", &self.state)` on every intent, even trivial ones. SolidJS's `setState` replaces the entire signal, so every mutation triggers a full UI re-render. For the current macro count this is fine; at scale, consider diffing or sending only the changed field. |
| `activeProfile` signal not synchronized with loaded profile data | `App.tsx:76,241` | When `load_profile` succeeds, `setActiveProfile(name)` is called but only if the code at line 241 also updates it — verify the loaded profile name is consistently reflected in the signal to avoid UI desync. |

---

### 3. Missing Error Boundaries

**Check:** SolidJS supports `<ErrorBoundary>` components. Without them, a thrown error in any rendering expression crashes the entire UI.

| Location | Risk | Fix |
|---|---|---|
| Entire `<App />` tree | No `<ErrorBoundary>` anywhere in `App.tsx` | An unexpected `null` dereference or type mismatch in JSX rendering will white-screen the app. Wrap the top-level return in `<ErrorBoundary fallback={(err) => <div>Something went wrong: {err.message}</div>}>`. |
| Macro list rendering | `state()?.macros` access via optional chaining | Current null-guarding uses `Show` component and optional chaining — this is correct and prevents crashes. No change needed here. |
| Profile list rendering | Similar `profiles()` access | Guarded correctly with `<For each={profiles()}>`. |

---

### 4. Accessibility (WCAG Basics for Desktop App UI)

**Check:** Tauri uses a system WebView. Screen readers on macOS (VoiceOver) and Windows (NVDA/Narrator) interact with it via standard ARIA. An automation tool used by power users may also be used by users who rely on keyboard navigation.

| Item | Current State | Fix |
|---|---|---|
| Zero `aria-*` attributes anywhere in `App.tsx` | No `aria-label`, `aria-pressed`, `aria-live`, or `role` found | All interactive elements should have labels accessible to screen readers |
| Engine toggle button | `<button>` at line 307 — text content provides label but no `aria-pressed` state | Add `aria-pressed={state()?.engine_active ?? false}` to communicate toggle state |
| Macro enable/disable buttons | Line 705/718 area — uses text labels | Add `aria-pressed` for toggle buttons; add `aria-label` that includes the macro name |
| Accessibility status indicator | Shows "Granted" / "Denied" text | Wrap in an `aria-live="polite"` region so screen readers announce status changes when permissions are granted |
| Tab navigation | Two tabs ("Dashboard", "Profiles") | Add `role="tablist"`, `role="tab"`, `aria-selected` to tab buttons |
| Form inputs | Line 448, 469, 479, 501 — `<input>` elements | Verify each `<input>` has an associated `<label>` or `aria-label`. Raw `placeholder` text is not sufficient for accessibility. |
| Keyboard navigation | No `tabIndex` management visible | Ensure macro action buttons are reachable via Tab key; modals or expanded sections should trap focus |
| Color contrast | Dark theme CSS in `App.css` — not audited | Run the rendered UI through a contrast checker (e.g., browser dev tools accessibility panel) against WCAG AA (4.5:1 for normal text) |

---

## CI/CD Checklist

### 1. GitHub Actions Workflow Correctness (`release.yml`)

| Item | Current State | Issue / Action |
|---|---|---|
| `tauri-action@v0` — unpinned major version | `uses: tauri-apps/tauri-action@v0` | Major version tags (`@v0`) can receive breaking changes. Pin to a specific release or SHA (e.g., `tauri-apps/tauri-action@v0.5.20`) and update intentionally. Unpinned versions have caused silent breakage in Tauri projects when the action API changed. |
| `actions/checkout@v4` and `actions/setup-node@v4` | Correctly pinned to major | Acceptable; prefer SHA pinning for security-critical supply chain but `@v4` is standard practice |
| `dtolnay/rust-toolchain@stable` | Unpinned to stable channel | `stable` floats with Rust releases. A new stable release with a breaking change (rare but possible) will silently affect CI. Acceptable for most projects; pin to a specific toolchain version if you want reproducible builds. |
| macOS universal binary — `aarch64-apple-darwin,x86_64-apple-darwin` | Passed as `targets` to `rust-toolchain` | Correct target list. Verify `tauri-action` receives these as build args via `args: --target aarch64-apple-darwin --target x86_64-apple-darwin` or that `tauri-action@v0` infers them from the toolchain targets. Without explicit `args`, the action may only build for the runner's native arch. |
| macOS runner is `macos-latest` | Line 15 | `macos-latest` on GitHub Actions currently maps to `macos-14` (Apple Silicon, arm64). Verify the x86_64 cross-compilation step actually produces a correct fat binary. If both targets are just added to the toolchain but `tauri build` is not passed `--target`, only the native (arm64) binary is produced. |
| No caching of Rust build artifacts | Workflow has no `actions/cache` for `~/.cargo` or `target/` | Each CI run recompiles from scratch. Add `Swatinem/rust-cache@v2` to cut build times significantly. |
| No caching of npm dependencies | No `actions/cache` for `node_modules` | Add `actions/setup-node@v4` cache option (`cache: 'npm'`) or explicit cache step. |
| `permissions: contents: write` at top level | Line 7 | This grants write permission to the entire workflow, not just the release step. Scope it to the job that needs it if possible, or at minimum document why job-level scoping was not used. |
| `releaseDraft: true` | Line 43 | Releases are created as drafts — must be manually published. This is intentional (verify). If releases should auto-publish on tag push, set `releaseDraft: false`. |
| No `fail-fast: false` interaction audit | Line 12 | `fail-fast: false` means if the Windows build fails, the macOS build continues and vice versa. This is correct for release artifacts — you want both platforms attempted. |

---

### 2. Code Signing Configuration

| Item | Current State | Required Action |
|---|---|---|
| macOS `signingIdentity: null` | `tauri.conf.json:48` | For distribution, set via `APPLE_SIGNING_IDENTITY` environment variable in CI. The workflow has no `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, or `APPLE_ID` secrets configured. Add these secrets and corresponding `env:` entries to the release step. |
| macOS `entitlements: null` | `tauri.conf.json:47` | Create `src-tauri/entitlements.plist` with at minimum `com.apple.security.cs.allow-jit` (for WebView JIT) and `com.apple.security.automation.apple-events` (for CGEventTap in notarized builds). Reference it in `tauri.conf.json`. |
| macOS notarization | Not configured | After signing, notarization requires `APPLE_ID`, `APPLE_PASSWORD` (app-specific password), and `APPLE_TEAM_ID` secrets. Add `notarize: true` (or equivalent `tauri-action` option) to the release step. Without notarization, macOS 13+ will block the app for all users. |
| Windows `certificateThumbprint: null` | `tauri.conf.json:52` | For distribution, set via `TAURI_WINDOWS_CERTIFICATE_THUMBPRINT` or equivalent. SmartScreen will warn users on unsigned Windows installers. |
| Windows timestamp URL uses HTTP | `"timestampUrl": "http://timestamp.digicert.com"` | HTTP timestamp URLs are technically valid but HTTPS is preferred. Some signing workflows fail with HTTP. Change to `https://timestamp.digicert.com`. |

---

### 3. Release Artifact Naming and Badge Expectations

| Item | Current State | Issue |
|---|---|---|
| `releaseName: "AutoMux ${{ github.ref_name }}"` | Line 41 | Release names include the tag (e.g., `AutoMux v1.1.0`). Verify this matches the expected format for any shields.io dynamic badges or README links that reference release asset URLs. |
| Artifact filenames produced by `tauri-action` | Not explicitly configured | Default artifact names follow Tauri's conventions: `AutoMux_1.1.0_aarch64.dmg`, `AutoMux_1.1.0_x64-setup.exe`, etc. If the README has hardcoded download links, these must match exactly. |
| `productName` in `tauri.conf.json` | `"AutoMux"` | Tauri uses `productName` as the artifact base name. Verify it does not contain spaces or special characters that cause artifact path issues on Windows CI runners. `AutoMux` (no spaces) is fine. |
| GitHub Releases badge | README uses shields.io release badge | The badge was broken when the release was in draft state. Now that releases use `releaseDraft: true`, every CI release starts as a draft — the release badge will always show "no release" until manually published. Either set `releaseDraft: false` or document this workflow. |
| License badge | README shows "not found" | The SPDX identifier must be `GPL-3.0-only` or `GPL-3.0-or-later`, not `GPLv3`. Fix in `package.json` `"license"` field and the shields.io badge URL. |

---

### 4. Common Tauri 2 CI Mistakes

| Mistake | Status in This Project | Notes |
|---|---|---|
| Forgetting `beforeBuildCommand` runs frontend build | `tauri.conf.json:9` has `"beforeBuildCommand": "npm run build"` — correct | `tauri-action` will run this automatically; do not add a separate `npm run build` step in the workflow or the frontend will be built twice |
| Using Tauri 1 `allowlist` config instead of Tauri 2 capabilities | Not present — project correctly uses `capabilities/` directory | Correct |
| `gen/schemas/` in `.gitignore` | Not in `.gitignore` | The `src-tauri/gen/schemas/desktop-schema.json` referenced in `capabilities/default.json` is generated by Tauri CLI. It should be in `.gitignore` and regenerated during CI, or committed if it is stable. **Audit whether it is committed or generated.** |
| Missing Tauri CLI in CI | `tauri-action` bundles its own Tauri CLI installation — correct | No separate `npm install -g @tauri-apps/cli` needed |
| Platform-specific targets not installed for cross-compilation | `targets: aarch64-apple-darwin,x86_64-apple-darwin` added to `rust-toolchain` | Correct for the toolchain step; verify `tauri-action` actually passes `--target` for both during build (see macOS runner note above) |
| Node version mismatch between dev and CI | CI pins Node 24 via `setup-node@v4`; `package.json` has no `engines` field | Add `"engines": { "node": ">=24" }` to `package.json` to make this explicit and prevent accidental downgrade |

---

## Repository Hygiene Checklist

### 1. Files That Should Be in `.gitignore` but Aren't

| File/Pattern | Current `.gitignore` Status | Action |
|---|---|---|
| `src-tauri/gen/` directory | Not in `.gitignore` | `src-tauri/gen/` contains generated schemas and bindings produced by `tauri build` or `tauri dev`. These should not be committed unless the project requires a stable schema for external consumers. Add `src-tauri/gen/` to `.gitignore` and regenerate in CI. |
| `src-tauri/target/` | Already in `.gitignore` — correct | No action |
| `node_modules/` | Already in `.gitignore` — correct | No action |
| `dist/` | Already in `.gitignore` — correct | No action |
| `.planning/` directory | Not in `.gitignore` | Planning docs are internal development artifacts. They should be `.gitignore`d unless the project intentionally commits them for collaboration. Currently committed — decide policy. If committed, ensure no sensitive data (API keys, personal notes) ends up there. |
| OS-generated files | `.DS_Store` in `.gitignore` — correct | Add `Thumbs.db` and `desktop.ini` for completeness (Windows OS artifacts) |
| Editor config directories | Not in `.gitignore` | Add `.vscode/`, `.idea/`, `*.swp`, `*.swo` if not already handled by global gitignore |

### 2. Planning / Temp Files Committed to Root

| File | Location | Action |
|---|---|---|
| `implementation_plan.md` | Repository root | Contains unchecked `[ ]` task items for work that is already implemented. Move to `archive/` or delete. Do not leave in root — misleads contributors about project status. |
| `archive/v1-production/implementation_plan.md` | `archive/v1-production/` | Historical artifact — acceptable to keep in `archive/` but confirm it is never imported from. |
| `archive/v1-production/state.md` | `archive/v1-production/` | Same — historical only. |
| `docs/ARCHITECTURE_VALIDATION.md` | `docs/` | Verify this is current or archive it. Stale validation docs are more dangerous than no docs (they create false confidence). |

### 3. Binary Artifacts or Large Files That Shouldn't Be Tracked

| Item | Status | Action |
|---|---|---|
| `src-tauri/icons/` | Committed — correct | Icon assets must be committed for Tauri builds. No action. |
| `public/tauri.svg`, `public/vite.svg` | Committed | Template scaffold assets. If not shown in the UI, remove them to reduce clutter. |
| `src/assets/logo.svg` | Committed | App logo — correct to commit. |
| Archive build artifacts | `archive/v1-production/` | Confirm this directory contains only source files, not compiled binaries (`.exe`, `.dmg`, `.app`). Run `find archive/ -name "*.exe" -o -name "*.dmg"` to verify. Binary release artifacts do not belong in git — they inflate repo size permanently. |

### 4. Additional Hygiene

| Item | Action |
|---|---|
| `CHANGELOG.md` | Verify it is maintained and reflects the v1.1.0 release. An empty or stale CHANGELOG signals an unmaintained project to new contributors. |
| Root `README.md` broken badges | Fix the license badge (SPDX `GPL-3.0-only`). Audit all shields.io badge URLs for correctness after each release. |
| No `engines` field in `package.json` | Add `"engines": { "node": ">=24" }` to formalize the Node version requirement visible in CI. |
| No `.editorconfig` | Optional but recommended for cross-contributor consistency. Not a blocker. |

---

## Priority Order

Ordered by: risk to correctness and user trust first, then maintainability, then polish.

### Priority 1 — Safety and Correctness (fix before any new feature)

1. **`platform/macos/input.rs:24` — `.expect()` on CGEventSource creation.** This is in the hot input injection path. A revoked Accessibility permission mid-run will panic the entire app rather than gracefully stopping. Fix: return `Err` and propagate. *File: `platform/macos/input.rs`*

2. **`platform/macos/observer.rs` — all `.lock().unwrap()` calls on static mutexes.** The macOS observer runs in a background thread. A panic in that thread poisons the mutex; the next lock attempt panics the StateActor or IPC path. Add poison recovery. *Files: `platform/macos/observer.rs`, `platform/windows/mod.rs`*

3. **`update_step_interval` missing bounds check.** The IPC command accepts an unchecked `step_index: usize` from the frontend. Validate `step_index < steps.len()` before forwarding `SchedulerIntent`. *File: `ipc/mod.rs`, `state/mod.rs`*

4. **Minimum interval floor is 1ms, not 5ms.** A 1ms interval floods the OS input queue. Change `interval_ms.max(1)` to `interval_ms.max(5)` in two places. *File: `scheduler/mod.rs:66,184`*

5. **Emergency stop race condition.** `process::exit(1)` can win over `flush_held_inputs`. Verify the synchronous inline flush in the CGEventTap callback is sufficient (macOS appears to do this); confirm Windows does the same inline flush before exit. *Files: `platform/macos/observer.rs:322`, `platform/windows/mod.rs:352`*

### Priority 2 — Silent Data Loss (fix before shipping new features)

6. **No auto-save on macro changes.** Users lose all changes when closing the app unless they manually save. The `persistence.rs` header comment falsely advertises auto-save. Either implement auto-save or remove the misleading comment. *Files: `state/mod.rs`, `persistence.rs`*

7. **Silent channel drops under load.** All three mpsc channels (`state_tx`, `sched_tx`, `action_tx`) use `try_send` and drop messages silently when at capacity 100. Add debug-mode drop counters or log warnings. *Files: `lib.rs:25`, `scheduler/mod.rs:301`*

8. **`MacPlatformObserver` dropped silently.** Store it in `tauri::Manager::manage()` so it lives for the application lifetime and `stop_observing` becomes callable. *File: `lib.rs:70-72`*

### Priority 3 — CI and Distribution (fix before next public release)

9. **Pin `tauri-apps/tauri-action@v0`** to a specific release SHA. Floating major versions have silently broken CI for Tauri projects during API changes.

10. **Verify macOS universal binary is actually built.** Confirm `tauri-action` receives `--target aarch64-apple-darwin` and `--target x86_64-apple-darwin` as build args, not just as installed toolchain targets.

11. **Add Rust build caching** (`Swatinem/rust-cache@v2`) to CI — reduces build time from ~15min to ~3min on warm cache.

12. **Create macOS entitlements plist** and configure `tauri.conf.json` — required before any distribution attempt to non-developer Macs.

13. **Fix Windows timestamp URL** from HTTP to HTTPS.

### Priority 4 — Repository Hygiene (clean up before inviting contributors)

14. **Remove `implementation_plan.md` from root** — move to `archive/` or delete.

15. **Audit `archive/v1-production/` for binary artifacts** — if `.exe` or `.dmg` files are present, remove them and rewrite history or accept the repo size cost.

16. **Add `src-tauri/gen/` to `.gitignore`** — generated Tauri schema files do not belong in version control.

17. **Fix hardcoded version string** in `App.tsx:291` — use `getVersion()` from `@tauri-apps/api/app` instead.

18. **Fix comment/interval mismatch** — `App.tsx:124` says "3s" but uses 10000ms. Pick one and make the code match.

### Priority 5 — Frontend Quality (address in parallel with new features)

19. **Add `<ErrorBoundary>` at top level of `App.tsx`** — prevents white-screen crashes from unexpected rendering errors.

20. **Audit `setLoading` signal** — the getter is discarded; either expose it in the UI or remove the signal.

21. **Add `aria-pressed` to toggle buttons** — engine toggle and per-macro enable/disable buttons need ARIA state attributes for screen reader users.

22. **Enable `noUnusedLocals: true` in `tsconfig.json`** — catches dead signals and imports at compile time rather than at audit time.

---

*Audit architecture: 2026-05-15*
