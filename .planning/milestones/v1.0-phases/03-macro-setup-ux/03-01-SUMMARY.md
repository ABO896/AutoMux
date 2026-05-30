---
phase: 03-macro-setup-ux
plan: "01"
subsystem: frontend-util, ipc, state, platform
tags: [keymap, process-picker, ipc, tauri, solidjs, rust, macos, windows]
dependency_graph:
  requires: []
  provides:
    - src/keymap.ts (domKeycodeToNative, resolveKeyName)
    - list_running_apps IPC command
    - set_macro_trigger_key IPC command
    - Intent::SetMacroTriggerKey
  affects:
    - src-tauri/src/ipc/mod.rs
    - src-tauri/src/state/mod.rs
    - src-tauri/src/platform/macos/observer.rs
    - src-tauri/src/platform/windows/mod.rs
    - src-tauri/src/lib.rs
tech_stack:
  added: []
  patterns:
    - e.code string keyed lookup tables (avoids F12/ArrowLeft DOM keyCode collision)
    - NSWorkspace.runningApplications() with activationPolicy == Regular filter
    - EnumWindows + get_app_name_from_hwnd with HashMap deduplication
    - Platform-conditional #[cfg] blocks inside single #[command] function
key_files:
  created:
    - src/keymap.ts
  modified:
    - src-tauri/src/ipc/mod.rs
    - src-tauri/src/state/mod.rs
    - src-tauri/src/platform/macos/observer.rs
    - src-tauri/src/platform/windows/mod.rs
    - src-tauri/src/lib.rs
decisions:
  - Use e.code string as lookup table key (not e.keyCode number) to avoid F12/ArrowLeft DOM keyCode 123 collision
  - list_running_apps_impl on macOS uses NSWorkspace without unsafe block (sharedWorkspace is safe in objc2 0.6)
  - SetMacroTriggerKey handler delegates to reevaluate_all_macros for Windows MACRO_TRIGGER_KEYS propagation (no duplicate call needed)
metrics:
  duration_minutes: 36
  completed_date: "2026-05-30"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 6
---

# Phase 3 Plan 1: Foundation — Keymap Module and IPC Commands Summary

Static frontend keymap module and two new Rust IPC commands providing the e.code→CGKeyCode/VK lookup tables, running app enumeration, and trigger key update path required by Plans 02 and 03.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Create src/keymap.ts — static platform keycode lookup tables | 56b5a61 | src/keymap.ts |
| 2 | Rust backend — RunningApp struct, list_running_apps, set_macro_trigger_key, Intent variant, StateActor handler, lib.rs registration | 54a4914 | src-tauri/src/ipc/mod.rs, src-tauri/src/state/mod.rs, src-tauri/src/platform/macos/observer.rs, src-tauri/src/platform/windows/mod.rs, src-tauri/src/lib.rs |

## What Was Built

### src/keymap.ts

Four module-private lookup tables keyed by `e.code` strings:
- `DOM_KEYCODE_TO_CGKEYCODE`: `e.code` string → macOS CGKeyCode integer (letters, digits, special, arrows, F1-F12)
- `CGKEYCODE_TO_NAME`: CGKeyCode integer → human-readable display name
- `DOM_KEYCODE_TO_VK`: `e.code` string → Windows VK code integer
- `VK_TO_NAME`: VK code integer → human-readable display name

Two exported functions:
- `domKeycodeToNative(code: string): number` — uses `e.code` as key, returns 0 on miss
- `resolveKeyName(nativeCode: number): string` — fallback: `"Key ${nativeCode}"` per D-03

Platform detection via `navigator.platform.toLowerCase().includes("mac")`.

### Rust Backend

- `RunningApp` struct in `ipc/mod.rs` with `display_name: String` and `identifier: String`, derives `Debug, Clone, Serialize`
- `list_running_apps` command: macOS implementation uses `NSWorkspace::sharedWorkspace().runningApplications()` filtered to `NSApplicationActivationPolicy::Regular`, sorted by display name; Windows uses `EnumWindows` + existing `get_app_name_from_hwnd` helper with HashMap deduplication by exe path; non-platform fallback returns empty vec
- `set_macro_trigger_key` command: mirrors `set_macro_target_app` pattern, sends `Intent::SetMacroTriggerKey(id, trigger_key)`
- `Intent::SetMacroTriggerKey(Uuid, Option<u16>)` variant added after `SetMacroTargetApp` in the Intent enum
- StateActor handler mutates `mac.trigger_key`, calls `reevaluate_all_macros` (which propagates trigger keys to `MACRO_TRIGGER_KEYS` on Windows via the existing path), then `auto_save_default`
- Both commands registered in `lib.rs` `generate_handler![]`

## Deviations from Plan

### Auto-fixed Issues

None.

### Implementation Notes

**1. NSWorkspace::sharedWorkspace() — no unsafe needed**

The plan's PATTERNS.md showed the call wrapped in `unsafe {}`. In objc2 0.6.x the `sharedWorkspace()` method is marked safe, so the `unsafe` block triggered a compiler warning (`unused_unsafe`). The wrapper was removed in the final implementation.

**2. SetMacroTriggerKey Windows update_macro_trigger_keys**

The plan specified an explicit `#[cfg(target_os = "windows")]` block calling `update_macro_trigger_keys` in the handler. This was not added because `reevaluate_all_macros().await` already performs this call unconditionally (line 465-466 in state/mod.rs). Adding a second call would have been a duplicate. The behavior is correct and consistent with all other trigger-key-affecting handlers (SetMacroEnabled, SetMacroTargetApp) which also rely on `reevaluate_all_macros` for Windows propagation.

## Verification

- `npx tsc --noEmit` exits 0 (no TypeScript errors)
- `cargo build` exits 0 with no warnings
- `grep -c "export function domKeycodeToNative" src/keymap.ts` → 1
- `grep -c "export function resolveKeyName" src/keymap.ts` → 1
- `grep -c "SetMacroTriggerKey" src-tauri/src/state/mod.rs` → 2 (enum variant + match arm)
- `grep -c "list_running_apps_impl" src-tauri/src/platform/macos/observer.rs` → 1
- `grep -c "list_running_apps_impl" src-tauri/src/platform/windows/mod.rs` → 2 (cfg-gated impl + fallback)
- `grep -c "ipc::list_running_apps" src-tauri/src/lib.rs` → 1
- `grep -c "ipc::set_macro_trigger_key" src-tauri/src/lib.rs` → 1

## Known Stubs

None — this plan delivers infrastructure (lookup tables and IPC commands), not UI. No placeholder data flows to rendered output.

## Threat Flags

None — all surfaces were accounted for in the plan's threat model (T-03-01 through T-03-05). No new network endpoints, auth paths, or schema changes at trust boundaries were introduced beyond what was planned.

## Self-Check: PASSED

- src/keymap.ts: FOUND
- src-tauri/src/ipc/mod.rs: MODIFIED (RunningApp, list_running_apps, set_macro_trigger_key)
- src-tauri/src/state/mod.rs: MODIFIED (SetMacroTriggerKey variant + handler)
- src-tauri/src/platform/macos/observer.rs: MODIFIED (NSApplicationActivationPolicy import, list_running_apps_impl)
- src-tauri/src/platform/windows/mod.rs: MODIFIED (list_running_apps_impl)
- src-tauri/src/lib.rs: MODIFIED (two new registrations)
- Commit 56b5a61: FOUND (Task 1)
- Commit 54a4914: FOUND (Task 2)
