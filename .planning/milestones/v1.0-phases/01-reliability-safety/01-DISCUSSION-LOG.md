# Phase 1: Reliability & Safety - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-16
**Phase:** 1-Reliability & Safety
**Areas discussed:** Permission re-check experience, Auto-save profile target, Failure recovery visibility, Interval floor: clamp vs reject

---

## Permission Re-check Experience (RELY-01)

### Q1: Poll frequency after dismissing to grant permissions

| Option | Description | Selected |
|--------|-------------|----------|
| Fix to 3s | Match stated intent in code comment | ✓ |
| Keep 10s | Less CPU pressure but poor UX after grant | |
| Fast-poll for 60s then settle | 3s window, then drop to 10s | |

**User's choice:** Fix to 3s

---

### Q2: Re-check button

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — add button | Immediate re-check on user click | |
| No — polling is enough | 3s fast enough, less UI complexity | ✓ |

**User's choice:** No re-check button

---

### Q3: On grant detected

| Option | Description | Selected |
|--------|-------------|----------|
| Clear prompt silently | UI state change is the signal | ✓ |
| Show brief success message | Toast "Accessibility permission granted" | |

**User's choice:** Clear prompt silently

---

### Q4: On launch with denied permissions

| Option | Description | Selected |
|--------|-------------|----------|
| Show prompt with button, no auto-redirect | Current behavior, user-initiated | ✓ |
| Auto-redirect immediately | Aggressive, poor first-run UX | |

**User's choice:** Show prompt with 'Open Settings' button, no auto-redirect

---

## Auto-save Profile Target (RELY-04)

### Q1: Which profile to write to

| Option | Description | Selected |
|--------|-------------|----------|
| Always 'default' profile | Simple, no active-profile tracking in AppState | ✓ |
| Last loaded/saved named profile | Intuitive for named-profile users, more complex | |

**User's choice:** Always 'default' profile

---

### Q2: Suppress auto-save during profile load

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — suppress during load | Prevents 50 writes for a 50-macro profile load | ✓ |
| No — each AddMacro triggers its own save | Simpler but wasteful | |

**User's choice:** Yes — suppress and write once after load completes

---

### Q3: Save failure handling

| Option | Description | Selected |
|--------|-------------|----------|
| Log in debug only, continue silently | Less noisy, data still in memory | |
| Surface error to frontend | More transparent | ✓ |

**User's choice:** Surface error to frontend

---

### Q4: Auto-save error UI presentation

| Option | Description | Selected |
|--------|-------------|----------|
| Transient status message | Non-blocking, auto-dismisses | ✓ |
| Persistent warning banner | Stays until dismissed or next save succeeds | |

**User's choice:** Transient status message

---

## Failure Recovery Visibility (SAFE-01, RELY-02, RELY-03)

### Q1: Injection action skipped (SAFE-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Silent — skip and continue | Transient OS-level events, auto-resolve | ✓ |
| Debug-log only | stderr under debug_assertions | |
| Surface to frontend | High noise risk | |

**User's choice:** Silent — skip and continue

---

### Q2: CGEventTap re-enable on timeout (RELY-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Silent recovery | Invisible to user, ideal outcome | ✓ |
| Brief status indicator | 'Tap recovered' message for power users | |

**User's choice:** Silent recovery

---

### Q3: Windows emergency stop flush failure (RELY-03)

| Option | Description | Selected |
|--------|-------------|----------|
| Best-effort: flush as many as possible, then exit | Match macOS approach | ✓ |
| Abort on flush error, don't exit | Defeats purpose of emergency stop | |

**User's choice:** Best-effort flush, then exit regardless

---

## Interval Floor: Clamp vs Reject (SAFE-03)

### Q1: Enforcement behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Clamp silently to 5ms | Non-blocking, standard min/max behavior | ✓ |
| Reject with error message | Honest but requires frontend changes | |
| Clamp + show warning | Clamp with transient notice | |

**User's choice:** Clamp silently to 5ms

---

### Q2: Frontend validation

| Option | Description | Selected |
|--------|-------------|----------|
| Scheduler only — single enforcement point | Out of scope for Phase 1 reliability fixes | ✓ |
| Both frontend and scheduler | Better UX but adds frontend work | |

**User's choice:** Scheduler only

---

## Claude's Discretion

- How to wire `ProfileManager` into `StateActor` (Arc, direct injection, or channel)
- Whether to model load suppression as a boolean flag on `AppState` or as `Intent::BeginProfileLoad` / `Intent::EndProfileLoad` pair

## Deferred Ideas

None — discussion stayed within phase scope.
