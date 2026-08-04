# AutoMux

## What This Is

AutoMux is a cross-platform desktop auto-clicker and macro automation tool for macOS and Windows, built with Tauri 2, Rust, and SolidJS. It lets users define multi-step macros (clicks, keypresses, hold, timing, optional process targeting) and run them system-wide or scoped to a specific application, configuring them via a key-capture widget and running-process picker without needing to know system key codes or process names. Multiple macros run concurrently on both platforms — triggering one never blocks, delays, or cancels another. Users can fully manage their macro set post-creation (edit name/action/key/timing, delete) through a redesigned, Apple-inspired UI on macOS and a matching modern UI on Windows, with a global-hotkey system that supports the full practical key range, warns on binding conflicts, and clearly communicates when a bind is system-wide. It is aimed at users who need reliable, configurable input automation — gamers, productivity power users, and anyone who needs to automate repetitive mouse/keyboard tasks.

## Core Value

A macro that was set up must fire reliably — platform permissions must be detected correctly and execution must be accurate.

*Re-affirmed at the end of v2.0: when a UI polish item (liquid-glass/vibrancy) conflicted with reliability effort, the user explicitly chose to drop the aesthetic rather than pull in a new native dependency — see Key Decisions.*

## Current State

**Version:** v2.0 "Redesign & Platform Excellence" (shipped 2026-08-04)

- 10 phases completed across v1.0/v1.2.0/v2.0, 47 plans shipped, 224 commits in the v2.0 window alone (Phase 6 start → v2.0 ship)
- v2.0 delivered: macOS Tahoe 26 compatibility, parallel macro execution on both platforms, macro delete/edit, a full UI redesign (Apple-inspired sidebar layout on macOS, matching modern UI on Windows), and a hardened hotkey system (full key range, conflict detection, same-input overlap warnings, system-wide-bind communication)
- ~7,800 LOC across `src/` (SolidJS/TS) + `src-tauri/src/` (Rust) at v2.0 ship
- Stack: Tauri 2 (IPC bridge), Rust (StateActor + Scheduler actors, CGEvent/SendInput platform layer), SolidJS + Tailwind CSS 4 + Vite 6
- macOS: CGEventTap input injection (narrowed to 8 consumed event types post-v2.0), NSWorkspace process enumeration + active-app observation, AXIsProcessTrusted + live-probe permission checks
- Windows: SendInput injection, EnumProcesses process enumeration, WH_KEYBOARD_LL global hook with injected-event filtering, single consolidated HOTKEY_BINDINGS registry (shared design with macOS)
- Builds: unsigned DMG (macOS arm64+x86_64 universal), unsigned NSIS installer (Windows x64) — signing/notarization still deferred (DIST-01/02)
- CI: GitHub Actions release.yml with SHA-pinned third-party actions; universal binary enforced + lipo-verified; `npm ci` for reproducible installs
- Security: every v2.0 phase (6-10) now has a threat register (`*-SECURITY.md`, phases 8/9 explicit, 6/7/10 folded into this review) — 0 open threats at the `high` block threshold across the milestone
- Known open items carried into the next milestone: see Deferred Items in STATE.md (Phase 6 device-verification checklist never run under its own name; `macros-dont-fire-post-crash` debug session never device-confirmed; COMPAT-02 permission-detection-on-Tahoe-26 requirement unchecked) — all have since been indirectly re-exercised by later phases' passing device UAT with no related failures, but none has a first-party confirmation record

## Next Milestone Goals

Not yet defined — run `/gsd-new-milestone` to scope the next round of work. Candidates surfaced during v2.0:
- Promote backlog item 999.1: settings page for app customization, including self-exclusion (prevent AutoMux from being a valid macro target for itself)
- Consider closing the carried-forward device-verification debt (Phase 6 scenarios A/C/D, COMPAT-02) if it becomes user-visible
- Distribution: code signing + auto-updater (DIST-01) and macOS notarization (DIST-02), previously deferred to "v3 scope"

## Requirements

### Validated

- ✓ Macro creation with click and keypress actions — existing
- ✓ Per-macro enable/disable toggle — existing
- ✓ Hotkey-triggered macro activation — existing
- ✓ Configurable timing/delays between actions — existing
- ✓ Optional process targeting (scoped vs. system-wide) — existing
- ✓ Profile save/load (JSON persistence) — existing
- ✓ Windows and macOS builds via CI — existing
- ✓ GNU GPLv3 license — v1.0
- ✓ RELY-01: macOS permissions check correctly starts event tap on grant — v1.0 (partial: passive System Settings grant requires restart)
- ✓ RELY-02: CGEventTap re-enables on OS timeout — v1.0
- ✓ RELY-03: Windows emergency stop flushes held inputs — v1.0
- ✓ RELY-04: Macro changes auto-saved after every mutation — v1.0
- ✓ SAFE-01: All 5 production panic paths eliminated — v1.0
- ✓ SAFE-02: MacPlatformObserver lifetime fixed via app.manage — v1.0
- ✓ SAFE-03: 5ms minimum interval floor enforced — v1.0
- ✓ README-01/02/03: GPL-3.0 badge, prose fix, README rewrite — v1.0
- ✓ AUDIT-01/02/03: Stale artifacts removed, dead code clean, .gitignore extended — v1.0
- ✓ UX-01: Key capture widget (no raw key codes shown) — v1.0
- ✓ UX-02/03: Running-process picker on macOS + Windows — v1.0
- ✓ CI-01: Universal binary enforced (arm64+x86_64 via lipo) — v1.0
- ✓ CI-02: Third-party GitHub Actions SHA-pinned — v1.0
- ✓ PERM-01: macOS permissions correctly detected after Request Access approval — v1.2.0 Phase 5
- ✓ RELY-06: Passive System Settings grant arms CGEventTap without restart — v1.2.0 Phase 5
- ✓ BUILD-02: `block v0.1.6` macOS deprecation resolved — v1.2.0 Phase 5
- ✓ COMPAT-01: Macros fire correctly on macOS 26 Tahoe (CGEventTap injection) — v2.0 Phase 6
- ✓ COMPAT-03: App launches fully on macOS 26 Tahoe without errors or crashes — v2.0 Phase 6
- ✓ COMPAT-04: Input Monitoring not-granted detection with actionable guidance — v2.0 Phase 7
- ✓ COMPAT-05: TCC identity-change (unsigned→signed) re-grant prompt — v2.0 Phase 7
- ✓ BUILD-01: Zero Rust compiler warnings on Windows target — v2.0 Phase 7
- ✓ MEM-01: Windows `OpenProcess` handle leak closed — v2.0 Phase 7
- ✓ CI-03: Updater JSON signature upload fixed — v2.0 Phase 7
- ✓ CI-04: CI uses `npm ci` for reproducible builds — v2.0 Phase 7
- ✓ CI-05: Binary artifact discovery resilient to tauri-action renames — v2.0 Phase 7
- ✓ SAFE-04: REGISTRY lock released before CGEvent post (emergency-stop deadlock) — v2.0 Phase 7
- ✓ ERR-01: `auto-save-error` event surfaced in the UI — v2.0 Phase 7
- ✓ UX-11: Hotkey conflict detection with explicit error/reassignment — v2.0 Phase 8
- ✓ UX-12: Same-input-type concurrent-macro warning — v2.0 Phase 8
- ✓ UX-13: Full practical hotkey key range (0-9, F1-F12, modifiers) — v2.0 Phase 8
- ✓ UX-14: Global (system-wide) hotkey verified on macOS + Windows, UI communicates it — v2.0 Phase 8
- ✓ EXEC-01: Parallel macro execution on macOS, no block/queue/cancel — v2.0 Phase 9
- ✓ EXEC-02: Parallel macro execution on Windows, no block/queue/cancel — v2.0 Phase 9
- ✓ UX-08: Macro delete directly from the macro list — v2.0 Phase 10
- ✓ UX-09: Macro edit (name, action type, key/button, timing) post-creation — v2.0 Phase 10
- ✓ UX-10: Unambiguous action-type labels (Left Click / Right Click / Hold / Key Press) — v2.0 Phase 10
- ✓ UI-01: macOS Apple-design-language redesign — v2.0 Phase 10 *(liquid-glass/vibrancy specifically descoped — see Out of Scope)*
- ✓ UI-02: macOS Raycast-inspired keyboard-navigable layout — v2.0 Phase 10
- ✓ UI-03: Windows modern-UI equivalent — v2.0 Phase 10
- ✓ UI-04: No measurable idle overhead increase vs v1.2.0 — v2.0 Phase 10

### Active

*(none — v2.0 shipped 2026-08-04; run `/gsd-new-milestone` to define the next set)*

**Carried forward, not cleanly closed (see Deferred Items in STATE.md for full detail):**
- **COMPAT-02**: Permissions detection accurate on macOS 26 Tahoe — no false negatives. Left unchecked: Phase 6's device-verification Scenario A (live permission-indicator update) was never confirmed under its own name. The underlying mechanism is unchanged since v1.2.0's PERM-01/RELY-06 fixes and has seen no related failure reports.

### Out of Scope

- **A11Y-01** (future UI milestone) — ARIA attributes deferred to the UI redesign so they're built into the new component structure; not tracked separately from UI-01/02/03
- Auto-updater (DIST-01, v3) — core reliability must be solid first; signing/entitlements required as prerequisite
- macOS notarization (DIST-02, v3) — signing identity config is a one-way door; defer until process is finalized
- Cross-platform profile portability (UX-05, future) — requires NamedKey schema migration (UX-04) first
- NamedKey schema migration (UX-04, future) — prerequisite for UX-05, not yet scheduled
- Window vibrancy / CSS `backdrop-filter` glass effects (v2.0) — Tauri's embedded WKWebView does not composite `backdrop-filter` despite computing it correctly (confirmed via direct computed-style inspection in the live app across two independent fix attempts; matches community-tracked Tauri/wry issues #13801, #2976, #2826). A native `window-vibrancy` fix would need a new dependency + cross-platform work; user decided 2026-08-04 that aesthetics are secondary to AutoMux's core reliability purpose. See `.planning/debug/resolved/glass-blur-still-not-visible.md`.
- Mobile / web app — desktop automation tool by design

## Context

- **Codebase state:** ~7,800 LOC across `src/` (SolidJS/TS) and `src-tauri/src/` (Rust) at v2.0 ship. Rust backend: StateActor/Scheduler actor model, platform input layer (macOS CGEventTap + NSWorkspace, Windows SendInput + WH_KEYBOARD_LL), single-registry hotkey dispatch shared in design across both platforms. Frontend: SolidJS with extracted `MacroCard.tsx` / `MacroForm.tsx` / `KeyCaptureField.tsx` components (post-Phase-10, no longer a single monolithic `App.tsx`), Tailwind v4 token-based theming with light/dark/OS-follow.
- **Key known issues:** COMPAT-02 (Tahoe 26 permission-detection live-update) and three Phase 6 device scenarios (A/C/D) never confirmed under their own name — see Requirements/Active and STATE.md Deferred Items. `macros-dont-fire-post-crash` debug session (2026-06-17) never got device confirmation of its applied fix. Neither has produced a user-visible failure since; both are candidates for closure early in the next milestone if they resurface.
- **CI state:** Release workflow produces unsigned DMG (macOS) and NSIS installer (Windows). lipo verification gates the macOS artifact. `npm ci` used for reproducible installs (CI-04, closed in Phase 7). Signing/notarization/auto-update remain deferred (DIST-01/02).
- **Security state:** Every v2.0 phase now has a `*-SECURITY.md` threat register; 0 open threats at the `high` block-on threshold across the whole milestone (ASVS level 1).
- **User feedback:** No external users yet. All v1.0/v1.2.0/v2.0 requirements traced back to the developer's own use and manual device UAT — the primary feedback loop so far has been the developer's own dogfooding (e.g. the 100ms-hotkey-macro system-lag report that drove the G-09-1a CGEventTap-mask fix, and the "process picker doesn't select what I clicked" report that drove the 2026-08-04 debug session).

## Constraints

- **Tech stack:** Tauri 2 + Rust + SolidJS — no framework changes; improvements must work within this architecture
- **Compatibility:** Must maintain working builds for both macOS (arm64 + x86_64) and Windows (x64)
- **Permissions model:** macOS Accessibility permission handling is OS-enforced; fixes must work within what Tauri and CGEvent allow
- **Rendering:** Tauri's embedded WKWebView does not composite CSS `backdrop-filter` — any future translucency/vibrancy work requires the native `window-vibrancy` crate, not CSS alone (confirmed 2026-08-04, see Out of Scope)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Reserve auto-updater as Phase 5 / v2 | Core reliability must be solid first; signing/entitlements required as prerequisite | ✓ Good — v1.0 shipped without the complexity |
| Store MacPlatformObserver via app.manage() | Observer token was dropped at end of setup function — lifetime fix needed | ✓ Good — resource leak closed |
| Key capture widget + keymap.ts mapping | Raw key codes are opaque to non-technical users | ✓ Good — UX-01 closed cleanly |
| Process picker instead of text input | Users have no reliable way to know the exact process name string | ✓ Good — UX-02/03 closed, though a second distinct picker defect (stale `<option>` DOM node reuse) surfaced and was fixed post-v2.0-close, 2026-08-04 |
| 3s frontend poll for permission grant | Polling sidesteps the need for OS notification APIs; restart-free grant detection | ⚠️ Revisit — passive System Settings grant still requires restart |
| SHA pinning for GitHub Actions | Supply chain security: floating tags are vulnerable to tag-moving attacks | ✓ Good — CI-02 closed |
| `--target universal-apple-darwin` + lipo check | Previous CI produced arm64-only binary — enforcement needed | ✓ Good — CI-01 enforced |
| Live CGEventTap probe replaces cached AXIsProcessTrusted() | Tahoe 26 introduced false-negative permission detection on the cached path | ✓ Good — COMPAT-01/03 closed; COMPAT-02's live-update scenario still lacks device confirmation |
| StateActor/Scheduler two-phase dispatch for parallel execution | EXEC-01/02 required macro B to never block/queue/cancel macro A — needed an architectural change, not a patch | ✓ Good — proven by scheduler concurrency tests; required 3 rounds of gap-closure (stuck-input HoldRelease bypass, hotkey-rebind rollback, dual-registry double-dispatch) before fully closing |
| Single consolidated HOTKEY_BINDINGS registry (drop the second MACRO_TRIGGER_KEYS map) | A dual-registry design caused a self-cancelling double-dispatch on every hotkey press | ✓ Good — closed symmetrically on macOS and Windows, Phase 9 Plan 10 |
| Drop CSS-only liquid-glass/vibrancy pursuit; keep Apple-esque layout without translucency | Tauri/wry doesn't composite `backdrop-filter` in its WKWebView; the reliable fix needs a new native dependency | ✓ Good — user explicitly reaffirmed Core Value (reliability over aesthetics) rather than adding a new dependency this late in the milestone |
| Narrow CGEventTap subscription mask to 8 consumed event types | The original mask subscribed to 5 unused high-frequency event types, causing a permanent system-wide responsiveness tax that persisted even after macros stopped | ✓ Good — user-confirmed fix; also improved the app's own information-disclosure posture (fewer unrelated events observed) |
| Retroactive per-phase SECURITY.md threat registers for Phases 8/9/10 | Security enforcement was enabled mid-milestone; earlier phases (8/9) had plan-time `<threat_model>` blocks but no compiled register | ✓ Good — 0 open threats at the `high` threshold across the milestone; no new work required, purely compilation |

## Milestone History

<details>
<summary>v1.0 MVP — SHIPPED 2026-05-30</summary>

Initial release: macro creation (click/keypress), profile persistence, hotkey activation, process targeting, Windows + macOS CI builds. See `.planning/milestones/v1.0-ROADMAP.md`.

</details>

<details>
<summary>v1.2.0 Reliability & Polish — SHIPPED 2026-06-02</summary>

Fixed the false "not granted" permission-detection bug and armed CGEventTap on direct System Settings grants (no restart needed). See `.planning/milestones/v1.0-ROADMAP.md` note / ROADMAP.md.

</details>

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-08-04 — after v2.0 milestone shipped*
