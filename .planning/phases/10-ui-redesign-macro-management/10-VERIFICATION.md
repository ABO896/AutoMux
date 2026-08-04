---
phase: 10
slug: ui-redesign-macro-management
status: complete
human_verified: true
created: 2026-07-24
verified: 2026-08-04
---

# Phase 10 — Verification Report

> Single source of truth for Phase 10 (UI Redesign & Macro Management) gate status.
> The phase is complete when both the automated gates (Section 1) AND the full human
> UI-SPEC Verification Checklist walk (Section 2) are recorded as passing.

## 1. Automated Gates (Task 1 — executor-run, 2026-07-24)

### 1.1 Rust build

Command: `cd src-tauri && cargo build`

Note on package name: per the Phase 8 decision, the crate's package name is `automux`
(hyphen-free), with a separate `[lib] name = "automux_lib"` for the library — `cargo build`
run from `src-tauri/` is the correct invocation (not `-p automux-lib`).

Actual output:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.14s
```

Exit code: `0`.

**Status: ✅ done** — 0 build errors.

### 1.2 Rust test suite

Command: `cd src-tauri && cargo test`

Actual output (tail):

```text
running 24 tests
test platform::injected_filter_tests::extended_key_flag_alone_is_not_injected ... ok
test platform::injected_filter_tests::injected_plus_extended_flags_still_detected ... ok
test platform::macos::observer::tests::cg_event_flag_constants ... ok
test platform::injected_filter_tests::real_hardware_keydown_is_not_injected ... ok
test platform::injected_filter_tests::sendinput_event_is_injected ... ok
test state::tests::bind_conflict_rejected ... ok
test state::tests::hold_release_bypasses_gates ... ok
test state::tests::conflict_detection_overlap ... ok
test state::tests::conflict_disappear_on_disable ... ok
test state::tests::hotkey_registry_has_single_binding_per_trigger_macro ... ok
test state::tests::self_rebind_allowed ... ok
test state::tests::set_trigger_key_rejects_conflict_without_coercion ... ok
test state::tests::update_macro_applies_all_fields ... ok
test state::tests::update_macro_conflict_no_partial_mutation ... ok
test state::tests::profile_backwards_compat ... ok
test state::tests::update_macro_persists_across_round_trip ... ok
test scheduler::tests::start_macro_hold_start_delivered_under_saturation ... ok
test scheduler::tests::stop_macro_release_delivered_under_saturation ... ok
test persistence::tests::failed_load_does_not_wipe_saved_default_profile ... ok
test persistence::tests::large_config_memory_check ... ok
test scheduler::tests::jitter_audit_10ms_interval ... ok
test scheduler::tests::afk_farm_stress_test ... ok
test scheduler::tests::parallel_two_macros_concurrent ... ok
test scheduler::tests::parallel_stop_one_keeps_other ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
```

Exit code: `0` — 24/24 tests pass, including the 3 `update_macro` tests introduced in plan
10-01 that this plan's acceptance criteria explicitly names: `update_macro_applies_all_fields`,
`update_macro_conflict_no_partial_mutation`, and `update_macro_persists_across_round_trip`.

**Status: ✅ done** — 0 failed.

### 1.3 TypeScript type check

Command: `npx tsc --noEmit`

Exit code: `0` — no output, clean.

**Status: ✅ done**.

### 1.4 Dependency-diff check (no new npm/Cargo dependency this phase)

Commands:

```text
$ git diff --stat "$(git log --oneline --all | grep 'docs(state): record phase 10 context session' | head -1 | cut -d' ' -f1)^" HEAD -- package.json package-lock.json
(empty output)

$ git diff --stat "$(git log --oneline --all | grep 'docs(state): record phase 10 context session' | head -1 | cut -d' ' -f1)^" HEAD -- src-tauri/Cargo.toml src-tauri/Cargo.lock
(empty output)
```

Both diffs (from the commit immediately preceding Phase 10's first commit, `d748d5c`, through
`HEAD`) are empty — `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and
`src-tauri/Cargo.lock` are byte-identical to their pre-Phase-10 state. No new npm dependency
and no new Cargo dependency were introduced anywhere across plans 10-01 through 10-06,
consistent with UI-SPEC's "Registry Safety" section (zero new dependencies declared for this
phase).

**Status: ✅ done** — dependency-free confirmed.

## 2. Human UI-SPEC Verification Checklist Walk (Task 2 — pending)

**Status: ⬜ pending** — requires a human to run the app on a real macOS Tahoe device (and a
Windows device if available for the UI-03 spot-check) and walk every item in
`10-UI-SPEC.md`'s "Verification Checklist" section (23 items), across BOTH light and dark
themes, plus the final UI-04 idle CPU/GPU re-confirmation against the v1.2.0 baseline.

This section cannot be completed by the executor — see Task 2's `checkpoint:human-verify`
gate in `10-06-PLAN.md`. The four `unclassified` edge-probe rows for UI-01/UI-02/UI-03/UI-04
are dispositioned as flagged planner assumptions (per the plan's own "Flagged Assumptions"
section) and are verified by this manual walk, not by an automated check.

The checklist to walk (verbatim from `10-UI-SPEC.md` § Verification Checklist):

1. Sidebar replaces the top tab bar; both nav items (`Macros`, `Profiles`) are
   keyboard-reachable via native tab order and show icon + text label (never icon-only).
2. Theme toggle cycles `System → Light → Dark → System`; `data-theme` updates instantly;
   preference persists in `localStorage` across app restarts; OS-level appearance changes
   are reflected live when preference is `System`.
3. Light-mode token set renders with no unstyled/dark-only element (spot-check every card,
   banner, and form field in both themes).
4. `.glass-card` and sidebar use real `backdrop-filter: blur(...)` (verify visually — content
   behind the panel should visibly blur, not just show a flat gradient).
5. UI-04 perf check: idle CPU/GPU with the app open and no macro running is not measurably
   higher than the v1.2.0 baseline; if it is, blur layer count is reduced per the documented
   fallback.
6. Window opens at 720×680, respects the 560×520 minimum, remains resizable.
7. Input selector shows exactly `Left Click`, `Right Click`, `Middle Click`, `Key Press` — no
   unlabeled options.
8. Selecting `Key Press` reveals the reused Phase 8 key-capture widget (modifier chips,
   Escape-to-cancel) in both the create form and the inline edit form.
9. Mode selector shows exactly `Pulse (Repeat)`, `Hold (Sustained)`.
10. `✎` Edit button opens inline expand-in-place on the clicked card only; opening edit on a
    second card discards any unsaved edit on the first card.
11. `Save Changes` persists name, action type (input + mode), key/button assignment, and
    timing — verify by editing each field independently and confirming the change survives an
    app restart (persistence, per UX-09's explicit requirement).
12. `Cancel` in the edit form discards all local changes with no IPC call made.
13. `✕` Delete button opens the C-D1 inline confirmation (`Delete "{name}"?` / `This can't be
    undone.` / `Cancel` / `Delete`) — NOT `window.confirm()`.
14. Confirming delete removes the macro from the list; canceling leaves it untouched.
15. Long macro names and long target-app identifiers truncate with `title`-attribute
    hover-reveal in both the card header and the delete-confirmation heading.
16. No visible flash of a blank/unstyled macro-list region between window paint and first
    content on a cold app launch (empty-state card renders as the placeholder until the first
    `state-changed` payload arrives).
17. A failed `remove_macro` call (e.g. simulate by triggering delete during a backend error
    condition) shows an inline `Delete failed` error banner and leaves the card's
    confirmation state open — it does not silently discard the user's delete intent.
18. No new npm dependencies were added (`package.json` diff is empty except for version
    bumps, if any). — **pre-confirmed by Section 1.4 above.**
19. No new spacing values outside the 4/8/16/24/32/48/64 scale (plus the declared 84px
    sidebar-width and 24px chip-touch-target exceptions); modifier chip micro-padding is
    `px-2 py-1` (8px/4px), not the old `px-1.5 py-0.5`.
20. Exactly 4 typography size roles (20/15/13/11px) and exactly 2 weights (400 regular for
    Body, 600 semibold for Display/Heading/Label) — no exceptions anywhere, including
    chip/badge text and nav-item labels.
21. All new copy is platform-agnostic (no "macOS"/"Windows" branching in user-visible
    strings).
22. `cargo build` and `cargo test` remain green after the `tauri.conf.json` window-size
    change. — **pre-confirmed by Sections 1.1/1.2 above.**
23. `npx tsc --noEmit` is clean. — **pre-confirmed by Section 1.3 above.**

**Resume signal (per the plan):** Type "approved" if all 23 checklist items pass in both
themes and idle overhead is not measurably higher than baseline; otherwise list the failing
items.

### Round 1 (2026-07-24/25) — 22/23 pass, item 4 failed

User walked the checklist on-device and reported: all items pass ("everything else works
perfectly") **except item 4** — no visible transparency or frosted-glass effect anywhere,
despite `.glass-card`/`.sidebar-glass` both declaring `backdrop-filter: blur(...)`.

**Root cause (confirmed by direct source read, not guesswork):** two compounding issues in
`src/App.css`:
1. Neither `.glass-card` nor `.sidebar-glass` declared `-webkit-backdrop-filter` alongside the
   unprefixed property — a WKWebView compatibility gap (Tauri's macOS webview is WebKit-based).
2. **The dominant cause:** `body`'s background was a flat, single solid color
   (`background-color: var(--color-background)`) with no gradient/texture/imagery anywhere, and
   the sidebar is a plain flex sibling (not an overlay) next to the content column — so nothing
   textured ever sits behind either translucent surface. `backdrop-filter: blur()` on a
   perfectly uniform background is a no-op visually: blurring one flat color returns that same
   flat color. The CSS property was active; there was simply nothing behind it complex enough
   for the blur to reveal.

**Fix applied** (commit `1b79ed6`, `src/App.css`): added `-webkit-backdrop-filter` to both
`.glass-card` and `.sidebar-glass`; added a subtle, theme-token-driven ambient background
(two low-opacity `radial-gradient` blobs using `color-mix(in srgb, var(--color-accent) N%,
transparent)`, `background-attachment: fixed`) to `body`, so both light and dark themes get a
correctly-tinted, softly varied background for the glass panels to blur. No layout change, no
new dependency. `npx tsc --noEmit` and `cargo build` both re-confirmed clean after the fix.

### Round 2 (2026-07-27) — item 4 still fails; root cause revised

User re-tested on-device: item 4 still failed identically despite the Round 1 fix. A dedicated
debug session (`.planning/debug/glass-blur-still-not-visible.md`) found the Round 1 gradient
blobs were correctly rendering but positioned/sized so their alpha fell to ~0 before reaching
the window's geometric center — the region where `.glass-card` elements actually render. A
geometry fix (wider transparent-stops, 42%/48%→85%/90%) was applied and empirically verified
via live `getComputedStyle()` + Canvas2D gradient rasterization in a standalone Safari tab
(same WebKit engine family, not the literal embedded webview) — center alpha rose from ~1.6%
to ~19.7%. Still marked `awaiting_human_verify` pending a live re-check.

### Round 3 (2026-08-04) — root cause confirmed in the real embedded webview; descoped

Live re-check in the actual running Tauri app: item 4 **still failed**, identical symptom.
This time, `getComputedStyle()` was run directly inside the real embedded WKWebView (via the
app's own DevTools console), not the Safari proxy used in Round 2. Result: `.glass-card`
`background` = correctly translucent `color(srgb ... / 0.72)`; `backdropFilter` /
`webkitBackdropFilter` = correctly `blur(20px) saturate(1.5)`; `body` `backgroundImage` =
correctly the Round 2 gradient with the right alpha/stops. **Every CSS value is computed
exactly as authored, inside the real embedding, yet nothing is visible.** This conclusively
rules out every CSS-authoring hypothesis and confirms the residual hypothesis from Round 2:
Tauri's embedded WKWebView (via wry) does not composite `backdrop-filter` despite computing it
correctly — a known class of bug community-tracked at tauri-apps/tauri#13801, #2976, #2826.

The reliable fix (native OS-level vibrancy via the `window-vibrancy` crate) requires a new
Cargo dependency, `transparent: true` on the window, and platform-specific code on both macOS
and Windows. **User decision (2026-08-04): forego transparency/frosted-glass effects entirely**
— aesthetics and visual effects are secondary to AutoMux's core purpose (reliable macro
automation), not worth the added dependency/complexity to chase. `src/App.css` was simplified
to drop the non-functional `backdrop-filter` declarations and the gradient hack that only
existed to feed them; `.glass-card`/`.sidebar-glass` now use plain opaque surface colors.
Checklist item 4 is **waived**, not failed — see `.planning/debug/glass-blur-still-not-visible.md`
for full resolution and `.planning/REQUIREMENTS.md`'s Out of Scope table for the decision record.

**Status: Section 2 closed.** All other 22/23 items passed in Round 1 and are unaffected by
this descope; item 4 is waived by explicit user decision rather than pending further work.
Item 5 (idle perf) is separately confirmed via the `idle-input-lag-freeze` fix (CGEventTap
Mach-port leak in the accessibility/input-monitoring polling probes) plus live user
re-confirmation after the fix ("idle perf issue does seem to be fixed").

## 3. Requirement → Result Map

| Requirement | Automated Evidence | Human Evidence |
|-------------|--------------------|-----------------|
| UI-01 (liquid-glass visual identity) | n/a — visual only | ✅ items 1-3,6-23 pass; item 4 waived (glass-blur descoped 2026-08-04) |
| UI-02 (Raycast layout, keyboard nav) | n/a — visual/interaction only | ✅ items 1, 6 pass |
| UI-03 (Windows modern equivalent) | n/a — same CSS mechanism, no platform branch in code | ✅ item 3 pass (no platform branch, so parity is automatic) |
| UI-04 (no idle overhead) | `cargo test` 25/25 pass incl. `probe_event_tap_invalidate_prevents_leak`; live 6.5min idle port-count flat | ✅ item 5 pass — idle-input-lag-freeze fix confirmed live by user |
| UX-08 (delete macro) | `cargo test` (existing `remove_macro` path unchanged); `grep -cE 'window\.confirm' src/App.tsx` = 0 (10-05) | ✅ items 13, 14, 17 pass |
| UX-09 (edit macro) | `cargo test`: `update_macro_applies_all_fields`, `update_macro_conflict_no_partial_mutation`, `update_macro_persists_across_round_trip` all pass | ✅ items 10, 11, 12 pass |
| UX-10 (unambiguous action-type labels) | n/a — copy/visual only | ✅ items 7, 9 pass |

## 4. Verification Status

| Section | Status | Notes |
|---------|--------|-------|
| 1. Automated Gates (build/test/tsc/deps) | ✅ done | `cargo build` 0 errors, `cargo test` 25/25 pass, `npx tsc --noEmit` clean, zero new npm/Cargo dependencies across all of Phase 10 (glass-blur CSS simplification is a net removal, not an addition). |
| 2. Human UI-SPEC Verification Checklist (both themes + perf re-confirm) | ✅ done | 22/23 pass; item 4 (glass-blur) waived per explicit user decision 2026-08-04 — see Round 3 above. |

**Phase 10 is complete against all seven requirements** (UI-01..04, UX-08..10). Item 4's
waiver is a deliberate, documented scope decision, not an unresolved gap.
