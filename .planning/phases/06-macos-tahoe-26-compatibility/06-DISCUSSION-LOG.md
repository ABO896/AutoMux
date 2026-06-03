# Phase 6: macOS Tahoe 26 Compatibility - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-03
**Phase:** 6-macOS Tahoe 26 Compatibility
**Areas discussed:** Known failures on Tahoe 26, CGEventTap fallback strategy, Entitlements file scope, Legacy dep cleanup scope

---

## Known failures on Tahoe 26

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — macros don't fire | App launches, permissions show OK, clicking/keypresses have no effect | ✓ |
| Yes — app crashes on launch | App doesn't reach working UI | |
| Yes — multiple things broken | Several failures | |
| Not yet — need to test first | Haven't run on Tahoe 26 | |

**User's choice:** Macros don't fire on Tahoe 26

---

| Option | Description | Selected |
|--------|-------------|----------|
| Shows granted, macros still don't fire | Permission UI says granted but injection silently fails | |
| Shows denied / not granted | Permission detection also broken | ✓ |

**User's choice (free text clarification):** "In automux shows denied, in settings permissions are granted, doesnt fire"
**Notes:** Both COMPAT-01 and COMPAT-02 are failing together. `AXIsProcessTrusted()` returns false on Tahoe 26 even when System Settings shows the permission is granted.

---

| Option | Description | Selected |
|--------|-------------|----------|
| No — stays denied after clicking Request Access | AXIsProcessTrusted() consistently returns false | ✓ |
| Yes — Request Access path works, only passive grant broken | Narrower regression | |
| Haven't tried Request Access | N/A | |

**User's choice (free text):** Button shows OS dialog routing to Accessibility Settings, user grants it, app still shows denied after grant.
**Notes:** Request Access UI flow works correctly (shows prompt, routes to Settings), but post-grant detection still fails on Tahoe 26.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Dev build (cargo tauri dev) | Running from source | |
| Packaged DMG / installed .app | Running from installed bundle | ✓ |
| Both — same behavior | Tested both | |

**User's choice:** Packaged DMG / installed .app
**Notes:** Confirms this is not a dev-path/bundle-ID mismatch issue. AXIsProcessTrusted() regression is genuine on Tahoe 26.

---

## CGEventTap fallback strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Stay within current APIs only | No new signing/entitlements beyond permissions fix | ✓ |
| Add entitlements + signing setup | Full signing setup if CGEventTap needs it | |
| Document as blocked until signed | Treat as v3 blocker if hard-blocked | |

**User's choice:** Stay within current APIs only

---

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — include CGEventPost investigation | Researcher checks CGEventPost as fallback | ✓ |
| No — focus on AXIsProcessTrusted() fix first | Don't research fallback | |

**User's choice:** Yes — include CGEventPost investigation

---

## Entitlements file scope

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — add entitlements.plist as part of this phase | Add even without signing (Recommended) | ✓ |
| Only if diagnosed as root cause | Wait for confirmation | |
| Defer — keep entitlements with signing in v3 | Do together | |

**User's choice:** Yes — add entitlements.plist as part of this phase

---

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal — only what's needed for Accessibility/input injection | Specific entitlements only | ✓ |
| Full Tauri template — all standard macOS entitlements | More future-proof | |
| You decide | Let researcher determine | |

**User's choice:** Minimal — only what's needed for Accessibility/input injection

---

## Legacy dep cleanup scope

| Option | Description | Selected |
|--------|-------------|----------|
| Fix narrowly — only patch what's specifically broken | Minimal surgery | |
| Full migration — objc 0.2.7 → objc2 here if broken | Full cleanup if forced | ✓ |
| Defer again — document as Phase 7 tech debt | Kick to Phase 7 | |

**User's choice:** Full migration — objc 0.2.7 → objc2 here if it's broken

---

| Option | Description | Selected |
|--------|-------------|----------|
| Proactively investigate — check for known Tahoe 26 objc issues during research | Research first | |
| Reactive — attempt a build first, fix if it fails | Build first, fix if broken | ✓ |

**User's choice:** Reactive — attempt a build first, fix if it fails

---

## Claude's Discretion

- Exact entitlements key names and values required for Accessibility on Tahoe 26 — determined by researcher
- Whether entitlements fix requires `tauri.conf.json` changes or standalone `.entitlements` file — planner's call
- Whether `AXIsProcessTrusted()` regression requires entitlements alone or also an API call change — determined by researcher

## Deferred Ideas

- **objc 0.2.7 → objc2 migration** — Only triggered in this phase if Tahoe 26 build/runtime failures confirm it (reactive). Otherwise stays deferred.
- **Code signing + notarization** — Stays in v3. If CGEventTap is hard-blocked for unsigned apps on Tahoe 26, this becomes a v3 blocker.
