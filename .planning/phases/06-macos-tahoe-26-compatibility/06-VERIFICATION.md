---
phase: 06-macos-tahoe-26-compatibility
verified: 2026-06-04T22:00:00Z
status: human_needed
score: 5/7 must-haves verified
overrides_applied: 0
re_verification: false
human_verification:
  - test: "SCENARIO A — COMPAT-02: Permissions detection updates without restart"
    expected: "Permission indicator in AutoMux UI updates to 'granted' within ~3 seconds after approving in System Settings → Privacy & Security → Accessibility — no app restart needed"
    why_human: "Live TCC probe behavior requires a real Tahoe 26 system with Accessibility grants; cannot be replicated with grep or cargo build"
  - test: "SCENARIO B — COMPAT-01: Macros fire after Accessibility granted"
    expected: "A mouse click or key press macro fires in the target application after CGEventTap arms; no action suppressed or silently dropped"
    why_human: "CGEventTap input injection requires a live system; cannot be verified statically"
  - test: "SCENARIO C — COMPAT-03: Clean launch, no Console errors"
    expected: "No com.alvaro.automux crash report, no 'entitlement' error, no CGEventTap failure message in Console.app at launch"
    why_human: "Runtime behavior on Tahoe 26; requires the physical device and Console.app inspection"
  - test: "SCENARIO D — App appears in Accessibility Settings list with description"
    expected: "AutoMux appears in System Settings → Privacy & Security → Accessibility with the string 'AutoMux requires Accessibility access to monitor and inject keyboard and mouse events for macro automation.'"
    why_human: "System Settings UI appearance requires Tahoe 26 device; Info.plist merge can be code-verified but the final rendered appearance cannot"
  - test: "DMG rebuild required before device testing"
    expected: "A new DMG must be built (npm run tauri build) to include the CR-01 and CR-02 fixes committed after the existing artifact (a2d47a4). The artifact at src-tauri/target/release/bundle/dmg/AutoMux_1.2.0_aarch64.dmg predates both post-review fixes."
    why_human: "The developer must decide whether to rebuild or test with the stale artifact; this is a process decision, not a code check"
---

# Phase 6: macOS Tahoe 26 Compatibility — Verification Report

**Phase Goal:** Fix macOS Tahoe 26 compatibility — AXIsProcessTrusted() stale cache bug, Info.plist NSAccessibilityUsageDescription, DMG build for device testing
**Verified:** 2026-06-04T22:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | check_accessibility_permissions(false) consults the live TCC database, not the per-process AXIsProcessTrusted cache | VERIFIED | mod.rs:20-35 — `AXIsProcessTrusted` (non-WithOptions) is absent; `CGEventTap::new` with `TailAppendEventTap + ListenOnly` is the sole implementation of the !prompt path |
| 2 | A successful CGEventTap::new() probe returns true from check_accessibility_permissions(false) | VERIFIED | mod.rs:35: `return probe.is_ok();` — the probe result directly determines the return value |
| 3 | A failed CGEventTap::new() probe returns false from check_accessibility_permissions(false) | VERIFIED | Same line — `is_ok()` returns false when `CGEventTap::new()` fails; no fallback overrides it |
| 4 | The probe tap does not set TAP_INITIALIZED or interact with TAP_STARTING | VERIFIED | mod.rs grep for TAP_INITIALIZED and TAP_STARTING yields only comment text (line 24); zero code references |
| 5 | The app bundle's Info.plist includes NSAccessibilityUsageDescription so it appears in System Settings Accessibility list | VERIFIED (wiring confirmed, runtime appearance needs human) | `src-tauri/Info.plist` contains `NSAccessibilityUsageDescription`; `tauri.conf.json` `bundle.macOS.infoPlist` = "Info.plist" (CR-01 fix, commit fe7b9b0) — Tauri 2.9.1 `tauri-utils` uses `info_plist: Option<PathBuf>` field to merge this file at build time |
| 6 | The project compiles clean on macOS with cargo build | VERIFIED | REVIEW-FIX.md confirms cargo build passed after CR-01 and CR-02 fixes; no new errors or warnings introduced |
| 7 | Macros fire correctly on macOS 26 Tahoe — CGEventTap arms after Accessibility is granted (06-02 plan truth) | HUMAN NEEDED | Requires Tahoe 26 device — CGEventTap arming and injection cannot be verified statically |

**Score:** 5/7 code-verifiable truths VERIFIED; 2 require human device testing (Truths 5 and 7 have the human components noted above)

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/platform/macos/mod.rs` | check_accessibility_permissions() with live CGEventTap probe | VERIFIED | File modified; contains `CGEventTap::new` at line 28; `TailAppendEventTap` placement (CR-02 fix); `AXIsProcessTrusted` (stale-cache call) absent |
| `src-tauri/Info.plist` | NSAccessibilityUsageDescription key for System Settings | VERIFIED | File exists, 9 lines, valid XML; contains `NSAccessibilityUsageDescription` (count 1); contains no `com.apple.security` keys (count 0) |
| `src-tauri/tauri.conf.json` (CR-01 fix) | bundle.macOS.infoPlist = "Info.plist" to wire plist into build | VERIFIED | `"infoPlist": "Info.plist"` present at line 49; this is the correct tauri-utils 2.9.1 field (`info_plist: Option<PathBuf>`) |
| `src-tauri/target/release/bundle/dmg/AutoMux_1.2.0_aarch64.dmg` | Release DMG for Tahoe 26 device testing | STALE | DMG exists but was built at commit a2d47a4 — BEFORE CR-01 (fe7b9b0) and CR-02 (3294a36) fixes. The artifact does not contain the correct probe placement or infoPlist wiring. A new build is required before device testing. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src-tauri/src/platform/macos/mod.rs` | `src-tauri/src/ipc/mod.rs:check_accessibility` | `check_accessibility_permissions(false)` return value | WIRED | ipc/mod.rs:196 calls `crate::platform::macos::check_accessibility_permissions(false)` and passes result to `initialize_tap()` at line 198 |
| `src-tauri/Info.plist` | `AutoMux.app/Contents/Info.plist` | Tauri CLI merge via `bundle.macOS.infoPlist` | WIRED (build-time) | tauri.conf.json line 49: `"infoPlist": "Info.plist"` — Tauri 2.9.1 reads this field as `PathBuf` and merges at bundle time |
| `check_accessibility_permissions(false)` | CGEventTap arming in ipc/mod.rs | returns true when live Accessibility permission exists | WIRED | ipc/mod.rs:198: `if granted { crate::platform::macos::observer::initialize_tap(); }` |

---

### Data-Flow Trace (Level 4)

Not applicable — this phase modifies a permission-check function and a build-time configuration file, not a data-rendering component. No state variable or JSX rendering to trace.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `AXIsProcessTrusted` stale-cache call absent from !prompt path | `grep -c "AXIsProcessTrusted() !=" src-tauri/src/platform/macos/mod.rs` | 0 | PASS |
| `CGEventTap::new` present in !prompt path | `grep -n "CGEventTap::new" src-tauri/src/platform/macos/mod.rs` | line 28 | PASS |
| Probe uses TailAppendEventTap (CR-02 fix) | `grep "TailAppendEventTap" src-tauri/src/platform/macos/mod.rs` | line 30 | PASS |
| TAP_INITIALIZED/TAP_STARTING not in code | `grep -n "TAP_INITIALIZED\|TAP_STARTING" src-tauri/src/platform/macos/mod.rs` | comment lines only | PASS |
| NSAccessibilityUsageDescription in Info.plist | `grep -c "NSAccessibilityUsageDescription" src-tauri/Info.plist` | 1 | PASS |
| No com.apple.security keys in Info.plist | `grep -c "com.apple.security" src-tauri/Info.plist` | 0 | PASS |
| infoPlist wired in tauri.conf.json | `grep "infoPlist" src-tauri/tauri.conf.json` | line 49: `"infoPlist": "Info.plist"` | PASS |
| Build passes (per REVIEW-FIX.md) | `cargo build` post CR fixes | 0 errors per REVIEW-FIX.md | PASS (claimed; not re-run by verifier) |

---

### Probe Execution

No `probe-*.sh` scripts found for this phase. Step 7c: SKIPPED (no conventional probe scripts).

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| COMPAT-01 | 06-01-PLAN.md, 06-02-PLAN.md | User's macros fire correctly on macOS 26 Tahoe — CGEventTap input injection works | NEEDS HUMAN | Code fix is in place (probe returns true when TCC grants); actual macro firing on Tahoe 26 requires device test |
| COMPAT-02 | 06-01-PLAN.md, 06-02-PLAN.md | Permissions detection accurately reports granted/denied on macOS 26 Tahoe | NEEDS HUMAN | Live probe logic is code-verified VERIFIED; runtime behavior on Tahoe 26 (no restart needed, 3s update) requires device test |
| COMPAT-03 | 06-01-PLAN.md, 06-02-PLAN.md | App launches on macOS 26 Tahoe without crashes or entitlement errors | NEEDS HUMAN | `NSAccessibilityUsageDescription` is wired; Console.app/launch behavior requires device test |

All three requirement IDs declared in plan frontmatter are present in REQUIREMENTS.md and mapped to Phase 6. No orphaned requirements found.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src-tauri/src/platform/macos/mod.rs` | 40 | `CString::new("...").unwrap()` in prompt=true branch | Warning (WR-02 / IN-01 from review) | Not in the probe path; would panic on null allocation only under memory exhaustion in the user-dialog branch. Not a phase blocker. |
| No TBD / FIXME / XXX debt markers found | — | — | — | Debt-marker gate: CLEAR |

No unreferenced TBD, FIXME, or XXX markers found in the two files modified by this phase. The `@safety-officer:` comment on line 24 of mod.rs is documentation convention per CLAUDE.md, not a debt marker. Debt marker gate: PASSED.

---

### Critical Finding: Existing DMG Predates Post-Review Fixes

The DMG artifact at `src-tauri/target/release/bundle/dmg/AutoMux_1.2.0_aarch64.dmg` was built at commit `a2d47a4`. The two critical code review fixes were committed afterwards:

- `fe7b9b0` (CR-01): Wire `infoPlist` into `tauri.conf.json` — means the bundled `Info.plist` was NOT merged in the existing DMG
- `3294a36` (CR-02): Change probe placement to `TailAppendEventTap` — means the existing DMG uses `HeadInsertEventTap` which can produce false negatives on macOS 15+

**A new DMG must be built before Tahoe 26 device testing.** Testing the existing artifact would not validate the fixes in their current corrected state.

---

### Human Verification Required

#### 1. Build a fresh DMG (prerequisite)

**Test:** From the project root, run `npm run tauri build` to produce a new DMG that includes the CR-01 and CR-02 fixes.
**Expected:** Build exits 0; a new DMG appears in `src-tauri/target/release/bundle/dmg/`.
**Why human:** The developer must trigger the build and confirm the DMG path before proceeding to device tests.

#### 2. SCENARIO A — COMPAT-02: Permissions detection updates without restart

**Test:** On a macOS 26 Tahoe device — remove AutoMux from the Accessibility list in System Settings. Launch AutoMux. Confirm it shows "not granted". Grant Accessibility. Switch back to AutoMux and wait up to 3 seconds.
**Expected:** Permission indicator updates to "granted" within ~3 seconds — no app restart needed.
**Why human:** Live TCC polling behavior on Tahoe 26 cannot be verified without the device.

#### 3. SCENARIO B — COMPAT-01: Macros fire after Accessibility granted

**Test:** With Accessibility granted (from Scenario A), create or enable a macro (mouse click or key press). Enable and trigger it.
**Expected:** The click or key press fires in the target application. If nothing happens, CGEventTap failed to arm.
**Why human:** CGEventTap input injection requires a live system and target app.

#### 4. SCENARIO C — COMPAT-03: Clean launch, no Console errors

**Test:** Check macOS Console.app for crash reports or error messages from AutoMux at launch. Look for: `com.alvaro.automux` crash reports, "entitlement" errors, "CGEventTap" failure messages.
**Expected:** No crash, no entitlement error, no framework exception.
**Why human:** Runtime launch behavior on Tahoe 26 requires the device.

#### 5. SCENARIO D — App appears in Accessibility Settings with description

**Test:** Open System Settings → Privacy & Security → Accessibility. Find AutoMux in the list.
**Expected:** AutoMux appears with the usage description "AutoMux requires Accessibility access to monitor and inject keyboard and mouse events for macro automation."
**Why human:** System Settings UI appearance requires the device. Code-level wiring (`infoPlist` in tauri.conf.json) is verified, but the merge outcome in the bundled app must be confirmed on device.

---

### Gaps Summary

No code-level FAILED items. All statically verifiable truths are VERIFIED. The phase is blocked on human device testing (Tahoe 26 not available at verification time), and the existing DMG artifact is stale and must be rebuilt before use.

**Before device testing:** rebuild the DMG (`npm run tauri build`) to pick up CR-01 and CR-02 fixes. The current artifact at `AutoMux_1.2.0_aarch64.dmg` does not contain either fix.

---

_Verified: 2026-06-04T22:00:00Z_
_Verifier: Claude (gsd-verifier)_
