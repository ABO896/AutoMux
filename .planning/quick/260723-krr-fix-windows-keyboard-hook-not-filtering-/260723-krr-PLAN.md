---
phase: 260723-krr
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/platform/windows/mod.rs
  - .planning/todos/pending/2026-07-23-windows-hook-injected-event-filtering.md
  - .planning/todos/completed/2026-07-23-windows-hook-injected-event-filtering.md
autonomous: true
requirements:
  - 09-REVIEW-CR-02
must_haves:
  truths:
    - "An injected keydown/syskeydown event on Windows (SendInput-originated, KBDLLHOOKSTRUCT.flags has LLKHF_INJECTED set) never triggers the Ctrl+Shift+Q emergency-stop check."
    - "An injected keydown/syskeydown event on Windows never matches a configured hotkey binding, so a macro's own injected keystroke cannot self-trigger a ToggleMacroHotkey Intent or self-cancel/self-stop another macro."
    - "Real hardware keydown/syskeydown events are completely unaffected — emergency-stop and hotkey matching still fire exactly as before the fix."
    - "The injected-event predicate has automated regression coverage that runs via a plain `cargo test` on any host (this macOS dev machine included), independent of the missing Windows cross-compile target."
  artifacts:
    - "src-tauri/src/platform/windows/mod.rs — new `flags_indicate_injected(u32) -> bool` predicate, `hook_callback`'s WM_KEYDOWN/WM_SYSKEYDOWN branch gated on `!flags_indicate_injected(kb_struct.flags)`"
    - "src-tauri/src/platform/windows/mod.rs — new ungated `#[cfg(test)] mod injected_filter_tests` covering hardware/extended/injected/injected+extended flag combinations"
    - ".planning/todos/completed/2026-07-23-windows-hook-injected-event-filtering.md — backlog item closed out"
  key_links:
    - "hook_callback's `if msg_id == WM_KEYDOWN || msg_id == WM_SYSKEYDOWN` condition -> `&& !flags_indicate_injected(kb_struct.flags)` -> gates BOTH the emergency-stop check and the CONFIGURABLE HOTKEYS loop with a single guard, mirroring the macOS CGEventTap's LLMHF_INJECTED early-return (`platform/macos/observer.rs:279-281`)."
---

<objective>
Fix the missing injected-event filter in the Windows low-level keyboard hook: `hook_callback` (`src-tauri/src/platform/windows/mod.rs`) never inspects `KBDLLHOOKSTRUCT.flags` for `LLKHF_INJECTED`, so a macro's own `SendInput`-synthesized keystroke can match another macro's configured hotkey — or the hardcoded Ctrl+Shift+Q emergency-stop combo — and self-trigger a toggle or emergency stop. This directly threatens the Core Value ("execution must be accurate"). The macOS `CGEventTap` already guards against exactly this (`observer.rs:279-281`); Windows has never had the equivalent check since the hook was first written (v1.0, commit faa1e1e).

Apply the pre-diagnosed fix (09-REVIEW.md CR-02 / `.planning/todos/pending/2026-07-23-windows-hook-injected-event-filtering.md`): extract the injected-event check into a plain, host-portable `u32 -> bool` predicate (not gated to `target_os = "windows"`, unlike `hook_callback` itself) so it can carry real automated test coverage on this macOS host despite the Windows cross-compile target not being installed. Wire that predicate into `hook_callback`'s single `if` guard so it skips BOTH the emergency-stop check and hotkey matching for injected events, then close out the backlog todo.

Purpose: Protect macro execution accuracy on Windows — an active macro's injected keystrokes must never be able to trigger, cancel, or emergency-stop macros (including itself), matching the safety guarantee macOS already has.
Output: Patched `hook_callback` + new `flags_indicate_injected` predicate with unit tests in `windows/mod.rs`, and the backlog todo moved to `completed/`.
</objective>

<execution_context>
@$HOME/.claude/gsd-core/workflows/execute-plan.md
@$HOME/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

# The file being patched — hook_callback is at the bottom (~line 493-540); imports/statics are above it:
@src-tauri/src/platform/windows/mod.rs

# The existing macOS injected-event guard to mirror (LLMHF_INJECTED check + early-return):
@src-tauri/src/platform/macos/observer.rs

# Project conventions: Rust snake_case, doc-comment tags (@safety-officer etc.), cfg-gated platform code, test module patterns
@CLAUDE.md

# The backlog item being closed — problem/solution description and CR-02 fix sketch reference:
@.planning/todos/pending/2026-07-23-windows-hook-injected-event-filtering.md
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add flags_indicate_injected predicate and gate hook_callback's emergency-stop + hotkey matching for injected events</name>
  <files>src-tauri/src/platform/windows/mod.rs</files>
  <behavior>
    - `flags_indicate_injected(0x0)` -> `false` (real hardware key, no flags set)
    - `flags_indicate_injected(0x1)` -> `false` (LLKHF_EXTENDED only — arrow keys, numpad, etc. — must not be mistaken for injected)
    - `flags_indicate_injected(0x10)` -> `true` (LLKHF_INJECTED set — SendInput/keybd_event synthesized)
    - `flags_indicate_injected(0x11)` -> `true` (LLKHF_INJECTED + LLKHF_EXTENDED both set — injected still detected)
  </behavior>
  <action>
Add a new free function `flags_indicate_injected(flags: u32) -> bool` immediately above the `#[cfg(target_os = "windows")]` attribute that precedes `unsafe extern "system" fn hook_callback`. Do NOT gate this new function to `target_os = "windows"` — it takes and returns only plain `u32`/`bool`, so it compiles (and is unit-testable) on every host, unlike `hook_callback` itself which only compiles on Windows. Body: declare a local `const LLKHF_INJECTED: u32 = 0x0000_0010;` (the stable Win32 `winuser.h` constant, unchanged since Windows 2000 — same "hardcode the stable Win32 constant with a citation comment" pattern the file already uses for `MOD_ALT`/`MOD_CONTROL`/`MOD_SHIFT`/`MOD_WIN` in the `windows_mod_constants` test) and return `flags & LLKHF_INJECTED != 0`.

Give the function a doc comment starting with `@safety-officer:` (per CLAUDE.md's doc-comment-tag convention for safety-critical invariants) stating: LLKHF_INJECTED marks a `WH_KEYBOARD_LL` event as `SendInput`/`keybd_event`-synthesized; `hook_callback` MUST treat these identically to the macOS CGEventTap's `LLMHF_INJECTED` guard (`platform/macos/observer.rs:279-281`) by skipping the emergency-stop check AND hotkey matching, or a macro's own injected keystroke can self-trigger a toggle or emergency stop (CR-02, `.planning/todos/pending/2026-07-23-windows-hook-injected-event-filtering.md`). Add `#[allow(dead_code)]` above the function with a one-line comment explaining why: on non-Windows hosts `hook_callback` (its only caller) is compiled out by its own `#[cfg(target_os = "windows")]`, which would otherwise make this ungated function trip `cargo clippy --all-targets -- -D warnings`; the attribute is inert on Windows, where the function is genuinely called.

In `hook_callback`, change the line `if msg_id == WM_KEYDOWN || msg_id == WM_SYSKEYDOWN {` to `if (msg_id == WM_KEYDOWN || msg_id == WM_SYSKEYDOWN) && !flags_indicate_injected(kb_struct.flags) {`. This is the ONLY change inside `hook_callback` — do not restructure the "Emergency stop check: Ctrl + Shift + Q" block or the "CONFIGURABLE HOTKEYS (per-binding)" loop; both already live inside this single `if`, so gating the condition once causes injected events to skip both, exactly as required. Real (non-injected) `WM_KEYDOWN`/`WM_SYSKEYDOWN` events are completely unaffected — `CallNextHookEx` at the bottom of the function still always runs, so injected events still pass through to the rest of the OS input pipeline unmodified; only AutoMux's own emergency-stop/hotkey reaction to them is suppressed.

Add a one-line comment directly above the modified `if` referencing CR-02 and the macOS mirror location, so a future reader sees why the extra `&& !flags_indicate_injected(...)` clause exists.
  </action>
  <verify>
    <automated>cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5</automated>
    <automated>cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -10</automated>
    <automated>def=$(grep -n "^fn flags_indicate_injected" src-tauri/src/platform/windows/mod.rs | head -1 | cut -d: -f1); guard=$(grep -n "flags_indicate_injected(kb_struct.flags)" src-tauri/src/platform/windows/mod.rs | head -1 | cut -d: -f1); emergency=$(grep -n "Emergency stop check" src-tauri/src/platform/windows/mod.rs | head -1 | cut -d: -f1); hotkeys=$(grep -n "CONFIGURABLE HOTKEYS" src-tauri/src/platform/windows/mod.rs | head -1 | cut -d: -f1); [ -n "$def" ] && [ -n "$guard" ] && [ -n "$emergency" ] && [ -n "$hotkeys" ] && [ "$guard" -lt "$emergency" ] && [ "$guard" -lt "$hotkeys" ] && echo "OK: injected guard wraps both emergency-stop and hotkey matching"</automated>
  </verify>
  <done>`cargo build` and `cargo clippy --all-targets -- -D warnings` are both clean on this macOS host. `flags_indicate_injected` is defined and its call site (`flags_indicate_injected(kb_struct.flags)`) appears in source before both the "Emergency stop check" and "CONFIGURABLE HOTKEYS" blocks — proving the single `if`-condition gates both behaviors for injected events. Non-injected events are unaffected (the only change is the added `&&` clause on the existing condition).</done>
</task>

<task type="auto">
  <name>Task 2: Add cross-platform unit tests proving the injected-event predicate is correct</name>
  <files>src-tauri/src/platform/windows/mod.rs</files>
  <action>
`hook_callback` itself only compiles under `#[cfg(target_os = "windows")]`, and this repo has no `x86_64-pc-windows-msvc` target installed on this host (same disposition as Phase 8 Plans 3/6 — see STATE.md Decisions); mocking a `WH_KEYBOARD_LL` hook end-to-end is not standard/practical either way. That is exactly why Task 1 extracted `flags_indicate_injected` as a plain, ungated function — it lets the fix's core logic get real, automated regression coverage on any host, including this one.

Add a new `#[cfg(test)] mod injected_filter_tests { ... }` at the end of the file, immediately after the existing `#[cfg(test)] #[cfg(target_os = "windows")] mod tests { ... }` block's closing brace. Do NOT add `#[cfg(target_os = "windows")]` to this new module — it must run on every host. Inside it, `use super::flags_indicate_injected;` and add four `#[test]` functions:
- `real_hardware_keydown_is_not_injected` — asserts `!flags_indicate_injected(0x0)`.
- `extended_key_flag_alone_is_not_injected` — asserts `!flags_indicate_injected(0x1)`, with a comment noting `LLKHF_EXTENDED` (0x1) is set for many real keys (arrows, numpad) and must not be mistaken for `LLKHF_INJECTED`.
- `sendinput_event_is_injected` — asserts `flags_indicate_injected(0x10)`.
- `injected_plus_extended_flags_still_detected` — asserts `flags_indicate_injected(0x11)`.

Add a doc comment above the module citing CR-02 and explaining, in one or two sentences, that this predicate extraction gives the fix real automated coverage independent of Windows-target availability (mirror the rationale already stated in Task 1, don't just repeat it verbatim).
  </action>
  <verify>
    <automated>cargo test --manifest-path src-tauri/Cargo.toml injected_filter_tests 2>&1 | tail -20</automated>
    <automated>cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -10</automated>
  </verify>
  <done>All four new tests in `injected_filter_tests` compile and pass via plain `cargo test` on this macOS host (no Windows target required), proving the `flags & 0x10 != 0` logic correctly distinguishes real hardware, extended-only, injected, and injected+extended flag combinations. Full `cargo test --manifest-path src-tauri/Cargo.toml` remains green.</done>
</task>

<task type="auto">
  <name>Task 3: Close out the backlog todo</name>
  <files>.planning/todos/completed/2026-07-23-windows-hook-injected-event-filtering.md</files>
  <action>
Move the backlog item from pending to completed now that the fix + regression tests are in place. Create the completed directory if it does not already exist, then move the file: `mkdir -p .planning/todos/completed` and `git mv .planning/todos/pending/2026-07-23-windows-hook-injected-event-filtering.md .planning/todos/completed/2026-07-23-windows-hook-injected-event-filtering.md` (fall back to a plain `mv` if the file is untracked). Do not edit the file's contents — the move alone closes the item.
  </action>
  <verify>
    <automated>test -f .planning/todos/completed/2026-07-23-windows-hook-injected-event-filtering.md && test ! -f .planning/todos/pending/2026-07-23-windows-hook-injected-event-filtering.md && echo MOVED</automated>
  </verify>
  <done>The todo file exists under .planning/todos/completed/ and no longer exists under .planning/todos/pending/.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Windows `WH_KEYBOARD_LL` hook stream ↔ AutoMux's own `SendInput`-injected keystrokes | The same low-level keyboard event stream carries both real hardware input and AutoMux's own synthesized macro playback; without a source check, the hook cannot distinguish them. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-krr-01 | Spoofing (injected event impersonates real hardware input, triggering unintended control-plane actions) | `hook_callback` in `src-tauri/src/platform/windows/mod.rs` | high | mitigate | Add `flags_indicate_injected(kb_struct.flags)` guard on the existing `if msg_id == WM_KEYDOWN \|\| msg_id == WM_SYSKEYDOWN` condition, skipping BOTH the Ctrl+Shift+Q emergency-stop check and hotkey-binding matching for injected events (Task 1) — mirrors the macOS CGEventTap's existing `LLMHF_INJECTED` guard. Unit-tested via `injected_filter_tests` on flag combinations including injected+extended (Task 2). |

No package-manager installs and no new external inputs are introduced by this change; no supply-chain checkpoint required.
</threat_model>

<verification>
- `cargo build --manifest-path src-tauri/Cargo.toml` succeeds.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` is clean.
- `cargo test --manifest-path src-tauri/Cargo.toml` is green, including the four new `injected_filter_tests` tests.
- `flags_indicate_injected(kb_struct.flags)` appears in source before both the emergency-stop check and the hotkey-matching loop inside `hook_callback`.
- Backlog todo moved from pending/ to completed/.
</verification>

<success_criteria>
- An injected Windows keydown/syskeydown event no longer reaches either the Ctrl+Shift+Q emergency-stop check or the configurable-hotkey matching loop.
- A real hardware keydown/syskeydown event behaves exactly as before this fix.
- The injected-event predicate has automated, host-portable test coverage (passes via plain `cargo test` on macOS despite no Windows cross-compile target being installed).
- Backlog item 2026-07-23-windows-hook-injected-event-filtering is in completed/.
</success_criteria>

<output>
Create `.planning/quick/260723-krr-fix-windows-keyboard-hook-not-filtering-/260723-krr-SUMMARY.md` when done.
</output>
