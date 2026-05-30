# AutoMux

## What This Is

AutoMux is a cross-platform desktop auto-clicker and macro automation tool for macOS and Windows, built with Tauri 2, Rust, and SolidJS. It lets users define multi-step macros (clicks, keypresses, timing, optional process targeting) and run them system-wide or scoped to a specific application. It is aimed at users who need reliable, configurable input automation — gamers, productivity power users, and anyone who needs to automate repetitive mouse/keyboard tasks.

## Core Value

A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.

## Requirements

### Validated

- ✓ Macro creation with click and keypress actions — existing
- ✓ Per-macro enable/disable toggle — existing
- ✓ Hotkey-triggered macro activation — existing
- ✓ Configurable timing/delays between actions — existing
- ✓ Optional process targeting (scoped vs. system-wide) — existing
- ✓ Profile save/load (JSON persistence) — existing
- ✓ Windows and macOS builds via CI — existing
- ✓ GNU GPLv3 license — existing (needs README badge fix)

### Active

- [ ] Fix macOS false-positive permissions denial — app reports "security denied" and redirects to Accessibility settings even when AutoMux already has permissions granted there
- [ ] Fix README: add correct GNU GPLv3 license badge; audit all badges and links for accuracy; improve appeal and clarity for new users
- [ ] Codebase audit: identify and remove dead/unused source files, stale planning docs from root, debug artifacts, and misconfigured CI/build files
- [ ] UX: replace "Key Code" raw-value field with a human-readable key name label or key capture input so users understand what value to enter
- [ ] UX: replace manual process name text field with a running-process picker (dropdown or modal showing active processes) for target app selection
- [ ] Critical bug/issue resolution: surface and fix any critical bugs or regressions uncovered during the audit

### Out of Scope

- Auto-updater / in-app update mechanism — reserved for a future phase after core reliability is established; see Phase 5 in ROADMAP.md

## Context

- **Stack:** Tauri 2 (IPC bridge), Rust backend (StateActor + Scheduler actors, platform input layer via CGEvent/SendInput), SolidJS frontend (TypeScript), Tailwind CSS 4, Vite 6
- **macOS permissions:** AutoMux uses `AXIsProcessTrusted()` to check Accessibility permissions. The known failure mode is that the check returns false after the app was just granted permissions, before a restart or re-check. The platform observer layer lives in `src-tauri/src/platform/macos/observer.rs`.
- **Key Code field:** Currently exposes raw CGKeyCode / virtual key integers to users, which is not user-friendly. Needs either a key-capture widget or a mapping to display human-readable key names.
- **Process picker:** Current target app field requires manual typing of the process name. Users don't know what string to enter. Should be replaced with an enumerated list of running processes.
- **README state:** Release badge was showing broken because release was in draft; now published and working. License badge still shows "not found" — GPLv3 SPDX identifier needs to be corrected. README overall needs to better represent the project to attract users.
- **Codebase:** `implementation_plan.md` exists in the project root — likely should not be committed. CI/build configs may have stale or misconfigured entries.

## Constraints

- **Tech stack:** Tauri 2 + Rust + SolidJS — no framework changes; improvements must work within this architecture
- **Compatibility:** Must maintain working builds for both macOS (arm64 + x86_64) and Windows (x64)
- **Permissions model:** macOS Accessibility permission handling is OS-enforced; the fix must work within what Tauri and CGEvent allow

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Reserve auto-updater as a future phase | Core reliability (permissions, UX, audit) must be solid first | — Pending |
| Process picker instead of text input | Users have no reliable way to know the exact process name string | — Pending |
| Key capture / human-readable key names | Raw key codes are opaque to non-technical users | — Pending |

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
*Last updated: 2026-05-30 — Phase 03 (macro-setup-ux) complete: key capture widget added with null-safe domKeycodeToNative, macOS card-edit now persists trigger_key via set_macro_trigger_key, key-less macro cards show clickable "Set key…" placeholder, app-targeting dropdown populated via list_running_apps IPC. Requirements UX-01, UX-02, UX-03 delivered.
