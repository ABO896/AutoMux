# Phase 9: Parallel Macro Execution - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-02
**Phase:** 9-parallel-macro-execution
**Areas discussed:** Running-state visibility, Parallel-execution verification test, Action-channel capacity & backpressure, Re-evaluate efficiency

---

## Running-state visibility

| Option | Description | Selected |
|--------|-------------|----------|
| Enabled but waiting — distinct state | Card shows the macro is 'armed' with a subtitle like 'Waiting for {target_app}'. Distinguishes enabled-but-waiting from enabled-and-firing. | ✓ |
| Same as enabled — no distinction | Card shows the same green dot as if it were firing, regardless of target match. Today's misleading behavior. | |
| Greyed out until firing | Card greys out the macro entirely when target doesn't match, even though enabled. | |
| You decide | (user delegated) | |

**User's choice:** Enabled but waiting — distinct state.
**Notes:** This is the primary deliverable for the area — the user wants users to know "I enabled this, why isn't it firing?" without checking the engine toggle or active app.

### Follow-up: Firing visual

| Option | Description | Selected |
|--------|-------------|----------|
| Pulsing green vs solid green | Firing dot has a subtle pulse animation; waiting is solid green. Reuses existing dot code at App.tsx:1202-1206. | |
| Different colors for each state | Firing uses saturated color (blue/purple) vs waiting green, with text label. | |
| Just a status text, no animation | Dot stays solid green; small text label appears next to macro name. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Pulsing green vs solid green" — minimal code, clear semantic.

### Follow-up: Where visible

| Option | Description | Selected |
|--------|-------------|----------|
| Per-card only | Only the per-macro card shows firing/waiting state. Engine toggle stays a single on/off switch. | |
| Add a count next to engine toggle | Engine toggle area shows '2 firing, 1 waiting' subtitle. | |
| Add a live actions-fired counter | Each card shows '342 clicks fired' live counter. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Per-card only" — least UI surface area, no extra info if user doesn't need it.

### Follow-up: Where computed

| Option | Description | Selected |
|--------|-------------|----------|
| Derive on the frontend | Frontend computes firing/waiting from existing state fields. No backend change. | |
| Add a `firing_macros` field to AppState | StateActor computes and emits a `firing_macros: HashSet<Uuid>` on every state-changed. | |
| Add a `MacroState` enum (Disabled/Waiting/Firing/Held) | Per-macro enum updated by StateActor. Bigger refactor. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Derive on the frontend" — cleanest, no backend change, no drift risk.

### Follow-up: Hold vs fire

| Option | Description | Selected |
|--------|-------------|----------|
| Same indicator for both | Both SustainedHold and InterleavedInterval show the same 'firing' indicator. | |
| Distinct indicators (held = one color, firing = another) | SustainedHold shows 'held' indicator (e.g., filled/locked icon); InterleavedInterval shows 'firing'. | ✓ |
| Show action description next to the dot | 'Holding Right-Click' or 'Firing Left Click every 100ms' text. | |
| You decide | (user delegated) | |

**User's choice:** Distinct indicators.
**Notes:** The user wants the visual distinction. Claude decides the specifics (color, icon) within the existing CSS custom properties.

### Follow-up: Combined state (Hold + Interval in one macro)

| Option | Description | Selected |
|--------|-------------|----------|
| Single combined indicator | One indicator that conveys 'active multi-step'. A small label or combined icon. | |
| Stack both indicators | Show both indicators stacked or side-by-side. | |
| Treat multi-step as 'Firing' only | Just show 'Firing' (the pulsing dot) for any active macro. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Single combined indicator" — keeps the card uncluttered.

---

## Parallel-execution verification test

| Option | Description | Selected |
|--------|-------------|----------|
| Unit + manual device tests on both platforms | New `parallel_execution_test` in scheduler tests + manual tests on macOS and Windows. | ✓ |
| Unit test only | Skip manual device tests. | |
| Manual device tests only | Skip the unit test, rely on manual tests. | |
| You decide | (user delegated) | |

**User's choice:** Unit + manual device tests on both platforms.
**Notes:** Mirrors Phase 8's test discipline (08-VERIFICATION.md sections 5+6).

### Follow-up: Test scenarios

| Option | Description | Selected |
|--------|-------------|----------|
| Full scenario matrix | (a) two macros concurrent fire, (b) stop-independence, (c) conflict scenario. | |
| Concurrent fire only | Just (a). | |
| Concurrent fire + stop-independence | (a) and (b) only. (c) is covered by Phase 8 tests. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Concurrent fire + stop-independence" — covers the two parallel-specific success criteria; conflict scenario is already covered.

### Follow-up: Test design

| Option | Description | Selected |
|--------|-------------|----------|
| Action-counter assertions | Use `action_rx.try_recv()` to count per macro, assert counts. Matches existing test pattern. | |
| Timing-based assertions | Measure gap between each macro's actions. Stronger proof but fragile. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Action-counter assertions" — matches existing test style, easy to debug.

### Follow-up: Manual test count

| Option | Description | Selected |
|--------|-------------|----------|
| Focused manual tests (3+3) | 3 macOS + 3 Windows manual tests, one per success criterion. | |
| Comprehensive manual tests (5+5) | Add edge cases (Hold + Interval, three macros, conflict with three). | |
| Minimal manual tests (1+1) | Just one test per platform. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Focused manual tests (3+3)" — mirrors Phase 8 structure, focused on parallel behavior.

---

## Action-channel capacity & backpressure

| Option | Description | Selected |
|--------|-------------|----------|
| Increase capacity + add drop counter | Increase to 1024 + AtomicU64 drop counter in debug builds. | |
| Increase capacity only | Just increase to 1024. | |
| Leave it for a future phase | Don't change. 100 is enough for AFK farm. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Increase capacity + add drop counter" — addresses CONCERNS.md flag, debug-only counter is non-invasive.

### Follow-up: Drop behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Silent drop, but log + counter in debug builds | Log drop + increment counter in debug. Silent in release. | |
| Surface drops to the user as a warning | Emit `action-channel-overflow` Tauri event + warning toast. | |
| Switch to `.await` (no drops) | Block until space available. Could cause timer loop to slip. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Silent drop, but log + counter in debug builds" — matches existing debug-only logging discipline.

### Follow-up: All channels

| Option | Description | Selected |
|--------|-------------|----------|
| Just the action channel | Drop counter only on action_tx. | |
| All three channels | Counter on all three channels. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Just the action channel" — focused on the actual concern (action channel is the one flagged).

---

## Re-evaluate efficiency

| Option | Description | Selected |
|--------|-------------|----------|
| In scope — optimize per-macro | Per-macro StartMacro/StopMacro for SetMacroEnabled, SetMacroTargetApp, UpdateSequence, UpdateStepInterval. | |
| Out of scope — keep full sweep | Leave `reevaluate_all_macros` as-is. CR-04 makes it cheap. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Out of scope — keep full sweep" — phase focus is correctness (success criteria), not performance.

### Follow-up: Other optimizations

| Option | Description | Selected |
|--------|-------------|----------|
| Just the per-macro re-evaluate (or nothing) | No debouncing or related optimizations. | |
| Re-evaluate + debounce rapid mutations | Also debounce consecutive SetMacroEnabled intents. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Just the per-macro re-evaluate (or nothing)" — minimal scope.

### Follow-up: CR-04 no-op check

| Option | Description | Selected |
|--------|-------------|----------|
| Leave CR-04 as-is | Full RunningConfig compare. Restart on any change. | |
| Tighten CR-04 to skip restart on non-runtime field changes | Separate runtime-config check. | |
| You decide | (user delegated) | ✓ |

**User's choice:** You decide.
**Notes:** Claude picks "Leave CR-04 as-is" — current behavior is well-defined and edge-case-free.

---

## Claude's Discretion

Items where the user said "you decide" or deferred to the agent:

1. **Firing visual** (D-02): Claude picks "Pulsing green vs solid green" — minimal code, clear semantic.
2. **Where visible** (D-03): Claude picks "Per-card only" — least UI surface.
3. **Where computed** (D-04): Claude picks "Derive on the frontend" — no backend change, no drift risk.
4. **Combined state for Hold + Interval** (D-02 extension): Claude picks "Single combined indicator" — keeps card uncluttered.
5. **Test scenarios** (D-06): Claude picks "Concurrent fire + stop-independence" — covers parallel-specific success criteria only.
6. **Test design** (D-07): Claude picks "Action-counter assertions" — matches existing pattern.
7. **Manual test count** (D-08): Claude picks "Focused manual tests (3+3)" — mirrors Phase 8 structure.
8. **Action channel capacity** (D-09): Claude picks "Increase capacity + add drop counter" — addresses CONCERNS.md.
9. **Drop behavior** (D-10): Claude picks "Silent drop, but log + counter in debug builds".
10. **All channels** (D-11): Claude picks "Just the action channel" — focused concern.
11. **Re-evaluate scope** (D-12): Claude picks "Out of scope" — focus on correctness.
12. **Other optimizations** (D-13): Claude picks "No debouncing or related optimizations" — minimal scope.
13. **CR-04** (D-14): Claude picks "Leave CR-04 as-is" — current behavior is well-defined.

## Deferred Ideas

None — discussion stayed within phase scope. No user-suggested features were redirected to the backlog.
