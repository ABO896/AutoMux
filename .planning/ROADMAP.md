# Roadmap: AutoMux

## Milestones

- ✅ **v1.0 MVP** — Phases 1–4 (shipped 2026-05-30)
- ✅ **v1.2.0 Reliability & Polish** — Phase 5 (shipped 2026-06-02; Phases 6–8 carried to v2.0)
- 📋 **v2.0 Redesign & Platform Excellence** — Phases 6–9 (current)

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

### 📋 v2.0 Redesign & Platform Excellence

- [ ] **Phase 6: macOS Tahoe 26 Compatibility** *(planned — 2 plans)* — Investigate and fix CGEventTap input injection and permissions detection on macOS 26 Tahoe; ensure the app launches without crashes or entitlement errors
- [ ] **Phase 7: Carry Work — Platform, CI & Safety** — Eliminate Windows compiler warnings and the OpenProcess handle leak; harden the CI release pipeline; fix the REGISTRY deadlock risk and surface auto-save failures in the UI
- [ ] **Phase 8: Parallel Macro Execution** — Redesign the StateActor/Scheduler execution model so multiple macros run concurrently on both macOS and Windows
- [ ] **Phase 9: UI Redesign & Macro Management** — Ship the full Apple/liquid-glass UI redesign for macOS and a modern equivalent for Windows; add macro delete and edit capabilities with clear action-type labeling

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

### Phase 6: macOS Tahoe 26 Compatibility
**Goal**: AutoMux is fully functional on macOS 26 Tahoe — macros fire, permissions are detected accurately, and the app launches without errors
**Depends on**: Nothing (first v2.0 phase; findings may constrain Phase 8 architecture)
**Requirements**: COMPAT-01, COMPAT-02, COMPAT-03
**Success Criteria** (what must be TRUE):
  1. A macro configured to click or press a key fires correctly on macOS 26 Tahoe — CGEventTap input injection is not blocked or silently dropped by the OS
  2. After approving Accessibility in System Settings on macOS 26 Tahoe, the app shows "granted" — no false "not granted" state or API regression
  3. The app launches on macOS 26 Tahoe to a working UI with no crash, missing entitlement error, or framework exception in the console
**Plans**: 2 plans
Plans:
- [x] 06-01-PLAN.md — Live CGEventTap probe fix in check_accessibility_permissions() + Info.plist NSAccessibilityUsageDescription (COMPAT-01, COMPAT-02, COMPAT-03)
- [ ] 06-02-PLAN.md — Build release DMG + manual Tahoe 26 device verification checkpoint (COMPAT-01, COMPAT-02, COMPAT-03)

### Phase 7: Carry Work — Platform, CI & Safety
**Goal**: All v1.2.0 outstanding reliability work is complete — the Windows build is clean, CI is reproducible and complete, and the macOS emergency-stop path cannot deadlock or silently lose errors
**Depends on**: Nothing (all items are independent of COMPAT and EXEC work; can run in parallel)
**Requirements**: BUILD-01, MEM-01, CI-03, CI-04, CI-05, SAFE-04, ERR-01
**Success Criteria** (what must be TRUE):
  1. `cargo build --target x86_64-pc-windows-msvc` produces zero warnings — no unused-import or unused-bool warnings in platform/windows/mod.rs
  2. Repeated calls to list running apps (opening the process picker multiple times) do not accumulate open HANDLE objects — every OpenProcess call is matched by a CloseHandle before the function returns
  3. A release CI run completes without "Signature not found for the updater JSON. Skipping upload" — the updater JSON signature artifact is successfully uploaded
  4. Triggering a macro emergency stop on macOS does not deadlock — the REGISTRY lock is released before any CGEvent is posted; and when a profile auto-save fails, the user sees a visible error in the UI
**Plans**: TBD

### Phase 8: Parallel Macro Execution
**Goal**: Multiple macros can run simultaneously on both macOS and Windows — triggering a second macro never blocks, queues, or cancels a running one
**Depends on**: Phase 6 (COMPAT findings may affect platform-layer changes needed for parallel execution on macOS)
**Requirements**: EXEC-01, EXEC-02
**Success Criteria** (what must be TRUE):
  1. On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both macros fire their actions concurrently at their configured intervals
  2. On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A without either blocking the other
  3. Stopping one running macro does not affect any other concurrently running macro
**Plans**: TBD

### Phase 9: UI Redesign & Macro Management
**Goal**: AutoMux has a fully redesigned UI — Apple design language with liquid glass on macOS 26, a modern equivalent on Windows, and users can delete and edit existing macros with unambiguous action-type labels
**Depends on**: Phase 6 (macOS 26 Tahoe APIs must be understood before implementing liquid glass effects)
**Requirements**: UI-01, UI-02, UI-03, UI-04, UX-08, UX-09, UX-10
**Success Criteria** (what must be TRUE):
  1. On macOS 26 Tahoe, the AutoMux window uses native liquid glass materials and vibrancy — window chrome and controls visually match the Tahoe HIG; layout is clean, focused, and keyboard-navigable in a Raycast-inspired hierarchy
  2. On Windows, the app presents a modern, polished UI that matches AutoMux's visual identity without macOS-specific effects
  3. A user can delete any existing macro directly from the macro list without entering a separate edit mode
  4. A user can edit an existing macro's name, action type, key/button assignment, and timing after creation — changes persist across app restarts
  5. Action type selection displays unambiguous labels — "Left Click", "Right Click", "Hold", "Key Press" — with no unlabeled or ambiguous options
**Plans**: TBD
**UI hint**: yes

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Reliability & Safety | v1.0 | 4/4 | Complete | 2026-05-16 |
| 2. Cleanup & Credibility | v1.0 | 4/4 | Complete | 2026-05-30 |
| 3. Macro Setup UX | v1.0 | 4/4 | Complete | 2026-05-30 |
| 4. CI Hardening | v1.0 | 2/2 | Complete | 2026-05-30 |
| 5. macOS Permissions & Reliability | v1.2.0 | 2/2 | Complete | 2026-06-02 |
| 6. macOS Tahoe 26 Compatibility | v2.0 | 1/2 | In Progress|  |
| 7. Carry Work — Platform, CI & Safety | v2.0 | 0/TBD | Not started | — |
| 8. Parallel Macro Execution | v2.0 | 0/TBD | Not started | — |
| 9. UI Redesign & Macro Management | v2.0 | 0/TBD | Not started | — |
