# Testing Patterns

**Analysis Date:** 2026-05-15

## Overview

Testing exists exclusively in the **Rust backend** (`src-tauri/src/`). The TypeScript
frontend (`src/`) has no test infrastructure — no test framework, no test files, and no
test scripts in `package.json`.

---

## Rust Test Framework

**Runner:** Cargo's built-in test runner (`#[test]` / `#[tokio::test]`)
- No external test framework (no `rstest`, `proptest`, etc.)
- Async tests use `#[tokio::test]` from the `tokio` crate

**Assertion Library:** Rust's built-in `assert!`, `assert_eq!`, `panic!`
- No third-party assertion crates

**Run Commands:**
```bash
cd src-tauri && cargo test              # Run all tests
cd src-tauri && cargo test -- --nocapture  # Run tests with eprintln! output visible
cd src-tauri && cargo test <test_name>    # Run a specific test by name
```

Note: No test commands exist in `package.json`. Frontend has zero test infrastructure.

---

## Test File Organization

**Location:** Tests are co-located with the module they test, inside a `#[cfg(test)]`
inline module at the bottom of the file.

**Files containing tests:**
- `src-tauri/src/scheduler/mod.rs` — 2 tests (scheduler behavior)
- `src-tauri/src/persistence.rs` — 1 test (serialization/memory)

**Naming:** Test functions use `snake_case` with descriptive names:
- `afk_farm_stress_test`
- `jitter_audit_10ms_interval`
- `large_config_memory_check`

**Structure:**
```
src-tauri/src/
├── scheduler/mod.rs     # #[cfg(test)] mod tests { ... }  — 2 tokio async tests
└── persistence.rs       # #[cfg(test)] mod tests { ... }  — 1 sync test
```

---

## Test Structure

**Module declaration pattern:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ActionSequence, ActionStep, InputEvent, MacroConfig, MouseButton};

    #[tokio::test]
    async fn afk_farm_stress_test() { ... }

    #[tokio::test]
    async fn jitter_audit_10ms_interval() { ... }
}
```

**Sync test pattern** (`persistence.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ActionSequence, ActionStep, InputEvent, MouseButton};

    #[test]
    fn large_config_memory_check() { ... }
}
```

---

## Test Types and Approach

### Integration / Stress Tests (scheduler)

Tests instantiate real `Scheduler` and `StateActor`-adjacent infrastructure via actual
Tokio channels. They do NOT mock the scheduler — they exercise the real timer loop.

**Pattern:** Arrange → Send intent → Sleep → Stop → Drain channel → Assert

```rust
#[tokio::test]
async fn afk_farm_stress_test() {
    // Arrange: create real channels
    let (intent_tx, intent_rx) = mpsc::channel::<SchedulerIntent>(100);
    let (action_tx, mut action_rx) = mpsc::channel::<ActionReady>(256);

    // Create real Scheduler (no mocking)
    let scheduler = Scheduler::new(intent_rx, action_tx);
    let scheduler_handle = tokio::spawn(async move {
        scheduler.run().await;
    });

    // Build a MacroConfig inline
    let macro_id = Uuid::new_v4();
    let config = MacroConfig {
        id: macro_id,
        name: "AFK Fish Farm".into(),
        // ... all fields specified explicitly, no factory helpers
    };

    // Act: start → wait → stop
    intent_tx.send(SchedulerIntent::StartMacro(config)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    intent_tx.send(SchedulerIntent::StopMacro(macro_id)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Shutdown: drop sender to close channel
    drop(intent_tx);
    let _ = scheduler_handle.await;

    // Drain result channel
    while let Ok(action) = action_rx.try_recv() { ... }

    // Assert counts and bounds
    assert_eq!(hold_starts, 1, "Exactly one HoldStart for right-click");
    assert!(interval_fires >= 5, "Expected ≥5 interval fires, got {}", interval_fires);
}
```

### Unit / Property Tests (persistence)

Tests that don't require async — plain `#[test]`. Builds data structures in memory and
validates serialization size + round-trip fidelity.

```rust
#[test]
fn large_config_memory_check() {
    let mut macros = HashMap::new();
    for i in 0..1000 {
        // Build MacroConfig inline, no factory
        macros.insert(id, MacroConfig { ... });
    }
    let json = serde_json::to_string(&profile).unwrap();
    assert!(json_size_kb < 1024, "JSON too large: {}KB", json_size_kb);

    let parsed: ProfileData = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.macros.len(), 1000);
}
```

---

## Mocking

**Framework:** None. No mocking library is used (`mockall`, `mock_it`, etc. are absent
from `Cargo.toml`).

**Strategy:** Tests use real implementations. The Scheduler's channel-based design makes
it naturally testable in isolation without mocking:
- `Scheduler` consumes an `mpsc::Receiver<SchedulerIntent>` and writes to an
  `mpsc::Sender<ActionReady>` — both provided by the test.
- The `StateActor` and platform I/O layer are NOT involved in scheduler tests.

**What to test without mocking:**
- Scheduler timer logic — observable via `ActionReady` messages in the output channel
- Persistence serialization — pure in-memory operations
- State machine transitions — the `Intent`/`StateActor` pattern is designed for this
  but no tests exist yet for it

**What NOT to test (requires real platform):**
- `InputProvider` trait implementations (macOS CGEvent, Windows SendInput) — these
  require real OS APIs
- `PlatformObserver` implementations — require system accessibility permissions
- Tauri IPC commands (`src-tauri/src/ipc/mod.rs`) — no tests exist; would require a
  Tauri test harness

---

## Fixtures and Factories

**No shared factory functions exist.** Each test constructs `MacroConfig` inline with all
fields specified explicitly. This is verbose but ensures tests are self-contained.

**Representative fixture pattern:**
```rust
let config = MacroConfig {
    id: Uuid::new_v4(),
    name: "Test Macro".into(),
    interval_ms: 50,
    enabled: true,
    target_app: None,
    trigger_key: None,
    trigger_mode: crate::state::TriggerMode::Pulse,
    sequence: ActionSequence {
        steps: vec![
            ActionStep::SustainedHold {
                input: InputEvent::MouseButton(MouseButton::Right),
            },
            ActionStep::InterleavedInterval {
                input: InputEvent::MouseButton(MouseButton::Left),
                interval_ms: 50,
            },
        ],
    },
};
```

**Recommendation for new tests:** Create a helper `fn make_macro(name, interval_ms) -> MacroConfig`
in the test module to reduce boilerplate, since every test builds this type.

---

## Coverage

**Requirements:** None enforced. No coverage tooling configured.

**View Coverage (manual):**
```bash
cd src-tauri && cargo tarpaulin --out Html  # requires cargo-tarpaulin installed
```

**Current coverage gaps (not tested):**
- `src-tauri/src/state/mod.rs` — `StateActor`, all `Intent` variants, targeting logic
- `src-tauri/src/ipc/mod.rs` — all IPC commands
- `src-tauri/src/platform/macos/` — all macOS input/observer code
- `src-tauri/src/platform/windows/mod.rs` — all Windows input/observer code
- `src/` — entire TypeScript frontend

---

## Test Comments and Assertions

Tests use `eprintln!` to emit diagnostic information visible with `-- --nocapture`.
This is the convention for "test telemetry" — not removed after writing:

```rust
eprintln!(
    "[Stress Test] hold_starts={}, hold_releases={}, interval_fires={}",
    hold_starts, hold_releases, interval_fires
);
```

Assertions include failure messages explaining the constraint:
```rust
assert_eq!(hold_starts, 1, "Exactly one HoldStart for right-click");
assert!(
    interval_fires >= 5,
    "Expected ≥5 interval fires, got {}",
    interval_fires
);
```

Role annotations appear in test doc comments to explain ownership:
```rust
// @safety-officer: AFK farm simulation test.
// Assertions — @safety-officer verified:
```

---

## Jitter / Timing Tests

The `jitter_audit_10ms_interval` test measures timer precision — a pattern unique to this
codebase because accuracy of input automation timing is a core product requirement.

**Pattern:**
1. Run an interval macro for N milliseconds
2. Collect `fired_at: Instant` timestamps from `ActionReady` messages
3. Compute inter-fire gap mean and standard deviation
4. Assert stddev is below a threshold (currently 3000µs for a 10ms interval)

```rust
let gaps: Vec<f64> = timestamps
    .windows(2)
    .map(|w| w[1].duration_since(w[0]).as_micros() as f64)
    .collect();
let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
let variance = gaps.iter().map(|g| (g - mean).powi(2)).sum::<f64>() / gaps.len() as f64;
let stddev = variance.sqrt();
assert!(stddev < 3000.0, "Jitter too high: stddev={:.0}µs (max 3000µs)", stddev);
```

New scheduler tests should follow this pattern for any timing-sensitive behavior.

---

## Frontend Testing (Not Present)

The frontend (`src/`) has no test setup. If adding frontend tests:
- **Recommended framework:** Vitest (already using Vite; add `vitest` to devDependencies)
- **Component testing:** `@solidjs/testing-library` for SolidJS component tests
- **Config file location:** `vite.config.ts` (Vitest can share the Vite config)

The explicit `id` attributes on interactive elements (`id="btn-create-macro"`,
`id="tab-dashboard"`, etc.) in `src/App.tsx` suggest these were added for testability —
they are ready to be targeted by DOM-based tests.

---

*Testing analysis: 2026-05-15*
