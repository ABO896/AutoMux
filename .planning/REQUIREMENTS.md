# Requirements: AutoMux v1.2.0

**Defined:** 2026-05-31
**Milestone:** v1.2.0 — Reliability & Polish
**Core Value:** A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.

## v1.2.0 Requirements

### Permissions

- [ ] **PERM-01**: macOS app correctly detects accessibility permission as granted after user clicks Request Access and approves the OS prompt — no false "not granted" state persists after approval

### Reliability

- [ ] **RELY-06**: Granting accessibility in System Settings directly (without clicking Request Access first) arms CGEventTap and activates macros without requiring an app restart

### Build Warnings

- [ ] **BUILD-01**: Zero Rust compiler warnings on Windows build — removes unused `GetWindowTextW`, `IsWindowVisible`, `HMODULE`, `HHOOK` imports and handles `TranslateMessage` unused-bool warning in `platform/windows/mod.rs`
- [ ] **BUILD-02**: `block v0.1.6` macOS deprecation resolved — crate upgraded or dependency migrated so the build is clean on future Rust versions

### CI Pipeline

- [ ] **CI-03**: Release workflow correctly uploads updater JSON signature artifact — no more "Signature not found for the updater JSON. Skipping upload…" in CI output
- [ ] **CI-04**: CI uses `npm ci` instead of `npm install` for reproducible, lockfile-gated dependency installs
- [ ] **CI-05**: Binary artifact discovery pattern hardened against tauri-action output path renames (no silent miss when artifact location changes)

### Safety

- [ ] **SAFE-04**: `flush_held_inputs` releases the REGISTRY lock before posting CGEvents — eliminates the potential deadlock on macOS emergency stop where lock was held across event dispatch

### Error Handling

- [ ] **ERR-01**: Persistence failures surface to the user in the UI — App.tsx listens to the `auto-save-error` Tauri event and displays an error indicator when auto-save fails

### Memory

- [ ] **MEM-01**: Windows `OpenProcess` handle is closed after use in `list_running_apps_impl` — no HANDLE leak on every process list refresh

## Future Requirements

### Accessibility (v1.3 — UI Milestone)

- **A11Y-01**: ARIA attributes and screen reader compatibility — deferred to UI redesign milestone so attributes are built into the new component structure from the start

### Distribution (v2)

- **DIST-01**: Auto-updater — deferred; signing/entitlements required as prerequisite
- **DIST-02**: macOS notarization — deferred; signing identity is a one-way door

### UX (v2)

- **UX-04**: NamedKey schema migration for cross-platform profile portability — deferred
- **UX-05**: Cross-platform profile portability — depends on UX-04

## Out of Scope

| Feature | Reason |
|---------|--------|
| A11Y-01 ARIA attributes | Deferred to UI redesign milestone — build into new components, not retrofitted |
| Auto-updater | Requires code signing; defer until signing/entitlements are finalized |
| macOS notarization | Signing identity config is a one-way door; defer until process is finalized |
| Cross-platform profiles | Requires NamedKey schema migration (UX-04) first |
| Mobile / web app | Desktop automation tool by design |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| PERM-01 | Phase 5 | Pending |
| RELY-06 | Phase 5 | Pending |
| BUILD-02 | Phase 5 | Pending |
| BUILD-01 | Phase 6 | Pending |
| MEM-01 | Phase 6 | Pending |
| CI-03 | Phase 7 | Pending |
| CI-04 | Phase 7 | Pending |
| CI-05 | Phase 7 | Pending |
| SAFE-04 | Phase 8 | Pending |
| ERR-01 | Phase 8 | Pending |

**Coverage:**
- v1.2.0 requirements: 10 total
- Mapped to phases: 10 (Phases 5–8)
- Unmapped: 0 ✓

---
*Requirements defined: 2026-05-31*
*Last updated: 2026-05-31 — roadmap created, all requirements mapped*
