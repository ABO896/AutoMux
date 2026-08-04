# Roadmap: AutoMux

## Milestones

- ✅ **v1.0 MVP** — Phases 1–4 (shipped 2026-05-30)
- ✅ **v1.2.0 Reliability & Polish** — Phase 5 (shipped 2026-06-02; Phases 6–8 carried to v2.0)
- ✅ **v2.0 Redesign & Platform Excellence** — Phases 6–10 (shipped 2026-08-04)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1–4) — SHIPPED 2026-05-30</summary>

- [x] Phase 1: Reliability & Safety (4/4 plans) — completed 2026-05-16
- [x] Phase 2: Cleanup & Credibility (4/4 plans) — completed 2026-05-30
- [x] Phase 3: Macro Setup UX (4/4 plans) — completed 2026-05-30
- [x] Phase 4: CI Hardening (2/2 plans) — completed 2026-05-30

Full phase details: `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>✅ v1.2.0 Reliability & Polish (Phase 5) — SHIPPED 2026-06-02</summary>

- [x] **Phase 5: macOS Permissions & Reliability** — Fix false "not granted" detection after Request Access approval, arm CGEventTap on direct System Settings grant, and remove the deprecated block crate dependency (completed 2026-06-02)

Note: v1.2.0 Phases 6–8 (Windows cleanup, CI hardening, safety & error surface) were not executed and are folded into v2.0 as Phase 7.

</details>

<details>
<summary>✅ v2.0 Redesign & Platform Excellence (Phases 6–10) — SHIPPED 2026-08-04</summary>

- [x] Phase 6: macOS Tahoe 26 Compatibility (3/3 plans) — completed 2026-06-12
- [x] Phase 7: Carry Work — Platform, CI & Safety (3/3 plans) — completed 2026-06-17
- [x] Phase 8: Hotkey Reliability & Conflict Safety (6/6 plans) — completed 2026-08-04
- [x] Phase 9: Parallel Macro Execution (11/11 plans) — completed 2026-08-04
- [x] Phase 10: UI Redesign & Macro Management (6/6 plans) — completed 2026-08-04

Full phase details: `.planning/milestones/v2.0-ROADMAP.md`

</details>

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Reliability & Safety | v1.0 | 4/4 | Complete | 2026-05-16 |
| 2. Cleanup & Credibility | v1.0 | 4/4 | Complete | 2026-05-30 |
| 3. Macro Setup UX | v1.0 | 4/4 | Complete | 2026-05-30 |
| 4. CI Hardening | v1.0 | 2/2 | Complete | 2026-05-30 |
| 5. macOS Permissions & Reliability | v1.2.0 | 2/2 | Complete | 2026-06-02 |
| 6. macOS Tahoe 26 Compatibility | v2.0 | 3/3 | Complete | 2026-06-12 |
| 7. Carry Work — Platform, CI & Safety | v2.0 | 3/3 | Complete | 2026-06-17 |
| 8. Hotkey Reliability & Conflict Safety | v2.0 | 6/6 | Complete | 2026-08-04 |
| 9. Parallel Macro Execution | v2.0 | 11/11 | Complete | 2026-08-04 |
| 10. UI Redesign & Macro Management | v2.0 | 6/6 | Complete | 2026-08-04 |

## Backlog

### Phase 999.1: Settings page for app customization, including self-exclusion (BACKLOG)

**Goal:** [Captured for future planning]
**Requirements:** TBD
**Plans:** 0 plans

Captured 2026-07-24 during the Phase 10 (10-03) UAT checkpoint: a dedicated settings page where users can tweak app-level behavior, starting with an option to exclude AutoMux itself as a valid macro target — preventing a macro/autoclick from firing on AutoMux's own window/process and causing runaway self-triggering chaos.

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)
