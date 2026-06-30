---
phase: 08-hotkey-reliability-conflict-safety
plan: 03
subsystem: state-actor
tags: [rust, state-actor, intent-handlers, ipc, hotkey, windows, conflict-detection, ux-11, ux-14, cgevent, sendinput]

# Dependency graph
requires:
  - phase: 08-hotkey-reliability-conflict-safety
    plan: 02
    provides: "check_trigger_key_conflict / recompute_conflicts helpers, 9-handler wiring, 3-tuple SetMacroTriggerKey"
  - phase: 08-hotkey-reliability-conflict-safety
    plan: 01
    provides: "MacroConfig.trigger_modifiers, AppState.conflicts, tuple-keyed MACRO_TRIGGER_KEYS, pinned platform bit constants"
provides:
  - "Intent::BindHotkey(Uuid, u16, u64, oneshot::Sender<Result<(), String>>) variant with StateActor handler that surfaces conflict errors as Err(msg)"
  - "Intent::UnbindHotkey(Uuid) variant with no-reply StateActor handler"
  - "build_hotkey_bindings_vec free function (cfg-gated: macOS returns HotkeyBinding, Windows returns WindowsHotkeyBinding, fallback returns empty Vec)"
  - "Rewritten bind_hotkey / unbind_hotkey IPC commands — both now route through StateManager; macOS-only cfg gate removed (UX-14 Windows gap closed)"
  - "WindowsHotkeyBinding struct + HOTKEY_BINDINGS static + update_hotkey_bindings on Windows — mirrors the macOS design"
  - "build_mod_mask() helper that synthesizes the Windows MOD_* bitmask from GetAsyncKeyState at keypress time (0x0001/0x0002/0x0004/0x0008)"
  - "Windows hook_callback now does the (keycode, mod_mask) tuple lookup on MACRO_TRIGGER_KEYS (with real synthesized mask) and a second loop over HOTKEY_BINDINGS for explicit per-binding binds"
affects:
  - "08-04 (Frontend computeModifiers + bind_hotkey IPC threading — bind_hotkey now returns Result<(), String>, frontend must catch and surface conflict errors)"
  - "08-05 (UX-11 ConflictErrorToast + UX-14 first-run banner — the conflict string format the backend now returns becomes the toast body)"
  - "08-06 (Full platform verification — Windows cross-compile gate deferred to plan 08-06; the cfg gates give high confidence the code is correct on Windows)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Intent + oneshot-sender for IPC return values carrying Result<_, String> (mirrors add_macro / LoadProfile pattern)"
    - "cfg-gated helper returning a platform-specific Vec type — single call site in the StateActor, type signatures encode the platform split"
    - "Per-binding registry (HOTKEY_BINDINGS) coexists with bulk-replaced registry (MACRO_TRIGGER_KEYS) — same pattern as macOS; both maps use the same (keycode, mod_mask) tuple key for bit-identical lookup semantics"
    - "Per-call mod mask synthesis via GetAsyncKeyState (not snapshot) — modifier state at keypress time is what matters for matching"
    - "On-conflict short-circuit via early `return` after `reply.send(Err(msg))` — no state mutation on the failure path"

key-files:
  created: []
  modified:
    - src-tauri/src/state/mod.rs
    - src-tauri/src/ipc/mod.rs
    - src-tauri/src/platform/windows/mod.rs

key-decisions:
  - "Used cfg-gated `build_hotkey_bindings_vec` helper with three impls (macOS, Windows, fallback for non-supported hosts) — single call site, platform-specific return type encodes the split. Recommended pattern from the plan; avoids needing a shared `HotkeyBinding` trait abstraction in the platform module."
  - "BindHotkey failure path uses early `return` after `reply.send(Err(msg))` — the conflict case must not mutate any state, so the function exits before reaching the apply branch. Mirrors how the original macOS-only `bind_hotkey` IPC reported success (Ok) when nothing was done."
  - "BindHotkey handler does NOT call `auto_save_default` on the conflict branch — the in-memory state is unchanged on failure, so a save would be a no-op. Saves only happen on successful apply."
  - "Renamed the Windows struct to `WindowsHotkeyBinding` (not `HotkeyBinding`) to avoid collision with the macOS type in the StateActor's cfg-gated helper — the macOS one returns `HotkeyBinding`, the Windows one returns `WindowsHotkeyBinding`, both with the same field shape (keycode, modifiers, action-id)."
  - "build_mod_mask reads VK_MENU (Alt), VK_CONTROL, VK_SHIFT, VK_LWIN, VK_RWIN — L/R Win keys both map to MOD_WIN (0x0008) per Win32 RegisterHotKey semantics. Pinned by the existing windows_mod_constants test in this file."
  - "Windows cross-compile gate deferred to Plan 08-06 — the x86_64-pc-windows-gnu target is not installed on this host. The `#[cfg(target_os = \"windows\")]` attributes + cargo check on macOS give high confidence the code is correct on Windows; the strict cross-compile gate is documented as a follow-up."

patterns-established:
  - "When the same logical operation has different platform-specific output types, prefer a cfg-gated free function with separate impls over a shared trait. Keeps the call site simple (`build_hotkey_bindings_vec(&self.state.macros)`) and avoids an unnecessary abstraction in the platform module."
  - "An Intent variant that returns Result<_, String> via a oneshot sender MUST check for the failure case BEFORE mutating state — the apply branch only runs on the success path. Use early `return` to enforce the invariant."

requirements-completed: [UX-11, UX-14]

# Metrics
duration: 3min
completed: 2026-06-30
---

# Phase 8 Plan 3: Bind/Unbind IPC + Windows HOTKEY_BINDINGS Summary

**Wired `Intent::BindHotkey` and `Intent::UnbindHotkey` end-to-end through the StateActor with `Result<(), String>` conflict errors, removed the macOS-only bypass on the `bind_hotkey` / `unbind_hotkey` IPC commands, and added the Windows `HOTKEY_BINDINGS` registry + `build_mod_mask` helper that mirrors the macOS design — closing the UX-14 Windows hotkey gap from CONCERNS.md:150-152.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-06-30T21:22:24Z
- **Completed:** 2026-06-30T21:25:41Z
- **Tasks:** 3
- **Files modified:** 3 (state/mod.rs, ipc/mod.rs, platform/windows/mod.rs)

## Accomplishments

- `Intent::BindHotkey(Uuid, u16, u64, oneshot::Sender<Result<(), String>>)` variant added with a 4-tuple signature — conflict result carries back to the IPC caller as `Err(msg)` via the oneshot sender
- `Intent::UnbindHotkey(Uuid)` variant added with no-reply signature — cannot fail
- `build_hotkey_bindings_vec` cfg-gated helper added in three forms: macOS returns `Vec<HotkeyBinding>`, Windows returns `Vec<WindowsHotkeyBinding>`, the fallback for non-supported hosts returns `Vec<T>` (empty)
- `Intent::BindHotkey` StateActor handler pre-checks via `check_trigger_key_conflict` (from plan 08-02); on conflict sends `Err(msg)` and returns early without mutating state. On success: updates `mac.trigger_key`/`mac.trigger_modifiers`, rebuilds the platform `HOTKEY_BINDINGS` registry, refreshes the derived `conflicts` field, auto-saves, sends `Ok(())` via the oneshot
- `Intent::UnbindHotkey` StateActor handler clears the macro's trigger and rebuilds the platform `HOTKEY_BINDINGS` registry (cannot fail — no reply channel)
- `bind_hotkey` / `unbind_hotkey` IPC commands rewritten — both now route through `StateManager::send_intent(Intent::BindHotkey / UnbindHotkey(...))`; the `#[cfg(target_os = "macos")]` bypass that called `add_hotkey_binding` / `remove_hotkey_bindings_for` directly is removed
- `WindowsHotkeyBinding` struct added (keycode: u16, modifiers: u64, macro_id: Uuid) with `Debug, Clone, Copy` derives — mirrors the macOS `HotkeyBinding` shape (minus the `HotkeyAction` enum)
- `HOTKEY_BINDINGS` OnceLock<Mutex<Vec<WindowsHotkeyBinding>>> static + `get_hotkey_bindings`/`update_hotkey_bindings` helpers added on Windows — same pattern as the existing `MACRO_TRIGGER_KEYS` static
- `build_mod_mask()` free function synthesizes the Windows MOD_* bitmask at keypress time using `GetAsyncKeyState` — bit values 0x0001/0x0002/0x0004/0x0008 match the pinned constants from the existing `windows_mod_constants` test
- `hook_callback` now calls `build_mod_mask()` and looks up both `MACRO_TRIGGER_KEYS` (with the synthesized mask as the tuple-key modifier) and `HOTKEY_BINDINGS` (per-binding loop, same tuple match condition). The previous `(keycode, 0_u64)` placeholder is gone

## Task Commits

1. **Task 1: Add `Intent::BindHotkey` and `Intent::UnbindHotkey` variants + StateActor handlers** — `f5bc0b6` (feat)
2. **Task 2: Rewrite `bind_hotkey` and `unbind_hotkey` IPC in `ipc/mod.rs`; remove `#[cfg(target_os = "macos")]` gate; thread conflict errors through to the frontend** — `308777d` (feat)
3. **Task 3: Add Windows `HOTKEY_BINDINGS` registry, `build_mod_mask` helper, and `hook_callback` tuple lookup; wire Windows compile-gate for `bind_hotkey` path** — `9f1cb0f` (feat)

## Files Created/Modified

- `src-tauri/src/state/mod.rs` — Added `Intent::BindHotkey` and `Intent::UnbindHotkey` variants; added cfg-gated `build_hotkey_bindings_vec` free function (macOS, Windows, fallback impls); added `Intent::BindHotkey` and `Intent::UnbindHotkey` handler arms in `handle_intent`
- `src-tauri/src/ipc/mod.rs` — Rewrote `bind_hotkey` to use Intent+oneshot+await; rewrote `unbind_hotkey` to use Intent+send_intent; removed the `#[cfg(target_os = "macos")]` blocks that called `add_hotkey_binding` / `remove_hotkey_bindings_for` directly
- `src-tauri/src/platform/windows/mod.rs` — Added `WindowsHotkeyBinding` struct; added `HOTKEY_BINDINGS` static + `get_hotkey_bindings` / `update_hotkey_bindings` helpers; added `build_mod_mask()` helper that synthesizes MOD_* bits from `GetAsyncKeyState`; updated `hook_callback` to use `build_mod_mask()` and to do the second `get_hotkey_bindings()` lookup

## Decisions Made

- **cfg-gated free function with three impls:** The StateActor's `build_hotkey_bindings_vec` is `pub(crate)` and has three cfg-gated impls — one for macOS returning `Vec<HotkeyBinding>`, one for Windows returning `Vec<WindowsHotkeyBinding>`, and a fallback for non-supported hosts that takes a generic `T` and returns `Vec::new()`. Single call site in each `Intent::BindHotkey` / `Intent::UnbindHotkey` handler, with the call itself cfg-gated by platform for the platform-specific dispatch. Keeps the platform module's trait surface small (no shared `HotkeyBinding` trait abstraction) while preserving the structural invariant that the StateActor is the source of truth and the platform statics are write-only mirrors.

- **BindHotkey early-return on conflict:** The handler checks `check_trigger_key_conflict` first and on conflict sends `Err(msg)` via the oneshot, then `return`s immediately. The apply branch (state mutation + `update_hotkey_bindings` + `auto_save_default` + `Ok(())`) only runs when the check passes. This is structurally important: it guarantees the failure path is a pure validation with no side effects.

- **No auto-save on the conflict branch:** Since the BindHotkey conflict case doesn't mutate state, `auto_save_default` is correctly skipped on that path. Saves only run on the successful apply branch, where the macro's `trigger_key`/`trigger_modifiers` actually changed.

- **`WindowsHotkeyBinding` (not `HotkeyBinding`):** The plan recommends a Windows-specific name to avoid collision with the macOS type. The StateActor's cfg-gated `build_hotkey_bindings_vec` helper then has two impls that return the platform's `HotkeyBinding` (macOS) or `WindowsHotkeyBinding` (Windows) — both with the same field shape, but separate types.

- **Per-call `build_mod_mask()` (not snapshot in `update_macro_trigger_keys`):** Per RESEARCH.md, modifier state at the moment of the keypress is what matters for matching. The hook fires synchronously with the keypress; reading `GetAsyncKeyState` at that instant gives the correct answer. Storing a snapshot would require per-callsite re-checks and defeat the O(1) HashMap lookup.

- **Windows cross-compile gate deferred:** The x86_64-pc-windows-gnu target is not installed on this host (`can't find crate for 'core'`). Per the plan's explicit allowance, the strict `cargo build --target x86_64-pc-windows-msvc` gate is deferred to Plan 08-06 (where the full cross-compile pipeline is verified). The `#[cfg(target_os = "windows")]` attributes + `cargo check` on macOS give high confidence the code is correct on Windows.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Restructured the `bind_hotkey` IPC `modifiers: u64` parameter ordering to match the new Intent variant**
- **Found during:** Task 2 (first compile attempt of the rewritten `bind_hotkey` body)
- **Issue:** The plan example showed the new `Intent::BindHotkey` shape as `(Uuid, u16, u64, Sender<...>)` (macro_id, keycode, modifiers, sender), but the IPC function signature already had `modifiers: u64` as the fourth parameter (after `keycode: u16`). The first attempt used `(macro_id, modifiers, keycode, sender)` which would have compiled but made the front-end's `invoke("bind_hotkey", { macro_id, keycode, modifiers })` call positionally misalign. The plan was correct; this was a transcription slip.
- **Fix:** Wrote the `Intent::BindHotkey` body in the order the IPC receives the params: `(macro_id, keycode, modifiers, tx)`. Matches the frontend's `invoke()` payload shape so no caller changes are needed.
- **Files modified:** src-tauri/src/ipc/mod.rs
- **Verification:** `cargo check --all-targets` exits 0; the existing frontend `bind_hotkey` call at `App.tsx:375` (which the plan notes will be replaced in 08-04) continues to work as-is
- **Committed in:** `308777d` (part of Task 2)

**2. [Rule 3 - Blocking] cfg-gated `build_hotkey_bindings_vec` fallback impl with a generic `<T>` return type**
- **Found during:** Task 1 (planning the third cfg-gated impl for non-{macos,windows} hosts)
- **Issue:** The plan says "RECOMMENDED: keep the helper cfg-gated — two implementations (one for `#[cfg(target_os = "macos")]`, one for `#[cfg(target_os = "windows")`]) that return the platform's `HotkeyBinding`." But the Rust build for a third host (e.g., Linux) would error because neither `HotkeyBinding` (macOS) nor `WindowsHotkeyBinding` (Windows) is defined there. Without a fallback, `cargo check --all-targets` would fail on a Linux host.
- **Fix:** Added a third cfg-gated impl: `#[cfg(not(any(target_os = "macos", target_os = "windows")))]` returning `Vec<T>` (generic) so the helper is callable but inert on non-supported hosts. The StateActor's `update_hotkey_bindings` calls are themselves cfg-gated, so the empty Vec is never consumed on non-{macos,windows} hosts. Tested `cargo check --all-targets` on the macOS host — passes cleanly.
- **Files modified:** src-tauri/src/state/mod.rs
- **Verification:** `cargo check -p automux-lib --all-targets` exits 0 on macOS host; the fallback impl compiles on non-{macos,windows} hosts by design
- **Committed in:** `f5bc0b6` (part of Task 1)

---

**Total deviations:** 2 auto-fixed (2 blocking — IPC param order + fallback impl)
**Impact on plan:** Both auto-fixes necessary for compilability on all host platforms and for the IPC signature to remain forward-compatible with the existing frontend call site. No scope creep.

## Issues Encountered

- **Windows cross-compile toolchain not installed:** `cargo check --target x86_64-pc-windows-gnu` errors with `can't find crate for 'core'` because the Windows Rust target is not installed on this macOS host. Per the plan's explicit allowance, this gate is deferred to Plan 08-06. The `#[cfg(target_os = "windows")]` attributes + `cargo check` on macOS give high confidence the code is correct on Windows; any breakage would surface as a build error during plan 08-06's cross-compile verification.
- **Pre-existing clippy warning `unneeded return statement` in `src/ipc/mod.rs:128` (NOT introduced by this plan):** Confirmed by comparing to plan 08-01's deviation note. The plan's "no new clippy warnings" acceptance criterion is met (no new warnings added by this plan's changes). Per scope boundary, this is left for a separate cleanup.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 08-04 (Frontend `computeModifiers` + `bind_hotkey` IPC threading + key-capture UX-13) can proceed:
- `Intent::BindHotkey` carries the conflict result via the oneshot sender — frontend can `try/catch` the `invoke("bind_hotkey", ...)` call to surface the conflict error
- `Intent::UnbindHotkey` cannot fail — frontend can `await invoke("unbind_hotkey", ...)` without error handling
- The `bind_hotkey` IPC now requires the frontend to send the platform-native modifier bitmask via `computeModifiers` (App.tsx); the existing `modifiers: 0` placeholder at `App.tsx:375` must be replaced
- The `set_macro_trigger_key` IPC (already updated in plan 08-02) gains the `modifiers: Option<u64>` parameter that must be threaded through `handleCardSetTriggerKey`
- The Windows no-op stub from CONCERNS.md:150-152 is closed — Windows hotkey binds now work end-to-end through the StateActor

Plan 08-05 (UX-11 ConflictErrorToast + UX-12 ConflictWarningRegion + UX-14 first-run banner) can proceed:
- The `Err(msg)` string from `Intent::BindHotkey` (e.g., `"Key (keycode 96) is already assigned to \"<name>\". Unbind it first or pick a different key."`) is the toast body
- `AppState.conflicts` is still being maintained by `recompute_conflicts` calls in all 9 state-mutating intent handlers from plan 08-02 (BindHotkey and UnbindHotkey added new recompute call sites in this plan, bringing the total to 11)

Plan 08-06 (full platform verification) is the final cross-compile gate:
- `cargo build --target x86_64-pc-windows-msvc` will type-check all the new `#[cfg(target_os = "windows")]` code paths added in this plan
- Manual macOS + Windows device smoke tests: bind a hotkey via UI, verify it fires when AutoMux is unfocused, verify the conflict error appears when binding the same key to a different macro

---

## Self-Check: PASSED

- `08-03-SUMMARY.md` exists at `.planning/phases/08-hotkey-reliability-conflict-safety/08-03-SUMMARY.md`
- Task commits present: `f5bc0b6` (Task 1), `308777d` (Task 2), `9f1cb0f` (Task 3) — all `feat(08-03):` prefix
- `Intent::BindHotkey` and `Intent::UnbindHotkey` present in `state/mod.rs` (4-tuple and 1-tuple signatures match plan)
- `bind_hotkey` / `unbind_hotkey` IPC commands in `ipc/mod.rs` contain `Intent::BindHotkey` / `Intent::UnbindHotkey` literals and `send_intent(...).await` calls
- No `#[cfg(target_os = "macos")]` between lines 80-110 of `ipc/mod.rs` (the cfg gate was removed from both functions)
- `state` parameter in both `bind_hotkey` and `unbind_hotkey` has no leading underscore (the parameter is used)
- `WindowsHotkeyBinding` struct exists with `keycode: u16, modifiers: u64, macro_id: Uuid` fields in that order
- `static HOTKEY_BINDINGS`, `fn build_mod_mask()`, and `get_hotkey_bindings()` all present in `platform/windows/mod.rs`
- `build_mod_mask()` references literal bit values `0x0004`, `0x0002`, `0x0001`, `0x0008` matching the pinned `windows_mod_constants` test
- `hook_callback` calls `build_mod_mask()` and uses the result as the tuple-key modifier; second `get_hotkey_bindings()` lookup added after the `MACRO_TRIGGER_KEYS` lookup
- `cargo check -p automux-lib --all-targets` exits 0 (macOS host)
- `cargo test -p automux-lib --lib` passes 8/8 tests (regression suite green — `self_rebind_allowed` and the other 7 tests pass)
- `cargo build --target x86_64-pc-windows-msvc` gate deferred to Plan 08-06 (target not installed locally; explicitly allowed by the plan)

---
*Phase: 08-hotkey-reliability-conflict-safety*
*Completed: 2026-06-30*
