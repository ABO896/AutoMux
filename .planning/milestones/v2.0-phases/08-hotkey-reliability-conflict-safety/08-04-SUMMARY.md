---
phase: 08-hotkey-reliability-conflict-safety
plan: 04
subsystem: frontend-ux
tags: [typescript, solidjs, key-capture, modifier-bits, cgeventflags, mod-constants, ux-11, ux-13, conflict-detection, error-routing]

# Dependency graph
requires:
  - phase: 08-hotkey-reliability-conflict-safety
    plan: 01
    provides: "MacroConfig.trigger_modifiers, AppState.conflicts, pinned CGEventFlag*/MOD_* bit constants, tuple-keyed MACRO_TRIGGER_KEYS"
  - phase: 08-hotkey-reliability-conflict-safety
    plan: 03
    provides: "Intent::BindHotkey with Result<(), String> oneshot reply, rewritten bind_hotkey / unbind_hotkey IPC, Windows HOTKEY_BINDINGS"
provides:
  - "computeModifiers(e: KeyboardEvent) helper that returns platform-native bitmask (CGEventFlags on macOS, MOD_* on Windows)"
  - "recordingModifiers + newMacroTriggerModifiers SolidJS signals"
  - "conflictError signal + showConflictError helper (8s auto-dismiss per UI-SPEC C-1)"
  - "TS MacroConfig.trigger_modifiers: number (mirrors Rust u64)"
  - "TS AppState.conflicts: Array<{macros, input}> (mirrors Rust Vec<InputConflict>)"
  - "startCapture onCommit signature (keycode, modifiers) — modifier bits threaded through DOM to backend"
  - "handleCardSetTriggerKey signature (id, keycode, modifiers) — real bits sent to bind_hotkey and set_macro_trigger_key"
  - "handleCreateMacro config carries newMacroTriggerModifiers() — UX-13 frontend complete"
  - "try/catch around bind/create IPC calls populates setConflictError — UX-11 frontend plumbing complete"
  - "Removed [Control, Shift, Alt, Meta] filter at startCapture — bare modifier binds now allowed"
affects:
  - "08-05 (C-1 ConflictErrorToast reads conflictError; C-5 ModifierPreviewChip reads recordingModifiers; C-2/C-3/C-4 add visible UI on top of these signals)"
  - "08-06 (Full platform verification: Cmd+F5 round-trip on macOS, Ctrl+F5 round-trip on Windows; UX-11 visible-error verification)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Module-level pure-helper for platform bit translation (computeModifiers next to formatInputEvent / formatStep)"
    - "Inline platform detection inside module-level helpers (avoids the App()-scoped IS_MACOS closure dependency)"
    - "Single-slot auto-dismiss timer ref pattern for transient toasts (mirrors showProfileMsg at 3s, conflictError at 8s)"
    - "try/catch on every bind IPC that parses the backend's well-known '... assigned to \"<name>\" ...' error format"
    - "void references for SolidJS signal getters whose visual consumers land in a later plan (satisfies noUnusedLocals: true)"

key-files:
  created: []
  modified:
    - src/App.tsx

key-decisions:
  - "computeModifiers is module-level (next to formatInputEvent / formatStep) with inline IS_MACOS detection rather than inside App() using the closure-bound constant. Same detection result, broader reuse surface, doesn't conflict with the existing App()-scoped IS_MACOS used for render branching."
  - "Combined Tasks 1 and 2 into a single commit. The plan's two-task separation (interface extension vs threading) doesn't survive tsconfig's noUnusedLocals: true: declaring the new signals and helper without using them in the same commit leaves a broken build. One logical change, one commit, fully tested."
  - "Added void references for the conflictError and recordingModifiers signal getters. The visual consumers (C-1 toast, C-5 chip) ship in Plan 08-05 per the plan's explicit scope boundary. The void references are no-op runtime reads that satisfy the strict TypeScript setting without rendering partial UI in this plan."
  - "showConflictError stores the timer ID in a module-scoped ref so a new conflict replaces the previous timer (mirrors the 3s profile message toast pattern at App.tsx:415-418). The 8-second duration per UI-SPEC C-1 is intentionally longer than the 3s profile message so the user has time to read the conflicting macro name."
  - "Conflict error parsing extracts the macro name from the backend's 'is already assigned to \"<name>\"' format. If the format doesn't match (defensive), the macro name falls back to the literal 'another macro' and the key is the resolved key name. The user is never blocked; the toast appears with whatever information is available."
  - "Removed the [Control, Shift, Alt, Meta] early-return filter per RESEARCH.md §3.3 step 1 and the plan's 'must_haves.truths' — bare modifier binds are now committed and reach the backend. The keymap.ts CGKEYCODE_TO_NAME / VK_TO_NAME tables already include modifier names (Shift/Cmd/Ctrl/Alt/Option), so a pure modifier binding renders correctly via resolveKeyName."

patterns-established:
  - "When extending TypeScript interfaces whose corresponding SolidJS signals are written but not yet read, use `void signalName;` references to satisfy noUnusedLocals: true without rendering partial UI. Document the deferred consumer in the comment."
  - "Conflict errors from the backend are parsed client-side with a regex and surfaced via a structured signal (conflictError). The regex extracts the conflicting macro's display name; parsing failures fall back to a generic name without throwing."

requirements-completed: [UX-11, UX-13]

# Metrics
duration: 5min
completed: 2026-06-30
---

# Phase 8 Plan 4: Frontend Modifier Pass-Through & Conflict Wiring Summary

**DOM KeyboardEvent modifier bits now reach the Rust backend end-to-end via a new `computeModifiers` helper, and the conflict errors returned by `Intent::BindHotkey` / `Intent::AddMacro` / `set_macro_trigger_key` populate a new `setConflictError` signal ready for the C-1 toast in Plan 08-05.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-30T21:29:38Z
- **Completed:** 2026-06-30T21:35:04Z
- **Tasks:** 2 (combined into a single commit — see Deviations)
- **Files modified:** 1 (`src/App.tsx`)

## Accomplishments

- `computeModifiers(e: KeyboardEvent): number` returns the platform-native bitmask (CGEventFlags bits on macOS, MOD_* values on Windows) — bit values match the constants pinned by `cg_event_flag_constants` and `windows_mod_constants` unit tests in Plan 08-01
- `recordingModifiers` and `newMacroTriggerModifiers` signals track the held modifier bits during capture and on the new-macro form
- `setConflictError` signal + `showConflictError` helper provide the UX-11 plumbing — the actual C-1 toast component ships in Plan 08-05
- `startCapture`'s `onCommit` callback now receives `(keycode, modifiers)`; the `["Control", "Shift", "Alt", "Meta"]` early-return filter is removed so users can bind bare modifiers
- `handleCreateMacro` builds a `MacroConfig` with `trigger_modifiers: newMacroTriggerModifiers()` — no more `0` placeholder
- `handleCardSetTriggerKey(id, keycode, modifiers)` forwards the real bitmask to both `bind_hotkey` and `set_macro_trigger_key` on macOS, and to `set_macro_trigger_key` on Windows
- Both bind/create flows are wrapped in `try/catch` that parse the backend's `... assigned to "<name>" ...` error format and populate the conflict signal
- TypeScript interfaces mirror the new Rust state shape: `MacroConfig.trigger_modifiers: number` and `AppState.conflicts: Array<{macros, input}>`

## Task Commits

1. **Task 1 + Task 2: `computeModifiers` helper + signals + interfaces + threading + try/catch** — `5031995` (feat)

**Plan metadata:** (this SUMMARY commit)

_Note: Tasks 1 and 2 of 08-04-PLAN.md were combined into a single commit because the strict `noUnusedLocals: true` setting in tsconfig requires the new signals and helper to be read (not just declared) for the build to be green. Splitting the changes would have left a broken intermediate tree. See Deviations for the rationale._

## Files Created/Modified

- `src/App.tsx` — Added `computeModifiers` module-level helper, `recordingModifiers` + `newMacroTriggerModifiers` + `conflictError` signals, `showConflictError` helper, extended `MacroConfig` and `AppState` TypeScript interfaces, removed the modifier-only filter in `startCapture`, threaded `modifiers` through `startCapture` → `handleCreateMacro` + `handleCardSetTriggerKey` → `bind_hotkey` / `set_macro_trigger_key` / `add_macro` IPC calls, added `try/catch` + error parsing in both `handleCreateMacro` and `handleCardSetTriggerKey`

## Decisions Made

- **Module-level `computeModifiers` with inline `IS_MACOS`:** Placed at module level (next to `formatInputEvent` / `formatStep`) rather than inside `App()`. The inline `navigator.userAgent.toLowerCase().includes("mac")` check mirrors the closure-bound `IS_MACOS` constant inside `App()`. Same detection result, broader reuse surface, no App()-scoped dependencies for a pure bit-translation function. The 08-PATTERNS.md document listed `IS_MACOS` as a module-level constant; the actual placement is inside `App()` (line 191), so the helper cannot reference the closure-bound constant from module scope.
- **Combined Tasks 1 and 2 into a single commit:** The plan's two-task separation (interface extension vs. threading) was incompatible with `noUnusedLocals: true`. Declaring the new signals and the `computeModifiers` helper without using them in the same commit leaves a broken build at the Task 1 commit boundary. The combined commit has 137 insertions, 10 deletions, all green on `tsc --noEmit` and `cargo check --all-targets`.
- **`void conflictError;` and `void recordingModifiers;` references:** The plan explicitly defers the C-1 `ConflictErrorToast` (reads `conflictError`) and the C-5 `ModifierPreviewChip` (reads `recordingModifiers`) to Plan 08-05. With `noUnusedLocals: true`, the signal getters would be flagged as unused. The `void` references are no-op runtime reads that satisfy the compiler without rendering partial UI. Documented inline in the signal declaration.
- **Single-slot auto-dismiss timer ref for `showConflictError`:** Mirrors the existing 3s `showProfileMsg` pattern (`App.tsx:415-418`). Stores the timer ID in a module-scoped `let _conflictErrorTimer` so a new conflict replaces the previous timer (avoids the "stale timer" bug where two overlapping toasts both dismiss the wrong one). The 8-second duration per UI-SPEC C-1 is intentionally longer than the 3s profile message so the user has time to read the conflicting macro name and decide.
- **Conflict error regex parsing with fallback:** Parses the backend's `... assigned to "<name>" ...` format using a regex; extracts the macro name into the toast. If the format doesn't match (defensive against backend error format changes), falls back to the literal `"another macro"` for the macro name and the resolved key name (via `resolveKeyName`) for the key. The user is never blocked by a parsing failure — the toast appears with whatever information is available.
- **Removed the modifier-only filter:** Per RESEARCH.md §3.3 step 1 and the plan's `must_haves.truths`. The `keymap.ts` `CGKEYCODE_TO_NAME` (lines 96-97) and `VK_TO_NAME` (line 167) tables already include modifier names (Shift, Cmd, Ctrl, Alt, Option), so a pure modifier binding renders correctly via `resolveKeyName`. The backend's CGEventTap and Windows WH_KEYBOARD_LL hooks do not currently match `FlagsChanged` events on macOS, so a bare-modifier binding is committed but the OS-level hotkey detection is a future concern (tracked in RESEARCH.md Open Questions #1).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Combined Tasks 1 and 2 into a single commit (no separate Task 1 and Task 2 commits)**
- **Found during:** Task 1 — first `tsc --noEmit` after adding the new signals and the `computeModifiers` helper
- **Issue:** `tsconfig.json` has `noUnusedLocals: true` and `noUnusedParameters: true`. The new declarations from Task 1 (`computeModifiers`, `recordingModifiers`, `newMacroTriggerModifiers`, `conflictError`, `showConflictError`) are all unused until Task 2's threading work uses them. The plan's instruction "Do NOT yet thread the modifiers into the IPC calls" + the verification step "`npx tsc --noEmit` exits 0" are in direct conflict: a Task 1 commit that respects the no-threading rule fails tsc.
- **Fix:** Combined Tasks 1 and 2 into a single commit (`5031995`) with the full plan content. The commit message explicitly documents the deviation. All Task 1 + Task 2 acceptance criteria pass on the combined commit.
- **Files modified:** `src/App.tsx`
- **Verification:** `npx tsc --noEmit` exits 0; `grep -c 'modifiers: 0' src/App.tsx` returns 0; `cargo check --all-targets` exits 0; `cargo test --lib` 8/8 pass
- **Committed in:** `5031995`

**2. [Rule 2 - Missing Critical] Added `void` references to satisfy `noUnusedLocals: true` for the deferred consumers**
- **Found during:** Same compile check as deviation #1
- **Issue:** `conflictError` and `recordingModifiers` getters would still be flagged even with the threading in place, because the plan explicitly defers the C-1 toast (reads `conflictError`) and C-5 chip (reads `recordingModifiers`) to Plan 08-05. A SolidJS signal with only the setter used is still flagged by `noUnusedLocals: true` for the unused getter.
- **Fix:** Added `void conflictError;` and `void recordingModifiers;` references immediately after the signal declarations. The `void` operator is a no-op runtime expression; it calls the getter and discards the value. SolidJS getters are pure functions with no side effects, so the call is safe. The references are documented in inline comments.
- **Files modified:** `src/App.tsx`
- **Verification:** `npx tsc --noEmit` exits 0; both signals are writable from the IPC handlers (`showConflictError`, `startCapture`) and will be readable from the C-1 and C-5 components in Plan 08-05
- **Committed in:** `5031995` (part of the combined commit)

---

**Total deviations:** 2 auto-fixed (2 blocking — same root cause: `noUnusedLocals: true` strict mode)
**Impact on plan:** Both auto-fixes necessary to keep the TypeScript build green. The first deviation collapses the plan's two-task commit structure into one; the second adds two no-op reference statements that do not affect behavior. No scope creep — every change is within the plan's `must_haves.truths`, `artifacts`, and `key_links` contract.

## Known Stubs

| Location | Stub | Reason | Resolved By |
|----------|------|--------|-------------|
| `src/App.tsx:170-172` | `void conflictError;` | The C-1 `ConflictErrorToast` (UX-11 visible surface) is deferred to Plan 08-05 per the plan's explicit scope boundary. The void reference is a no-op runtime read that satisfies `noUnusedLocals: true`. | Plan 08-05 C-1 — `ConflictErrorToast` reads `conflictError()` and renders the danger-tinted toast with the key + macro name. |
| `src/App.tsx:199-201` | `void recordingModifiers;` | The C-5 `ModifierPreviewChip` (UX-13 visible surface) is deferred to Plan 08-05 per the plan's explicit scope boundary. The void reference is a no-op runtime read that satisfies `noUnusedLocals: true`. | Plan 08-05 C-5 — `ModifierPreviewChip` reads `recordingModifiers()` and renders the modifier chip row during capture. |

## Issues Encountered

- None beyond the deviations above. The Rust backend, keymap.ts, and `keymap.ts` modifier tables required no changes — the frontend changes are self-contained in `src/App.tsx`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 08-05 (UI surfaces for UX-11, UX-12, UX-14) can proceed:

- `conflictError` signal is populated by `showConflictError` from both `handleCreateMacro` and `handleCardSetTriggerKey` error paths — the C-1 toast (UX-11) renders this signal with the key + macro name. 8-second auto-dismiss timer is already in place.
- `recordingModifiers` signal is updated by `startCapture` on every keydown — the C-5 chip (UX-13) renders this during capture to show the held modifier combo.
- `conflicts: Array<{macros, input}>` is now in the `AppState` TypeScript interface — the C-2 `ConflictWarningRegion` (UX-12) renders `state().conflicts` as a list of warning cards.
- `newMacroTriggerModifiers` is sent as `trigger_modifiers` on every new macro — the per-macro card rendering in C-4 (`↗ Global` subtitle) and the existing `resolveKeyName(macro.trigger_key!)` chip can be extended to also display the modifier combo if desired.

Plan 08-06 (full platform verification) can proceed:

- macOS: bind `Cmd+F5` to macro A, bind `F5` to macro B, expect a `bind_hotkey` IPC with `modifiers: 0x100000` and a conflict error in the UI. The bit constant is verified by `cg_event_flag_constants` test (08-01).
- Windows: bind `Ctrl+F5` to macro A, bind `F5` to macro B, expect a `bind_hotkey` IPC with `modifiers: 0x0002` and a conflict error in the UI. The bit constant is verified by `windows_mod_constants` test (08-01).
- The `showConflictError` regex expects the format `is already assigned to "<name>"`. The Rust `check_trigger_key_conflict` in Plan 08-02 must produce error strings matching this pattern; the test in `bind_conflict_rejected` (08-02) pins the macro name in the error.

## Verification Commands Run

```bash
# TypeScript build green
npx tsc --noEmit
# → exit 0

# No hardcoded modifiers: 0 in IPC payloads
grep -c 'modifiers: 0' src/App.tsx
# → 0

# The modifier-only filter is removed
grep -n '"Control", "Shift", "Alt", "Meta"' src/App.tsx
# → no match

# Rust regression suite green
cd src-tauri && cargo check --all-targets
# → Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s

cd src-tauri && cargo test --lib
# → test result: ok. 8 passed; 0 failed; 0 ignored
```

## Self-Check: PASSED

- `08-04-SUMMARY.md` exists at `.planning/phases/08-hotkey-reliability-conflict-safety/08-04-SUMMARY.md`
- Task commit `5031995` (feat) is present in git log
- `src/App.tsx` contains `function computeModifiers(e: KeyboardEvent): number` and the function body branches on `IS_MACOS`
- The macOS branch ORs in `0x020000` / `0x040000` / `0x080000` / `0x100000`
- The non-macOS branch ORs in `0x0004` / `0x0002` / `0x0001` / `0x0008`
- `recordingModifiers` and `newMacroTriggerModifiers` signals declared with `createSignal<number>(0)`
- `conflictError` signal declared and populated by `showConflictError` from both error paths
- `MacroConfig.trigger_modifiers: number` interface field present (line 24)
- `AppState.conflicts: Array<{macros, input}>` interface field present (line 44)
- `startCapture` no longer contains the `["Control", "Shift", "Alt", "Meta"]` filter
- `startCapture`'s `onCommit` callback signature is `(nativeCode: number, modifiers: number) => void`
- `handleCreateMacro` builds `config` with `trigger_modifiers: newMacroTriggerModifiers()` and resets the signal on success
- `handleCardSetTriggerKey` signature is `(id: string, nativeCode: number, modifiers: number)`
- `handleCardSetTriggerKey` sends `modifiers` (not `0`) to `bind_hotkey` and both `set_macro_trigger_key` calls
- `handleCreateMacro` and `handleCardSetTriggerKey` are wrapped in `try/catch` that call `showConflictError`
- The 3 `startCapture` callsites (new-macro form + unset-key branch + bound-key branch) all pass the `mods` argument
- `npx tsc --noEmit` exits 0
- `grep -c 'modifiers: 0' src/App.tsx` returns 0
- `cargo check --all-targets` exits 0 (no Rust changes this plan)
- `cargo test --lib` 8/8 pass (regression suite green)

---
*Phase: 08-hotkey-reliability-conflict-safety*
*Completed: 2026-06-30*
