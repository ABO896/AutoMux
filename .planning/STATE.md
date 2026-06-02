---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Redesign & Platform Excellence
status: planning
stopped_at: Milestone v2.0 started — roadmap pending
last_updated: "2026-06-02T00:00:00.000Z"
last_activity: 2026-06-02 -- Milestone v2.0 started
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-02 — milestone v2.0 started)

**Core value:** A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.
**Current focus:** Defining requirements and roadmap for v2.0

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-06-02 — Milestone v2.0 started

```
Progress: [░░░░░░░░░░░░░░░░░░░░] 0% (0/? phases)
```

## Phase Summary

*(Roadmap not yet created — phases defined after roadmap is approved)*

## Accumulated Context

### Key Decisions

- v2.0 is a major version bump: full UI redesign + macOS Tahoe 26 compat + parallel macro execution
- All features must ship on both macOS and Windows — no platform-exclusive fixes
- UI redesign: Apple design language + liquid glass (macOS 26), modern equivalent on Windows, Raycast-inspired layout
- Parallel execution: triggering macro B while macro A runs must not block or cancel macro A — architectural change required
- macOS Tahoe 26 compatibility is the critical path: macros are fully broken on the new OS (CGEventTap regression suspected)
- COMPAT investigation should happen first before assuming the architecture for EXEC-01/02 — the Tahoe 26 fix may require platform changes that affect parallel execution design
- v1.2.0 Phase 5 completed: PERM-01, RELY-06, BUILD-02 — phases 6–8 not executed, carried to v2.0

### Known Constraints

- COMPAT-01/02: Must work within Tauri and CGEvent API surface — OS enforces permission model
- UI-01: Liquid glass requires macOS 26+ APIs — cannot backport to Monterey/Ventura/Sonoma/Sequoia
- UI-04: UI redesign must not increase idle memory/CPU overhead — keep AutoMux lightweight
- SAFE-04: Lock ordering fix (release REGISTRY before CGEvent dispatch) is subtle — requires careful audit

## Deferred Items

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Distribution | Auto-updater (DIST-01) | v3 scope | v1.2.0 roadmap creation |
| Distribution | macOS notarization (DIST-02) | v3 scope | v1.2.0 roadmap creation |
| UX | NamedKey schema migration (UX-04) | future | v1.2.0 roadmap creation |
| UX | Cross-platform profile portability (UX-05) | future | v1.2.0 roadmap creation |

## Session Continuity

Last session: 2026-06-02
Stopped at: Milestone v2.0 requirements defined — roadmap pending
Next: Spawn roadmapper to create ROADMAP.md
