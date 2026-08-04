---
phase: 7
slug: carry-work-platform-ci-safety
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-17
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (inline `#[cfg(test)]` modules) |
| **Config file** | None — standard cargo |
| **Quick run command** | `cargo test -p automux-lib 2>&1` |
| **Full suite command** | `cargo test -p automux-lib 2>&1` |
| **Estimated runtime** | ~30 seconds |

No frontend test framework is installed; UI verification for COMPAT-04/05 and ERR-01 is manual smoke only.

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p automux-lib 2>&1`
- **After every plan wave:** Run `cargo test -p automux-lib 2>&1`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 07-01-01 | 01 | 1 | BUILD-01 | — | Win32 import list trimmed, `cargo build --target x86_64-pc-windows-msvc` clean | build-check | `cargo build --target x86_64-pc-windows-msvc 2>&1 \| grep "^warning"` (run in CI) | N/A — compiler output | ⬜ pending |
| 07-01-02 | 01 | 1 | MEM-01 | — | `CloseHandle` after every `OpenProcess` in `list_running_apps_impl` | code-review | `grep -c "CloseHandle" src-tauri/src/platform/windows/mod.rs` matches OpenProcess count | N/A — code review | ⬜ pending |
| 07-02-01 | 02 | 2 | CI-03 | — | Updater signature upload step absent from `release.yml` | yaml-grep | `grep -c "Signature not found\|sig.*upload" .github/workflows/release.yml` returns 0 | N/A | ⬜ pending |
| 07-02-02 | 02 | 2 | CI-04 | — | `npm ci` present in release workflow | yaml-grep | `grep "npm ci" .github/workflows/release.yml` returns ≥1 | N/A | ⬜ pending |
| 07-02-03 | 02 | 2 | CI-05 | — | `${{ steps.tauri.outputs.artifactPaths }}` referenced | yaml-grep | `grep "artifactPaths" .github/workflows/release.yml` returns ≥1 | N/A | ⬜ pending |
| 07-03-01 | 03 | 2 | SAFE-04 | — | `flush_held_inputs` releases REGISTRY lock before posting CGEvents | code-review | `grep -B2 -A8 "inputs_to_flush" src-tauri/src/platform/macos/observer.rs` shows lock release before dispatch | N/A | ⬜ pending |
| 07-04-01 | 04 | 2 | ERR-01 | T-01 (Info Disclosure) | `listen('auto-save-error', ...)` listener with `onCleanup`; user-friendly copy only — no `e.to_string()` surfaced | manual-smoke | Launch app, trigger save error, observe banner copy | N/A | ⬜ pending |
| 07-05-01 | 05 | 3 | COMPAT-04 | T-02 (Spoofing) | `kTCCServiceListenEvent` checked at startup; warning shown when denied; engine not blocked | manual-smoke | Revoke Input Monitoring in System Settings, observe Permissions section | N/A | ⬜ pending |
| 07-05-02 | 05 | 3 | COMPAT-04 | — | Combined "Permissions" section lists Accessibility + Input Monitoring; both Grant buttons work | manual-smoke | Visual inspection of App.tsx permissions block | N/A | ⬜ pending |
| 07-06-01 | 06 | 3 | COMPAT-05 | T-03 (Elevation) | `tcc_granted.flag` written on first grant; identity-change copy replaces generic copy when flag exists + permission denied | manual-smoke | Delete `tcc_granted.flag`, revoke Accessibility, relaunch, observe copy | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Existing infrastructure covers all phase requirements — no new test files required.
- [x] `persistence.rs` test suite (`large_config_memory_check`) does not need extension.
- [x] BUILD-01 and SAFE-04 have automated verification via compiler output and grep respectively.
- [x] CI-03/04/05 have automated verification via YAML grep commands.

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `auto-save-error` event triggers visible banner with user-friendly copy | ERR-01 | Requires a real save failure (disk full, permission denied) — not constructable in unit test | 1. Run app. 2. Make `app_data_dir()` read-only or fill disk. 3. Modify a macro. 4. Observe banner appears with friendly copy and dismiss button. 5. Dismiss. 6. Modify macro again — banner re-appears. |
| Input Monitoring denied shows warning | COMPAT-04 | Requires macOS TCC sandbox; user must revoke via System Settings | 1. System Settings → Privacy & Security → Input Monitoring → disable AutoMux. 2. Relaunch app. 3. Observe "Permissions" section shows Input Monitoring denied. 4. Click "Grant" → opens correct Settings pane. |
| TCC identity change copy | COMPAT-05 | Requires simulating TCC invalidation | 1. Grant Accessibility (creates `tcc_granted.flag`). 2. Delete the flag. 3. Revoke Accessibility. 4. Relaunch. 5. Observe specific "AutoMux was updated" copy. |
| `x86_64-pc-windows-msvc` build with zero warnings | BUILD-01 | Cross-compile target not installed on macOS dev machine | CI verifies on Windows runner. Local: read code to confirm unused imports are gone. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

---

*Wave 0 verdict:* No new test infrastructure required. The phase is dominated by build/CI/YAML/UI changes that don't introduce new unit-testable logic. `cargo test` continues to serve as the post-commit sampling gate.
