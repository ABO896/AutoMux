---
phase: 10
slug: ui-redesign-macro-management
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-23
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust backend, inline `#[cfg(test)] mod tests` blocks — see `state/mod.rs:971-1382`) |
| **Config file** | none — Cargo's built-in test harness, no separate config |
| **Quick run command** | `cargo test -p automux update_macro` (once new tests below are added) |
| **Full suite command** | `cargo test` (backend) + `npx tsc --noEmit` (frontend type-check, existing gate) + manual visual QA against `10-UI-SPEC.md`'s Verification Checklist |
| **Estimated runtime** | ~10 seconds (backend) |

**Frontend:** No test framework installed. UI changes validated manually via device smoke tests against `10-UI-SPEC.md`'s own Verification Checklist — this was already the established mechanism pre-phase (per `CONVENTIONS.md`: "No frontend test framework detected").

---

## Sampling Rate

- **After every task commit:** `cargo test -p automux <new test name>` (fast, scoped) + `npx tsc --noEmit`
- **After every plan wave:** `cargo test` (full backend suite) + `npx tsc --noEmit` + spot-check against the relevant rows of UI-SPEC's Verification Checklist
- **Before `/gsd-verify-work`:** Full backend suite green + `npx tsc --noEmit` clean + UI-04 perf measurement documented with before/after numbers + full UI-SPEC Verification Checklist walked
- **Max feedback latency:** 10s

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| {N}-01-01 | 01 | 1 | REQ-{XX} | T-{N}-01 / — | {expected secure behavior or "N/A"} | unit | `{command}` | ✅ / ❌ W0 | ⬜ pending |

*Seeded at plan-phase before task IDs exist — populate real rows once PLAN.md tasks are assigned, using the Phase Requirements → Test Map below as the source mapping.*

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Phase Requirements → Test Map (from RESEARCH.md, pending Task ID assignment)

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UX-09 | `Intent::UpdateMacro` atomically updates name/sequence/mode/target/key on success | unit | `cargo test update_macro_applies_all_fields` | ❌ Wave 0 |
| UX-09 | `Intent::UpdateMacro` rejects on trigger-key conflict WITHOUT mutating any field (atomicity) | unit | `cargo test update_macro_conflict_no_partial_mutation` | ❌ Wave 0 |
| UX-09 | Edited macro persists across restart | integration (existing pattern) | `cargo test profile_backwards_compat`-style round-trip via `ProfileManager` | ❌ Wave 0 (new test, same style as existing) |
| UX-10 | Input selector shows exactly 4 labeled options, Mode selector exactly 2 | manual/visual | N/A — no frontend test framework; covered by UI-SPEC's own Verification Checklist item | manual-only |
| UI-01/02/03 | Visual redesign correctness (translucency, sidebar, typography) | manual/visual | N/A | manual-only — `10-UI-SPEC.md`'s Verification Checklist serves as acceptance criteria |
| UI-04 | No measurable idle CPU/GPU overhead vs v1.2.0 | manual measurement | N/A (Activity Monitor / Task Manager, documented protocol) | manual-only — no perf-profiling tooling in CI |
| UX-08 | Delete restyle — no regression to existing `remove_macro` IPC behavior | existing coverage | (no new backend logic — backend half already covered by Phase 9) | ✓ (unchanged) |

---

## Wave 0 Requirements

- [ ] `src-tauri/src/state/mod.rs` — new `#[cfg(test)] mod tests` additions (same file, following the existing inline-test convention) covering: successful `UpdateMacro` atomic apply, conflict-rejection-without-mutation, and a persistence round-trip
- [ ] No frontend test framework exists and none is being added this phase — the Wave 0 gap for UI-01/02/03/UX-10 is "none, by design" (manual QA via UI-SPEC's own checklist is the sampling mechanism)

*(3 new backend unit tests needed, same file, same style as existing `hold_release_bypasses_gates`/`set_trigger_key_rejects_conflict_without_coercion` tests.)*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Input selector shows exactly 4 labeled options (Left Click/Right Click/Hold/Key Press), Mode selector exactly 2 | UX-10 | No frontend test framework | Open macro creation form, verify labels match exactly, no ambiguous/unlabeled options |
| Visual redesign correctness (translucency, layout, typography, dark/light theme) | UI-01, UI-02, UI-03 | Pure visual-design phase; no frontend test framework | Walk `10-UI-SPEC.md`'s 20-item Verification Checklist on both macOS and Windows |
| No measurable idle CPU/GPU overhead vs v1.2.0 | UI-04 | No automated perf-profiling tooling in this project's CI | Activity Monitor (macOS) / Task Manager (Windows) idle measurement before/after, documented protocol from RESEARCH.md |
| Edited macro persists across restart, visible in UI | UX-09 | End-to-end UI + restart flow | Edit a macro's fields, restart app, verify changes persisted and displayed |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
