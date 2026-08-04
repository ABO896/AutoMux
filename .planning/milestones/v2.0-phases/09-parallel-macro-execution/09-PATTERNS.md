# Phase 9: Parallel Macro Execution — Pattern Map

**Mapped:** 2026-07-20
**Files analyzed:** 4 modified (3 Rust + 1 TSX), no new files
**Analogs found:** 6 / 6 — every planned change has a strong in-file or sibling-file analog

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|----------------|
| `src-tauri/src/scheduler/mod.rs` — new `#[tokio::test] parallel_execution_test` (D-05/D-06/D-07) | test | event-driven | `afk_farm_stress_test` (`scheduler/mod.rs:404-498`) | exact (in-file, same module) |
| `src-tauri/src/scheduler/mod.rs` — `AtomicU64` drop counter at 3 `try_send` sites (D-09) | utility (instrumentation) | event-driven | existing `try_send` call sites themselves (`scheduler/mod.rs:287, 343, 362`) | exact (in-place) |
| `src-tauri/src/lib.rs` — `action_tx` capacity 100 → 1024 (D-09) | config | event-driven | self — same line, literal edit (`lib.rs:26`) | exact (in-place) |
| `src-tauri/src/ipc/mod.rs` — new debug-only `get_debug_action_drop_count` command (D-09) | IPC handler | request-response | `get_state` / other no-arg query commands (`ipc/mod.rs`) | role-match |
| `src/App.tsx` — `computeRunningState` derivation helper (D-01–D-04) | utility (pure derivation) | transform | `formatInputEvent` / `formatStep` (`App.tsx:69-73`) and the 3-gate logic in `handle_action` (`state/mod.rs:373-394`, mirrored on the frontend) | role-match |
| `src/App.tsx` — running-state dot + "Waiting for…" subtitle (D-01–D-03) | component | request-response | existing enabled/disabled dot (`App.tsx:1201-1207`) and the `↗ Global` subtitle pattern (`App.tsx:1279-1281`) | exact (in-place) |

---

## Pattern Assignments

### `src-tauri/src/scheduler/mod.rs` — new `parallel_execution_test`

**Analog:** `afk_farm_stress_test` (`scheduler/mod.rs:404-498`) and `jitter_audit_10ms_interval` (`scheduler/mod.rs:503-583`), both in the existing `#[cfg(test)] mod tests` block (`scheduler/mod.rs:393-584`).

**Setup pattern to copy** (channel sizes, scheduler spawn):
```rust
// scheduler/mod.rs:405-411 (existing — exact setup to copy)
let (intent_tx, intent_rx) = mpsc::channel::<SchedulerIntent>(100);
let (action_tx, mut action_rx) = mpsc::channel::<ActionReady>(256);

let scheduler = Scheduler::new(intent_rx, action_tx);
let scheduler_handle = tokio::spawn(async move {
    scheduler.run().await;
});
```

**MacroConfig literal to copy** (note all fields required, including the newer `trigger_modifiers` field added in Phase 8):
```rust
// scheduler/mod.rs:413-434 (existing — copy this shape for macro A and macro B)
let macro_id = Uuid::new_v4();
let config = MacroConfig {
    id: macro_id,
    name: "AFK Fish Farm".into(),
    interval_ms: 50,
    enabled: true,
    target_app: None,
    trigger_key: None,
    trigger_modifiers: 0,
    trigger_mode: crate::state::TriggerMode::Pulse,
    sequence: ActionSequence {
        steps: vec![ActionStep::InterleavedInterval {
            input: InputEvent::MouseButton(MouseButton::Left),
            interval_ms: 50,
        }],
    },
};
```

**Action-counter collection pattern (D-07)** — copy exactly, this is the "easy to debug" pattern the user picked:
```rust
// scheduler/mod.rs:458-477 (existing — the pattern for per-macro action counting)
let mut hold_starts = 0u32;
let mut hold_releases = 0u32;
let mut interval_fires = 0u32;

while let Ok(action) = action_rx.try_recv() {
    assert_eq!(action.macro_id, macro_id);
    match action.action_type {
        ActionType::HoldStart(InputEvent::MouseButton(MouseButton::Right)) => hold_starts += 1,
        ActionType::HoldRelease(InputEvent::MouseButton(MouseButton::Right)) => hold_releases += 1,
        ActionType::Interval(InputEvent::MouseButton(MouseButton::Left)) => interval_fires += 1,
        other => panic!("Unexpected action: {:?}", other),
    }
}
```
→ For Phase 9's two-macro scenario, the counter map keys on `action.macro_id` instead of asserting a single fixed id — e.g. `HashMap<Uuid, (u32 /* fires */, ...)>` accumulated the same way, then asserted per-macro with the same `>=`/`<=` bounded-range style used at lines 483-491.

**Scenario (a) — two macros at different intervals fire concurrently (D-06a):**
- Send two `SchedulerIntent::StartMacro(config)` for macro A (e.g. 50ms interval) and macro B (e.g. 80ms interval) back-to-back, `sleep(500ms)`, then `StopMacro` both, then drop `intent_tx` and await the scheduler handle exactly as in `afk_farm_stress_test` (lines 436-456).
- Assert macro A's fire count is roughly `500/50 = 10` (bounded range, mirroring lines 483-491) and macro B's is roughly `500/80 = 6`, independently — proving neither blocks the other.

**Scenario (b) — stopping A does not affect B (D-06b):**
- Start A and B, `sleep(200ms)`, `StopMacro(A)`, `sleep(300ms)` more, then `StopMacro(B)`.
- Assert A's total fire count stops growing after the stop point (bounded to what it should have fired in the first 200ms) while B's count reflects fires across the full ~500ms window.
- Both scenarios should be separate `#[tokio::test]` fns per D-06 ("separate test for clean test names and isolated failures") — do not combine into one mega-test.

---

### `src-tauri/src/scheduler/mod.rs` — `AtomicU64` drop counter (D-09/D-10/D-11)

**Analog:** the existing `try_send` call sites themselves — this is an in-place instrumentation addition, not a new pattern borrowed from elsewhere in the codebase. The debug-logging convention comes from ARCHITECTURE.md's `#[cfg(debug_assertions)] eprintln!` cross-cutting pattern, already used verbatim in `state/mod.rs:397-398`:

```rust
// state/mod.rs:397-398 (existing — the exact debug-eprintln convention to copy)
#[cfg(debug_assertions)]
eprintln!("[Action] macro={} {:?}", mac.name, action.action_type);
```

**Counter declaration** (add near the top of `scheduler/mod.rs`, alongside other statics/imports):
```rust
#[cfg(debug_assertions)]
static ACTION_DROP_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
```

**The 3 call sites to instrument** (each currently silently discards the `try_send` result):
```rust
// scheduler/mod.rs:287 (start_macro — HoldStart)
let _ = self.action_tx.try_send(ActionReady { .. });

// scheduler/mod.rs:343 (release_holds — HoldRelease)
let _ = self.action_tx.try_send(ActionReady { .. });

// scheduler/mod.rs:362 (fire_due_actions — Interval)
// @safety-officer: try_send backpressure — drops on overflow.
let _ = self.action_tx.try_send(ActionReady { .. });
```
→ Each becomes:
```rust
if self.action_tx.try_send(ActionReady { .. }).is_err() {
    #[cfg(debug_assertions)]
    {
        ACTION_DROP_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        eprintln!("[Scheduler] action_tx full — dropped action for macro={}", macro_id);
    }
}
```
> Keep the `@safety-officer:` comment above the `fire_due_actions` site — it documents the backpressure invariant and should not be deleted, only extended.

**Debug-only accessor** (for the IPC command):
```rust
#[cfg(debug_assertions)]
pub fn get_action_drop_count() -> u64 {
    ACTION_DROP_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}
```

---

### `src-tauri/src/lib.rs` — `action_tx` capacity change (D-09)

**Analog:** self — direct literal edit, no pattern transfer needed.

```rust
// lib.rs:26 (existing)
let (action_tx, action_rx) = mpsc::channel::<scheduler::ActionReady>(100);
```
→
```rust
let (action_tx, action_rx) = mpsc::channel::<scheduler::ActionReady>(1024);
```
`state_tx` (`lib.rs:23`, capacity 100) and `sched_tx` (`lib.rs:24`, capacity 100) are explicitly **out of scope** per D-11 — do not touch them.

---

### `src-tauri/src/ipc/mod.rs` — new `get_debug_action_drop_count` command (D-09)

**Analog:** any existing no-arg query `#[command]`, e.g. `get_state` (shape: `State<'_, StateManager>` in, `Result<T, String>` out, `#[cfg(debug_assertions)]` gated).

```rust
/// Debug-only diagnostic: exposes the scheduler's action-channel drop count
/// for ad-hoc parallel-execution diagnosis (D-09). Not present in release builds.
#[cfg(debug_assertions)]
#[command]
pub async fn get_debug_action_drop_count() -> Result<u64, String> {
    Ok(crate::scheduler::get_action_drop_count())
}
```
> Registration: add to the `invoke_handler!` list in `lib.rs`, gated the same way (`#[cfg(debug_assertions)]` around the registration line, or an always-present list with the function itself absent in release — follow whatever pattern other debug-only commands in this codebase already use; if none exist yet, gate at both the fn and the registration call).

---

### `src/App.tsx` — `computeRunningState` derivation helper (D-01–D-04)

**Analog (data source):** the Rust 3-gate check in `handle_action` (`src-tauri/src/state/mod.rs:373-394`) — this is the authoritative logic the frontend must mirror in read-only form (D-04 explicitly says: derive on frontend from existing `AppState` fields, do not add a backend field):

```rust
// src-tauri/src/state/mod.rs:373-394 (existing — the 3 gates to mirror in TS)
fn handle_action(&self, action: crate::scheduler::ActionReady) {
    // Gate 1: Engine must be active
    if !self.state.engine_active || self.state.emergency_stop_active {
        return;
    }
    // Gate 2: Macro must exist and be enabled
    let mac = match self.state.macros.get(&action.macro_id) {
        Some(m) if m.enabled => m,
        _ => return,
    };
    // Gate 3: Target app must match (or be Global)
    let matches_target = match &mac.target_app {
        Some(target) => self.state.active_app.as_deref() == Some(target.as_str()),
        None => true, // "Global"
    };
    if !matches_target {
        return;
    }
    // ...
}
```

**Analog (pure-helper shape on the frontend):** `formatInputEvent` / `formatStep` (`src/App.tsx:69-73`) — small top-level pure functions, no signals, take plain data in, return a plain value:

```typescript
// src/App.tsx:69-73 (existing — pure-helper pattern to copy the shape of)
if ("SustainedHold" in step)
  return `Hold ${formatInputEvent(step.SustainedHold.input)}`;
```

→ New helper (placed near `formatInputEvent`/`formatStep`, before the component body):
```typescript
type RunningState = "firing" | "held" | "combined" | "waiting" | "disabled";

/**
 * D-01–D-04: Pure derivation of a macro's visible running state.
 * Mirrors the 3 gates in Rust `StateActor::handle_action`
 * (state/mod.rs:373-394) plus a 4th check (does the macro have any
 * SustainedHold step) for the held/combined distinction (D-02).
 */
function computeRunningState(
  macro: MacroConfig,
  state: AppState
): RunningState {
  if (!macro.enabled) return "disabled";
  if (!state.engine_active || state.emergency_stop_active) return "disabled";

  const matchesTarget =
    macro.target_app == null || state.active_app === macro.target_app;
  if (!matchesTarget) return "waiting";

  const hasHold = macro.sequence.steps.some((s) => "SustainedHold" in s);
  const hasInterval = macro.sequence.steps.some((s) => "InterleavedInterval" in s);
  // Legacy empty-sequence macros fall back to a single interval click
  // (see scheduler/mod.rs:257-270) — treat as "firing".
  if (macro.sequence.steps.length === 0) return "firing";
  if (hasHold && hasInterval) return "combined";
  if (hasHold) return "held";
  return "firing";
}
```

---

### `src/App.tsx` — running-state dot & subtitle rendering (D-01–D-03)

**Analog:** the existing enabled/disabled dot (`App.tsx:1201-1207`) and the `↗ Global` inline-subtitle pattern (`App.tsx:1279-1281`, added in Phase 8 — same technique: a conditionally-rendered `<span>` next to the trigger-mode text).

```tsx
// App.tsx:1201-1207 (existing — the dot to extend)
<div
  class={`w-2 h-2 rounded-full ${
    macro.enabled
      ? "bg-success shadow-[0_0_6px_var(--color-success-glow)]"
      : "bg-text-dim"
  }`}
/>
```
→ Replace the ternary with a switch over `computeRunningState`:
```tsx
<div
  class={(() => {
    switch (computeRunningState(macro, state()!)) {
      case "firing":
        return "w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)] animate-pulse";
      case "held":
        return "w-2 h-2 rounded-full bg-accent shadow-[0_0_6px_var(--color-accent-glow)]";
      case "combined":
        return "w-2 h-2 rounded-full bg-info shadow-[0_0_6px_var(--color-info-glow)] animate-pulse";
      case "waiting":
        return "w-2 h-2 rounded-full bg-success shadow-[0_0_6px_var(--color-success-glow)]"; // solid, no pulse
      default:
        return "w-2 h-2 rounded-full bg-text-dim";
    }
  })()}
/>
```

**"Waiting for…" subtitle (D-01)** — same conditional-`<span>` technique as the existing `↗ Global` subtitle:
```tsx
// App.tsx:1279-1281 (existing — the pattern to clone for the waiting subtitle)
<span class="text-[10px] text-text-muted">({macro.trigger_mode})</span>
<span class="text-[10px] text-text-dim">↗ Global</span>
```
→ Add a new conditional span next to the macro name (near `App.tsx:1200-1209`, in the title row rather than the trigger-key row, per D-01's card layout):
```tsx
<Show when={computeRunningState(macro, state()!) === "waiting"}>
  <span class="text-[10px] text-text-dim">
    Waiting for {macro.target_app}
  </span>
</Show>
```
> Per D-03, this is per-card only — no global summary counter is added anywhere (e.g. not near the engine toggle at `App.tsx:740` or `:799`, which are existing analogous dots for the *engine* state, not per-macro state — do not conflate the two).

> **Combined-indicator label (D-02):** if a small text label is preferred over a color-only distinction for the "combined" case (Hold + Interval), follow the same inline-`<span>` technique used for `↗ Global` — e.g. `<span class="text-[10px] text-text-dim">Active (Hold + Click)</span>` — Claude's discretion per D-02/CONTEXT.md specifics section.

---

## Shared Patterns

### Pattern P-1: `#[cfg(debug_assertions)] eprintln!` for debug-only diagnostics
**Source:** `state/mod.rs:397-398` (existing, exact convention).
**Apply to:** the new drop-counter logging in `scheduler/mod.rs` (D-10) — silent in release, logged in debug.

### Pattern P-2: Action-counter test pattern (`action_rx.try_recv()` loop)
**Source:** `scheduler/mod.rs:458-477` (`afk_farm_stress_test`).
**Apply to:** the new `parallel_execution_test` (D-07) — same drain-and-count-by-variant loop, extended to count per `macro_id`.

### Pattern P-3: Bounded-range assertions for timer-based tests
**Source:** `scheduler/mod.rs:483-491` (`assert!(interval_fires >= 5 && interval_fires <= 20)`).
**Apply to:** both new parallel-execution test scenarios (D-06a, D-06b) — avoid exact-count or timing-based assertions (D-07 explicitly rejects fragile timing assertions).

### Pattern P-4: Literal-edit for channel capacity constants
**Source:** `lib.rs:23-26` (the 3 channel declarations, all in one place).
**Apply to:** the `action_tx` capacity bump (D-09) — a single-line, single-file change with no ripple effects (no other file references the literal `100`/`1024`).

### Pattern P-5: Pure top-level derivation function (no signals, no side effects)
**Source:** `formatInputEvent` / `formatStep` (`App.tsx:69-73`).
**Apply to:** `computeRunningState` (D-04) — must remain a pure function of `(macro, state)`, called inline in JSX, no new `createSignal`/`createMemo`.

---

## No Analog Found

None. Every planned Phase 9 change has a direct in-file or sibling-file analog — this phase is primarily additive instrumentation and a new test alongside two nearly-identical existing tests, plus a frontend derivation that mirrors existing backend logic and existing subtitle/dot rendering conventions.

| Concept | Where the pattern comes from |
|---------|-------------------------------|
| `parallel_execution_test` (2 scenarios) | `afk_farm_stress_test` + `jitter_audit_10ms_interval` (scheduler/mod.rs) |
| Action-channel drop counter | `#[cfg(debug_assertions)] eprintln!` convention (state/mod.rs:397-398) |
| `action_tx` capacity change | self — literal edit (lib.rs:26) |
| `get_debug_action_drop_count` IPC command | existing no-arg query command shape (ipc/mod.rs) |
| `computeRunningState` helper | `handle_action` 3-gate logic (state/mod.rs:373-394) mirrored + `formatInputEvent`/`formatStep` pure-helper shape (App.tsx:69-73) |
| Running-state dot & subtitle | existing enabled/disabled dot (App.tsx:1201-1207) + `↗ Global` subtitle pattern (App.tsx:1279-1281, added Phase 8) |

---

## Metadata

**Analog search scope:** `src-tauri/src/scheduler/mod.rs`, `src-tauri/src/state/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/ipc/mod.rs`, `src/App.tsx`, `.planning/phases/08-hotkey-reliability-conflict-safety/08-PATTERNS.md`.
**Files scanned:** 5 source files + 1 prior-phase pattern doc (no RESEARCH.md this phase — research explicitly skipped; CONTEXT.md's `<code_context>` section substituted for research-derived analogs).
**Pattern extraction date:** 2026-07-20.
