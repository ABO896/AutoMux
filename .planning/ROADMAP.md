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

- [x] **Phase 6: macOS Tahoe 26 Compatibility** *(planned — 3 plans)* — Investigate and fix CGEventTap input injection and permissions detection on macOS 26 Tahoe; ensure the app launches without crashes or entitlement errors (06-01/06-02 shipped 2026-06-04; 06-03 fixes UAF crash + injection regression surfaced on device) (completed 2026-06-12)
- [x] **Phase 7: Carry Work — Platform, CI & Safety** — Eliminate Windows compiler warnings and the OpenProcess handle leak; harden the CI release pipeline; fix the REGISTRY deadlock risk and surface auto-save failures in the UI; add Input Monitoring detection and re-grant UX for macOS 26 signed-build upgrades (completed 2026-06-17)
- [ ] **Phase 8: Hotkey Reliability & Conflict Safety** — Fix hotkey binding to support the full key range (not just A-Z); prevent duplicate hotkey assignments; warn on concurrent same-action macros; verify and communicate global (system-wide) hotkey behavior *(in progress — 6/6 plans done; 08-VERIFICATION.md gate artifact created; 4/4 automated gates green, 2/2 manual device test plans documented; awaiting human device verification on real macOS + Windows hosts to mark Sections 5+6 done)*
- [ ] **Phase 9: Parallel Macro Execution** — Redesign the StateActor/Scheduler execution model so multiple macros run concurrently on both macOS and Windows
- [ ] **Phase 10: UI Redesign & Macro Management** — Ship the full Apple/liquid-glass UI redesign for macOS and a modern equivalent for Windows; add macro delete and edit capabilities with clear action-type labeling

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

**Plans**: 3 plans
Plans:

- [x] 06-01-PLAN.md — Live CGEventTap probe fix in check_accessibility_permissions() + Info.plist NSAccessibilityUsageDescription (COMPAT-01, COMPAT-02, COMPAT-03)
- [x] 06-02-PLAN.md — Build release DMG + manual Tahoe 26 device verification checkpoint (COMPAT-01, COMPAT-02, COMPAT-03)
- [x] 06-03-PLAN.md — Fix UAF crash in CGEventTap re-enable callback (CFRunLoop stop+reinit) and verify UI-toggled macro injection on Tahoe 26 (COMPAT-01, COMPAT-03)

### Phase 7: Carry Work — Platform, CI & Safety

**Goal**: All v1.2.0 outstanding reliability work is complete and macOS 26 permission follow-ups are addressed — the Windows build is clean, CI is reproducible and complete, the macOS emergency-stop path cannot deadlock, and the app handles macOS 26 TCC changes gracefully
**Depends on**: Nothing (all items are independent of COMPAT and EXEC work; can run in parallel)
**Requirements**: BUILD-01, MEM-01, CI-03, CI-04, CI-05, SAFE-04, ERR-01, COMPAT-04, COMPAT-05
**Success Criteria** (what must be TRUE):

  1. `cargo build --target x86_64-pc-windows-msvc` produces zero warnings
  2. Repeated calls to list running apps do not accumulate open HANDLE objects — every OpenProcess is matched by a CloseHandle
  3. A release CI run completes without the "Signature not found" skip — updater JSON artifact uploads successfully
  4. Triggering a macro emergency stop on macOS does not deadlock — REGISTRY lock released before CGEvent post; auto-save failures show a visible UI error
  5. On macOS 26, the app shows a clear prompt when Input Monitoring is not granted — user knows why hotkeys don't fire and how to fix it
  6. After installing a signed build over an unsigned one, the app detects the TCC identity change and prompts the user to re-add Accessibility in System Settings

**Plans**: 3 plansPlans:
**Wave 1**

- [x] 07-01-PLAN.md — Windows platform cleanup (BUILD-01, MEM-01) + CI hardening (CI-03, CI-04, CI-05) + SAFE-04 verification
- [x] 07-02-PLAN.md — macOS 26 permission backend: check_input_monitoring probe + TCC flag helpers + get_tcc_identity_status IPC (COMPAT-04, COMPAT-05)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 07-03-PLAN.md — Frontend: combined PermissionsCard with Accessibility + Input Monitoring, TCC identity-change copy swap, auto-save error banner (COMPAT-04, COMPAT-05, ERR-01)

### Phase 8: Hotkey Reliability & Conflict Safety

**Goal**: The hotkey binding system is reliable, full-featured, and safe — supports a broad key range, prevents silent conflicts between macros, and users understand that binds are system-wide
**Depends on**: Nothing (independent of EXEC and UI work)
**Requirements**: UX-11, UX-12, UX-13, UX-14
**Success Criteria** (what must be TRUE):

  1. A user can bind a hotkey using not just A-Z but also 0-9, F1-F12, and modifier combinations — the binding UI exposes a picker or accepts any of these inputs
  2. Attempting to bind a key that is already assigned to another macro shows an explicit conflict error or reassignment prompt — no silent shadowing
  3. Enabling a second macro that injects the same input (e.g., left click) as an already-active macro triggers a visible warning — the user is not left wondering why double-speed clicks are happening
  4. Hotkeys fire when AutoMux is not the focused app — the UI communicates this clearly (e.g., "Binds are system-wide"), and global operation is verified on both macOS and Windows

**Plans**: 6 plans
Plans:

- [x] 08-01-PLAN.md — Hotkey data model + modifier bit pinning (UX-11, UX-13)
- [x] 08-02-PLAN.md — Conflict detection helpers + 9-handler wiring (UX-11, UX-12)
- [x] 08-03-PLAN.md — Intent::BindHotkey / Intent::UnbindHotkey + new Windows HOTKEY_BINDINGS (UX-11, UX-14)
- [x] 08-04-PLAN.md — Frontend computeModifiers + hotkey IPC threading (UX-11, UX-13)
- [x] 08-05-PLAN.md — Conflict toast / warning region / first-run global notice (UX-11, UX-12, UX-14)
- [x] 08-06-PLAN.md — Global hotkey behavior verification on both platforms (UX-14)

### Phase 9: Parallel Macro Execution

**Goal**: Multiple macros can run simultaneously on both macOS and Windows — triggering a second macro never blocks, queues, or cancels a running one
**Depends on**: Phase 6 (COMPAT findings may affect platform-layer changes needed for parallel execution on macOS)
**Requirements**: EXEC-01, EXEC-02
**Success Criteria** (what must be TRUE):

  1. On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals
  2. On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A
  3. Stopping one running macro does not affect any other concurrently running macro

**Plans**: 9/9 plans executed
Plans:
**Wave 1**

- [x] 09-01-PLAN.md — Scheduler parallel-execution tests + action_tx capacity 1024 + debug-only drop counter & IPC (EXEC-01, EXEC-02)
- [x] 09-02-PLAN.md — Frontend computeRunningState derivation + per-card firing/waiting/held/combined indicators (EXEC-01, EXEC-02)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 09-03-PLAN.md — 09-VERIFICATION.md gate: automated test/clippy gates + 3 manual device tests per platform (EXEC-01, EXEC-02)

**Gap Closure** *(from 09-VERIFICATION.md blockers)*

- [x] 09-04-PLAN.md — Fix computeRunningState to show "held" for Hold-mode macros (branch on trigger_mode, mirroring scheduler Hold conversion) + fold in WR-01 compute-once-per-card (EXEC-01, EXEC-02)
- [x] 09-05-PLAN.md — Harden per-macro stop path (stop_macro/release_holds→async, guaranteed .await HoldRelease delivery mirroring StopAll) so a Hold-mode macro's held input is never left stuck down under action-channel saturation + saturation regression test (EXEC-01, EXEC-02)
- [x] 09-06-PLAN.md — Harden the start-side HoldStart send (start_macro→guaranteed .await delivery, mirroring 09-05's release-side fix) so a Hold-mode macro is never recorded as "held" when the HoldStart was dropped under action-channel saturation (CR-01 asymmetric gap) + start-side saturation regression test (EXEC-01, EXEC-02)

**Gap Closure** *(from 09-UAT.md — G-09-1a / G-09-1b / G-09-1c)*

- [x] 09-07-PLAN.md — macOS observer fixes: narrow the CGEventTap event mask to consumed types (G-09-1a perf tax) + instrument & harden the NSWorkspace active-app observer, reading the activated bundle id from the notification userInfo (G-09-1c targeting) (EXEC-01, EXEC-02)
- [x] 09-08-PLAN.md — Frontend macro management: per-card delete button wired to the existing `remove_macro` IPC (G-09-1b) + controlled card-edit target-app select (G-09-1c secondary UI bug) (EXEC-01, EXEC-02)

**Gap Closure** *(from 09-VERIFICATION.md gaps #10/#11 — IPC argument-casing)*

- [x] 09-09-PLAN.md — Add `rename_all = "snake_case"` to the 5 IPC commands with underscore params (set_macro_target_app, bind_hotkey, unbind_hotkey, set_macro_trigger_key, update_step_interval) so card-edit target-app and hotkey changes actually persist; fix handleCardSetTriggerKey catch-block error misclassification (EXEC-01, EXEC-02)

### Phase 10: UI Redesign & Macro Management

**Goal**: AutoMux has a fully redesigned UI — Apple design language with liquid glass on macOS 26, a modern equivalent on Windows, and users can delete and edit existing macros with unambiguous action-type labels
**Depends on**: Phase 6 (macOS 26 Tahoe APIs must be understood before implementing liquid glass effects); Phase 8 (hotkey UI rework feeds into the redesign)
**Requirements**: UI-01, UI-02, UI-03, UI-04, UX-08, UX-09, UX-10
**Success Criteria** (what must be TRUE):

  1. On macOS 26 Tahoe, the AutoMux window uses native liquid glass materials and vibrancy — window chrome and controls match the Tahoe HIG; layout is clean, focused, and keyboard-navigable in a Raycast-inspired hierarchy
  2. On Windows, the app presents a modern, polished UI matching AutoMux's visual identity without macOS-specific effects
  3. A user can delete any existing macro directly from the macro list without entering a separate edit mode
  4. A user can edit an existing macro's name, action type, key/button assignment, and timing after creation — changes persist across restarts
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
| 6. macOS Tahoe 26 Compatibility | v2.0 | 3/3 | Complete | 2026-06-17 |
| 7. Carry Work — Platform, CI & Safety | v2.0 | 3/3 | Complete   | 2026-06-17 |
| 8. Hotkey Reliability & Conflict Safety | v2.0 | 6/6 | In Progress | — |
| 9. Parallel Macro Execution | v2.0 | 9/9 | In Progress|  |
| 10. UI Redesign & Macro Management | v2.0 | 0/TBD | Not started | — |
