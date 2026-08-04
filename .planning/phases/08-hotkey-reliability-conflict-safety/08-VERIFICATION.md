---
phase: 08-hotkey-reliability-conflict-safety
verified: 2026-08-04T14:44:40Z
status: human_needed
score: 4/4 must-have truths verified (source-level); 1 ROADMAP success criterion present-but-behavior-unverified at the phase's own literal device-test granularity
behavior_unverified: 1
overrides_applied: 0
re_verification:
  previous_status: complete (informal gate-status frontmatter; ROADMAP.md's own Phase 8 checkbox was — and remains — unchecked)
  previous_score: "4/4 automated gates green (08-VERIFICATION.md 2026-06-30T21:46:00Z); Sections 5+6 (manual device tests) recorded pending"
  gaps_closed:
    - "None required closing — no regressions found. This pass re-confirms all UX-11/12/13/14 backend and frontend guarantees against CURRENT source (post Phase-9 and Phase-10 changes to the same files), not just the June 30 snapshot."
  gaps_remaining:
    - "ROADMAP SC4 / UX-14 literal device verification (08-VERIFICATION.md Sections 5 and 6 — macOS Tests 5.1-5.6, Windows Tests 6.1-6.5) has never been executed and marked done under Phase 8's own name. Unchanged since the original pass."
  regressions: []
gaps: []
deferred: []
behavior_unverified_items:
  - truth: "ROADMAP SC4 — global hotkey operation is verified on both macOS and Windows real devices, per Phase 8's own literal test protocol (08-VERIFICATION.md Section 5 macOS Tests 5.1-5.6, Section 6 Windows Tests 6.1-6.5)"
    test: "Run the 6 macOS tests (system-wide Cmd+F5 toggle while unfocused, first-run banner appears-once-and-dismisses, trigger-key conflict toast, same-input overlap warning, in-card Global subtitle, modifier chip preview order) and the 5 Windows tests (system-wide Ctrl+Shift+F5 toggle while unfocused, bind_hotkey IPC no-longer-a-no-op, first-run banner + Global subtitle, same-input overlap warning, modifier chip preview order) exactly as written in 08-VERIFICATION.md's original Section 5/6, on real macOS and Windows hardware with Accessibility + Input Monitoring (macOS) granted"
    expected: "All 11 steps pass as literally specified; 08-VERIFICATION.md Section 7 status table Sections 5 and 6 can be marked done"
    why_human: "Requires live CGEvent/Win32-hook injection, live OS focus switching, and human visual confirmation of toast/banner/chip UI — cannot be verified by static analysis. Phase 9's 2026-08-04 device UAT (09-UAT.md Round 2, Tests 3-4) exercised closely-related infrastructure (hotkey-triggered macro toggling while unfocused on both platforms, and confirmed the Phase 8 conflict warning renders on a real macOS device) and passed — but it is a distinct test protocol scoped to Phase 9's concurrent-execution goal, not a verbatim re-run of Phase 8's own 11 steps (e.g. the first-run-banner dismiss-persistence check, the Windows bind_hotkey-no-longer-no-op check, and the modifier-chip semantic-order check were not part of Phase 9's re-test). The project's own STATE.md (updated 2026-08-04) and ROADMAP.md (Phase 8 checkbox still unchecked) both independently confirm this is the sole open item and describe Phase 8 as 'source-complete' otherwise."
human_verification:
  - test: "Execute 08-VERIFICATION.md Section 5 (macOS Tests 5.1-5.6) and Section 6 (Windows Tests 6.1-6.5) on real macOS and Windows hosts."
    expected: "All 11 steps pass exactly as specified in the original plan."
    why_human: "Live device, live OS focus-switching, and human visual confirmation of toast/banner/chip UI — not verifiable by static analysis. This is the only outstanding item; the codebase itself has been independently confirmed correct and unregressed by this pass."
---

# Phase 8: Hotkey Reliability & Conflict Safety — Verification Report

**Phase Goal:** The hotkey binding system is reliable, full-featured, and safe — supports a broad key range, prevents silent conflicts between macros, and users understand that binds are system-wide
**Verified:** 2026-08-04T14:44:40Z
**Status:** human_needed
**Re-verification:** Yes — the prior 08-VERIFICATION.md (2026-06-30T21:46:00Z) was flagged stale by `gsd-tools` because 08-06-SUMMARY.md committed ~22 minutes after that verification timestamp, and because Phases 9 and 10 (both now complete, dated 2026-08-04) subsequently touched every file Phase 8 modified: `src/App.tsx`, `src-tauri/src/state/mod.rs`, `src-tauri/src/ipc/mod.rs`, `src-tauri/src/platform/windows/mod.rs`, and `src-tauri/src/platform/macos/observer.rs`. This pass independently re-derives every UX-11/12/13/14 guarantee from the CURRENT state of those files (not the June 30 snapshot) and re-runs the test suite myself rather than trusting any prior report's numbers.

## Re-verification Summary

**No regressions found.** Phase 9's Plan 10 (commit `76d7b3c`, "remove redundant hotkey registry from both platform observers") deleted the `MACRO_TRIGGER_KEYS: HashMap<(u16, u64), Uuid>` registry that Phase 8 Plan 08-01 originally introduced, consolidating both platforms onto the single `HOTKEY_BINDINGS: Vec<HotkeyBinding>` / `Vec<WindowsHotkeyBinding>` registry that already existed pre-Phase-8 for hotkey *dispatch*. I independently confirmed:

1. **The consolidation preserves modifier-aware matching.** `HotkeyBinding.modifiers` and `WindowsHotkeyBinding.modifiers` are populated from `mac.trigger_modifiers` by `build_hotkey_bindings_vec()` (`state/mod.rs:384-411`, introduced in Phase 8 Plan 08-03, commit `f5bc0b6` — unchanged by 09-10). macOS's `HotkeyBinding::matches()` (`observer.rs:50-52`) does a bitwise-containment check (`flags.bits() & self.modifiers == self.modifiers`); Windows's `hook_callback` (`platform/windows/mod.rs:541`) does an exact-equality check (`binding.modifiers == mod_mask`) against a `build_mod_mask()` synthesized at keypress time. Both were already designed this way before Phase 9 touched the files (08-RESEARCH.md explicitly called this "correct bitwise containment check", the intended design); 09-10 only removed the *duplicate* lookup path that had caused a double-dispatch bug (closed by its own regression test, `state::tests::hotkey_registry_has_single_binding_per_trigger_macro`, which explicitly asserts exactly one binding per trigger-key macro).
2. **`MACRO_TRIGGER_KEYS` is completely gone crate-wide** (`grep -rn "MACRO_TRIGGER_KEYS"` on `src-tauri/src/` returns nothing) — the literal artifact named in 08-01's must-haves (`HashMap<(u16, u64), Uuid>`) no longer exists, but the *truth* it existed to satisfy ("the platform trigger-key registry is keyed by matching including modifier bits") is fully preserved by the surviving `HOTKEY_BINDINGS` registry, which I verified is the sole path both platforms consult (no dead second lookup remains).
3. **`bind_hotkey`/`unbind_hotkey` IPC commands remain platform-unconditional** (`ipc/mod.rs:81-108`, no `#[cfg(target_os = "macos")]` gate) — UX-14's "no longer macOS-only" guarantee holds.
4. **All 9 state-mutating `Intent` handlers still call `recompute_conflicts()`** (`AddMacro`, `RemoveMacro`, `SetMacroEnabled`, `SetMacroTriggerKey`, `UpdateMacro`, `BindHotkey`, `UnbindHotkey`, `ToggleMacroHotkey`, `ToggleEngineHotkey`, plus `LoadProfile`) — confirmed by direct grep and read of each handler body.
5. **All Phase 8 frontend UI surfaces survived Phase 10's component extraction** (`App.tsx` → `MacroCard.tsx`, `MacroForm.tsx`, `KeyCaptureField.tsx`): the conflict toast, conflict warning region, first-run global notice, `↗ Global` subtitle, and modifier-preview chips are all present, unchanged in behavior, and correctly re-wired through the new component props.
6. **`cargo test --lib` (run by me just now, not trusted from any report): 25/25 pass**, including every Phase-8-specific test (`cg_event_flag_constants`, `self_rebind_allowed`, `bind_conflict_rejected`, `conflict_detection_overlap`, `conflict_disappear_on_disable`, `profile_backwards_compat`) plus later regression tests that directly re-exercise Phase 8 surfaces (`hotkey_registry_has_single_binding_per_trigger_macro`, `set_trigger_key_rejects_conflict_without_coercion`). `cargo build` and `npx tsc --noEmit` both clean.
7. **One documentation staleness finding (not a code gap):** `REQUIREMENTS.md` still shows UX-13 as unchecked (`[ ]`) with the note "OS-level modifier matching on macOS is a follow-up." I traced this via `git log -S` and found the note was written in the Plan 08-05 docs commit (`9edbac4`, 2026-06-30 23:44), which is *after* Plan 08-03's commit (`f5bc0b6`, 2026-06-30 23:23) already wired `trigger_modifiers` into `build_hotkey_bindings_vec` and made the pre-existing `HotkeyBinding::matches()` bitwise check operate on real, non-zero modifier bits for the first time. The note was stale at the moment it was written and remains stale today — the functionality it describes as a "follow-up" was already complete. Recommend updating `REQUIREMENTS.md` line 42/95 to check the box and correct the note; this is a documentation fix, not a code change.
8. **The one genuine open item is unchanged from the original pass and is NOT something Phase 9/10 caused:** ROADMAP.md's Phase 8 checkbox is still unchecked, and `.planning/STATE.md` (last updated 2026-08-04, today) explicitly states "Phase 8 is the sole remaining blocker, awaiting human device verification" and "no code work is pending for it — it is source-complete." I independently corroborate this: 08-VERIFICATION.md Section 7's status table has never had Sections 5 (macOS device tests) or 6 (Windows device tests) marked done. Phase 9's own device UAT (`09-UAT.md`, updated today) did run real macOS and Windows hardware tests that incidentally exercised hotkey-triggered toggling and confirmed "the Phase 8 conflict warning also displays" on a real macOS device — good corroborating evidence the backend is sound end-to-end — but it is Phase 9's own test protocol (T9.1-T9.7 macOS, its own 6.1-6.3 Windows numbering) scoped to concurrent execution, not a verbatim re-run of Phase 8's specific 11-step checklist (first-run-banner persistence, Windows bind_hotkey-no-longer-no-op, modifier-chip order). I am not crediting it as closing Phase 8's own device-verification requirement.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | ROADMAP SC1 / UX-13 — a user can bind a hotkey using A-Z, 0-9, F1-F12, and modifier combinations; the binding UI accepts all of these | VERIFIED | `src/keymap.ts` confirms full A-Z (KeyA-KeyZ), 0-9 (Digit0-Digit9), F1-F12 coverage on both the macOS `CGKeyCode` map and the Windows `VK_*` map. `computeModifiers()`/`modifierChips()` (`App.tsx:154-201`) emit and label the exact bit values pinned by `cg_event_flag_constants` (macOS) and the `#[cfg(windows)]`-gated `windows_mod_constants` test. The `["Control","Shift","Alt","Meta"]` early-return filter is confirmed absent from `startCapture.onKeyDown` (`App.tsx:708-733`) — modifier-only keys can be committed. `trigger_modifiers` flows end-to-end into `HotkeyBinding`/`WindowsHotkeyBinding` and is exercised by a bitwise-containment (macOS) / exact-equality (Windows) match at keypress time — confirmed by direct source read, not just presence. |
| 2 | ROADMAP SC2 / UX-11 — binding an already-assigned hotkey shows an explicit conflict error, no silent shadowing | VERIFIED | `check_trigger_key_conflict` (`state/mod.rs:263-278`) is invoked by `Intent::BindHotkey` (line 700), `Intent::AddMacro` (line 558), and `resolve_trigger_key_update` (used by both `SetMacroTriggerKey` and `UpdateMacro`, lines 619/658). Self-rebind returns `None` (no-op). `cargo test --lib` (run by me): `self_rebind_allowed`, `bind_conflict_rejected`, `set_trigger_key_rejects_conflict_without_coercion`, `update_macro_conflict_no_partial_mutation` all pass. Frontend `ConflictErrorToast` (`App.tsx:1094-1108`) renders on the exact error-string pattern the backend emits. |
| 3 | ROADMAP SC3 / UX-12 — enabling a second macro injecting the same input surfaces a visible warning | VERIFIED | `recompute_conflicts` (`state/mod.rs:336-372`) is called from all 9 state-mutating `Intent` handlers plus `LoadProfile` (confirmed by direct grep + read of every call site). `cargo test --lib`: `conflict_detection_overlap`, `conflict_disappear_on_disable` pass. Frontend conflict-warning card (`App.tsx:1143`, `id="conflict-warning-card"`) renders one card per `state().conflicts` entry with correct Oxford-comma / singular-plural grammar. |
| 4 | ROADMAP SC4 / UX-14 — hotkeys fire when AutoMux is unfocused; the UI communicates system-wide binding; verified on both macOS and Windows | PRESENT_BEHAVIOR_UNVERIFIED (source-level: VERIFIED) | Source-level: `bind_hotkey`/`unbind_hotkey` are platform-unconditional (`ipc/mod.rs:81-108`); Windows `HOTKEY_BINDINGS` registry (consolidated by Phase 9's 09-10, `platform/windows/mod.rs:434`) is populated identically to macOS via `build_hotkey_bindings_vec`; `FirstRunGlobalNotice` banner (`App.tsx:1069-1090`) and per-card `↗ Global` subtitle (`MacroCard.tsx:218`, present in both the bound-key and unset-key render branches) are both present and unchanged by Phase 10's component extraction. Device-level: 08-VERIFICATION.md's own Section 5 (6 macOS tests) and Section 6 (5 Windows tests) have never been executed and marked done under Phase 8's name — confirmed unchanged from the original pass, corroborated by today's STATE.md and ROADMAP.md. See `behavior_unverified_items` and Human Verification below. |

**Score:** 4/4 truths hold at the source level (all backend logic, unit tests, and frontend UI surfaces independently re-confirmed against current post-Phase-9/10 source, zero regressions). 1 of those 4 (SC4/UX-14) remains present-but-behavior-unverified specifically at the real-device granularity Phase 8's own plan defined — this is the same open item the project has tracked since the original pass, not a new finding.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/state/mod.rs` | `MacroConfig.trigger_modifiers`, `AppState.conflicts`, `InputConflict`, `check_trigger_key_conflict`, `recompute_conflicts`, `Intent::BindHotkey`/`UnbindHotkey` | VERIFIED | All present; `trigger_modifiers: u64` (line 115, `#[serde(default)]`), `conflicts: Vec<InputConflict>` (line 136, `#[serde(default)]`) — both confirmed backwards-compatible by the passing `profile_backwards_compat` test. |
| `src-tauri/src/platform/macos/observer.rs` | Modifier-aware hotkey matching keyed by keycode+modifiers | VERIFIED (alternate implementation of the 08-01 must-have) | The literal `MACRO_TRIGGER_KEYS: HashMap<(u16, u64), Uuid>` artifact named in 08-01's must-haves was removed by Phase 9 Plan 10 as part of a deliberate, tested double-dispatch bug fix. The surviving `HOTKEY_BINDINGS: Vec<HotkeyBinding>` registry (pre-existing, now the sole registry) achieves the same modifier-aware-matching truth via `HotkeyBinding.modifiers` + `.matches()`'s bitwise check, fed by the same `trigger_modifiers` data Phase 8 introduced. `cg_event_flag_constants` test passes. |
| `src-tauri/src/platform/windows/mod.rs` | Windows `HOTKEY_BINDINGS` registry + `build_mod_mask` + modifier-aware `hook_callback` lookup | VERIFIED | `WindowsHotkeyBinding{keycode, modifiers, macro_id}` (line 38-42), `HOTKEY_BINDINGS` static (line 434), `build_mod_mask()` (line 470-491), exact-match lookup in `hook_callback` (line 541). `windows_mod_constants` test present (cfg-gated to Windows, correctly absent from this macOS host's test run). No leftover `MACRO_TRIGGER_KEYS` anywhere in the crate. |
| `src-tauri/src/ipc/mod.rs` | `bind_hotkey`/`unbind_hotkey` routed through `StateManager`, platform-unconditional | VERIFIED | Lines 81-108; no `#[cfg(target_os = "macos")]` gate; routes through `Intent::BindHotkey`/`UnbindHotkey` with a `Result`-carrying oneshot. |
| `src/App.tsx` + `src/components/{MacroCard,MacroForm,KeyCaptureField}.tsx` | `computeModifiers`, `ConflictErrorToast`, `ConflictWarningRegion`, `FirstRunGlobalNotice`, in-card `↗ Global`, `ModifierPreviewChip` | VERIFIED | All 5 UI surfaces from Plan 08-05 confirmed present after Phase 10's component extraction; semantic modifier-chip order (`Shift → Ctrl → Alt → Cmd/Win`) unchanged in `modifierChips()` (`App.tsx:182-201`). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `MacroConfig.trigger_modifiers` | `build_hotkey_bindings_vec` | `mac.trigger_modifiers` field read | WIRED | `state/mod.rs:391-395` (macOS), `407-411` (Windows). |
| `Intent::BindHotkey` handler | platform `update_hotkey_bindings` | rebuild-and-replace the whole registry on every bind | WIRED | `state/mod.rs:733-741`; confirmed for both `#[cfg(target_os = "macos")]` and `#[cfg(target_os = "windows")]` branches. |
| `check_trigger_key_conflict` | `Intent::BindHotkey`/`AddMacro`/`SetMacroTriggerKey`/`UpdateMacro` | pre-mutation conflict gate | WIRED | Confirmed at 4 call sites (lines 558, 619, 658, 700-701); conflicting bind returns `Err` via oneshot before any state mutation, confirmed by `set_trigger_key_rejects_conflict_without_coercion` and `update_macro_conflict_no_partial_mutation` passing tests. |
| `recompute_conflicts` | `AppState.conflicts` → `state-changed` event → frontend `ConflictWarningRegion` | derived-field push on every state-mutating intent | WIRED | 9 call sites confirmed by direct read; frontend renders `state().conflicts` (`App.tsx:1143`+). |
| `ipc::bind_hotkey`/`unbind_hotkey` | `Intent::BindHotkey`/`UnbindHotkey` | oneshot-carried `Result` | WIRED | `ipc/mod.rs:81-108`; both platforms consult the resulting registry identically. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full backend regression suite (run by me, not trusted from any report) | `cargo test --lib` (from `src-tauri/`) | 25 passed; 0 failed | PASS |
| Backend compiles clean | `cargo build --manifest-path src-tauri/Cargo.toml` | exit 0 | PASS |
| Frontend typechecks clean | `npx tsc --noEmit` | exit 0, no output | PASS |
| Clippy (informational — not a Phase 8 gate) | `cargo clippy --all-targets -- -D warnings` | 1 warning: `too_many_arguments` on `update_macro` (`ipc/mod.rs:170`) | PRE-EXISTING, NOT PHASE 8 — introduced by Phase 10 Plan 10-01 (`git log -S`, commit `6dd8abb`), unrelated to any Phase-8-touched function. The original Phase-8-era `needless_return` warning (`ipc/mod.rs:128`, documented in the prior 08-VERIFICATION.md) has since been fixed and no longer appears. |
| No leftover `MACRO_TRIGGER_KEYS` registry anywhere in the crate | `grep -rn "MACRO_TRIGGER_KEYS" src-tauri/src/` | no matches | PASS — confirms Phase 9's consolidation is complete and clean |
| No debt markers in Phase-8-touched files (current state) | `grep -n 'TBD\|FIXME\|XXX'` across `state/mod.rs`, `ipc/mod.rs`, `platform/macos/observer.rs`, `platform/windows/mod.rs`, `App.tsx`, `MacroCard.tsx`, `KeyCaptureField.tsx` | no matches | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| UX-11 | 08-01..08-05 | No silent shadowing of an existing hotkey bind | SATISFIED | `check_trigger_key_conflict` wired into all 4 mutation paths; toast UI present; regression tests pass; unregressed by Phase 9/10. |
| UX-12 | 08-02, 08-05 | Warn on concurrent same-input macros | SATISFIED | `recompute_conflicts` wired into all 9 state-mutating handlers; warning UI present; regression tests pass; unregressed. |
| UX-13 | 08-01, 08-04, 08-05 | Full practical key range incl. modifiers | SATISFIED at the source level | Full A-Z/0-9/F1-F12 + modifier-bit support confirmed end-to-end (frontend capture → IPC → backend match). **`REQUIREMENTS.md` still marks this unchecked with a stale "follow-up" note** — traced via `git log -S` to predate the very commit (`f5bc0b6`, same day) that already completed the described work. Recommend a documentation-only fix to `REQUIREMENTS.md` lines 42 and 95. |
| UX-14 | 08-03, 08-05, 08-06 | Global hotkey behavior verified on both platforms, UI communicates system-wide | SATISFIED at the source level; device verification outstanding | Backend/frontend guarantees fully confirmed and unregressed. The phase's own literal device-test protocol (08-VERIFICATION.md Section 5/6) has never been executed — same open item the project has tracked since 2026-06-30, independently reconfirmed by today's STATE.md/ROADMAP.md. See Human Verification. |

No orphaned requirements: REQUIREMENTS.md's Phase 8 row set (UX-11, UX-12, UX-13, UX-14) maps exactly to the `requirements:` frontmatter declared across all 6 plans (08-01 through 08-06).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `.planning/REQUIREMENTS.md` | 42, 95 | UX-13 checkbox unchecked with a stale "OS-level modifier matching on macOS is a follow-up" note that predates the commit which completed that work | Info (documentation only) | No functional impact — the underlying code is correct and tested. Recommend checking the box and correcting the note text in a follow-up docs commit. |
| `.planning/ROADMAP.md` | 36 | Phase 8 checkbox unchecked, pending Sections 5+6 device verification | Info (tracked, not new) | Matches this verification's own finding — not a discrepancy, this is the project accurately tracking its one remaining item. |

No blocker-severity anti-patterns found in any Phase-8-touched source file.

## Human Verification Required

### 1. Execute Phase 8's own macOS + Windows device test protocol

**Test:** Run 08-VERIFICATION.md Section 5 (6 macOS tests: system-wide `Cmd+F5` toggle while unfocused; first-run banner appears-once-and-dismisses; trigger-key conflict toast; same-input overlap warning; in-card `↗ Global` subtitle; modifier-chip preview semantic order) and Section 6 (5 Windows tests: system-wide `Ctrl+Shift+F5` toggle while unfocused; `bind_hotkey` IPC no-longer-a-no-op — closes the original CONCERNS.md:150-152 gap; first-run banner + `↗ Global`; same-input overlap warning; modifier-chip preview order with Windows modifiers) on real macOS and Windows hardware with Accessibility + Input Monitoring granted (macOS).

**Expected:** All 11 steps pass exactly as specified; update 08-VERIFICATION.md Section 7's status table to mark Sections 5 and 6 done, and check the Phase 8 box in ROADMAP.md and correct REQUIREMENTS.md's UX-13/UX-14 entries.

**Why human:** Requires live CGEvent/Win32-hook injection while the app is unfocused, live first-launch `localStorage` state, and human visual confirmation of toast/banner/chip rendering — none of this is verifiable by static source analysis. This is the sole remaining item for Phase 8, and the codebase underlying it has now been independently re-confirmed correct and unregressed by two full downstream phases (9 and 10) that both touched the same files.

## Gaps Summary

No code gaps. Every backend and frontend guarantee behind UX-11, UX-12, UX-13, and UX-14 was independently re-derived from the CURRENT state of every file Phase 9 and Phase 10 also touched (`src/App.tsx`, `src-tauri/src/state/mod.rs`, `src-tauri/src/ipc/mod.rs`, `src-tauri/src/platform/windows/mod.rs`, `src-tauri/src/platform/macos/observer.rs`), not merely re-read from the June 30 snapshot or trusted from either SUMMARY.md's or the prior 08-VERIFICATION.md's prose. `cargo test --lib` (25/25), `cargo build`, and `npx tsc --noEmit` all pass cleanly when run directly by me. Phase 9's registry consolidation (09-10) removed the literal `HashMap<(u16,u64),Uuid>` artifact named in Plan 08-01's must-haves, but I confirmed the underlying truth it existed to satisfy — modifier-aware hotkey matching — is fully preserved by the surviving, now-sole `HOTKEY_BINDINGS` registry, and that the consolidation itself fixed a genuine double-dispatch bug (pinned by a passing regression test).

The sole outstanding item — real-device execution of Phase 8's own 11-step manual test protocol (Section 5 macOS, Section 6 Windows) — is unchanged from the original 2026-06-30 pass and is not something Phase 9 or Phase 10 introduced or could have closed; the project's own STATE.md and ROADMAP.md (both updated today, 2026-08-04) independently agree this is the sole remaining blocker for milestone v2.0 and describe Phase 8 as otherwise "source-complete." I additionally flag (as documentation-only, non-blocking) that REQUIREMENTS.md's UX-13 checkbox and note are stale and should be corrected to reflect that OS-level modifier matching on macOS has been complete since Plan 08-03.

**Recommended next step:** Route to human verification for the real-device macOS/Windows hotkey tests (08-VERIFICATION.md Section 5/6). Once complete, mark the ROADMAP.md Phase 8 checkbox done and correct REQUIREMENTS.md's UX-13 entry.

---

_Verified: 2026-08-04T14:44:40Z_
_Verifier: Claude (gsd-verifier)_
