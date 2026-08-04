# Phase 9: Parallel Macro Execution - Context

**Gathered:** 2026-07-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Make multiple macros run simultaneously on both macOS and Windows — triggering a second macro while a first is firing must not pause, delay, or cancel the first. Stopping one macro must not affect any other. Includes:

- A unit test that proves two macros run concurrently in the scheduler (no test exists today)
- Manual device tests on both macOS and Windows mirroring the Phase 8 verification structure
- Per-macro "running state" UI visibility (enabled-but-waiting vs enabled-and-firing vs held vs disabled)
- An increased action-channel capacity with a debug-only drop counter

The scheduler architecture already supports multiple macros in principle (single `BTreeMap<Instant, Vec<StepId>>` timeline, per-macro `RunningConfig` keyed by `macro_id`, CR-04 no-op restart). The phase is primarily about (a) proving this works, (b) making the parallel behavior visible to users, and (c) hardening the action-channel for the increased fire rate that parallel execution creates.

Requirements: EXEC-01, EXEC-02.

</domain>

<decisions>
## Implementation Decisions

### Running-state visibility (UI layer)

- **D-01:** A macro that is `enabled && engine_active && !emergency_stop && (target_app == active_app || target_app == None)` shows as **firing** on its card. A macro that is `enabled` but the target app doesn't match the active app shows as **enabled-but-waiting** (distinct from firing). The green-dot-on-enabled behavior of today is misleading because the user can't tell "I enabled this, why isn't it firing?" without checking the engine toggle or active app.
- **D-02:** Held vs firing have **distinct visual indicators**. A macro with only `SustainedHold` steps shows a "held" indicator (e.g., a different dot color or a small lock icon). A macro with `InterleavedInterval` steps shows a pulsing green dot. A macro with both (the AFK farm case, e.g., "Hold Right-Click + Left-Click every 100ms") shows a **single combined indicator** that conveys "active multi-step" (small label or combined icon) — stacking two indicators on one card is too busy.
- **D-03:** The running state is shown **on the per-macro card only**. No global "X firing, Y waiting" summary in the engine toggle area. Per-card is the minimum UI surface that answers "is this macro firing?" — adding a global count is a separate capability.
- **D-04 (Claude's discretion):** The firing/waiting/held state is **derived on the frontend** from existing `AppState` fields (`macros[*].enabled`, `state.engine_active`, `state.emergency_stop_active`, `state.active_app`, `macros[*].target_app`, plus the `sequence.steps` to know if any step is `SustainedHold`). No new field on `AppState`, no backend computation. The frontend already has all the inputs; the derivation is a pure function in `App.tsx`.

### Parallel-execution verification test

- **D-05:** Phase 9 adds both a **Rust unit test** (in `src-tauri/src/scheduler/mod.rs`'s `#[cfg(test)] mod tests`, alongside the existing `afk_farm_stress_test` and `jitter_audit_10ms_interval`) **and manual device tests on both macOS and Windows** (mirroring Phase 8's structure: `09-VERIFICATION.md` with sections 5+6 for manual tests, following the same gate format as `08-VERIFICATION.md`).
- **D-06 (Claude's discretion):** The unit test covers two scenarios: **(a) two macros at different intervals fire concurrently** and **(b) stopping macro A while macro B is running does not affect B**. The conflict scenario (two macros injecting the same input, both fire with the warning shown) is **already covered by Phase 8's `conflict_detection_overlap` and `conflict_disappear_on_disable`** — not duplicated in Phase 9. Each scenario is a separate `#[tokio::test]` for clean test names and isolated failures.
- **D-07 (Claude's discretion):** The unit test uses the **action-counter pattern** (collect fired `ActionReady` events via `action_rx.try_recv()`, count per macro, assert within expected ranges). Same pattern as the existing `afk_farm_stress_test` — easy to debug when it fails. Timing-based assertions (e.g., "macro A fires within X ms of macro B's fire") are too fragile on slow CI.
- **D-08 (Claude's discretion):** **3 manual device tests per platform** (macOS + Windows), focused on the parallel behavior. Each test corresponds to one of the three success criteria in `ROADMAP.md`. Example macOS tests: T9.1 = enable macro B while A is firing — verify both fire. T9.2 = stop macro A while B is running — verify B continues. T9.3 = enable two macros with same input — verify both fire and the conflict warning shows. The Windows version mirrors with Win32-appropriate setup.

### Action-channel capacity & backpressure

- **D-09 (Claude's discretion):** Increase `action_tx` capacity **from 100 to 1024** in `src-tauri/src/lib.rs:26` and **add a `try_send` drop counter in `#[cfg(debug_assertions)]` builds only**. The counter is a simple `AtomicU64` incremented on every `try_send` failure in `scheduler/mod.rs` (3 sites: `fire_due_actions` at the interval fire, `start_macro` for `HoldStart`, `release_holds` for `HoldRelease`). The counter is exposed via a debug-only IPC command (e.g., `get_debug_action_drop_count`) for ad-hoc diagnosis.
- **D-10 (Claude's discretion):** Drops are **silent in release builds** (same as today — no user-visible warning) but **logged in debug builds** via `eprintln!` (matches the existing `[Component]` logging convention from `.planning/codebase/ARCHITECTURE.md`). No Tauri event is emitted for drops — adding one for an edge case adds UI surface for something the typical user (1-5 macros, typical intervals) won't hit.
- **D-11 (Claude's discretion):** The drop counter is **only on the `action_tx` channel** (the one flagged in `CONCERNS.md` as the most likely to overflow under parallel execution). The `state_tx` and `sched_tx` channels are at low risk (user actions are infrequent — at human action speed, not timer speed) and are left as-is.

### Re-evaluate efficiency (out of scope)

- **D-12 (Claude's discretion):** The per-macro re-evaluate optimization (replace full-sweep `reevaluate_all_macros` with per-macro sends in `SetMacroEnabled` / `SetMacroTargetApp` / `UpdateSequence` / `UpdateStepInterval`) is **out of scope** for Phase 9. The full sweep is correct, and CR-04 makes it cheap (5 no-op `StartMacro` messages for 5 existing macros per state change, negligible cost at user-action speed). The performance bottleneck for parallel execution is the action channel (D-09–D-11), not the re-evaluate path. Defer per-macro optimization to a future performance phase.
- **D-13 (Claude's discretion):** No debouncing of rapid mutations (e.g., user toggles 3 macros off in quick succession) is added in Phase 9. The CONCERNS.md O(n) bottleneck is not a correctness issue for parallel execution.
- **D-14 (Claude's discretion):** **CR-04 stays as-is** (compare full `RunningConfig` for no-op detection). Tightening it to skip restart on non-runtime field changes (e.g., macro name only) is a future optimization — current behavior is well-defined and edge-case-free.

### Claude's Discretion summary

Items where user said "you decide":
- D-02, D-03, D-04 — Running-state visual & computation (front-end details)
- D-06, D-07, D-08 — Test scenario coverage and design
- D-09, D-10, D-11 — Action-channel capacity and drop counter
- D-12, D-13, D-14 — Re-evaluate efficiency (out of scope)

Rationale for the major discretion picks:
- **Derive firing state on the frontend** (D-04) over adding a `firing_macros` field to `AppState`: avoids drift risk if the scheduler's internal state diverges from the StateActor's view (e.g., dropped `try_send`s).
- **Out-of-scope for re-evaluate optimization** (D-12): the success criteria are about correctness, not performance. Including the optimization adds scope and risk to a phase whose primary deliverable is proving parallel execution works.
- **1024 capacity + debug-only drop counter** (D-09): handles the realistic case (5-10 macros at typical intervals) with headroom, without changing release-build performance (try_send returns immediately regardless of capacity).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & constraints
- `.planning/PROJECT.md` — Core value ("A macro that was set up must fire reliably") and project constraints (Tauri 2 + Rust + SolidJS, both platforms)
- `.planning/REQUIREMENTS.md` — EXEC-01, EXEC-02 acceptance criteria
- `.planning/ROADMAP.md` — Phase 9 goal and success criteria (3 explicit "what must be TRUE" bullets)
- `.planning/codebase/ARCHITECTURE.md` — Single-scheduler pattern, two-phase dispatch, input-injection-site invariant (`StateActor::handle_action` is the only injection site), per-macro-spawns anti-pattern, channel backpressure section
- `.planning/codebase/CONCERNS.md` — Performance Bottlenecks (`reevaluate_all_macros` O(n)), Fragile Areas (action channel capacity 100), Scaling Limits (silent `try_send` drops)

### Implementation files (must read for context)
- `src-tauri/src/scheduler/mod.rs` — `Scheduler`, `IntervalTask`, `RunningConfig`, `BTreeMap<Instant, Vec<StepId>>` timeline, `fire_due_actions` (line 353), `start_macro` (line 243, with CR-04 no-op check), `release_holds` (line 340), `try_send` sites (lines 287, 343, 362)
- `src-tauri/src/state/mod.rs` — `StateActor::handle_intent` (line 446), `reevaluate_all_macros` (line 755), `handle_action` (line 373), `Intent` enum (line 152)
- `src-tauri/src/lib.rs` — Channel setup (line 23-26): `state_tx` 100, `sched_tx` 100, `action_tx` 100. Phase 9 changes line 26 (action_tx capacity)
- `src/App.tsx` — Macro card rendering (line 1200+), green dot styling (line 1202-1206: `bg-success shadow-[0_0_6px_var(--color-success-glow)]`)

### Pattern reference (Phase 8, mirror exactly)
- `.planning/phases/08-hotkey-reliability-conflict-safety/08-VERIFICATION.md` — Verification report format. Phase 9 should produce `09-VERIFICATION.md` with the same 7-section structure (test suite, clippy, Windows cross-compile, profile backwards-compat — adapted, manual macOS device tests §5, manual Windows device tests §6, verification status §7). Phase 9 does NOT need a profile backwards-compat section (no schema changes) — the test count gate and clippy gate apply.
- `.planning/phases/08-hotkey-reliability-conflict-safety/08-PATTERNS.md` — Pattern map. Phase 9 has its own analogous patterns to apply (free-function test helpers, `try_send` debug-counter pattern from `scheduler/mod.rs:362`).

### Failure modes (interval floor)
- `docs/FAILURE_MODES.md` — Documents the 5ms minimum interval floor (already enforced in `scheduler/mod.rs:66`). Not a Phase 9 change, but parallel execution at <5ms intervals per macro × N macros is the worst-case for action-channel overflow — informs D-09's capacity choice.

### v1.0 research (background only)
- `.planning/research/PITFALLS.md` — Section 1 (macOS Accessibility), section 2 (cross-platform input injection). Not directly Phase 9-relevant but context for the architecture's design constraints.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`#[cfg(test)] mod tests` in `scheduler/mod.rs:393`** — Existing test module with `afk_farm_stress_test` and `jitter_audit_10ms_interval`. The new `parallel_execution_test` (D-06) goes here, alongside the existing two. Use the same `(intent_tx, intent_rx) = mpsc::channel(100)` / `(action_tx, mut action_rx) = mpsc::channel(256)` setup pattern.
- **CR-04 no-op guard in `scheduler/mod.rs:248-252`** — `start_macro` already short-circuits if `running_configs[macro_id] == new_running`. This is what makes the full-sweep `reevaluate_all_macros` cheap for parallel execution (D-12 rationale).
- **`RunningConfig` struct in `scheduler/mod.rs:109-114`** — Captures the runtime-relevant fields (`interval_ms`, `target_app`, `trigger_mode`, `steps`). Already a clean separation between display-name-style fields (which can change without restart) and runtime fields.
- **The 3-gate validation in `StateActor::handle_action`** (`state/mod.rs:377-394`) — Engine active + macro enabled + target app matches. The frontend can derive the same 3 gates for its running-state visibility (D-04). The frontend needs the step types to know if any `SustainedHold` exists for the "held" indicator (D-02).

### Established Patterns
- **Single-scheduler with BTreeMap timeline** (`ARCHITECTURE.md` §Pattern Overview) — The architectural pattern that makes parallel execution possible. Each macro's timers are keyed by `StepId { macro_id, step_index }` and the timeline merges them all. The planner should NOT spawn per-macro tasks (anti-pattern documented in `ARCHITECTURE.md`).
- **Two-phase dispatch** (`ARCHITECTURE.md` §Pattern Overview) — Scheduler sends `ActionReady`, StateActor validates and injects. This is the single bottleneck for input injection; informs the action-channel capacity choice (D-09).
- **Free-function + StateActor-wrapper for testability** (Phase 8 Plan 02 decision, `STATE.md` Decisions section) — If Phase 9 needs new helpers in `state/mod.rs`, the same pattern applies: free function in module scope, thin `&self` wrapper on `StateActor` that delegates, unit tests call the free function directly.
- **`#[serde(default)]` for backwards-compat** (Phase 8 PATTERN S-2) — Not directly relevant to Phase 9 (no schema changes), but the convention to follow if any new field is added.
- **`#[cfg(debug_assertions)] eprintln!` for debug-only logging** (`ARCHITECTURE.md` §Cross-Cutting Concerns) — The pattern for the drop counter's debug logging (D-10).

### Integration Points
- **`scheduler/mod.rs:362` `try_send` call** — One of three sites to add the drop counter. The other two are `start_macro:287` (HoldStart) and `release_holds:343` (HoldRelease).
- **`lib.rs:26` channel capacity literal** — Direct edit for the 100 → 1024 change (D-09).
- **`App.tsx:1202-1206` dot styling** — The starting point for the new running-state indicators. The current `bg-success shadow-[0_0_6px_var(--color-success-glow)]` is the "enabled" state; the firing indicator adds a Tailwind `animate-pulse` class, the held indicator uses a different color (e.g., `bg-accent` or `bg-info`).
- **`App.tsx:1280` `↗ Global` subtitle** — The existing pattern for inline status text on the macro card. Phase 9's "Waiting for {target_app}" subtitle (D-01) follows the same pattern, conditional on `macro.enabled && !firing && macro.target_app != null`.
- **`08-VERIFICATION.md` 7-section structure** — The template for `09-VERIFICATION.md`. Phase 9's verification artifact should mirror sections 1, 2, 5, 6, 7 from Phase 8 (skip 3 [Windows cross-compile] and 4 [profile backwards-compat] which are not Phase 9 concerns). Section 5 (macOS manual) and Section 6 (Windows manual) get the 3 new tests each (D-08).

</code_context>

<specifics>
## Specific Ideas

### Concrete examples from the discussion

**D-01 (enabled-but-waiting visual) example:**
- Macro "AFK Fish Farm" is enabled, target_app = `com.mojang.minecraftpe` (or equivalent bundle id)
- User opens TextEdit
- Card shows: solid green dot + small text "Waiting for Minecraft" (or "Waiting for {display name of target_app}")
- User switches to Minecraft
- Card shows: pulsing green dot, "Waiting for…" text disappears

**D-02 (held vs firing vs combined) example:**
- Macro "Hold Right-Click only" → solid blue/purple "held" indicator (or a small lock icon)
- Macro "Left-Click every 100ms only" → pulsing green "firing" indicator
- Macro "Hold Right-Click + Left-Click every 100ms" (AFK farm) → single combined indicator (e.g., a small label "Active (Hold + Click)" or a stacked icon that conveys both)

**D-08 manual test example (macOS T9.1):**
1. Create macro A: "Click A" with action = Left Click, interval = 200ms
2. Create macro B: "Click B" with action = Right Click, interval = 300ms
3. Enable A via toggle. Verify: A starts firing (pulsing dot).
4. Wait 1 second (A fires ~5 times).
5. Enable B via toggle. Verify: B starts firing. **A continues firing at 200ms** (don't pause, don't double-rate).
6. Disable A. Verify: A stops. **B continues firing at 300ms** (stopping one doesn't affect the other).
7. Expected: T9.2 is the same setup minus the "disable A" step (just verifies the concurrent fire portion).

### No specific UI framework or color preferences

The user did not pin a specific color for the "held" indicator (D-02) or the "active multi-step" combined indicator. The existing CSS custom properties in `src/App.css` (`--color-success`, `--color-accent`, `--color-info`, `--color-warning`, `--color-danger`) are the available palette. Claude's discretion to pick colors that fit the v2.0 design language.

### No new dependencies required

The drop counter (D-09) uses `std::sync::atomic::AtomicU64` from std — no new crate. The action_tx capacity change is a literal edit. The frontend derivation (D-04) is a pure helper function. No `Cargo.toml` or `package.json` changes are expected.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

All four areas selected for discussion resolved cleanly to specific decisions. No user-suggested features were redirected to the backlog.

</deferred>

---

*Phase: 9-parallel-macro-execution*
*Context gathered: 2026-07-02*
