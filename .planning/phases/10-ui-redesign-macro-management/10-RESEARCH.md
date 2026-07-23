# Phase 10: UI Redesign & Macro Management - Research

**Researched:** 2026-07-23
**Domain:** Cross-platform Tauri 2 / SolidJS UI redesign (CSS translucency, theming) + Rust backend intent-model extension for macro editing
**Confidence:** MEDIUM — architecture and pitfalls are HIGH confidence (verified against actual source); the CSS `backdrop-filter` perf question is inherently empirical (must be measured on-device, not predicted) so is flagged LOW/needs-verification throughout

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** One unified custom visual design is used on **both Windows and macOS, across all supported OS versions** — no OS-version tiering, no macOS-26-only glass tier with a lesser fallback elsewhere. Reversibility: costly.
- **D-02:** The redesign adopts the "modern liquid-glass-era" design language broadly (translucent panels, blur, sleek rounded elements) but is **not a literal Raycast clone** — skip patterns (e.g. command-bar search) that don't fit AutoMux's actual content (a macro list, not a launcher).
- **D-03:** Translucency uses **real CSS `backdrop-filter: blur(...)`** — genuine translucency, replacing the current gradient-only `.glass-card` treatment. Must verify this doesn't violate UI-04 (no measurable idle overhead vs v1.2.0).
- **D-04 (Claude's discretion — RESOLVED in UI-SPEC):** CSS-only blur between opaque internal panels; **no** native OS-level window vibrancy. `transparent: false` stays. This research independently corroborates that choice (see Common Pitfalls).
- **D-05 (Claude's discretion — RESOLVED in UI-SPEC):** Keep existing indigo accent (`#6366f1` dark / `#4f46e5` light).
- **D-06:** Add a **light/dark theme toggle** — AutoMux is dark-only today. New scope. Reversibility: costly.
- **D-07:** Theme **defaults to OS system preference**, manual override available.
- **D-08 (Claude's discretion — RESOLVED in UI-SPEC):** Persistent left sidebar rail (84px), replacing the top tab bar. List+detail split explicitly ruled out (redundant with D-13's inline expand).
- **D-09 (Claude's discretion — RESOLVED in UI-SPEC):** Window resized to 720×680 (min 560×520), from 420×640 (min 360×480).
- **D-10 (Claude's/researcher's discretion — RESOLVED in UI-SPEC):** **Two selectors** (Input: Left/Right/Middle Click/Key Press; Mode: Pulse/Hold), NOT a unified 4-item flat list — because `Hold` is a mode that crosses with any input type in the backend model, not a peer item.
- **D-11:** "Key Press" must become genuinely selectable in both create and edit forms — `InputEvent::Key` already exists in the type system but the creation dropdown (`App.tsx:1111-1113`) only offers 3 mouse buttons today.
- **D-12:** Key-press action capture **reuses the exact same key-capture widget** already shipped in Phase 8 for hotkey trigger-key binding (modifier chips, keydown capture, Escape-to-cancel) — no new key-picker component.
- **D-13:** Editing happens via **inline expand-in-place** on the macro card — no modal/overlay. Reuses the same fields/components as the creation form.
- **D-14:** Delete's `window.confirm()` is replaced with a **custom in-app inline confirmation** (not a native dialog) — exact form left to planner, inline "are you sure?" card state is the natural fit given D-13.

### Claude's Discretion (all resolved by UI-SPEC — treat as locked for this phase)

D-04, D-05, D-08, D-09, D-10 were "Claude's discretion" per CONTEXT.md but have since been resolved and locked by the approved `10-UI-SPEC.md` design contract. This research treats those UI-SPEC resolutions as binding — do not re-litigate them.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope. No user-suggested features were redirected to backlog.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UI-01 | macOS uses Apple design language / liquid-glass-era visual effects (reinterpreted by D-01 as a single cross-platform look, not literal Tahoe-only Liquid Glass API) | See Translucency Implementation, Common Pitfalls #1/#2/#3 — CSS-only `backdrop-filter` + `color-mix()` translucency achieves the visual goal without native APIs |
| UI-02 | Raycast-inspired layout — clean, focused hierarchy, keyboard-navigable | See Architecture Patterns — sidebar as real `<button>` elements (native tab order), component decomposition strategy |
| UI-03 | Windows presents a modern, polished equivalent UI without macOS-specific effects | Same CSS-only mechanism works identically on WebView2 (Chromium-based) — see Common Pitfalls #2 for the one WebView2-specific gotcha |
| UI-04 | No measurable idle memory/CPU overhead vs v1.2.0 | See Performance section — measurement protocol, `.glass-card`'s pre-existing (non-functional) `backdrop-filter` baseline, and the documented fallback (reduce blur layer count) |
| UX-08 | Delete without entering edit mode (already shipped Phase 9; this phase restyles the confirmation UX) | See Component Inventory (UI-SPEC C-D1) — inline confirm state, existing `remove_macro` IPC unchanged |
| UX-09 | Edit name, action type, key/button, timing after creation; persists across restarts | See Backend Intent Extension — new `Intent::UpdateMacro` required; existing per-field intents do NOT cover `name` or `trigger_mode` |
| UX-10 | Unambiguous action-type labels | See Action-Type Model verification — confirms UI-SPEC's two-selector resolution is technically sound against the actual `ActionStep`/`TriggerMode` backend model |

</phase_requirements>

## Summary

This phase is a full visual re-skin of a single-file 1601-line SolidJS component (`src/App.tsx`) plus two new backend-touching interaction surfaces (macro edit, restyled delete confirm). The UI-SPEC (`10-UI-SPEC.md`) has already locked every visual decision — colors, spacing, typography, component specs, copy — so this research focuses on **implementation mechanics**, not design choices.

Three technical questions drove this research. First, whether real `backdrop-filter` translucency can be added without violating UI-04's no-overhead constraint: the current `.glass-card` class **already declares** `backdrop-filter: blur(12px)` (`App.css:81`) but it has **zero visible effect** today because the card's own `background` is a fully opaque gradient — blur only matters when there's alpha to blend through. This means the GPU compositing layer for `.glass-card` may already exist at the v1.2.0 baseline (browsers may or may not skip compositing work for backdrop-filter behind fully-opaque paint — unverified, flagged LOW), and the redesign's real change is making that background translucent (`color-mix(..., 72%, transparent)`), which is what actually makes the blur (and its cost) visible for the first time. The safe path is the one UI-SPEC already prescribes: measure against v1.2.0 with the documented protocol, and if idle CPU/GPU rises, cut blur layer count (fewer, larger translucent surfaces — sidebar + banners — rather than blur on every scrollable macro card).

Second, whether native OS-level window vibrancy (`transparent: true` + NSVisualEffectView/Acrylic-Mica) is worth the platform-specific Rust code: this research found **direct evidence it would actively work against D-03**. Multiple confirmed `tauri-apps/tauri` GitHub issues (#12437, #2827, #2976) report that setting `transparent: true` on a Tauri window **degrades or breaks CSS `backdrop-filter` blur quality specifically on Windows WebView2** — the opposite of what D-03 requires. Combined with Tauri's own documented "bad performance when resizing/dragging" warning on the `Acrylic`/`Blur` window effects, this independently corroborates UI-SPEC's D-04 resolution (`transparent: false`, CSS-only blur) as the technically correct choice, not just the lower-effort one.

Third, the backend surface for UX-09 (edit): tracing the actual `Intent` enum in `src-tauri/src/state/mod.rs` shows the existing per-field intents (`SetMacroTargetApp`, `SetMacroTriggerKey`, `UpdateSequence`) cover target app, trigger key, and step content — but **there is no intent to update a macro's `name` or `trigger_mode`** at all today (both are write-once at `AddMacro` time). A new consolidated `Intent::UpdateMacro` is required, not just wiring to existing commands.

**Primary recommendation:** Implement translucency exactly as UI-SPEC specifies but measure early (first plan/wave, not last) so the fallback (reduce blur surface count) can be applied before the rest of the phase is built on top of it; add one atomic `Intent::UpdateMacro`/`update_macro` IPC pair (Result-carrying oneshot, mirroring `BindHotkey`'s conflict-check pattern) rather than sequencing multiple existing per-field IPC calls from the frontend; decompose `App.tsx` into sibling `PascalCase.tsx` files under `src/components/` with signals/handlers owned by `App()` and passed down as accessor-function props, never destructured.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Visual redesign (translucency, theme, sidebar, typography) | Frontend Server/Client (SolidJS component tree) | CDN/Static (bundled CSS via Vite) | Pure client-rendered desktop webview; no SSR tier exists in this Tauri app — `src/App.tsx` + `src/App.css` own 100% of visual presentation |
| Theme preference persistence | Browser/Client (`localStorage`) | — | No backend involvement — matches existing pattern (`automux.hotkey_global_notice_dismissed` uses the same mechanism) |
| Macro edit (name/action-type/key/timing) | API/Backend (`StateActor` via new `Intent::UpdateMacro`) | Frontend Client (form UI, optimistic none — waits for `state-changed`) | All macro state mutations go through the Intent channel per the existing architectural invariant (`ARCHITECTURE.md`: "All mutations to AppState are expressed as Intent variants") — the edit form is pure UI, not a new mutation path |
| Macro delete confirmation UX | Browser/Client (local `confirmingDeleteId` signal) | API/Backend (`remove_macro`, unchanged) | The confirm/cancel state is transient UI-only; only the final `Delete` click touches the backend, via the already-shipped `remove_macro` IPC |
| Window chrome / vibrancy | (Not applicable — explicitly rejected) | — | D-04 resolution: no native window-tier code touched this phase; window dimensions only (`tauri.conf.json`) |
| Persistence of edited macro across restart | Database/Storage (`ProfileManager` → JSON files) | API/Backend (`auto_save_default()`) | Existing auto-save-on-every-mutation pattern — no new persistence mechanism needed, `Intent::UpdateMacro`'s handler calls the same `auto_save_default()` as every other mutating intent |

## Standard Stack

### Core

**This phase introduces zero new npm or Cargo dependencies** — confirmed by `10-UI-SPEC.md`'s Registry Safety section and independently verified against `package.json`/`Cargo.toml` during this research. The existing stack is sufficient:

| Library | Installed Version | Verified Current (npm registry) | Purpose |
|---------|---------|---------|---------|
| `solid-js` | `^1.9.3` | 1.9.14 [VERIFIED: npm registry] | Reactive UI framework — `Show`/`For`, `createSignal`, `createEffect` cover every new interaction (sidebar nav, theme cycle, inline edit/confirm) |
| `tailwindcss` | `^4.3.0` | 4.3.3 [VERIFIED: npm registry] | Utility CSS + `@theme` custom-property design tokens; v4's `@theme`/data-attribute pattern officially supports the light/dark scheme this phase needs [CITED: tailwindcss.com/docs/dark-mode] |
| `@tauri-apps/api` | `^2` | — (pinned to `^2`, no action needed) | `invoke`/`listen` — unchanged, no new IPC surface pattern needed beyond one new command |
| `@tauri-apps/cli` | `^2` | 2.11.4 [VERIFIED: npm registry] | Build tooling, unaffected |

### Supporting

None new. `keymap.ts` (existing, 196 lines) already provides `domKeycodeToNative`/`resolveKeyName` and is reused as-is by the D-12 key-capture reuse.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| CSS-only `backdrop-filter` blur (D-04 resolution) | `window-vibrancy` Rust crate (community) + `transparent: true` for true NSVisualEffectView/Acrylic vibrancy | Rejected: platform-specific Rust code on both OSes, Tauri's own docs flag `Acrylic`/`Blur` window effects as having "bad performance when resizing/dragging" on Windows, and multiple open `tauri-apps/tauri` issues report `transparent: true` actively **degrading** CSS `backdrop-filter` quality on Windows WebView2 — directly undermining D-03's translucency goal, not just adding risk |
| One consolidated `Intent::UpdateMacro` | Sequential frontend calls to existing `set_macro_target_app` + `set_macro_trigger_key` + `set_macro_sequence` + a new tiny `set_macro_name`/`set_macro_trigger_mode` pair | Rejected as primary approach: the `StateActor::run()` loop broadcasts `state-changed` and (for most intents) calls `auto_save_default()` after **every single handled intent** — 4-5 sequential IPC calls from one form submission means 4-5 disk writes and 4-5 broadcast events per edit, and if the trigger-key conflict check (the only one of these that can fail) fails on call #3 after calls #1-2 already succeeded, the macro is left in a partially-edited, already-persisted state with no rollback. A single atomic intent avoids both problems. |
| SolidJS component decomposition into `src/components/*.tsx` | Keep everything in one `App.tsx` file | Rejected for this phase specifically because of scale: this phase adds a sidebar (~40 lines), theme toggle (~20 lines), inline edit form (~150+ lines duplicating/generalizing the existing ~150-line creation form), and inline delete confirm (~30 lines) on top of an already-1601-line file with no internal structure. `CONVENTIONS.md`'s "No separate component files currently" is a description of the current state, not a rule against ever introducing one — and `keymap.ts` already establishes precedent for splitting non-component logic into sibling files. |

## Package Legitimacy Audit

**Not applicable — this phase adds zero new npm or Cargo packages.** Verified against `10-UI-SPEC.md`'s Registry Safety section (explicit "zero new npm dependencies" declaration) and confirmed by reading `package.json`/`Cargo.toml` during this research. No `package-legitimacy check` run was needed.

## Architecture Patterns

### System Architecture Diagram

```text
┌─────────────────────────────────────────────────────────────────────┐
│  SolidJS Frontend (src/App.tsx + new src/components/*.tsx)           │
│                                                                        │
│  ┌──────────┐   ┌──────────────┐   ┌───────────────────────────┐    │
│  │ Sidebar  │   │ ThemeToggle  │   │  Content column            │    │
│  │ (nav)    │   │ (localStorage│   │  ┌───────────────────────┐│    │
│  └────┬─────┘   │  + data-theme│   │  │ MacroForm (shared by  ││    │
│       │         │  attribute)  │   │  │ create panel AND      ││    │
│       │         └──────────────┘   │  │ inline card-edit)     ││    │
│       │ activeTab()                │  └───────────┬───────────┘│    │
│       ▼                            │              │             │    │
│  ┌─────────────────────────────────┴──────────────┴───────────┐│    │
│  │ For each macro: <MacroCard> — 3 display states:              ││    │
│  │   normal → (click ✎) → edit (mounts MacroForm, pre-filled)   ││    │
│  │   normal → (click ✕) → confirm-delete (inline, no modal)     ││    │
│  └───────────────────────────────┬────────────────────────────┘│    │
└──────────────────────────────────┼───────────────────────────────────┘
                                    │ invoke("update_macro" | "remove_macro" | ...)
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Tauri IPC Layer (src-tauri/src/ipc/mod.rs)                          │
│  update_macro(id, name, sequence, trigger_mode, target_app,          │
│               trigger_key, modifiers) → oneshot<Result<(), String>>  │
└──────────────────────────────────┬────────────────────────────────────┘
                                    │ Intent::UpdateMacro
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│  StateActor (src-tauri/src/state/mod.rs)                             │
│  1. resolve_trigger_key_update() — conflict pre-check (reused)       │
│  2.   Err → reply Err(msg), NO mutation (atomic reject)               │
│  3.   Ok  → mutate name/sequence/trigger_mode/target_app/trigger_key  │
│  4.        reevaluate_all_macros() + recompute_conflicts()            │
│  5.        auto_save_default() → ProfileManager → default.json        │
│  6.        reply Ok(())                                               │
│  7. run() loop broadcasts state-changed ONCE (not N times)            │
└─────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
src/
├── App.tsx                  # orchestrator: signals, IPC calls, effects, tab state — shrinks from 1601 lines
├── App.css                  # @theme dark tokens (unchanged values) + new [data-theme="light"] override block
├── keymap.ts                # unchanged — domKeycodeToNative / resolveKeyName
├── theme.ts                 # NEW — small module: read/write localStorage pref, resolve "system" via matchMedia, boot-time apply
└── components/
    ├── Sidebar.tsx           # NEW — C-SB1 nav rail (Macros/Profiles tabs)
    ├── ThemeToggle.tsx        # NEW — C-SB2, 3-state cycle button
    ├── MacroCard.tsx          # NEW — normal/edit/confirm-delete display states for one macro
    ├── MacroForm.tsx          # NEW — shared field set (name, Input+Mode selectors, key-capture, target-app), used by both the "New Macro" panel and MacroCard's inline edit
    └── KeyCaptureField.tsx    # NEW — wraps existing startCapture()/modifierChips() logic + Press…/Set key… display, reused for BOTH hotkey trigger-key capture (existing) and Key Press action-type capture (new, D-11/D-12)
```

Rationale for this exact split: `MacroForm` is the direct implementation of D-13's requirement ("reuses the same fields/components as the macro creation form") — building it once and mounting it from two call sites (the always-a-panel creation form, and a per-card conditional inside `MacroCard`) is what makes that requirement concretely enforceable rather than aspirational. `KeyCaptureField` is the direct implementation of D-12 (same widget for hotkey binding AND the new Key Press input capture) for the same reason.

### Pattern 1: Signals-down, accessor-props, no destructuring

**What:** Parent (`App.tsx`) owns all `createSignal` state; child components receive plain values or accessor functions (`() => T`) as props and read them inside JSX/reactive scopes, never destructured at the top of the component body.

**When to use:** Every new child component this phase introduces.

**Why it matters here specifically:** the codebase already has a working mutual-exclusion invariant — `editingCardId()`/`editingField()` signals live in `App()` and gate "only one card editable at a time" (existing pattern, `App.tsx:279-280`). Decomposing `MacroCard` into its own file does NOT mean giving each card its own local edit-state signal — that would silently break the mutual-exclusion guarantee D-13 depends on ("opening edit on card B while card A is mid-edit cancels card A's edit"). Keep `editingCardId`, and the new `confirmingDeleteId`, centralized in `App()`, passed down as `isEditing={() => editingCardId() === macro.id}` style accessor props.

**Example:**
```tsx
// Source: SolidJS docs (props.md) — never destructure; splitProps/mergeProps preserve reactivity
// src/components/MacroCard.tsx
import type { Component } from "solid-js";
import type { MacroConfig, AppState } from "../types"; // hoisted from App.tsx

interface MacroCardProps {
  macro: MacroConfig;
  state: AppState;
  isEditing: () => boolean;
  isConfirmingDelete: () => boolean;
  onEditStart: (id: string) => void;
  onEditCancel: () => void;
  onDeleteStart: (id: string) => void;
  onDeleteConfirm: (id: string, name: string) => void;
  onDeleteCancel: () => void;
}

const MacroCard: Component<MacroCardProps> = (props) => {
  // Read via props.macro, props.isEditing() — NEVER `const { macro } = props`
  // (destructuring at the top would capture the value once instead of
  // re-reading the getter on every reactive update — see splitProps docs)
  return (
    <div class="glass-card p-4">
      {/* ... */}
    </div>
  );
};
export default MacroCard;
```

### Pattern 2: Boot-time theme resolution (no flash of wrong theme)

**What:** A synchronous read of `localStorage` + `matchMedia` that sets `data-theme` on `<html>` BEFORE SolidJS mounts, so the first paint is already correct.

**When to use:** Once, at app boot — this is the mechanism the UI-SPEC's "Persistence" section requires ("read on boot before first paint to avoid a flash of the wrong theme").

**Example:**
```html
<!-- Source: standard FOUC-prevention pattern, index.html <head>, before the stylesheet link -->
<script>
  (function () {
    var pref = localStorage.getItem("automux.theme_preference") || "system";
    var resolved = pref === "system"
      ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
      : pref;
    document.documentElement.setAttribute("data-theme", resolved);
  })();
</script>
```
```ts
// src/theme.ts — the runtime module used after mount for the live toggle +
// the matchMedia 'change' listener while preference === "system"
export type ThemePreference = "system" | "light" | "dark";
const KEY = "automux.theme_preference";

export function getStoredPreference(): ThemePreference {
  const v = localStorage.getItem(KEY);
  return v === "light" || v === "dark" || v === "system" ? v : "system";
}

export function resolveTheme(pref: ThemePreference): "light" | "dark" {
  if (pref !== "system") return pref;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyTheme(pref: ThemePreference) {
  document.documentElement.setAttribute("data-theme", resolveTheme(pref));
  localStorage.setItem(KEY, pref);
}
```

### Pattern 3: `@theme` + `[data-theme]` for Tailwind v4 dual-mode tokens

**What:** Tailwind v4's officially documented pattern for attribute-driven (not class-driven, not pure `prefers-color-scheme`) theming.

**Example:**
```css
/* Source: tailwindcss.com/docs/dark-mode + tailwindcss.com/docs/colors [CITED] */
/* src/App.css */
:root {
  /* dark values = current @theme block values, moved here unchanged */
  --color-background: #0a0a0f;
  --color-surface: #12121a;
  /* ...rest of the existing dark token set... */
}

[data-theme="light"] {
  --color-background: #f4f4f8;
  --color-surface: #ffffff;
  /* ...rest of the UI-SPEC's light token set... */
}

@theme inline {
  --color-background: var(--color-background);
  --color-surface: var(--color-surface);
  /* ...maps every existing bg-*/text-* utility to the swappable custom property... */
}
```
This is the exact mechanism UI-SPEC's Theme System section describes ("`data-theme='light'|'dark'` attribute... a new `[data-theme='light'] { ... }` override block") — confirmed against official Tailwind v4 docs rather than assumed.

### Anti-Patterns to Avoid

- **`backdrop-filter` on every item in an unbounded scrollable list:** general web-perf guidance is explicit that backdrop-filter belongs on "static or rare elements (modals, navbars)... not lists of cards" [CITED: multiple perf sources, LOW individually but consistent across sources]. UI-SPEC's own documented fallback (reduce blur to sidebar + banners only) exists precisely because this is a known risk for AutoMux's macro-card list specifically — measure before assuming the full-blur-everywhere version is safe.
- **Giving `transparent: true` a try "just to see":** confirmed to actively degrade the CSS blur this phase depends on (Windows WebView2) — this is not a neutral experiment, per the GitHub issues cited above.
- **New per-card local edit-state instead of centralized `editingCardId`:** breaks the existing mutual-exclusion invariant silently (no compile error, no test failure — just two cards editable simultaneously in the running app).
- **Sequential per-field IPC calls for a single "Save Changes" click:** works today for single-field inline edits (target app, trigger key) but multiplies to a disk-write storm and a partial-apply risk once 4-5 fields change in the same submit — use the single `Intent::UpdateMacro` instead.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Modifier-key capture UI | A new key-picker widget for the Key Press action-type slot | Reuse `startCapture()` + the existing modifier-chip rendering (`App.tsx:532-567`, `172-191`) — wrap in `KeyCaptureField.tsx` | D-12 explicitly mandates this; the existing widget already handles Escape-to-cancel, modifier bit computation, and platform (macOS/Windows) bit mapping correctly |
| Trigger-key conflict detection for the new `UpdateMacro` intent | A parallel conflict-check implementation inside the new intent handler | Reuse `resolve_trigger_key_update()` / `check_trigger_key_conflict()` (`state/mod.rs:226-262`) — both are already free functions, already unit-tested, already handle self-rebind correctly | Duplicating this logic risks the two conflict-checkers drifting apart over time (e.g. `BindHotkey`'s check gets a bugfix, `UpdateMacro`'s copy doesn't) |
| Dark/light theme switching mechanism | A JS-driven class-toggle + manual re-render of every color usage | Tailwind v4's `@theme` custom-property + `[data-theme]` attribute pattern — CSS custom properties cascade automatically, zero component re-renders needed | This is Tailwind v4's documented first-class mechanism [CITED: tailwindcss.com/docs/colors]; a JS-driven approach would need to touch every one of the dozens of `bg-*`/`text-*`/`border-*` utility usages across the file |
| Tooltip/hover-reveal for truncated long macro names | A custom tooltip component/library | Native `title="{full value}"` attribute + Tailwind `truncate` | UI-SPEC already specifies this (Verification Checklist item, `overflow` UI-consideration row) — zero-dependency, works identically across WKWebView and WebView2 |

**Key insight:** Every "don't hand-roll" item above has an existing, already-correct implementation somewhere in this same codebase or the framework's own docs. The pattern for this specific phase is not "avoid building custom things" in the abstract — it's "don't re-derive logic that Phase 8/9 already got right" (conflict detection, key capture) and "don't fight the framework's built-in mechanism" (Tailwind theming).

## Common Pitfalls

### Pitfall 1: `.glass-card`'s existing `backdrop-filter: blur(12px)` is already dead code, visually
**What goes wrong:** A developer might assume the current build has zero `backdrop-filter` cost because "the design doesn't look blurred" — and be surprised when adding real translucency changes the perf profile even though `backdrop-filter` was technically already declared.
**Why it happens:** `App.css:78-81` declares `background: linear-gradient(...)` (fully opaque, no alpha) AND `backdrop-filter: blur(12px)` on the same rule. Blur-behind-an-element only has a visible (and possibly a compositing) effect when there's something to see through — a 100%-opaque background defeats it visually even though the property is present in the stylesheet.
**How to avoid:** Don't treat "v1.2.0 has no backdrop-filter" as a starting assumption when reasoning about the UI-04 perf delta — the property already exists; what changes is whether the background becomes translucent (`color-mix(..., 72%, transparent)`), which is the actual behavioral change to measure.
**Warning signs:** A perf comparison that assumes the "before" state has zero GPU compositing layers for `.glass-card` may understate the true baseline.

### Pitfall 2: `transparent: true` window config degrades CSS blur on Windows WebView2
**What goes wrong:** If a future iteration (or an over-eager implementation) flips `transparent: true` to "get real vibrancy," the CSS `backdrop-filter` blur this phase depends on for D-03 gets visually worse on Windows, not better.
**Why it happens:** Confirmed via multiple `tauri-apps/tauri` GitHub issues (#12437 "Inconsistent backdrop-blur Effect on Transparent Window", #2827, #2976) — WebView2's compositing behavior with a transparent host window interacts poorly with `backdrop-filter`. As of this research, issue #12437 is open/unresolved (`status: needs triage`).
**How to avoid:** Keep `transparent: false` (already UI-SPEC's D-04 resolution) — this pitfall is pre-empted by the locked decision, but worth documenting so nobody "fixes" it later without checking this research first.
**Warning signs:** If `tauri.conf.json`'s `transparent` field is ever changed to `true`, the blur intensity should be re-verified on a real Windows device, not assumed to improve.

### Pitfall 3: Stacking `backdrop-filter` across an unbounded macro list
**What goes wrong:** UI-04's "no measurable idle overhead" constraint could be violated for users with many macros (each one is a `.glass-card` with its own blur layer, scrolling triggers repaint of every visible one).
**Why it happens:** General `backdrop-filter` guidance (consistent across multiple independent sources, though each individually LOW confidence) says each element with the property gets its own compositing layer, and scrolling repaints the backdrop on every frame for elements whose blur region moves.
**How to avoid:** UI-SPEC already documents the fallback (blur only sidebar + topmost banners, not every card) — the planner should schedule the UI-04 perf measurement EARLY (e.g. end of the first frontend-touching plan/wave) rather than as a final gate, so there's still time to apply the fallback without re-touching every already-built surface.
**Warning signs:** Noticeable scroll jank with 10+ macros in the list on a lower-end Windows machine (Intel integrated graphics) — this is a scenario worth testing against, not just Apple Silicon.

### Pitfall 4: Missing backend Intent for `name` and `trigger_mode` edits
**What goes wrong:** A plan that assumes "the backend already supports per-field macro edits" (reasonable given how much of the Intent enum already exists) will discover mid-implementation that `name` and `trigger_mode` have literally no update path — they're `AddMacro`-only fields today.
**Why it happens:** The Intent enum grew incrementally across Phases 8/9 to solve specific problems (hotkey conflicts, target app, sequence/interval updates) — none of those phases needed to rename a macro or flip its Pulse/Hold mode after creation, so no intent was ever added for it.
**How to avoid:** Add `Intent::UpdateMacro` (see Code Examples) in the same plan/wave as the frontend edit form — don't split "backend edit support" and "frontend edit form" across plans expecting the backend half to already exist.
**Warning signs:** Searching `state/mod.rs` for `mac.name =` or `mac.trigger_mode =` outside of `AddMacro`'s handler returns zero results (verified during this research).

### Pitfall 5: SolidJS prop destructuring silently breaks reactivity during decomposition
**What goes wrong:** Extracting `MacroCard` (or any other component) from `App.tsx` and writing `const { macro, isEditing } = props;` at the top compiles fine and often *looks* correct in a quick manual test, but the extracted values stop updating when the parent's signal changes later.
**Why it happens:** SolidJS props are plain object getters, not reactive proxies re-evaluated per-render (there is no per-render re-execution in SolidJS's compiled-fine-grained model) — destructuring reads the getter once, at component-creation time, and that's the only read that ever happens [CITED: docs.solidjs.com/reference/reactive-utilities/split-props].
**How to avoid:** Access via `props.macro`, `props.isEditing()` inline in JSX/reactive scopes; use `splitProps`/`mergeProps` if a subset needs to be forwarded to a further-nested component.
**Warning signs:** A card shows stale data after an edit/toggle that should have updated it, but `get_state`/refresh on tab-switch "fixes" it — that refresh-fixes-it symptom is the signature of a destructured prop.

## Code Examples

### Rust: `Intent::UpdateMacro` (new, for UX-09)

```rust
// Source: derived from the existing SetMacroTriggerKey / BindHotkey pattern
// in src-tauri/src/state/mod.rs (Result-carrying oneshot, conflict pre-check
// before any mutation). Add to the Intent enum:

/// UX-09: Consolidated macro edit — updates name, action sequence,
/// trigger mode, target app, and trigger key/modifiers in one atomic
/// operation. The oneshot reply is `Err(msg)` ONLY for a trigger-key
/// conflict (reusing resolve_trigger_key_update); on Err, NO field is
/// mutated — the edit form stays open with the user's unsaved changes
/// intact, matching the Interaction Contract's "3alt" row in UI-SPEC.
UpdateMacro(
    Uuid,
    String,               // name
    ActionSequence,       // sequence — single-step, mirrors AddMacro's shape
    TriggerMode,
    Option<String>,       // target_app
    Option<u16>,           // trigger_key
    Option<u64>,           // trigger_modifiers
    tokio::sync::oneshot::Sender<Result<(), String>>,
),

// Handler (inside handle_intent's match):
Intent::UpdateMacro(id, name, sequence, trigger_mode, target_app, trigger_key, trigger_modifiers, reply) => {
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
}
```

### Rust: `update_macro` IPC command (new)

```rust
// Source: mirrors set_macro_trigger_key's Result-unwrap pattern in
// src-tauri/src/ipc/mod.rs
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

### TypeScript: building the `ActionSequence` from the Input+Mode selectors (mirrors existing `handleCreateMacro`)

```typescript
// Source: pattern already established in App.tsx:460-481 (handleCreateMacro) —
// the edit form must build the SAME shape, not a novel one.
function buildActionSequence(input: InputEvent, mode: TriggerMode, intervalMs: number): ActionSequence {
  return mode === "Hold"
    ? { steps: [{ SustainedHold: { input } }] }
    : { steps: [{ InterleavedInterval: { input, interval_ms: intervalMs } }] };
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `.glass-card` gradient-only "glass" look (opaque background + inert `backdrop-filter`) | Real translucency via `color-mix()` + functioning `backdrop-filter` | This phase (D-03) | First time the app's blur declaration has a visible effect — perf profile must be re-measured, not assumed unchanged |
| Top tab bar (`Dashboard | Profiles`) | Persistent left sidebar rail | This phase (D-08) | `activeTab()` signal/pattern unchanged — only the nav chrome changes from `<button>` row to `<button>` column |
| `window.confirm()` for delete | Inline in-card confirmation state | This phase (D-14) | Native dialogs are being phased out of this app entirely — no other native dialogs remain after this phase |
| Dark-only | System-following light/dark with manual override | This phase (D-06/D-07) | First theme-variant work in the project — establishes the `@theme`/`[data-theme]` pattern other future work should follow |

**Deprecated/outdated in this codebase after this phase:**
- `window.confirm()` usage pattern (was `App.tsx:520`) — fully removed, no remaining native-dialog call sites.
- Ad-hoc `text-[10px]`/`text-[11px]` inline pixel sizes scattered through the file — UI-SPEC consolidates to exactly 4 typography roles; any inline size outside `20/15/13/11px` after this phase is a spec violation, not a stylistic choice.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Making `.glass-card`'s background translucent (rather than merely having `backdrop-filter` declared on an opaque background) is what actually triggers additional GPU compositing cost — i.e., browsers likely skip meaningful blur compositing work when there's nothing translucent to blend through | Pitfall 1 / Summary | If wrong (browsers always pay the full compositing cost regardless of background opacity), the "existing baseline already includes this cost" framing is incorrect and the true UI-04 delta could be larger than this research suggests — does not change the recommended action (measure empirically either way) but affects how surprised the executor should be by the measurement |
| A2 | The `tauri-apps/tauri` GitHub issues cited (#12437, #2827, #2976) describing `transparent:true` degrading `backdrop-filter` on WebView2 still reflect current Tauri 2.x behavior | Common Pitfalls #2, Alternatives Considered | If Tauri has since fixed this, D-04's "CSS-only is safer" justification loses one of its two supporting arguments (the Acrylic/Blur resize-perf warning from Tauri's own docs still stands independently) — low risk since D-04 is already locked for other valid reasons too |
| A3 | Browsers create a new GPU compositing layer per `backdrop-filter` element and repaint on scroll — general web-performance claim, not verified specifically against Tauri's embedded WKWebView/WebView2 builds | Pitfall 3, Don't-Hand-Roll table | If this doesn't hold in the embedded webview engines specifically, the "avoid backdrop-filter on every list card" caution may be overly conservative — the UI-SPEC's own fallback plan already accounts for this being possibly-necessary, so no action changes, just the urgency |

**If measurement contradicts A1/A3:** the fallback path (reduce blur layer count) is already fully specified in `10-UI-SPEC.md`'s Translucency section — no new design work needed, just apply it.

## Open Questions

1. **Does the perf delta actually materialize on real hardware?**
   - What we know: `backdrop-filter` is inherently GPU-cost-bearing per general web guidance; this codebase's exact before/after delta has not been measured (can't be, in a research pass — needs a running build on real devices).
   - What's unclear: Whether AutoMux's specific card count (typically small, per its target audience) and window size (720×680, not full-screen) keep the cost negligible in practice.
   - Recommendation: Schedule the UI-04 perf measurement (Activity Monitor on macOS / Task Manager on Windows, idle app, no macro running, no window movement, compare against a same-session v1.2.0 build) as an early gate in the plan sequence — not the final verification step — so the documented fallback (fewer/larger blur surfaces) can be applied before downstream plans build visual assumptions on top of full-card blur.

2. **Should `Intent::UpdateMacro` validate `name` non-empty at the backend, or trust the frontend's existing `disabled={!newMacroName().trim()}` guard?**
   - What we know: `AddMacro`'s handler does not itself validate name non-emptiness — it trusts the frontend's disabled-button guard (`App.tsx:1225`).
   - What's unclear: Whether the edit form's `Save Changes` button should mirror that same client-only guard, or whether the new intent is a good opportunity to add a defensive backend check.
   - Recommendation: Mirror existing precedent (client-only guard) for consistency — this is a UI-only phase per CONTEXT.md's domain statement ("no scheduler/backend behavior changes" beyond what's strictly needed for UX-09), and adding new backend validation semantics is a scope question for the planner/user, not something this research should silently decide.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Node.js | Frontend build (Vite) | ✓ | v26.5.0 | — |
| npm | Package management | ✓ | 11.17.0 | — |
| Rust (cargo/rustc) | Backend build | ✓ | 1.97.1 | — |
| `@tauri-apps/cli` | Dev server, bundling | ✓ | 2.11.4 (registry-current) | — |

No missing dependencies — this phase requires no new external tools beyond the existing verified toolchain.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust backend, inline `#[cfg(test)] mod tests` blocks — see `state/mod.rs:971-1382`) — **no frontend test framework exists** in this project |
| Config file | none — Cargo's built-in test harness, no separate config |
| Quick run command | `cargo test -p automux update_macro` (once the new tests below are added) |
| Full suite command | `cargo test` (backend) + `npx tsc --noEmit` (frontend type-check, existing gate) + manual visual QA against `10-UI-SPEC.md`'s Verification Checklist |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UX-09 | `Intent::UpdateMacro` atomically updates name/sequence/mode/target/key on success | unit | `cargo test update_macro_applies_all_fields` | ❌ Wave 0 |
| UX-09 | `Intent::UpdateMacro` rejects on trigger-key conflict WITHOUT mutating any field (atomicity) | unit | `cargo test update_macro_conflict_no_partial_mutation` | ❌ Wave 0 |
| UX-09 | Edited macro persists across restart | integration (existing pattern) | `cargo test profile_backwards_compat`-style round-trip via `ProfileManager` | ❌ Wave 0 (new test, same style as existing) |
| UX-10 | Input selector shows exactly 4 labeled options, Mode selector exactly 2 | manual/visual | N/A — no frontend test framework; covered by UI-SPEC's own Verification Checklist item | manual-only, justified: no test infra exists for JSX option text in this project |
| UI-01/02/03 | Visual redesign correctness (translucency, sidebar, typography) | manual/visual | N/A | manual-only, justified: this is a pure visual-design phase; `10-UI-SPEC.md`'s 20-item executor-facing Verification Checklist already serves as the acceptance-criteria list |
| UI-04 | No measurable idle CPU/GPU overhead vs v1.2.0 | manual measurement | N/A (Activity Monitor / Task Manager, documented protocol) | manual-only, justified: no automated perf-profiling tooling exists in this project's CI |
| UX-08 | Delete restyle — no regression to existing `remove_macro` IPC behavior | existing coverage | (no new backend logic — UX-08's backend half was already covered by Phase 9) | ✓ (unchanged) |

### Sampling Rate
- **Per task commit:** `cargo test -p automux <new test name>` (fast, scoped) + `npx tsc --noEmit`
- **Per wave merge:** `cargo test` (full backend suite) + `npx tsc --noEmit` + spot-check against the relevant rows of UI-SPEC's Verification Checklist
- **Phase gate:** Full backend suite green + `npx tsc --noEmit` clean + the UI-04 perf measurement documented with before/after numbers + full UI-SPEC Verification Checklist walked

### Wave 0 Gaps
- [ ] New `#[cfg(test)] mod tests` additions inside `state/mod.rs` (same file, following the existing inline-test convention — do NOT create a separate test file, that would break from the established pattern) covering: successful `UpdateMacro` atomic apply, conflict-rejection-without-mutation, and a persistence round-trip
- [ ] No frontend test framework exists and none is being added this phase — the Wave 0 gap for UI-01/02/03/UX-10 is "none, by design" (manual QA via UI-SPEC's own checklist is the sampling mechanism)

*(Backend gap: 3 new unit tests needed, same file, same style as existing `hold_release_bypasses_gates`/`set_trigger_key_rejects_conflict_without_coercion` tests. Frontend: no gap — manual visual QA was already the established mechanism pre-this-phase, per `CONVENTIONS.md`'s "No frontend test framework detected.")*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Single-user local desktop app, no auth surface — unaffected by this phase |
| V3 Session Management | No | No session concept in this app |
| V4 Access Control | No | No multi-user/role concept |
| V5 Input Validation | Marginal | New `update_macro` command's `name: String` param is deserialized via `serde` (same as every existing `MacroConfig` field) — Tauri's IPC layer already enforces type-correctness at the deserialization boundary; no new validation gap is introduced beyond what `AddMacro` already accepts today (i.e., an empty/malformed name is already possible via `AddMacro` if the frontend guard is bypassed — this phase doesn't change that risk posture) |
| V6 Cryptography | No | No cryptographic operations touched by this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed/oversized `name` string sent via `update_macro` IPC | Denial of Service (minor — local process only) | Not a new risk this phase introduces (identical exposure already exists via `add_macro`) — Tauri's local-IPC-only surface (no network exposure, `connect-src ipc:` CSP) means the threat model is "a malicious local process abusing IPC," which is out of scope for a desktop automation tool's threat model and unchanged by this phase |
| Trigger-key conflict logic bypass (edit path skips the conflict check) | Tampering | Directly mitigated by reusing `resolve_trigger_key_update()` in the new intent rather than writing a parallel, potentially-incomplete check — see Don't Hand-Roll table |

This phase is UI/UX-surface-only per its own domain statement (CONTEXT.md: "No new automation capabilities, no scheduler/backend behavior changes... this phase is UI/UX surface and interaction only") — the one backend change (`Intent::UpdateMacro`) is a same-shape extension of an existing, already-reviewed mutation pattern, not a new attack surface category.

## Sources

### Primary (HIGH confidence)
- Direct source reading: `src/App.tsx`, `src/App.css`, `src/keymap.ts`, `src-tauri/src/state/mod.rs`, `src-tauri/src/ipc/mod.rs`, `src-tauri/tauri.conf.json`, `package.json` — all claims about existing code structure, the `Intent` enum's current coverage, and the `.glass-card` opaque-background-defeats-blur observation are verified directly against this repository's current state, not inferred.
- `10-UI-SPEC.md` (this project, checker-approved 2026-07-23) — the binding design contract this research implements against.
- `10-CONTEXT.md` (this project, 2026-07-23) — the locked user decisions.

### Secondary (MEDIUM confidence)
- Context7 `/websites/tauri_app` (Tauri 2 official docs) — `Effect` enum platform notes (Acrylic/Blur resize-perf warnings), `transparent`/window-effects config requirements, macOS transparent-titlebar customization pattern via `objc2-app-kit`. [CITED]
- Context7 `/tailwindlabs/tailwindcss.com` (official Tailwind CSS docs) — `@custom-variant` dark-mode override, `[data-theme]` attribute pattern, `@theme inline` + `:root`/`[data-theme="dark"]` color-token switching pattern. [CITED]

### Tertiary (LOW confidence — corroborated by consistency across multiple independent sources, not by an authoritative single source)
- WebSearch: `tauri-apps/tauri` GitHub issues #12437, #2827, #2976 — `transparent: true` degrading `backdrop-filter` quality on Windows WebView2. Multiple independent issue reports describing the same symptom raises confidence above a single anecdotal report, but this is still community bug-tracker evidence, not official documentation of the behavior.
- WebSearch: general `backdrop-filter` GPU-compositing-cost and best-practices guidance (compositing layer per element, scroll repaint cost, keep blur under ~20px, avoid stacking, prefer static/rare elements over list items). Consistent across several independent web sources but none individually authoritative (browser vendor performance docs were not directly queried this session).
- WebSearch: SolidJS prop-destructuring-breaks-reactivity guidance — corroborated by the official `docs.solidjs.com/reference/reactive-utilities/split-props` page reference surfaced in search results, treated as CITED-equivalent for that specific claim.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new dependencies, all version numbers verified directly against the npm registry
- Architecture (component decomposition, Intent extension): HIGH — derived directly from reading the actual current source, not assumed
- Translucency/perf mechanics: MEDIUM for the mechanism (Tailwind v4 pattern is officially documented), LOW for the specific magnitude of any perf cost (inherently requires on-device measurement, which this research pass cannot perform)
- Pitfalls: HIGH for pitfalls 1, 4, 5 (verified against actual source); MEDIUM for pitfalls 2, 3 (corroborated external evidence, not first-party verified in this repo)

**Research date:** 2026-07-23
**Valid until:** 30 days (stable stack, no fast-moving dependencies) — EXCEPT the Tauri WebView2/`transparent` interaction claim (Pitfall 2 / Assumption A2), which should be re-checked if Tauri is upgraded past its current `^2` pin before this phase is implemented, since that specific GitHub issue was open/unresolved at research time and could be patched upstream.
