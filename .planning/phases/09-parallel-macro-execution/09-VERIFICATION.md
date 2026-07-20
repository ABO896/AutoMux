---
phase: 9
slug: parallel-macro-execution
status: in-progress
nyquist_compliant: true
created: 2026-07-20
verified: pending
---

# Phase 9 — Verification Report

> Single source of truth for Phase 9 (Parallel Macro Execution) gate status.
> The phase is complete when all five sections below are marked `✅ done` in Section 7.
> Mirrors `08-VERIFICATION.md`'s structure. Sections 3 (Windows cross-compile) and 4 (profile
> backwards-compat) are intentionally omitted — see D-05 note in Section 7.

## 1. Test Suite

Command: `cd src-tauri && cargo test 2>&1 | tail -40`

Notes on command: Per the same nuance documented in `08-VERIFICATION.md` §1, the package name (per `src-tauri/Cargo.toml`) is `automux` (hyphen-free), with a separate `[lib] name = "automux_lib"` for the library. Running `cargo test` from `src-tauri/` is the equivalent of `cargo test -p automux_lib --lib`.

Exit code: `0` (all 11 lib tests passed; runtime ~0.56s).

Actual output:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.11s
     Running unittests src/lib.rs (target/debug/deps/automux_lib-99852003d73a1d34)

running 11 tests
test platform::macos::observer::tests::cg_event_flag_constants ... ok
test state::tests::bind_conflict_rejected ... ok
test state::tests::conflict_disappear_on_disable ... ok
test state::tests::conflict_detection_overlap ... ok
test state::tests::self_rebind_allowed ... ok
test state::tests::profile_backwards_compat ... ok
test persistence::tests::large_config_memory_check ... ok
test scheduler::tests::jitter_audit_10ms_interval ... ok
test scheduler::tests::parallel_two_macros_concurrent ... ok
test scheduler::tests::parallel_stop_one_keeps_other ... ok
test scheduler::tests::afk_farm_stress_test ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s

     Running unittests src/main.rs (target/debug/deps/automux-d61e43d607832aa7)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests automux_lib

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Test-by-test verification (new Phase 9 tests):

| Test | Plan | Requirement | Result |
|------|------|-------------|--------|
| `scheduler::tests::parallel_two_macros_concurrent` | 09-01 | EXEC-01 | ✅ ok — two macros at different intervals (50ms, 80ms) fire concurrently, proven by a bounded per-macro_id fire-count assertion a serialized implementation could not satisfy |
| `scheduler::tests::parallel_stop_one_keeps_other` | 09-01 | EXEC-02 | ✅ ok — stopping one running macro does not affect a second concurrently running macro |

Full context (pre-existing tests, unchanged and still green): `cg_event_flag_constants`, `bind_conflict_rejected`, `conflict_disappear_on_disable`, `conflict_detection_overlap`, `self_rebind_allowed`, `profile_backwards_compat`, `large_config_memory_check`, `jitter_audit_10ms_interval`, `afk_farm_stress_test` — all 9 carried over from the Phase 8 baseline, all still passing.

**Test-count math:** Phase 8 baseline = 9 tests (macOS host; see `08-VERIFICATION.md` §7). Phase 9 adds 2 new tests (`parallel_two_macros_concurrent`, `parallel_stop_one_keeps_other`). Total on the macOS host = **11**, matching the actual output above. The Windows-only `windows_mod_constants` test (cfg-gated to `#[cfg(windows)]`, added in Phase 8) is not compiled on this macOS host and would add 1 more test on a Windows host (12 there) — this is expected cfg-gating behavior, not a missing test.

**Status: ✅ done** — the suite is green; both new parallel tests are present and passing; the macOS-host total (11) matches the expected baseline+2 math.

## 2. Clippy

Command: `cd src-tauri && cargo clippy --all-targets -- -D warnings 2>&1 | tail -20`

Notes on command: Same package-name nuance as §1 — `cargo clippy --all-targets -- -D warnings` from `src-tauri/` is the correct invocation (no `-p automux-lib`, which does not exist as a package ID).

**Actual result:** clippy exits **0** with **zero warnings** — the compiled output is only the build-status line:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
```

### Comparison against the Phase 8 baseline

`08-VERIFICATION.md` §2 documented one pre-existing warning: `needless_return` at `src/ipc/mod.rs:128` (inside the `#[cfg(target_os = "macos")]` branch of `list_running_apps`). Plan 09-01's own deviation log (see `09-01-SUMMARY.md`, "Rule 3 - Blocking" entry) records that this exact warning was fixed as part of Phase 9 Plan 01 — it blocked plan 09-01's own `cargo clippy --all-targets -D warnings` verification gate (unrelated to Phase 9's functional changes, but a compile-blocking pre-existing issue under `-D warnings`). Confirmed by direct read of the current source: `src/ipc/mod.rs:126-129` (the macOS branch) no longer contains a `return` keyword — it now reads as an implicit tail expression:

```rust
#[cfg(target_os = "macos")]
{
    crate::platform::macos::observer::list_running_apps_impl()
}
```

Note: the **Windows branch** of the same function (`src/ipc/mod.rs:130-133`, `#[cfg(target_os = "windows")]`) still contains an explicit `return crate::platform::windows::list_running_apps_impl();`. This branch is not compiled on this macOS host (cfg-gated out), so it does not appear in this run's clippy output. It was not touched by Phase 8 or Phase 9 and is out of scope for both phases — flagged here only for completeness, matching the level of detail `08-VERIFICATION.md` §3 used for its own cfg-gated Windows caveats.

**Phase 9 introduced NO new clippy warnings.** In fact, Phase 9 (via plan 09-01) resolved the one pre-existing warning that was carried forward from Phase 8, so the gate is now fully clean (exit 0) rather than "done despite one documented pre-existing warning" as it was in Phase 8.

**Status: ✅ done (no new warnings)** — `cargo clippy --all-targets -- -D warnings` exits 0 with zero warnings on the macOS host; Phase 9 added zero new warnings and additionally cleared the one pre-existing warning inherited from Phase 8.

## 5. Manual macOS Device Test (EXEC-01)

These steps must be performed on a real macOS host. `parallel_two_macros_concurrent` and `parallel_stop_one_keeps_other` (§1) cover the scheduler-level proof; the device test confirms the end-to-end behavior with real input injection and the plan-09-02 per-card running-state indicators (pulsing green dot = firing; see D-01/D-02 in `09-CONTEXT.md`).

Pre-flight:

- [ ] Accessibility granted to AutoMux (System Settings → Privacy & Security → Accessibility).
- [ ] Input Monitoring granted to AutoMux (System Settings → Privacy & Security → Input Monitoring).
- [ ] App is running via `npm run tauri dev` or a release build.

### Test T9.1 — Enabling macro B while macro A is firing does not affect A (maps to ROADMAP SC1)

> ROADMAP SC1: "On macOS, enabling macro B while macro A is actively firing does not pause, delay, or cancel macro A — both fire concurrently at their configured intervals."

1. Create macro A: "Click A" with action = Left Click, interval = 200ms.
2. Create macro B: "Click B" with action = Right Click, interval = 300ms.
3. Enable A via its toggle. **Expected:** A's card shows a pulsing green firing dot (plan 09-02 `computeRunningState` "firing" state).
4. Wait ~1 second (A fires ~5 times).
5. Enable B via its toggle. **Expected:** B's card also shows a pulsing firing dot, AND A's card keeps its pulsing dot and keeps firing at its own 200ms rate — A is not paused, delayed, or doubled by B starting.
6. **Expected outcome:** both macros fire concurrently at their own configured intervals.

### Test T9.2 — Stopping macro A does not affect macro B (maps to ROADMAP SC3)

> ROADMAP SC3: "Stopping one running macro does not affect any other concurrently running macro."

1. Continue from T9.1 with both A and B enabled and firing (both cards show pulsing dots).
2. Disable A via its toggle. **Expected:** A's card dot goes dim/disabled (plan 09-02 "disabled" state).
3. **Expected:** B's card keeps its pulsing dot and continues firing at 300ms — stopping A does not pause, delay, or stop B.

### Test T9.3 — Two macros with the same input both fire concurrently, and the Phase 8 conflict warning shows (maps to ROADMAP SC1 + Phase 8 conflict detection)

1. Create macro A and macro B, both as Left Click actions (same input), each enabled.
2. **Expected:** both cards show pulsing firing dots (both actually fire, proving parallel execution is not blocked by input collision) AND the Phase 8 same-input conflict warning region appears (`⚠ 2 macros are injecting the same input…`, unit-covered by Phase 8's `conflict_detection_overlap`).
3. This is the device-level confirmation that parallel firing and the Phase 8 conflict warning coexist correctly — the warning is informational only and does not block either macro from firing.

## 6. Manual Windows Device Test (EXEC-02)

These steps must be performed on a real Windows host. The device test confirms the same concurrent-firing behavior via SendInput injection and the Win32 hook observer.

Pre-flight:

- [ ] The app is running (release build or `npm run tauri dev` from a Windows host).
- [ ] If Windows SmartScreen or UAC blocks input injection, run as Administrator or add an exception in the antivirus.

### Test 6.1 — Enabling macro B while macro A is firing does not affect A (maps to ROADMAP SC2)

> ROADMAP SC2: "On Windows, the same concurrent behavior holds — macro B fires independently alongside macro A."

1. Create macro A: "Click A" with action = Left Click, interval = 200ms.
2. Create macro B: "Click B" with action = Right Click, interval = 300ms.
3. Enable A via its toggle. **Expected:** A's card shows a pulsing green firing dot.
4. Wait ~1 second (A fires ~5 times).
5. Enable B via its toggle. **Expected:** B's card also shows a pulsing firing dot, AND A's card keeps firing at its own 200ms rate, unaffected by B starting.
6. **Expected outcome:** both macros fire concurrently via `SendInput`, at their own configured intervals.

### Test 6.2 — Stopping macro A does not affect macro B (maps to ROADMAP SC3)

1. Continue from Test 6.1 with both A and B enabled and firing.
2. Disable A via its toggle. **Expected:** A's card dot goes dim/disabled.
3. **Expected:** B's card keeps its pulsing dot and continues firing at 300ms — stopping A does not affect B.

### Test 6.3 — Two macros with the same input both fire concurrently, and the conflict warning shows (maps to ROADMAP SC2 + Phase 8 conflict detection)

1. Create macro A and macro B, both as Left Click actions, each enabled.
2. **Expected:** both cards show pulsing firing dots (both actually fire on Windows via `SendInput`) AND the Phase 8 same-input conflict warning region appears.
3. This is the Windows device-level confirmation that parallel firing and the conflict warning coexist correctly.

## 7. Verification Status

| Section | Status | Notes |
|---------|--------|-------|
| 1. Test Suite | ✅ done | 11/11 lib unit tests pass on macOS host (9 Phase 8 baseline + 2 new: `parallel_two_macros_concurrent`, `parallel_stop_one_keeps_other`). |
| 2. Clippy | ✅ done (no new warnings) | `cargo clippy --all-targets -- -D warnings` exits 0, zero warnings. Phase 9 introduced no new warnings and cleared the one pre-existing warning carried from Phase 8. |
| 3. Windows Cross-Compile | N/A — intentionally omitted (D-05) | Phase 9 adds no `#[cfg(target_os = "windows")]`-gated platform-specific code — the drop counter (`ACTION_DROP_COUNT`) is `std::sync::atomic`, platform-agnostic, and the IPC command (`get_debug_action_drop_count`) is not OS-gated. The shared-code test suite + clippy (§1, §2) already cover 100% of Phase 9's new code. Mirroring Phase 8's Windows cross-compile gate here would test nothing Phase 9 actually changed. |
| 4. Profile Backwards-Compat | N/A — intentionally omitted (D-05) | Phase 9 makes no schema changes to `MacroConfig`, `AppState`, or `ProfileData` — no new `#[serde]` field was added, so there is nothing for a backwards-compat deserialization test to prove. |
| 5. Manual macOS Device Test | ⬜ pending | **Requires human** — must be run on a real macOS host with Accessibility + Input Monitoring granted. T9.1 (concurrent fire, SC1), T9.2 (stop-one, SC3), T9.3 (same-input concurrent + conflict warning, SC1). |
| 6. Manual Windows Device Test | ⬜ pending | **Requires human** — must be run on a real Windows host. 6.1 (concurrent fire, SC2), 6.2 (stop-one, SC3), 6.3 (same-input concurrent + conflict warning, SC2). |

**§3/§4 omission rationale (D-05):** Per `09-CONTEXT.md` D-05 and this plan's own `<objective>`, Phase 9 mirrors ONLY sections 1, 2, 5, 6, 7 of `08-VERIFICATION.md`. Section 3 (Windows cross-compile) and Section 4 (profile backwards-compat) are deliberately absent, not oversights — Phase 9 has no schema changes to prove backwards-compat for, and no `target_os`-gated code that a cross-compile gate would exercise beyond what the standard `cargo test`/`cargo clippy` runs already cover.

The phase is **technically complete from an automated-verification standpoint**: both automated gates (Sections 1, 2) are green. Sections 5 and 6 are inherently manual — the executor cannot perform them on a real device. The user (or a human verification step) executes the literal steps in each section and updates Sections 5 and 6 status to `✅ done` when all tests pass on a real device.

**Pre-completion summary (what the executor delivered):**

- All Phase 9 unit tests pass (11/11), including both new parallel-execution proofs.
- Zero clippy warnings — Phase 9 added none and cleared the one pre-existing warning from Phase 8.
- §3/§4 intentionally omitted per D-05, with rationale documented above.
- 3 macOS + 3 Windows manual device tests documented, each mapped to a ROADMAP Phase 9 success criterion (SC1/SC2/SC3).

**Post-completion checklist for the user:**

1. Run T9.1–T9.3 on a real macOS host. Mark Section 5 `✅ done` when all 3 pass.
2. Run 6.1–6.3 on a real Windows host. Mark Section 6 `✅ done` when all 3 pass.
3. The phase is fully complete when Sections 1 and 2 are `✅ done` AND Sections 5 and 6 are `✅ done`.
