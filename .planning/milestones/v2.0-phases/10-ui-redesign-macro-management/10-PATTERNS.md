# Phase 10: UI Redesign & Macro Management - Pattern Map

**Mapped:** 2026-07-23
**Files analyzed:** 12 (create/modify)
**Analogs found:** 12 / 12 (all analogs are within the existing single-file `App.tsx`/`App.css`/`state/mod.rs`/`ipc/mod.rs` — this phase decomposes `App.tsx` into new component files, so most "analogs" are sections of the file being decomposed, not separate files)

**Key context:** Per RESEARCH.md's recommended structure, this phase splits the current 1601-line `src/App.tsx` into `src/theme.ts` + `src/components/{Sidebar,ThemeToggle,MacroCard,MacroForm,KeyCaptureField}.tsx`. Because no component files currently exist, every new frontend file's "analog" is a specific block inside the current monolithic `App.tsx`/`App.css` — extraction, not net-new invention. The one net-new backend surface (`Intent::UpdateMacro` / `update_macro` IPC) has a direct analog in the existing `Intent::SetMacroTriggerKey` / `set_macro_trigger_key` pair.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/theme.ts` | utility (module) | event-driven (matchMedia + localStorage) | `src/keymap.ts` (sibling non-component module, exported pure functions) | role-match |
| `index.html` (boot script) | config/bootstrap | transform (sync read → DOM attribute) | none (net-new pattern; FOUC-prevention scripts are a standard idiom, not present in this codebase yet) | no analog |
| `src/components/Sidebar.tsx` | component (nav) | request-response (click → signal write) | `App.tsx:733-755` (current tab-bar `<button>` row wired to `activeTab()`/`setActiveTab`) | exact |
| `src/components/ThemeToggle.tsx` | component (control) | request-response (click → cycle state) | `App.tsx:1293-1303` (toggle-track click handler pattern) + `src/theme.ts` (new) | role-match |
| `src/components/MacroCard.tsx` | component (list item, 3 display states) | CRUD (toggle/edit/delete) + request-response | `App.tsx:1254-1461` (current inline macro-card `<For>` block, normal + editingCardId/editingField states) | exact |
| `src/components/MacroForm.tsx` | component (form, shared by create + edit) | CRUD (create) / request-response (edit submit) | `App.tsx:1081-1233` ("New Macro" form block, `handleCreateMacro`) | exact |
| `src/components/KeyCaptureField.tsx` | component (input widget) | event-driven (keydown capture) | `App.tsx:532-567` (`startCapture`) + `App.tsx:1160-1219` (capture chip UI + modifier chips) | exact |
| `src/App.tsx` (orchestrator, shrinks) | controller/orchestrator | request-response + event-driven | itself (refactor in place) — signal ownership pattern at `App.tsx:206-280` | exact |
| `src/App.css` (theme tokens + sidebar/card translucency) | config (design tokens) | transform (CSS custom properties) | `App.css:4-22` (`@theme` block), `App.css:76-88` (`.glass-card`) | exact |
| `src-tauri/tauri.conf.json` (window dims) | config | — | itself (value edit only) | exact |
| `src-tauri/src/state/mod.rs` (`Intent::UpdateMacro` + handler) | service/model (state mutation) | CRUD (atomic multi-field update) | `Intent::SetMacroTriggerKey` handler (`state/mod.rs:594-635`), `resolve_trigger_key_update` (`state/mod.rs:226+`) | exact |
| `src-tauri/src/ipc/mod.rs` (`update_macro` command) | controller (IPC command) | request-response | `set_macro_trigger_key` (`ipc/mod.rs:149-160`), `bind_hotkey` (`ipc/mod.rs:80-93`) | exact |

## Pattern Assignments

### `src/theme.ts` (utility module)

**Analog:** `src/keymap.ts` — sibling non-component `.ts` module exporting pure functions, imported by name into `App.tsx` (`import { domKeycodeToNative, resolveKeyName } from "./keymap";` at `App.tsx:6`).

**Pattern to copy:** module structure — plain exported functions, no class, no default export, imported via relative path with no alias (CONVENTIONS.md: "no path aliases"). `theme.ts` should mirror this shape:
```typescript
// keymap.ts convention: exported pure functions, no side effects at module scope
export function domKeycodeToNative(code: string): number | null { /* ... */ }
export function resolveKeyName(nativeCode: number): string { /* ... */ }
```
Apply the same shape to `theme.ts`'s `getStoredPreference` / `resolveTheme` / `applyTheme` (already spec'd concretely in RESEARCH.md's Pattern 2 code example — copy that verbatim, it already follows this file's conventions).

**localStorage pattern precedent:** `automux.hotkey_global_notice_dismissed` key referenced in RESEARCH.md's Architectural Responsibility Map row — confirms the `automux.` prefix convention for localStorage keys is already established; `theme.ts` must use `automux.theme_preference` per UI-SPEC, consistent with that prefix.

---

### `src/components/Sidebar.tsx` (new, replaces tab bar)

**Analog:** `App.tsx:733-755` — current top tab-bar buttons wired to `activeTab()`/`setActiveTab`.

**Core pattern to copy (signal-driven active state via `data-*`/class ternary, real `<button>` for native tab order):**
```tsx
// Source: App.tsx:733-755 (paraphrase of the existing structure — read exact
// lines during implementation since line numbers will shift after this
// phase's own edits)
<button
  onClick={() => setActiveTab("dashboard")}
  class={activeTab() === "dashboard" ? "<active classes>" : "<inactive classes>"}
>
  Dashboard
</button>
```
**Props pattern (per RESEARCH.md Pattern 1 — signals-down, accessor-props, no destructuring):**
```tsx
interface SidebarProps {
  activeTab: () => Tab;
  onSelectTab: (tab: Tab) => void;
}
const Sidebar: Component<SidebarProps> = (props) => {
  // read via props.activeTab(), NEVER `const { activeTab } = props`
};
```
`activeTab`/`setActiveTab` signal itself stays owned by `App.tsx` (unchanged location, per RESEARCH.md's "Sidebar (nav)" node in the architecture diagram) — Sidebar only receives the accessor and a callback.

---

### `src/components/ThemeToggle.tsx` (new)

**Analog:** `App.tsx:1293-1303` — `.toggle-track` click-to-mutate-state pattern (though ThemeToggle is a 3-state cycle button, not a boolean toggle, so the click handler shape — not the `.toggle-track` CSS class — is what's reused).

**Core pattern to copy (click handler mutating a signal, single source of truth via `theme.ts`):**
```tsx
// Source: App.tsx:1293-1303 pattern (click → mutate → re-render), adapted
// to cycle through theme.ts's ThemePreference union instead of a boolean
<div
  onClick={() => {
    const next = cyclePreference(currentPref()); // "system"→"light"→"dark"→"system"
    applyTheme(next); // from theme.ts — sets data-theme + localStorage
    setThemePref(next);
  }}
>
  {glyphFor(currentPref())}
</div>
```
Accessibility: per UI-SPEC, add `aria-label` describing the NEXT state (not current) since click semantics differ from the toggle-track's static Show when={active}/else pattern — no direct precedent in this codebase, net-new requirement, implement per UI-SPEC's exact wording.

---

### `src/components/MacroCard.tsx` (new — normal / edit / confirm-delete states)

**Analog:** `App.tsx:1254-1461` — the full `<For each={macroList()}>` card block, including the existing `editingCardId()`/`editingField()` mutual-exclusion pattern for target-app/trigger-key inline editors.

**Imports pattern** (mirrors `App.tsx:1-6`, hoist shared types to a `src/types.ts` or keep in `App.tsx` and import):
```tsx
import type { Component } from "solid-js";
import { Show } from "solid-js";
import type { MacroConfig, AppState, RunningState } from "../App"; // or hoisted ../types
```

**Core CRUD/display pattern** (running-state dot + toggle + delete, `App.tsx:1262-1313`):
```tsx
// Source: App.tsx:1262-1313
<div class="glass-card p-4">
  <div class="flex items-center justify-between mb-2">
    <div class="flex items-center gap-2">
      <div class={/* dot color switch on runningState() */} />
      <span class="text-sm font-medium">{macro.name}</span>
    </div>
    <div class="flex items-center gap-1.5">
      <div class="toggle-track" data-active={macro.enabled} onClick={() => handleToggleMacro(macro.id, macro.enabled)}>
        <div class="toggle-thumb" />
      </div>
      <button onClick={() => handleRemoveMacro(macro.id, macro.name)} class="... bg-danger/10 text-danger ..." title="Delete macro">✕</button>
    </div>
  </div>
</div>
```

**Mutual-exclusion edit-state pattern to extend, not duplicate** (`App.tsx:279-280`, `App.tsx:1320-1401`):
```tsx
// Source: App.tsx:279-280 — centralized in App(), NOT per-card local state
const [editingCardId, setEditingCardId] = createSignal<string | null>(null);
const [editingField, setEditingField] = createSignal<"key" | "target" | null>(null);
```
D-13's new full-card edit mode and D-14's new `confirmingDeleteId` MUST follow this same centralized-in-`App()`-passed-down-as-accessor-prop shape (RESEARCH.md explicitly calls this out as a pitfall to avoid: "New per-card local edit-state instead of centralized `editingCardId`"). Extend the existing `editingField` union or add a sibling `confirmingDeleteId` signal in `App.tsx`, pass both down as `() => boolean` accessor props to `MacroCard`.

**Delete confirmation — analog is the toast/message pattern, NOT a component** (`App.tsx:1466-1479`, message toast) combined with the NEW inline replacement of the header row per UI-SPEC C-D1. There is no existing inline-confirm-in-card precedent in this codebase (current `handleRemoveMacro` at `App.tsx:519-526` uses `window.confirm()` directly) — this is the one part of MacroCard that is genuinely new interaction, though UI-SPEC already fully specifies its markup/classes (see UI-SPEC C-D1).

**Error handling pattern to reuse for delete failure** (per UI-SPEC's backstop row): reuse `profileMessage`-style toast (`App.tsx:222-225`, rendered at `App.tsx:1466-1479`) — copy that signal shape (`{ text: string; type: "success" | "error" }`) for the new `Delete failed` banner, OR route through a card-local error state if scoped tighter; UI-SPEC leaves exact placement to the planner.

---

### `src/components/MacroForm.tsx` (new — shared by create panel AND inline edit)

**Analog:** `App.tsx:1081-1233` (the "New Macro" panel) + `App.tsx:453-509` (`handleCreateMacro`).

**Imports/structure pattern** — controlled inputs bound to signals, `<select>` with `<For>`-rendered options (`App.tsx:1094-1146`):
```tsx
// Source: App.tsx:1094-1146
<input value={newMacroName()} onInput={(e) => setNewMacroName(e.currentTarget.value)} class="bg-background border border-border rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent/50 transition-colors placeholder:text-text-dim" />
<select value={newMacroInput()} onChange={(e) => setNewMacroInput(e.currentTarget.value)} class="...">
  <option value="Left">🖱 Left Click</option>
  {/* extend with Key Press per D-11/UX-10 — 4th option */}
</select>
```

**Core "build the ActionSequence" pattern** (`App.tsx:460-481`, also given verbatim in RESEARCH.md Code Examples):
```typescript
// Source: App.tsx:460-481 (handleCreateMacro) — MacroForm's submit handler
// (both create AND edit paths) MUST build the SAME shape:
const input: InputEvent = inputVal === "Left" || inputVal === "Right" || inputVal === "Middle"
  ? { MouseButton: inputVal as "Left" | "Right" | "Middle" }
  : { Key: parseInt(inputVal) || 0 };
// mode === "Hold" ? { steps: [{ SustainedHold: { input } }] }
//                 : { steps: [{ InterleavedInterval: { input, interval_ms } }] };
```

**Error handling pattern (conflict surfacing)** (`App.tsx:494-508`):
```typescript
// Source: App.tsx:494-508
} catch (e) {
  const msg = String(e);
  const macroMatch = msg.match(/is already assigned to "([^"]+)"/);
  const macroName = macroMatch ? macroMatch[1] : "another macro";
  const keyLabel = triggerKey !== null ? resolveKeyName(triggerKey) : "Key";
  showConflictError(keyLabel, macroName);
}
```
The edit path's `Save Changes` handler must call `showConflictError` identically on `update_macro` rejection (per UI-SPEC Interaction Contract row "3alt" — reuses Phase 8 `ConflictErrorToast`, edit form stays open).

**Validation pattern** — client-only, no backend validation (per RESEARCH.md Open Question #2, resolved as "mirror existing precedent"): `disabled={!newMacroName().trim()}` (`App.tsx:1225`) — copy this exact guard onto the edit form's `Save Changes` button.

---

### `src/components/KeyCaptureField.tsx` (new — wraps `startCapture`, reused for hotkey binding AND Key Press action input)

**Analog:** `App.tsx:532-567` (`startCapture` function) + `App.tsx:1160-1219` (capture-chip rendering with modifier preview).

**Core event-driven pattern** (`App.tsx:532-567`, copy near-verbatim — this is the exact D-12-mandated reuse):
```typescript
// Source: App.tsx:532-567
function startCapture(onCommit: (nativeCode: number, modifiers: number) => void) {
  if (_keyCaptureListener) {
    document.removeEventListener("keydown", _keyCaptureListener, true);
    _keyCaptureListener = null;
  }
  setTriggerKeyRecording(true);
  setRecordingModifiers(0);
  function onKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") { /* cancel */ return; }
    const mods = computeModifiers(e);
    setRecordingModifiers(mods);
    const nativeCode = domKeycodeToNative(e.code);
    if (nativeCode === null) return;
    onCommit(nativeCode, mods);
    /* reset + remove listener */
  }
  _keyCaptureListener = onKeyDown;
  document.addEventListener("keydown", onKeyDown, true);
}
```
**Single-listener invariant comment to preserve** (`App.tsx:528`, `// @architect: Single-listener invariant via module-level ref (T-03-08)`) — when extracted into `KeyCaptureField.tsx`, the `_keyCaptureListener` module-level ref must remain a SINGLE shared ref if two independent capture instances (trigger-key AND Key-Press-input, per UI-SPEC C-E1 "two independent key-capture instances can coexist on one card") are mounted simultaneously — verify this invariant still holds after extraction; may need to become a `Map`/keyed-by-instance if truly concurrent, or confirm only one can be actively recording at a time (current code's `if (editingCardId() !== null)` cancel-on-open at `App.tsx:1186-1190` suggests the latter is the intended model).

**Chip rendering pattern** (`App.tsx:1160-1219`, modifier chips + capture-state chip):
```tsx
// Source: App.tsx:1164-1174 (modifier chips) + 1175-1218 (capture chip)
<Show when={triggerKeyRecording() && recordingModifiers() !== 0}>
  <For each={modifierChips(recordingModifiers())}>
    {(label) => <span class="px-1.5 py-0.5 rounded bg-surface-alt border border-border text-[10px] font-mono text-text-main">{label}</span>}
  </For>
</Show>
```
**UI-SPEC typography note:** the `text-[10px]` chip class above must be updated to `text-[11px] font-semibold` per UI-SPEC's consolidated 4-size/2-weight typography scale (Label role) — do not copy the `10px`/`font-mono`-only styling verbatim, apply the UI-SPEC typography override during extraction.

---

### `src/App.tsx` (orchestrator, post-decomposition)

**Analog:** itself — signal-ownership block at `App.tsx:206-280` is the pattern to PRESERVE, not replace: all `createSignal` calls stay in `App()`, extracted components receive accessor-function props (RESEARCH.md Pattern 1). No new state-management library or pattern is introduced.

**New signals needed (co-located with existing analogous signals):**
- `confirmingDeleteId` — alongside `editingCardId`/`editingField` (`App.tsx:279-280`)
- theme preference signal, initialized from `theme.ts`'s `getStoredPreference()` — alongside `appVersion`/other boot-read signals (`App.tsx:250`)

---

### `src/App.css` (theme tokens + translucency)

**Analog:** `App.css:4-22` (`@theme` block) — becomes the dark-mode token source; `App.css:76-88` (`.glass-card`) — becomes the translucent version.

**Exact current dead-blur declaration to replace** (confirms RESEARCH.md's Pitfall 1 finding):
```css
/* Source: App.css:76-83 — CURRENT (gradient-only, backdrop-filter inert) */
.glass-card {
  background: linear-gradient(135deg, var(--color-surface) 0%, var(--color-surface-alt) 100%);
  border: 1px solid var(--color-border);
  border-radius: 12px;
  backdrop-filter: blur(12px);
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}
```
UI-SPEC's replacement (`background: color-mix(in srgb, var(--color-surface) 72%, transparent)` + `backdrop-filter: blur(20px) saturate(150%)`) goes in this exact rule location — same selector, same file, values swapped per UI-SPEC Translucency section.

**Token restructure pattern** (RESEARCH.md Pattern 3, Tailwind v4 `@theme` + `[data-theme]`) — copy that code example directly; it already shows the exact `:root` / `[data-theme="light"]` / `@theme inline` three-block structure to replace the current flat `@theme { ... }` block at `App.css:4-22`.

**New sidebar/toggle-track-adjacent CSS classes:** `.sidebar-glass` (new, per UI-SPEC C-SB1 "implemented as a small CSS class... since Tailwind 4 utilities alone can't express color-mix + custom blur values inline") — follow the same rule shape as `.glass-card` (a hand-written CSS class alongside Tailwind utility classes in the same file, not a Tailwind `@apply` chain — matches this file's existing convention of hand-rolled classes for `.toggle-track`/`.toggle-thumb`/`.status-pulse`).

---

### `src-tauri/tauri.conf.json` (window dimensions)

**Analog:** itself. Pure value edit — `width`/`height` 420×640 → 720×680, `minWidth`/`minHeight` 360×480 → 560×520, `transparent` stays `false`. No structural change, no analog needed beyond the current file's own schema.

---

### `src-tauri/src/state/mod.rs` — `Intent::UpdateMacro` (new)

**Analog:** `Intent::SetMacroTriggerKey` handler (`state/mod.rs:594-635`) — same Result-carrying-oneshot, conflict-pre-check-before-mutation shape; and `resolve_trigger_key_update` (`state/mod.rs:226+`) — reused as-is, not reimplemented.

**Enum variant pattern to copy** (mirrors `BindHotkey` at `state/mod.rs:176`):
```rust
// Source: state/mod.rs:176 (BindHotkey) shape, extended per RESEARCH.md's
// Code Examples section (already fully drafted there — copy verbatim):
UpdateMacro(
    Uuid,
    String,               // name
    ActionSequence,
    TriggerMode,
    Option<String>,       // target_app
    Option<u16>,           // trigger_key
    Option<u64>,           // trigger_modifiers
    tokio::sync::oneshot::Sender<Result<(), String>>,
),
```

**Handler pattern to copy exactly** (`state/mod.rs:594-635`, `Intent::SetMacroTriggerKey`):
```rust
// Source: state/mod.rs:594-635 — SAME shape: match resolve_trigger_key_update,
// Err → reply Err(msg) WITHOUT mutating (atomic reject), Ok → mutate fields +
// reevaluate_all_macros() + recompute_conflicts() + auto_save_default() + reply Ok(())
match resolve_trigger_key_update(&self.state, id, trigger_key, trigger_modifiers) {
    Err(conflicting_id) => {
        let conflicting_name = self.state.macros.get(&conflicting_id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| format!("{:?}", conflicting_id));
        let msg = format!(
            "Key (keycode {}) is already assigned to \"{}\". Unbind it first or pick a different key.",
            trigger_key.unwrap_or_default(), conflicting_name
        );
        let _ = reply.send(Err(msg));
    }
    Ok((new_key, new_mods)) => {
        if let Some(mac) = self.state.macros.get_mut(&id) {
            mac.name = name;
            mac.sequence = sequence;
            mac.trigger_mode = trigger_mode;
            mac.target_app = target_app;
            mac.trigger_key = new_key;
            mac.trigger_modifiers = new_mods;
        }
        self.reevaluate_all_macros().await;
        self.recompute_conflicts();
        self.auto_save_default().await;
        let _ = reply.send(Ok(()));
    }
}
```
**Do-not-hand-roll note:** `resolve_trigger_key_update` is called AS-IS, not reimplemented — RESEARCH.md's Don't-Hand-Roll table explicitly flags duplicating this logic as a drift risk.

**Test pattern to copy** (per RESEARCH.md Validation Architecture — inline `#[cfg(test)] mod tests` in the SAME file, not a separate test file): follow the existing style of `set_trigger_key_rejects_conflict_without_coercion` and `hold_release_bypasses_gates` (referenced at `state/mod.rs:1038-1076` region) for the three new tests: `update_macro_applies_all_fields`, `update_macro_conflict_no_partial_mutation`, and a persistence round-trip test matching the existing `profile_backwards_compat`-style pattern.

---

### `src-tauri/src/ipc/mod.rs` — `update_macro` command (new)

**Analog:** `set_macro_trigger_key` (`ipc/mod.rs:149-160`) and `bind_hotkey` (`ipc/mod.rs:80-93`) — both are `#[command(rename_all = "snake_case")]`, oneshot-channel, `Result`-returning async fns.

**Imports pattern** (`ipc/mod.rs:1-5`):
```rust
use crate::state::{ActionSequence, AppState, Intent, MacroConfig, StateManager};
use tauri::{command, State};
use uuid::Uuid;
```

**Core pattern to copy exactly** (`ipc/mod.rs:80-93`, `bind_hotkey`):
```rust
// Source: ipc/mod.rs:80-93 (bind_hotkey) — same oneshot + double-unwrap
// (`.map_err` on send, `?` then `.map_err` on the inner Result) shape
#[command(rename_all = "snake_case")]
pub async fn update_macro(
    state: State<'_, StateManager>,
    id: Uuid,
    name: String,
    sequence: ActionSequence,
    trigger_mode: crate::state::TriggerMode,
    target_app: Option<String>,
    trigger_key: Option<u16>,
    trigger_modifiers: Option<u64>,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    state
        .send_intent(Intent::UpdateMacro(id, name, sequence, trigger_mode, target_app, trigger_key, trigger_modifiers, tx))
        .await
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}
```
**IPC naming convention:** `update_macro` (snake_case) on the Rust side, `invoke<void>("update_macro", { ... })` verbatim on the TypeScript side per CONVENTIONS.md's "IPC Naming" rule (Rust `#[command]` function names use snake_case; TypeScript `invoke` call strings use the same names verbatim).

---

## Shared Patterns

### Signals-down, accessor-props, no destructuring (SolidJS-wide)
**Source:** `App.tsx:279-280` (`editingCardId`/`editingField` centralized-in-parent pattern), RESEARCH.md Pattern 1
**Apply to:** ALL new component files (`Sidebar.tsx`, `ThemeToggle.tsx`, `MacroCard.tsx`, `MacroForm.tsx`, `KeyCaptureField.tsx`)
```tsx
// NEVER: const { macro, isEditing } = props;
// ALWAYS: props.macro, props.isEditing() read inline in JSX/reactive scopes
```

### Message/toast pattern for async operation feedback
**Source:** `App.tsx:222-225` (`profileMessage` signal) + `App.tsx:1466-1479` (render) + `App.tsx:241-248` (`showConflictError`, 8s auto-dismiss via `setTimeout`)
**Apply to:** New delete-failure banner (UI-SPEC backstop item), reuse either the `profileMessage` shape or the `conflictError` auto-dismiss-timer shape depending on scope.

### Result-carrying oneshot + atomic-reject-on-conflict (Rust)
**Source:** `Intent::BindHotkey` (`state/mod.rs:636-663`), `Intent::SetMacroTriggerKey` (`state/mod.rs:594-635`)
**Apply to:** `Intent::UpdateMacro` — the ONLY backend addition this phase requires. Every mutating Intent that can fail must follow this exact "check first, mutate only on Ok, reply via oneshot" shape.

### `@theme` custom-property + `[data-theme]` attribute Tailwind v4 theming
**Source:** RESEARCH.md Pattern 3 (Tailwind v4 official docs, cited)
**Apply to:** `App.css` restructure — the single mechanism for BOTH the light/dark tokens (D-06/D-07) and, incidentally, is the same custom-property mechanism `.glass-card`'s new translucent background depends on (`color-mix(in srgb, var(--color-surface) 72%, transparent)`).

### Controlled-input form fields with Tailwind utility classes (no `className`, no CSS modules)
**Source:** `App.tsx:1094-1146` (name input, Input select, target-app select)
**Apply to:** `MacroForm.tsx` — every field in both create and edit modes uses this exact `value={signal()} onInput={(e) => setSignal(e.currentTarget.value)}` controlled pattern with inline `class=` Tailwind strings (CONVENTIONS.md: "All styling via inline `class=` Tailwind utility strings").

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `index.html` boot-time theme script | config/bootstrap | transform | No FOUC-prevention script exists yet in this codebase (app is dark-only today, no theme-flash concern existed). RESEARCH.md Pattern 2 provides a complete, ready-to-copy example from standard practice — use that directly, no local precedent to point to instead. |
| `src/components/MacroCard.tsx`'s inline delete-confirm sub-state (C-D1) | UI state (within component) | request-response | `window.confirm()` (`App.tsx:519-526`) is being fully replaced, not extended — there's no existing in-app inline-confirmation-card precedent anywhere in this codebase to copy from. UI-SPEC's C-D1 section already fully specifies the exact markup/classes to use instead of a code analog. |

## Metadata

**Analog search scope:** `src/App.tsx` (all 1601 lines, via targeted grep + offset reads), `src/App.css` (full file, 120 lines), `src-tauri/src/state/mod.rs` (Intent enum + SetMacroTriggerKey/BindHotkey handlers), `src-tauri/src/ipc/mod.rs` (first 100 lines, command definitions)
**Files scanned:** 4 primary source files (no separate component/service directories exist yet — single-file architecture per STRUCTURE.md)
**Pattern extraction date:** 2026-07-23
