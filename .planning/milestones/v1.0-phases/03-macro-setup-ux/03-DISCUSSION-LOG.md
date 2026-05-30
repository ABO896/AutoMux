# Phase 3: Macro Setup UX - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-30
**Phase:** 3-Macro Setup UX
**Areas discussed:** Key capture interaction, Process picker UI, Edit scope

---

## Key Capture Interaction

### How does the user enter capture mode?

| Option | Description | Selected |
|--------|-------------|----------|
| Click field to capture | Field shows "Click to set key…"; clicking enters recording state that intercepts next keydown, shows key name; Clear/Cancel exits without committing | ✓ |
| Dedicated "Capture" button | Button next to field opens capture mode; field itself is not clickable | |
| Always listening | Field auto-captures any key pressed while focused | |

**User's choice:** Click field to capture
**Notes:** No additional notes.

---

### What label to show after capture?

| Option | Description | Selected |
|--------|-------------|----------|
| Human-readable name only | Show only the key name: "Q", "Space", "F5", "Left Shift" | ✓ |
| Name + code | Show both: "Q (12)" or "Space (49)" | |
| You decide | Let planner/researcher pick based on app's existing style | |

**User's choice:** Human-readable name only
**Notes:** Raw integers are never displayed to users.

---

### Where does the human-readable key name come from?

| Option | Description | Selected |
|--------|-------------|----------|
| Frontend lookup table | Static Map<number, string> in TypeScript layer; no new IPC needed | ✓ |
| Rust IPC command | New `get_key_name(code: u16) -> String` command; async round-trip for display-only data | |
| Store name in Rust model | Add `trigger_key_name: Option<String>` to MacroConfig; enables profile portability but requires model migration | |

**User's choice:** Frontend lookup table
**Notes:** Model storage deferred to v2 (UX-04).

---

## Process Picker UI

### How should the process picker be presented?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline `<select>` dropdown | Replaces text input with `<select>` populated by `list_running_apps` IPC on open; consistent with existing selects | ✓ |
| Picker modal/popover | "Pick App" button opens modal with searchable list; more room for detail; higher effort | |
| Searchable combobox | Text input with autocomplete dropdown; needs custom component (none exists) | |

**User's choice:** Inline `<select>` dropdown
**Notes:** Keeps the UI surface minimal and consistent with existing form selects.

---

### What should each option show?

| Option | Description | Selected |
|--------|-------------|----------|
| Display name + bundle ID / path | macOS: "Minecraft (com.mojang.minecraft)"; Windows: "Minecraft (C:\...\Minecraft.exe)" | ✓ |
| Display name only | Just "Minecraft"; stored value is still bundle ID / path | |
| You decide | Let researcher/planner pick based on what NSWorkspace / Win32 APIs surface easily | |

**User's choice:** Display name + bundle ID / path
**Notes:** Helps users distinguish between apps with similar display names.

---

### When should the process list be fetched?

| Option | Description | Selected |
|--------|-------------|----------|
| On open each time | Call `list_running_apps` every time the dropdown opens; always fresh | ✓ |
| Once at form mount, refresh button | Fetch once when creation form opens, with a manual refresh button | |
| You decide | Let the planner choose the fetch strategy | |

**User's choice:** On open each time
**Notes:** No caching; always reflects current running processes.

---

## Edit Scope

### Does Phase 3 add editing to existing macro cards?

| Option | Description | Selected |
|--------|-------------|----------|
| Creation form only | Phase 3 improves only the creation form; editing existing macros requires delete and recreate | |
| Creation form + inline card editing | Phase 3 also adds click-to-edit for trigger key and target app on macro cards | ✓ |

**User's choice:** Creation form + inline card editing
**Notes:** Macro cards are currently fully read-only; Phase 3 introduces edit affordances for these two fields.

---

### How is inline card editing triggered and committed?

| Option | Description | Selected |
|--------|-------------|----------|
| Click field, auto-commit on change | Clicking "Key X" badge → capture mode → keypress commits via `bind_hotkey`; clicking target badge → picker → selection commits via `set_macro_target_app`; no save button | ✓ |
| Edit button per card | "Edit" button shows inline form for all editable fields with explicit Save button | |

**User's choice:** Click field, auto-commit on change
**Notes:** Escape/cancel gesture exits without committing.

---

## Claude's Discretion

- Exact visual treatment of the recording state (border pulse, placeholder text color)
- Fallback display when a captured key has no entry in the lookup table (raw integer vs "Unknown (N)")
- Whether "Global" sentinel is first `<option>` in the picker or a separate "Clear target" affordance

## Deferred Ideas

- `trigger_key_name` field in Rust model / profile migration (UX-04 — v2 scope)
- Windows hotkey modifier support (`bind_hotkey` is macOS-only; Windows gap documented in CONCERNS.md)
- Duplicate trigger key validation (known bug in CONCERNS.md; not Phase 3 scope)
- Cargo dep audit (`cocoa`, `objc` 0.2) — deferred from Phase 2
