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

- [ ] **EXEC-01**: Multiple macros can run simultaneously on macOS — triggering macro B while macro A is running does not block, queue, or cancel macro A
- [ ] **EXEC-02**: Multiple macros can run simultaneously on Windows — same parallel behavior as macOS

### Macro Management

- [ ] **UX-08**: User can delete an existing macro — action is available directly from the macro list without entering an edit mode
- [ ] **UX-09**: User can edit an existing macro's name, action type, key/button assignment, and timing configuration after creation
- [ ] **UX-10**: Action type selection unambiguously labels each option: left click, right click, hold (sustained), key press — no unlabeled or unclear choices presented to the user

### UI Redesign

- [ ] **UI-01**: macOS app uses Apple design language with liquid glass visual effects native to macOS 26 — window materials, vibrancy, and controls match the Tahoe 26 HIG
- [ ] **UI-02**: macOS UI layout is Raycast-inspired — clean, focused hierarchy, efficient use of space, keyboard-navigable
- [ ] **UI-03**: Windows app uses a modern, polished equivalent UI — matches AutoMux's visual identity without mimicking macOS-specific effects unavailable on Windows
- [ ] **UI-04**: UI redesign adds no measurable increase in memory or CPU overhead at idle compared to v1.2.0 — AutoMux stays lightweight

### macOS 26 Follow-ups *(surfaced during Phase 6)*

- [ ] **COMPAT-04**: App detects when Input Monitoring (`kTCCServiceListenEvent`) is not granted on macOS 26 and shows actionable guidance — user understands why hotkeys do not fire without it
- [ ] **COMPAT-05**: App detects TCC identity change (unsigned → signed upgrade) and prompts user to re-add Accessibility in System Settings — no silent grant failure after a build upgrade

### Hotkey System Reliability

- [ ] **UX-11**: Binding a hotkey that is already assigned to another macro shows an error or asks the user to reassign — no silent shadowing of an existing bind
- [ ] **UX-12**: App prevents or warns when multiple enabled macros inject the same input type simultaneously (e.g., two left-click macros both active) — unpredictable timing multiplication is surfaced, not silently allowed
- [ ] **UX-13**: Hotkey binding supports the full practical key range — not just letters (A-Z) but also number keys (0-9), function keys (F1-F12), and modifier combinations (Cmd/Ctrl/Shift/Option as modifiers on macOS; Ctrl/Alt/Shift/Win on Windows)
- [ ] **UX-14**: Macros trigger correctly when AutoMux is not the focused application — global hotkey observation is verified on both macOS (CGEventTap HID) and Windows (Win32 global hook), and the UI clearly communicates that binds are system-wide

### Platform Cleanup *(carried from v1.2.0)*

- [ ] **BUILD-01**: `cargo build --target x86_64-pc-windows-msvc` produces zero warnings — unused `GetWindowTextW`, `IsWindowVisible`, `HMODULE`, `HHOOK` imports removed; `TranslateMessage` unused-bool handled
- [ ] **MEM-01**: Every `OpenProcess` handle opened in `list_running_apps_impl` is closed before the function returns — no HANDLE leak on process list refresh

### CI Hardening *(carried from v1.2.0)*

- [ ] **CI-03**: Release workflow correctly uploads updater JSON signature artifact — no more silent "Signature not found" skip in CI output
- [ ] **CI-04**: CI uses `npm ci` instead of `npm install` for reproducible, lockfile-gated dependency installs
- [ ] **CI-05**: Binary artifact discovery pattern is resilient to tauri-action output path renames — no silent miss when artifact location changes

### Safety & Error Surface *(carried from v1.2.0)*

- [ ] **SAFE-04**: `flush_held_inputs` releases the REGISTRY lock before posting CGEvents — eliminates the potential deadlock on macOS emergency stop
- [ ] **ERR-01**: Persistence failures surface to the user in the UI — App.tsx listens to the `auto-save-error` Tauri event and shows a visible error notification when auto-save fails

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
| COMPAT-04 | Phase 7 | Pending |
| COMPAT-05 | Phase 7 | Pending |
| UX-11 | Phase 8 | Pending |
| UX-12 | Phase 8 | Pending |
| UX-13 | Phase 8 | Pending |
| UX-14 | Phase 8 | Pending |
| BUILD-01 | Phase 7 | Pending |
| MEM-01 | Phase 7 | Pending |
| CI-03 | Phase 7 | Pending |
| CI-04 | Phase 7 | Pending |
| CI-05 | Phase 7 | Pending |
| SAFE-04 | Phase 7 | Pending |
| ERR-01 | Phase 7 | Pending |
| EXEC-01 | Phase 9 | Pending |
| EXEC-02 | Phase 9 | Pending |
| UX-08 | Phase 10 | Pending |
| UX-09 | Phase 10 | Pending |
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
*Last updated: 2026-06-17 — added COMPAT-04/05 (macOS 26 follow-ups) and UX-11/12/13/14 (hotkey reliability); phases renumbered 8→9, 9→10*
