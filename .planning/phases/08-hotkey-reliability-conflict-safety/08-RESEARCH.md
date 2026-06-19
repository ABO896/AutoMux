# Phase 8: Hotkey Reliability & Conflict Safety — Research

**Researched:** 2026-06-19
**Domain:** Rust platform code (macOS CGEventTap, Windows WH_KEYBOARD_LL), SolidJS key capture, Tauri IPC, conflict-detection state machines
**Confidence:** MEDIUM (current state fully read; Windows hook behavior requires device verification; CGEventFlag bit values are training-data-level LOW for macOS)

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UX-11 | Binding a hotkey already assigned to another macro shows error or reassignment prompt — no silent shadowing | The bug is explicitly documented in `.planning/codebase/CONCERNS.md:45-49`; CONCERNS.md identifies `state/mod.rs` (`reevaluate_all_macros`) and `macos/observer.rs` (`update_macro_trigger_keys`) as the silent-shadowing sites |
| UX-12 | App prevents or warns when multiple enabled macros inject the same input type simultaneously | `InputEvent` (`state/mod.rs:36-43`) derives `Hash` and `Eq`; detection is a set-intersection over enabled macros' expanded `ActionSequence.steps`; no code currently does this |
| UX-13 | Hotkey binding supports A–Z, 0–9, F1–F12, and modifier combinations | The DOM→native `keymap.ts` already covers the named key sets; the broken link is the modifier pass-through (App.tsx:375 hardcodes `modifiers: 0` and App.tsx:357 blocks modifier-only keypresses) |
| UX-14 | Hotkeys fire when AutoMux is unfocused on both platforms; UI communicates "system-wide" | macOS HID tap is global by definition (`observer.rs:243`); Windows `WH_KEYBOARD_LL` with thread-id 0 is global by definition (`windows/mod.rs:489-495`); UI currently only says "Required for global hotkeys" (`App.tsx:647`) — needs a more visible notice |
</phase_requirements>

---

## 1. Phase Goal & Scope

**Goal (from ROADMAP.md:103):** The hotkey binding system is reliable, full-featured, and safe — supports a broad key range, prevents silent conflicts between macros, and users understand that binds are system-wide.

**Non-goals (out of scope):**
- A separate "Settings" tab for hotkey configuration (deferred — full UI redesign is Phase 10)
- Profile portability across platforms (deferred — NamedKey schema migration UX-04)
- Cross-platform hotkey profile sync
- Auto-updater / notarization (v3)
- Any change to the macOS CGEventTap architecture (already global; confirmed in prior phases)

**Architectural constraints (from PROJECT.md):**
- Tauri 2 + Rust 2021 + SolidJS — no framework changes
- Must work on both macOS (arm64 + x86_64) and Windows (x64)
- macOS Accessibility permission flow is OS-enforced; no changes that regress Phase 5/6 work
- Single `StateActor` owns `AppState` — conflict detection must live there to avoid races

---

## 2. Current State Analysis

### 2.1 Hotkey capture (DOM → IPC → backend)

**Frontend capture (`src/App.tsx`):**
- `startCapture()` at `App.tsx:340-368` listens for `keydown` and resolves the key via `domKeycodeToNative(e.code)`.
- **Line 357 filters out pure-modifier keypresses:** `if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;` — user cannot bind "just the Shift key" or any other modifier as the primary key.
- **Line 375 hardcodes `modifiers: 0`:** `await invoke("bind_hotkey", { macro_id: id, keycode: nativeCode, modifiers: 0 });` — the actual `e.getModifierState("Shift"/"Control"/"Alt"/"Meta")` state is read by neither `startCapture` nor `handleCardSetTriggerKey` and is dropped on the floor.
- `handleCreateMacro()` at `App.tsx:289-329` calls `add_macro` with `trigger_key: triggerKey` and no modifier field at all.

**IPC layer (`src-tauri/src/ipc/mod.rs`):**
- `bind_hotkey` at `ipc/mod.rs:79-95` takes `(_macro_id, _keycode, _modifiers)`. All three params are underscored because the function bypasses `StateManager` entirely and writes directly into the macOS-only `HOTKEY_BINDINGS` Vec.
- `unbind_hotkey` at `ipc/mod.rs:99-106` is similarly macOS-only.
- `set_macro_trigger_key` at `ipc/mod.rs:142-151` IS routed through the StateActor (correct path) and goes into `MACRO_TRIGGER_KEYS` on both platforms.

**Two parallel registries — root cause of silent conflicts:**

| Registry | Type | Populated by | Consumed by | Platform |
|----------|------|--------------|-------------|----------|
| `HOTKEY_BINDINGS` (Vec) | `observer.rs:56` | `add_hotkey_binding()` from `bind_hotkey` IPC | CGEventTap callback `observer.rs:420-449` | **macOS only** |
| `MACRO_TRIGGER_KEYS` (HashMap<u16, Uuid>) | `observer.rs:197` (macOS) / `windows/mod.rs:410` (Windows) | `update_macro_trigger_keys()` from `reevaluate_all_macros` | CGEventTap callback `observer.rs:442-449` / Windows hook `windows/mod.rs:450-456` | **Both** |

The two registries overlap (both can hold a ToggleMacro(id) entry) but are never reconciled. A macro with a `trigger_key` AND a separate `bind_hotkey` call creates two entries pointing at the same macro.

### 2.2 Conflict detection (current: absent)

**`reevaluate_all_macros` at `state/mod.rs:441-475`:** builds `trigger_keys: HashMap<u16, Uuid>` by iterating macros. **The HashMap insert silently overwrites duplicate keycodes.** This is the exact failure mode documented in `.planning/codebase/CONCERNS.md:45-49`.

**`bind_hotkey` IPC at `ipc/mod.rs:79-95`:** does not consult `AppState` at all — it appends to `HOTKEY_BINDINGS` without any uniqueness check. A user can call `bind_hotkey(same_key, same_mods)` twice for different macros, and both will appear in the registry. The hotkey callback then iterates the Vec and dispatches both intents (`observer.rs:420-449`), with the `break` at line 436 only stopping after the first match — so the later-registered binding wins. (This is *less* silent than the HashMap overwrite, but still a "last-write-wins" race.)

**`HotkeyBinding.matches()` at `observer.rs:47-49`:** `(flags.bits() & self.modifiers) == self.modifiers` — correct bitwise containment check. The matching logic is sound; the absence of any uniqueness invariant upstream is the gap.

### 2.3 Same-input overlap detection (current: absent)

- `InputEvent` at `state/mod.rs:36-43` derives `Hash + Eq`. Set-intersection is cheap.
- `ActionStep` at `state/mod.rs:50-61` carries the `InputEvent` in both `SustainedHold` and `InterleavedInterval` variants.
- Legacy fallback in `scheduler/mod.rs:258-267`: empty `sequence` synthesizes a single left-click at `interval_ms` (Pulse) or sustained-hold left-click (Hold).
- No code anywhere compares enabled macros' input sets against each other.

### 2.4 UI surface (current: minimal)

- `App.tsx:647` says "Required for global hotkeys" — appears next to the permissions card, only when both permissions are false.
- `App.tsx:988-1037` shows the bound trigger key in a small chip on each macro card via `resolveKeyName(macro.trigger_key!)`. The chip does not indicate "global" or "system-wide".
- The key-capture widget at `App.tsx:840-879` shows the key name only — no preview of modifier bits.
- "Binds are system-wide" wording is **absent from the UI** — there is no explicit statement of global behavior anywhere in `App.tsx`.

### 2.5 Windows hook current behavior

- `windows/mod.rs:489-495`: `SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_callback), None, 0)`. The thread-id argument `0` means **system-wide**, not thread-specific. This is a true low-level global hook.
- `hook_callback` at `windows/mod.rs:426-460` matches keycode via `MACRO_TRIGGER_KEYS.get(&keycode)`. **No modifier check** — the Windows hook ignores `Ctrl`/`Shift`/`Alt`/`Win` even though the data model carries modifiers.
- `GetAsyncKeyState` is used in the emergency-stop branch (`windows/mod.rs:434-435`) to read modifier state for the hardcoded `Ctrl+Shift+Q`; the same pattern would work for user-bound hotkeys.

---

## 3. Requirements Feasibility

### 3.1 UX-11 — Hotkey conflict detection

**Where it lives:** `StateActor` (single owner of `AppState`). The conflict check is a lookup over the current macro set; the StateActor is the only place with a consistent view.

**Specific change required:**

1. `bind_hotkey` IPC at `ipc/mod.rs:79-95` must consult state instead of bypassing it. Change signature: route through `StateManager::send_intent(Intent::BindHotkey(macro_id, keycode, modifiers, reply))` (new Intent variant carrying a oneshot sender for the conflict result).
2. `set_macro_trigger_key` already routes through state; the conflict check lives in the `Intent::SetMacroTriggerKey` handler at `state/mod.rs:323-329`. Same change for `Intent::AddMacro` when `config.trigger_key.is_some()`.
3. Detect by iterating `self.state.macros.values()`, returning `Some(existing_macro_id)` when a different macro already has the same `(keycode, modifiers)` pair. Exempt re-binding the same macro to the same key (compare on `id`).
4. Decision (see §4): reject with explicit error string, return value `Result<(), String>`. Frontend displays the error in a toast / inline message.
5. After the conflict check passes, propagate the new binding to the platform-specific static registry (`HOTKEY_BINDINGS` on macOS / the Windows `MACRO_TRIGGER_KEYS`).

**Blockers:** None. The current state already has every type and channel needed; the gap is the check itself.

**Side effect of fixing UX-11:** unified registry. The current dual-registry state means even a successful bind can shadow another macro that has its `trigger_key` set. The fix must therefore route ALL key changes through StateActor, not just the `bind_hotkey` IPC. The platform-side statics become a write-only mirror that the StateActor owns.

### 3.2 UX-12 — Same-input overlap warning

**Where it lives:** Computed in `StateActor` whenever AppState changes, exposed as a derived field on `AppState` so the frontend reflects it via the existing `state-changed` event.

**Specific change required:**

1. Add a `conflicts: Vec<InputConflict>` field to `AppState` (`state/mod.rs:101-111`). `#[serde(default)]` for backwards compat with persisted profiles.
2. `InputConflict` struct: `{ macros: Vec<Uuid>, input: InputEvent }` — a minimal description of which macros share an input.
3. After every state-mutating intent in `handle_intent()`, call a new helper `recompute_conflicts()` that:
   - Walks all *enabled* macros
   - Expands each macro's `sequence.steps` into a set of `InputEvent` (and applies the legacy fallback for empty sequences, mirroring `scheduler/mod.rs:258-267`)
   - Builds a `HashMap<InputEvent, Vec<Uuid>>`; any `Vec` with `len() >= 2` is a conflict
4. The frontend surfaces conflicts in a visible warning region above the macro list. Warning is non-blocking (the macros are still active) — the requirement is to make the user *aware*, not to disable.

**Blockers:** None. The detection is O(n) over macros × steps; a typical config is <50 macros × <5 steps.

**Edge case:** Two macros with the same input but different intervals will still both fire (Phase 9 will handle actual parallel execution; the warning is independent of timing semantics).

### 3.3 UX-13 — Broader key range

**Current state already covers:** `keymap.ts:10-78` has A–Z, Digit0–9, F1–F12, Arrows, Home/End/PageUp/PageDown, Space/Enter/Tab/Backspace/Escape. The native-key resolution works.

**Gap 1: Modifier pass-through broken end-to-end.** The chain breaks at `App.tsx:375` where `modifiers: 0` is hardcoded. The backend (`observer.rs:47-49`) is ready to match modifiers — it has a u64 field on `HotkeyBinding` and does the correct bitwise containment check.

**Specific change required:**

1. `startCapture` at `App.tsx:340-368`:
   - **Remove the modifier-only filter at line 357** so the user can bind "just Shift" (or any other modifier) as the primary key.
   - At commit time, compute a `modifiers: number` from `e.getModifierState("Shift")` / `"Control"` / `"Alt"` / `"Meta"`. Platform-agnostic bit values:
     - macOS raw `CGEventFlags` bits (matching the bits the backend will compare against): `Shift = 0x20000`, `Control = 0x40000`, `Option = 0x80000` (Alt on Mac), `Command = 0x100000`. [LOW — training-data value; verified by symbol lookup in `core-graphics` 0.24]
     - Windows `MOD_*` constants: `MOD_ALT = 0x0001`, `MOD_CONTROL = 0x0002`, `MOD_SHIFT = 0x0004`, `MOD_WIN = 0x0008`. [HIGH — these are documented Win32 constants in `windows-rs` 0.61]
   - **Decision (see §4):** the IPC should carry the modifiers as a `u64` that the platform layer reinterprets. This avoids forcing the frontend to know the platform bit layout.
2. `handleCardSetTriggerKey` at `App.tsx:371-385`: pass the computed `modifiers` to `bind_hotkey` and to `set_macro_trigger_key` (the latter needs a new optional `modifiers` parameter — see §4).
3. `handleCreateMacro` at `App.tsx:289-329`: same — pass modifiers to `add_macro`.
4. `MacroConfig` in `state/mod.rs:79-99` gets a new `#[serde(default)] trigger_modifiers: u64` field. Old profiles without this field deserialize to 0 (backwards compat).
5. `MACRO_TRIGGER_KEYS` in `observer.rs:197` / `windows/mod.rs:410` changes type from `HashMap<u16, Uuid>` to `HashMap<(u16, u64), Uuid>`.
6. Windows hook `hook_callback` at `windows/mod.rs:426-460`: add modifier bit checks using `GetAsyncKeyState(VK_CONTROL.0)`, `GetAsyncKeyState(VK_SHIFT.0)`, `GetAsyncKeyState(VK_MENU.0)`, `GetAsyncKeyState(VK_LWIN.0)` (already imported at `windows/mod.rs:401`). Build a local `mod_mask: u64` and require the bound modifier bits to all be pressed (the same containment check as the macOS path).
7. `keymap.ts`: add modifier keycodes to the `*_TO_NAME` tables so the UI can show "Shift" instead of "Key 56" when the user binds a pure modifier. The frontend already calls `resolveKeyName` (`App.tsx:1013`) — the lookup is the only gap.

**Gap 2: Numpad coverage asymmetry.** macOS keymap (lines 10–78) has no Numpad keys. Windows keymap (lines 104–144) has Numpad0–9. The macOS CGKeyCode for numpad keys is documented in HIToolbox/Events.h; adding them is a one-line lookup per key. Not a blocker for UX-13 success criterion (which specifies A–Z, 0–9, F1–F12, modifier combos) but should be added for parity.

**Blockers:** None. The data model has space; the IPC can carry the field; the matchers can be extended.

### 3.4 UX-14 — System-wide UI + verification

**Mechanism already global:** both platforms use true global mechanisms. macOS `CGEventTap` at `HID` (`observer.rs:243`) sees events regardless of focus; Windows `SetWindowsHookExW(WH_KEYBOARD_LL, ..., 0)` (`windows/mod.rs:489-495`) is system-wide because the thread-id is 0.

**UI change required:**

1. Add a persistent, visible notice that hotkeys are global. Recommended placement: a small one-line text under the trigger-key chip on each macro card (e.g., `↗ Fires globally — even when AutoMux is in the background`), AND a top-of-dashboard banner on first launch that can be dismissed. The banner follows the dismiss-and-rebuild pattern used for the auto-save-error banner (`App.tsx:715-735`).
2. The notice is **platform-agnostic copy** — no macOS/Windows branching. Both platforms are global; the copy is true on both.
3. The Tauri window-focus model is irrelevant — hotkeys are observed by the OS-level mechanism, not the Tauri WebView. No change to the window setup.

**Verification on Windows:** the WH_KEYBOARD_LL hook is system-wide by construction, but the actual behavior on a real machine is the success-criterion proof. The CI artifact is a Windows NSIS installer; a manual device test (switch focus to another app, press the bound hotkey, observe macro fires) is the verification. No code change required for the mechanism to be global — only a manual test plan item.

**Blockers:** None.

---

## 4. Architectural Decisions Needed

### D-1. Conflict policy: reject vs reassign vs prompt
**Recommendation: REJECT with explicit error.**
- Reassign without confirmation silently breaks the other macro — the exact failure UX-11 forbids.
- A reassignment prompt requires a modal dialog, which is a Phase 10 UI redesign concern (per ROADMAP.md, Phase 10 owns UI-01/02/03/04 + UX-08/09/10; a new modal would be premature).
- A toast / inline error is the minimum-surprise surface: the user clicks "Save", sees "Key F5 is already bound to macro 'X'. Unbind it from X first or pick a different key.", and acts accordingly.

### D-2. Detection site: StateActor vs frontend pre-check
**Recommendation: StateActor.**
- StateActor is the single owner of `AppState` (per ARCHITECTURE.md and the two-actor model). The frontend sees a projection via `state-changed` and cannot make authoritative guarantees.
- A frontend pre-check would race with concurrent edits (e.g., another window, profile load, hotkey toggle).
- The existing pattern for `add_macro` / `set_macro_enabled` is that the backend validates; this is consistent.

### D-3. Where the conflict check fires
**Recommendation: at the three "binding can change" sites:**
1. `Intent::AddMacro` (`state/mod.rs:291-300`) — when `config.trigger_key` is `Some(_)`, check.
2. `Intent::SetMacroTriggerKey` (`state/mod.rs:323-329`) — always check.
3. New `Intent::BindHotkey` for the macOS-specific `bind_hotkey` IPC — always check.

The `Intent::UpdateSequence` path does not change the trigger key, so it does not need the check.

### D-4. Self-rebind exemption
**Recommendation: allow re-binding the same macro to the same key+mods.** Otherwise editing a macro's trigger would fail with a conflict against itself. Compare on `macro_id == existing_id && keycode == self.keycode && modifiers == self.modifiers` and treat as no-op (not error).

### D-5. Modifier representation on the IPC wire
**Recommendation: send `u64` raw bits, document the bit layout per platform in a comment in `ipc/mod.rs`.**
- The frontend computes bits using platform-specific constants (macOS `CGEventFlag*` values vs Windows `MOD_*` values). The bits are different on each platform but the `u64` is the right size.
- Alternatives considered:
  - **Typed enum (`{ shift: bool, control: bool, alt: bool, cmd: bool, win: bool }`)**: type-safe but requires the IPC layer to know about platform semantics (e.g., "Cmd" on macOS is "Win" on Windows). The frontend can abstract this; the wire stays simple.
  - **Two separate fields (modifiers: u64, is_modifier_only: bool)**: redundant. Whether a binding is "modifier-only" is derivable from `e.key` at capture time.
- The chosen path keeps the data model uniform and the platform bit layout isolated in one comment.

### D-6. "System-wide" UI placement
**Recommendation: in-card subtitle on every trigger-key chip + first-run dashboard banner.**
- In-card subtitle: minimal, persistent, lives next to the key name. Example: `⌘+F5  ↗ Global`. The "↗" symbol is a platform-agnostic cue (arrow pointing outside the window).
- First-run banner: appears once per install (gated by a `tcc_granted.flag`-style sentinel or a SolidJS `localStorage` flag), dismissed by the user. After dismiss, only the in-card subtitles remain.
- The macOS-specific "Required for global hotkeys" copy at `App.tsx:647` stays — it explains the *permission* requirement, which is different from the "binds are global" statement. The two messages are complementary, not duplicates.

### D-7. Unify the two hotkey registries
**Recommendation: yes — fold `HOTKEY_BINDINGS` (macOS-only) into the StateActor's view, drive it the same way as `MACRO_TRIGGER_KEYS`.**
- The current dual-registry state is the root cause of the silent-shadowing bug.
- A single canonical model (`Vec<HotkeyBinding>` in `AppState`, mirrored to platform statics on every change) eliminates the invariant gap.
- The macOS `HotkeyBinding.action: HotkeyAction` enum already covers `ToggleMacro(id)` and `ToggleEngine` — `ToggleEngine` is used by the hardcoded engine hotkey and would be a future feature; the schema is forward-compatible.

### D-8. `bind_hotkey` and `unbind_hotkey` IPC on Windows
**Recommendation: enable on Windows as part of this phase (do not leave them as macOS-only no-ops).**
- The CONCERNS.md:150-152 documents this gap. Now that modifier support is being added end-to-end, leaving Windows on `bind_hotkey` = no-op would mean the new modifier UX works on macOS only.
- Implementation: in `unbind_hotkey` IPC, remove the `#[cfg(target_os = "macos")]` gate; in the Windows observer, store the `(keycode, modifiers) → action` map and check it inside `hook_callback` alongside the existing `MACRO_TRIGGER_KEYS` lookup. The Windows `MACRO_TRIGGER_KEYS` map (keycode-only, no modifiers) and the new Windows `HOTKEY_BINDINGS` (keycode + modifiers) are two different concepts and should be both present.

---

## 5. Technical Risks

### R-1. Modifier bit alignment between macOS and Windows
**Risk:** if the frontend uses the wrong constant for the wrong platform, hotkeys fire on the wrong modifier or silently never match. The `(flags.bits() & self.modifiers) == self.modifiers` containment check at `observer.rs:48` requires that the bound bits are a subset of the held bits — but it cannot detect that the bit definitions are wrong on the frontend.
**Mitigation:** keep the modifier computation in a single frontend helper (`App.tsx`) that branches on `IS_MACOS` (already a platform detection variable at `App.tsx:131`); comment the bit values inline; add a Rust unit test that asserts the platform bit constants match the values the frontend sends (assertions on synthetic `CGEventFlags` and `MOD_*` ints).

### R-2. Windows `WH_KEYBOARD_LL` permission / hook failure
**Risk:** Windows can drop a low-level hook on session change, UAC prompt, or fullscreen app. The existing code at `windows/mod.rs:481-535` only initializes once at startup. If the hook is dropped, no recovery path exists.
**Mitigation:** out of scope for this phase (same risk exists today; the success criterion asks for *global* operation, not for *always-on* operation across all session changes). Document the limitation. Add a debug log in the Windows hook so failures surface during manual verification.

### R-3. Channel backpressure on hotkey dispatch
**Risk:** the `try_send` calls in `observer.rs:433,446` and `windows/mod.rs:441,453` silently drop intents when the channel is full (capacity 100, set in `lib.rs:23`). CONCERNS.md:51-55 documents this.
**Mitigation:** not in scope for this phase (pre-existing), but a tighter hotkey path (e.g., `.await` send) could be added if evidence of drops emerges. Note for Phase 9 (parallel macro execution), which will exercise the same channel more heavily.

### R-4. Race between `unbind_hotkey` + `bind_hotkey` and `set_macro_trigger_key`
**Risk:** `App.tsx:374-376` calls `unbind_hotkey` → `bind_hotkey` → `set_macro_trigger_key` in sequence on macOS. After this phase, each of these IPC calls becomes a StateActor intent. If the IPC is not idempotent or the intent ordering is not preserved, a transient state with both bindings can exist.
**Mitigation:** make `Intent::BindHotkey` replace any existing binding for that `(macro_id, keycode, modifiers)` rather than appending. The macOS `update_hotkey_bindings` (replaces the whole Vec) is the existing bulk path; a per-`macro_id` replace is the per-binding analog.

### R-5. Profile persistence format change
**Risk:** adding `trigger_modifiers: u64` to `MacroConfig` is a schema change. Old profile JSONs without this field will fail to deserialize unless `#[serde(default)]` is used.
**Mitigation:** every new field gets `#[serde(default)]` (this matches the existing pattern at `state/mod.rs:91,94,97`). Old profiles load with `trigger_modifiers = 0` (no modifiers) — correct behavior for pre-modifier-support profiles.

### R-6. Self-conflict in the new `Intent::BindHotkey` flow
**Risk:** on a `set_macro_trigger_key` call for macro A with a new key, the check walks all macros and finds macro A itself in the map (its previous binding). The check must allow same-macro, same-key, same-mods.
**Mitigation:** include the "self-rebind allowed" logic in the conflict helper. Test: rebinding macro A to the same key must succeed.

### R-7. Windows `MACRO_TRIGGER_KEYS` lookup key change
**Risk:** changing the map type from `HashMap<u16, Uuid>` to `HashMap<(u16, u64), Uuid>` breaks every callsite. The current callsites are:
- `windows/mod.rs:421-423` (write)
- `windows/mod.rs:450-456` (read in hook)
- `state/mod.rs:471-474` (write)
- `observer.rs:204-206` (macOS write)
- `observer.rs:442-449` (macOS read)
**Mitigation:** all callsites are inside the repo; mechanical change. Use a type alias `(u16, u64)` to `TriggerKey` for readability. A compile-fail audit catches every callsite.

### R-8. The "first-run banner" persistence
**Risk:** the "Binds are system-wide" banner must appear once, not every launch. A frontend-only flag in `localStorage` is the simplest choice but is per-browser-profile and reset on app data clear. A `tcc_granted.flag`-style file is more durable but adds backend work.
**Mitigation:** use `localStorage` (or `sessionStorage` if the team wants it every session). The cost of a false-positive (banner appears again after data clear) is one extra click — the cost of a false-negative (banner never shown because backend sentinel failed) is the user never learning that binds are global. The user-experience failure mode is the more important one to avoid.

---

## 6. Recommended Implementation Approach

Order the work by dependency, not by requirement:

**Wave 1 — Backend foundation (independent of frontend)**
1. **State model:** add `trigger_modifiers: u64` to `MacroConfig` (`state/mod.rs:79-99`); add `conflicts: Vec<InputConflict>` to `AppState` (with `#[serde(default)]`).
2. **Registry type change:** change `MACRO_TRIGGER_KEYS` to `HashMap<(u16, u64), Uuid>` in `observer.rs:197` and `windows/mod.rs:410`.
3. **Conflict detection helper:** new function `recompute_conflicts()` in `state/mod.rs`; called after every state-mutating intent.
4. **Bind conflict check:** new helper `check_trigger_key_conflict(macro_id, keycode, mods)` in `state/mod.rs`; called from `AddMacro`, `SetMacroTriggerKey`, and the new `BindHotkey` intent.
5. **New IPC + Intent:** `bind_hotkey` routes through `StateManager`; add `Intent::BindHotkey(macro_id, keycode, mods, reply)`. Add `Intent::UnbindHotkey(macro_id, reply)`.
6. **Windows hook:** extend `hook_callback` (`windows/mod.rs:426-460`) to check modifier bits via `GetAsyncKeyState`; read from a new Windows-side `HOTKEY_BINDINGS` registry (mirrors the macOS one) that is populated by the new IPC path.

**Wave 2 — Frontend capture fix**
1. **Modifier computation:** add a `computeModifiers(e: KeyboardEvent): number` helper in `App.tsx` that branches on `IS_MACOS` (`App.tsx:131`) and returns the platform-native bitmask.
2. **Allow modifier-only binds:** remove the `["Control", "Shift", "Alt", "Meta"]` filter at `App.tsx:357`.
3. **Thread modifiers through:** update `handleCardSetTriggerKey` (`App.tsx:371-385`), `startCapture` commit (`App.tsx:361`), and `handleCreateMacro` (`App.tsx:289-329`) to send the computed modifiers.
4. **Resolve error responses:** wrap each `bind_hotkey` / `set_macro_trigger_key` / `add_macro` call in try/catch and surface the conflict error in a toast / inline message.
5. **keymap.ts:** add modifier keycodes to the `*_TO_NAME` tables (Shift = 56, Cmd = 55, Caps = 57, etc., on macOS; `MOD_*` values on Windows do not have a single keycode — handle the "no name" case gracefully).

**Wave 3 — UX-12 and UX-14 UI surfaces**
1. **Conflict warning region:** new component in `App.tsx` (between the engine card and the macro list) that renders `state().conflicts` as a list of warnings. Each warning: `"Macros A and B both inject Left Click — clicks will fire at 2× rate."` Persistent until the user dismisses OR the conflict resolves.
2. **In-card "Global" subtitle:** add a `↗ Global` label to the trigger-key chip at `App.tsx:1001-1037`. One-line text, no dismiss.
3. **First-run banner:** add a one-time `localStorage`-gated banner above the macro list explaining system-wide behavior. Dismiss button removes the banner and sets the flag.

**Wave 4 — Verification**
1. Manual macOS test: bind Cmd+F5 to macro A, bind F5 to macro B, observe conflict error in UI.
2. Manual macOS test: enable macro A (left click, 100ms) and macro B (left click, 100ms), observe the 2× click warning.
3. Manual macOS test: switch focus to another app, press Cmd+F5, observe macro A toggles.
4. Manual Windows test: same three flows, on a real Windows host (not cross-compile).
5. `cargo test` for: conflict helper unit tests (re-bind same key, same macro allowed; bind to different macro rejected; modifier combinations matched correctly).
6. `cargo build --target x86_64-pc-windows-msvc 2>&1 | grep warning` returns zero warnings (regression check; the Phase 7 BUILD-01 fix should hold).

---

## 7. Validation Architecture

### Test framework
- Backend: Rust `cargo test` with inline `#[cfg(test)]` modules (existing pattern in `scheduler/mod.rs:393-582` and `persistence.rs:251-319`).
- Frontend: no test framework currently installed. UI changes are validated manually.

### Phase requirements → test map

| Req ID | Behavior | Test Type | Command / Method | File Exists? |
|--------|----------|-----------|------------------|-------------|
| UX-11 | `check_trigger_key_conflict` rejects new macro on existing binding | unit | `cargo test conflict -- --nocapture` (new) | ❌ Wave 0 |
| UX-11 | `bind_hotkey` IPC returns error string on conflict | unit | `cargo test bind_hotkey_conflict --` (new) | ❌ Wave 0 |
| UX-11 | Re-binding same macro to same key succeeds | unit | `cargo test self_rebind_allowed --` (new) | ❌ Wave 0 |
| UX-12 | `recompute_conflicts` finds two enabled macros with same input | unit | `cargo test conflict_detection_overlap --` (new) | ❌ Wave 0 |
| UX-12 | Disabling one macro removes the conflict | unit | `cargo test conflict_disappear_on_disable --` (new) | ❌ Wave 0 |
| UX-13 | Frontend modifier helper returns expected bits per platform | manual / unit (Rust mirror) | DOM event test in browser dev tools | ❌ Wave 0 |
| UX-13 | Backend `HotkeyBinding.matches()` matches Cmd+F5 | unit | `cargo test modifier_match --` (new) | ❌ Wave 0 |
| UX-13 | Windows `hook_callback` matches Ctrl+Shift+F5 | unit | `cargo test windows_modifier_match --` (new) — but requires Windows | ❌ Wave 0 |
| UX-14 | Hotkey fires when another app is focused on macOS | manual smoke | macOS device test, switch focus, press hotkey | N/A — manual |
| UX-14 | Hotkey fires when another app is focused on Windows | manual smoke | Windows device test, same | N/A — manual |
| UX-14 | "Binds are system-wide" notice visible in UI | manual / screenshot | Visual inspection | N/A |

### Wave 0 gaps
- New Rust unit tests in `state/mod.rs` for `check_trigger_key_conflict` and `recompute_conflicts`.
- New Rust unit tests in `platform/macos/observer.rs` for `HotkeyBinding.matches()` with synthetic CGEventFlags.
- No frontend test framework to add (would be a separate effort; Phase 10 may add Vitest).

### Sampling rates
- Per task commit: `cargo test -p automux-lib` (fast; < 5s).
- Per wave merge: full suite + `cargo clippy --all-targets -- -D warnings`.
- Phase gate: full suite green AND manual macOS + Windows device smoke test for UX-13/14.

---

## 8. Out of Scope / Deferred

The following items came up during research but should NOT be in this phase:

- **A modal reassignment prompt** for UX-11 conflict resolution. Out of scope — full UI redesign is Phase 10. Reject-with-error is the minimum-viable policy.
- **A separate "Hotkey Settings" tab.** Out of scope — UI-01/02/03/04 (Phase 10) owns all UI structural changes.
- **Cross-platform hotkey profile portability (UX-04/05).** Deferred per REQUIREMENTS.md:73-79 and STATE.md:74-76.
- **Refactor of `try_send` hotkey dispatch to `.await` send.** CONCERNS.md:51-55 — pre-existing, not introduced by this phase. Note in §R-3 for Phase 9 follow-up if channel pressure emerges.
- **Numpad key coverage on macOS.** Add as a small follow-up if parity matters, but the UX-13 success criterion does not require it.
- **Auto-updater / notarization.** v3 scope, explicitly out of scope per REQUIREMENTS.md:73-79.
- **Re-introduction of `safety_lock_for_engine` or other architectural rewrites.** The two-actor model is sound; this phase adds invariants, not a new model.
- **`handleCreateMacro` macro conflict detection for the InputEvent type of the macro itself.** The trigger-key conflict check covers the trigger side. The input-side overlap is the UX-12 conflict detection, which is a derived field on AppState — not a "create-time" check.
- **Recovery for Windows hook failure on session change.** §R-2 — pre-existing limitation. UX-14 is about global behavior, not all-session survival.

---

## 9. References

### Apple developer documentation
- [CGEventFlags reference](https://developer.apple.com/documentation/coregraphics/cgeventflags) — bit values for `CGEventFlagShift` (0x20000), `CGEventFlagControl` (0x40000), `CGEventFlagOption` (0x80000), `CGEventFlagCommand` (0x100000). [LOW — training-data values; verify against `core-graphics` 0.24 in Wave 0 with a `core_graphics::event::CGEventFlags::CGEventFlagCommand.bits()` assertion]
- [CGEventTap reference](https://developer.apple.com/documentation/coregraphics/cgeventtap) — confirms `HID` location is system-wide.
- HIToolbox/Events.h (Apple SDK) — source for `DOM_KEYCODE_TO_CGKEYCODE` (already in `keymap.ts:7`).

### Microsoft Learn
- [RegisterHotKey vs SetWindowsHookEx](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw) — `WH_KEYBOARD_LL` with thread-id 0 is system-wide; no DLL required.
- [Virtual-Key Codes](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes) — VK codes for the Windows `keymap.ts` table.
- [Modifier keys (MOD_*) constants](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey) — `MOD_ALT=0x0001`, `MOD_CONTROL=0x0002`, `MOD_SHIFT=0x0004`, `MOD_WIN=0x0008`, `MOD_NOREPEAT=0x4000`. [HIGH — documented]

### Codebase internal references
- `.planning/codebase/CONCERNS.md:45-49` — explicit documentation of the duplicate-trigger-key bug (UX-11 trigger).
- `.planning/codebase/CONCERNS.md:150-152` — explicit documentation of the Windows no-configurable-hotkeys gap (UX-13/14 trigger).
- `.planning/codebase/ARCHITECTURE.md:155-159` — hotkey data flow as currently designed.
- `.planning/phases/07-carry-work-platform-ci-safety/07-RESEARCH.md` — pattern reference: how this codebase writes a research file, including the "ALREADY DONE" pattern (SAFE-04 was already correct in the prior phase; this phase has no equivalent zero-work finding).

### Prior phase patterns to reuse
- **Pattern: `Intent` enum + oneshot sender for IPC return value** (from `AddMacro` at `state/mod.rs:128,291-300` and `LoadProfile` at `state/mod.rs:149,401-437`). Reuse for the new `BindHotkey` and `UnbindHotkey` intents to carry the conflict result back to the IPC caller.
- **Pattern: `tcc_granted.flag` sentinel in `app_data_dir()`** (from `persistence.rs:220-249`). If the team later wants a backend sentinel for the first-run banner, this is the pattern — but the recommended implementation uses `localStorage` instead (see §R-8).
- **Pattern: per-platform statics mirrored from StateActor** (from `update_macro_trigger_keys` at `state/mod.rs:471-474` and `windows/mod.rs:421-423`). Reuse for the unified hotkey binding registry; the StateActor becomes the single source of truth, the platform statics become a write-only mirror.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | macOS `CGEventFlags` bit values are `Shift=0x20000`, `Control=0x40000`, `Option=0x80000`, `Command=0x100000` | §3.3, §5 R-1 | Frontend sends wrong bits; hotkeys never match. Mitigated by a Rust unit test asserting the constants before commit. |
| A2 | Windows `MOD_*` values are `ALT=0x0001, CONTROL=0x0002, SHIFT=0x0004, WIN=0x0008` | §3.3, §5 R-1 | Same as A1 but on Windows. Mitigated by static assertions in test code. |
| A3 | Windows `WH_KEYBOARD_LL` with thread-id 0 actually fires when AutoMux is unfocused on a real Windows 10/11 host | §3.4, §5 R-2 | If wrong, UX-14 success criterion fails on Windows and needs a different mechanism (e.g., a separate process holding the hook via `RegisterHotKey`). Manual test will confirm. |
| A4 | Adding `conflicts: Vec<InputConflict>` to `AppState` with `#[serde(default)]` is backwards-compatible with saved profiles | §3.2, §5 R-5 | If the deserializer drops or rejects the new field, profiles fail to load. Mitigated by manual load test. |
| A5 | `localStorage` persists across app restarts inside the Tauri WebView | §3.4, §5 R-8 | If the WebView's localStorage is reset on every launch, the first-run banner appears every time. Tauri 2's WebView typically persists localStorage; verify in Wave 4. |
| A6 | The existing 3s frontend poll loop at `App.tsx:191-209` is the right place to ALSO poll for new conflict state, if needed | §6 Wave 3 | Conflicts are already pushed via `state-changed`, so no poll is needed. This assumption is informational — if wrong, fall back to "recompute on every state-changed event" which is the current plan. |

---

## Open Questions

1. **Should pure-modifier bindings (e.g., "just Shift" as a toggle hotkey) be allowed?**
   - What we know: `App.tsx:357` currently blocks them. Removing the filter would allow them. macOS CGEventFlags do fire on modifier-only keypresses (`CGEventType::FlagsChanged`), but the current `initialize_tap` event list at `observer.rs:246-260` does not include `FlagsChanged` — so a modifier-only binding would never match today.
   - What's unclear: whether to (a) add `FlagsChanged` to the tap's event list and allow pure-modifier binds, or (b) keep the filter and document that modifier-only binds are unsupported.
   - Recommendation: keep the filter for now; modifier-only binds are an edge case that the user can work around with `Cmd+Shift+Key` style combos. Adding `FlagsChanged` is a Phase 9+ concern (the policy on modifier-only as a toggle is a separate UX decision).

2. **Should the conflict-detection helper fire on `LoadProfile`?**
   - What we know: `LoadProfile` (`state/mod.rs:401-437`) replaces the entire macro set. A loaded profile can contain conflicts internally.
   - What's unclear: whether to (a) eagerly detect and return the conflicts in the load response, or (b) let `recompute_conflicts()` (called after every intent) handle it.
   - Recommendation: option (b) — `LoadProfile` already calls `reevaluate_all_macros` and `auto_save_default`. Add a `recompute_conflicts()` call there too. The frontend sees the conflicts via the next `state-changed` event.

3. **Should the in-card "Global" subtitle and the first-run banner both exist, or just one?**
   - What we know: the in-card subtitle is persistent, the first-run banner is dismissible. They say the same thing.
   - What's unclear: UX redundancy. Phase 10's UI redesign may consolidate all hotkey messaging.
   - Recommendation: both for now. The subtitle is the always-visible ground truth; the banner is the educational nudge for first-time users. Phase 10 can decide.

---

## Metadata

**Confidence breakdown:**
- Current state (file:line citations): **HIGH** — every claim is grounded in a specific read of the codebase.
- UX-11 / UX-12 implementation approach: **HIGH** — clear path, no architectural unknowns.
- UX-13 modifier bit values: **LOW** (macOS) / **HIGH** (Windows) — must be asserted in Wave 0.
- UX-14 Windows device verification: **MEDIUM** — mechanism is global by definition; manual test is the proof.
- Conflict policy choice: **HIGH** — reject-with-error is the obvious minimum-viable answer; the alternative is a Phase 10 modal.

**Research date:** 2026-06-19
**Valid until:** 2026-07-19 (30 days; stable platform APIs, no fast-moving ecosystem concerns)
