---
phase: 10-ui-redesign-macro-management
plan: 02
subsystem: ui
tags: [css-custom-properties, tailwind-v4, theming, backdrop-filter, solidjs]

# Dependency graph
requires:
  - phase: 10-ui-redesign-macro-management (plan 01)
    provides: "Interim translucent .glass-card (color-mix background + functioning backdrop-filter) that this plan finalizes and extends"
provides:
  - "Dark + light token sets via :root / [data-theme=\"dark\"] alias / [data-theme=\"light\"] override / @theme inline mapping — the single mechanism every downstream visual plan (10-03 through 10-06) renders against"
  - "Finalized .glass-card and new .sidebar-glass translucency classes (D-03 foundation for the sidebar plan 10-03 builds)"
  - "src/theme.ts (getStoredPreference/resolveTheme/applyTheme) + index.html boot-time FOUC-prevention script — the runtime theme system plan 10-03's ThemeToggle control will drive"
  - "App.tsx themePreference signal + live matchMedia OS-follow effect, ready for the sidebar ThemeToggle to consume in plan 10-03"
affects: ["10-03 (sidebar + ThemeToggle control consumes themePreference/setThemePreference)", "10-04/10-05 (all typography/color utilities resolve against these tokens)", "10-06 (final polish/verification, including the UI-04 perf gate)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tailwind v4 attribute-driven theming: :root (dark default) + [data-theme=\"dark\"] alias + [data-theme=\"light\"] override + @theme inline mapping — swaps only the underlying custom-property values, all existing bg-*/text-*/border-* utilities keep working unchanged"
    - "Boot-time FOUC-prevention IIFE in index.html <head>, before any stylesheet link — resolves and applies data-theme before first paint"
    - "theme.ts sibling-module convention (mirrors keymap.ts): plain exported functions, no class, no default export"

key-files:
  created:
    - src/theme.ts
  modified:
    - src/App.css (token restructure, .sidebar-glass added)
    - index.html (boot-time theme script)
    - src/App.tsx (themePreference signal + matchMedia live-follow effect)

key-decisions:
  - "resolveTheme was NOT imported into App.tsx despite being named in the task's read_first/action guidance — App.tsx only needs applyTheme (which calls resolveTheme internally) and getStoredPreference; importing resolveTheme without a direct call site would fail strict noUnusedLocals. Kept the import list to exactly what's consumed."
  - "themePreference's setter (setThemePreference) is referenced via a bare `void setThemePreference;` statement (evaluates the reference, doesn't call it) to satisfy noUnusedLocals ahead of the plan 10-03 ThemeToggle control that will actually call it — same deferred-consumer precedent as Phase 8 Plan 4's conflictError/recordingModifiers void reads."
  - "requirements mark-complete was NOT run for UI-01/UI-03/UI-04 — this plan only lays the token/translucency foundation; the full redesign (sidebar, forms, Windows-equivalent surfaces) spans plans 10-03 through 10-06, and the UI-04 perf gate is explicitly deferred to plan 10-03 per this plan's own <verification> section. Marking these complete now would misrepresent phase progress in REQUIREMENTS.md, mirroring plan 10-01's identical decision for UX-09/UI-01."

requirements-completed: []

coverage:
  - id: D1
    description: "App.css restructured from a flat @theme block into :root (dark) + [data-theme=\"dark\"] alias + [data-theme=\"light\"] override + @theme inline mapping, with light-theme values matching the UI-SPEC table verbatim (including the darkened #4f46e5 accent) and no font-size outside the 20/15/13/11px scale"
    requirement: "UI-01"
    verification:
      - kind: other
        ref: "grep -c 'data-theme=\"light\"' src/App.css (1), grep -c '@theme inline' src/App.css (1), grep -c '--color-accent: #4f46e5' src/App.css (1), grep -oE 'font-size:[^;]+' src/App.css (0 matches)"
        status: pass
      - kind: other
        ref: "npx tsc --noEmit (exit 0)"
        status: pass
    human_judgment: true
    rationale: "Visual correctness of both theme token sets (no dark-only element visible in light mode, per D-06) requires spot-checking every card/banner/form field in a running app on a real display — this is explicitly called out as manual-only in RESEARCH.md's Validation Architecture (no automated visual-regression tooling exists in this project)."
  - id: D2
    description: ".glass-card finalized (color-mix 72% + blur(20px) saturate(150%)) and new .sidebar-glass class added (color-mix 60% + blur(24px)) per UI-SPEC Translucency section"
    requirement: "UI-01"
    verification:
      - kind: other
        ref: "grep -A5 '^\\.glass-card {' src/App.css shows color-mix + blur(20px) + saturate(150%); grep -c sidebar-glass src/App.css (1)"
        status: pass
    human_judgment: true
    rationale: "Real visible blur (content behind a panel visibly blurred, not a flat gradient) requires a running app on a real display, same as plan 10-01's D3 coverage entry for the interim .glass-card."
  - id: D3
    description: "src/theme.ts exports getStoredPreference (T-10-03 validation guard falling back to \"system\"), resolveTheme (matchMedia-based resolution), and applyTheme (sets data-theme + persists preference); index.html's boot-time IIFE applies the resolved theme before first paint, positioned before the Google Fonts stylesheet link"
    requirement: "UI-01"
    verification:
      - kind: other
        ref: "grep -c 'export function' src/theme.ts (3); grep -c 'automux.theme_preference' src/theme.ts index.html (1 each); npx tsc --noEmit (exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "App.tsx owns a themePreference signal seeded from getStoredPreference() and a createEffect that follows OS prefers-color-scheme changes live while preference is \"system\", with onCleanup removing the matchMedia listener"
    requirement: "UI-01"
    verification:
      - kind: other
        ref: "grep -c 'from \"./theme\"' src/App.tsx (1); grep -c 'prefers-color-scheme' src/App.tsx (2 — declaration + listener); npx tsc --noEmit (exit 0, strict noUnusedLocals satisfied)"
        status: pass
    human_judgment: true
    rationale: "The live-follow behavior (toggling OS appearance while preference is System flips the app theme without restart) requires a running app and OS-level appearance toggling — no frontend test framework and no OS-appearance-simulation tooling exist in this project, per RESEARCH.md's Validation Architecture."

duration: 10min
completed: 2026-07-24
status: complete
---

# Phase 10 Plan 02: Theme Token Foundation + Translucency Finalization Summary

**Dark/light theme system via Tailwind v4's `:root`/`[data-theme]`/`@theme inline` pattern, finalized `.glass-card` + new `.sidebar-glass` translucency, and a boot-before-paint `theme.ts` + `index.html` runtime with live OS-follow in `App.tsx`**

## Performance

- **Duration:** ~10 min
- **Completed:** 2026-07-24
- **Tasks:** 3/3
- **Files modified:** 4 (`src/App.css`, `src/theme.ts` [new], `index.html`, `src/App.tsx`)

## Accomplishments

- Restructured `App.css`'s single flat `@theme` block into a dark `:root` token set + an explicit `[data-theme="dark"]` alias + a `[data-theme="light"]` override block with the UI-SPEC's exact light values (including the darkened `#4f46e5` accent, kept indigo per D-05) + an `@theme inline` mapping block so every existing `bg-*`/`text-*`/`border-*` utility class keeps working unchanged
- Finalized `.glass-card` (already interim-translucent from plan 10-01, values unchanged: `color-mix(...72%, transparent)` + `blur(20px) saturate(150%)`) and added the new `.sidebar-glass` class (`color-mix(...60%, transparent)` + `blur(24px)`) per UI-SPEC's Translucency section
- Created `src/theme.ts` (mirrors `keymap.ts`'s sibling-module convention) exporting `getStoredPreference` (with the T-10-03 tamper-validation guard falling back to `"system"`), `resolveTheme`, and `applyTheme`
- Added a boot-time FOUC-prevention `<script>` IIFE to `index.html`'s `<head>`, positioned before the Google Fonts stylesheet `<link>`, so `data-theme` is set before first paint
- Wired a `themePreference` signal in `App.tsx` (seeded from `getStoredPreference()`) plus a `createEffect` that listens for `matchMedia("(prefers-color-scheme: dark)")` `change` events and re-applies the theme live while preference is `"system"`, with `onCleanup` removing the listener

## Task Commits

Each task was committed atomically:

1. **Task 1: Restructure App.css tokens + finalize glass/sidebar + typography scale** - `e5fa8f2` (feat)
2. **Task 2: theme.ts module + index.html boot script** - `f0b7321` (feat)
3. **Task 3: Wire theme preference signal + live OS-follow in App.tsx** - `996bf8c` (feat)

## Files Created/Modified

- `src/App.css` - Token restructure (`:root`/`[data-theme="dark"]`/`[data-theme="light"]`/`@theme inline`), `.sidebar-glass` class added
- `src/theme.ts` - New module: `ThemePreference` type, `getStoredPreference`, `resolveTheme`, `applyTheme`
- `index.html` - Boot-time theme-resolution `<script>` IIFE in `<head>`
- `src/App.tsx` - `themePreference` signal + matchMedia live-follow `createEffect`, new `./theme` import

## Decisions Made

- **`resolveTheme` not imported into `App.tsx`:** the task's guidance listed importing `getStoredPreference`, `resolveTheme`, `applyTheme`, and `ThemePreference`, but `App.tsx` only calls `getStoredPreference()` (signal seed) and `applyTheme("system")` (live-follow re-apply) — `applyTheme` already resolves internally via `resolveTheme`. Importing `resolveTheme` with no direct call site fails strict `noUnusedLocals`. Kept the import list to exactly what's consumed.
- **`void setThemePreference;` placeholder:** the setter has no UI consumer until plan 10-03's `ThemeToggle` control. A bare `void` reference (not a call) satisfies `noUnusedLocals` without building the visible toggle ahead of schedule — same pattern precedent as Phase 8 Plan 4's `conflictError`/`recordingModifiers` deferred-consumer reads (per STATE.md's decisions log).
- **`requirements mark-complete` skipped for UI-01/UI-03/UI-04:** this plan only establishes the token/translucency foundation. The full redesign (sidebar, Windows-equivalent surfaces, forms) spans plans 10-03 through 10-06, and the UI-04 perf gate is explicitly deferred to plan 10-03 (per this plan's own `<verification>` section: "formal on-device gate is plan 10-03"). Marking these requirements complete now would misrepresent phase progress, mirroring plan 10-01's identical decision for UX-09/UI-01.

## Deviations from Plan

None - plan executed exactly as written. The one adjustment (dropping the `resolveTheme` import) is documented above as a decision driven by the project's own strict `noUnusedLocals` TypeScript config (CLAUDE.md-mandated), not a scope change.

## Issues Encountered

None. `npx tsc --noEmit` and `cargo build --manifest-path src-tauri/Cargo.toml` are both clean after all three tasks.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The token foundation (dark + light sets, `@theme inline` mapping) and finalized `.glass-card`/`.sidebar-glass` classes are ready for plan 10-03 to build the sidebar rail against.
- `theme.ts`'s `applyTheme`/`getStoredPreference` and `App.tsx`'s `themePreference`/`setThemePreference` signal are ready for plan 10-03's `ThemeToggle` control to consume directly — no further backend/runtime wiring needed, only the visible 3-state cycle button.
- **Outstanding manual verification (human UAT, not blocking this plan's completion):** boot the app with no stored preference in both OS light and dark appearances and confirm no flash of the wrong theme; toggle OS appearance live while preference is unset (System) and confirm the app follows without restart; spot-check that no dark-only element is visible once light mode renders (full spot-check is easier once plan 10-03's sidebar exists, since more surfaces will render).
- **UI-04 perf measurement not run in this plan** — per RESEARCH.md's Pitfall 3 guidance and this plan's own `<verification>` note, the formal idle-CPU/GPU on-device measurement (Activity Monitor/Task Manager vs. v1.2.0 baseline) is scheduled for plan 10-03, once the sidebar's own `.sidebar-glass` blur is visually active alongside `.glass-card`. The documented fallback (reduce blur layer count) remains available if the measurement shows a rise.

## Self-Check: PASSED

All 4 files verified present (`src/App.css`, `src/theme.ts`, `index.html`, `src/App.tsx`) and all 3 commit hashes (`e5fa8f2`, `f0b7321`, `996bf8c`) verified present in `git log`.

---
*Phase: 10-ui-redesign-macro-management*
*Completed: 2026-07-24*
