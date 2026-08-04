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

## Milestone: v2.0 — Redesign & Platform Excellence

**Shipped:** 2026-08-04
**Phases:** 5 (6-10) | **Plans:** 31 | **Commits:** 224 (Phase 6 start → ship)
**Timeline:** ~61 days (2026-06-04 → 2026-08-04)

### What Was Built

- macOS Tahoe 26 compatibility: live CGEventTap probe replacing a stale cached permission check, plus a CFRunLoop stop/reinit fix for a UAF crash in the re-enable callback
- Platform carry-work: Windows compiler warnings and `OpenProcess` handle leak eliminated, CI hardened (`npm ci`, updater signature upload, resilient artifact discovery), macOS emergency-stop deadlock risk verified safe, Input Monitoring + TCC identity-change detection added
- Hotkey reliability: full key-range binding (0-9, F1-F12, all modifiers), conflict detection with explicit reassignment prompts, same-input overlap warnings, and verified/communicated system-wide (global) hotkey behavior on both platforms
- Parallel macro execution: StateActor/Scheduler two-phase dispatch redesign so triggering one macro never blocks, queues, or cancels another — required 3 rounds of gap-closure (stuck-input HoldRelease bypass, hotkey-rebind rollback safety, dual-registry double-dispatch) before the architecture was genuinely sound
- Macro management + UI redesign: delete/edit for existing macros, unambiguous action-type labels, an Apple-inspired sidebar layout with light/dark/OS-follow theming — liquid-glass/vibrancy was pursued but ultimately descoped (Tauri's WKWebView doesn't composite `backdrop-filter`)
- Retroactive per-phase security threat registers (`*-SECURITY.md`) for all 5 phases, 0 open threats at the `high` block-on threshold

### What Worked

- **Independent re-verification against source, not prose:** Multiple verification passes (09-VERIFICATION.md's re-verification, this session's Phase 8 refresh) explicitly re-read implementation source rather than trusting SUMMARY.md/prior-VERIFICATION.md claims — this is what caught that Phase 9's HOTKEY_BINDINGS consolidation hadn't regressed Phase 8's conflict matching, with concrete line-number evidence rather than assumption.
- **Regression tests pinning each gap-closure fix:** Every Blocker-severity fix (HoldRelease bypass, hotkey-rebind rejection, dual-registry consolidation) shipped with a dedicated test that fails if the fix regresses — this is why 3 rounds of gap-closure converged instead of oscillating.
- **User-reported device bugs → root-caused, not patched around:** The 100ms-hotkey-lag report led to finding and fixing a genuine architectural issue (CGEventTap subscribing to unused high-frequency events) rather than a superficial workaround. Same pattern for the process-picker bug — traced to a SolidJS `<For>` reference-diffing + focus-refetch race, not just "add a key prop."
- **Debug-session-first for ambiguous bugs:** Both major post-milestone bugs (process picker, and the historical G-09-1a/b/c cluster) went through a structured debug protocol (symptom gathering → evidence → hypothesis → regression test) rather than a guessed fix — this caught a mid-session regression from an incorrect first fix attempt (`reconcile()` misapplied to a plain `createSignal`) before it shipped.

### What Was Inefficient

- **Verification staleness went unnoticed for months:** Phase 6's verification report went stale and Phase 7 never got one at all, and neither was caught until this milestone-close session's `init.manager` readiness check — ~2 months after those phases were marked "complete" in ROADMAP.md. The gap only surfaced because milestone completion runs a canonical cross-check that day-to-day phase transitions don't.
- **Non-canonical status strings silently break tooling:** Phase 10's `10-VERIFICATION.md` used `status: complete` instead of the canonical `passed` — a one-word typo that made the phase invisible to `init.manager`'s readiness aggregation until manually diagnosed and fixed at milestone close.
- **`phase.complete`'s STATE.md auto-update assumes strictly sequential phases:** When Phase 9 (or Phase 8) completed out of order relative to an already-shipped Phase 10, the tool reset `Current Position` to "next phase, ready to plan" — pointing at a phase that had already shipped days earlier. Had to be manually corrected twice in this session alone.
- **Debug-session bookkeeping drift:** Three Phase 9 gap-closure debug sessions (G-09-1a/b/c) and one Phase 10 session (glass-blur) were genuinely resolved and re-verified but never had their `status` field updated or file moved to `debug/resolved/` — only surfaced by the pre-milestone-close artifact audit, not by the individual phase-completion flows that should have caught it.
- **REQUIREMENTS.md/ROADMAP.md checklist vs. traceability-table inconsistency:** EXEC-01/EXEC-02 stayed unchecked in the requirements checklist for weeks while the traceability table already said "Complete (source-level)" — a documentation format with two sources of truth that can silently diverge.

### Patterns Established

- **Retroactive-STRIDE security compilation:** When `<threat_model>` blocks exist at plan-time but no dedicated audit ran, build the `*-SECURITY.md` register directly from those blocks plus the corresponding VERIFICATION.md's independent source-level confirmations, rather than re-running a full security audit from scratch — sound when severities top out at `medium` and `asvs_level` is 1.
- **UAT round-numbering for re-tested device checkpoints:** When a UAT session's device tests fail, get gap-closure plans, and need retesting, keep the original "Round 1" results (with `superseded_by` annotations) rather than deleting them — preserves the improvement narrative while still letting automated `phase uat-passed` tooling see a clean, unambiguous pass.
- **Debug session → resolved/ with `resolved_by` frontmatter:** Every genuinely-closed debug session should record which plan/commit closed it and get moved to `debug/resolved/` at the moment of closure, not deferred to the next audit that happens to notice.

### Key Lessons

- A phase marked "complete" in ROADMAP.md is not the same claim as "verification passed" — the two can diverge for months without anyone noticing unless something runs the canonical cross-check (`init.manager`, `phase uat-passed --require-verification`). Worth running that check periodically, not just at milestone boundaries.
- When automation writes a "next phase" pointer, always sanity-check it against reality before trusting it, especially in a project where phases can be worked out of numeric order (this milestone had Phase 10 finish before Phase 8/9's device verification did).
- A single non-canonical enum value in a status field can make a fully-shipped phase look unverified to tooling — canonical status vocabularies should be validated at write time, not just at read time.
- "The functionality has since been exercised by later passing phases" is a legitimate reason to defer old device-verification debt (used for Phase 6/`macros-dont-fire-post-crash` at this milestone's close) — but it should be recorded explicitly as a deferred item with its reasoning, not silently dropped.

### Cost Observations

- Model mix: primarily Sonnet for orchestration and subagent work (debugger, verifier, security compilation) this session; Opus/Sonnet split for planner/checker roles in earlier phases per `gsd-planner`/`gsd-plan-checker` model-profile defaults
- Sessions: at least 2 major sessions for the final verification+close push (one hit a session rate limit mid-verification and resumed cleanly via agent continuation)
- Notable: the milestone-close session itself (verification refresh + UAT + security + audit reconciliation + PROJECT.md evolution) took roughly as long as executing 2-3 ordinary plans — canonicalizing months of accumulated documentation drift is real, non-trivial work, not a formality

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

| Metric | v2.0 |
|--------|------|
| Phases | 5 (6-10) |
| Plans | 31 |
| Timeline (days) | ~61 |
| Requirements satisfied | 24/25 (COMPAT-02 carried forward, device-verification only) |
| Tech debt items closed at milestone open | 2 (LoadProfile data-loss, Windows injected-keystroke self-trigger — found and fixed as quick tasks) |
| Gap-closure plans | 9 (Phase 9: 04, 05, 06, 07, 08, 09, 10, 11; Phase 8: none needed) |
| Bookkeeping/tooling gaps found only at milestone close | 5 (Phase 6 stale verification, Phase 7 missing verification, Phase 10 non-canonical status string, 4 unreconciled debug sessions, `phase.complete`'s sequential-only STATE.md update) |
