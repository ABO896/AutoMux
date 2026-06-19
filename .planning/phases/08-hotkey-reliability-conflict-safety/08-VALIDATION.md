---
phase: 8
slug: hotkey-reliability-conflict-safety
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-19
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (inline `#[cfg(test)]` modules — existing pattern) |
| **Config file** | none — pattern reused from `scheduler/mod.rs:393-582` and `persistence.rs:251-319` |
| **Quick run command** | `cargo test -p automux-lib` |
| **Full suite command** | `cargo test -p automux-lib && cargo clippy --all-targets -- -D warnings` |
| **Estimated runtime** | ~10 seconds |

**Frontend:** No test framework installed. UI changes validated manually with device smoke tests. Phase 10 may add Vitest.

---

## Sampling Rate

- **After every task commit:** `cargo test -p automux-lib`
- **After every plan wave:** `cargo test -p automux-lib && cargo clippy --all-targets -- -D warnings`
- **Before `/gsd-verify-work`:** Full suite green AND manual macOS + Windows device smoke test
- **Max feedback latency:** 10s

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 8-01-01 | 01 | 1 | UX-13 | R-1 | `CGEventFlags` bit values asserted in test | unit | `cargo test cg_event_flag_constants` | ⬜ W0 | ⬜ pending |
| 8-01-02 | 01 | 1 | UX-13 | R-1 | `MOD_*` Windows constants asserted in test | unit | `cargo test windows_mod_constants` | ⬜ W0 | ⬜ pending |
| 8-01-03 | 01 | 1 | UX-13 | — | `HotkeyBinding.matches()` matches `Cmd+F5` style binding | unit | `cargo test hotkey_modifier_match` | ⬜ W0 | ⬜ pending |
| 8-01-04 | 01 | 1 | UX-11/13 | — | `MACRO_TRIGGER_KEYS` keyed by `(u16, u64)` tuple | unit | `cargo test trigger_keys_keyed_by_modifiers` | ⬜ W0 | ⬜ pending |
| 8-02-01 | 02 | 1 | UX-11 | — | `check_trigger_key_conflict` rejects new macro on existing binding | unit | `cargo test check_trigger_key_conflict_rejects` | ⬜ W0 | ⬜ pending |
| 8-02-02 | 02 | 1 | UX-11 | R-6 | Re-binding same macro to same key succeeds (self-rebind allowed) | unit | `cargo test check_trigger_key_conflict_self_rebind` | ⬜ W0 | ⬜ pending |
| 8-02-03 | 02 | 2 | UX-12 | — | `recompute_conflicts` finds two enabled macros with same input | unit | `cargo test recompute_conflicts_detects_overlap` | ⬜ W0 | ⬜ pending |
| 8-02-04 | 02 | 2 | UX-12 | — | Disabling one macro removes the conflict from `AppState` | unit | `cargo test recompute_conflicts_disappear_on_disable` | ⬜ W0 | ⬜ pending |
| 8-03-01 | 03 | 2 | UX-11 | — | `bind_hotkey` IPC returns error string on conflict | unit | `cargo test bind_hotkey_ipc_conflict` | ⬜ W0 | ⬜ pending |
| 8-03-02 | 03 | 2 | UX-14 | — | `bind_hotkey` / `unbind_hotkey` wired on Windows path (no-op stub on non-Windows) | compile | `cargo build --target x86_64-pc-windows-msvc 2>&1 \| grep warning` returns zero | ⬜ W0 | ⬜ pending |
| 8-04-01 | 04 | 3 | UX-13 | — | Frontend `computeModifiers` returns expected bits per platform (manual / browser dev tools) | manual | Open app, press `Cmd+F5` in capture, inspect modifier bits in `bind_hotkey` IPC payload | N/A | ⬜ pending |
| 8-04-02 | 04 | 3 | UX-11 | — | Frontend surfaces conflict error from `bind_hotkey` to user | manual | Try to bind same key to two macros; verify visible error | N/A | ⬜ pending |
| 8-05-01 | 05 | 3 | UX-12 | — | Conflict warning region renders `state().conflicts` as a list | manual | Enable two same-input macros; verify warning appears | N/A | ⬜ pending |
| 8-05-02 | 05 | 3 | UX-14 | — | "↗ Global" subtitle visible on trigger-key chip | manual | Visual inspection | N/A | ⬜ pending |
| 8-05-03 | 05 | 3 | UX-14 | — | First-run banner appears once, dismissible, sets `localStorage` flag | manual | Clear `localStorage`, reload, dismiss banner, reload, verify not shown | N/A | ⬜ pending |
| 8-06-01 | 06 | 4 | UX-14 | R-2 | Hotkey fires when AutoMux unfocused on macOS | manual smoke | macOS device test: switch focus to another app, press bound hotkey | N/A | ⬜ pending |
| 8-06-02 | 06 | 4 | UX-14 | R-2 | Hotkey fires when AutoMux unfocused on Windows | manual smoke | Windows device test: same flow | N/A | ⬜ pending |
| 8-06-03 | 06 | 4 | — | — | `cargo test -p automux-lib` green; `cargo clippy --all-targets -- -D warnings` clean | regression | full suite | ✅ | ⬜ pending |
| 8-06-04 | 06 | 4 | — | R-5 | Existing profile loads with new `conflicts: Vec<InputConflict>` field (backwards compat) | manual | Load a v2.0 profile saved before this phase | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/state/mod.rs` — inline `#[cfg(test)]` module with `check_trigger_key_conflict` and `recompute_conflicts` tests (UX-11 / UX-12)
- [ ] `src-tauri/src/platform/macos/observer.rs` — inline test asserting `CGEventFlagCommand.bits() == 0x100000` and `HotkeyBinding.matches()` with synthetic flags
- [ ] `src-tauri/src/platform/windows/mod.rs` — inline test asserting `MOD_ALT=0x0001, MOD_CONTROL=0x0002, MOD_SHIFT=0x0004, MOD_WIN=0x0008` (compile-gated `#[cfg(windows)]`)

*Existing infrastructure covers persistence (`persistence.rs:251-319`) and scheduler (`scheduler/mod.rs:393-582`) unit tests — pattern reuse, no new framework install required.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Frontend `computeModifiers` returns expected bits | UX-13 | No frontend test framework installed | Press modifier combos in capture UI, log computed bits in console, verify against platform constants |
| `bind_hotkey` IPC surfaces conflict in UI | UX-11 | End-to-end UI flow | Bind same key to two macros; verify visible error message in App.tsx |
| Conflict warning region renders | UX-12 | UI rendering | Enable two same-input macros; verify warning text appears |
| "Binds are system-wide" notice visible | UX-14 | UI rendering | Visual inspection of trigger-key chip and first-run banner |
| macOS global hotkey fires when unfocused | UX-14 | Device-specific behavior | macOS device test: switch focus, press bound hotkey, verify macro fires |
| Windows global hotkey fires when unfocused | UX-14 | Device-specific behavior | Windows device test: same flow |
| Profile backwards compatibility | R-5 | Requires existing saved profile | Save a v2.0 profile before this phase, run phase, load it, verify no errors |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
