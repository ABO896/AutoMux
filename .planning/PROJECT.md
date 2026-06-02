# AutoMux

## What This Is

AutoMux is a cross-platform desktop auto-clicker and macro automation tool for macOS and Windows, built with Tauri 2, Rust, and SolidJS. It lets users define multi-step macros (clicks, keypresses, timing, optional process targeting) and run them system-wide or scoped to a specific application, configuring them via a key-capture widget and running-process picker without needing to know system key codes or process names. It is aimed at users who need reliable, configurable input automation — gamers, productivity power users, and anyone who needs to automate repetitive mouse/keyboard tasks.

## Core Value

A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.

## Current Milestone: v2.0 Redesign & Platform Excellence

**Goal:** Make AutoMux fully functional on macOS Tahoe 26 and Windows, ship a modern liquid-glass UI redesign, add parallel macro execution and core UX improvements, and complete all outstanding reliability work — without sacrificing low overhead or click/hold macro support.

**Target features:**
- macOS Tahoe 26 compatibility: restore macro execution and fix permissions detection on the new OS
- Cross-platform: all features and fixes work on both macOS and Windows
- Parallel macro execution: multiple macros run simultaneously on both platforms
- Macro management: delete and edit existing macros; clearer action-type UI (left click / right click / hold / key press)
- Full UI redesign: Apple design language + Tahoe 26 liquid glass (macOS), modern equivalent (Windows), Raycast-inspired layout
- Platform cleanup (v1.2.0 carry): Windows compiler warnings (BUILD-01), OpenProcess handle leak (MEM-01)
- CI hardening (v1.2.0 carry): updater signature (CI-03), npm ci (CI-04), artifact discovery (CI-05)
- Safety & error surface (v1.2.0 carry): REGISTRY deadlock risk (SAFE-04), auto-save error in UI (ERR-01)

## Current State

**Version:** v1.2.0 (shipped 2026-06-02, Phase 5 complete) | v2.0 in planning

- 5 phases completed, 16 plans shipped — Phase 05 complete (CGEventTap post-grant arming, cocoa dep removal, pending UI state)
- ~22,900 lines added across Rust backend + SolidJS frontend
- Stack: Tauri 2 (IPC bridge), Rust (StateActor + Scheduler actors, CGEvent/SendInput platform layer), SolidJS + Tailwind CSS 4 + Vite 6
- macOS: CGEventTap input injection, NSWorkspace process enumeration, AXIsProcessTrusted permission check
- Windows: SendInput injection, EnumProcesses process enumeration
- Builds: unsigned DMG (macOS arm64+x86_64 universal), unsigned NSIS installer (Windows x64)
- CI: GitHub Actions release.yml with SHA-pinned third-party actions; universal binary enforced + lipo-verified

## Requirements

### Validated

- ✓ Macro creation with click and keypress actions — existing
- ✓ Per-macro enable/disable toggle — existing
- ✓ Hotkey-triggered macro activation — existing
- ✓ Configurable timing/delays between actions — existing
- ✓ Optional process targeting (scoped vs. system-wide) — existing
- ✓ Profile save/load (JSON persistence) — existing
- ✓ Windows and macOS builds via CI — existing
- ✓ GNU GPLv3 license — v1.0
- ✓ RELY-01: macOS permissions check correctly starts event tap on grant — v1.0 (partial: passive System Settings grant requires restart)
- ✓ RELY-02: CGEventTap re-enables on OS timeout — v1.0
- ✓ RELY-03: Windows emergency stop flushes held inputs — v1.0
- ✓ RELY-04: Macro changes auto-saved after every mutation — v1.0
- ✓ SAFE-01: All 5 production panic paths eliminated — v1.0
- ✓ SAFE-02: MacPlatformObserver lifetime fixed via app.manage — v1.0
- ✓ SAFE-03: 5ms minimum interval floor enforced — v1.0
- ✓ README-01/02/03: GPL-3.0 badge, prose fix, README rewrite — v1.0
- ✓ AUDIT-01/02/03: Stale artifacts removed, dead code clean, .gitignore extended — v1.0
- ✓ UX-01: Key capture widget (no raw key codes shown) — v1.0
- ✓ UX-02/03: Running-process picker on macOS + Windows — v1.0
- ✓ CI-01: Universal binary enforced (arm64+x86_64 via lipo) — v1.0
- ✓ CI-02: Third-party GitHub Actions SHA-pinned — v1.0
- ✓ PERM-01: macOS permissions correctly detected after Request Access approval — v1.2.0 Phase 5
- ✓ RELY-06: Passive System Settings grant arms CGEventTap without restart — v1.2.0 Phase 5
- ✓ BUILD-02: `block v0.1.6` macOS deprecation resolved — v1.2.0 Phase 5

### Active

- [ ] **COMPAT-01**: Macros fire correctly on macOS 26 Tahoe — input injection works
- [ ] **COMPAT-02**: Permissions detection accurate on macOS 26 Tahoe — no false negatives
- [ ] **COMPAT-03**: App launches fully on macOS 26 Tahoe without errors or crashes
- [ ] **EXEC-01**: Multiple macros run simultaneously on macOS — parallel, not sequential
- [ ] **EXEC-02**: Multiple macros run simultaneously on Windows — parallel, not sequential
- [ ] **UX-08**: User can delete an existing macro from the macro list
- [ ] **UX-09**: User can edit an existing macro's name, action type, key/button, and timing
- [ ] **UX-10**: Action type selection clearly labels left click, right click, hold, and key press
- [ ] **UI-01**: macOS app uses Apple design language with liquid glass effects native to macOS 26
- [ ] **UI-02**: macOS UI layout is Raycast-inspired — clean, focused, keyboard-navigable
- [ ] **UI-03**: Windows app uses a modern, polished equivalent UI
- [ ] **UI-04**: UI redesign adds no measurable idle overhead vs v1.2.0
- [ ] **BUILD-01**: Zero Rust compiler warnings on Windows target (`platform/windows/mod.rs`)
- [ ] **MEM-01**: Windows `OpenProcess` handle closed after use in `list_running_apps_impl`
- [ ] **CI-03**: Updater JSON signature included in release workflow (not silently skipped)
- [ ] **CI-04**: CI uses `npm ci` for reproducible builds
- [ ] **CI-05**: Binary artifact discovery resilient to tauri-action renames
- [ ] **SAFE-04**: `flush_held_inputs` releases REGISTRY lock before posting CGEvents
- [ ] **ERR-01**: `auto-save-error` event surfaces persistence failures in the UI

### Out of Scope

- **A11Y-01** (v1.3 UI milestone) — ARIA attributes deferred to UI redesign so they're built into the new component structure
- Auto-updater (v2) — core reliability must be solid first; signing/entitlements required as prerequisite
- macOS notarization (v2) — signing identity config is a one-way door; defer until process is finalized
- Cross-platform profile portability (v2) — requires NamedKey schema migration (UX-04) first
- Mobile / web app — desktop automation tool by design

## Context

- **Codebase state:** ~22,900 lines added in v1.0 across Rust + TypeScript. Rust backend: StateActor/Scheduler actor model, platform input layer. Frontend: single App.tsx with SolidJS signals.
- **Key known issues:** RELY-01 passive permission grant gap (requires app restart if user goes directly to System Settings); auto-save-error event has no UI listener; Windows OpenProcess handle leak in list_running_apps.
- **CI state:** Release workflow produces unsigned DMG (macOS) and NSIS installer (Windows). lipo verification gates the macOS artifact. npm install (not npm ci) — reproducibility gap.
- **User feedback:** No external users yet — all requirements from initial requirements definition.

## Constraints

- **Tech stack:** Tauri 2 + Rust + SolidJS — no framework changes; improvements must work within this architecture
- **Compatibility:** Must maintain working builds for both macOS (arm64 + x86_64) and Windows (x64)
- **Permissions model:** macOS Accessibility permission handling is OS-enforced; fixes must work within what Tauri and CGEvent allow

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Reserve auto-updater as Phase 5 / v2 | Core reliability must be solid first; signing/entitlements required as prerequisite | ✓ Good — v1.0 shipped without the complexity |
| Store MacPlatformObserver via app.manage() | Observer token was dropped at end of setup function — lifetime fix needed | ✓ Good — resource leak closed |
| Key capture widget + keymap.ts mapping | Raw key codes are opaque to non-technical users | ✓ Good — UX-01 closed cleanly |
| Process picker instead of text input | Users have no reliable way to know the exact process name string | ✓ Good — UX-02/03 closed |
| 3s frontend poll for permission grant | Polling sidesteps the need for OS notification APIs; restart-free grant detection | ⚠️ Revisit — passive System Settings grant still requires restart |
| SHA pinning for GitHub Actions | Supply chain security: floating tags are vulnerable to tag-moving attacks | ✓ Good — CI-02 closed |
| `--target universal-apple-darwin` + lipo check | Previous CI produced arm64-only binary — enforcement needed | ✓ Good — CI-01 enforced |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-06-02 — Milestone v2.0 started*
