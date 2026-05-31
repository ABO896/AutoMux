# Retrospective: AutoMux

---

## Milestone: v1.0 — MVP

**Shipped:** 2026-05-30
**Phases:** 4 | **Plans:** 14
**Timeline:** 15 days (2026-05-15 → 2026-05-30)

### What Was Built

- Eliminated all 5 production panic paths on the input injection hot path (macOS + Windows)
- CGEventTap inline re-enable on OS timeout — hotkeys survive long sessions
- 3-second frontend poll for accessibility permission grant — no restart required after granting
- Auto-save via Arc<ProfileManager> in StateActor — zero work loss on close
- MacPlatformObserver lifetime fixed via app.manage()
- Key-capture widget + keymap.ts — users configure keys by pressing them
- Running-process picker (NSWorkspace/EnumProcesses) — no manual process-name typing
- GPL-3.0 legal correction: LICENSE file, shields.io badge, README prose
- Stale artifacts removed; .gitignore extended; dead code eliminated
- SHA-pinned GitHub Actions + `--target universal-apple-darwin` + blocking lipo verification

### What Worked

- **Actor model:** Single StateActor + Scheduler task isolation made parallel plan execution safe — no shared-state conflicts across phases
- **Phased execution:** Breaking the milestone into 4 focused phases with clear requirements per phase kept scope creep minimal; each plan had a single responsibility
- **Audit before close:** Running `/gsd-audit-milestone` before completing revealed 2 integration gaps and catalogued tech debt proactively — no surprises at close
- **Gap-closure plan (03-04):** Adding an explicit gap-closure plan after code review caught 3 issues (null keymap guard, macOS persistence regression, Set-key placeholder) before they reached production

### What Was Inefficient

- **REQUIREMENTS.md not updated incrementally:** Traceability table stayed "Pending" for phases 2–4 because phase transitions didn't update the doc; required bulk update at milestone close
- **STATE.md progress counter stale:** Completed plan count drifted from actual (12 vs 14) — not updated after each phase
- **ROADMAP.md progress table:** Phase 2/3/4 remained "Not started" in the progress table throughout execution — diverged from actual status

### Patterns Established

- Gap-closure plan pattern: after code review of a phase, create an explicit plan to close all critical and warning findings before moving on
- SHA pinning with inline tag annotation: `# @v3 tag, SHA locked` comment pattern for GitHub Actions pins
- MacInputProvider option-guard pattern: `fn source() -> Option<CGEventSource>` with `let Some(x) = ... else { return; }` at each call site

### Key Lessons

- Keep REQUIREMENTS.md traceability table updated at each phase transition — batch updates at milestone close are error-prone
- Keep STATE.md progress counters accurate after each plan completion — stale counters create false confidence
- The two-phase dispatch architecture (Scheduler fires ActionReady → StateActor validates → injects) successfully kept input injection fully out of the Scheduler; this invariant held across all 4 phases
- The gap-closure plan pattern (one extra plan after code review per phase) is worth the overhead — it caught real issues that would have shipped

### Cost Observations

- Sessions: ~15 (estimated; one per plan + discussion/planning sessions)
- Notable: Phase 3 (UX) required the most planning iteration — key capture and process picker had subtle cross-platform differences that required a gap-closure plan

---

## Cross-Milestone Trends

| Metric | v1.0 |
|--------|------|
| Phases | 4 |
| Plans | 14 |
| Timeline (days) | 15 |
| Requirements satisfied | 18/18 |
| Tech debt items | 9 |
| Gap-closure plans | 1 (Phase 3) |
