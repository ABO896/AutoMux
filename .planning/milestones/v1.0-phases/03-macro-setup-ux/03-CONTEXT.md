# Phase 3: Macro Setup UX - Context

**Gathered:** 2026-05-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace two opaque input fields in the macro creation form — a raw key-code number input and a raw process-name text field — with user-friendly widgets: a click-to-capture key widget and a running-process picker dropdown. Extend both widgets to support inline editing on existing macro cards (not creation form only).

**Requirements in scope:** UX-01, UX-02, UX-03

**Not in scope:** Key name storage in the Rust model / profile migration (v2 concern), cross-platform profile portability (DIST-02/UX-04), Windows hotkey modifier support (tracked in CONCERNS.md as a separate gap). Do not expand scope.

</domain>

<decisions>
## Implementation Decisions

### Key Capture Widget (UX-01)

- **D-01:** **Click-to-capture interaction model.** The trigger key field shows "Click to set key…" by default. Clicking the field enters a recording state that intercepts the next `keydown` event; the captured key name is then displayed and the integer keycode stored in the model. A Clear/Cancel affordance exits recording without committing. Raw CGKeyCode / VK integers are never surfaced in the UI.
- **D-02:** **Human-readable name only.** After capture, the field displays the key name only (e.g., "Q", "Space", "F5", "Left Shift"). No raw integer shown alongside the name.
- **D-03:** **Frontend lookup table for key name resolution.** A static `Map<number, string>` in the TypeScript layer maps CGKeyCode values (macOS) and VK codes (Windows) to human-readable names. No new Rust IPC command needed. The researcher should build or find a complete mapping table for both platforms.

### Process Picker (UX-02, UX-03)

- **D-04:** **Inline `<select>` dropdown.** The text input for target app is replaced by a `<select>` element populated via a new Rust IPC command (`list_running_apps`). No modal or custom popover needed. Consistent with the existing `<select>` elements already in the creation form.
- **D-05:** **Display name + identifier per option.** Each option shows: macOS — "Minecraft (com.mojang.minecraft)"; Windows — "Minecraft (C:\...\Minecraft.exe)". The stored `target_app` value remains the bundle ID (macOS) or process path (Windows) — same format the backend already understands.
- **D-06:** **Fetch on open, every time.** `list_running_apps` is called each time the dropdown is opened or focused, ensuring the list is always current. No manual refresh button needed.

### Edit Scope

- **D-07:** **Creation form AND inline card editing.** Phase 3 improves both the "New Macro" creation form and adds click-to-edit for trigger key and target app on existing macro cards. Macro cards are currently read-only — Phase 3 introduces minimal edit affordances for these two fields only.
- **D-08:** **Click-to-edit with auto-commit.** On a macro card:
  - Clicking the "Key X" trigger key badge enters capture mode for that macro; a new keypress auto-commits via `bind_hotkey` IPC. No save button.
  - Clicking the target app badge opens the process picker dropdown for that macro; selecting an app auto-commits via `set_macro_target_app` IPC. No save button.
  - An escape/cancel gesture exits capture/picker mode without committing.

### Claude's Discretion

- Exact visual treatment of the "recording" state in the key capture field (e.g., pulsing border, placeholder text color change).
- How to handle keys that have no entry in the lookup table — display the raw integer as a fallback or show "Unknown (N)".
- Whether to show a "Global" sentinel as the first `<option>` in the process picker with an empty/null value, or as a separate "Clear target" affordance.
- Exact IPC command signature for `list_running_apps` (return type, platform-conditional compilation).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — UX-01, UX-02, UX-03 are Phase 3 scope; v2 requirements UX-04/UX-05 are explicitly deferred
- `.planning/ROADMAP.md` — Phase 3 success criteria (3 acceptance tests that must pass)

### Known Issue Locations (Frontend)
- `src/App.tsx` lines 493–501 — current target app `<input type="text">` (replace with `<select>`)
- `src/App.tsx` lines 515–523 — current trigger key `<input type="number">` (replace with click-to-capture field)
- `src/App.tsx` lines 585–602 — macro card "Target & Trigger" display row (add click-to-edit affordances here)
- `src/App.tsx` lines 91, 176–193 — `newMacroInput` signal and `handleCreateMacro` (key type dropdown + trigger key parsing to update)

### Known Issue Locations (Rust IPC)
- `src-tauri/src/ipc/mod.rs` lines 64–96 — `get_active_app`, `bind_hotkey`, `unbind_hotkey` (bind_hotkey is macOS-only; `list_running_apps` is a new command needed)
- `src-tauri/src/ipc/mod.rs` lines 42–49 — `set_macro_target_app` (existing command, no change needed — picker just calls it)
- `src-tauri/src/platform/macos/observer.rs` line 26 — `NSWorkspace` already imported; `sharedWorkspace().runningApplications()` is the enumeration API for UX-02
- `src-tauri/src/state/mod.rs` line 40 — comment documents CGKeyCode value examples

### Codebase Analysis
- `.planning/codebase/CONCERNS.md` — documents "UI keyboard key input not exposed" (lines 19–23) and "No configurable hotkeys on Windows" (lines 150–152); read before planning to understand Windows trigger key gap
- `.planning/codebase/ARCHITECTURE.md` — IPC layer responsibilities; how to add a new `#[command]`
- `.planning/codebase/CONVENTIONS.md` — SolidJS patterns (createSignal, Show, For, createEffect + onCleanup); Rust naming; platform-conditional compilation guards

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `<select>` pattern — already used twice in the creation form (input type selector lines 472–482, trigger mode selector lines 505–514); the process picker `<select>` should match this visual style exactly
- `NSWorkspace::sharedWorkspace()` — already imported and used in `platform/macos/observer.rs`; `runningApplications()` on the shared workspace returns the running apps list needed for UX-02
- `invoke()` pattern — standard Tauri IPC call already used throughout `App.tsx`; `list_running_apps` follows the same pattern
- `#[cfg(target_os = "macos")]` / `#[cfg(target_os = "windows")]` guards — already used for platform-conditional IPC in `ipc/mod.rs`; `list_running_apps` needs separate macOS and Windows implementations

### Established Patterns
- `createSignal` + `createEffect` + `onCleanup` — the SolidJS async pattern for fetching data on mount or event; use for fetching running processes on dropdown open
- `try/catch` on `invoke()` — TypeScript error handling pattern already established; `list_running_apps` should be wrapped the same way
- `data-active` attribute on toggle (line 575) — shows how the existing UI encodes state for interactive elements; similar approach for capture mode state

### Integration Points
- The key capture widget writes `trigger_key: number | null` to `handleCreateMacro` (line 193) and to `bind_hotkey` IPC for existing macros
- The process picker writes `target_app: string | null` — same field as the text input it replaces; no Rust model changes needed
- Macro card click-to-edit calls existing `bind_hotkey` (macOS) and `set_macro_target_app` IPC commands — no new IPC needed for the edit path, only for listing processes

</code_context>

<specifics>
## Specific Ideas

- The "recording" state for key capture is a common pattern in macro/hotkey apps (Karabiner-Elements, BetterTouchTool, OBS). The interaction model should feel familiar: click → field highlights / shows "Press a key…" placeholder → user presses key → field shows name, exits recording mode.
- On Windows, `trigger_key` flows directly through `MacroConfig.trigger_key` (not through `bind_hotkey`, which is macOS-only). The key capture widget captures a keydown event and translates it to a Windows VK code via the frontend lookup table — no new Rust IPC needed for Windows key capture itself.
- The process picker should include a "Global (no targeting)" sentinel option so users can clear the target app without needing a separate button.

</specifics>

<deferred>
## Deferred Ideas

- **`trigger_key_name` field in Rust model / profile migration** — storing the human-readable name in `MacroConfig` would enable cross-platform profile portability (UX-04). This is a v2 requirement; Phase 3 does display-only mapping in the frontend.
- **Cargo dep audit** (`cocoa` 0.26, `objc` 0.2) — out of scope, deferred from Phase 2.
- **Windows hotkey modifier support** — `bind_hotkey`/`unbind_hotkey` are macOS-only. Full Windows hotkey binding with modifiers is a separate gap documented in CONCERNS.md (lines 150–152). Not Phase 3 scope.
- **Duplicate trigger key validation** — known bug (CONCERNS.md lines 45–49); no UI validation exists. Out of Phase 3 scope.

</deferred>

---

*Phase: 3-Macro Setup UX*
*Context gathered: 2026-05-30*
