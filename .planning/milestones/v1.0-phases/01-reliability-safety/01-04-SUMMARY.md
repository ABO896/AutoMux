---
phase: 01-reliability-safety
plan: "04"
subsystem: persistence
tags: [rust, tauri, state-actor, auto-save, profile-manager, arc, ipc]

requires: []
provides:
  - "Arc<ProfileManager> injected into StateActor for auto-save without user action"
  - "loading_profile suppression flag preventing N-write storm on startup"
  - "Intent::LoadProfile bracketed batch loader (single disk write per batch)"
  - "auto-save-error Tauri event for non-blocking persistence failure surfacing"
  - "Unconditional engine_active restore on profile load (no one-way ratchet)"
affects:
  - "01-reliability-safety (RELY-04 closed)"
  - "future profile management plans — IPC now uses Arc<ProfileManager>"

tech-stack:
  added: []
  patterns:
    - "Arc<T> injection into long-lived actor for shared service access"
    - "loading_profile bool flag with #[serde(skip)] to suppress auto-save during batch operations"
    - "auto-save via async helper that guards on suppression flag and emits Tauri event on failure"
    - "Single intent replaces N-loop startup restoration pattern"

key-files:
  created: []
  modified:
    - "src-tauri/src/state/mod.rs"
    - "src-tauri/src/lib.rs"
    - "src-tauri/src/ipc/mod.rs"

key-decisions:
  - "Inject Arc<ProfileManager> into StateActor (not AppState) — AppState must stay serializable"
  - "loading_profile flag with #[serde(skip)] ensures suppression state is invisible to frontend and profile JSON"
  - "Single Intent::LoadProfile replaces startup AddMacro loop — prevents N disk writes at app launch (Pitfall 2)"
  - "Unconditional engine_active = profile.engine_active in LoadProfile handler — prevents one-way ratchet (Pitfall 6)"
  - "auto-save-error Tauri event reuses same emission pattern as state-changed — single error surface for persistence failures (D-07)"

patterns-established:
  - "auto_save_default: guard on loading_profile, build ProfileData from live state, await save_profile, emit auto-save-error on failure"
  - "Intent::LoadProfile: clear, StopAll, set flag=true, load+populate+unconditional engine_active, reevaluate, set flag=false, single auto_save_default, broadcast"

requirements-completed: [RELY-04]

duration: 18min
completed: 2026-05-16
---

# Phase 01 Plan 04: Auto-Save End-to-End Summary

**Arc<ProfileManager> injected into StateActor with loading_profile suppression flag, wiring auto-save after every macro mutation and bracketing profile loads into a single disk write**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-05-16T12:30:00Z
- **Completed:** 2026-05-16T12:48:00Z
- **Tasks:** 3 (1a, 1b, 2)
- **Files modified:** 3

## Accomplishments

- StateActor now persists all macro mutations to the `default` profile on disk without user action — closing the gap between the docs claiming auto-save and the actual behavior
- Startup restoration dispatches a single `Intent::LoadProfile("default")` intent instead of N `Intent::AddMacro` sends, eliminating the N-write storm on app launch (Pitfall 2 closed)
- IPC `load_profile` command simplified to use the same bracketed `Intent::LoadProfile` path, ensuring loading a named profile also yields exactly one disk write afterwards (D-05)
- engine_active unconditionally restored from saved profile on load — no one-way ratchet (Pitfall 6 closed)
- `auto-save-error` Tauri event surfaces persistence failures non-blockingly to the frontend (D-07)

## Task Commits

1. **Task 1a: Add data-shape plumbing** - `8154d6d` (feat)
   - AppState.loading_profile field, Intent::LoadProfile variant, StateActor.profile_mgr field, StateActor::new 5th param, auto_save_default helper, Arc wrapping in lib.rs
2. **Task 1b: Wire auto-save and LoadProfile handler** - `b1e4e9a` (feat)
   - auto_save_default().await appended after 6 mutating arms; full Intent::LoadProfile handler with suppression bracket
3. **Task 2: Rewire lib.rs + IPC** - `061b558` (feat)
   - Startup loop replaced with single Intent::LoadProfile; 4 IPC State annotations updated to Arc; load_profile body collapsed to single intent send

## Files Created/Modified

- `src-tauri/src/state/mod.rs` - AppState.loading_profile, Intent::LoadProfile, StateActor.profile_mgr, StateActor::new signature, auto_save_default helper, LoadProfile handler, auto_save_default calls in 6 mutating arms
- `src-tauri/src/lib.rs` - Arc::new wrapping of ProfileManager, startup spawn collapsed to single Intent::LoadProfile send, StateActor::new gains profile_mgr arg
- `src-tauri/src/ipc/mod.rs` - use std::sync::Arc added; 4 State type annotations updated to Arc<ProfileManager>; load_profile body replaced with single intent send

## Key Diff Snippets

**AppState extension (Task 1a):**
```rust
#[serde(skip)]
pub loading_profile: bool,
```

**auto_save_default helper (Task 1a):**
```rust
async fn auto_save_default(&self) {
    if self.state.loading_profile { return; }
    let profile = ProfileData {
        name: "default".to_string(),
        macros: self.state.macros.clone(),
        engine_active: self.state.engine_active,
    };
    if let Err(e) = self.profile_mgr.save_profile(&profile).await {
        use tauri::Emitter;
        let _ = self.app_handle.emit("auto-save-error", e.to_string());
    }
}
```

**Intent::LoadProfile handler engine_active assignment (Task 1b — Pitfall 6 closed):**
```rust
self.state.engine_active = profile.engine_active; // unconditional — not a ratchet
```

**lib.rs startup (Task 2 — Pitfall 2 closed):**
```rust
let _ = startup_tx.send(Intent::LoadProfile("default".to_string())).await;
```

## Decisions Made

- Wrap ProfileManager in Arc rather than Clone for StateActor injection — avoids structural duplication of the PathBuf; Arc::deref gives transparent method access
- loading_profile is `#[serde(skip)]` so it never appears in profile JSON or frontend AppState projections — suppression is fully internal to StateActor
- IPC load_profile keeps the leading `profile_mgr.load_profile(&name).await?` call to preserve the return value for the frontend, then delegates the state change to the actor via intent

## Deviations from Plan

**1. [Rule 3 - Blocking] Minimal lib.rs Arc update included in Task 1a**
- **Found during:** Task 1a (verifying `cargo build --lib`)
- **Issue:** lib.rs is the library entry point; `StateActor::new` call site in lib.rs fails compilation when the signature gains a 5th argument — even for `--lib` builds
- **Fix:** Added `use std::sync::Arc`, wrapped `ProfileManager` in `Arc::new`, added `profile_mgr.clone()` to `StateActor::new` call in lib.rs during Task 1a (the minimal change needed to restore `--lib` buildability). Full lib.rs rewire (startup loop collapse) deferred to Task 2 as planned.
- **Files modified:** src-tauri/src/lib.rs
- **Committed in:** 8154d6d (Task 1a commit)

---

**Total deviations:** 1 auto-fixed (Rule 3 - blocking compile error)
**Impact on plan:** Minimal lib.rs addition in Task 1a was structurally necessary. Task 2 still performed its full rewire as planned. No scope creep.

## Issues Encountered

- The plan's verify command used `cargo build --lib` (library-only) expecting `lib.rs` changes would not be needed for Task 1a, but `lib.rs` IS the library crate root and the `StateActor::new` call site lives there. Minimal Arc wiring added to Task 1a commit to restore buildability; full rewire deferred to Task 2 as specified.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- RELY-04 fully closed: closing AutoMux without saving now persists all macro changes to `default` profile
- Startup restoration produces exactly one disk write regardless of macro count
- IPC profile commands all operate against Arc<ProfileManager> managed state
- Frontend can listen for `auto-save-error` Tauri events to surface persistence failures
- No blockers for remaining Phase 1 plans

## Known Stubs

None.

## Threat Flags

No new security-relevant surface introduced beyond what the plan's threat model covers. The `auto-save-error` event payload derives from serde_json/tokio::fs error strings (same as existing debug eprintln in persistence.rs). The `default` profile name is hardcoded — no user input crosses the path construction boundary for auto-save writes.

## Self-Check: PASSED

All files exist. All task commits verified in git log.
