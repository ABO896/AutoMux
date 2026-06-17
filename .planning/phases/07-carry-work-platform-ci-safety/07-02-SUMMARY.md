---
phase: 07-carry-work-platform-ci-safety
plan: 02
subsystem: platform
tags: [macos, tcc, permissions, cgeventtap, input-monitoring, accessibility, sentinel-file]

# Dependency graph
requires:
  - phase: 06-macos-tahoe-26-compatibility
    provides: check_accessibility_permissions probe pattern; existing ipc::check_accessibility dual-stub; observer TAP_INITIALIZED guard
provides:
  - check_input_monitoring() CGEventTap ListenOnly probe (COMPAT-04 backend)
  - tcc_granted_flag_path / write_tcc_granted_flag / tcc_granted_flag_exists helpers (COMPAT-05 backend)
  - ipc::check_input_monitoring and ipc::get_tcc_identity_status IPC commands
  - check_accessibility now writes the TCC flag on every successful grant
affects:
  - phase: 07-carry-work-platform-ci-safety
    plan: 03
    context: Frontend wiring (App.tsx PermissionsCard) will consume check_input_monitoring and get_tcc_identity_status

# Tech tracking
tech-stack:
  added: []
  patterns:
    - CGEventTap ListenOnly probe pattern (mirrors check_accessibility_permissions false branch)
    - "Dual-stub" #[cfg(target_os)] pattern for cross-platform IPC commands returning Ok(true) on non-macOS
    - Module-level pub fn helpers in persistence.rs (free functions, not on ProfileManager)
    - Idempotent sentinel-file write on every successful permission grant
    - Tauri AppHandle parameter auto-injected by #[command] macro (no State<...> needed)

key-files:
  created: []
  modified:
    - src-tauri/src/platform/macos/mod.rs
    - src-tauri/src/ipc/mod.rs
    - src-tauri/src/persistence.rs
    - src-tauri/src/lib.rs

key-decisions:
  - "Replicated the @safety-officer comment block from check_accessibility_permissions verbatim into check_input_monitoring — the plan requires this and the discipline (no TAP_INITIALIZED mutation) is identical"
  - "TCC flag write is idempotent — written on every successful grant, not just the first — per RESEARCH §Pitfall 6 (write-timing invariant). The file's existence is the only signal; content is always b'1'"
  - "Both new IPC commands (check_input_monitoring, get_tcc_identity_status) are registered as the last two entries in invoke_handler! to keep the diff localized"

patterns-established:
  - "Pattern: Module-level pub fn helpers in persistence.rs that resolve app_data_dir() and return Option<PathBuf> for best-effort disk operations"
  - "Pattern: Silent CGEventTap probe — CGEventTap::new with ListenOnly + MouseMoved, dropped at function return, no global state mutation"
  - "Pattern: Tauri AppHandle parameter auto-injected into #[command] functions without explicit Manager import in the command body"

requirements-completed: [COMPAT-04, COMPAT-05]

# Metrics
duration: 3min
completed: 2026-06-17
---

# Phase 7 Plan 2: macOS 26 Permission Backend Summary

**CGEventTap ListenOnly probe for `kTCCServiceListenEvent` + `app_data_dir()` sentinel file for TCC identity-change detection, exposed via two new IPC commands.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-06-17T21:59:24Z
- **Completed:** 2026-06-17T22:02:08Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- **COMPAT-04 backend**: `pub fn check_input_monitoring()` in `platform/macos/mod.rs` performs a CGEventTap ListenOnly probe — the only reliable signal for `kTCCServiceListenEvent` independent of the stale `AXIsProcessTrusted()` cache. The probe is scoped to the function and does NOT touch the global `TAP_INITIALIZED` / `TAP_STARTING` guards.
- **COMPAT-05 backend**: Three module-level helpers in `persistence.rs` (`tcc_granted_flag_path`, `write_tcc_granted_flag`, `tcc_granted_flag_exists`) manage a `tcc_granted.flag` sentinel file in `app_data_dir()`. The `check_accessibility` IPC handler now writes this flag on every successful grant (idempotent: file existence is the only signal).
- **Two new IPC commands registered**: `ipc::check_input_monitoring` and `ipc::get_tcc_identity_status` are appended to `invoke_handler!` in `lib.rs`. The frontend in plan 07-03 will poll both via the existing 3-second `setInterval` pattern.
- **Zero regressions**: The full `cargo test` suite (3 tests: persistence memory check, scheduler jitter audit, AFK farm stress test) still passes; `cargo check --target aarch64-apple-darwin` finishes in <1s with no warnings.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add check_input_monitoring() probe + register IPC** - `ce8911e` (feat)
2. **Task 2: TCC flag helpers + get_tcc_identity_status + wire write into check_accessibility** - `5631138` (feat)

## Files Created/Modified

- `src-tauri/src/platform/macos/mod.rs` — Added `pub fn check_input_monitoring() -> bool` (96 lines total, +28 lines). Replicates the @safety-officer comment block from `check_accessibility_permissions` verbatim. Probe is scoped — no `TAP_INITIALIZED.store()` or `TAP_STARTING.store()` calls.
- `src-tauri/src/ipc/mod.rs` — Added `check_input_monitoring` IPC command (dual-stub: macOS probe or `Ok(true)`). Modified `check_accessibility` to take `app_handle: tauri::AppHandle` and write the TCC flag on every successful grant. Added `get_tcc_identity_status` IPC command.
- `src-tauri/src/persistence.rs` — Added `use tauri::Manager;` at module scope. Added `TCC_GRANTED_FLAG_FILENAME` const + three module-level `pub fn` helpers (`tcc_granted_flag_path`, `write_tcc_granted_flag`, `tcc_granted_flag_exists`). All reuse the same `app_data_dir()` resolution pattern as `ProfileManager::from_app_handle`.
- `src-tauri/src/lib.rs` — Appended `ipc::check_input_monitoring,` and `ipc::get_tcc_identity_status,` to `tauri::generate_handler![...]`.

## Decisions Made

None - followed plan as specified. The plan's pre-extracted `<interfaces>` block, `<read_first>` lists, and the analogous `07-PATTERNS.md` file made the edits mechanical.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. The pre-extracted `interfaces` block in the plan (lines 90-153) gave exact code templates and line references, eliminating any ambiguity. `cargo check` and `cargo test` both passed on the first run for each task.

## Threat Model Disposition

The plan's `threat_model` register had 4 threats. All dispositions were applied during execution:

| Threat | Disposition | Where Applied |
|--------|-------------|---------------|
| T-07-05 Spoofing (stale verdict) | mitigate | CGEventTap ListenOnly probe runs live TCC check (same pattern as Phase 5/6 check_accessibility fix) |
| T-07-06 Elevation (flag tampering) | accept | Flag is advisory; worst case is misleading "AutoMux was updated" subtitle |
| T-07-07 Information Disclosure (`app_data_dir` error) | accept | Errors map to `None` (Option<PathBuf>); IPC command returns `Ok(false)` on error path, never the raw error string |
| T-07-SC Tampering (npm/cargo installs) | accept | No new packages installed; existing dependencies unchanged |

## Next Phase Readiness

Plan 07-03 (UI wiring in `src/App.tsx`) can now:
- Add `check_input_monitoring` to the 3-second `setInterval` poll alongside `check_accessibility`
- Add `get_tcc_identity_status` to the initial-fetch `Promise.all` block
- Build the merged `PermissionsCard` (replacing the solo Accessibility card) with two rows: Accessibility + Input Monitoring
- Render the TCC identity-change copy in the Accessibility row when `tccIdentityChanged() === true && accessibility() === false`

No blockers. The backend surface is complete and verified.

## Self-Check: PASSED

- ✓ `.planning/phases/07-carry-work-platform-ci-safety/07-02-SUMMARY.md` exists on disk
- ✓ Commit `ce8911e` (Task 1) present in git log
- ✓ Commit `5631138` (Task 2) present in git log
- ✓ `cargo check --target aarch64-apple-darwin` exits 0 (no warnings, no errors)
- ✓ `cargo test` exits 0 (3/3 tests pass — no regressions)
- ✓ All 6 verification gates from PLAN.md §verification pass (matches 1, 2-comments-only, ≥1, 2, ≥3, ≥3)
