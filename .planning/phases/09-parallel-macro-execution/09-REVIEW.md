---
phase: 09-parallel-macro-execution
reviewed: 2026-07-20T18:05:57Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/scheduler/mod.rs
  - src/App.tsx
findings:
  critical: 1
  warning: 2
  info: 3
  total: 6
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-07-20T18:05:57Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

This phase (parallel-macro-execution) touches four files across three commits: (1) a debug-only
`action_tx` overflow diagnostic (counter + IPC command + channel capacity bump 100→1024) in
`scheduler/mod.rs`/`ipc/mod.rs`/`lib.rs`, (2) two new concurrency-proof scheduler tests, and (3) a
new frontend `computeRunningState()` derivation in `App.tsx` that renders a per-macro
firing/held/combined/waiting/disabled indicator.

The Rust-side diagnostic work (drop counter, capacity increase, debug-gated IPC command) is sound —
`cargo check` and `cargo clippy` (both dev and release profiles) are clean, and the cfg-gating is
correctly wired end-to-end (`lib.rs` registration, `scheduler::get_action_drop_count`,
`ipc::get_debug_action_drop_count`).

The frontend running-state indicator has a real correctness gap: it derives "held" vs "firing"
purely from the *persisted* `sequence.steps` shape, but the scheduler only performs the
Hold-mode→SustainedHold conversion on its own in-memory copy at dispatch time — that conversion is
never written back to `AppState`/persisted `MacroConfig`. Since the only macro-creation path
currently wired into the UI (`handleCreateMacro`) always stores a single `InterleavedInterval` step
regardless of the selected trigger mode, every Hold-mode macro created through the app will be
misclassified as "firing" instead of "held". This is traced in detail below (CR-01). The two new
scheduler stress tests are functionally sound but rely on generous-but-still-timing-dependent sleep
windows that carry a real (if bounded) CI-flakiness risk (WR-02).

## Critical Issues

### CR-01: `computeRunningState` never reports "held" for Hold-mode macros created via the app UI

**File:** `src/App.tsx:90-107` (also implicated: `src/App.tsx:460-472`, `src-tauri/src/scheduler/mod.rs:272-294`, `src-tauri/src/state/mod.rs:448-478`)

**Issue:**
`computeRunningState` (added this phase, `09-02`) derives the `held` / `combined` / `firing`
distinction solely from the *stored* `macro.sequence.steps` shape:

```ts
// src/App.tsx:102-106
const hasHold = macro.sequence.steps.some((s) => "SustainedHold" in s);
const hasInterval = macro.sequence.steps.some((s) => "InterleavedInterval" in s);
if (hasHold && hasInterval) return "combined";
if (hasHold) return "held";
return "firing";
```

But the scheduler performs Hold-mode conversion on a *local, ephemeral* copy of the steps that is
never written back to persisted state:

```rust
// src-tauri/src/scheduler/mod.rs:287-294
if config.trigger_mode == crate::state::TriggerMode::Hold {
    // Force all steps to be SustainedHold when in Hold mode
    for step in &mut steps {
        if let ActionStep::InterleavedInterval { input, .. } = step {
            *step = ActionStep::SustainedHold { input: *input };
        }
    }
}
```

`Intent::AddMacro` (`src-tauri/src/state/mod.rs:448-478`) stores the `MacroConfig` exactly as
submitted by the frontend (`self.state.macros.insert(new_id, config)`), with no equivalent
conversion. And the only macro-creation UI path, `handleCreateMacro`, always builds a single
`InterleavedInterval` step regardless of the chosen trigger mode:

```ts
// src/App.tsx:460-472
const config: MacroConfig = {
  ...
  sequence: {
    steps: [{ InterleavedInterval: { input, interval_ms: interval } }],
  },
  ...
  trigger_mode: newMacroTriggerMode(),   // "Hold" is a valid selection here
};
```

Net effect: a macro created with Trigger Mode = "Hold" is actually held down continuously at
runtime (per the scheduler's forced conversion), but `AppState.macros[id].sequence.steps` — the
only thing `computeRunningState` inspects — still contains an `InterleavedInterval` step, so the
indicator shows "firing" (green, `animate-pulse`) instead of "held" (accent, static). Since there is
currently no UI path (`set_macro_sequence` / `update_step_interval` are unused in `App.tsx`) that
ever produces a genuine `SustainedHold` step, the "held" state this phase's commit message
advertises ("add per-macro running-state indicator (firing/held/combined/waiting/disabled)") is
unreachable for any macro created through the product — the entire Hold trigger mode is
misrepresented. This directly undermines the phase's stated goal (letting a user visually confirm
"why isn't it firing / is it actually held") for one of only two trigger modes.

**Fix:** Base the derivation on `trigger_mode`, not (or not only) on the raw stored steps — mirror
the scheduler's actual runtime conversion:

```ts
function computeRunningState(macro: MacroConfig, state: AppState): RunningState {
  if (!macro.enabled) return "disabled";
  if (!state.engine_active || state.emergency_stop_active) return "disabled";

  const matchesTarget =
    macro.target_app == null || state.active_app === macro.target_app;
  if (!matchesTarget) return "waiting";

  if (macro.sequence.steps.length === 0) {
    return macro.trigger_mode === "Hold" ? "held" : "firing";
  }

  // Mirror scheduler/mod.rs: Hold mode forces every step to SustainedHold at
  // dispatch time, regardless of what's persisted in sequence.steps.
  if (macro.trigger_mode === "Hold") return "held";

  const hasHold = macro.sequence.steps.some((s) => "SustainedHold" in s);
  const hasInterval = macro.sequence.steps.some((s) => "InterleavedInterval" in s);
  if (hasHold && hasInterval) return "combined";
  if (hasHold) return "held";
  return "firing";
}
```

(Adjust exact semantics to match whatever the intended UX is for a Hold-mode macro whose stored
sequence *also* contains an independent `SustainedHold` step from a different input — but the core
fix is: `trigger_mode === "Hold"` must not be ignored.)

## Warnings

### WR-01: Duplicate `computeRunningState()` invocations per macro card risk logic drift

**File:** `src/App.tsx:1233, 1248, 1253`

**Issue:** Within a single `<For each={macroList()}>` item, `computeRunningState(macro, state()!)`
is called three separate times with identical arguments — once for the status-dot class (line
1233), once for the "waiting" label `<Show>` (line 1248), and once for the "combined" label
`<Show>` (line 1253). Because it's re-derived independently at each call site rather than computed
once and reused, a future edit to the classification logic (e.g. as part of fixing CR-01) risks
being applied at one call site and missed at another, producing a UI where the dot color and the
inline label text disagree.

**Fix:** Compute once per card and reuse:

```tsx
<For each={macroList()}>
  {(macro) => {
    const runningState = () => computeRunningState(macro, state()!);
    return (
      <div class="glass-card p-4">
        ...
        <div class={/* switch on runningState() */} />
        ...
        <Show when={runningState() === "waiting"}>...</Show>
        <Show when={runningState() === "combined"}>...</Show>
      </div>
    );
  }}
</For>
```

### WR-02: New parallel-scheduler stress tests are timing-dependent and can flake under CI load

**File:** `src-tauri/src/scheduler/mod.rs:653-755` (`parallel_two_macros_concurrent`), `src-tauri/src/scheduler/mod.rs:759-869` (`parallel_stop_one_keeps_other`)

**Issue:** Both new tests assert on real wall-clock `tokio::time::sleep` windows (500ms / 200ms /
300ms) and bound the resulting fire counts with fixed numeric ranges (e.g. `(5..=16)`,
`(3..=11)`, `a_count <= 10`, `b_count >= 6`). `cargo test` runs the test binary's tests
concurrently on the OS thread pool by default, and CI runners (especially shared/throttled ones)
can introduce scheduling delays well beyond what a local dev machine sees. A sufficiently starved
run could push a macro's fire count outside these bounds even though the scheduler behaved
correctly, causing an intermittent, non-actionable test failure. This is explicitly a test
*reliability* concern (not a style nit) since flaky CI tests erode trust in the suite and get
reflexively re-run or skipped.

**Fix:** Either widen the bounds further with a documented rationale, run these two tests with
`--test-threads=1` for this module (or `#[serial]`-style isolation) to reduce contention, or assert
on relative fire-count *ratios* (e.g. `b_count as f64 / a_count as f64` within a ratio window) rather
than absolute counts tied to a specific wall-clock window, which is more robust to uniform
system-wide slowdown.

## Info

### IN-01: "firing" and "waiting" states are both rendered in `bg-success` (green), distinguished only by `animate-pulse`

**File:** `src/App.tsx:1234-1241`

**Issue:** The `firing` case and the `waiting` case both resolve to the same `bg-success` color;
the only visual differentiator is the `animate-pulse` class on `firing`. For a static screenshot,
a user with `prefers-reduced-motion` enabled, or simply a quick glance, "actively firing" and
"enabled but target app doesn't match" are indistinguishable — despite being semantically very
different (one is doing input injection right now, the other is not).

**Fix:** Consider a distinct color (or an icon/hollow-ring treatment) for `waiting` so it doesn't
read as a healthy/active state at a glance.

### IN-02: "Waiting for {macro.target_app}" shows the raw platform identifier, not a friendly name

**File:** `src/App.tsx:1248-1252`

**Issue:** `macro.target_app` stores the raw bundle ID (macOS, e.g. `com.mojang.minecraft`) or full
exe path (Windows), per the `RunningApp`/`target_app` field documentation in this same file
(`src/App.tsx:112-114`). The new "waiting" label surfaces this raw identifier directly to the user
(`Waiting for {macro.target_app}`) rather than a resolved display name, which is inconsistent with
how the process picker itself renders `display_name` elsewhere in the same file.

**Fix:** Resolve `macro.target_app` against the last-fetched `apps()` list (matching on
`identifier`) to show `display_name` when available, falling back to the raw identifier only if no
match is found.

### IN-03: Debug-only `get_debug_action_drop_count` command has no consumer

**File:** `src-tauri/src/ipc/mod.rs:227-231`, `src-tauri/src/lib.rs:105-106`

**Issue:** The new `get_debug_action_drop_count` IPC command is registered and correctly cfg-gated
to debug builds, but nothing in `src/App.tsx` (or any other reviewed file) invokes it — it's
presumably intended to be polled manually via devtools during ad-hoc diagnosis per its doc comment
("ad-hoc parallel-execution overflow diagnosis (D-09)"). This is not a functional defect, but worth
flagging so it isn't mistaken for dead code in a future pass — if the intent was for this counter
to eventually back a visible debug HUD, that wiring is not yet present.

**Fix:** None required if this is intentionally a manual/devtools-only diagnostic; otherwise wire it
into a debug-build-only UI element.

---

_Reviewed: 2026-07-20T18:05:57Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
