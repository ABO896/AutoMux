---
phase: 08-hotkey-reliability-conflict-safety
plan: 01
subsystem: state-model
tags: [rust, macos, windows, cgevent, mod-constants, state-actor, conflict-detection-foundation]

# Dependency graph
requires:
  - phase: 07-carry-work-platform-ci-safety
    provides: "Two-actor model, scheduler/state-actor dispatch, established serde(default) backwards-compat pattern"
provides:
  - "MacroConfig.trigger_modifiers: u64 (backwards-compatible via #[serde(default)])"
  - "AppState.conflicts: Vec<InputConflict> (derived, surfaced to frontend via state-changed)"
  - "InputConflict struct (macros, input) for UX-12 conflict detection"
  - "Tuple-keyed MACRO_TRIGGER_KEYS: HashMap<(u16, u64), Uuid> on both macOS and Windows"
  - "CGEventFlag* bit constants pinned by unit test (frontend/backend bit layout cannot drift)"
  - "MOD_* Windows constants pinned by unit test (Windows-only, cfg-gated)"
affects:
  - "08-02 (UX-11 conflict detection builds on check_trigger_key_conflict + recompute_conflicts — uses InputConflict and trigger_modifiers)"
  - "08-03 (Intent::BindHotkey / Intent::UnbindHotkey + new Windows HOTKEY_BINDINGS — relies on tuple-keyed registry)"
  - "08-04 (Frontend computeModifiers / hotkey IPC threading — matches bit constants pinned here)"
  - "08-05 (UI surfaces for UX-12 conflicts and UX-14 first-run banner)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tuple-keyed HashMap registry for (keycode, modifiers) binding matching"
    - "#[serde(default)] for backwards-compatible new fields on persisted structs"
    - "Platform bit-value pinning via cfg-gated unit tests (CGEventFlags, MOD_*)"
    - "Read-site change from keycode-only to (keycode, modifier-mask) for global hotkey dispatch"
    - "Placeholder (keycode, 0_u64) lookup on Windows until 08-03 synthesizes the real mod mask via GetAsyncKeyState"

key-files:
  created: []
  modified:
    - src-tauri/src/state/mod.rs
    - src-tauri/src/platform/macos/observer.rs
    - src-tauri/src/platform/windows/mod.rs
    - src-tauri/src/persistence.rs
    - src-tauri/src/scheduler/mod.rs

key-decisions:
  - "Used core-graphics 0.24 binding name 'CGEventFlagAlternate' (not 'CGEventFlagOption' as the plan text suggested) — the Apple documentation uses 'Option' but the Rust binding uses 'Alternate'. Test message documents both names for clarity."
  - "Sequenced the platform tuple-keyed registry (Task 2) and the producer-side reevaluate_all_macros (Task 3) in the same commit because the project does not compile between them — splitting them would leave a broken tree."
  - "Used placeholder (keycode, 0_u64) on the Windows hook read site to keep the build green between plans; the real synthesized mod mask via GetAsyncKeyState is added in plan 08-03 per the original plan."

patterns-established:
  - "Pin platform constants in unit tests immediately when adding IPC fields that carry raw platform bit values"
  - "Always pair a new struct field with #[serde(default)] when the struct is persisted to JSON"
  - "Marker-only field additions to existing structs belong in the producer-side call site, not in shared deserialization helpers"

requirements-completed: [UX-11, UX-13]

# Metrics
duration: 5min
completed: 2026-06-30
---

# Phase 8 Plan 1: Hotkey Data Model & Modifier Bit Pinning Summary

**Added `trigger_modifiers: u64` and `conflicts: Vec<InputConflict>` to the data model, defined the `InputConflict` struct, converted the platform `MACRO_TRIGGER_KEYS` registries to `HashMap<(u16, u64), Uuid>`, and pinned the macOS `CGEventFlag*` and Windows `MOD_*` bit constants via cfg-gated unit tests.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-30T21:06:28Z
- **Completed:** 2026-06-30T21:11:32Z
- **Tasks:** 3
- **Files modified:** 5 (3 production, 2 test helpers updated for new field)

## Accomplishments

- `MacroConfig.trigger_modifiers: u64` added with `#[serde(default)]` so pre-Phase-8 profiles deserialize as `0` (no modifiers)
- `AppState.conflicts: Vec<InputConflict>` added with `#[serde(default)]` and `conflicts: Vec::new()` in `AppState::default()` — the field is sent to the frontend via the existing `state-changed` event
- `InputConflict { macros: Vec<Uuid>, input: InputEvent }` struct created; derives `Debug, Clone, Serialize, Deserialize` (mirrors the `ProfileData` wrapper pattern)
- macOS `MACRO_TRIGGER_KEYS` is now `HashMap<(u16, u64), Uuid>`; the hook read site extracts `flags.bits()` as the modifier component
- Windows `MACRO_TRIGGER_KEYS` is now `HashMap<(u16, u64), Uuid>`; the hook read site uses `(keycode, 0_u64)` placeholder (real synthesized mask added in plan 08-03)
- `reevaluate_all_macros` in `state/mod.rs` now inserts `(trigger_key, trigger_modifiers)` tuples into the tuple-keyed map
- `cg_event_flag_constants` test pins macOS bit values: `Shift=0x20000, Control=0x40000, Alternate=0x80000, Command=0x100000`
- `windows_mod_constants` test (gated on `#[cfg(windows)]`) pins Windows bit values: `MOD_ALT=0x0001, MOD_CONTROL=0x0002, MOD_SHIFT=0x0004, MOD_WIN=0x0008`

## Task Commits

1. **Task 1: Add `trigger_modifiers`, `conflicts` fields, and `InputConflict` struct to `state/mod.rs`** — `b6e3a1f` (feat)
2. **Task 2: Convert `MACRO_TRIGGER_KEYS` to `HashMap<(u16, u64), Uuid>` on macOS and Windows; add modifier-bit unit tests** — `6ad0f97` (feat, combined with Task 3)
3. **Task 3: Update `reevaluate_all_macros` in `state/mod.rs` to build `HashMap<(u16, u64), Uuid>` and propagate to platform statics** — `6ad0f97` (feat, combined with Task 2)

**Plan metadata:** (this SUMMARY commit)

_Note: Tasks 2 and 3 were committed in a single commit because the project does not compile between them — the platform-side registry type change (Task 2) requires the producer-side map type change (Task 3) for the build to succeed. Splitting them would leave a broken tree._

## Files Created/Modified

- `src-tauri/src/state/mod.rs` — Added `InputConflict` struct, `trigger_modifiers: u64` field to `MacroConfig`, `conflicts: Vec<InputConflict>` field to `AppState`, updated `AppState::default()`, updated `reevaluate_all_macros` to insert tuple keys
- `src-tauri/src/platform/macos/observer.rs` — Changed `MACRO_TRIGGER_KEYS` to `HashMap<(u16, u64), Uuid>`, updated hook read site to use `flags.bits()`, added `cg_event_flag_constants` test
- `src-tauri/src/platform/windows/mod.rs` — Changed `MACRO_TRIGGER_KEYS` to `HashMap<(u16, u64), Uuid>`, updated hook read site to use `(keycode, 0_u64)` placeholder, added `windows_mod_constants` test (cfg-gated on Windows)
- `src-tauri/src/persistence.rs` — Updated `large_config_memory_check` test to include new `trigger_modifiers: 0` field in `MacroConfig` constructor
- `src-tauri/src/scheduler/mod.rs` — Updated `afk_farm_stress_test` and `jitter_audit_10ms_interval` tests to include new `trigger_modifiers: 0` field in `MacroConfig` constructor

## Decisions Made

- **Used `CGEventFlagAlternate` (not `CGEventFlagOption`):** The plan text referenced the Apple documentation name "Option" but the `core-graphics` 0.24 Rust binding uses "Alternate". The test message documents both names for clarity. Both refer to the same `0x80000` bit.
- **Combined Tasks 2 and 3 in a single commit:** The platform-side registry type change (Task 2) and the producer-side map type change (Task 3) are inseparable for a green build. Splitting them would leave a broken tree. The commit message documents this for traceability.
- **Windows placeholder `(keycode, 0_u64)`:** Per the plan, kept the placeholder on the Windows hook read site so `cargo check` stays green between plans. The real synthesized mod mask via `GetAsyncKeyState` is added in plan 08-03.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Used `CGEventFlagAlternate` instead of `CGEventFlagOption` in macOS bit-pinning test**
- **Found during:** Task 2 (first compile attempt of `cg_event_flag_constants` test)
- **Issue:** Plan text referenced `CGEventFlagOption` (matching Apple documentation naming), but the `core-graphics` 0.24 Rust binding uses `CGEventFlagAlternate`. Using `CGEventFlagOption` caused a compile error: "no associated item named `CGEventFlagOption` found for struct `core_graphics::event::CGEventFlags`"
- **Fix:** Used `CGEventFlagAlternate` and added a comment documenting that it corresponds to "Option" in Apple documentation
- **Files modified:** src-tauri/src/platform/macos/observer.rs
- **Verification:** Test compiles and passes on macOS host; `cargo test -p automux-lib cg_event_flag_constants` exits 0
- **Committed in:** `6ad0f97`

**2. [Rule 3 - Blocking] Updated test code in `persistence.rs` and `scheduler/mod.rs` to include new `trigger_modifiers` field**
- **Found during:** Task 1 (first compile attempt after adding `trigger_modifiers` to `MacroConfig`)
- **Issue:** Test code constructs `MacroConfig` directly; the new required field caused 3 compile errors in `persistence.rs` (1 site) and `scheduler/mod.rs` (2 sites)
- **Fix:** Added `trigger_modifiers: 0` to each `MacroConfig { ... }` constructor in test code
- **Files modified:** src-tauri/src/persistence.rs, src-tauri/src/scheduler/mod.rs
- **Verification:** `cargo check --all-targets` exits 0; `cargo test --lib` shows 4/4 tests pass
- **Committed in:** `b6e3a1f`

**3. [Rule 3 - Blocking] Sequenced Tasks 2 and 3 as a single commit**
- **Found during:** Task 2 (after type change, project would not compile until Task 3 was also done)
- **Issue:** The plan presents Tasks 2 and 3 as separate commits, but the type unification requires both producer-side and consumer-side changes to compile. Committing only Task 2 would leave a broken tree.
- **Fix:** Combined Tasks 2 and 3 into a single commit (`6ad0f97`) with a documented rationale in the commit message
- **Files modified:** (same as Tasks 2 and 3)
- **Verification:** `cargo check --all-targets` exits 0 after the single commit
- **Committed in:** `6ad0f97`

---

**Total deviations:** 3 auto-fixed (1 bug, 2 blocking)
**Impact on plan:** All auto-fixes necessary for correctness/compilability. No scope creep.

## Issues Encountered

- Pre-existing clippy warning `unneeded return statement` in `src/ipc/mod.rs:126` (NOT introduced by this plan). Confirmed by stashing changes and re-running `cargo clippy --all-targets -- -D warnings` on master — same warning exists at the same location. Per scope boundary, this is left for a separate cleanup. The plan's "no new clippy warnings" acceptance criterion is met (no new warnings added by this plan's changes).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 08-02 (UX-11 conflict detection + UX-12 recompute_conflicts) can proceed:
- `trigger_modifiers` is now available on `MacroConfig` for the conflict check
- `conflicts: Vec<InputConflict>` is on `AppState` and surfaces via `state-changed`
- `InputConflict` struct is defined
- `HashMap<(u16, uuid), Uuid>` is in place so the conflict check can detect duplicate `(keycode, modifiers)` pairs at `reevaluate_all_macros` time
- Platform bit constants are pinned by tests so the frontend/backend bit layout cannot drift

The state-side `reevaluate_all_macros` still silently overwrites when two macros share the same `(keycode, modifiers)` pair — this is expected per the threat model T-08-05; the new `check_trigger_key_conflict` pre-check in plan 08-02 makes the silent overwrite unreachable in normal user flow.

---
*Phase: 08-hotkey-reliability-conflict-safety*
*Completed: 2026-06-30*
