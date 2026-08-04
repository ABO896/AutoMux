# Milestones: AutoMux

## v2.0 Redesign & Platform Excellence (Shipped: 2026-08-04)

**Phases completed:** 5 phases (6-10), 31 plans, 72 tasks
**Requirements:** 24/25 satisfied (COMPAT-02 carried forward as device-verification debt — see PROJECT.md)
**Security:** 0 open threats at the `high` block-on threshold, all 5 phases

**Key accomplishments:**

- macOS Tahoe 26 compatibility restored: live CGEventTap probe replacing a stale cached permission check, plus a CFRunLoop stop/reinit fix for a UAF crash in the tap's re-enable callback
- Windows platform cleanup (compiler warnings, `OpenProcess` handle leak) + CI hardening (`npm ci`, updater signature upload, resilient artifact discovery) + Input Monitoring / TCC identity-change detection on macOS 26
- Hotkey system reliability: full key-range binding (0-9, F1-F12, all modifiers), conflict detection with explicit reassignment, same-input overlap warnings, and verified/communicated global (system-wide) hotkey behavior on both platforms
- Parallel macro execution via a StateActor/Scheduler two-phase dispatch redesign — triggering one macro never blocks, queues, or cancels another; closed through 9 gap-closure plans (stuck-input HoldRelease bypass, hotkey-rebind rollback safety, dual-registry double-dispatch, IPC argument-casing)
- Macro management (delete, full edit — name/action/key/timing) and a redesigned Apple-inspired sidebar UI on macOS with a matching modern UI on Windows, light/dark/OS-follow theming
- Retroactive per-phase security threat registers compiled for all 5 phases

---

## v1.0 — MVP

**Shipped:** 2026-05-30
**Phases:** 4 (Phases 1–4)
**Plans:** 14
**Timeline:** 15 days (2026-05-15 → 2026-05-30)
**Code delta:** 145 files changed, +22,890 / −7,703 lines

### Delivered

Hardened AutoMux from a working prototype into a reliable, user-friendly tool — fixing the macOS permissions loop, eliminating all production panic paths, replacing raw key-code and process-name fields with a key-capture widget and running-process picker, restoring GPL-3.0 legal correctness, and locking in CI supply-chain security with SHA pinning and universal binary enforcement.

### Key Accomplishments

1. **Reliability:** macOS CGEventTap re-enable on OS timeout + 3s permission-grant detection — hotkeys survive long sessions and no restart required after granting accessibility.
2. **Safety:** All 5 production `unwrap()`/`expect()` panics on the input-injection hot path eliminated; Windows emergency stop now flushes held inputs synchronously.
3. **Auto-save:** Arc<ProfileManager> injected into StateActor — every macro mutation persisted to disk automatically; no work lost on app restart.
4. **UX:** Raw CGKeyCode/VK integers replaced with key-capture widget; manual process-name field replaced with running-process picker (macOS via NSWorkspace, Windows via EnumProcesses).
5. **Legal + README:** Full GPL-3.0 LICENSE file created; shields.io badge corrected; README rewritten to lead with user value.
6. **CI hardening:** dtolnay/rust-toolchain and tauri-apps/tauri-action pinned to full 40-char commit SHAs; `--target universal-apple-darwin` + blocking lipo verification enforces universal binary.

### Requirements

18/18 v1 requirements satisfied. See `.planning/milestones/v1.0-REQUIREMENTS.md`.

### Known Tech Debt

- RELY-01 partial: passive System Settings permission grant without clicking "Request Access" requires app restart
- MacOS emergency stop: flush_held_inputs holds REGISTRY lock while posting CGEvents (potential deadlock)
- Windows: OpenProcess HANDLE never closed in list_running_apps_impl
- CI: npm install instead of npm ci; binary discovery pattern fragile to tauri-action renames
- auto-save-error Tauri event emitted but App.tsx has no listener

Full inventory: `.planning/milestones/v1.0-MILESTONE-AUDIT.md`

### Archive

- Roadmap: `.planning/milestones/v1.0-ROADMAP.md`
- Requirements: `.planning/milestones/v1.0-REQUIREMENTS.md`
- Audit: `.planning/milestones/v1.0-MILESTONE-AUDIT.md`
- Phases: `.planning/milestones/v1.0-phases/`

---
