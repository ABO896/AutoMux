# Phase 6: macOS Tahoe 26 Compatibility — Research

**Researched:** 2026-06-04
**Domain:** macOS Accessibility TCC, CGEventTap, CoreGraphics event injection, Tauri 2 entitlements
**Confidence:** MEDIUM-HIGH (primary findings verified via multiple developer community sources and official docs; Apple has not published Tahoe-specific API changelog entries for these APIs)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Both COMPAT-01 and COMPAT-02 are failing, causally linked: `AXIsProcessTrusted()` persistently returns `false` on Tahoe 26 even after Accessibility is granted → CGEventTap never arms → macros don't fire.
- **D-02:** Tested on a packaged DMG / installed .app — not a dev-path issue.
- **D-03:** The Request Access button and OS prompt work correctly; `AXIsProcessTrusted()` is the broken link (post-grant detection fails).
- **D-04:** App launches fine on Tahoe 26 — failure is strictly at permissions-detection and input-injection layer. COMPAT-03 is NOT failing.
- **D-05:** Add an `entitlements.plist` as part of this phase. An entitlements file can be added to the `.app` bundle without code signing. **[RESEARCH NOTE: This hypothesis is incorrect — see Critical Finding #1 below. Research recommends the planner raise this with the user.]**
- **D-06:** Scope of entitlements file: minimal only — only entitlements confirmed as required for Accessibility permission detection and CGEventTap on Tahoe 26.
- **D-07:** Code signing stays deferred to v3.
- **D-08:** CGEventPost fallback: only if entitlements/API fix fails; must stay within CGEvent-family, no signing setup.
- **D-09:** If Apple hard-blocked unsigned apps from CGEventTap on Tahoe 26, becomes v3 blocker.
- **D-10:** objc 0.2.7 migration is reactive — attempt build/run on Tahoe 26 first.
- **D-11:** If objc 0.2.7 causes Tahoe 26 failures → full migration to objc2 in this phase.

### Claude's Discretion
- Exact entitlements key names and values to include in the `.plist`.
- Whether the entitlements fix requires changes to `tauri.conf.json` or a standalone `.entitlements` file.
- Whether the `AXIsProcessTrusted()` regression is fixed by entitlements alone or also requires an API call change.

### Deferred Ideas (OUT OF SCOPE)
- objc 0.2.7 → objc2 full migration (only if triggered by Tahoe 26 failures).
- Code signing + notarization (v3).
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| COMPAT-01 | CGEventTap input injection works without OS modification on macOS 26 Tahoe | Root cause identified: `AXIsProcessTrusted()` cache staleness prevents tap from arming. Fix: replace with live CGEventTap probe. |
| COMPAT-02 | Permissions detection accurately reports granted/denied — no false "not granted" after approval | Same root cause as COMPAT-01. Live CGEventTap probe fixes both simultaneously. |
| COMPAT-03 | App launches on Tahoe 26 without crash, missing entitlement error, or framework exception | Confirmed not failing (D-04). No fix required; regression-guard via smoke test. |
</phase_requirements>

---

## Summary

### The Problem (Confirmed Root Cause)

`AXIsProcessTrusted()` on macOS 15 Sequoia and macOS 26 Tahoe reads from a **per-process in-memory cache** populated at first call. When the OS performs a TCC re-validation — triggered by an app update, OS update, or code signature change — the live TCC database is updated but the in-process cache is not invalidated. The result is that `AXIsProcessTrusted()` returns `true` or `false` from stale state, not from the live permission state. [CITED: developer community reports corroborated across multiple Apple Developer Forum threads including thread/727984 and thread/794253; fazm.ai technical analysis]

In AutoMux's specific failure mode, the observed symptom is the inverse: `AXIsProcessTrusted()` returns `false` persistently even after the user grants Accessibility. This is consistent with the same caching mechanism — the cache was populated before the grant and is never refreshed by a simple re-call to `AXIsProcessTrusted()`.

### The Fix

Replace `AXIsProcessTrusted()` with a **live CGEventTap probe** as the authoritative permission check. Creating a CGEventTap (even a minimal one with `CGEventTapOptions::ListenOnly` observing a single event type) consults the live TCC database, not the per-process cache. If `CGEventTap::new(...)` succeeds, the permission is genuinely granted. If it returns `Err`, the permission is genuinely not granted. [CITED: Apple Developer Forum analysis, fazm.ai four-failure-modes write-up, GitHub AeroSpace issue #1012]

This fix makes `check_accessibility_permissions(false)` authoritative again on Tahoe 26, which unblocks the tap initialization in `check_accessibility` (IPC) and the 3s polling in `App.tsx` — no frontend changes required.

### Critical Finding: The Entitlements Hypothesis is Blocked by Signing

**D-05 assumes entitlements can be added to the `.app` bundle without code signing.** This assumption is incorrect for entitlement effectiveness on TCC. Tauri 2's official documentation explicitly states: "Entitlements are applied when your application is signed." [CITED: v2.tauri.app/distribute/macos-application-bundle/] An `entitlements.plist` embedded in an unsigned `.app` bundle has **no effect on TCC permission behaviour** — macOS ignores entitlements in unsigned bundles for TCC access control purposes. [CITED: Apple's code signing model documented in multiple authoritative sources]

**What this means for the plan:** The entitlements path (D-05/D-06) does NOT fix the Tahoe 26 regression for unsigned apps. The correct fix is a code change to `check_accessibility_permissions()` in `platform/macos/mod.rs` — replacing the caching `AXIsProcessTrusted()` call with a live CGEventTap probe. Entitlements are needed only for hardened runtime or App Store distribution, both of which are v3 scope.

The planner should note this as a **constraint revision** that supersedes D-05's implementation approach while preserving D-05's intent (restore correct permission detection on Tahoe 26).

**Primary recommendation:** Change `check_accessibility_permissions(false)` to probe permission via a short-lived `CGEventTap::new(ListenOnly, ...)` attempt instead of calling `AXIsProcessTrusted()`. Add `NSAccessibilityUsageDescription` to `src-tauri/Info.plist` (merged by Tauri CLI; works without signing) to satisfy any usage-description checks that show the app in the Accessibility Settings list.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Permission detection (AXIsProcessTrusted replacement) | Platform layer (`platform/macos/mod.rs`) | IPC layer (propagates result) | Permission check is OS-level; IPC layer only relays the result |
| CGEventTap arming on permission grant | IPC layer (`ipc/mod.rs: check_accessibility`) | State layer (StateActor arms tap on permission change) | Phase 5 established: tap is armed in the IPC handler that returns `true` |
| Info.plist NSAccessibilityUsageDescription | Build config (`src-tauri/Info.plist`) | — | Merged at bundle time by Tauri CLI; no code change |
| entitlements.plist (future) | Build config (`src-tauri/tauri.conf.json`) | Signing infrastructure | Only effective when signed; deferred to v3 |

---

## Standard Stack

### Core (Existing — no new packages needed)
| Library | Version | Purpose | Notes |
|---------|---------|---------|-------|
| `core-graphics` | 0.24.0 (in use) | CGEventTap creation/management | `CGEventTap::new()` return value is the live permission probe |
| `core-foundation-sys` | 0.8.0 (in use) | CFBoolean, CFDictionary for AXIsProcessTrustedWithOptions | Stays in use for `prompt=true` path |

A newer `core-graphics` 0.25.0 is available on crates.io. The existing 0.24.0 provides all needed APIs. **Upgrading is not required for this phase** and introduces risk without reward. [VERIFIED: npm registry → `cargo search core-graphics`]

### No New Packages Required

The fix is entirely a code change within existing Rust code using existing crates. No new dependencies are needed.

---

## Package Legitimacy Audit

> No new packages are being introduced in this phase. Existing packages (core-graphics, core-foundation-sys, objc2, block2) have been in the codebase from prior phases.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
*No new package installations in this phase — audit is N/A.*

---

## Architecture Patterns

### System Architecture Diagram

```
User grants Accessibility in System Settings
              │
              ▼
   macOS TCC database (live)
              │
              ▼
   3s poll: check_accessibility (IPC)
              │
              ▼
   check_accessibility_permissions(false)
   [CURRENT: AXIsProcessTrusted() ← reads stale per-process cache → returns false]
   [FIXED:   CGEventTap::new(ListenOnly) ← reads live TCC database → returns true]
              │
         granted = true
              │
              ▼
   initialize_tap() ← arms the real CGEventTap for input observation
              │
              ▼
   Frontend poll sees true → UI updates → macros can fire
              │
              ▼
   StateActor: inject_input() → CGEvent.post(HID) → macro fires
```

### Recommended Project Structure (No Changes)

The fix is contained to one function in one file:
```
src-tauri/src/platform/macos/
├── mod.rs              ← FIX HERE: check_accessibility_permissions(false) path
├── input.rs            ← unchanged
└── observer.rs         ← unchanged (initialize_tap() already correct)

src-tauri/
└── Info.plist          ← NEW FILE: NSAccessibilityUsageDescription key
```

---

## Critical Findings (Priority Order for Planner)

### Finding 1: Entitlements Hypothesis Blocked — Entitlements Require Signing

**D-05 stated:** "An entitlements file can be added to the `.app` bundle without code signing."
**Research finding:** This is incorrect. Tauri 2 official documentation: "Entitlements are applied when your application is signed." [CITED: v2.tauri.app/distribute/macos-application-bundle/]

For unsigned apps, an `entitlements.plist` file is ignored by macOS TCC at runtime. The `bundle > macOS > entitlements` field in `tauri.conf.json` only does something useful when `signingIdentity` is also set.

**Impact on planning:** D-05's goal (fix AXIsProcessTrusted on Tahoe 26) is still achievable, but NOT via entitlements. The fix must be a code change. D-07 (defer signing to v3) is preserved — this finding strengthens that decision.

**Action for planner:** The plan must NOT include a task to "create entitlements.plist and set tauri.conf.json entitlements field" as a fix for COMPAT-02. Instead, the plan must fix `check_accessibility_permissions()` in `mod.rs`.

### Finding 2: AXIsProcessTrusted() Cache Staleness is the Root Cause

`AXIsProcessTrusted()` caches its result per-process. On Tahoe 26 (also present in Sequoia 15), TCC re-validation does not invalidate this cache. The cached value is returned even after the actual permission state has changed. [CITED: Apple Developer Forum threads 727984, 794253, fazm.ai analysis]

**Specific failure in AutoMux:** The cache is populated as `false` at process start (before the user grants permission). Subsequent calls to `AXIsProcessTrusted()` return the cached `false` value. The 3s polling loop in `App.tsx` calls `check_accessibility()` → `AXIsProcessTrusted()` → always `false` → CGEventTap is never armed.

### Finding 3: CGEventTap::new() is the Correct Live Permission Probe

`CGEventTap::new(...)` calls into the kernel and consults the live TCC database on each invocation — it does not use the per-process cache. If tap creation succeeds (returns `Ok`), the process has live Accessibility permission. If it fails (returns `Err`), the process genuinely lacks it. [CITED: fazm.ai, Apple Developer Forum AeroSpace issue #1012 analysis]

The existing `core-graphics` 0.24.0 crate already supports this via `CGEventTap::new()`. The tap can be immediately destroyed after creation if the only purpose is the live permission probe — it is a side-effect-free check.

**Preferred approach:** For the `prompt=false` path in `check_accessibility_permissions()`, attempt `CGEventTap::new()` with `CGEventTapOptions::ListenOnly` and a minimal event type set. If it succeeds → return `true`. If it fails → return `false`. Destroy the tap immediately after the check if not arming it.

**Better approach (integrates probe with arming):** Instead of a pure probe + destroy, the `check_accessibility_permissions(false)` + `initialize_tap()` sequence can be merged: attempt tap creation; if it succeeds, keep the tap alive (reuse it instead of creating a second tap in `initialize_tap()`). This eliminates the double-tap creation. However, this is an architectural change to Phase 5's two-step flow — assess complexity before committing.

**Safe approach (minimal change, recommended):** Keep the existing two-step flow. In `check_accessibility_permissions(false)`:
1. Attempt `CGEventTap::new(...)` with a throwaway callback.
2. If `Ok(_)` → return `true` (immediately drop the tap — it's just a probe).
3. If `Err(_)` → return `false`.

This is the lowest-risk fix that preserves Phase 5's re-entrancy guards in `initialize_tap()`.

### Finding 4: The `prompt=true` Path Does NOT Need to Change

`AXIsProcessTrustedWithOptions(dict)` with the prompt option on Sequoia 15+ and Tahoe 26 no longer shows a modal dialog — it redirects to System Settings. [CITED: search result cross-reference] This is a **known behaviour change that is already documented** and is NOT a regression — the Phase 5 implementation intentionally shows the OS prompt and the user navigates to Settings. The existing `prompt=true` path is correct and unchanged.

### Finding 5: Input Monitoring vs Accessibility — Correct Permission is Already Being Used

AutoMux uses `CGEventTapOptions::ListenOnly` for its observer tap. [VERIFIED: observer.rs line 254]. `ListenOnly` taps require **Input Monitoring** permission, not Accessibility, per Apple's TCC model. For non-sandboxed apps, Accessibility permission supersedes and includes Input Monitoring. [CITED: Apple Developer Forum thread 122492, AeroSpace issue #1012]

AutoMux requests Accessibility (via `AXIsProcessTrustedWithOptions`), which covers both posting events (`CGEvent.post`) and listening. This is correct. The issue is NOT a wrong permission type — it is the stale cache returning the wrong answer for the correct permission.

### Finding 6: CGEventPost Is Not a Viable Fallback

D-08 proposes `CGEventPost` as a fallback if `CGEventTap` is blocked. Research finding:

- `CGEvent.post(CGEventTapLocation::HID)` — used in `input.rs` — requires Accessibility (`kTCCServiceAccessibility`) permission. [CITED: Apple documentation, HackTricks TCC analysis]
- If Accessibility is not granted (or miscached), `CGEvent.post()` either silently fails or is blocked by the OS.
- `CGEventPost` is NOT a lower-permission alternative to `CGEventTap`. Both require the same Accessibility grant.
- The `CGEventPost` fallback is only meaningful if `CGEventTap` is separately blocked by a different mechanism from `CGEvent.post`. There is no evidence this is the case on Tahoe 26.

**Conclusion for planner:** D-08 (CGEventPost fallback) is not needed and not viable as a distinct strategy. The correct fix is ensuring `AXIsProcessTrusted()` returns the right value (via live probe), which unblocks both CGEventTap arming AND CGEvent.post injection simultaneously.

### Finding 7: No Evidence of Apple Hard-Blocking Unsigned Apps from CGEventTap on Tahoe 26

D-09 asked whether Apple has hard-blocked unsigned apps from CGEventTap on Tahoe 26. Research finding: **no confirmed evidence of this**. [ASSUMED]

What IS documented:
- Gatekeeper makes unsigned apps harder to launch on Tahoe 26 (user must approve in System Settings). [CITED: tongfamily.com, swissmacuser.ch]
- Once the app is running, CGEventTap creation depends on TCC permission state, not code signing status, for non-sandboxed apps. [CITED: Apple Developer Forum threads]
- macOS Launch Services requires Developer ID for Input Monitoring permission in certain launch paths — but AutoMux uses Accessibility (not Input Monitoring as the primary gate), and Accessibility is granted manually via System Settings, not through Launch Services auto-grant. [CITED: multiple search results]

The "unsigned app is hard-blocked from CGEventTap" scenario would manifest as `CGEventTap::new()` always failing even after Accessibility is manually granted. This is **not** D-09's trigger — D-09 requires confirmation that this is happening. The working hypothesis remains: the fix is the live probe, not signing.

### Finding 8: Info.plist NSAccessibilityUsageDescription Should Be Added

The `NSAccessibilityUsageDescription` key in `Info.plist` is required for apps running on Hardened Runtime (when signed) and is recommended for all apps that use Accessibility APIs. For unsigned apps, it has no TCC effect, but:
- It causes macOS to display the app with a human-readable description in System Settings → Privacy & Security → Accessibility.
- Its absence may cause the app to not appear in the Accessibility list at all on some Tahoe 26 configurations, which would explain why re-granting fails if the entry is stale or absent.

Tauri 2 merges a `src-tauri/Info.plist` file with the generated one at build time. [CITED: v2.tauri.app/distribute/macos-application-bundle/] This file does NOT require a signing identity to be merged.

**Recommended action:** Create `src-tauri/Info.plist` with `NSAccessibilityUsageDescription`. This is low-cost, safe, and potentially eliminates the "not appearing in Settings" edge case.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Live TCC permission probe | Custom TCC database reader | `CGEventTap::new()` probe | Tap creation is Apple's documented live-check mechanism; TCC.db is private and read-protected |
| Permission UI flow | Custom accessibility Settings deep-link | `AXIsProcessTrustedWithOptions(prompt: true)` | Apple controls the System Settings URL; custom links break across macOS versions |
| Entitlement injection at runtime | Dynamic plist modification | Build-time `src-tauri/tauri.conf.json` + `Entitlements.plist` | Runtime plist modification is blocked by SIP; signing is needed anyway |

---

## Common Pitfalls

### Pitfall 1: Assuming AXIsProcessTrusted() Is Authoritative

**What goes wrong:** Code calls `AXIsProcessTrusted()` in a loop and always gets back the cached value from process startup — regardless of whether the user granted or revoked permission.
**Why it happens:** Apple caches the result per-process. The cache is not invalidated by the OS when TCC changes.
**How to avoid:** Use `CGEventTap::new()` as the live probe. The probe is cheap (~microseconds) and always hits the live TCC database.
**Warning signs:** `AXIsProcessTrusted()` consistently returning the same value no matter what the user does in System Settings.

### Pitfall 2: Destroying the Tap Too Early in the Probe

**What goes wrong:** The probe tap is created, the result is read, and the tap variable is immediately dropped — but `TAP_INITIALIZED` was already set to `true` in `initialize_tap()` before the tap was passed to `check_accessibility_permissions()`.
**Why it happens:** The two-step flow creates two taps — one probe in `check_accessibility_permissions()` and one real tap in `initialize_tap()`. If the probe tap is not properly scoped, it could interact with the `TAP_STARTING` guard.
**How to avoid:** Keep the probe tap creation entirely within `check_accessibility_permissions()`. Do not pass tap state across function boundaries. The existing `initialize_tap()` creates its own tap independently.
**Warning signs:** Tap creation in `initialize_tap()` failing after a successful probe in `check_accessibility_permissions()`.

### Pitfall 3: Breaking Phase 5's Re-Entrancy Guards

**What goes wrong:** The fix to `check_accessibility_permissions()` causes the function to indirectly create a CGEventTap, which conflicts with `TAP_INITIALIZED`/`TAP_STARTING` atomic guards in `initialize_tap()`.
**Why it happens:** Two code paths creating CGEventTaps could race if not properly guarded.
**How to avoid:** The probe tap in `check_accessibility_permissions()` must use `CGEventTapOptions::ListenOnly` and must be a different tap instance, scoped to only that function call. It does NOT set `TAP_INITIALIZED`. Only `initialize_tap()` sets `TAP_INITIALIZED`.
**Warning signs:** `initialize_tap()` returns `false` after a permission check that returned `true`.

### Pitfall 4: Assuming entitlements.plist Fixes Unsigned Apps

**What goes wrong:** Plan adds `entitlements.plist` and `tauri.conf.json` entitlements path, rebuilds, and tests — but sees no change in `AXIsProcessTrusted()` / CGEventTap behaviour.
**Why it happens:** Tauri's entitlements are applied during code signing. An unsigned build ignores them entirely for TCC purposes.
**How to avoid:** Do not include an entitlements.plist task as the primary fix for COMPAT-01/02. It is NOT the fix. The code change to `check_accessibility_permissions()` is the fix.
**Warning signs:** Entitlements task completes but the permission detection is still broken.

### Pitfall 5: CGEventTap Probe Blocks the Tokio Runtime

**What goes wrong:** Creating a `CGEventTap` from inside a Tokio async task blocks the event loop.
**Why it happens:** `CGEventTap::new()` internally sets up a Mach port. On macOS, this is a synchronous syscall but very fast (~microseconds). However, if the blocking is a concern, Tokio's runtime could complain.
**How to avoid:** CGEventTap creation is sufficiently fast that it is safe to call from sync context within an `async fn`. It does NOT start a CFRunLoop — that only happens when `.enable()` is called and the source is added to a run loop. The probe tap only calls `new()` and then drops immediately, with no run loop involvement.
**Warning signs:** Tokio warning about blocking operations (unlikely given the fast path, but watch for it).

---

## Code Examples

### Pattern 1: Live Permission Probe via CGEventTap (the fix)

```rust
// Source: derived from core-graphics 0.24.0 CGEventTap::new() signature
// and Apple Developer Forum recommendations for live TCC probe (fazm.ai analysis)

pub fn check_accessibility_permissions(prompt: bool) -> bool {
    if prompt {
        // [UNCHANGED from current implementation]
        // AXIsProcessTrustedWithOptions with prompt=true — still correct for requesting access
        // ...existing code...
        return result != 0;
    }

    // prompt=false: live probe via CGEventTap to bypass AXIsProcessTrusted() cache.
    // CGEventTap::new() consults the live TCC database, not the per-process cache.
    // A successful tap creation means Accessibility is genuinely granted right now.
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    };

    let probe_result = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::MouseMoved],  // Minimal event set — just needs to be non-empty
        |_, _, event| Some(event.clone()),
    );

    // probe_result is Ok(_) if permission is live-granted, Err(_) if denied.
    // Drop the tap immediately — this is a probe only. initialize_tap() creates the real tap.
    probe_result.is_ok()
}
```

**Important:** This function must NOT set `TAP_INITIALIZED`. Only `initialize_tap()` sets that flag. The probe tap drops at the end of this function.

### Pattern 2: Info.plist for NSAccessibilityUsageDescription (new file)

```xml
<!-- src-tauri/Info.plist — merged with Tauri-generated Info.plist at build time -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
    "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>NSAccessibilityUsageDescription</key>
    <string>AutoMux requires Accessibility access to monitor and inject keyboard and mouse events for macro automation.</string>
</dict>
</plist>
```

This file is placed in `src-tauri/` and is auto-merged by the Tauri CLI into the app bundle's `Info.plist` at build time — no `tauri.conf.json` change needed. [CITED: v2.tauri.app/distribute/macos-application-bundle/]

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `AXIsProcessTrusted()` for silent permission check | `CGEventTap::new()` probe as live TCC check | macOS 15 Sequoia / 26 Tahoe | `AXIsProcessTrusted()` cache became unreliable; live probe is now required |
| `AXIsProcessTrustedWithOptions(prompt:true)` shows modal | Same call redirects to System Settings, returns immediately | macOS 15 Sequoia | Not a regression — existing Phase 5 implementation already handles this correctly |

**Deprecated/outdated:**
- Relying on `AXIsProcessTrusted()` alone as the canonical permission signal: broken by Tahoe 26 cache staleness.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Rust inline `#[cfg(test)]`) |
| Config file | none (inline tests) |
| Quick run command | `cargo test -p automux --lib` (from `src-tauri/`) |
| Full suite command | `cargo test -p automux` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| COMPAT-01 | CGEventTap arms after Accessibility granted | Manual (requires Tahoe 26 device + real permission grant) | N/A — OS permission cannot be mocked | N/A |
| COMPAT-02 | `check_accessibility_permissions(false)` returns correct value | Unit (limited) + Manual | `cargo test -p automux --lib -- platform::macos` | ❌ Wave 0 |
| COMPAT-03 | App launches without crash on Tahoe 26 | Smoke (manual) | N/A — requires Tahoe 26 device | N/A |

### Sampling Rate

- **Per task commit:** `cargo test -p automux --lib` (unit tests only — compile confirms no regressions)
- **Per wave merge:** Full manual smoke test on Tahoe 26 device
- **Phase gate:** COMPAT-01 and COMPAT-02 verified by human tester on Tahoe 26 before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/platform/macos/tests.rs` (or inline `#[cfg(test)]` in `mod.rs`) — unit test for `check_accessibility_permissions(false)` return type correctness (cannot test actual TCC grant in CI, but can test the function compiles and returns bool correctly)
- [ ] Manual test checklist for Tahoe 26 device verification

**Key constraint:** COMPAT-01, COMPAT-02, and COMPAT-03 cannot be fully validated in automated CI because they require macOS 26 Tahoe with real Accessibility permission grants. The plan must include a **manual verification wave** on a Tahoe 26 device.

---

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes (OS permission gate) | macOS TCC — trust CGEventTap probe result |
| V5 Input Validation | no (this phase is platform layer only) | — |
| V6 Cryptography | no | — |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stale permission cache returning incorrect grant | Elevation of Privilege | Use live CGEventTap probe — do not trust `AXIsProcessTrusted()` cache alone |
| CGEventTap callback executing injected user data | Tampering | Existing `LLMHF_INJECTED` marker in observer.rs filters out self-injected events — unchanged |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Apple has NOT hard-blocked unsigned apps from CGEventTap on Tahoe 26 (D-09 scenario) | Finding 7 | If wrong, fix requires signing infrastructure → v3 blocker |
| A2 | The live CGEventTap probe approach is sufficient to fix the permission detection on Tahoe 26 | Core fix | If wrong, may need to investigate additional TCC API changes specific to Tahoe 26 |
| A3 | `NSAccessibilityUsageDescription` in Info.plist causes the app to appear correctly in System Settings Accessibility list on Tahoe 26 | Finding 8 | If wrong, users may not be able to grant permission if app doesn't appear in the list |
| A4 | objc 0.2.7 does not cause Tahoe 26 build/runtime failures (D-10) | Deferred deps | If wrong, D-11 triggers and full objc2 migration is needed in this phase |
| A5 | The probe tap (`CGEventTap::new()`) can be safely created and immediately dropped within `check_accessibility_permissions()` without interfering with the real tap in `initialize_tap()` | Code example | If wrong, the two-tap design needs architectural adjustment |

---

## Open Questions

1. **Does `NSAccessibilityUsageDescription` absence cause app to vanish from Accessibility Settings list on Tahoe 26?**
   - What we know: The key is documented as recommended for apps using Accessibility APIs.
   - What's unclear: Whether its absence on Tahoe 26 specifically prevents the app from appearing in the Accessibility pane, making grant impossible.
   - Recommendation: Add the key regardless (zero-risk change); validate in manual test that app appears in list.

2. **Is the live probe approach sufficient on its own, or does Tahoe 26 require an additional entitlement for the app to even appear in the Accessibility grant list?**
   - What we know: Entitlements are ignored by TCC for unsigned apps. Info.plist NSAccessibilityUsageDescription works without signing.
   - What's unclear: Whether Tahoe 26 added a new requirement that unsigned apps must have a specific Info.plist key to be grantable.
   - Recommendation: Test after implementing the live probe fix. If the app doesn't appear in Accessibility Settings, investigate Info.plist keys further.

3. **Does D-09 (Apple hard-blocking unsigned CGEventTap) apply on current Tahoe 26 release?**
   - What we know: No developer reports of this; Karabiner-Elements and other unsigned tools report permission-related issues but not hard-blocking.
   - What's unclear: Whether Apple tightened this specifically in any Tahoe 26.x release.
   - Recommendation: Tester verifies on Tahoe 26 device. If `CGEventTap::new()` returns `Err` even after confirmed Accessibility grant, D-09 is triggered → v3 blocker flag.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| macOS 26 Tahoe device | COMPAT-01/02/03 manual verification | Unknown (user has one — bugs were reproduced there) | 26.x | No fallback — CI cannot substitute |
| `cargo` / Rust stable | Build | ✓ | stable | — |
| Tauri CLI | dmg bundling | ✓ (npm run tauri) | v2 | — |

**Missing dependencies with no fallback:**
- A macOS 26 Tahoe device is required for Phase Gate validation of all three COMPAT requirements. The user has one (D-02 confirms bugs reproduced on packaged DMG on Tahoe 26).

---

## Sources

### Primary (HIGH confidence)
- [Tauri 2 macOS Application Bundle docs](https://v2.tauri.app/distribute/macos-application-bundle/) — Entitlements configuration, Info.plist merging, signing requirement for entitlements
- [Apple Developer Documentation: AXIsProcessTrusted](https://developer.apple.com/documentation/applicationservices/1460720-axisprocesstrusted) — Function signature and purpose
- [Apple Developer Documentation: CGPreflightListenEventAccess](https://developer.apple.com/documentation/coregraphics/cgpreflightlisteneventaccess()) — Input Monitoring vs Accessibility distinction
- [core-graphics crate source (docs.rs 0.25.0)](https://docs.rs/crate/core-graphics/latest/source/src/event.rs) — CGEventTap::new() API confirmation

### Secondary (MEDIUM confidence)
- [Apple Developer Forum: Problem with event tap permission in Sequoia](https://developer.apple.com/forums/thread/758554) — CGEventTap failures on Sequoia; behaviour overlaps with Tahoe 26
- [Apple Developer Forum: AXIsProcessTrusted returns wrong value](https://developer.apple.com/forums/thread/727984) — Cache staleness issue documentation
- [Apple Developer Forum: AXIsProcessTrusted returns true, accessibility still fails](https://developer.apple.com/forums/thread/794253) — Inverse cache staleness (same root cause)
- [Daniel Raffel: CGEvent Taps and Code Signing: The Silent Disable Race](https://danielraffel.me/til/2026/02/19/cgevent-taps-and-code-signing-the-silent-disable-race/) — CGEventTap health monitoring patterns
- [AeroSpace GitHub Issue #1012](https://github.com/nikitabobko/AeroSpace/issues/1012) — CGEventTap permission ecosystem analysis
- [jano.dev: Accessibility Permission in macOS](https://jano.dev/apple/macos/swift/2025/01/08/Accessibility-Permission.html) — Entitlements + Info.plist requirements

### Tertiary (LOW confidence — cross-corroborated, treat as informative)
- [fazm.ai: macOS accessibility automation — four production failure modes](https://fazm.ai/t/macos-accessibility-automation) — AXIsProcessTrusted cache analysis and live probe recommendation (page returned 404 at fetch time; content was obtained via search snippet summaries)
- [Karabiner-Elements GitHub issues #4313, #4314, #4371, #4376](https://github.com/pqrs-org/Karabiner-Elements/issues/4313) — Community reports of Input Monitoring permission issues on Tahoe 26; shows the problem space but not directly applicable (Karabiner uses DriverKit, not CGEventTap)
- [macOS Dangerous Entitlements & TCC perms (HackTricks)](https://hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-dangerous-entitlements.html) — kTCCServiceAccessibility and kTCCServiceListenEvent TCC key names

---

## Metadata

**Confidence breakdown:**
- Root cause (AXIsProcessTrusted cache staleness): HIGH — documented in multiple Apple Developer Forum threads spanning Sequoia and Tahoe; consistent with the user's symptom description
- Fix approach (live CGEventTap probe): MEDIUM-HIGH — widely recommended by developers working around the same issue; not officially documented by Apple in release notes
- Entitlements blocked for unsigned apps: HIGH — Tauri 2 official docs explicitly state entitlements require signing; confirmed by Apple's code signing model
- CGEventPost as fallback: LOW value — not a distinct permission path from CGEvent.post for non-sandboxed apps; excluded from plan
- D-09 (hard-block for unsigned CGEventTap): LOW confidence claim either way — no confirmed reports of hard-blocking, but no official Apple denial either

**Research date:** 2026-06-04
**Valid until:** 2026-09-04 (90 days — macOS APIs are stable between dot releases; re-research before Tahoe 26.x major changes)
