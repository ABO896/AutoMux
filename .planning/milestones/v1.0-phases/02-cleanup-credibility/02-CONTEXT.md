# Phase 2: Cleanup & Credibility - Context

**Gathered:** 2026-05-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix what the repository says (README license and appeal), remove what shouldn't be there (dead code, stale artifacts), close the MacPlatformObserver resource leak, and correct the hardcoded version string. No new features are added in this phase — only correctness and cleanliness fixes.

**Requirements in scope:** README-01, README-02, README-03, SAFE-02, AUDIT-01, AUDIT-02, AUDIT-03

**Not in scope:** Key capture widget (Phase 3), process picker (Phase 3), CI hardening (Phase 4). Do not expand scope.

</domain>

<decisions>
## Implementation Decisions

### README Fixes (README-01, README-02, README-03)

- **D-01:** Change "MIT License" to "GNU GPL v3" (or "GNU General Public License v3") everywhere it appears in README prose (README-01).
- **D-02:** Add a `GPL-3.0-only` shields.io badge to the README. Use the shields.io SPDX identifier `GPL-3.0-only`. Remove or replace the existing broken license badge (README-02).
- **D-03:** README-03 scope is a **polish + appeal** pass — not a full rewrite. No new top-level sections added. The following four areas are in scope:
  - **Opening paragraph:** Tighten the description — what AutoMux is, who it's for, why it exists.
  - **Feature list accuracy:** Ensure listed features match what's actually implemented. Do NOT mention key capture or process picker (those are Phase 3 features not yet built).
  - **Platform info:** Clearly call out macOS and Windows support (platform compatibility is a selling point).
  - **Badges audit:** Audit all badges for accuracy; fix stale links; remove broken badges.
- **D-04:** No screenshot or demo GIF in Phase 2. Screenshot section can be added after a release build.

### MacPlatformObserver Leak Fix (SAFE-02)

- **D-05:** Store `MacPlatformObserver` in Tauri managed state using `app.manage()` in `src-tauri/src/lib.rs`. The observer must live for the full application lifetime so `stop_observing` is reachable (not dead code).
- **D-06:** No new IPC command. The goal is correct resource lifetime, not external exposure of `stop_observing`. SAFE-02 requires the token to be stored — nothing more.

### Dead Code & Artifact Removal (AUDIT-01, AUDIT-02, AUDIT-03)

- **D-07:** Remove `implementation_plan.md` from the repository root (AUDIT-01). Planner's choice whether to move it to `archive/` or delete outright.
- **D-08:** Dead code audit scope is **source files only** — scan Rust (`src-tauri/src/`) and SolidJS (`src/`) for unused source files, unreachable code, and dead functions. Do NOT audit Cargo dependencies in this phase (removing `cocoa` or `objc` 0.2 is higher risk and deferred).
- **D-09:** `.gitignore` additions (AUDIT-03): add coverage for `src-tauri/gen/`, `*.log` / debug temp files, and OS artifacts (`.DS_Store`, `Thumbs.db`). Researcher should inspect the current `.gitignore` and identify any additional gaps.

### Version String Fix

- **D-10:** Replace the hardcoded `v1.0.0` version string at `src/App.tsx:291` with a runtime call to `getVersion()` from `@tauri-apps/api/app`. This keeps the displayed version in sync with `tauri.conf.json` and `package.json` automatically.

### Claude's Discretion

- Exact README section order and specific wording for the appeal/clarity improvements.
- Whether to move `implementation_plan.md` to `archive/` or delete it (either is acceptable).
- Exact `.gitignore` entry format, ordering, and whether to add comments grouping new entries.
- What constitutes "dead" in the TypeScript source — the TypeScript compiler's `noUnusedLocals: true` and `noUnusedParameters: true` flags already surface these; researcher should run the compiler to enumerate them.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — README-01, README-02, README-03, SAFE-02, AUDIT-01, AUDIT-02, AUDIT-03 are Phase 2 scope
- `.planning/ROADMAP.md` — Phase 2 success criteria (4 acceptance tests that must pass)

### Known Issue Locations (Rust)
- `src-tauri/src/platform/macos/observer.rs` lines 386–496 — MacPlatformObserver struct, `_observer_token` raw pointer, `stop_observing` implementation (SAFE-02 fix target)
- `src-tauri/src/lib.rs` lines 67–73 — where observer is currently dropped at end of setup closure; `app.manage()` call goes here (SAFE-02)

### Known Issue Locations (Frontend)
- `src/App.tsx:291` — hardcoded `v1.0.0`; replace with `getVersion()` from `@tauri-apps/api/app`

### Known Issue Locations (Repository)
- `implementation_plan.md` — stale planning artifact in repo root; remove (AUDIT-01)

### Codebase Analysis
- `.planning/codebase/CONCERNS.md` — precise file/line references for every known issue; read before planning to avoid hunting
- `.planning/codebase/STRUCTURE.md` — directory layout and where files belong
- `.planning/codebase/CONVENTIONS.md` — coding conventions for both Rust and TypeScript layers

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `app.manage()` pattern — Tauri managed state; already used in the app for `StateManager`; the same mechanism applies to `MacPlatformObserver` (SAFE-02)
- `@tauri-apps/api/app` module — `getVersion()` is already available as a Tauri API; no new dependency needed for the version string fix

### Established Patterns
- `#[cfg(target_os = "macos")]` gates — all macOS-only code uses this; SAFE-02 fix must be gated to macOS only
- TypeScript `noUnusedLocals: true` / `noUnusedParameters: true` — compiler will surface dead TS code automatically; run `npm run build` or `tsc --noEmit` to enumerate
- `cargo check` / `cargo clippy` — will surface unused Rust imports and dead code warnings; researcher should run these to enumerate AUDIT-02 candidates

### Integration Points
- `src-tauri/src/lib.rs::run()` — the wiring point for `app.manage()`; MacPlatformObserver must be stored before the app enters its event loop
- `src/index.tsx` — frontend mount point; `getVersion()` call in `App.tsx` will execute after mount; ensure async init pattern is followed (already established in the codebase)

</code_context>

<specifics>
## Specific Ideas

- The `getVersion()` fix (D-10) is a small async call — follow the existing `createEffect` + `invoke` async pattern already established in `App.tsx`. Import from `@tauri-apps/api/app`.
- For SAFE-02, researcher must confirm the Tauri managed state ownership model: `app.manage()` requires `T: Send + Sync + 'static`. Verify `MacPlatformObserver` can satisfy these bounds or determine what wrapping (e.g., `Mutex`) is needed.
- The TypeScript dead code audit is aided by the strict compiler config — run `tsc --noEmit` and fix `noUnusedLocals` errors first before manual review.

</specifics>

<deferred>
## Deferred Ideas

- **Cargo dep audit** (`cocoa` 0.26, `objc` 0.2 removal) — useful but higher risk; defer to a dedicated cleanup phase or Phase 4.
- **Windows hook initialization error surfacing** — noted in CONCERNS.md as fragile; not in Phase 2 scope.
- **Duplicate trigger key validation** — known bug from CONCERNS.md; out of Phase 2 scope.

</deferred>

---

*Phase: 2-Cleanup & Credibility*
*Context gathered: 2026-05-19*
