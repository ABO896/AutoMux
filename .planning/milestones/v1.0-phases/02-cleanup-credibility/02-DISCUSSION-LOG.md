# Phase 2: Cleanup & Credibility - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-19
**Phase:** 02-cleanup-credibility
**Areas discussed:** README depth, Observer fix approach, Dead code depth, Version string bug

---

## README depth

| Option | Description | Selected |
|--------|-------------|----------|
| License fixes only | Fix MIT→GPL prose, add correct badge. No structural changes. | |
| Polish + appeal | Improve opening paragraph, feature list accuracy, platform clarity, badges audit. | ✓ |
| Full first-impressions revamp | Restructure for new-user landing: add Features, Installation, Usage sections, screenshot. | |

**User's choice:** Polish + appeal

### Follow-up: Which sections to touch?

| Option | Selected |
|--------|----------|
| Opening paragraph | ✓ |
| Feature list accuracy | ✓ |
| Platform badges/info | ✓ |
| Badges cleanup | ✓ |

**User's choice:** All four sections.

### Follow-up: Screenshot or demo GIF?

| Option | Selected |
|--------|----------|
| No — skip for now | ✓ |
| Placeholder only | |
| Yes, add a screenshot | |

**User's choice:** No screenshot in Phase 2.

---

## Observer fix approach

| Option | Description | Selected |
|--------|-------------|----------|
| Tauri managed state | Store via app.manage() in lib.rs — full app lifetime, stop_observing callable from managed state. | ✓ |
| Implement Drop | Add Drop impl that calls stop_observing(). Simpler; stop_observing not externally callable. | |
| OnceLock static | Store in static OnceLock alongside STATE_TX and TAP_INITIALIZED. | |

**User's choice:** Tauri managed state.

### Follow-up: Add IPC command to expose stop_observing?

| Option | Selected |
|--------|----------|
| Just keep alive — no IPC | ✓ |
| Add IPC command | |

**User's choice:** No IPC command — SAFE-02 only requires the token to be stored.

---

## Dead code depth

| Option | Description | Selected |
|--------|-------------|----------|
| Source files only | Scan Rust + SolidJS source for unused files/code. Skip Cargo dep audit. | ✓ |
| Source files + Cargo deps | Also run cargo tree to verify cocoa and objc 0.2 usage; remove if unused. | |
| Source files + deps + archive | Also evaluate archive/v1-production/ removal. | |

**User's choice:** Source files only — Cargo dep removal deferred.

### Follow-up: .gitignore additions

| Option | Selected |
|--------|----------|
| src-tauri/gen/ | ✓ |
| *.log / debug temp files | ✓ |
| OS artifacts (.DS_Store, Thumbs.db) | ✓ |

**User's choice:** All three .gitignore additions.

---

## Version string bug

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — Phase 2 | Cosmetic cleanup; one-line fix using getVersion() from @tauri-apps/api/app. | ✓ |
| No — defer to Phase 3 | Phase 3 touches App.tsx anyway; batch it there. | |

**User's choice:** Fix in Phase 2 — it's a cleanup issue, not a Phase 3 UX concern.

---

## Claude's Discretion

- Exact README section order and wording for the appeal/clarity improvements
- Whether to move implementation_plan.md to archive/ or delete it
- Exact .gitignore entry format, ordering, and comment grouping
- What constitutes "dead" in the TypeScript source (compiler flags will surface these)

## Deferred Ideas

- Cargo dep audit (cocoa 0.26, objc 0.2 removal) — higher risk; deferred from this phase
- Windows hook initialization error surfacing — fragile area noted in CONCERNS.md; out of Phase 2 scope
- Duplicate trigger key validation — known bug; out of Phase 2 scope
