# Phase 10: UI Redesign & Macro Management - Context

**Gathered:** 2026-07-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Ship a full UI redesign for AutoMux — a single, Raycast-inspired, modern glass-style aesthetic used identically on both macOS and Windows (not literal macOS 26 Liquid Glass, no OS-version tiering) — plus macro management: users can delete (already shipped in Phase 9) and edit existing macros, with unambiguous action-type labeling (Left Click / Right Click / Hold / Key Press). No new automation capabilities, no scheduler/backend behavior changes — this phase is UI/UX surface and interaction only, on top of the existing StateActor/Scheduler architecture.

Requirements: UI-01, UI-02, UI-03, UI-04, UX-08 (delete — already shipped, verify it fits the new design), UX-09 (edit — net new), UX-10 (action-type labeling).

</domain>

<decisions>
## Implementation Decisions

### Cross-platform visual identity (supersedes ROADMAP's literal "liquid glass on macOS 26 / modern equivalent on Windows" framing)

- **D-01:** One unified custom visual design is used on **both Windows and macOS, across all supported OS versions** — no OS-version tiering, no macOS-26-only glass tier with a lesser fallback elsewhere. This is a deliberate reinterpretation of UI-01/UI-03's roadmap language: the user wants a single Raycast-inspired look everywhere, not the native macOS 26 Liquid Glass API gated to Tahoe. — **Reversibility:** costly — a later split into OS-specific tiers would mean re-deriving two design surfaces from one and is a meaningfully different visual QA matrix.
- **D-02:** The redesign adopts the "modern liquid-glass-era" design language broadly (translucent panels, blur, sleek rounded elements) wherever it fits AutoMux's existing macro-list-centric layout — but it is **not a literal Raycast clone**. Skip specific Raycast UI patterns (e.g., a command-bar-style search) if they aren't the best fit for this app's actual content (a list of macros, not a command launcher).
- **D-03:** Translucency uses **real CSS `backdrop-filter: blur(...)`** — genuine translucency, not the current `.glass-card` gradient-only treatment (which has no actual blur). Planner/researcher must verify this doesn't violate UI-04 (no measurable idle overhead vs. v1.2.0) — `backdrop-filter` has a known GPU compositing cost, worth a quick perf check.
- **D-04 (Claude's discretion):** Whether the **app window itself gets true OS-level vibrancy** (`transparent: true` in `tauri.conf.json` + native window-effect APIs — NSVisualEffectView on macOS, Acrylic/Mica backdrop on Windows) or stays **opaque with blur only between internal panels** (CSS-only, cross-platform, no native window-effect code). Current config: `decorations: true`, `transparent: false`. Weigh authenticity vs. implementation effort and the UI-04 constraint — CSS-only is the lower-risk default absent a strong reason to go native.
- **D-05 (Claude's discretion):** Accent color — may keep the existing indigo (`--color-accent: #6366f1`) or introduce a new one, whichever fits the Raycast-inspired direction best.
- **D-06:** Add a **light/dark theme toggle** — AutoMux is dark-only today; this phase adds light theme support. This is new scope beyond a pure re-skin. — **Reversibility:** costly — every new/redesigned component needs both a light and dark treatment; reverting to dark-only later means auditing and stripping light-mode styles across the whole redesigned surface.
- **D-07:** The theme **defaults to following the OS system light/dark preference**, with a manual override available to the user.

### Navigation & layout structure

- **D-08 (Claude's discretion):** Whether to restructure navigation (e.g., a list+detail split view, or a persistent sidebar) or keep the current two-tab (Dashboard/Profiles) structure re-skinned with the new aesthetic. Pick based on what best fits AutoMux's content density and the chosen window size (D-09). Note: a list+detail split would also naturally host the inline macro-edit surface (D-12).
- **D-09 (Claude's discretion):** Whether to keep the current compact widget footprint (420×640, min 360×480) or widen the window toward Raycast-like proportions. Decide based on which layout answer above (D-08) needs the space — e.g., a list+detail split likely needs more width than the current narrow window allows.

### Action-type model (create & edit)

- **D-10 (Claude's/researcher's discretion):** Whether the 4 action types required by UX-10 (Left Click / Right Click / Hold / Key Press) become **one unified "action type" selector**, or stay as **two selectors** (input type: click/key: and trigger mode: Pulse/Hold) with clearer labels plus the currently-missing Key Press option added to the input selector. Decide based on how cleanly the backend's `ActionStep`/`TriggerMode` model (in `src-tauri/src/state/mod.rs`) maps to a single unified UI concept — `Hold` can currently combine with any input type, so a unified selector needs a clear interaction design for that combination, not just a flat 4-item list.
- **D-11:** **"Key Press" is currently not selectable anywhere in the macro creation form** — the backend already supports `InputEvent::Key` (see `formatInputEvent` in `App.tsx:65-68`, which already renders `⌨ Key({ev.Key})` for conflicts/steps), but the creation form's input dropdown (`App.tsx:1111-1113`) only offers Left/Right/Middle mouse buttons. Phase 10 must add Key Press as a genuinely selectable action type in both create and edit — this is new UI surface, not just relabeling.
- **D-12:** Key-press action capture **reuses the existing hotkey trigger-key capture widget** (the one with modifier chips, already shipped in Phase 8 for hotkey binding) rather than building a distinct, simpler key picker.

### Edit interaction & delete confirmation

- **D-13:** Editing an existing macro happens via **inline expand-in-place** on the macro card itself — the card expands into an editable form, reusing the same fields/components as the macro creation form. No modal/overlay pattern is introduced (AutoMux has no modals today).
- **D-14:** Delete's current plain `window.confirm()` (added in Phase 9 Plan 8, `App.tsx:520`) is **replaced with a custom-styled in-app confirmation** matching the new glass aesthetic — a native OS dialog would look jarring in an otherwise fully custom-styled UI. Exact form (inline "are you sure?" card state vs. small styled dialog) is left to the planner, consistent with D-13's no-modal preference (an inline confirm-in-place state is the natural fit).

### Claude's Discretion summary

Items where the user said "you decide":
- D-04 — OS-level window vibrancy vs. CSS-only internal blur
- D-05 — Accent color
- D-08, D-09 — Navigation restructure and window sizing
- D-10 — Unified vs. dual action-type selector

Rationale for the major discretion picks:
- **CSS-only blur as the safer default (D-04):** true native window vibrancy touches platform-specific Rust/window-chrome code on both macOS and Windows and risks UI-04's no-overhead constraint; CSS-only achieves the visual goal (D-03) with zero native surface.
- **List+detail as the layout to evaluate first (D-08):** it directly serves both the navigation-structure goal and the inline-edit decision (D-13) with one architectural change, rather than solving them independently.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project & requirements
- `.planning/PROJECT.md` — Core value, v2.0 milestone goals, UI-01/UI-04 constraints (liquid glass requires macOS 26+; UI redesign must not increase idle overhead)
- `.planning/REQUIREMENTS.md` — UI-01, UI-02, UI-03, UI-04, UX-08, UX-09, UX-10 acceptance criteria
- `.planning/ROADMAP.md` §Phase 10 — phase goal and success criteria (note: D-01 reinterprets the literal "liquid glass on macOS 26 / modern equivalent on Windows" framing per user direction — single unified look everywhere)

### Architecture & conventions
- `.planning/codebase/ARCHITECTURE.md` — StateActor/Scheduler pattern (unaffected by this phase — UI-only work); frontend is a read-only projection of `AppState`
- `.planning/codebase/CONVENTIONS.md` — SolidJS patterns (`createSignal`, `createEffect`, `Show`/`For`, `handle*` naming), Tailwind-only styling convention (no `className`, no component libraries), no path aliases
- `.planning/codebase/STRUCTURE.md` — single-file `src/App.tsx` component model; "No separate component files currently" — planner must decide whether this phase's scale forces decomposition into multiple files (an implementation decision, not covered by this discussion)

### Prior phase context (must reconcile, do not duplicate)
- `.planning/phases/08-hotkey-reliability-conflict-safety/08-UI-SPEC.md` — the CURRENT design system this phase supersedes: exact color tokens (`--color-accent #6366f1`, `--color-success`, `--color-warning`, `--color-danger`, etc.), spacing scale (4/8/16/24/32/48/64px), typography scale, and 5 UI components (ConflictErrorToast, ConflictWarningRegion, FirstRunGlobalNotice, in-card Global subtitle, ModifierPreviewChip) that MUST be preserved/migrated into the new visual language, not dropped. Also documents the existing key-capture widget referenced by D-12.
- `.planning/phases/09-parallel-macro-execution/09-CONTEXT.md` — D-01/D-02 running-state visual indicators (firing/waiting/held/combined dot states) shipped in Phase 9 — these behaviors must carry into the redesigned macro card, just re-skinned.

### Implementation files (must read for context)
- `src-tauri/tauri.conf.json` — current window config: `decorations: true`, `transparent: false`, 420×640 (min 360×480) — the baseline D-04/D-09 decisions modify
- `src/App.tsx` — entire current UI (1601 lines): macro creation form (`~1090-1170`), action-type/input dropdowns (`1111-1113`, `1150-1159`), macro card rendering, `handleRemoveMacro` (delete, `519-...`, uses `window.confirm` at line 520), `formatInputEvent`/`InputEvent` type (`32-40`, `65-68`) — shows `Key` variant already modeled but unexposed in the create form (D-11)
- `src/App.css` — current CSS custom properties (`@theme` block) and `.glass-card`/`.toggle-track`/`.status-pulse` classes — the starting point for the new design tokens

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Hotkey trigger-key capture widget** (`App.tsx`, used for macro hotkey binding since Phase 8) — has modifier-chip display, keydown capture, Escape-to-cancel. D-12 reuses this directly for the new Key Press action-type capture.
- **`formatInputEvent`/`InputEvent` type** (`App.tsx:32-40, 65-68`) — already models `{ Key: number }` alongside `{ MouseButton: ... }`; the type system is ready for Key Press, only the creation/edit form UI needs to expose it (D-11).
- **`.glass-card`, `.toggle-track`, `.toggle-thumb`, `.status-pulse`** (`App.css:67-119`) — existing "glass" visual vocabulary (gradient-only, no real blur) that the new `backdrop-filter`-based treatment (D-03) replaces/extends.
- **Existing message/toast pattern** (`profileMessage` signal with type discriminant + `setTimeout` auto-dismiss, per CONVENTIONS.md) — a reusable pattern for the new delete-confirmation UI (D-14) if an inline confirm-state approach is chosen over a dialog.

### Established Patterns
- **Single-file component model** — all UI lives in `App.tsx`; CONVENTIONS.md explicitly notes "No separate component files currently." A redesign at this scale is a natural inflection point for decomposition, but that's an implementation/architecture call for the planner, not a user-vision decision captured here.
- **No modal library, no component library** — SolidJS project, Tailwind-only styling, no Radix/Ark/Base UI. D-13's inline-expand and D-14's inline-confirm decisions are consistent with keeping this true (no new dependency needed for either).
- **`#[serde(default)]` backwards-compat convention** (Rust side) — if any new macro fields are needed for edit support (unlikely — edit should reuse existing `MacroConfig` fields via `UpdateMacro`-style intent), follow this convention for schema evolution.

### Integration Points
- **`Intent` enum in `src-tauri/src/state/mod.rs`** — UX-09 (edit) likely needs a new `Intent::UpdateMacro` (or similar) variant if one doesn't already exist; check current IPC command list (`add_macro`, `remove_macro`, `set_macro_enabled`, `set_macro_target_app`, `set_macro_sequence`, `update_step_interval`) for what's already editable field-by-field vs. what needs a new consolidated update path.
- **Macro card rendering block in `App.tsx`** — the insertion point for D-13's inline-expand edit form, mirroring how the "New Macro" form already expands/collapses.

</code_context>

<specifics>
## Specific Ideas

### Concrete examples from the discussion

- User's own words on the aesthetic direction: *"I want a raycast aesthetic, modern, sleek, sort of like apple liquid glass but not actually liquid glass, i want the app to look the same on windows and mac, like raycast which is on both windows and mac and looks roughly the same."*
- Refinement on homage level: *"Only what makes sense, so maybe no command bar style search if it isn't necessary or the best solution. But for everything that does make sense yes. not a clone but the same design language etc (the modern liquid-glass era design that a lot of modern sleek apps are adopting)"*

### No specific component library or icon set requested

The user did not name a specific icon library or component kit. Current convention (per 08-UI-SPEC.md) is inline emoji/glyphs only, no icon library — no signal this changes for Phase 10, but not explicitly re-confirmed either.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope. No user-suggested features were redirected to the backlog; all four discussed areas resolved to specific decisions or explicit Claude's-discretion delegations.

</deferred>

---

*Phase: 10-ui-redesign-macro-management*
*Context gathered: 2026-07-23*
