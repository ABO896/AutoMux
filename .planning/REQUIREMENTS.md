# Requirements: AutoMux

**Defined:** 2026-05-15
**Core Value:** A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.

## v1 Requirements

### Reliability

- [ ] **RELY-01**: macOS permissions check correctly starts the event tap when accessibility permission transitions from denied to granted — no false "security denied" prompt after grant
- [ ] **RELY-02**: macOS CGEventTap re-enables itself when the OS disables it via timeout (kCGEventTapDisabledByTimeout) — hotkeys do not silently stop working during long sessions
- [ ] **RELY-03**: Windows emergency stop (Ctrl+Shift+Q) flushes held inputs before exiting — no keys left stuck after macro stop
- [ ] **RELY-04**: Macro changes are auto-saved after every mutation — no user work lost on app restart

### Safety

- [ ] **SAFE-01**: All 5 production `unwrap()` / `expect()` calls on the input injection hot path replaced with recoverable error handling — app cannot panic during macro execution
- [ ] **SAFE-02**: MacPlatformObserver resource leak fixed — observer token is correctly reclaimed; `stop_observing` is no longer dead code
- [ ] **SAFE-03**: Minimum macro interval floor enforced at 5ms — intervals below this threshold are rejected or clamped to prevent system instability

### README & License

- [ ] **README-01**: README prose corrected from "MIT License" to "GNU GPL v3" — no legal contradiction between prose and actual license file
- [ ] **README-02**: Correct GPL-3.0 badge added to README (shields.io SPDX identifier `GPL-3.0-only`)
- [ ] **README-03**: README copy improved for appeal and clarity to new users — accurately represents features, platforms, and purpose

### Codebase Cleanup

- [ ] **AUDIT-01**: `implementation_plan.md` and other non-project planning documents removed from repository root
- [ ] **AUDIT-02**: Dead/unused source files identified and removed across Rust and SolidJS layers
- [ ] **AUDIT-03**: Debug artifacts and temp files removed and added to `.gitignore`

### UX — Macro Setup

- [ ] **UX-01**: Key Code field replaced with a key capture widget — user presses the key and it is captured; raw CGKeyCode/VK integers never shown to user
- [ ] **UX-02**: macOS target app field replaced with a running-process picker showing app name + bundle ID — no manual text entry required
- [ ] **UX-03**: Windows target app field replaced with a running-process picker showing process name and path — no manual text entry required

### CI & Distribution

- [ ] **CI-01**: macOS release CI verified to produce universal binary (arm64 + x86_64) — not arm64-only
- [ ] **CI-02**: GitHub Actions SHA pinning added to release.yml for security hardening

## v2 Requirements

### Distribution

- **DIST-01**: In-app auto-updater — checks for new releases and prompts user to update without manual download
- **DIST-02**: macOS notarization — app signed and notarized for Gatekeeper-clean distribution (requires entitlements plist)

### UX Polish

- **UX-04**: `NamedKey` enum schema migration — profiles store human-readable key names (e.g. `"Space"`) instead of platform-specific integers; enables cross-platform profile portability
- **UX-05**: Cross-platform profile portability — targeted macros work when profile is transferred between macOS and Windows

### Accessibility

- **A11Y-01**: ARIA attributes added to all interactive elements — `aria-pressed`, `aria-label`, `role` attributes for screen reader compatibility

## Out of Scope

| Feature | Reason |
|---------|--------|
| Auto-updater (v1) | Deferred to v2 — core reliability must be solid first; signing/entitlements required as prerequisite |
| macOS notarization (v1) | Signing identity config is a one-way door — changing it after release invalidates existing users' Accessibility grants; defer until process is finalized |
| Cross-platform profile portability (v1) | Known architectural limitation; macOS uses bundle IDs, Windows uses exe paths; requires NamedKey schema migration first |
| Mobile / web app | Out of scope entirely — desktop automation tool by design |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| RELY-01 | Phase 1 | Pending |
| RELY-02 | Phase 1 | Pending |
| RELY-03 | Phase 1 | Pending |
| RELY-04 | Phase 1 | Pending |
| SAFE-01 | Phase 1 | Pending |
| SAFE-02 | Phase 2 | Pending |
| SAFE-03 | Phase 1 | Pending |
| README-01 | Phase 2 | Pending |
| README-02 | Phase 2 | Pending |
| README-03 | Phase 2 | Pending |
| AUDIT-01 | Phase 2 | Pending |
| AUDIT-02 | Phase 2 | Pending |
| AUDIT-03 | Phase 2 | Pending |
| UX-01 | Phase 3 | Pending |
| UX-02 | Phase 3 | Pending |
| UX-03 | Phase 3 | Pending |
| CI-01 | Phase 4 | Pending |
| CI-02 | Phase 4 | Pending |

**Coverage:**
- v1 requirements: 18 total
- Mapped to phases: 18
- Unmapped: 0 ✓

---
*Requirements defined: 2026-05-15*
*Last updated: 2026-05-15 after initial definition*
