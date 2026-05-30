# Roadmap: AutoMux

## Overview

This milestone hardens AutoMux from a working prototype into a reliable, user-friendly tool. Four active phases address the root causes of user-visible failures — a broken permissions loop, unsafe panic paths, a misleading README, and UX dead-ends for key binding and process targeting — then lock in CI hygiene before shipping. Phase 5 is reserved for auto-updater infrastructure once the core is stable.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Reliability & Safety** - Fix the permissions loop, CGEventTap timeout, Windows flush gap, auto-save loss, interval floor, and panic paths (completed 2026-05-16)
- [ ] **Phase 2: Cleanup & Credibility** - Correct the README license, fix the MacPlatformObserver leak, and scrub the repository of dead code and stale artifacts
- [ ] **Phase 3: Macro Setup UX** - Replace the raw key-code field and manual process-name entry with a key capture widget and a running-process picker
- [ ] **Phase 4: CI Hardening** - Verify the universal binary build and pin third-party GitHub Actions to specific SHAs
- [ ] **Phase 5: Auto-Updater (Reserved)** - Reserved for v2; no v1 requirements assigned

## Phase Details

### Phase 1: Reliability & Safety
**Goal**: Macros fire reliably — permissions are detected without restart, hotkeys survive long sessions, no work is lost on close, and the app cannot panic during execution
**Mode**: mvp
**Depends on**: Nothing (first phase)
**Requirements**: RELY-01, RELY-02, RELY-03, RELY-04, SAFE-01, SAFE-03
**Success Criteria** (what must be TRUE):
  1. User grants Accessibility permission in System Settings, returns to AutoMux without restarting, and macros fire immediately — no false "security denied" prompt shown after the grant
  2. A macro running continuously for 30+ minutes keeps responding to its hotkey — no silent tap-disabled state during long sessions
  3. User triggers the Windows emergency stop (Ctrl+Shift+Q); after exit, no keyboard keys remain physically stuck on the system
  4. User creates and edits macros, closes AutoMux without manually saving, reopens — all changes are present
  5. Setting a macro step interval below 5ms is rejected or clamped to 5ms; the app cannot be used to saturate the OS input queue
**Plans**: 4 plans
- [x] 01-01-PLAN.md — Frontend permissions poll fix (3s) + scheduler 5ms interval floor (RELY-01, SAFE-03)
- [x] 01-02-PLAN.md — macOS panic elimination + CGEventTap timeout re-enable (SAFE-01 macOS, RELY-02)
- [x] 01-03-PLAN.md — Windows lock-unwrap panic elimination + emergency-stop synchronous flush (SAFE-01 Windows, RELY-03)
- [x] 01-04-PLAN.md — StateActor auto-save end-to-end with LoadProfile bracketing (RELY-04)

### Phase 2: Cleanup & Credibility
**Goal**: The repository presents accurate legal information, contains no dead code or stale artifacts, and the MacPlatformObserver resource leak is closed
**Mode**: mvp
**Depends on**: Phase 1
**Requirements**: README-01, README-02, README-03, SAFE-02, AUDIT-01, AUDIT-02, AUDIT-03
**Success Criteria** (what must be TRUE):
  1. README displays a "GPL-3.0-only" shields.io badge and contains no mention of "MIT License" anywhere in the document
  2. `implementation_plan.md` is absent from the repository root; `src-tauri/gen/` and debug temp files are covered by `.gitignore`
  3. No unused source files remain in the Rust or SolidJS layers — dead code removed or documented as intentional
  4. MacPlatformObserver token is stored for its full application lifetime; `stop_observing` is no longer dead code
**Plans**: 4 plans
- [x] 02-01-PLAN.md — License correctness + README appeal pass (README-01, README-02, README-03)
- [x] 02-02-PLAN.md — Repo hygiene: delete implementation_plan.md, extend .gitignore, verify AUDIT-02 (AUDIT-01, AUDIT-02, AUDIT-03)
- [x] 02-03-PLAN.md — MacPlatformObserver lifetime fix via app.manage (SAFE-02)
- [x] 02-04-PLAN.md — Frontend version string fix + comment alignment (README-03 credibility)
**UI hint**: yes

### Phase 3: Macro Setup UX
**Goal**: Users can configure key bindings and target applications without knowing raw system codes or exact process name strings
**Mode**: mvp
**Depends on**: Phase 1
**Requirements**: UX-01, UX-02, UX-03
**Success Criteria** (what must be TRUE):
  1. User sets a trigger key by clicking the key-binding field and pressing the desired key — raw CGKeyCode or VK integers are never shown in the UI
  2. On macOS, user opens the target-app picker and sees a list of running applications by display name; selecting one sets the correct bundle ID without any manual text entry
  3. On Windows, user opens the target-app picker and sees running processes by name and path; selecting one sets the correct exe path without any manual text entry
**Plans**: 4 plans
- [x] 03-01-PLAN.md — keymap.ts lookup tables + Rust list_running_apps + set_macro_trigger_key IPC (UX-01, UX-02, UX-03)
- [x] 03-02-PLAN.md — Key capture widget in creation form + card key badge click-to-edit (UX-01)
- [x] 03-03-PLAN.md — Process picker <select> in creation form + card target badge click-to-edit (UX-02, UX-03)
- [x] 03-04-PLAN.md — Gap closure: CR-02 macOS trigger_key persistence, CR-03 null keymap guard, WR-02 Set-key placeholder (UX-01)
**UI hint**: yes

### Phase 4: CI Hardening
**Goal**: The CI pipeline is verifiably secure and produces the correct universal binary artifacts for macOS
**Mode**: mvp
**Depends on**: Phase 2
**Requirements**: CI-01, CI-02
**Success Criteria** (what must be TRUE):
  1. A macOS release build produces a single universal binary containing both arm64 and x86_64 slices — confirmed by inspecting the artifact with `lipo -info`
  2. Every `uses:` reference in `release.yml` that previously pointed to a floating tag (e.g., `@v0`) is replaced with a pinned full-length commit SHA
**Plans**: 2 plans
- [x] 04-01-PLAN.md — Pin dtolnay/rust-toolchain and tauri-apps/tauri-action to full commit SHAs with tag-comment annotations (CI-02)
- [x] 04-02-PLAN.md — Add `--target universal-apple-darwin` arg to tauri-action on macOS leg + blocking lipo verification step (CI-01)

### Phase 5: Auto-Updater (Reserved)
**Goal**: Reserved for v2 — in-app update mechanism with signing and notarization prerequisites
**Mode**: mvp
**Depends on**: Phase 4
**Requirements**: None (v2 scope — DIST-01, DIST-02 are v2 requirements)
**Success Criteria** (what must be TRUE):
  1. (Reserved — success criteria to be defined when this phase is activated)
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Reliability & Safety | 4/4 | Complete    | 2026-05-16 |
| 2. Cleanup & Credibility | 0/4 | Not started | - |
| 3. Macro Setup UX | 0/4 | Not started | - |
| 4. CI Hardening | 0/2 | Not started | - |
| 5. Auto-Updater (Reserved) | - | Reserved (v2) | - |
