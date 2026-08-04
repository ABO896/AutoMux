---
phase: 10-ui-redesign-macro-management
plan: 03
subsystem: ui
tags: [solidjs, tailwind-v4, sidebar-nav, theme-toggle, tauri-window-config]

# Dependency graph
requires:
  - phase: 10-ui-redesign-macro-management (plan 02)
    provides: "Theme token foundation (:root/[data-theme]/@theme inline), finalized .glass-card, new .sidebar-glass class, theme.ts (getStoredPreference/resolveTheme/applyTheme), and App.tsx's themePreference signal (setter deferred to this plan)"
provides:
  - "src/components/Sidebar.tsx — 84px keyboard-navigable nav rail (Macros/Profiles), accessor-props, children slot for the footer control"
  - "src/components/ThemeToggle.tsx — 3-state System/Light/Dark cycle control with next-action aria-label"
  - "App.tsx cycleThemePreference handler consuming the setThemePreference setter deferred from plan 10-02"
  - "src-tauri/tauri.conf.json window resized to 720x680 (min 560x520), transparent unchanged (false)"
affects: ["10-04/10-05 (build the edit/delete/action-type surfaces inside the now-sidebar-driven layout)", "10-06 (final polish/verification)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "New src/components/ directory — first component-file extraction from the single-file App.tsx (Sidebar.tsx, ThemeToggle.tsx)"
    - "Sidebar footer uses a children slot (JSX.Element) rather than threading theme props through Sidebar, keeping Sidebar decoupled from ThemeToggle's implementation"
    - "Type-only circular import: Sidebar.tsx imports `type Tab` from ../App (App.tsx imports the Sidebar component) — erased at compile time, no runtime cycle"

key-files:
  created:
    - src/components/Sidebar.tsx
    - src/components/ThemeToggle.tsx
  modified:
    - src/App.tsx
    - src-tauri/tauri.conf.json

key-decisions:
  - "Sidebar accepts theme-toggle content via a `children` prop rather than threading `themePreference`/`onCycleTheme` accessor props through Sidebar itself — keeps Sidebar's prop surface scoped to navigation only; App.tsx composes `<Sidebar ...><ThemeToggle .../></Sidebar>`."
  - "Exported the previously-unexported `Tab` type alias from App.tsx so Sidebar.tsx can import it type-only without duplicating the union."
  - "cycleThemePreference lives in App.tsx (not inside ThemeToggle) — App() remains the sole owner of all signals per the established signals-down/accessor-props pattern; ThemeToggle only calls the passed-in onCycle callback."

requirements-completed: []

coverage:
  - id: D1
    description: "Sidebar.tsx replaces the top tab bar with an 84px rail; Macros/Profiles are real <button> elements (native tab order), active item shows bg-accent/10 + text-accent pill, inactive is text-text-muted with hover:text-text-main"
    requirement: "UI-02"
    verification:
      - kind: other
        ref: "grep -c 'const { ' src/components/Sidebar.tsx (0); grep -c '<button' src/components/Sidebar.tsx (2); grep -c 'w-\\[84px\\]' src/components/Sidebar.tsx (1); grep -c 'from \"./components/Sidebar\"' src/App.tsx (1)"
        status: pass
      - kind: other
        ref: "npx tsc --noEmit (exit 0)"
        status: pass
    human_judgment: true
    rationale: "Visual correctness of the active/inactive/hover pill states and keyboard tab-reachability requires a running app on a real display and an actual Tab key press — deferred to this plan's own Task 3 human-verify checkpoint, which also covers the UI-04 perf gate."
  - id: D2
    description: "ThemeToggle.tsx: 3-state System -> Light -> Dark -> System cycle glyph at the sidebar footer, with an aria-label naming the NEXT action (not the current state)"
    requirement: "UI-02"
    verification:
      - kind: other
        ref: "grep -c 'aria-label' src/components/ThemeToggle.tsx (2); grep -c 'const { ' src/components/ThemeToggle.tsx (0); npx tsc --noEmit (exit 0, strict noUnusedLocals satisfied for setThemePreference)"
        status: pass
    human_judgment: true
    rationale: "Live click-through-the-cycle behavior (data-theme swap with no reload, localStorage persistence across restart) requires a running app — deferred to Task 3's checkpoint per this plan's own verify step precedent from plan 10-02."
  - id: D3
    description: "Window resized to 720x680 (min 560x520), transparent stays false (D-04 CSS-only blur, no native window vibrancy)"
    requirement: "UI-04"
    verification:
      - kind: other
        ref: "node -e \"...\" prints '720 680 560 520 false'; cargo build --manifest-path src-tauri/Cargo.toml (clean)"
        status: pass
    human_judgment: false
  - id: D4
    description: "UI-04 EARLY perf gate: idle CPU/GPU with all blur surfaces active (.glass-card + .sidebar-glass) measured on-device against the v1.2.0 baseline, with the documented blur-layer-reduction fallback available if the measurement regresses"
    requirement: "UI-04"
    verification:
      - kind: other
        ref: "Human report: idle CPU/GPU not measurably higher; macros run with no lag before/during/after. No fallback (blur-layer reduction) applied."
        status: pass
    human_judgment: true
    rationale: "Confirmed on-device by the user (2026-07-24): no measurable idle overhead increase. Blur-layer-reduction fallback was NOT needed."

# Metrics
duration: ~15min (Tasks 1-2), checkpoint resolved same day
completed: 2026-07-24
status: complete
---

# Phase 10 Plan 03: Sidebar Navigation + ThemeToggle + Window Resize Summary

**84px keyboard-navigable sidebar rail (replacing the top tab bar) with a 3-state ThemeToggle at its footer, plus the Raycast-leaning 720x680 window resize — all 3 tasks complete, including the early UI-04 perf gate + both-theme visual spot-check checkpoint**

## Performance

- **Duration:** ~15 min (Tasks 1-2), checkpoint resolved same day
- **Completed:** 2026-07-24
- **Tasks:** 3/3
- **Files modified:** 4 (`src/App.tsx`, `src-tauri/tauri.conf.json`, plus 2 new files: `src/components/Sidebar.tsx`, `src/components/ThemeToggle.tsx`)

## Accomplishments

- Extracted `src/components/Sidebar.tsx` — the first component-file split out of the single-file `App.tsx` — an 84px `.sidebar-glass` rail with two real `<button>` nav items (`⚡ Macros`, `📁 Profiles`), accessor-props (no destructuring), driven by the `App()`-owned `activeTab`/`setActiveTab` signal
- Replaced the horizontal tab-bar `<div>` row in `App.tsx` with a `flex` row: `<Sidebar>` as the first child, the existing content column (internally unchanged, `Show when={activeTab() === ...}` blocks preserved) as the second
- Created `src/components/ThemeToggle.tsx` — a 3-state `System → Light → Dark → System` cycle control with a next-action `aria-label` (e.g. hovering `🖥` while System is active announces "Switch to light theme"), mounted as `Sidebar`'s `children` at the footer, below a `flex-grow` spacer
- Added `cycleThemePreference` in `App.tsx`, consuming the `setThemePreference` setter that plan 10-02 deferred (`void setThemePreference;` placeholder removed, setter now genuinely wired)
- Resized the Tauri window (`src-tauri/tauri.conf.json`) to `720×680` (min `560×520`); `transparent` stays `false` per D-04 (CSS-only blur, no native `NSVisualEffectView`/Acrylic vibrancy)
- `npx tsc --noEmit` and `cargo build --manifest-path src-tauri/Cargo.toml` both clean after all changes

## Task Commits

Each completed task was committed atomically:

1. **Task 1: Extract Sidebar.tsx and replace the top tab bar** - `dbdc8ee` (feat)
2. **Task 2: ThemeToggle.tsx (3-state cycle) at the sidebar footer + window resize** - `3f5237d` (feat)
3. **Task 3: Early UI-04 perf gate + both-theme visual spot-check** - **APPROVED** (human-verify checkpoint, no commit — verification only). User confirmed on-device (2026-07-24): idle CPU/GPU not measurably higher (macros ran with no lag before/during/after), both light and dark themes render cleanly, sidebar nav items are keyboard-reachable. No blur-layer-reduction fallback needed.

## Files Created/Modified

- `src/components/Sidebar.tsx` - New: 84px nav rail, `SidebarProps` = `{ activeTab, onSelectTab, children? }`, `.sidebar-glass` translucency
- `src/components/ThemeToggle.tsx` - New: `ThemeToggleProps` = `{ preference, onCycle }`, glyph + next-action `aria-label`
- `src/App.tsx` - Tab bar row replaced with `flex` (Sidebar + content column); `Tab` type exported; `cycleThemePreference` handler added; `ThemeToggle` mounted as `Sidebar`'s footer child
- `src-tauri/tauri.conf.json` - Window `width`/`height`/`minWidth`/`minHeight` updated to `720`/`680`/`560`/`520`; `transparent`/`decorations`/`resizable` unchanged

## Decisions Made

- **Children slot over threaded props:** `Sidebar` accepts theme-toggle content via `children: JSX.Element` rather than accepting `themePreference`/`onCycleTheme` accessor props directly. This keeps `Sidebar`'s prop surface scoped to navigation concerns only; `App.tsx` composes `<Sidebar activeTab={...} onSelectTab={...}><ThemeToggle preference={themePreference} onCycle={cycleThemePreference} /></Sidebar>`. Both options were explicitly left to the executor's discretion by the plan (Task 1 action: "pick one and keep it consistent").
- **Exported the `Tab` type alias:** previously private to `App.tsx`; now `export type Tab = "dashboard" | "profiles";` so `Sidebar.tsx` can `import type { Tab } from "../App"` — a type-only import that is erased at compile time, so it introduces no runtime circular-dependency between the two modules despite `App.tsx` also importing the `Sidebar` component value.
- **`cycleThemePreference` stays in `App.tsx`:** consistent with the codebase's established signals-down/accessor-props pattern (all `createSignal` calls and their mutating handlers live in `App()`); `ThemeToggle` is a pure presentational control that only invokes the `onCycle` callback it's given.

## Deviations from Plan

None — plan executed exactly as written for Tasks 1-2. Task 3 is an explicit blocking `checkpoint:human-verify` in the plan itself (`gate="blocking"`), not a deviation; per this executor's instructions it cannot be simulated or auto-approved and is reported to the orchestrator as a checkpoint.

## Issues Encountered

None blocking. `npx tsc --noEmit` and `cargo build --manifest-path src-tauri/Cargo.toml` are both clean after Tasks 1 and 2. Two pre-existing (not introduced by this phase) bugs surfaced during the human checkpoint's manual testing, both out of scope for this plan:
- **Macro delete button silently no-ops** — root cause is almost certainly `window.confirm()` being unreliable inside Tauri's WKWebView; this predates Phase 10 (commit `948f27a`). Plan 10-05 already replaces this with an in-card glass confirmation (D-14), so no separate fix needed here.
- **Target-app process picker doesn't bind the clicked option** — pre-existing, unrelated to any file this phase touches. Logged as `.planning/todos/pending/2026-07-24-process-picker-doesnt-select-clicked-app.md` for a future `/gsd-debug` session.

Also captured as a backlog idea (not a bug): a future settings page for app-level customization, starting with excluding AutoMux itself as a valid macro target to prevent self-triggering chaos. Filed as `.planning/phases/999.1-settings-page-app-customization-self-exclusion/` (ROADMAP.md Backlog section).

## User Setup Required

None further — Task 3 was completed via the human checkpoint below.

**Task 3 checkpoint result (2026-07-24):** User built/ran the app (`npm run tauri dev`) on their macOS Tahoe device and reported:
1. No measurable idle lag with all blur surfaces active (`.glass-card` on every card + `.sidebar-glass` on the rail) — macros ran with no lag before, during, or after. Fallback (blur-layer reduction) NOT needed.
2. Both light and dark themes render cleanly with no dark-only elements bleeding into light mode.
3. Theme toggle placement is discoverable, though the user noted (as a soft UX observation, not a blocking issue) it isn't exactly where they'd instinctively look first — no action taken, footer placement was an explicit design decision (D-06) from discuss-phase.

## Next Phase Readiness

- **Plan 10-03 is complete.** All 3 tasks done, checkpoint approved. Requirements UI-01/UI-02/UI-03/UI-04 remain "Pending" at the phase level in REQUIREMENTS.md (consistent with 10-01/10-02) — they're only marked complete when the full Phase 10 finishes.
- The sidebar rail, ThemeToggle, and resized 720×680 window are ready for plans 10-04/10-05 to build the macro-edit/delete/action-type surfaces on top.

## Self-Check: PASSED

Both files verified present (`src/components/Sidebar.tsx`, `src/components/ThemeToggle.tsx`) and both commit hashes (`dbdc8ee`, `3f5237d`) verified present in `git log`. Human-verify checkpoint (Task 3) approved by the user 2026-07-24.

---
*Phase: 10-ui-redesign-macro-management*
*Completed: 2026-07-24*
