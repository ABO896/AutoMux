---
phase: 2
slug: cleanup-credibility
status: draft
nyquist_compliant: false
wave_0_complete: true
created: 2026-05-19
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust inline tests); no frontend test framework |
| **Config file** | Inline `#[cfg(test)]` for Rust; none for frontend |
| **Quick run command** | `cargo check --manifest-path src-tauri/Cargo.toml` |
| **Full suite command** | `cargo test --manifest-path src-tauri/Cargo.toml && npx tsc --noEmit` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo check --manifest-path src-tauri/Cargo.toml`
- **After every plan wave:** Run `cargo test --manifest-path src-tauri/Cargo.toml && npx tsc --noEmit`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|--------|
| 02-01-01 | 01 | 1 | README-01, README-02 | — | N/A | grep | `grep -c "MIT License" README.md` (expect 0) + `grep -c "GPL" README.md` (expect ≥1) | ⬜ pending |
| 02-01-02 | 01 | 1 | README-03 | — | N/A | manual + grep | Manual review; `grep -c "Key Capture\|Process Picker" README.md` (expect 0) | ⬜ pending |
| 02-02-01 | 02 | 1 | AUDIT-01, AUDIT-03 | — | N/A | shell + grep | `test ! -f implementation_plan.md && echo PASS` + `grep -E "Thumbs.db\|\.log" .gitignore` | ⬜ pending |
| 02-02-02 | 02 | 1 | AUDIT-02 | — | N/A | compile | `RUSTFLAGS="-Wdead-code" cargo check --manifest-path src-tauri/Cargo.toml 2>&1 \| grep -c "warning:"` (expect 0) + `npx tsc --noEmit` | ⬜ pending |
| 02-03-01 | 03 | 1 | SAFE-02 | — | Observer stored for app lifetime; stop_observing reachable | compile + grep | `cargo check --manifest-path src-tauri/Cargo.toml` + `grep -n "app.manage(observer)" src-tauri/src/lib.rs` | ⬜ pending |
| 02-04-01 | 04 | 1 | README-03 | — | N/A | compile + grep | `npx tsc --noEmit` + `grep -n "getVersion" src/App.tsx` | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements — no new test files needed. Phase 2 is correctness/cleanup; behavioral tests are not required beyond compile-time checks.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| README clarity pass | README-03 | Subjective quality judgment — opening paragraph, feature list accuracy | Read README.md; confirm opening paragraph is tight, feature list matches implemented features, platform support is clear, no Key Capture or Process Picker mentioned |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
