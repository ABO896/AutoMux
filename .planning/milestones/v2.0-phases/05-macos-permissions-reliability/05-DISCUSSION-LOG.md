# Phase 5: macOS Permissions & Reliability - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-31
**Phase:** 5-macOS Permissions & Reliability
**Areas discussed:** Post-grant UX (PERM-01), Tap arming strategy (RELY-06), block dep scope (BUILD-02)

---

## Post-grant UX (PERM-01)

**Q1: What should the UI show during the approval window after Request Access is clicked?**

| Option | Description | Selected |
|--------|-------------|----------|
| Pending state | Show "Waiting for approval..." indicator; prevents false "not granted" flash | ✓ |
| Keep polling as-is | 3s polling corrects the state; brief "not granted" is acceptable | |
| Quick-retry loop | Frontend polls every 500ms for up to 10s after request_accessibility returns | |

**User's choice:** Pending state
**Notes:** None

---

**Q2: When should the "Pending" state be cleared if the user dismisses the OS dialog without approving?**

| Option | Description | Selected |
|--------|-------------|----------|
| Timeout after 30s | Clear pending if polling hasn't returned true within 30s of clicking Request Access | ✓ |
| Next poll result | Clear pending as soon as the next poll resolves (to granted or not granted) | |
| Manual dismiss | Show "Check again" button requiring explicit user action | |

**User's choice:** Timeout after 30s
**Notes:** None

---

## Tap Arming Strategy (RELY-06)

**Q1: What triggers initialize_tap() when permission is detected post-launch?**

| Option | Description | Selected |
|--------|-------------|----------|
| Backend auto-arms | check_accessibility IPC handler arms tap before returning true, if not yet initialized | ✓ |
| Frontend-driven arm | Frontend detects false→true transition and calls a new arm_tap IPC command | |
| Startup-only, require restart | Keep tap at startup only; document restart requirement; RELY-06 not fixed | |

**User's choice:** Backend auto-arms
**Notes:** None

---

**Q2: When the tap arms post-grant, should the user see feedback?**

| Option | Description | Selected |
|--------|-------------|----------|
| Silent activation | Permission indicator updates to "granted"; macros start working; no notification | ✓ |
| Toast notification | Brief toast "Macros now active" when tap arms post-grant | |
| State banner | Temporary banner in permissions area; auto-dismisses after 5s | |

**User's choice:** Silent activation
**Notes:** None

---

## block Dep Scope (BUILD-02)

**Q1: What should Phase 5 do about block v0.1.6?**

| Option | Description | Selected |
|--------|-------------|----------|
| Remove cocoa | Verify no direct cocoa usage, remove from Cargo.toml; block dep disappears | ✓ |
| Broader legacy cleanup | Also audit and migrate objc 0.2.7 to objc2 equivalents | |
| Pin block version only | Cargo.toml override for newer block; band-aid, preserves cocoa/objc as-is | |

**User's choice:** Remove cocoa
**Notes:** None

---

## Claude's Discretion

- Exact wording / styling of the "Pending approval…" indicator (chip, italic text, spinner)
- Whether `initialize_tap()` is called synchronously inside `check_accessibility` or dispatched to a background task (depends on CGEventTap threading constraints)

## Deferred Ideas

- `objc 0.2.7` legacy cleanup → future tech-debt phase
- Broader `cocoa` → `objc2` migration audit → future tech-debt phase
