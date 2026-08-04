---
status: resolved
trigger: "G-09-1c-targeted-macro-not-firing: A macro configured with a specific target-process (app-scoped, not \"global\"/system-wide) does not fire at all, while a global-scope macro on the same build does fire."
created: 2026-07-22T00:00:00Z
updated: 2026-08-04T00:00:00Z
resolved_by: "09-07-PLAN.md (NSWorkspace observer hardening) + 09-08-PLAN.md (controlled target-app select fix); independently re-verified with no regression in 09-VERIFICATION.md (2026-07-22T21:30:00Z) and confirmed still working after Phase 9's device UAT (T9.1-T9.7, 2026-08-04) and Phase 10's component extraction."
---

## Current Focus

hypothesis: CONFIRMED (with caveat) — active_app tracking in platform/macos/observer.rs is the only mechanism that updates AppState.active_app after the initial launch-time snapshot; its NSWorkspaceDidActivateApplicationNotification observer registration has zero error handling/logging on failure, making a silent registration failure or non-firing callback indistinguishable from working code via static analysis alone. The targeting comparison logic itself (state/mod.rs Gate 3 + reevaluate_all_macros) is verified correct and byte-identical between the pre-Phase-09 base and current master.
test: Diff e0159c2 (this worktree's base, pre-Phase-05/07/08/09) against master (ac42a7c) for state/mod.rs handle_action, reevaluate_all_macros, and platform/macos/observer.rs start_observing — confirms zero changes to targeting logic or observer registration across all of Phase 05/07/08/09.
expecting: If Phase 9 regressed the gate, the diff would show changes to Gate 3 / reevaluate_all_macros / observer registration. It does not.
next_action: DONE for find_root_cause_only — reported to caller. Recommend specialist add eprintln instrumentation inside the NSWorkspace notification handler (observer.rs handler block) and on registration failure (the `if let Some(obs) = observer` branch's implicit else) to empirically confirm whether the notification fires on real app switches.

## reasoning_checkpoint (structured reasoning per debugger philosophy)

hypothesis: "AppState.active_app becomes stale after the initial launch-time snapshot because the NSWorkspaceDidActivateApplicationNotification observer registered in platform/macos/observer.rs:551-568 (also present at master:551-568, offset +9 due to unrelated Phase 08 additions) either fails to register or fails to reliably invoke its Rust closure on subsequent app-switch events, and this failure is completely silent (no logging on the registration-failure branch, no logging inside the notification handler). Since app-scoped macros gate on `active_app == target_app` (state/mod.rs Gate 3 + reevaluate_all_macros) while Global macros bypass this comparison entirely (`None => true`), a stale/never-updating active_app explains exactly the observed asymmetry: Global fires, targeted does not."
confirming_evidence:
  - "state/mod.rs handle_action() Gate 3 (line 231-237) and reevaluate_all_macros() (line 449-453) both correctly compare `self.state.active_app.as_deref() == Some(target.as_str())` — logic is provably correct in isolation."
  - "Frontend picker (src/App.tsx lines 651-672, 807-832) and backend list_running_apps_impl (observer.rs list_running_apps_impl) both use bundle-ID (`identifier`) consistently as target_app's stored value — no format mismatch between what's picked and what's compared."
  - "git diff e0159c2..master for src-tauri/src/state/mod.rs and src-tauri/src/platform/macos/observer.rs shows the entire targeting-check code path and the NSWorkspace observer registration are byte-identical (only line-number shifts from unrelated additions elsewhere in the files) — Phase 9 (and Phase 07/08) did not touch this code at all."
  - "observer.rs start_observing()'s registration branch (`if let Some(obs) = observer { ... }`) has no else branch and no logging — a failed `addObserverForName` call (returns nil observer) would silently do nothing, leaving active_app permanently pinned to whatever was frontmost at AutoMux's own launch."
  - "The notification handler closure itself (observer.rs lines 532-549) has zero instrumentation — contrast with initialize_tap()'s CGEventTap path, which explicitly logs failures (`eprintln!(\"Failed to create CGEventTap...\")`) and even the caller in start_observing logs when initialize_tap() returns false (WR-04 comment). No equivalent safety net exists for the NSWorkspace observer."
falsification_test: "Add eprintln! at the top of the notification handler closure (observer.rs:532) and one on registration failure. Rebuild in dev mode, switch focus between two apps several times, and check stdout. If the eprintln fires on every switch and active_app updates correctly in the emitted state-changed event, this hypothesis is refuted and the bug is elsewhere (e.g. a UI-only display issue, or user process/reproduction error)."
fix_rationale: "N/A — find_root_cause_only mode. Fix is deferred to the specialist/implementer once the falsification test confirms which branch (registration failure vs. non-firing callback vs. something else entirely) is the actual failure mode."
blind_spots: "Could not execute the actual macOS GUI app in this sandboxed environment to directly observe eprintln output on a real app-switch, so this remains a well-evidenced hypothesis rather than an empirically-confirmed root cause. Also did not rule out a narrower possibility: a one-time startup race between the async-spawned LoadProfile(\"default\") intent (lib.rs line 36-44) and the synchronous initial ActiveAppChanged dispatch inside start_observing() (observer.rs line 574-586) — this race is real and would incorrectly stop profile-loaded targeted macros at launch, but would self-correct on the next real app switch (assuming the notification observer works at all), so it does not fully explain a persistent failure across an explicit focus-switch during the user's live test session."

## Symptoms

expected: A macro whose target is set to a specific process (via `set_macro_target_app`) should fire its configured action(s) whenever that target process is the active/focused app and the macro is enabled — mirroring the global-mode behavior but gated by active-app matching instead of firing unconditionally.
actual: User tested during Phase 9 UAT (parallel macro execution): "Global macro worked, even if it slowed down computer... targetted macro did not seem to work." I.e. the targeted macro produced no visible firing/injection at all, unlike the global one.
errors: None reported by user; check app stdout/stderr for any silent match failures (e.g. active-app name/bundle-id mismatch causing the targeting gate to always evaluate false).
reproduction: In `npm run tauri dev` on macOS: create a macro, set its target app to a specific running application (not "Global"/all-apps) via the app's target-app picker, enable the macro, focus that target application, and observe whether the macro fires. Compare against a macro left on "Global" target, which does fire.
started: Discovered during UAT for Phase 9 (Parallel Macro Execution). Phase 9 touched scheduler/mod.rs, state/mod.rs, src/App.tsx (computeRunningState) — app-targeting itself is described as an OLDER feature pre-Phase-9 per 09-VERIFICATION.md. Need to determine if Phase 9 broke it or if it's pre-existing.

## Eliminated

- hypothesis: "Phase 9 (parallel macro execution) changes to scheduler/mod.rs or state/mod.rs broke the target-app gating logic."
  evidence: "`git diff e0159c2 master -- src-tauri/src/state/mod.rs` shows Gate 3 in handle_action() and the matches_target computation in reevaluate_all_macros() are byte-identical before and after Phase 07/08/09. Phase 9's actual scheduler/state changes were: raising action_tx capacity 100→1024 (09-01), a debug-only drop counter (09-01), the HoldStart/HoldRelease saturation-delivery fixes (09-05/09-06), and UX-11/UX-12 hotkey-conflict fields unrelated to target_app matching (Phase 08). None touch the target_app/active_app comparison."
  timestamp: 2026-07-22T00:20:00Z
- hypothesis: "Bundle-ID format mismatch between the frontend picker's stored target_app and the backend's active_app comparison (e.g. display name vs bundle ID, or Windows exe path vs macOS bundle ID confusion)."
  evidence: "src/App.tsx uses `app.identifier` (bundle ID from list_running_apps_impl / RunningApp.identifier) consistently as the <option value> for both the create-macro picker (line 653/667) and the card-edit picker (line 810/827). Backend list_running_apps_impl (observer.rs) and the NSWorkspace notification handler both populate the string via `bundleIdentifier()?.to_string()` — same format on both sides."
  timestamp: 2026-07-22T00:20:00Z
- hypothesis: "STATE_TX not registered before the platform observer starts, causing the initial ActiveAppChanged dispatch to silently fail (tx.try_send on an unset OnceLock)."
  evidence: "lib.rs calls `platform::macos::observer::set_state_tx(state_tx)` (line 67) strictly before `observer.start_observing()` (line 69) — ordering is correct, ruling out this specific startup race for the STATE_TX registration itself."
  timestamp: 2026-07-22T00:20:00Z

## Evidence

- timestamp: 2026-07-22T00:05:00Z
  checked: src-tauri/src/state/mod.rs handle_action() (Gate 3, lines 230-237) and reevaluate_all_macros() (lines 449-453)
  found: "Both correctly compute `matches_target` via `self.state.active_app.as_deref() == Some(target.as_str())` for Some(target), and `true` for None (Global). Logic is correct in isolation. Two-phase dispatch means the scheduler only fires ActionReady for macros already started via SchedulerIntent::StartMacro, which itself is only sent when reevaluate_all_macros() finds `mac.enabled && matches_target` — so Gate 3 is a defensive re-check, not the primary gate."
  implication: "The gating code itself is not the bug. Root cause must be in how active_app gets populated/updated, not in how it's compared."

- timestamp: 2026-07-22T00:10:00Z
  checked: src-tauri/src/platform/macos/observer.rs start_observing() (lines 521-591 in this worktree; master:526-596, same content shifted)
  found: "active_app is set in exactly two places: (1) a synchronous one-time snapshot via workspace.frontmostApplication() at the end of start_observing() (lines 569-581), and (2) inside the NSWorkspaceDidActivateApplicationNotification handler block registered via a raw, hand-rolled `msg_send!` call (lines 527-568). The registration branch `if let Some(obs) = observer { ... }` has no else/failure path — if addObserverForName returns nil, nothing is logged and active_app silently never updates again after snapshot (1)."
  implication: "If this notification handler is not being reliably invoked (registration failure, wrong block signature, or any other native FFI issue), active_app is permanently pinned to whatever was frontmost when AutoMux itself launched — which will essentially never equal a user-configured target_app in real usage, since the user configures the target AFTER launch and switches to it AFTER configuring. This perfectly explains 'targeted macro never fires' while 'global macro fires fine' (Global bypasses this comparison via `None => true`)."

- timestamp: 2026-07-22T00:15:00Z
  checked: "git diff e0159c2 master -- src-tauri/src/state/mod.rs src-tauri/src/platform/macos/observer.rs (this worktree's base vs. the current tip of the phase-09 branch, ac42a7c)"
  found: "handle_action Gate 3, reevaluate_all_macros, and the entire NSWorkspace observer registration block (start_observing) are byte-identical between the pre-Phase-05 base and current master (only line-number shifts from unrelated additions — UX-11/UX-12 hotkey-conflict fields in Phase 08, HoldStart/HoldRelease saturation fixes in Phase 09-05/09-06). This code has existed unchanged since the original 'advanced triggering and process parity' feature (faa1e1e, pre-v1.1)."
  implication: "This is a PRE-EXISTING bug, not a Phase 9 regression. It surfaced during Phase 9 UAT simply because that was the first UAT session to explicitly test app-targeting under real conditions."

- timestamp: 2026-07-22T00:18:00Z
  checked: "src/App.tsx card-edit target-app <select> (lines 807-832) vs. the create-macro <select> (lines 651-672)"
  found: "The card-edit <select> has no `value={macro.target_app}` binding (uncontrolled), unlike the create-macro select which has `value={newMacroTarget()}`. This means the dropdown doesn't visually reflect the macro's current target when entering edit mode (defaults to the browser's default first-option display)."
  implication: "Minor, separate UI bug — does not explain a macro that never fires (no onChange fires unless the user actively picks something), but worth flagging to the specialist as a related but distinct issue discovered during this investigation."

- timestamp: 2026-07-22T00:19:00Z
  checked: "lib.rs startup sequence: LoadProfile(\"default\") intent dispatch (async spawn, lines 35-44) vs. the synchronous initial ActiveAppChanged dispatch inside start_observing() (lines 67-69)"
  found: "LoadProfile is dispatched via `tauri::async_runtime::spawn` (no ordering guarantee relative to the synchronous start_observing() call happening later in the same setup closure). If the StateActor processes LoadProfile before the initial ActiveAppChanged, any profile-loaded macro with a target_app gets evaluated against `active_app: None` (AppState::default()), incorrectly stopping a targeted macro even if its target is already the actual frontmost app at launch."
  implication: "Real race, but scoped to STARTUP only for macros loaded from the default profile — does not by itself explain the user's reproduction (a macro created fresh during the live session, well after this startup window, then explicitly focused after enabling). Documented as a secondary finding, not the primary root cause."

## Resolution

root_cause: "PRE-EXISTING (not a Phase 9 regression — confirmed via diff against pre-Phase-05 base) reliability gap in macOS active-app tracking: `AppState.active_app` is populated once at launch via a synchronous NSWorkspace snapshot, then is only ever updated again via the NSWorkspaceDidActivateApplicationNotification observer registered with a raw hand-rolled `msg_send!` call in src-tauri/src/platform/macos/observer.rs (start_observing(), observer registration at lines ~527-568 / master lines 532-568). That registration path has zero error handling and the notification handler itself has zero logging, so a registration failure or non-firing callback is completely silent. Because app-scoped macros gate on `active_app == target_app` (src-tauri/src/state/mod.rs handle_action() Gate 3, lines 230-237, and reevaluate_all_macros(), lines 449-453) while Global-scope macros bypass this comparison entirely (`None => true`), a stale/non-updating active_app exactly explains why targeted macros never fire while Global macros fire normally. This root cause is a well-evidenced hypothesis (static-analysis-backed, file:line-cited) but was NOT empirically confirmed by running the macOS GUI app in this environment — recommend the specialist add eprintln instrumentation to the notification handler and the registration-failure branch, rebuild, and observe stdout across real app switches to confirm before implementing a fix (e.g. replacing the raw msg_send! registration with a more robust/typed observer mechanism, or reading the NSNotification's userInfo directly instead of re-querying frontmostApplication())."
fix:
verification:
files_changed: []
