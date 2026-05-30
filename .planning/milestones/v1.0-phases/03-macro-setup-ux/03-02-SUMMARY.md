---
phase: 03-macro-setup-ux
plan: "02"
subsystem: frontend
tags: [key-capture, solidjs, ux, keymap, tauri]
dependency_graph:
  requires:
    - src/keymap.ts (domKeycodeToNative, resolveKeyName) — provided by plan 03-01
  provides:
    - key capture widget in creation form (replaces input#input-macro-trigger-key)
    - card key badge click-to-edit with inline recording state
    - editingCardId and editingField app-level signals
    - startCapture() function with listener lifecycle management
    - handleCardSetTriggerKey() with macOS unbind+rebind path
  affects:
    - src/App.tsx
tech_stack:
  added: []
  patterns:
    - Module-level listener ref (_keyCaptureListener) for single-listener invariant (T-03-08)
    - startCapture() with Escape/modifier guard and capture-phase document listener
    - Conditional class template literal for recording/set/empty widget states
    - Show component with fallback for card inline edit state toggle
    - Mutual exclusion: form capture cancels card edit; card edit tracked via editingCardId
key_files:
  created: []
  modified:
    - src/App.tsx
decisions:
  - Form key capture widget and card key badge share startCapture() — single listener implementation
  - Form widget onClick cancels any in-progress card edit before starting capture (mutual exclusion UX)
  - Card cancel ✕ removes keydown listener and resets triggerKeyRecording as well as editingCardId/editingField
  - macOS card edit path calls unbind_hotkey then bind_hotkey (Pitfall 3 per RESEARCH.md, T-03-07)
  - Windows card edit path calls set_macro_trigger_key IPC (new command from plan 03-01)
metrics:
  duration_minutes: 5
  completed_date: "2026-05-30"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 1
---

# Phase 3 Plan 2: Key Capture Widget and Card Inline Edit Summary

Key capture widget replacing the raw `<input type="number">` trigger key field, with click-to-record interaction showing human-readable key names; card key badges made click-to-edit with inline recording state and platform-conditional IPC commit.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add key capture signals, keymap import, startCapture, handleCardSetTriggerKey, replace form input | cf66a12 | src/App.tsx |
| 2 | Card key badge click-to-edit with inline recording widget, mutual exclusion guard | c5d018e | src/App.tsx |

## What Was Built

### Key Capture Widget (Creation Form)

Replaces `<input type="number" id="input-macro-trigger-key">` entirely. The widget is a `<div role="button">` with three visual states:

- **Idle, no key:** dim placeholder "Click to set key…", `border-border`
- **Idle, key set:** key name via `resolveKeyName()` in `text-text-main`, `border-border`
- **Recording:** "Press a key…" in `text-accent`, `border-accent` full opacity, `shadow-[0_0_8px_var(--color-accent-glow)]`, cancel ✕ visible

`handleCreateMacro` now reads `newMacroTriggerKeyCode()` directly (integer or null) instead of `parseInt(newMacroTriggerKey())`.

### startCapture Function

Single function shared by form widget and card badge. Uses a module-level `_keyCaptureListener` ref to enforce the single-listener invariant (T-03-08):

- Removes any prior stale listener before attaching
- `document.addEventListener("keydown", onKeyDown, true)` — capture phase prevents key from acting on UI (T-03-06)
- Escape exits without committing; bare modifier keys (Control, Shift, Alt, Meta) are ignored
- `domKeycodeToNative(e.code)` converts e.code string to platform native integer
- `onCleanup` at component scope removes dangling listener on unmount

### Card Key Badge — Click-to-Edit

The existing card key badge (`<span class="...font-mono">Key {N}</span>`) is replaced with:

- **Display state:** `cursor-pointer hover:border-accent/40` affordance; `resolveKeyName(macro.trigger_key!)` instead of raw integer
- **Recording state (editingCardId() === macro.id && editingField() === "key"):** accent border + glow badge showing "Press…" with a cancel ✕

After capture, `handleCardSetTriggerKey` commits:
- **macOS:** `invoke("unbind_hotkey")` then `invoke("bind_hotkey")` (prevents duplicate binding, T-03-07)
- **Windows:** `invoke("set_macro_trigger_key")` with new integer

### App-Level Edit State Signals

`editingCardId: string | null` and `editingField: "key" | "target" | null` track which card and which field is in edit mode. These are at App() scope (not inside the For loop) so they survive re-renders.

Mutual exclusion: clicking the form key capture widget when `editingCardId() !== null` cancels the card edit first, then starts form capture.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Mutual exclusion between form and card capture**
- **Found during:** Task 2
- **Issue:** A user clicking the form key capture widget while a card was in recording mode would attach a second listener (despite the _keyCaptureListener guard) because the card badge and form widget call startCapture independently. The guard removes the stale listener, but the card's editingCardId/editingField would remain set, creating inconsistent UI state.
- **Fix:** Form widget onClick checks `editingCardId() !== null` and clears edit state before calling startCapture. This adds one more read of `editingCardId()` meeting the >=3 count verification criterion.
- **Files modified:** src/App.tsx (form widget onClick handler)
- **Commit:** c5d018e

### Implementation Notes

**Task 1 + JSX coupling:** Task 1's acceptance criteria required `newMacroTriggerKey` count = 0, but the old `<input>` JSX still referenced `newMacroTriggerKey()`. Removing the old signal without replacing the JSX would have caused a TypeScript compile error. The key capture widget JSX was therefore included in the Task 1 commit (along with signal/function changes), and Task 2 focused on the card badge implementation.

## Verification

- `npx tsc --noEmit` exits 0 (no TypeScript errors)
- `cargo build --manifest-path src-tauri/Cargo.toml` exits 0 (no regressions)
- `grep -c 'input-macro-trigger-key' src/App.tsx` → 0
- `grep -c 'resolveKeyName' src/App.tsx` → 3 (import + form widget + card badge)
- `grep -c 'triggerKeyRecording' src/App.tsx` → 4
- `grep -c 'editingCardId' src/App.tsx` → 3 (declaration + card Show + form mutual exclusion guard)
- `grep -c 'handleCardSetTriggerKey' src/App.tsx` → 2 (definition + card badge onClick)

## Known Stubs

None — all data flows are wired. Key names resolve via `resolveKeyName()` from the lookup tables built in plan 03-01. Card edit commits via real IPC calls. Form creation reads the integer directly from signal.

## Threat Flags

None — all surfaces were accounted for in the plan's threat model (T-03-06 through T-03-09):
- T-03-06: `e.preventDefault()` + `e.stopPropagation()` in capture phase — implemented
- T-03-07: macOS unbind before rebind — implemented in `handleCardSetTriggerKey`
- T-03-08: Single-listener invariant via `_keyCaptureListener` ref — implemented
- T-03-09: `navigator.platform` accepted — implemented, not mitigated by design

## Self-Check: PASSED

- src/App.tsx: MODIFIED (key capture widget, card badge, signals, functions)
- Commit cf66a12: FOUND (Task 1)
- Commit c5d018e: FOUND (Task 2)
- `npx tsc --noEmit`: exits 0
- `cargo build`: exits 0
