---
created: 2026-07-24T00:00:00Z
title: Target-app process picker doesn't select the process the user clicked
area: frontend
severity: high
files:
  - src/App.tsx:1443-1465
---

## Problem

The per-card target-app picker (`<select>` at `src/App.tsx:1443-1465`, bound `value={macro.target_app ?? ""}`, `onChange={(e) => handleCardSetTargetApp(macro.id, e.currentTarget.value || null)}`) does not correctly bind to the option the user clicks — reported during the 10-03 checkpoint manual walkthrough (2026-07-24) as "process picker is buggy (doesn't select process you clicked on)". This blocks manual testing of any macro scoped to a single target app, since the app can't be reliably assigned.

Not caused by Phase 10 changes — no 10-01/10-02/10-03 plan touched this block; `apps()`/`list_running_apps` wiring predates this phase (Phase 9 Plan 8 already fixed a related "uncontrolled dropdown" bug by adding the `value={macro.target_app ?? ""}` binding, so this may be a second, distinct defect in the same picker).

## Solution

Not yet diagnosed. Needs a `/gsd-debug` session: check whether `apps()` re-renders/reorders between the dropdown opening and the click landing (causing a stale index to resolve to the wrong `app.identifier`), whether `handleCardSetTargetApp`'s `set_macro_target_app` IPC call round-trips correctly, or whether the native `<select>`'s `onChange` value doesn't match the visually clicked option under SolidJS's reactivity model.
