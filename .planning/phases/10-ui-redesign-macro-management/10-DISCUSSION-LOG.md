# Phase 10: UI Redesign & Macro Management - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-23
**Phase:** 10-ui-redesign-macro-management
**Areas discussed:** Cross-platform visual tiering, Navigation & layout structure, Action-type model (create & edit), Edit interaction & delete confirmation

---

## Cross-platform visual tiering

| Option | Description | Selected |
|--------|-------------|----------|
| Same tier as Windows (recommended) | One shared modern theme on Windows + older macOS | |
| Native vibrancy tier | macOS-native NSVisualEffectView look, distinct from Windows | |
| You decide | Claude picks | |

**User's choice:** Free text — rejected the whole tiering premise. *"i want a raycast aesthetic, modern, sleek, sort of like apple liquid glass but not actually liquid glass, i want the app to look the same on windows and mac, like raycast which is on both windows and mac and looks roughly the same."*
**Notes:** One unified design across both platforms and all OS versions — no macOS-26-only glass tier, no fallback tier. This reinterprets the ROADMAP's literal "liquid glass on macOS 26 / modern equivalent on Windows" framing.

| Homage level | Description | Selected |
|--------|-------------|----------|
| Direct visual homage | Closely mirror Raycast's specific patterns | |
| General inspiration only | Borrow translucency/minimalism spirit, own layout | |
| You decide | | |

**User's choice:** Free text — *"Only what makes sense, so maybe no command bar style search if it isn't necessary or the best solution. But for everything that does make sense yes. not a clone but the same design language etc (the modern liquid-glass era design that a lot of modern sleek apps are adopting)"*

| Blur approach | Description | Selected |
|--------|-------------|----------|
| Real backdrop blur (recommended) | Genuine CSS backdrop-filter translucency | ✓ |
| Keep current gradient-only glass-card | No real blur | |
| You decide | | |

**User's choice:** Real backdrop blur (recommended)

| Accent color | Description | Selected |
|--------|-------------|----------|
| Keep existing indigo (recommended) | #6366f1, no re-branding | |
| Introduce a new accent color | Fresh visual identity | |
| You decide | | ✓ |

**User's choice:** You decide

| Theme scope | Description | Selected |
|--------|-------------|----------|
| Dark-only (recommended) | Matches current app | |
| Add light/dark toggle | Matches Raycast's feature set, bigger scope | ✓ |

**User's choice:** Add light/dark toggle

| Window vibrancy | Description | Selected |
|--------|-------------|----------|
| CSS-only internal blur (recommended) | Opaque window, blur only between panels | |
| True OS-level window vibrancy | transparent:true + native window-effect APIs | |
| You decide | | ✓ |

**User's choice:** You decide

| Theme default | Description | Selected |
|--------|-------------|----------|
| Follow OS system preference (recommended) | Auto-detect, user can override | ✓ |
| Default dark, manual toggle only | No OS detection | |

**User's choice:** Follow OS system preference (recommended)

---

## Navigation & layout structure

| Nav structure | Description | Selected |
|--------|-------------|----------|
| List + detail split view | Macro list + detail/edit panel | |
| Keep tabs, re-skin only | Same Dashboard/Profiles tabs, restyled | |
| Sidebar navigation | Persistent left sidebar | |
| You decide | | ✓ |

**User's choice:** You decide

| Window size | Description | Selected |
|--------|-------------|----------|
| Keep compact widget size (recommended) | 420x640 footprint preserved | |
| Widen the window | Wider Raycast-like proportions | |
| You decide | | ✓ |

**User's choice:** You decide

---

## Action-type model (create & edit)

| Action model | Description | Selected |
|--------|-------------|----------|
| Unified single selector (recommended) | One dropdown: Left Click/Right Click/Key Press/Hold | |
| Two selectors, clearer labels | Keep input-type + trigger-mode split, add Key Press | |
| You decide | | ✓ |

**User's choice:** You decide

| Key capture | Description | Selected |
|--------|-------------|----------|
| Reuse existing key-capture widget (recommended) | Same widget as hotkey binding | ✓ |
| You decide | | |

**User's choice:** Reuse existing key-capture widget (recommended)

---

## Edit interaction & delete confirmation

| Edit surface | Description | Selected |
|--------|-------------|----------|
| Inline expand-in-place (recommended) | Card expands into editable form | ✓ |
| Separate modal/panel | New overlay pattern | |
| You decide | | |

**User's choice:** Inline expand-in-place (recommended)

| Delete confirm | Description | Selected |
|--------|-------------|----------|
| Custom-styled confirmation (recommended) | Replace window.confirm() with styled in-app UI | ✓ |
| Leave as plain window.confirm() | No change | |
| You decide | | |

**User's choice:** Custom-styled confirmation (recommended)

---

## Claude's Discretion

- Accent color (keep indigo or introduce new)
- OS-level window vibrancy vs. CSS-only internal blur
- Navigation restructure (list+detail / sidebar / re-skinned tabs)
- Window size (keep compact widget or widen)
- Unified vs. dual action-type selector design

## Deferred Ideas

None — discussion stayed within phase scope.
