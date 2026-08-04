---
phase: 09
slug: parallel-macro-execution
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-08-04
---

# Phase 09 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
>
> Register built retroactively from the `<threat_model>` blocks authored at plan time in all 11 phase-9 plans (09-01 through 09-11) — `register_authored_at_plan_time: true`. Classification cross-checked against `09-VERIFICATION.md`'s two independent verification passes (2026-07-22, re-verification), which directly re-read the implementation source for every `mitigate`-disposition threat below rather than trusting plan/summary prose alone. With `asvs_level: 1` and `threats_open: 0` at or above the `block_on: high` threshold, this audit was resolved without spawning `gsd-security-auditor` (per the workflow's L1 short-circuit rule) — the evidence already independently gathered by `09-VERIFICATION.md` meets L1 grep-depth.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| WebView (frontend) → Rust IPC | Frontend `invoke()` calls cross the Tauri IPC bridge (macro CRUD, hotkey bind/unbind, target-app/trigger-key edits, debug drop-count) | Macro config fields, trigger keycodes/modifiers, process identifiers — all user-supplied, no external network input |
| Scheduler task ↔ StateActor task | Internal bounded `mpsc` channels (`action_tx` capacity 1024, `sched_tx` capacity 100) carry `ActionReady`/`SchedulerIntent` between the two first-party Tokio tasks | Action-fire signals and scheduling commands; no external/untrusted input crosses here |
| OS global input stream → CGEventTap (macOS) / WH_KEYBOARD_LL hook (Windows) | System-wide input events observed for hotkey matching and emergency-stop detection | Raw keycodes/modifiers from the OS; narrowed this phase to only the 8 consumed event types |
| macOS NSWorkspace notification → active-app observer | OS-supplied active-application identity used for app-scoped macro targeting | Bundle identifier, read from the notification's own `userInfo` (trusted first-party OS source) |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-09-01 (09-01) | Information Disclosure | `get_debug_action_drop_count` IPC command | low | accept | `cfg(debug_assertions)`-gated; returns only a `u64` drop count, absent from release builds | closed |
| T-09-02 (09-01) | Denial of Service | `action_tx` overflow under parallel fire rate | medium | mitigate | Capacity raised 100→1024; drop counter surfaces overflow in debug builds | closed |
| T-09-03 (09-01) | Tampering | drop-counter instrumentation accuracy | low | accept | `AtomicU64` relaxed-ordering counter; a miscount affects only diagnostics, never injection correctness | closed |
| T-09-04 (09-02) | Tampering/Spoofing (display integrity) | `computeRunningState` derivation drift vs. backend gates | low | mitigate | Read-only display mirrors `state/mod.rs` gates exactly; no injection authority | closed |
| T-09-05 (09-03) | Information Disclosure | `09-VERIFICATION.md` contents | low | accept | Internal planning artifact; no secrets/credentials | closed |
| T-09-05 (09-04) | Tampering/Spoofing (display integrity) | `computeRunningState` Hold-branch drift vs. scheduler | low | mitigate | Mirrors `scheduler/mod.rs` Hold conversion exactly; read-only display | closed |
| T-09-05a (09-05) | Tampering (input integrity) | `release_holds` HoldRelease delivery | **high** | mitigate | `try_send`→guaranteed `.await` delivery, mirroring `StopAll`; proven by capacity-1 saturation regression test. Independently re-confirmed via source read in `09-VERIFICATION.md` truth #4. | **closed** |
| T-09-05b (09-05) | Denial of Service (mutual-block) | scheduler↔StateActor bidirectional `.await` | medium | accept | Documented residual risk of the 09-05a mitigation; same accepted pattern as the pre-existing `StopAll` path; StateActor's biased select loop drains independently, 1024-capacity headroom | closed (accepted) |
| T-09-05c (09-05) | Denial of Service (starvation) | scheduler paused during awaited release under saturation | low | accept | Bounded, resolves as soon as one channel slot drains; intended trade-off | closed (accepted) |
| T-09-01 (09-06) | Denial of Service (availability) | `Scheduler::start_macro` SustainedHold `HoldStart` delivery | **high** | mitigate | `try_send`→guaranteed `.await`; proven by `start_macro_hold_start_delivered_under_saturation` regression test. Independently re-confirmed in `09-VERIFICATION.md` truth #4. | **closed** |
| T-09-02 (09-06) | Tampering (state integrity) | `active_holds` / held-indicator UI derivation | medium | mitigate | Consequence of the 09-06/T-09-01 fix — `active_holds` now only records a hold on genuine delivery | closed |
| T-09-07-01 (09-07) | Information Disclosure | CGEventTap subscription mask | low | mitigate | Mask narrowed to 8 consumed event types, dropping motion/drag/scroll — net reduction in observed data (G-09-1a fix) | closed |
| T-09-07-02 (09-07) | Spoofing | active-app identity via NSWorkspace notification | low | accept | Identity sourced from the OS's own trusted notification `userInfo`; spoofing would require code-execution privilege that already moots targeting | closed (accepted) |
| T-09-07-03 (09-07) | Denial of Service | permanent CGEventTap responsiveness tax | medium | mitigate | Mask narrowing removes the always-on system-wide relay tax (G-09-1a fix, user-confirmed via idle-input-lag-freeze follow-up) | closed |
| T-09-08-01 (09-08) | Denial of Service (accidental data loss) | macro card delete button | low | mitigate | `window.confirm` guards the destructive action; per-id removal cannot affect other macros (G-09-1b fix) | closed |
| T-09-08-02 (09-08) | Tampering | `remove_macro` IPC arg (`id`) | low | accept | Id originates from AppState already rendered client-side; backend no-ops safely on an absent id | closed |
| T-09-01 (09-09) | Tampering (data integrity) | IPC arg deserialization casing (`set_macro_target_app` et al.) | medium | mitigate | `rename_all = "snake_case"` added; confirmed closed per STATE.md decision log (Phase 9 Plan 9) and `09-VERIFICATION.md` gaps #10/#11 | closed |
| T-09-02 (09-09) | Repudiation/transparency | `App.tsx` `handleCardSetTriggerKey` catch-block mislabeling | low | mitigate | Non-conflict IPC failures no longer mislabeled as hotkey conflicts | closed |
| T-09-03 (09-09) | Spoofing/Info-disclosure/DoS/Elevation | phase scope (no new IPC/auth/network surface) | low | accept | No new external input surface introduced this round | closed (accepted) |
| T-09-01 (09-10) | Tampering | `HOTKEY_BINDINGS` static registry | low | accept | Process-local, written only from StateActor's owned state via the Intent channel; this change reduces surface by removing a redundant second registry | closed (accepted) |
| T-09-02 (09-10) | Denial of Service | keydown callback dispatch path | low | mitigate | Removed duplicate registry lookup and self-cancelling double-dispatch; independently re-confirmed in `09-VERIFICATION.md` truth #12 | closed |
| T-09.11-01 (09-11) | Denial of Service | `StateActor::handle_action` HoldRelease gating (stuck-input) | medium | mitigate | Unconditional `HoldRelease` bypass before Gate 1/2/3; independently re-confirmed via direct source read (`action_should_inject`, `state/mod.rs:281-309`) + passing `hold_release_bypasses_gates` regression test — `09-VERIFICATION.md` truth #13 | **closed** |
| T-09.11-02 (09-11) | Tampering (integrity) | `Intent::SetMacroTriggerKey` conflict path | medium | mitigate | Reject-on-conflict via `oneshot::Sender<Result<..>>`, no silent coercion; independently re-confirmed via source read + passing `set_trigger_key_rejects_conflict_without_coercion` test — `09-VERIFICATION.md` truth #14 | **closed** |
| T-09.11-03 (09-11) | Elevation (scope creep) | HoldRelease bypass blast radius | low | mitigate | Negative regression test proves the bypass is release-only, not a blanket Gate 1/2/3 skip | closed |
| T-09-SC (all plans) | Tampering (supply chain) | package installs | n/a | accept | No `Cargo.toml`/`package.json` changes in any of the 11 phase-9 plans — zero dependency surface added | closed (accepted) |

*Status: open · closed · open — below {block_on} threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` (`high`) count toward `threats_open`*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**Both `high`-severity threats (T-09-05a, T-09-01/09-06) are closed** — each was independently re-verified against implementation source (not merely trusted from plan/summary prose) in `09-VERIFICATION.md`'s re-verification pass, with a dedicated regression test pinning the fix. `threats_open: 0` at the `high` block threshold.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|--------------|------|
| AR-09-1 | T-09-05b | Bidirectional-await mutual-block risk accepted as a low-likelihood extension of an already-shipped, already-accepted pattern (`StopAll`'s guaranteed-delivery `.await`); the alternative (best-effort `try_send`) is a guaranteed, higher-severity Core Value violation | Plan 09-05 author, ratified in `09-VERIFICATION.md` re-verification | 2026-07-22 |
| AR-09-2 | T-09-07-02 | Active-app identity spoofing would require local code-execution privilege that already defeats the targeting gate independently | Plan 09-07 author | 2026-07-22 |
| AR-09-3 | T-09-08-02 / T-09-01 (09-10) | Backend no-ops safely on IPC args (missing macro id / registry) originating from client-rendered state; no privilege escalation path from the WebView | Plan 09-08 / 09-10 authors | 2026-07-22 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-07-22 | 24 | 24 | 0 | `09-VERIFICATION.md` re-verification pass (Claude, gsd-verifier) — independent source-level re-confirmation of every `mitigate`-disposition threat above |
| 2026-08-04 | 24 | 24 | 0 | Retroactive SECURITY.md compilation (Claude, gsd-secure-phase) — register assembled from all 11 plan-time `<threat_model>` blocks; classification adopted from the 2026-07-22 verification pass without re-auditing (asvs_level 1, L1 short-circuit) |

**Note — out-of-scope findings, resolved separately:** `09-VERIFICATION.md`'s re-verification pass also surfaced two Critical findings that predate Phase 9 and were explicitly assessed as out-of-scope for this phase's threat register (neither introduced/regressed by any 09-01..09-11 plan, neither describes a parallel-execution interaction): (1) `Intent::LoadProfile` wiping `state.macros` on a failed load, and (2) the Windows keyboard hook missing an `LLKHF_INJECTED` filter (a Tampering/Spoofing-relevant gap — a macro's own `SendInput` keystroke could self-trigger a hotkey or the emergency-stop combo). Both were subsequently fixed via quick tasks `260723-k9l` (commit `88ee2a7`) and `260723-krr` (commit `6ab0b66`), independent of this phase's sign-off.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-08-04
