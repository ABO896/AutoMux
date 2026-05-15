# Domain Pitfalls

**Domain:** Tauri 2 desktop auto-clicker/macro tool (macOS + Windows)
**Analyzed:** 2026-05-15
**Confidence:** HIGH — all pitfalls are grounded in the actual codebase or documented Apple/Win32 behaviors verified through the source. External fetch tools were unavailable; analysis draws on code evidence plus well-established platform behaviors (training knowledge cutoff August 2025).

---

## 1. macOS Accessibility / Input Automation

---

### CRITICAL: AXIsProcessTrusted Returns False After Grant — No Restart

**What goes wrong:** The process identity that macOS anchors Accessibility trust to is the combination of code-signing identity (or unsigned-app hash for development builds) and the bundle identifier. When `AXIsProcessTrusted()` is called and returns `false` right after the user grants permission in System Settings, the OS has updated its database but the calling process is still identified against the **pre-grant snapshot**. The permission does not apply to the already-running process until the process is relaunched (or until the OS flushes its internal TCC cache on re-check).

**Evidence in codebase:** `ipc/mod.rs:check_accessibility` calls `AXIsProcessTrusted()` directly; `App.tsx` polls it every 10 seconds. After granting permissions, a user will wait up to 10 seconds, see `false` still returned, click "Request Access" again, and get sent back to System Settings — even though they already granted it.

**Why it happens:** macOS TCC (Transparency, Consent, Control) stores the decision persistently, but `AXIsProcessTrusted()` uses the Mach audit token of the calling process to look up the database entry. An unsigned or ad-hoc-signed build gets a dynamic identity computed from the binary's hash. If the binary has been modified (e.g., Tauri hot-reload, Rust recompile) since the trust was recorded, the identity no longer matches and the check returns `false`.

**Warning signs:**
- User reports "I already gave permission but the app keeps asking"
- `check_accessibility` returns `false` in polling but Accessibility settings shows the app as enabled
- Happens consistently on dev builds after recompile

**Prevention/Fix:**
1. After `AXIsProcessTrusted()` returns `false` and the user is sent to System Settings, re-check on next poll with a short retry window (3s, not 10s) — the fix for the comment/interval mismatch in `App.tsx:124-134` belongs here.
2. For development: understand that each `cargo build` changes the binary hash, invalidating the TCC entry. Developers must re-grant after every recompile of the unsigned binary.
3. For production: code-signing with a stable identity (Apple Developer ID or ad-hoc) ensures the TCC database entry survives across relaunches as long as the identity is consistent.
4. Never use `AXIsProcessTrustedWithOptions(prompt: true)` as a substitute for a proper re-check loop — it prompts the dialog every call, not just when needed.

**Phase:** Milestone 1 (permissions fix) — the re-check timing fix is the immediate change; the signing identity guidance belongs in release documentation.

---

### CRITICAL: CGEventTap Disabled by OS — No Detection, No Recovery

**What goes wrong:** macOS will automatically disable a `CGEventTap` if it detects the callback is taking too long (`kCGEventTapDisabledByTimeout` — the OS sends this as a synthetic event type through the same callback channel). When this happens, the tap stops receiving events silently. There is no crash, no error, no log message from the OS. The app appears to be running normally, but hotkeys stop working and the held-input registry stops updating.

**Evidence in codebase:** `observer.rs:initialize_tap()` (line 186–383) creates the tap and runs `CFRunLoop::run_current()`. The callback at line 206 is a `ListenOnly` passive tap — passive taps have a longer timeout budget but are not immune. Critically, the callback acquires `get_registry().lock()` (a `Mutex`) inside the tap callback for every single mouse event (including `MouseMoved`, `LeftMouseDragged`, etc.). If the Mutex is ever contended — e.g., `flush_held_inputs()` holds it during emergency stop — the callback blocks, and the OS timer for the tap starts ticking down.

**The missing piece:** The callback never handles `CGEventType` values that represent the tap being disabled. The OS delivers `kCGEventTapDisabledByTimeout` (value 0xFFFFFFFE on macOS) and `kCGEventTapDisabledByUserInput` (0xFFFFFFFD) as event types through the same callback. The current code's `match event_type` arms have no handler for these values. The fix is: when the callback receives `kCGEventTapDisabledByTimeout`, call `CGEventTapEnable(tap_ref, true)` to re-enable it.

**Warning signs:**
- Hotkeys stop responding after extended runtime, especially when macros were running at high rates
- Emergency stop sequence (Cmd+Shift+Q) stops working
- `is_tap_initialized()` returns `true` but no events arrive

**Prevention/Fix:**
1. In the CGEventTap callback, add a branch for `CGEventType::Null` or the numeric values for tap-disabled events. Call `CGEventTapEnable` to re-enable.
2. Reduce lock contention in the callback: the `get_registry().lock()` call happens on *every* event including `MouseMoved` and drag events (high frequency). Use `try_lock()` with a no-op fallback for the registry update in non-critical paths, or batch the registry update outside the callback.
3. Consider tracking tap health via a watchdog: periodically verify the tap is still enabled by checking `CGEventTapIsEnabled`.

**Phase:** Milestone 1 or a dedicated reliability phase. This is the root cause of the "macOS permissions false positive" class of bugs — a disabled tap is indistinguishable from a permission-denied tap from the frontend's perspective.

---

### HIGH: App Sandboxing Incompatibility with CGEventTap

**What goes wrong:** `CGEventTap` with `CGEventTapLocation::HID` (the HID event tap that sees all system input) requires that the process **not** be sandboxed, or that it hold specific entitlements. A sandboxed app cannot use HID taps even with Accessibility permissions granted. This affects App Store distribution and any build with the sandbox entitlement enabled.

**Evidence in codebase:** `tauri.conf.json` sets `entitlements: null` — no entitlements file is referenced. Tauri 2 does not automatically add a sandbox entitlement, so local/CI builds are currently unsandboxed. This is correct for the current use case. The risk is that a future attempt to add App Store distribution or inadvertently enabling `com.apple.security.app-sandbox` would silently break all CGEventTap functionality.

**Warning signs:**
- `CGEventTap::new()` returns `Err` immediately after enabling sandbox
- App previously working on a developer machine fails on a new machine where a signing profile adds sandbox
- `tauri.conf.json` or a CI step adds `--sandbox` or an entitlements file with `com.apple.security.app-sandbox: true`

**Prevention/Fix:**
1. Do not add App Sandbox entitlement. AutoMux's core feature (system-wide input monitoring) is fundamentally incompatible with App Store sandbox restrictions.
2. Document explicitly in CI comments and release checklist that sandbox must remain disabled.
3. If App Store distribution becomes a goal in future, the only path is a LaunchAgent/daemon helper process outside the sandbox that holds the tap and communicates via XPC — a significant architectural change.

**Phase:** Release workflow phase / documentation. No code change needed now, but must be a hard guard in release checklist.

---

### HIGH: Code Signing Identity Change Breaks Existing TCC Grants

**What goes wrong:** macOS TCC stores Accessibility grants keyed to the signing identity. If the signing identity changes between releases (e.g., from unsigned/ad-hoc to a real Developer ID, or if the certificate is renewed with a different Team ID), existing users' TCC grants are silently invalidated. The app will appear in System Settings Accessibility list but as a different entry, and `AXIsProcessTrusted()` returns `false`.

**Evidence in codebase:** `tauri.conf.json:signingIdentity: null` — currently unsigned. When a real signing certificate is added (which CONCERNS.md flags as needed for production), every existing user who had previously granted Accessibility will need to re-grant. There is currently no user-facing explanation for why permissions were "lost."

**Warning signs:**
- After a version update, users report that permissions were revoked even though they did not change anything
- App appears twice in Accessibility settings list (old unsigned entry + new signed entry)

**Prevention/Fix:**
1. Choose a signing identity *once* and keep it consistent. Do not change Team ID or certificate between releases.
2. On the first release that introduces signing, include prominent release notes warning users to re-grant Accessibility if they had a previous version installed.
3. The UI should detect the "permissions lost" state gracefully and guide users through re-granting rather than silently failing to fire macros.

**Phase:** Release workflow phase. The entitlements/signing setup in `tauri.conf.json` must be finalized before any public release with signing.

---

## 2. Cross-Platform Input Injection

---

### CRITICAL: Windows Emergency Stop Does Not Flush Held Inputs Before Exit

**What goes wrong:** On Windows, the emergency stop handler (`windows/mod.rs:347-353`) calls `tx.try_send(TriggerEmergencyStop)` then immediately calls `std::process::exit(1)`. Unlike the macOS path — which performs an inline registry flush before exit — the Windows path does not flush held keys or mouse buttons before calling `exit(1)`. If `try_send` succeeds but the StateActor hasn't processed it yet (likely, given `try_send` is non-blocking), the `flush_held_inputs()` call in the StateActor never runs. Keys remain stuck.

**Evidence in codebase:** `windows/mod.rs:349-353`:
```rust
let _ = tx.try_send(crate::state::Intent::TriggerEmergencyStop);
std::process::exit(1);
```
There is no inline flush before `exit(1)`, unlike macOS which manually iterates the registry and posts KeyUp events before calling `process::exit(1)`.

**Warning signs:**
- On Windows, triggering emergency stop (Ctrl+Shift+Q) while a Hold macro is active leaves keys stuck
- User must manually press the stuck key to release it

**Prevention/Fix:** Call `WindowsInputProvider::flush_all_held_inputs()` synchronously before `process::exit(1)` in the Windows hook callback. Model it directly on the macOS path.

**Phase:** Immediate fix — this is a safety regression.

---

### HIGH: macOS Uses Bundle ID for Process Matching; Windows Uses Process Name — Incompatible Mental Model

**What goes wrong:** On macOS, `target_app` is matched against the app's `bundleIdentifier` (e.g., `com.apple.safari`). On Windows, the active app tracking uses `GetWindowThreadProcessId` + process name lookup, which returns the executable filename (e.g., `safari.exe`). Users who configure a macro with `target_app = "com.apple.safari"` on macOS and then expect it to work on Windows with the same string will be confused when it doesn't. More critically, users on Windows who set `target_app` to a bundle ID will find it never matches.

**Evidence in codebase:** `windows/mod.rs:win_event_hook_callback` calls `get_app_name_from_hwnd(hwnd)` which returns the executable name. `macos/observer.rs:start_observing` uses `app.bundleIdentifier()`.

**Warning signs:**
- Process-targeted macros work on macOS but never trigger on Windows (or vice versa)
- User enters `com.apple.finder` as the target app on Windows

**Prevention/Fix:**
1. The UI must communicate the platform difference. When a user sets a target app, label the field differently per OS: "Bundle ID (macOS)" vs "Process name (Windows)".
2. The process picker UX improvement (currently in scope) should surface the correct format automatically — the picker will enumerate running processes in the native format for the current OS, so users never type the wrong format.
3. In documentation, note the incompatibility: profiles with `target_app` set are not portable between macOS and Windows.

**Phase:** Process picker implementation phase.

---

### MODERATE: Windows inject_mouse_click Ignores Coordinates

**What goes wrong:** `WindowsInputProvider::inject_mouse_click(x, y, button)` accepts coordinates but ignores them, posting the mouse button event at the current cursor position instead. The trait signature promises click-at-coordinates behavior. Any future feature that relies on clicking at a specific screen position will silently click at the wrong location on Windows.

**Evidence in codebase:** `windows/mod.rs:183-199` — comment at line 185 acknowledges the coordinate parameters are ignored.

**Warning signs:**
- A feature is added to click at a specific (x, y) coordinate; it works on macOS but not Windows
- Click-at-position macros appear to always click wherever the cursor happens to be on Windows

**Prevention/Fix:** Implement coordinates properly using `MOUSEINPUT` with `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE` flags to move to the target position before the click event.

**Phase:** Fix when click-at-position feature is added. Not urgent for current scope (current macros click at current cursor position by design).

---

### MODERATE: Race Condition — Hotkey Toggles Dropped at High Frequency

**What goes wrong:** The `try_send` pattern in both `macos/observer.rs` (lines 341, 354) and `windows/mod.rs` (lines 350, 359) for dispatching hotkey toggles is non-blocking and silently drops the message if the channel is at capacity. With a channel capacity of 100 and a rapidly-pressed hotkey, toggle presses can be lost. A user quickly toggling a macro on/off may find the state ends up opposite to what they intended.

**Evidence in codebase:** Both platform observers use `tx.try_send(Intent::ToggleMacroHotkey(...))`. Channel capacity is 100 (`lib.rs:25`). Rapid hotkey presses in burst do not queue — they are dropped.

**Warning signs:**
- User presses a hotkey twice quickly and macro ends up in original state (two presses = on+off)
- Stress testing hotkey dispatch shows non-100% toggle fidelity

**Prevention/Fix:** For hotkey dispatch specifically, consider a dedicated unbounded channel or a locking queue, since hotkey presses are low-frequency by human interaction standards and the backpressure concern is minimal. Alternatively, log dropped intents in debug builds so the issue is detectable.

**Phase:** Can be deferred unless stress testing surfaces it. Low priority given human hotkey press rates.

---

## 3. Tauri 2 IPC and State

---

### HIGH: State Desync — No Auto-Save After Mutations

**What goes wrong:** The persistence layer comment says "auto-saved on changes," but the StateActor never calls `save_profile` after mutations (`AddMacro`, `RemoveMacro`, `SetMacroEnabled`, etc.). Users configure macros and close the app — all work is lost. The frontend shows the changes (via `state-changed` event), confirming the round-trip works, but after relaunch the default profile on disk has the old state.

**Evidence in codebase:** `CONCERNS.md` documents this as a tech debt item. `state/mod.rs:handle_intent` never calls `ProfileManager::save_profile`. `persistence.rs` line 9 comment says "auto-saved on changes" — this is false.

**Warning signs:**
- User creates macros, closes app, reopens — macros are gone
- Profiles tab shows "Saved X" only if user explicitly saves

**Prevention/Fix:** Inject `ProfileManager` into `StateActor`. After any mutating intent (`AddMacro`, `RemoveMacro`, `SetMacroEnabled`, `SetMacroTargetApp`, `UpdateSequence`, `UpdateStepInterval`), call `save_profile("Default")` asynchronously. Fire-and-forget errors (log only) to avoid blocking the StateActor hot path. Fix the misleading comment in `persistence.rs`.

**Phase:** Milestone 1 or earliest active phase — this is a silent data loss bug that undermines user trust.

---

### HIGH: Tauri Event Listener Not Cleaned Up — Potential Listener Accumulation

**What goes wrong:** In SolidJS, reactive effects re-run when their tracked signals change. If a `createEffect` that sets up a Tauri `listen` call is inside a context that re-mounts (e.g., conditional rendering with `Show`), and the cleanup (`onCleanup`) is not wired correctly, multiple listeners accumulate. Each fires independently on the same `state-changed` event, causing multiple `setState` calls per event and potential stale-closure bugs.

**Evidence in codebase:** `App.tsx:114-122` — the `listen` effect correctly uses `onCleanup(() => unlisten.then(fn => fn()))`. This is properly implemented. However, the pattern is fragile: `unlisten` is a `Promise<() => void>`, and if the component unmounts before the Promise resolves (rare but possible on fast navigation), the cleanup lambda is never called because `then()` fires after unmount. The `onCleanup` itself runs synchronously but the actual unsubscribe is deferred.

**Warning signs:**
- Console shows multiple state updates per single backend event
- Memory profiling shows Tauri event listener count growing over time
- Rapid tab switching causes duplicate UI updates

**Prevention/Fix:** The current pattern is acceptable for this app (single `App` component, no conditional remounting of the listener effect). The risk is low as written. The key rule to enforce: never register a Tauri listener inside a conditional `Show` or `For` block — always register at the root component level with a paired `onCleanup`. If components are ever componentized further, use a dedicated store/context that holds the single listener.

**Phase:** Keep as a guard rule during any future componentization. Not an active bug in current single-component architecture.

---

### MODERATE: get_state IPC Round-Trip Pattern Creates Snapshot Inconsistency

**What goes wrong:** The `get_state` IPC command uses a `oneshot` channel to synchronously request a snapshot of `AppState` from the `StateActor`. Between the `send_intent(GetState)` call and the `StateActor` processing it, other intents may be processed, meaning the snapshot returned is already stale by the time the frontend receives it. The frontend also receives `state-changed` events pushed by the StateActor. If both arrive "at the same time" (within the same JS event loop tick), the frontend may apply them out of order.

**Evidence in codebase:** `ipc/mod.rs:46-52` (`get_state`), `ipc/mod.rs:204-218` (`save_profile` — uses `get_state` internally). The frontend's initial data fetch at `App.tsx:96-103` calls `get_state` once at startup.

**Warning signs:**
- On profile save, the saved profile is missing a macro that was just added
- Startup state shows stale values that are immediately overwritten by the first `state-changed` event

**Prevention/Fix:** The current design is acceptable because `get_state` is only used at startup and in `save_profile`. For `save_profile`, the snapshot is taken inside the `StateActor` message queue, so it reflects all prior mutations. The risk is mostly at startup where the initial `get_state` and the first `state-changed` event can race. Fix: at startup, prefer the `state-changed` event as the authoritative source if it arrives before the `get_state` response resolves.

**Phase:** Not urgent. Acceptable as-is for current scope.

---

### MODERATE: Minimum Interval Floor Too Low — OS Event Queue DOS

**What goes wrong:** The Scheduler enforces `interval_ms.max(1)` (a 1ms floor), but the documented minimum is 5ms. A user who sets a macro to 1ms will post OS input events at up to 1000Hz per action. On macOS, CGEvent posting at this rate can saturate the HID event queue; on Windows, SendInput at 1000Hz causes the UI to become unresponsive system-wide. This is a self-inflicted denial-of-service against the user's own machine.

**Evidence in codebase:** `CONCERNS.md` documents this. `scheduler/mod.rs:66, 184` enforce `max(1)`.

**Warning signs:**
- User sets interval to 1ms and system becomes unresponsive
- Mouse cursor freezes or becomes jerky
- Other apps stop receiving input

**Prevention/Fix:** Change `interval_ms.max(1)` to `interval_ms.max(5)` in both locations in `scheduler/mod.rs`. Add frontend validation that rejects intervals below 5ms before they reach the backend. Add a user-visible label "minimum 5ms" to the interval input field.

**Phase:** Milestone 1 — this is a documented spec violation that can cause system instability.

---

### LOW: Duplicate Trigger Key Registration Silently Drops First Binding

**What goes wrong:** If two macros are configured with the same `trigger_key`, `update_macro_trigger_keys` replaces the entire HashMap. The second macro to be processed by `reevaluate_all_macros` wins; the first macro's trigger key is silently unregistered. There is no error, no UI warning.

**Evidence in codebase:** `CONCERNS.md` documents this. `state/mod.rs:391-397` + `observer.rs:158`.

**Warning signs:**
- User assigns the same hotkey to two macros; only one responds
- No error or warning is shown

**Prevention/Fix:** Before registering a trigger key in `update_macro_trigger_keys`, check for conflicts and either reject the second binding with an error or warn the user. The frontend macro creation form should validate uniqueness of trigger keys.

**Phase:** UX improvement phase or any phase touching the trigger key / hotkey system.

---

## 4. README and Release Workflow

---

### HIGH: README Claims MIT License; Actual License Is GNU GPLv3

**What goes wrong:** `README.md` line 79 states "Distributed under the MIT License." The `PROJECT.md` confirms the actual license is GNU GPLv3. The shields.io license badge (`img.shields.io/github/license/ABO896/AutoMux`) auto-detects the license from the repository's `LICENSE` file. If the LICENSE file contains GPL-3.0 text, the badge will display "GPL-3.0" — but the README's prose text contradicts it, creating a legal ambiguity. For an open-source project, the license statement is a legally significant representation.

**Evidence in codebase:** `README.md:79` says MIT; `PROJECT.md` says GPLv3; `PROJECT.md` also notes "needs README badge fix."

**Warning signs:**
- The shields.io badge shows "GPL-3.0" but the text says MIT
- A contributor relies on the README's MIT claim and submits code under the assumption it will be MIT-licensed

**Prevention/Fix:**
1. Replace the prose "MIT License" with "GNU General Public License v3.0 (GPL-3.0)"
2. The shields.io badge `?style=flat-square` on `img.shields.io/github/license/ABO896/AutoMux` auto-detects from the LICENSE file — ensure the LICENSE file is GPLv3, then the badge is correct by default. No custom label needed.
3. The correct SPDX identifier for the license badge, if using a manual override, is `GPL-3.0-only` or `GPL-3.0-or-later` (not `GPL-3.0` which is deprecated in SPDX 3.x, though shields.io accepts the common form `GPL-3.0`).

**Phase:** Milestone 1 / README fix phase — this is the active known issue already called out in PROJECT.md.

---

### MODERATE: GitHub Releases Badge Behavior — Draft vs. Pre-Release vs. Release

**What goes wrong:** The `img.shields.io/github/v/release/ABO896/AutoMux` badge only picks up **published** releases, not drafts. Draft releases are invisible to the badge. Pre-releases are excluded by default unless `?include_prereleases` is appended. The `PROJECT.md` documents this exact failure: "Release badge was showing broken because release was in draft; now published and working."

**What breaks:**
- A release is created in GitHub as a draft → badge shows "No releases" or the previous version
- A pre-release is published but `?include_prereleases` is absent → badge skips to last stable release
- The tag format matters: the badge reads from the `tag_name` of the release object, not the branch. Tags must be semver-compatible (e.g., `v1.1.0`, not `release-1.1.0`) for shields.io to parse them correctly.

**Evidence in codebase:** `README.md:6` — badge URL is `img.shields.io/github/v/release/ABO896/AutoMux?style=flat-square&color=blue`. The current URL is correct for stable published releases. `PROJECT.md` documents the prior draft-release failure.

**Warning signs:**
- Badge shows an old version after a new release is published
- New release was published as a draft and not promoted

**Prevention/Fix:**
1. Always promote releases from draft to published. Never merge to master and tag without promoting the corresponding GitHub Release.
2. If pre-releases should be visible, append `?include_prereleases` to the badge URL.
3. Keep tag names in `v{semver}` format consistently.
4. Add a release checklist step: "Verify release badge at README shows new version before announcing."

**Phase:** Release workflow phase — build this into the standard release procedure.

---

### LOW: Hardcoded Version String in UI Will Always Be Wrong

**What goes wrong:** `App.tsx:291` hardcodes `v1.0.0` in the titlebar. `tauri.conf.json` declares `1.1.0`. They are already out of sync and will drift with every release.

**Evidence in codebase:** `CONCERNS.md` documents this. `App.tsx:291`.

**Prevention/Fix:** Replace with `getVersion()` from `@tauri-apps/api/app` called once at mount. Store in a signal. The Tauri API reads the version from `tauri.conf.json` at build time — single source of truth.

**Phase:** Any phase that touches `App.tsx` — low effort, high hygiene value.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Permissions fix (macOS) | AXIsProcessTrusted returns false even after grant | Reduce poll interval to 3s; improve user messaging; document dev-build recompile invalidation |
| Permissions fix (macOS) | CGEventTap disabled by OS timeout | Add tap-disabled event handler in callback; reduce lock contention in callback |
| README fix | License badge auto-detects from LICENSE file | Ensure LICENSE file is GPLv3; fix prose MIT → GPLv3 |
| README fix | Release badge misses draft releases | Checklist: promote draft before announcing; verify badge after promotion |
| Process picker | macOS bundle ID vs. Windows process name mismatch | Surface OS-appropriate format in picker; document non-portability of target_app |
| Key capture UX | Raw CGKeyCode / Virtual Key codes are not user-visible strings | Key capture widget must translate code → name at display time; mapping table needed |
| Auto-save implementation | StateActor never persists mutations | Inject ProfileManager into StateActor; call save after each mutating intent |
| Interval floor fix | 1ms minimum causes OS input queue saturation | Change max(1) to max(5) in scheduler; add frontend validation |
| Code signing / release | Signing identity change silently invalidates existing TCC grants | Finalize signing identity before first signed release; release notes for affected users |
| Windows emergency stop | No held-input flush before process::exit(1) | Add synchronous flush inline before exit, matching macOS pattern |

---

## Sources

- Codebase: `src-tauri/src/platform/macos/observer.rs` (CGEventTap initialization, callback, tap-disabled handling gap)
- Codebase: `src-tauri/src/platform/macos/mod.rs` (AXIsProcessTrusted call pattern)
- Codebase: `src-tauri/src/platform/windows/mod.rs` (Windows hook, emergency stop, coordinate-ignoring click)
- Codebase: `src-tauri/src/ipc/mod.rs` (IPC command patterns, accessibility commands)
- Codebase: `src-tauri/src/state/mod.rs` (StateActor, auto-save gap)
- Codebase: `src-tauri/tauri.conf.json` (signing identity null, entitlements null)
- Codebase: `src/App.tsx` (listener patterns, polling interval, hardcoded version, key-path dead code)
- Codebase: `README.md` (license claim mismatch, badge URLs)
- Planning: `.planning/codebase/CONCERNS.md` (tech debt, known bugs, fragile areas)
- Planning: `.planning/PROJECT.md` (active issues, context)
- Apple platform knowledge (training): TCC database behavior, CGEventTap timeout disabling (`kCGEventTapDisabledByTimeout`), App Sandbox CGEventTap incompatibility, signing identity TCC binding — HIGH confidence from well-established platform behaviors documented in Apple WWDC sessions and developer documentation.
- shields.io badge behavior: MEDIUM confidence from training data (shields.io documentation conventions).
