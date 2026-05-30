---
phase: 03-macro-setup-ux
plan: "04"
subsystem: frontend-keymap-ux
tags: [gap-closure, null-safety, key-capture, trigger-key, macos]
depends_on:
  requires: [03-03]
  provides: [UX-01-complete, CR-02-fix, CR-03-fix, WR-02-fix]
  affects: [src/keymap.ts, src/App.tsx]
tech_stack:
  added: []
  patterns: [null-guard-before-callback, three-invoke-macos-path, show-fallback-placeholder]
key_files:
  created: []
  modified:
    - src/keymap.ts
    - src/App.tsx
decisions:
  - "domKeycodeToNative returns null (not 0) on miss so callers can distinguish 'unmapped' from 'KeyA' (CGKeyCode 0)"
  - "Null guard placed before onCommit in startCapture so capture stays active — user can press a valid key without restarting capture"
  - "IS_MACOS branch adds set_macro_trigger_key as third invoke after bind_hotkey so StateActor.trigger_key stays in sync with the hotkey binding"
  - "Set-key fallback uses dashed border style to visually distinguish 'not yet assigned' from the solid-border assigned-key badge"
metrics:
  duration: ~8m
  completed: "2026-05-30T14:43:10Z"
  tasks_completed: 2
  files_modified: 2
---

# Phase 03 Plan 04: Gap-Closure — domKeycodeToNative null return, startCapture guard, macOS persist, Set-key fallback Summary

## One-Liner

Null-safe key capture pipeline: domKeycodeToNative returns null on miss, startCapture guards before commit, macOS card-edit persists trigger_key via third invoke, and null-trigger cards show a clickable "Set key..." placeholder.

## What Was Built

Three UX blockers found in Phase 3 verification are fixed across two files:

**CR-03 (src/keymap.ts + src/App.tsx):** `domKeycodeToNative` previously returned `0` for unmapped keys (numpad, PrintScreen, international), which silently assigned CGKeyCode 0 ("A") as the trigger. The function now returns `number | null` with `map[code] ?? null`. A guard `if (nativeCode === null || nativeCode === 0) return;` was added in `startCapture` before `onCommit()` — capture stays active so the user can press a different key without restarting.

**CR-02 (src/App.tsx):** `handleCardSetTriggerKey` IS_MACOS branch called `unbind_hotkey` + `bind_hotkey` but never called `set_macro_trigger_key`, so `MacroConfig.trigger_key` in StateActor remained stale after card-edit. A third awaited invoke `await invoke("set_macro_trigger_key", { id, trigger_key: nativeCode })` was added after `bind_hotkey`, ensuring the state broadcast carries the new keycode.

**WR-02 (src/App.tsx):** The outer `<Show when={macro.trigger_key !== null}>` rendered nothing for macros without a trigger key. A `fallback=` prop was added rendering a dashed-border "Set key..." span that calls `setEditingCardId` + `setEditingField("key")` + `startCapture` — giving users an actionable path to assign a trigger key without deleting the macro.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Fix domKeycodeToNative null return + startCapture guard (CR-03) | ce39615 | src/keymap.ts, src/App.tsx |
| 2 | Persist trigger_key on macOS + Set-key fallback (CR-02 + WR-02) | 6e6d6a5 | src/App.tsx |

## Verification

```
grep -n "number | null" src/keymap.ts         → line 180 (function signature)
grep -n "map\[code\] ?? null" src/keymap.ts   → line 182
grep -n "nativeCode === null || nativeCode === 0" src/App.tsx → line 270
grep -n "set_macro_trigger_key" src/App.tsx   → lines 286 (IS_MACOS) + 288 (Windows)
grep -n "Set key" src/App.tsx                 → line 790 (fallback span)
npx tsc --noEmit                              → exits 0
```

## Deviations from Plan

None — plan executed exactly as written. Both tasks applied in the order and with the exact code patterns specified.

## Threat Flags

No new trust-boundary surface introduced. The null guard (T-03-04-01) and IS_MACOS invoke ordering (T-03-04-03) were explicitly mitigated as specified in the plan's threat model.

## Self-Check: PASSED

- src/keymap.ts modified: FOUND
- src/App.tsx modified: FOUND
- Commit ce39615: FOUND (Task 1)
- Commit 6e6d6a5: FOUND (Task 2)
- npx tsc --noEmit: exits 0
