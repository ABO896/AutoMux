# Phase 7: carry-work-platform-ci-safety — Research

**Researched:** 2026-06-17
**Domain:** Rust platform code (macOS CGEventTap / TCC, Windows Win32), SolidJS event handling, GitHub Actions CI
**Confidence:** MEDIUM (all locked decisions are clear; D-05 resolved; key unknowns documented)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Input Monitoring detection (COMPAT-04)**
- D-01: Check `kTCCServiceListenEvent` at startup alongside Accessibility, using the same polling pattern from Phase 5.
- D-02: Replace the solo Accessibility indicator with a combined "Permissions" section listing both Accessibility and Input Monitoring.
- D-03: Input Monitoring "Grant" button opens System Settings → Privacy → Input Monitoring via the opener plugin.
- D-04: Extend the existing 3s polling loop (currently Accessibility-only) to also check Input Monitoring.
- D-06: If Input Monitoring is denied, warn in UI but do not block the engine. Mouse click macros still work. Only global hotkey activation is affected.

**TCC identity change (COMPAT-05)**
- D-07: Persist a `was-ever-granted` flag to `app_data_dir()` the first time Accessibility is successfully granted.
- D-08: On startup, if any permission is denied AND the flag exists → treat as likely TCC identity change.
- D-09: When identity-change detected, replace generic "not granted" copy with specific message. Same "Grant" button, same flow — only copy changes.
- D-10: Unified flag: covers both Accessibility and Input Monitoring. One flag file, one detection path.

**Auto-save error UI (ERR-01)**
- D-11: Add `listen('auto-save-error', ...)` in App.tsx with `onCleanup` teardown.
- D-12: Persistent inline banner at top of macro list; stays until dismissed.
- D-13: User-friendly copy only; do not surface raw Rust error string.
- D-14: Dismiss-and-clear: dismiss button removes banner; re-appears if another event fires.

**CI hardening**
- D-15 (CI-03): Remove updater signature upload step from `release.yml` entirely. Add comment explaining ad-hoc signing context.
- D-16 (CI-04): Replace `npm install` with `npm ci`.
- D-17 (CI-05): Use `${{ steps.tauri.outputs.artifactPaths }}` for artifact discovery.

**Platform cleanup (clear-cut)**
- BUILD-01: Remove unused Win32 imports in `platform/windows/mod.rs`: `GetWindowTextW`, `IsWindowVisible`, `HMODULE`, `HHOOK`. Handle `TranslateMessage` unused-bool warning.
- MEM-01: Add `CloseHandle(handle)` after each `OpenProcess` call in `list_running_apps_impl` (Windows platform).
- SAFE-04: In `flush_held_inputs`, collect held inputs while holding REGISTRY lock, then release lock, then post CGEvents.

### Claude's Discretion

- Whether Input Monitoring check applies only on macOS 26+ or all supported macOS versions — **researcher resolves below (see D-05 Resolution)**.
- Exact copy for "AutoMux was updated" identity-change message.
- Exact copy for the auto-save error banner.
- Whether `check_accessibility_permissions` is extended to also return Input Monitoring status, or a separate `check_input_monitoring` backend function is added.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BUILD-01 | Zero warnings for `x86_64-pc-windows-msvc` build — remove unused Win32 imports | Unused symbols identified in code audit; fix is import removal + `#[allow(unused_must_use)]` or `let _ =` for TranslateMessage |
| MEM-01 | Every `OpenProcess` handle closed before `list_running_apps_impl` returns | `CloseHandle` must be imported from `windows::Win32::Foundation` and called after `QueryFullProcessImageNameW` |
| CI-03 | Remove misleading updater signature step from `release.yml` | Step identified; no `.sig` file exists with ad-hoc signing |
| CI-04 | Replace `npm install` with `npm ci` in release workflow | Single line change in `release.yml` |
| CI-05 | Artifact discovery uses `${{ steps.tauri.outputs.artifactPaths }}` | Confirmed as the correct output variable name from tauri-action's `action.yml` |
| SAFE-04 | `flush_held_inputs` releases REGISTRY lock before posting CGEvents | Current code in observer.rs is ALREADY CORRECT — see note below |
| ERR-01 | `auto-save-error` Tauri event shown as visible UI notification | Backend already emits event; only App.tsx listener + banner UI needed |
| COMPAT-04 | Input Monitoring not granted → actionable guidance in UI | Probe pattern with CGEventTap ListenOnly; `x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent` deep link |
| COMPAT-05 | TCC identity change detected, user prompted to re-add Accessibility | `was-ever-granted` flag file in `app_data_dir()`; flag read at startup before permission check |
</phase_requirements>

---

## Summary

Phase 7 is a cleanup and hardening phase across four parallel tracks: Windows platform (BUILD-01, MEM-01), CI (CI-03, CI-04, CI-05), safety/error surface (SAFE-04, ERR-01), and macOS 26 permission follow-ups (COMPAT-04, COMPAT-05). No new architectural patterns are introduced — every fix reuses existing patterns established in earlier phases.

**Critical codebase discovery — SAFE-04 is already done:** Reading `flush_held_inputs()` in `observer.rs` (lines 80–138) shows the lock-release-before-dispatch pattern is already correctly implemented in the current code. The same comment block (`// @safety-officer: CR-01`) confirms the lock is released before CGEvent dispatch. The planner must verify the current code matches the SAFE-04 requirement and document it as "already correct" rather than scheduling a code change.

**D-05 Resolution (Claude's Discretion):** Input Monitoring (`kTCCServiceListenEvent`) has been required since macOS 10.15 Catalina. However, **if Accessibility permission is already granted, Input Monitoring is implicitly granted** — the permissions are not additive, Accessibility subsumes Input Monitoring. Since AutoMux requires and requests Accessibility, Input Monitoring denial in practice only occurs when: (a) the user somehow has Accessibility revoked but not Input Monitoring, or (b) the app is reinstalled/upgraded and TCC identity changes (the COMPAT-05 scenario). **Recommendation:** Check Input Monitoring on all supported macOS versions (12+) but make the UI copy reflect that Input Monitoring is only independently relevant when Accessibility is missing.

**Primary recommendation:** Plan all nine requirements as separate, independently-completable tasks. No task has prerequisites within this phase.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Win32 import cleanup (BUILD-01) | Backend — platform/windows | — | Pure Rust import-level change |
| HANDLE leak fix (MEM-01) | Backend — platform/windows | — | OS resource management in Rust |
| CI workflow fix (CI-03/04/05) | CI/CD — .github/workflows | — | YAML-only, no code changes |
| Emergency-stop lock ordering (SAFE-04) | Backend — platform/macos/observer | — | Already correct in current code |
| Auto-save error UI (ERR-01) | Frontend — App.tsx | — | Backend already emits event; UI only |
| Input Monitoring detection (COMPAT-04) | Backend — platform/macos/mod.rs | Frontend — App.tsx | Backend probe → IPC → frontend UI |
| TCC identity flag (COMPAT-05) | Backend — persistence / ipc/startup | Frontend — App.tsx | Flag written on first grant, read on startup |

---

## Standard Stack

### Core (no new dependencies needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `windows` crate | 0.61 (existing) | Win32 CloseHandle for MEM-01 | Already in Cargo.toml; CloseHandle is in `windows::Win32::Foundation` |
| `core-graphics` | 0.24.0 (existing) | CGEventTap ListenOnly probe for COMPAT-04 | Same API used in existing check_accessibility_permissions |
| Tauri path API | 2 (existing) | `app_data_dir()` for was-ever-granted flag | Already used in persistence.rs ProfileManager |
| tauri-plugin-opener | 2 (existing) | Open Input Monitoring System Settings URL | Already used for Accessibility "Grant" button |
| SolidJS `listen` | — (existing) | `auto-save-error` Tauri event subscription | Already used for `state-changed` event |

**No new packages required.** Every fix uses libraries already in the dependency tree.

### Package Legitimacy Audit

No new packages are introduced in this phase. All changes use the existing dependency set.

| Package | Registry | Verdict | Disposition |
|---------|----------|---------|-------------|
| (no new packages) | — | — | N/A — existing deps only |

---

## Architecture Patterns

### System Architecture Diagram

```
[App Startup]
    │
    ├─► check_accessibility() ──────────────────────► probe CGEventTap (defaultTap)
    │   + check_input_monitoring() [NEW] ────────────► probe CGEventTap (listenOnly)
    │   + read was_ever_granted flag [NEW] ──────────► ~/.../com.alvaro.automux/was_granted
    │
    ├─► Permissions Section (App.tsx) [UPDATED]
    │   ├─ Accessibility row: Granted / Denied / Pending
    │   ├─ Input Monitoring row: Granted / Denied / Pending (warning-only if denied)
    │   └─ TCC identity change copy if was_granted && denied
    │
    ├─► 3s poll loop ──────────────────────────────► check_accessibility + check_input_monitoring
    │
    └─► auto-save-error event ──────────────────────► persistent banner at top of macro list
                                                        (dismiss button; reappears on new events)

[CI: release.yml]
    │
    ├─► npm ci [WAS: npm install]
    ├─► tauri-action → ${{ steps.tauri.outputs.artifactPaths }}
    └─► (updater sig step REMOVED)

[Windows build]
    ├─► platform/windows/mod.rs: remove unused imports (BUILD-01)
    └─► list_running_apps_impl: CloseHandle(handle) after QueryFullProcessImageNameW (MEM-01)
```

### Recommended Project Structure

No structural changes. All modifications are in-place edits to existing files:

```
src/
└── App.tsx               ← COMPAT-04 permissions UI, ERR-01 banner, COMPAT-05 copy

src-tauri/src/
├── ipc/mod.rs            ← COMPAT-04: add check_input_monitoring command (or extend check_accessibility)
├── platform/
│   ├── macos/
│   │   └── mod.rs        ← COMPAT-04: add check_input_monitoring() probe function
│   └── windows/
│       └── mod.rs        ← BUILD-01: remove imports; MEM-01: add CloseHandle
└── lib.rs                ← COMPAT-05: read was_ever_granted flag on startup; register new IPC if added

.github/workflows/
└── release.yml           ← CI-03: remove sig step; CI-04: npm ci; CI-05: artifactPaths
```

---

## SAFE-04 Status — Already Correct

**Critical finding:** The `flush_held_inputs()` function in `src-tauri/src/platform/macos/observer.rs` lines 80–138 already implements the required lock-release-before-CGEvent-dispatch pattern:

```rust
// observer.rs:80-138 (existing correct implementation)
pub fn flush_held_inputs() {
    // @safety-officer: CR-01 — drain registry into a local Vec BEFORE releasing
    // the lock, then post CGEvents outside the lock.
    let inputs_to_flush: Vec<ActiveInput> = {
        let mut reg = get_registry().lock().unwrap();
        reg.drain().collect()
    };
    // Lock released here — safe to post CGEvents.
    // ... CGEvent dispatch follows ...
}
```

The emergency-stop path in the CGEventTap callback (lines 352–415) also already uses this pattern. **SAFE-04 is already satisfied.** The planner should schedule a verification task rather than an implementation task.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Input Monitoring permission probe | Custom TCC database query or private API | `CGEventTap::new(ListenOnly)` probe — same as existing Accessibility probe pattern | Avoids private API dependency; probe is idiomatic and already precedented in the codebase |
| System Settings deep link | Shell script or NSWorkspace | `x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent` via tauri-plugin-opener | Already used for Accessibility grant button |
| Was-ever-granted flag storage | SQLite or Keychain | Plain file in `app_data_dir()` with tokio::fs | ProfileManager already resolves this path; same directory |
| CI artifact path tracking | Glob globs hardcoded by Tauri output subdir | `${{ steps.tauri.outputs.artifactPaths }}` | Official output variable; resilient to tauri-action renames |

---

## D-05 Resolution: Input Monitoring scope

**Recommendation: Check Input Monitoring on all macOS versions 12+ (not just 26+).**

Rationale:
- `kTCCServiceListenEvent` has been enforced since macOS 10.15 (Catalina) [LOW confidence — Apple Developer Forums]
- If Accessibility is granted, Input Monitoring is implicitly granted — the UI will not show a warning for most users who have set up AutoMux correctly
- The denial scenario primarily occurs after TCC identity change (COMPAT-05 case), which can happen on any macOS version
- Checking on all versions is more correct and requires no version-gating logic
- Runtime cost is one additional CGEventTap probe on the 3s poll — negligible

**How to check Input Monitoring in Rust:**
```rust
// Mirrors check_accessibility_permissions(false) in platform/macos/mod.rs
// Source: Apple Developer Forums (CGEventTap options and TCC service mapping)
pub fn check_input_monitoring() -> bool {
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };
    // ListenOnly taps require kTCCServiceListenEvent (Input Monitoring).
    // Probe with a passive tail-append tap — same approach as accessibility probe.
    let probe = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::TailAppendEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::MouseMoved],
        |_, _, _| None,
    );
    probe.is_ok()
}
```

**IPC decision (Claude's Discretion):** Add a separate `check_input_monitoring` IPC command rather than extending `check_accessibility`. Reasons: (a) the return type stays `bool` in both cases, making them composable; (b) the polling effect in App.tsx reads both independently; (c) no signature change needed on the existing `check_accessibility` command.

---

## Common Pitfalls

### Pitfall 1: SAFE-04 Implementation Work That Isn't Needed
**What goes wrong:** Planner schedules code changes to `flush_held_inputs` when the fix is already in place.
**Why it happens:** The REQUIREMENTS.md says SAFE-04 is "Pending" but the code already satisfies the requirement.
**How to avoid:** Planner should open `observer.rs:80-138` and verify the drain-before-release pattern before scheduling any edits. If it matches the requirement, mark as verified, not implemented.
**Warning signs:** If a code diff shows changes to `flush_held_inputs`, that's a signal something is wrong.

### Pitfall 2: COMPAT-04 Adding Redundant Warning When Accessibility Is Granted
**What goes wrong:** Showing "Input Monitoring: Denied" when Accessibility is already granted — which is actually fine because Accessibility implies Input Monitoring.
**Why it happens:** Checking Input Monitoring independently without considering the Accessibility-implies-InputMonitoring relationship.
**How to avoid:** In App.tsx, the Input Monitoring warning should only show when BOTH Accessibility is denied AND Input Monitoring probe fails, OR when Accessibility is granted but Input Monitoring probe still fails (which shouldn't happen but would indicate a system misconfiguration). Simplest safe rule: show Input Monitoring status only when `accessibility() !== true`.
**Warning signs:** Users with Accessibility granted seeing a spurious Input Monitoring warning.

### Pitfall 3: MEM-01 — Forgetting CloseHandle on Early Return Paths
**What goes wrong:** Adding `CloseHandle` only on the success path, not on the `?` early-return path in `get_app_name_from_hwnd`.
**Why it happens:** `?` operator returns before `CloseHandle` is called.
**How to avoid:** The current `get_app_name_from_hwnd` uses `?` on `OpenProcess` so a failed open never needs CloseHandle. But after `let handle = OpenProcess(...).ok()?`, if `QueryFullProcessImageNameW` fails, the handle must still be closed. Use a guard pattern or ensure `CloseHandle` is called before each `return None`.
**Warning signs:** Cargo `--target x86_64-pc-windows-msvc` warnings about handle leaks, or Windows leak tools showing HANDLE count growth.

### Pitfall 4: BUILD-01 — HMODULE, HHOOK Are Used in `initialize_hook`
**What goes wrong:** Removing HMODULE and HHOOK blindly causes compile errors because they ARE used in the hook initialization code.
**Why it happens:** The CONTEXT.md says to remove them, but reading the actual code shows they are imported at the top-level `use` block alongside used items, inside `#[cfg(target_os = "windows")]` guards.
**How to avoid:** Read the actual import list. `HMODULE` appears at line 389 in the top-level unconditional import block. But `SetWindowsHookExW` takes `HMODULE` — confirm whether HMODULE is used in `initialize_hook`. If it is, do NOT remove it.

**Code audit result:** Line 389: `use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, WPARAM};` — `HMODULE` is in the unconditional import but `SetWindowsHookExW` passes `None` for it (see line 489: `SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_callback), None, 0)`). The `None` uses `Option<HMODULE>` which is typed by the compiler, not the import. Verify whether the `HMODULE` import produces an unused-import warning or if it's needed for type resolution in the `unsafe` context.

**Also note:** `HHOOK` — line 399 imports it, and it IS used in `UnhookWindowsHookEx(hook.unwrap())` where `hook` is `Result<HHOOK, _>`. Do not remove `HHOOK`.

**Recommendation for BUILD-01 planner:** Run `cargo build --target x86_64-pc-windows-msvc 2>&1 | grep "^warning"` to get the exact list of unused imports before editing. CONTEXT.md's list is a best-guess from code reading on macOS, not from an actual cross-compilation run. The planner should treat the compiler warning list as the source of truth.

### Pitfall 5: CI-05 tauri-action Output Syntax
**What goes wrong:** Using `${{ steps.tauri.outputs.artifact-paths }}` (kebab-case) instead of `${{ steps.tauri.outputs.artifactPaths }}` (camelCase).
**Why it happens:** GitHub Actions output variable naming is not consistent across actions.
**How to avoid:** The confirmed correct name from tauri-action's `action.yml` is `artifactPaths` (camelCase). The step `id` must be `tauri` to match `steps.tauri.outputs.*`.

### Pitfall 6: was-ever-granted Flag — Write Timing
**What goes wrong:** Writing the flag after emitting the IPC response, so a crash between grant and flag-write leaves an inconsistent state.
**Why it happens:** Async ordering is easy to get wrong.
**How to avoid:** Write the flag synchronously (or await it) before returning from the permission check that first detects a grant. Best location: in `check_accessibility()` IPC handler, after `granted` is true and `initialize_tap()` has been called.

---

## Code Examples

### COMPAT-04: Input Monitoring Probe Function
```rust
// src-tauri/src/platform/macos/mod.rs
// Mirrors check_accessibility_permissions(false) probe pattern
// [ASSUMED] — based on CGEventTap option semantics; no official Rust binding doc
pub fn check_input_monitoring() -> bool {
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };
    let probe = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::TailAppendEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::MouseMoved],
        |_, _, _| None,
    );
    probe.is_ok()
}
```

### COMPAT-04: IPC Command
```rust
// src-tauri/src/ipc/mod.rs — new command
// [ASSUMED] — based on existing check_accessibility pattern
#[command]
pub async fn check_input_monitoring() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(crate::platform::macos::check_input_monitoring())
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows/Linux: no Input Monitoring gate
        Ok(true)
    }
}
```

### COMPAT-04: System Settings deep link
```typescript
// App.tsx — Input Monitoring "Grant" button handler
// [CITED: https://github.com/bvanpeski/SystemPreferences]
async function handleRequestInputMonitoringAccess() {
    setInputMonitoringPending(true);
    try {
        await invoke("open_url", {
            url: "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        });
    } catch (e) {
        console.error("Failed to open Input Monitoring settings:", e);
    }
}
// Note: use tauri-plugin-opener's open() function:
// import { open } from "@tauri-apps/plugin-opener";
// await open("x-apple.systempreferences:...");
```

### COMPAT-05: was-ever-granted flag
```rust
// Flag file path: {app_data_dir}/tcc_granted.flag
// Write on first successful Accessibility grant — in check_accessibility IPC handler
// [ASSUMED] — based on existing ProfileManager pattern in persistence.rs
fn flag_path(app_handle: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    use tauri::Manager;
    app_handle.path().app_data_dir().ok().map(|d| d.join("tcc_granted.flag"))
}

pub fn write_tcc_granted_flag(app_handle: &tauri::AppHandle) {
    if let Some(path) = flag_path(app_handle) {
        let _ = std::fs::write(&path, b"1");
    }
}

pub fn tcc_granted_flag_exists(app_handle: &tauri::AppHandle) -> bool {
    flag_path(app_handle).map(|p| p.exists()).unwrap_or(false)
}
```

### MEM-01: CloseHandle after OpenProcess
```rust
// src-tauri/src/platform/windows/mod.rs — in get_app_name_from_hwnd
// [ASSUMED] — standard Win32 resource management pattern
use windows::Win32::Foundation::CloseHandle;  // add to imports

let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
let mut buf = [0u16; 260];
let mut len = buf.len() as u32;
let result = QueryFullProcessImageNameW(
    handle,
    PROCESS_NAME_FORMAT(0),
    windows::core::PWSTR(buf.as_mut_ptr()),
    &mut len,
);
unsafe { CloseHandle(handle); }  // Must close on both success and failure paths
result.ok()?;  // Now propagate error after handle is closed
Some(String::from_utf16_lossy(&buf[..len as usize]))
```

### ERR-01: auto-save-error listener in App.tsx
```typescript
// src/App.tsx — add inside App() component
// [ASSUMED] — follows existing listen('state-changed') pattern
const [saveError, setSaveError] = createSignal<string | null>(null);

createEffect(() => {
    const unlisten = listen<string>('auto-save-error', (_event) => {
        setSaveError("Failed to save changes — check available disk space.");
    });
    onCleanup(() => { unlisten.then((fn) => fn()); });
});
```

### CI-05: artifactPaths output variable
```yaml
# .github/workflows/release.yml
# [VERIFIED: tauri-action action.yml] — confirmed exact output variable name
- name: Build and Release Tauri App
  id: tauri         # Must set id so outputs are accessible
  uses: tauri-apps/tauri-action@84b9d35b5fc46c1e45415bdb6144030364f7ebc5
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  with:
    tagName: ${{ github.ref_name }}
    # ... other args ...

# Downstream step using the output:
# ${{ steps.tauri.outputs.artifactPaths }}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `npm install` in CI | `npm ci` | npm 5.7.0+ | Lockfile-gated; no implicit upgrades |
| AXIsProcessTrusted() for Accessibility probe | CGEventTap probe (TailAppendEventTap) | Phase 6 fix | Avoids stale-cache false-negative on macOS 15+ |
| Hardcoded glob paths for CI artifacts | `steps.tauri.outputs.artifactPaths` | tauri-action v0 | Resilient to tauri-action internal path changes |

**Notable:** SAFE-04 (lock-before-CGEvent-dispatch) was documented as "pending" in requirements but was already implemented during Phase 6 work. This is the most important finding from code inspection.

---

## Runtime State Inventory

> This section is included because phase touches startup behavior (COMPAT-05 flag read) and persistent state (flag file write).

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | ProfileData in `~/Library/Application Support/com.alvaro.automux/profiles/` — no rename | None |
| Live service config | No external service configs | None |
| OS-registered state | macOS TCC grants for com.alvaro.automux — persist across app reinstalls until TCC identity changes | Flag write on first Accessibility grant |
| Secrets/env vars | `GITHUB_TOKEN` in CI — unchanged | None |
| Build artifacts | Existing DMG/NSIS artifacts — CI-05 changes how they're referenced in workflow, not how they're built | None |

**New persistent artifact introduced:** `{app_data_dir}/tcc_granted.flag` — a sentinel file written when Accessibility is first granted. This file must NOT be deleted by profile operations or app cleanup. It should survive upgrades.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (inline `#[cfg(test)]` modules) |
| Config file | None — standard cargo |
| Quick run command | `cargo test -p automux-lib 2>&1` |
| Full suite command | `cargo test -p automux-lib 2>&1` |

No frontend test framework is installed.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BUILD-01 | Zero compiler warnings for Windows target | Build check | `cargo build --target x86_64-pc-windows-msvc 2>&1 \| grep "^warning"` | N/A — compiler output |
| MEM-01 | CloseHandle called after every OpenProcess | Code review | Manual — no Windows runtime on macOS dev machine | N/A |
| CI-03 | Sig upload step absent from release.yml | Manual review | `grep -c "Signature not found\|sig.*upload" .github/workflows/release.yml` | ❌ no CI test |
| CI-04 | `npm ci` present in release.yml | Manual review | `grep "npm ci" .github/workflows/release.yml` | ❌ no CI test |
| CI-05 | artifactPaths used in release.yml | Manual review | `grep "artifactPaths" .github/workflows/release.yml` | ❌ no CI test |
| SAFE-04 | flush_held_inputs lock released before CGEvent | Code review | `grep -A5 "inputs_to_flush" src-tauri/src/platform/macos/observer.rs` | Existing (verify) |
| ERR-01 | auto-save-error event triggers visible banner | Manual smoke | Launch app, trigger save error, observe banner | ❌ no automated test |
| COMPAT-04 | Input Monitoring denied → warning shown | Manual smoke | Revoke Input Monitoring, observe Permissions section | ❌ no automated test |
| COMPAT-05 | was_ever_granted flag triggers specific copy | Manual smoke | Delete tcc_granted.flag, revoke Accessibility, relaunch | ❌ no automated test |

### Wave 0 Gaps

- No new test files are required — the requirements are primarily build/CI/UI changes without unit-testable logic.
- The `persistence.rs` test suite (`large_config_memory_check`) does not need to be extended for this phase.
- BUILD-01 and SAFE-04 have automated verification via compiler output and grep respectively.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | BUILD-01, MEM-01, SAFE-04 | ✓ | 1.95.0 | — |
| `node` / `npm` | CI-04 reference | ✓ | v25.9.0 | — |
| Rust `windows` cross-compile target | BUILD-01 verification | ✗ on macOS dev | — | CI verifies; dev can check imports by reading code |
| macOS TCC sandbox (Input Monitoring) | COMPAT-04 runtime test | ✓ (macOS dev machine) | macOS 26 | — |
| tauri-plugin-opener | COMPAT-04 Settings URL | ✓ (existing dep) | 2.x | — |

**Missing with no fallback:** `x86_64-pc-windows-msvc` cross-compile target for BUILD-01 smoke-test locally. Mitigation: CI is the verification gate for BUILD-01.

---

## Security Domain

> COMPAT-04/05 involve TCC permission handling — security-relevant.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes — TCC permission checks | macOS TCC probe (CGEventTap); do not bypass or mock |
| V5 Input Validation | yes — was-ever-granted flag path | `app_data_dir()` via Tauri — path is platform-sandboxed |
| V6 Cryptography | no | — |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| TCC flag tampering (user deletes tcc_granted.flag) | Elevation of Privilege | Flag is advisory only — worst case is no identity-change warning shown; no security boundary crossed |
| Stale permission cache returning wrong TCC state | Spoofing | Use CGEventTap probe (not AXIsProcessTrusted) — already precedented in codebase |
| Displaying raw Rust error string in UI (D-13) | Information Disclosure | Use user-friendly fixed copy; never surface `e.to_string()` in the banner |

---

## Open Questions

1. **BUILD-01: Are HMODULE and HHOOK actually unused?**
   - What we know: CONTEXT.md says remove them; code audit suggests HHOOK IS used in `UnhookWindowsHookEx`; HMODULE's usage is ambiguous (passed as `None` which may or may not require the import).
   - What's unclear: Whether the Rust compiler generates a warning for HMODULE when it's used only for the type of an `Option<HMODULE>` passed as `None`.
   - Recommendation: The planner should verify by running `cargo build --target x86_64-pc-windows-msvc` in CI (or with a Windows cross-compiler) and using the actual compiler warning list, not CONTEXT.md's list.

2. **COMPAT-04: Opener plugin API for URLs**
   - What we know: `tauri-plugin-opener` is in the dep tree; the `open` function can open URLs.
   - What's unclear: Whether `invoke("open_url", ...)` or `import { open } from "@tauri-apps/plugin-opener"` is the right frontend call pattern.
   - Recommendation: Use the frontend JS import path (`@tauri-apps/plugin-opener`) directly in App.tsx — it's the documented Tauri 2 pattern and doesn't require a custom IPC command.

3. **COMPAT-04: App restart required after granting Input Monitoring?**
   - What we know: One source notes the app must be restarted after granting Input Monitoring, as macOS TCC changes don't take effect in a running process.
   - What's unclear: Whether this applies when detecting via CGEventTap probe (not AXIsProcessTrusted), and whether the existing 3s poll pattern can detect the grant without restart.
   - Recommendation: The existing Accessibility poll pattern on Phase 5 already handles dynamic grants without restart (using CGEventTap probe). The same probe for Input Monitoring should behave identically.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | CGEventTap with ListenOnly option probes Input Monitoring specifically (not Accessibility) | D-05 Resolution, Code Examples | If wrong: the probe always returns true (or false) regardless of Input Monitoring, making COMPAT-04 backend detection unreliable |
| A2 | `x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent` opens Input Monitoring pane | Code Examples | If wrong: URL opens wrong pane or fails silently |
| A3 | HMODULE is not used in the Windows import block (safe to remove) | Pitfall 4, BUILD-01 | If wrong: compile error when HMODULE is removed |
| A4 | HHOOK IS used in `UnhookWindowsHookEx` (do NOT remove) | Pitfall 4, BUILD-01 | If wrong: unnecessary `allow(dead_code)` or redundant keep |
| A5 | Input Monitoring grant detection works without app restart when using CGEventTap probe | Open Questions | If wrong: COMPAT-04 polling flow requires user to restart app to see granted state |

---

## Sources

### Primary (MEDIUM confidence)
- tauri-apps/tauri-action `action.yml` — confirmed `artifactPaths` output variable name [VERIFIED: tauri-action action.yml]
- `src-tauri/src/platform/macos/observer.rs` — SAFE-04 already implemented; `flush_held_inputs` drain-before-release pattern at lines 80–138 [VERIFIED: codebase inspection]
- `src-tauri/src/state/mod.rs:427,499` — `auto-save-error` event already emitted by backend [VERIFIED: codebase inspection]
- `src-tauri/src/persistence.rs:70` — `app_data_dir()` path already used; reuse for tcc_granted.flag [VERIFIED: codebase inspection]

### Secondary (LOW confidence)
- Apple Developer Forums thread 122492 — Accessibility permission subsumes Input Monitoring; CGEventTap option determines which TCC service is checked [CITED: https://developer.apple.com/forums/thread/122492]
- bvanpeski/SystemPreferences GitHub — `x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent` URL [CITED: https://github.com/bvanpeski/SystemPreferences/blob/main/macos_preferencepanes-Monterey.md]
- WebSearch results — Input Monitoring introduced macOS 10.15; applies all versions through macOS 26 [ASSUMED — corroborated by multiple web sources]

### Tertiary (LOW confidence — marked ASSUMED in text)
- All code examples for new functions (check_input_monitoring, flag write/read, CloseHandle pattern) are patterns derived from existing codebase analogy [ASSUMED]

---

## Metadata

**Confidence breakdown:**
- SAFE-04 status: HIGH — code directly read and confirmed correct
- Standard Stack: HIGH — no new packages; all existing deps verified in Cargo.toml
- CI-05 artifactPaths name: MEDIUM — confirmed from tauri-action action.yml
- COMPAT-04 probe pattern: LOW — Apple Developer Forums, not official Rust docs
- COMPAT-04 Settings URL: LOW — community-maintained list
- BUILD-01 exact import list: LOW — requires cross-compilation run to confirm

**Research date:** 2026-06-17
**Valid until:** 2026-07-17 (30 days; stable platform APIs)
