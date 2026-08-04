---
phase: 05-macos-permissions-reliability
verified: 2026-06-02T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "RELY-06 functional — grant accessibility directly in System Settings without clicking Request Access"
    expected: "Within 3 seconds, macros become active and the UI shows 'Granted' — no app restart required"
    why_human: "Requires a macOS device with Accessibility not yet granted; cannot be verified by static code analysis"
  - test: "PERM-01 functional — click Grant Access, approve OS dialog"
    expected: "UI shows 'Pending...' (amber dot, no Grant Access button) immediately after click; within 3s of approval UI shows 'Granted'"
    why_human: "Requires interaction with the macOS system permission dialog at runtime"
  - test: "PERM-01 fallback — click Grant Access, dismiss OS dialog without approving"
    expected: "After 30 seconds the 'Pending...' indicator disappears and the UI reverts to 'Denied' with the Grant Access button"
    why_human: "Requires runtime interaction with the macOS permission dialog and waiting 30 seconds"
  - test: "BUILD-02 functional — cargo build on macOS produces no block v0.1.6 deprecation warnings"
    expected: "cargo build output contains no lines matching 'block', 'deprecat', or 'cocoa'"
    why_human: "Verification environment is darwin but cannot run cargo build in this session; static analysis confirms cocoa is removed from Cargo.toml"
---

# Phase 5: macOS Permissions & Reliability Verification Report

**Phase Goal:** macOS permissions reliability — arm CGEventTap on post-launch accessibility grant (RELY-06), add pending UI state (PERM-01), eliminate block v0.1.6 deprecation warning (BUILD-02)
**Verified:** 2026-06-02
**Status:** human_needed — all code truths VERIFIED; runtime behavioral checks require macOS device
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A user who grants accessibility directly in System Settings sees macros activate without restarting — CGEventTap arms within 3s poll | ✓ VERIFIED (code) / ? HUMAN (runtime) | `check_accessibility` (ipc/mod.rs:193–206) calls `initialize_tap()` when `granted == true` inside `#[cfg(target_os = "macos")]` block; polling effect in App.tsx calls `check_accessibility` every 3s |
| 2 | The Rust build produces no deprecation warnings related to block v0.1.6 on macOS | ✓ VERIFIED (code) / ? HUMAN (build) | `cocoa = "0.26.1"` absent from Cargo.toml (grep returns 0 matches); `block2 = "0.6.2"` retained at line 35; no other reference to cocoa in Cargo.toml |
| 3 | After clicking Grant Access and approving the OS dialog, the UI shows 'Pending...' (not 'Denied') while waiting for the 3s poll | ✓ VERIFIED (code) / ? HUMAN (runtime) | `handleRequestAccess` sets `accessibilityPending(true)` before `invoke("request_accessibility")`; pending Show branch renders amber dot + "Pending…" at App.tsx:474–492 |
| 4 | The pending indicator disappears and the UI shows 'Granted' within 3s of the OS dialog approval | ✓ VERIFIED (code) / ? HUMAN (runtime) | 3s polling effect at App.tsx:163–174 calls `clearPending()` when `check_accessibility` returns true; `clearPending()` sets `accessibilityPending(false)` and cancels the 30s timeout |
| 5 | If the user dismisses the OS dialog without approving, the pending indicator disappears after 30 seconds and the UI reverts to 'Denied' with the Grant Access button | ✓ VERIFIED (code) / ? HUMAN (runtime) | `handleRequestAccess` starts a 30s `setTimeout` at App.tsx:197–200 that calls `setAccessibilityPending(false)` and nulls `_pendingTimeoutId`; Grant Access button guard `accessibility() === false && !accessibilityPending()` at line 503 ensures button reappears |

**Score:** 5/5 truths verified at code level

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/ipc/mod.rs` | check_accessibility with post-grant tap initialization | ✓ VERIFIED | Lines 193–206 contain `initialize_tap()` call inside `#[cfg(target_os = "macos")]` block; `#[cfg(not(target_os = "macos"))]` branch returns `Ok(true)` unchanged |
| `src-tauri/Cargo.toml` | Cargo manifest without cocoa dependency | ✓ VERIFIED | `grep "cocoa" Cargo.toml` returns 0 matches; `block2 = "0.6.2"` retained at line 35; macOS dep section contains 8 crates (core-graphics, core-foundation, core-foundation-sys, objc, objc2, objc2-app-kit, objc2-foundation, block2) |
| `src/App.tsx` | accessibilityPending signal + clearPending helper + 3-branch Show UI | ✓ VERIFIED | Signal declared at line 82; `clearPending()` defined at lines 187–193; 3-branch Show at lines 471–501; button guard at line 503 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `ipc/mod.rs:check_accessibility` | `platform/macos/observer.rs:initialize_tap` | direct function call inside `#[cfg(target_os = "macos")]` block | ✓ WIRED | `crate::platform::macos::observer::initialize_tap()` at ipc/mod.rs:198; pattern confirmed present |
| `App.tsx:handleRequestAccess` | `App.tsx:_pendingTimeoutId` | setTimeout sets `_pendingTimeoutId`; `clearPending()` clears it | ✓ WIRED | `_pendingTimeoutId = setTimeout(...)` at line 197; `clearTimeout(_pendingTimeoutId)` in `clearPending()` at lines 189–192; 6 occurrences of `_pendingTimeoutId` total |
| `App.tsx:createEffect (3s poll)` | `App.tsx:clearPending` | `if (ok) clearPending()` inside setInterval callback | ✓ WIRED | `if (ok) clearPending()` at App.tsx:168; inside the setInterval at lines 163–174 |
| `App.tsx:onCleanup` | `App.tsx:_pendingTimeoutId` | `clearPending()` called in the existing onCleanup block | ✓ WIRED | `clearPending()` at line 182 inside the `onCleanup(() => {...})` block at lines 177–183; `grep -A5 "onCleanup" | grep -c "clearPending"` returns 1 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `src/App.tsx` (accessibility status UI) | `accessibility()` | `invoke<boolean>("check_accessibility")` → Rust `AXIsProcessTrusted()` | Yes — real OS permission query | ✓ FLOWING |
| `src/App.tsx` (pending state) | `accessibilityPending()` | State machine: `setAccessibilityPending(true)` in `handleRequestAccess`, cleared by `clearPending()` on poll grant or 30s timeout | Yes — driven by real OS interaction timing | ✓ FLOWING |

### Behavioral Spot-Checks

Step 7b: SKIPPED — requires running Tauri app on macOS with actual Accessibility permission dialog interaction; no standalone runnable entry points applicable to these behaviors.

### Probe Execution

Step 7c: No probe scripts found under `scripts/*/tests/probe-*.sh`. No probes declared in PLAN frontmatter.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| PERM-01 | 05-02-PLAN.md | macOS app correctly detects accessibility permission as granted after user clicks Request Access and approves the OS prompt — no false "not granted" state persists | ✓ SATISFIED | `accessibilityPending` signal bridges the gap between click and poll confirmation; 3-branch Show never shows Denied while pending; Grant Access button hidden during pending window |
| RELY-06 | 05-01-PLAN.md | Granting accessibility in System Settings directly arms CGEventTap without restart | ✓ SATISFIED | `check_accessibility` calls `initialize_tap()` on grant; frontend polls every 3s; tap arming is idempotent via `TAP_INITIALIZED` AtomicBool CAS |
| BUILD-02 | 05-01-PLAN.md | block v0.1.6 macOS deprecation resolved | ✓ SATISFIED (code) / ? HUMAN (build output) | `cocoa = "0.26.1"` fully removed from Cargo.toml — confirmed by 0 grep matches; this was the sole source of the block v0.1.6 transitive pull |

**Requirement count check:** REQUIREMENTS.md maps PERM-01, RELY-06, BUILD-02 to Phase 5. All 3 are claimed in plan frontmatter and verified above. No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/App.tsx` | 232 | `// WR-05: id is generated by the backend; supply a placeholder that gets overwritten.` | Info | Pre-existing comment, not a debt marker; documents an architectural invariant. Not introduced by this phase. |

No `TBD`, `FIXME`, or `XXX` markers found in `src-tauri/src/ipc/mod.rs` or `src/App.tsx`. No stub returns or disconnected wiring detected in phase-modified code.

### Human Verification Required

#### 1. RELY-06 Runtime: Post-grant tap arming without clicking Request Access

**Test:** Revoke Accessibility for AutoMux in System Settings. Launch the app (macros show inactive). Open System Settings → Privacy & Security → Accessibility. Grant permission to AutoMux directly WITHOUT clicking "Grant Access" in the app. Wait up to 3 seconds.
**Expected:** The UI shows "Granted" and macros become active without requiring an app restart.
**Why human:** Requires a macOS device with an unsigned app build and the OS permission dialog; cannot simulate `AXIsProcessTrusted()` returning true in a static analysis context.

#### 2. PERM-01 Runtime: Pending indicator on Grant Access click

**Test:** Revoke Accessibility. Launch the app (shows "Denied" + "Grant Access" button). Click "Grant Access". Observe UI immediately. Then approve the OS dialog.
**Expected:** Immediately after clicking: UI shows amber "Pending…" dot and the Grant Access button disappears. Within 3 seconds of approving: UI shows "Granted", pending disappears.
**Why human:** Requires OS permission dialog interaction at runtime.

#### 3. PERM-01 Runtime: 30-second fallback reverts pending to denied

**Test:** Revoke Accessibility. Launch app. Click "Grant Access". When the OS dialog appears, dismiss it WITHOUT approving. Wait 30 seconds.
**Expected:** After 30 seconds, "Pending…" disappears and UI reverts to "Denied" with the Grant Access button re-appearing.
**Why human:** Requires OS dialog dismissal and a 30-second wait; cannot be automated without a running app.

#### 4. BUILD-02 Runtime: cargo build output clean on macOS

**Test:** On a macOS host, run `cargo build 2>&1 | grep -E "block|cocoa|deprecat|error\["` from the `src-tauri/` directory.
**Expected:** No matching output lines. Build exits 0.
**Why human:** Build environment not available in this verification session; static analysis confirms the structural fix (cocoa removed) but cannot execute the compiler.

### Gaps Summary

No gaps found. All 5 observable truths are verified at the code level. All 3 required artifacts exist, are substantive, and are properly wired. All 3 requirement IDs (PERM-01, RELY-06, BUILD-02) are satisfied by the implementation. No debt markers or stubs introduced. Status is `human_needed` because runtime behavioral verification on a macOS device is required to confirm the OS-dialog interaction flows work as designed — the code is fully correct, the human checks are behavioral confirmation only.

---

_Verified: 2026-06-02_
_Verifier: Claude (gsd-verifier)_
