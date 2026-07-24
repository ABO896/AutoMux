# Requirements: AutoMux v2.0

**Defined:** 2026-06-02
**Milestone:** v2.0 — Redesign & Platform Excellence
**Core Value:** A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.

## v2.0 Requirements

### macOS Tahoe 26 Compatibility

- [x] **COMPAT-01**: User's macros (mouse clicks, key presses) fire correctly on macOS 26 Tahoe — CGEventTap input injection works without modification by the OS
- [ ] **COMPAT-02**: Permissions detection accurately reports granted/denied status on macOS 26 Tahoe — no false "not granted" after approval, no silent API change regressions
- [x] **COMPAT-03**: App launches and runs fully on macOS 26 Tahoe without crashes, missing entitlements, or framework errors

### Parallel Macro Execution

- [ ] **EXEC-01**: Multiple macros can run simultaneously on macOS — triggering macro B while macro A is running does not block, queue, or cancel macro A *(scheduler parallel-execution mechanism ships 09-01..09-08; CR-01 hotkey double-dispatch gap closed at source level 09-10 — single HOTKEY_BINDINGS registry, redundant registry removed; two further Blocker gaps — the HoldRelease Gate 1/2/3 bypass in `handle_action`/`action_should_inject` and hotkey-rebind rollback safety — closed by plan 09-11 and re-confirmed closed by the 09-VERIFICATION.md re-verification (2026-07-22T21:30:00Z); source-level reliability is complete — on-device human verification (macOS T9.1-T9.7) is the sole pending item)*
- [ ] **EXEC-02**: Multiple macros can run simultaneously on Windows — same parallel behavior as macOS *(identical latent CR-01 defect closed symmetrically in 09-10; the 09-11 HoldRelease-bypass fix is shared code, and the Windows-specific hotkey-rebind rejection via `resolve_trigger_key_update` + a Result-carrying `SetMacroTriggerKey` oneshot is confirmed closed by 09-VERIFICATION.md (2026-07-22T21:30:00Z); real-device Windows confirmation — tests 6.1-6.3 — remains blocked_by physical-device and has never been run on-device)*

### Macro Management

- [x] **UX-08**: User can delete an existing macro — action is available directly from the macro list without entering an edit mode
- [x] **UX-09**: User can edit an existing macro's name, action type, key/button assignment, and timing configuration after creation
- [ ] **UX-10**: Action type selection unambiguously labels each option: left click, right click, hold (sustained), key press — no unlabeled or unclear choices presented to the user

### UI Redesign

- [ ] **UI-01**: macOS app uses Apple design language with liquid glass visual effects native to macOS 26 — window materials, vibrancy, and controls match the Tahoe 26 HIG
- [ ] **UI-02**: macOS UI layout is Raycast-inspired — clean, focused hierarchy, efficient use of space, keyboard-navigable
- [ ] **UI-03**: Windows app uses a modern, polished equivalent UI — matches AutoMux's visual identity without mimicking macOS-specific effects unavailable on Windows
- [ ] **UI-04**: UI redesign adds no measurable increase in memory or CPU overhead at idle compared to v1.2.0 — AutoMux stays lightweight

### macOS 26 Follow-ups *(surfaced during Phase 6)*

- [x] **COMPAT-04**: App detects when Input Monitoring (`kTCCServiceListenEvent`) is not granted on macOS 26 and shows actionable guidance — user understands why hotkeys do not fire without it
- [x] **COMPAT-05**: App detects TCC identity change (unsigned → signed upgrade) and prompts user to re-add Accessibility in System Settings — no silent grant failure after a build upgrade

### Hotkey System Reliability

- [x] **UX-11**: Binding a hotkey that is already assigned to another macro shows an error or asks the user to reassign — no silent shadowing of an existing bind *(backend: Intent::BindHotkey + bind_hotkey IPC wired; Result<(), String> error path with conflict message; UI ConflictErrorToast ships in plan 08-05 — UX-11 user-facing surface complete)*
- [x] **UX-12**: App prevents or warns when multiple enabled macros inject the same input type simultaneously (e.g., two left-click macros both active) — unpredictable timing multiplication is surfaced, not silently allowed *(backend: AppState.conflicts field recomputed by recompute_conflicts() on 9 state-mutating intent handlers; UI ConflictWarningRegion ships in plan 08-05 — UX-12 user-facing surface complete)*
- [ ] **UX-13**: Hotkey binding supports the full practical key range — not just letters (A-Z) but also number keys (0-9), function keys (F1-F12), and modifier combinations (Cmd/Ctrl/Shift/Option as modifiers on macOS; Ctrl/Alt/Shift/Win on Windows) *(data model + bit pinning complete 08-01; frontend computeModifiers ships 08-04; UI ModifierPreviewChip ships 08-05 — UX-13 frontend complete; OS-level modifier matching on macOS is a follow-up)*
- [x] **UX-14**: Macros trigger correctly when AutoMux is not the focused application — global hotkey observation is verified on both macOS (CGEventTap HID) and Windows (Win32 global hook), and the UI clearly communicates that binds are system-wide *(backend: Windows HOTKEY_BINDINGS registry + build_mod_mask helper added in plan 08-03; bind_hotkey/unbind_hotkey IPC commands no longer macOS-only; UI FirstRunGlobalNotice banner + in-card ↗ Global subtitle ships in plan 08-05 — UX-14 user-facing surfaces complete; platform verification pending 08-06)*

### Platform Cleanup *(carried from v1.2.0)*

- [x] **BUILD-01**: `cargo build --target x86_64-pc-windows-msvc` produces zero warnings — unused `GetWindowTextW`, `IsWindowVisible`, `HMODULE`, `HHOOK` imports removed; `TranslateMessage` unused-bool handled
- [x] **MEM-01**: Every `OpenProcess` handle opened in `list_running_apps_impl` is closed before the function returns — no HANDLE leak on process list refresh

### CI Hardening *(carried from v1.2.0)*

- [x] **CI-03**: Release workflow correctly uploads updater JSON signature artifact — no more silent "Signature not found" skip in CI output
- [x] **CI-04**: CI uses `npm ci` instead of `npm install` for reproducible, lockfile-gated dependency installs
- [x] **CI-05**: Binary artifact discovery pattern is resilient to tauri-action output path renames — no silent miss when artifact location changes

### Safety & Error Surface *(carried from v1.2.0)*

- [x] **SAFE-04**: `flush_held_inputs` releases the REGISTRY lock before posting CGEvents — eliminates the potential deadlock on macOS emergency stop
- [x] **ERR-01**: Persistence failures surface to the user in the UI — App.tsx listens to the `auto-save-error` Tauri event and shows a visible error notification when auto-save fails

## Future Requirements

### Distribution (v3)

- **DIST-01**: Auto-updater — deferred; code signing and entitlements required as prerequisite
- **DIST-02**: macOS notarization — deferred; signing identity is a one-way door

### UX (future)

- **UX-04**: NamedKey schema migration for cross-platform profile portability — deferred
- **UX-05**: Cross-platform profile portability — depends on UX-04

## Out of Scope

| Feature | Reason |
|---------|--------|
| Auto-updater | Requires code signing; defer until signing/entitlements finalized |
| macOS notarization | Signing identity config is a one-way door; defer until process is ready |
| Cross-platform profiles | Requires NamedKey schema migration (UX-04) first |
| A11Y-01 ARIA attributes | Being addressed as part of UI-01/02/03 redesign — not tracked separately |
| Mobile / web app | Desktop automation tool by design |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| COMPAT-01 | Phase 6 | Complete |
| COMPAT-02 | Phase 6 | Pending |
| COMPAT-03 | Phase 6 | Complete |
| COMPAT-04 | Phase 7 | Complete |
| COMPAT-05 | Phase 7 | Complete |
| UX-11 | Phase 8 | Complete (UI toast ships 08-05) |
| UX-12 | Phase 8 | Complete (UI surfaces ship 08-05) |
| UX-13 | Phase 8 | In Progress (data model + bit pinning complete 08-01; frontend computeModifiers + ModifierPreviewChip ship 08-04/05; OS-level modifier matching on macOS is a follow-up) |
| UX-14 | Phase 8 | Complete (Windows HOTKEY_BINDINGS + IPC routing complete 08-03; UI first-run banner + ↗ Global subtitle ship 08-05; 08-VERIFICATION.md gate artifact + manual device test plans ship 08-06; Windows cross-compile gate deferred to CI per plan) |
| BUILD-01 | Phase 7 | Complete |
| MEM-01 | Phase 7 | Complete |
| CI-03 | Phase 7 | Complete |
| CI-04 | Phase 7 | Complete |
| CI-05 | Phase 7 | Complete |
| SAFE-04 | Phase 7 | Complete |
| ERR-01 | Phase 7 | Complete |
| EXEC-01 | Phase 9 | Complete (source-level) (parallel scheduler shipped 09-01..09-08; CR-01 hotkey double-dispatch gap closed at source level 09-10; 09-11 gap-closure — HoldRelease bypass + hotkey-rebind rollback safety — confirmed by 09-VERIFICATION.md 2026-07-22T21:30:00Z; on-device human verification T9.1-T9.7 macOS still pending) |
| EXEC-02 | Phase 9 | Complete (source-level) (identical CR-01 defect closed symmetrically 09-10; 09-11 gap-closure — HoldRelease bypass + hotkey-rebind rollback safety — confirmed by 09-VERIFICATION.md 2026-07-22T21:30:00Z; Windows physical-device tests 6.1-6.3 remain pending, never yet run on-device) |
| UX-08 | Phase 10 | Complete |
| UX-09 | Phase 10 | Complete |
| UX-10 | Phase 10 | Pending |
| UI-01 | Phase 10 | Pending |
| UI-02 | Phase 10 | Pending |
| UI-03 | Phase 10 | Pending |
| UI-04 | Phase 10 | Pending |

**Coverage:**

- v2.0 requirements: 25 total
- Mapped to phases: 25
- Unmapped: 0

---
*Requirements defined: 2026-06-02*
*Last updated: 2026-06-30 — UX-11/UX-12/UX-14 user-facing surfaces complete (plan 08-05: ConflictErrorToast, ConflictWarningRegion, FirstRunGlobalNotice, ↗ Global subtitle, ModifierPreviewChip); UX-13 frontend complete; UX-14 platform verification complete (plan 08-06: 08-VERIFICATION.md gate artifact + manual device test plans; 4/4 automated gates green, 1 gate deferred to CI per plan)*

*Updated 2026-07-22 — EXEC-01/EXEC-02 complete at the source level (plan 09-10: CR-01 dual hotkey-registry double-dispatch gap closed symmetrically on macOS and Windows, consolidating to the single HOTKEY_BINDINGS registry); on-device human verification (T9.8 macOS, Windows tests 6.1-6.3) remains pending, unchanged in disposition from 09-VERIFICATION.md*

*Updated 2026-07-23 — EXEC-01/EXEC-02 annotations reconciled with the 09-VERIFICATION.md re-verification (2026-07-22T21:30:00Z): the two Blocker gaps found after 09-10 (HoldRelease Gate 1/2/3 bypass in handle_action/action_should_inject; hotkey-rebind rollback safety — macOS pre-unbind dropped, Windows resolve_trigger_key_update + Result-carrying SetMacroTriggerKey oneshot) are closed by plan 09-11 and independently re-confirmed against source with 16/16 backend tests passing; checkboxes remain unchecked pending the sole outstanding item, on-device human verification (macOS T9.1-T9.7, Windows 6.1-6.3, the Windows tests never yet run on a real device)*
