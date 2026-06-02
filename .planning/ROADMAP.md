# Roadmap: AutoMux

## Milestones

- ✅ **v1.0 MVP** — Phases 1–4 (shipped 2026-05-30)
- 📋 **v1.2.0 Reliability & Polish** — Phases 5–8 (current)
- 📋 **v2.0** — Phase 9+ (planned)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1–4) — SHIPPED 2026-05-30</summary>

- [x] Phase 1: Reliability & Safety (4/4 plans) — completed 2026-05-16
- [x] Phase 2: Cleanup & Credibility (4/4 plans) — completed 2026-05-30
- [x] Phase 3: Macro Setup UX (4/4 plans) — completed 2026-05-30
- [x] Phase 4: CI Hardening (2/2 plans) — completed 2026-05-30

Full phase details: `.planning/milestones/v1.0-ROADMAP.md`

</details>

### 📋 v1.2.0 Reliability & Polish

- [x] **Phase 5: macOS Permissions & Reliability** — Fix false "not granted" detection after Request Access approval, arm CGEventTap on direct System Settings grant, and remove the deprecated block crate dependency (completed 2026-06-02)
- [ ] **Phase 6: Windows Platform Cleanup** — Eliminate all Rust compiler warnings in platform/windows/mod.rs and close the OpenProcess handle leak in the process-list path
- [ ] **Phase 7: CI Pipeline Hardening** — Fix updater JSON signature upload, replace npm install with npm ci, and harden binary artifact discovery against tauri-action renames
- [ ] **Phase 8: Safety & Error Surface** — Eliminate the REGISTRY deadlock risk in flush_held_inputs and surface auto-save failures to the user in the UI

### 📋 v2.0 (Planned)

- [ ] Phase 9: Auto-Updater — in-app update mechanism with signing and notarization prerequisites (DIST-01, DIST-02)

## Phase Details

### Phase 5: macOS Permissions & Reliability
**Goal**: macOS accessibility permission detection is accurate and CGEventTap arms immediately on any grant path — no false negatives, no restart required
**Depends on**: Nothing (independent macOS layer work)
**Requirements**: PERM-01, RELY-06, BUILD-02
**Success Criteria** (what must be TRUE):
  1. After clicking Request Access and approving the OS prompt, the app immediately reflects "granted" — the false "not granted" state never persists
  2. A user who grants accessibility directly in System Settings (without ever clicking Request Access) sees macros activate without restarting the app
  3. The Rust build produces no deprecation warnings related to block v0.1.6 on macOS — build output is clean
**Plans**: 2 plans
Plans:
- [x] 05-01-PLAN.md — Arm CGEventTap in check_accessibility + remove cocoa dep (RELY-06, BUILD-02)
- [x] 05-02-PLAN.md — Add accessibilityPending state machine and 3-branch permission UI (PERM-01)

### Phase 6: Windows Platform Cleanup
**Goal**: The Windows build is warning-free and the process-list path has no handle leak
**Depends on**: Nothing (independent Windows layer work; can run in parallel with Phase 5)
**Requirements**: BUILD-01, MEM-01
**Success Criteria** (what must be TRUE):
  1. `cargo build --target x86_64-pc-windows-msvc` produces zero warnings — no unused import or unused-bool warnings in platform/windows/mod.rs
  2. Repeated calls to list running apps (opening the process picker multiple times) do not accumulate open HANDLE objects — each call closes every OpenProcess handle it opens
**Plans**: TBD

### Phase 7: CI Pipeline Hardening
**Goal**: The release CI workflow is reproducible, complete, and resilient to tooling changes
**Depends on**: Nothing (CI workflow changes only; can run in parallel with Phases 5–6)
**Requirements**: CI-03, CI-04, CI-05
**Success Criteria** (what must be TRUE):
  1. A release run completes without "Signature not found for the updater JSON. Skipping upload…" — the updater JSON signature artifact is successfully uploaded
  2. The CI frontend install step uses `npm ci`, so a lockfile mismatch fails loudly rather than silently pulling a different dependency version
  3. Binary artifact discovery does not silently miss the built artifact if tauri-action changes its output path — either the discovery is path-agnostic or it fails loudly with a useful error
**Plans**: TBD

### Phase 8: Safety & Error Surface
**Goal**: The macOS emergency stop path cannot deadlock and auto-save failures are visible to the user
**Depends on**: Nothing (cross-platform Rust + frontend changes; can run in parallel with earlier phases)
**Requirements**: SAFE-04, ERR-01
**Success Criteria** (what must be TRUE):
  1. Triggering a macro emergency stop (all-held-inputs flush) on macOS does not deadlock — the REGISTRY lock is released before any CGEvent is posted
  2. When a profile auto-save fails, the user sees a visible error indicator in the UI — the failure is never silently swallowed
**Plans**: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Reliability & Safety | v1.0 | 4/4 | Complete | 2026-05-16 |
| 2. Cleanup & Credibility | v1.0 | 4/4 | Complete | 2026-05-30 |
| 3. Macro Setup UX | v1.0 | 4/4 | Complete | 2026-05-30 |
| 4. CI Hardening | v1.0 | 2/2 | Complete | 2026-05-30 |
| 5. macOS Permissions & Reliability | v1.2.0 | 2/2 | Complete    | 2026-06-02 |
| 6. Windows Platform Cleanup | v1.2.0 | 0/TBD | Not started | — |
| 7. CI Pipeline Hardening | v1.2.0 | 0/TBD | Not started | — |
| 8. Safety & Error Surface | v1.2.0 | 0/TBD | Not started | — |
| 9. Auto-Updater | v2.0 | 0/TBD | Reserved | — |
