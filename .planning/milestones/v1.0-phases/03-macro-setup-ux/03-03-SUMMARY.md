---
phase: 03-macro-setup-ux
plan: "03"
subsystem: frontend
tags: [process-picker, solidjs, ux, tauri, select, card-edit]
dependency_graph:
  requires:
    - src-tauri/src/ipc/mod.rs list_running_apps — provided by plan 03-01
    - src-tauri/src/ipc/mod.rs set_macro_target_app — existing command
    - editingCardId / editingField signals — provided by plan 03-02
  provides:
    - Process picker <select id="select-macro-target"> replacing input#input-macro-target
    - RunningApp interface declaration
    - apps / appsLoading / appsError signals
    - handlePickerFocus with double-fetch guard
    - handleCardSetTargetApp with null-clear for Global targeting
    - Card target display converted to click-to-edit inline picker with Escape cancel
  affects:
    - src/App.tsx
tech_stack:
  added: []
  patterns:
    - onFocus fetch pattern with loading/error/data state (mirrors refreshProfiles)
    - Double-fetch guard via if (appsLoading()) return at top of handler
    - Show component for conditional option groups inside <select>
    - Card click-to-edit with Show fallback switching between display span and inline select
    - Escape key onKeyDown on inline select cancels edit without committing
    - Empty string → null conversion before set_macro_target_app IPC (T-03-13)
key_files:
  created: []
  modified:
    - src/App.tsx
decisions:
  - Both Task 1 and Task 2 committed in single commit — noUnusedLocals: true enforced by tsconfig prevents Task 1 signals/functions from compiling clean before Task 2 JSX wires them (identical coupling issue documented in 03-02 SUMMARY)
  - handlePickerFocus guard uses if (appsLoading()) return only — no stale-cache check — to comply with D-06 (always re-fetch on picker open)
  - Card onClick calls handlePickerFocus() before the select's onFocus fires so apps load immediately on card edit activation
metrics:
  duration_minutes: 2
  completed_date: "2026-05-30"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 1
---

# Phase 3 Plan 3: Process Picker Select and Card Target Inline Edit Summary

Process picker `<select>` replacing the raw text input for target app, with on-focus list_running_apps fetch showing loading/error/data states; card target display made click-to-edit with auto-commit on selection and Escape cancel.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add RunningApp interface, picker signals, handlePickerFocus, handleCardSetTargetApp | f2973a0 | src/App.tsx |
| 2 | Replace target app input with process picker select; make card target display click-to-edit | f2973a0 | src/App.tsx |

Note: Tasks 1 and 2 committed together in f2973a0 — same coupling issue as plan 03-02 (noUnusedLocals: true prevents Task 1 from compiling before Task 2 wires the signals into JSX).

## What Was Built

### RunningApp Interface

Added after `ProfileData` in the type block:
```ts
interface RunningApp {
  display_name: string;
  identifier: string;
}
```

### Process Picker Signals

Three signals added in the New Macro Form State section:
- `apps: RunningApp[]` — list populated from IPC on picker open
- `appsLoading: boolean` — true while fetch in progress (drives disabled Loading… option)
- `appsError: boolean` — true if fetch threw (drives disabled Failed to load apps option)

### handlePickerFocus

Mirrors `refreshProfiles` pattern with loading/error state and a double-fetch guard:
- First line: `if (appsLoading()) return;` — prevents duplicate in-flight request (T-03-12)
- Sets `appsLoading(true)` and `appsError(false)` before fetch
- On success: `setApps(result)`
- On error: `setAppsError(true)`
- `finally`: `setAppsLoading(false)`

### handleCardSetTargetApp

Mirrors `handleCardSetTriggerKey` shape from plan 03-02:
- `targetApp || null` converts empty string to null before IPC (T-03-13 — Global targeting)
- On success: clears editingCardId and editingField
- On error: console.error

### Creation Form Process Picker

Replaces `<input id="input-macro-target" type="text">` with:
```jsx
<select id="select-macro-target" value={newMacroTarget()} onChange={...} onFocus={handlePickerFocus}>
  <option value="">🌐 Global (no target)</option>
  <Show when={appsLoading()}><option disabled>Loading…</option></Show>
  <Show when={appsError()}><option disabled>Failed to load apps</option></Show>
  <For each={apps()}>{(app) => <option value={app.identifier}>{app.display_name} ({app.identifier})</option>}</For>
</select>
```

### Card Target Click-to-Edit

Target display section replaced with a Show/fallback conditional:

**Display state (fallback):** `<span class="font-mono cursor-pointer border border-transparent hover:border-accent/40 rounded px-1">` with onClick that:
- Sets editingCardId and editingField to activate edit mode
- Calls `handlePickerFocus()` to start the apps fetch immediately

**Edit state (Show when):** Inline `<select>` with:
- Same option structure as creation form picker (Global + loading + error + For list)
- `onChange` calls `handleCardSetTargetApp(macro.id, value || null)` — auto-commit on selection (D-08)
- `onKeyDown` Escape: clears editingCardId and editingField without committing
- `onFocus={handlePickerFocus}` for additional safety (guard prevents double-fetch)

## Deviations from Plan

### Implementation Notes

**Task 1 + JSX coupling (same as plan 03-02):** Task 1 acceptance criteria requires `npx tsc --noEmit` to exit 0, but `noUnusedLocals: true` in tsconfig causes TypeScript to error on the three new signals and two new functions until Task 2 wires them into JSX. Both tasks were committed together in f2973a0. The plan's task structure documents the logical split; the commit captures the functional whole.

## Verification

- `npx tsc --noEmit` exits 0 (no TypeScript errors)
- `cargo build --manifest-path src-tauri/Cargo.toml` exits 0 (no regressions)
- `grep -c 'input-macro-target' src/App.tsx` → 0 (old text input removed)
- `grep -c 'select-macro-target' src/App.tsx` → 1 (new select present)
- `grep -c 'interface RunningApp' src/App.tsx` → 1
- `grep -c 'handlePickerFocus' src/App.tsx` → 4 (definition + form onFocus + card onClick + card select onFocus)
- `grep -c 'handleCardSetTargetApp' src/App.tsx` → 2 (definition + card onChange)
- `grep -c 'Global (no target)' src/App.tsx` → 2 (form + card picker)
- `grep -c 'Loading…' src/App.tsx` → 2 (form + card picker)
- `grep -c 'Failed to load apps' src/App.tsx` → 2 (form + card picker)
- `grep -c 'if (appsLoading()) return' src/App.tsx` → 1 (double-fetch guard)

## Known Stubs

None — all data flows are wired. Process picker options populate from live IPC call to list_running_apps. Card edit commits via real set_macro_target_app IPC. No hardcoded placeholder values flow to rendered output.

## Threat Flags

None — all surfaces were accounted for in the plan's threat model (T-03-10 through T-03-13):
- T-03-11: Picker value comes from select options populated from IPC — no free-form text entry
- T-03-12: Double-fetch guard `if (appsLoading()) return` — implemented
- T-03-13: `targetApp || null` converts empty string to null — implemented

## Self-Check: PASSED

- src/App.tsx: MODIFIED (RunningApp interface, signals, handlePickerFocus, handleCardSetTargetApp, form picker, card inline edit)
- Commit f2973a0: FOUND (Tasks 1 + 2)
- `npx tsc --noEmit`: exits 0
- `cargo build`: exits 0
