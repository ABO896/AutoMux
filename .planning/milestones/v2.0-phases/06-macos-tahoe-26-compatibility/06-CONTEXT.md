# Phase 6: macOS Tahoe 26 Compatibility - Context

**Gathered:** 2026-06-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Diagnose and fix what is broken on macOS 26 Tahoe so that AutoMux is fully functional: macros fire correctly, Accessibility permission is detected accurately, and the app launches without errors. The root issue has been identified: `AXIsProcessTrusted()` returns false on Tahoe 26 even when Accessibility is granted in System Settings, which prevents CGEventTap from arming, which in turn blocks all macro execution. No new features, no UI redesign, no Windows changes.

</domain>

<decisions>
## Implementation Decisions

### Confirmed failures on Tahoe 26

- **D-01:** Both COMPAT-01 and COMPAT-02 are failing, and causally linked: `AXIsProcessTrusted()` persistently returns `false` on Tahoe 26 even after the user grants Accessibility in System Settings → CGEventTap never arms → no macros fire.
- **D-02:** Tested on a **packaged DMG / installed .app** — not a dev-path issue. The permission detection failure is a genuine Tahoe 26 regression.
- **D-03:** The Request Access button correctly shows the OS prompt and routes the user to Accessibility Settings, but post-grant detection still fails. The UI flow is correct; `AXIsProcessTrusted()` is the broken link.
- **D-04:** App launches fine on Tahoe 26 (COMPAT-03 is NOT failing) — no crash on launch, no missing entitlement error at startup. The failure is strictly at the permissions-detection and input-injection layer.

### Entitlements

- **D-05:** ~~Add an `entitlements.plist` as part of this phase.~~ **REVISED BY RESEARCH (2026-06-04):** Entitlements in an unsigned `.app` bundle are ignored by TCC at runtime — confirmed by Tauri 2 official docs and Apple's code signing model. The entitlements.plist approach cannot fix COMPAT-01/02 for an unsigned build. The plan substitutes `src-tauri/Info.plist` with `NSAccessibilityUsageDescription` (which works without signing) plus a live CGEventTap probe replacing the stale `AXIsProcessTrusted()` call. No entitlements.plist will be created in this phase.
- **D-06:** Scope of entitlements file: **minimal only** — include only the entitlements confirmed as required for Accessibility permission detection and CGEventTap input injection on Tahoe 26. Do not add a full Tauri template or anticipate future needs. Let researcher determine the exact entitlements needed.
- **D-07:** Code signing itself stays deferred to v3. The entitlements file is separate from signing.

### CGEventTap fallback strategy

- **D-08:** If fixing `AXIsProcessTrusted()` (via entitlements or API changes) does not fully restore macro firing, the fallback stays **within current CGEvent-family APIs** — no new entitlements beyond what fixes permissions, no signing setup. Researcher should also investigate whether `CGEventPost` (direct event posting without a tap) works as a fallback injection method on Tahoe 26 in case CGEventTap itself is blocked.
- **D-09:** If Apple has hard-blocked unsigned apps from CGEventTap on Tahoe 26, this becomes a v3 blocker (requires signing + notarization). Do not implement workarounds requiring new signing infrastructure in this phase.

### Legacy dep cleanup (objc 0.2.7)

- **D-10:** Approach is **reactive**: attempt to build and run on Tahoe 26 first. Do not spend research time investigating potential objc 0.2.7 issues preemptively.
- **D-11:** If Tahoe 26 causes build or runtime failures from `objc 0.2.7`, do the **full migration** from `objc 0.2.7` → `objc2` in this phase (not a narrow patch). This is the forcing function for the tech debt deferred from Phase 5. If `objc 0.2.7` does NOT cause failures on Tahoe 26, migration stays deferred.

### Claude's Discretion

- Exact entitlements key names and values to include in the `.plist` — determined by researcher findings.
- Whether the entitlements fix requires changes to `tauri.conf.json` or a standalone `.entitlements` file — implementation choice for planner.
- Whether the `AXIsProcessTrusted()` regression is fixed by entitlements alone or also requires an API call change (e.g., different options dict) — determined by researcher.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Permissions + tap initialization (Phase 5 fix — must understand before changing)
- `src-tauri/src/platform/macos/mod.rs` — `check_accessibility_permissions(prompt: bool)` — the function both `check_accessibility` and `request_accessibility` IPC commands call; the false-negative on Tahoe 26 likely originates here
- `src-tauri/src/platform/macos/observer.rs` — `initialize_tap()` and `TAP_INITIALIZED` AtomicBool — tap arming function; Phase 5 made this re-entrant safe; must remain safe after any Tahoe 26 fix
- `src-tauri/src/ipc/mod.rs` — `request_accessibility` and `check_accessibility` IPC handlers — where D-03 (post-grant arming) lives from Phase 5
- `src/App.tsx` — `handleRequestAccess()` and the polling `createEffect` — frontend permission flow; must continue to work correctly after the fix

### Build configuration (entitlements)
- `src-tauri/tauri.conf.json` — current `signingIdentity: null`, `entitlements: null` — entitlements.plist path will be added here (D-05)
- `src-tauri/Cargo.toml` — `objc = "0.2.7"` direct dep; `objc2 = "0.6.4"` already present — if D-11 triggers, migration happens here

### Requirements
- `.planning/REQUIREMENTS.md` — COMPAT-01, COMPAT-02, COMPAT-03 acceptance criteria
- `.planning/ROADMAP.md` — Phase 6 success criteria and phase dependencies

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TAP_INITIALIZED: AtomicBool` in `observer.rs` — Phase 5 made `initialize_tap()` re-entrant safe via this guard. Any Tahoe 26 fix that triggers tap arming must go through the same guard.
- 3s polling `createEffect` in `App.tsx` — already calls `check_accessibility` on interval; the Tahoe 26 fix in the backend will automatically surface via this existing poll if `AXIsProcessTrusted()` is made correct.
- `check_accessibility_permissions(false)` → `AXIsProcessTrusted()` — the silent check path used by polling. This is where the false-negative fix must land.

### Established Patterns
- Phase 5 pattern: fix lives in the **backend handler**, not the frontend — `check_accessibility` backend handler arming the tap before returning `true`. Same pattern applies for Tahoe 26 fix.
- `if let Ok(event) = CGEvent::new(...)` — platform errors silently dropped; no error propagation for CGEvent failures. This means a silently-failing CGEventTap won't surface errors — must confirm tap is actually firing, not just "not erroring."

### Integration Points
- `AXIsProcessTrusted()` / `AXIsProcessTrustedWithOptions` in `platform/macos/mod.rs` → fix propagates to `check_accessibility` IPC → 3s polling in `App.tsx` surfaces the corrected state
- entitlements.plist → `tauri.conf.json` `entitlements` field → Tauri bundles it into the `.app` — no code changes needed on the app logic side, only build config

</code_context>

<specifics>
## Specific Ideas

No specific UI or UX references for this phase — it's a compatibility fix with no visible feature changes beyond the permission indicator showing "granted" correctly.

</specifics>

<deferred>
## Deferred Ideas

- **objc 0.2.7 → objc2 full migration** — Only triggered in this phase if Tahoe 26 build/runtime failures confirm it's needed (D-10/D-11). If not triggered here, remains as tech debt for a future phase.
- **Code signing + notarization** — Stays in v3 per existing plan. If CGEventTap turns out to be hard-blocked for unsigned apps on Tahoe 26 (D-09), this becomes a v3 blocker.

</deferred>

---

*Phase: 6-macOS Tahoe 26 Compatibility*
*Context gathered: 2026-06-03*
