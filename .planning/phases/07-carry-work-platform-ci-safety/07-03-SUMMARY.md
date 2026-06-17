---
phase: 07-carry-work-platform-ci-safety
plan: 03
subsystem: frontend
tags: [solidjs, tauri, typescript, permissions, accessibility, input-monitoring, tcc, auto-save, error-banner]

# Dependency graph
requires:
  - phase: 07-carry-work-platform-ci-safety
    plan: 02
    provides: "ipc::check_input_monitoring + ipc::get_tcc_identity_status backend commands; write_tcc_granted_flag in check_accessibility"
provides:
  - "Combined PermissionsCard UI: Accessibility + Input Monitoring rows with TCC identity-change copy swap (D-02, D-09)"
  - "Input Monitoring Grant button opens x-apple.systempreferences:...Privacy_ListenEvent via openUrl() (D-03)"
  - "3-second polling loop extended to call check_accessibility + check_input_monitoring via Promise.all (D-04)"
  - "listen('auto-save-error', ...) subscription + AutoSaveErrorBanner with dismissable fixed copy (ERR-01, D-11, D-12, D-13, D-14)"
affects:
  - phase: 10-ui-redesign-macro-management
    context: "PermissionsCard and banner establish the combined-row + inline-banner patterns that the redesign will retain"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SolidJS <Show when={...} fallback={...}> tri-state pattern (Granted/Pending/Denied-or-Warning) for permission status dots"
    - "Reuse of shared requestAccessCancelled flag for Input Monitoring 30s pending timeout (flows never run concurrently)"
    - "3s setInterval Promise.all pattern for paired permission polls"
    - "Fixed-copy auto-save error banner: payload is consumed but never displayed (D-13 Information Disclosure mitigation)"

key-files:
  created: []
  modified:
    - src/App.tsx

key-decisions:
  - "Used openUrl() from @tauri-apps/plugin-opener instead of open() (plan referenced open) — the actual API in v2.5.4 is openUrl. Plan bug auto-fixed per deviation Rule 1."
  - "Reused the existing requestAccessCancelled flag for the IM 30s pending timeout (no new cancellation ref). The two grant flows never run concurrently from the user's perspective — they share the same teardown path."
  - "Input Monitoring row status label is 'Warning' (not 'Denied') per UI-SPEC D-06 — mouse macros still work, only global hotkey activation is affected. Status dot stays danger-colored; only the verb softens."

patterns-established:
  - "Pattern: Combined-permission card with shared header — single .glass-card flex-1 with stacked <flex flex-col gap-3> rows; each row has its own status dot+label+conditional Grant button."
  - "Pattern: TCC identity-change copy swap via <Show when={tccChanged && !granted} fallback={defaultCopy}> — minimal JSX, no effect needed; signal-driven only."

requirements-completed: [COMPAT-04, COMPAT-05, ERR-01]

# Metrics
duration: 5min
completed: 2026-06-17
---

# Phase 7 Plan 3: Frontend (Permissions Card + Auto-Save Banner) Summary

**Combined PermissionsCard (Accessibility + Input Monitoring with TCC identity-change copy) + persistent AutoSaveErrorBanner, wired against the 07-02 backend surface.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-17T22:03:24Z
- **Completed:** 2026-06-17T22:08:19Z
- **Tasks:** 2
- **Files modified:** 1 (`src/App.tsx` — 1045 → 1198 lines, +191 / −38 net)

## Accomplishments

- **Combined PermissionsCard replaces the solo Accessibility card** (D-02). One `.glass-card flex-1` containing two stacked rows; the Engine card sits beside it in the same flex row, unchanged.
- **Input Monitoring row** renders with its own status dot, tri-state (Granted / Warning / Pending…), and a Grant Access button (D-03). Clicking Grant calls `handleRequestInputMonitoringAccess`, which opens `x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent` via `openUrl()` from `@tauri-apps/plugin-opener` — NOT a custom `invoke()`.
- **3-second polling loop** (D-04) now calls `check_accessibility` and `check_input_monitoring` in parallel via `Promise.all` and clears both pending states (`clearPending()` + `clearImPending()`) on each tick. Initial fetch extended the same way, plus fetches `get_tcc_identity_status` and sets `tccIdentityChanged` signal.
- **TCC identity-change copy swap** (D-09): when `tccIdentityChanged() === true && accessibility() === false`, the Accessibility row's sublabel swaps from `"Required for input injection"` to `"AutoMux was updated — Accessibility needs to be re-added in System Settings."` — using `<Show when={...} fallback={...}>` for a signal-driven, effect-free render.
- **Auto-save error banner** (ERR-01): `listen<string>('auto-save-error', ...)` wrapped in `createEffect` with `onCleanup` teardown (mirroring the existing `state-changed` listener shape). The banner sits directly above the "Macros" section header, shows fixed copy (`"Save failed"` + `"Your changes are not being saved. Check available disk space and file permissions."`), and has a text-only `Dismiss` button that sets `saveError(false)`. The event payload is consumed but never displayed (D-13). A subsequent event re-shows the banner (D-14, no permanent suppression).
- **Plan bug auto-fixed**: the plan referenced `import { open } from "@tauri-apps/plugin-opener"` but the actual API in v2.5.4 is `openUrl`. Used `openUrl` to match the installed package version. Documented in commit message and below.

## Task Commits

Each task was committed atomically:

1. **Task 1: Replace solo Accessibility card with combined PermissionsCard** — `6b9ab2c` (feat)
2. **Task 2: Add auto-save-error listener + AutoSaveErrorBanner UI** — `171d9d9` (feat)

## Files Created/Modified

- `src/App.tsx` (+191 / −38 net, 1045 → 1198 lines) — added 4 new signals (`inputMonitoring`, `inputMonitoringPending`, `saveError`, `tccIdentityChanged`), 1 new timeout ref (`_imPendingTimeoutId`), 2 new helpers (`clearImPending`, `handleRequestInputMonitoringAccess`), 1 new listener (`auto-save-error`), and replaced the solo Accessibility card with a combined PermissionsCard containing two stacked rows. Added the `openUrl` import from `@tauri-apps/plugin-opener`.

## Decisions Made

- **Used `openUrl()` instead of `open()`**: plan referenced `open` (auto-fixed per deviation Rule 1). The actual API in `@tauri-apps/plugin-opener` 2.5.4 is `openUrl(url, openWith?)` — verified via `dist-js/index.d.ts` exports.
- **Reused `requestAccessCancelled` flag** for the IM pending timeout: the Accessibility and IM grant flows never run concurrently (user clicks one or the other, not both), so they share the same teardown semantics. No new cancellation ref needed.
- **Status label "Warning" on Input Monitoring denial** (per UI-SPEC D-06): the status dot is danger-colored but the label verb softens because mouse macros still work — only global hotkey activation is affected. Communicates "this is degraded, not broken."
- **Banner sits between the active-profile badge and the Macros header**: not at the very top of the page, but at the top of the macro list area (per UI-SPEC §Component Inventory > New: `AutoSaveErrorBanner`).
- **No `IS_MACOS` gate on the Input Monitoring button**: the URL is harmless on non-macOS and the polling returns `true` for IM there anyway. Conditional hiding would add complexity for no behavioral benefit (the row's state will simply always show Granted on non-macOS).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Used `openUrl` instead of `open` from `@tauri-apps/plugin-opener`**
- **Found during:** Task 1 (Change 1: add opener import)
- **Issue:** The plan repeatedly references `import { open } from "@tauri-apps/plugin-opener"` and `await open("x-apple.systempreferences:...")`. The actual API in the installed version (2.5.4) is `openUrl(url, openWith?)` — verified via `node_modules/@tauri-apps/plugin-opener/dist-js/index.d.ts`. Using `open` as written would fail to compile with `Module '"@tauri-apps/plugin-opener"' has no exported member 'open'`.
- **Fix:** Used `openUrl` instead. The plan's intent (open a URL via the opener plugin) is preserved; the function name was the only thing wrong.
- **Files modified:** `src/App.tsx`
- **Verification:** `npx tsc --noEmit` exits 0; runtime call `await openUrl("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")` matches the package's documented signature.
- **Committed in:** `6b9ab2c` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 plan-bug, 0 missing-critical, 0 blocking)
**Impact on plan:** Minimal — one identifier name change. The plan's behavioral requirements (open the Input Monitoring System Settings pane via the opener plugin) are fully satisfied.

## Issues Encountered

None. The plan's `<interfaces>` block gave exact insertion points and the UI-SPEC + PATTERNS docs provided verbatim copy and patterns. `npx tsc --noEmit` passes on the first run after both tasks.

## Threat Model Disposition

The plan's `threat_model` register had 4 threats. All dispositions were applied during execution:

| Threat | Disposition | Where Applied |
|--------|-------------|---------------|
| T-07-08 Information Disclosure (raw Rust error string in UI) | mitigate | D-13 honored: listener body is `(_event) => { setSaveError(true); }` — the `_event` parameter is prefixed with `_` and never read; banner shows fixed copy only. `grep _event.payload` returns 0 matches. |
| T-07-09 Tampering (TCC identity-change copy driven by file existence) | accept | `tccIdentityChanged` signal is set once on initial fetch; tampering with the flag (e.g., user deletes `tcc_granted.flag`) only suppresses the "AutoMux was updated" copy — no security boundary crossed. |
| T-07-10 Denial of Service (3s polling overhead) | accept | Two boolean invocations + one IPC poll every 3s; `Promise.all` parallelizes. Negligible CPU/network. |
| T-07-SC Tampering (npm/cargo installs) | accept | No new packages installed; `@tauri-apps/plugin-opener` was already a dependency. |

## Verification

All 12 plan-level verification gates pass:

| # | Gate | Result |
|---|------|--------|
| 1 | `grep check_input_monitoring src/App.tsx` ≥ 1 | 2 matches (initial fetch + polling) |
| 2 | `grep get_tcc_identity_status src/App.tsx` ≥ 1 | 1 match (initial fetch) |
| 3 | `grep handleRequestInputMonitoringAccess src/App.tsx` ≥ 2 | 2 matches (declaration + onClick) |
| 4 | `grep tccIdentityChanged src/App.tsx` ≥ 2 | 2 matches (declaration + sublabel swap) |
| 5 | `grep @tauri-apps/plugin-opener src/App.tsx` ≥ 1 | 1 match (the import) |
| 6 | `grep "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent" src/App.tsx` = 1 | 1 match |
| 7 | `grep "AutoMux was updated" src/App.tsx` = 1 | 1 match |
| 8 | `grep auto-save-error src/App.tsx` ≥ 2 | 2 matches (listener + banner id) |
| 9 | `grep "Save failed" src/App.tsx` = 1 | 1 match in banner (existing `Save failed: ${e}` in profile toast is on a different line and not a banner heading) |
| 10 | `grep auto-save-error-banner src/App.tsx` = 1 | 1 match |
| 11 | `_event.payload` absent | 0 matches (D-13 honored) |
| 12 | `npx tsc --noEmit` exits 0 | exit 0, no errors |

**Engine card preserved** — the `<div class="glass-card flex-1 p-4">` with "Engine" header is at lines 656–676, unchanged.
**Old solo Accessibility card removed** — `grep "Accessibility Permissions"` returns 0 matches; the old `<p class="text-xs text-text-dim">Accessibility Permissions</p>` literal no longer exists.

## User Setup Required

None — no external service configuration required. All changes are in-process frontend code that consumes IPC commands already registered in plan 07-02.

## Next Phase Readiness

Plan 07-03 closes out **Phase 7 (Carry Work — Platform, CI & Safety)**. All nine requirements are now complete:

- ✅ BUILD-01, MEM-01 (Windows platform cleanup) — Plan 07-01
- ✅ CI-03, CI-04, CI-05 (CI hardening) — Plan 07-01
- ✅ SAFE-04 (lock ordering, verified already correct) — Plan 07-01
- ✅ COMPAT-04, COMPAT-05 (macOS 26 permission backend + frontend) — Plans 07-02 and 07-03
- ✅ ERR-01 (auto-save error UI) — Plan 07-03

**Phase 7 → Phase 8 (Hotkey Reliability & Conflict Safety) is unblocked.** The PermissionsCard pattern established here (combined-row card with tri-state status dots + conditional Grant button) is a reusable primitive for any future permission or capability detection UX.

---

*Phase: 07-carry-work-platform-ci-safety*
*Completed: 2026-06-17*
