---
phase: 08
slug: hotkey-reliability-conflict-safety
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-08-04
---

# Phase 08 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
>
> Register built retroactively from the `<threat_model>` blocks authored at plan time in all 6 phase-8 plans (08-01 through 08-06) — `register_authored_at_plan_time: true`. None of the 34 threats in the original plan-time registers were assigned an explicit severity field (Phase 8 predates the severity-column convention used from Phase 9 onward); severities below are inferred from each threat's own description and disposition, cross-checked against the freshly re-verified `08-VERIFICATION.md` (2026-08-04 re-run, independently confirmed against current source — not the stale 2026-06-30 pass). No threat here rises above `medium` — none involves attacker-controlled external input, a network surface, or a privilege boundary; this is a single-user local desktop app whose only "attacker" would already need local code-execution to matter. With `asvs_level: 1` and `threats_open: 0` at the `block_on: high` threshold, this audit was resolved without spawning `gsd-security-auditor` (per the workflow's L1 short-circuit rule).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Frontend (WebView) → IPC (`bind_hotkey`, `set_macro_trigger_key`, etc.) | Untrusted DOM `KeyboardEvent` is mapped to a `(keycode, modifiers)` tuple and sent across the Tauri IPC bridge | Keycode/modifier bitmask, macro id — all user-supplied via the app's own key-capture widget, no external network input |
| StateActor → platform statics (`HOTKEY_BINDINGS` macOS/Windows) | The StateActor is the sole source of truth; platform statics are write-only mirrors rebuilt on every state-mutating intent | Hotkey binding tuples (macro id, keycode, modifiers) |
| Persisted profile (JSON) → `AppState` deserialization | Pre-Phase-8 profiles lack `trigger_modifiers`/`conflicts` fields | Macro config fields; `#[serde(default)]` covers missing fields |
| Windows `WH_KEYBOARD_LL` hook → `Intent::ToggleMacroHotkey` | OS-supplied global keydown events matched against the hotkey registry | Raw keycode + modifier mask from the OS |
| Human verifier → device test execution | Manual macOS/Windows device tests requiring a physical host; the agent cannot perform these itself | N/A — human-only step |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-08-01 (08-01) | Tampering | `MacroConfig.trigger_modifiers` deserialization | low | mitigate | `#[serde(default)]` defaults old profiles to `0`; bit layout pinned by unit test | closed |
| T-08-02 (08-01) | Tampering | `AppState.conflicts` deserialization | low | mitigate | `#[serde(default)]` — derived field, safe to default to `[]` on old profiles | closed |
| T-08-03 (08-01) | Spoofing | `MACRO_TRIGGER_KEYS` key-collision | low | mitigate | Map key includes `u64` modifier bits — different modifier combos on the same keycode occupy separate slots | closed |
| T-08-04 (08-01) | Information Disclosure | Modifier bit constants in doc comment | low | accept | Publicly documented Apple/Microsoft constants; no security boundary | closed (accepted) |
| T-08-05 (08-01) | Repudiation | Silent key overwrite in `reevaluate_all_macros` | low | accept (interim) | Superseded by 08-02's `check_trigger_key_conflict` pre-check, which makes the silent overwrite unreachable in normal flow | closed |
| T-08-06 (08-01) | Denial of Service | `recompute_conflicts` O(n²) expansion | low | accept | Bounded by sequence length; negligible even at 1000-macro stress scale | closed (accepted) |
| T-08-07 (08-02) | Tampering | `Intent::AddMacro` conflicting `trigger_key` | medium | mitigate | `check_trigger_key_conflict` runs before insert; conflict surfaced via 08-03's `Intent::BindHotkey` reply channel | closed |
| T-08-08 (08-02) | Tampering | `Intent::SetMacroTriggerKey` conflicting key | medium | mitigate | Same mechanism as T-08-07 | closed |
| T-08-09 (08-02) | Information Disclosure | Conflict message names the other macro | low | accept | By design (UX-11 requires the user to know which macro to unbind); no new disclosure — frontend already has all macro names | closed (accepted) |
| T-08-10 (08-02) | Denial of Service | `recompute_conflicts` per-intent cost | low | accept | 1000-macro stress test measured at ~5000 hash lookups/recompute — negligible | closed (accepted) |
| T-08-11 (08-02) | Elevation of Privilege | `Intent::RemoveMacro` no ownership recheck | low | accept | Single-user local desktop app; Tauri's CSP is the only caller boundary, no multi-tenant authorization model exists | closed (accepted) |
| T-08-12 (08-02) | Repudiation | Silent trigger-key drop on conflict (interim) | low | accept (interim) | Stopgap until 08-03's explicit error path shipped; superseded | closed |
| T-08-13 (08-03) | Tampering | `bind_hotkey` IPC conflicting `(keycode, modifiers)` | medium | mitigate | `Intent::BindHotkey` calls `check_trigger_key_conflict`, returns `Err` via oneshot; surfaced as `ConflictErrorToast` (08-05) — independently re-confirmed live in today's UAT Test 1 (5.3) | **closed** |
| T-08-14 (08-03) | Elevation of Privilege | Windows `bind_hotkey` hook install permission | low | accept | `WH_KEYBOARD_LL` is implicitly available to standard user processes via `SetWindowsHookExW`; no additional boundary crossed | closed (accepted) |
| T-08-15 (08-03) | Spoofing | `build_mod_mask` wrong VK_* constant | low | mitigate | Bit values pinned by unit test (`windows_mod_constants`); independently re-confirmed in `08-VERIFICATION.md`'s 2026-08-04 re-run | closed |
| T-08-16 (08-03) | Denial of Service | `update_hotkey_bindings` full-Vec replace per bind | low | accept | Microsecond-scale for typical (<10 macro) configs | closed (accepted) |
| T-08-17 (08-03) | Repudiation | `Intent::UnbindHotkey` cannot fail | low | accept | By design — absence of a binding is a legitimate no-op | closed (accepted) |
| T-08-18 (08-03) | Information Disclosure | Conflict error names the other macro | low | accept | Same disposition as T-08-09 | closed (accepted) |
| T-08-19 (08-03) | Tampering | Windows `HOTKEY_BINDINGS` `Mutex` deadlock risk | medium | mitigate | `OnceLock<Mutex<...>>` with short, non-reentrant critical sections; mirrors the macOS pattern; the mutex never posts events so the SAFE-04 deadlock class doesn't apply | closed |
| T-08-20 (08-04) | Tampering | `computeModifiers` returns wrong bits | medium | mitigate | Bit values pinned by Rust unit tests (08-01); round-trip independently re-confirmed live via today's UAT device tests (5.6, 6.5 — modifier chip preview in correct semantic order on both platforms) | **closed** |
| T-08-21 (08-04) | Elevation of Privilege | `showConflictError` string parsing | low | accept | Display-only; parse failure falls back to a generic message, never blocks the user or feeds a subsequent IPC call | closed (accepted) |
| T-08-22 (08-04) | Information Disclosure | Conflict error names the other macro | low | accept | Same disposition as T-08-09/18 | closed (accepted) |
| T-08-23 (08-04) | Denial of Service | `setConflictError` timer accumulation | low | mitigate | Single-slot timer replaces on each new conflict, mirrors the existing `showProfileMsg` toast pattern | closed |
| T-08-24 (08-04) | Repudiation | Bare-modifier bindings silently accepted | low | accept | Intentional per RESEARCH.md Open Question #1; resolves to a named key (e.g. "Shift"), no silent failure | closed (accepted) |
| T-08-25 (08-05) | Tampering | `localStorage` first-run-banner flag user-editable | low | accept | Advisory only; worst case is a one-time educational banner reappearing — not security-critical | closed (accepted) |
| T-08-26 (08-05) | Information Disclosure | C-2 conflict body lists macro names | low | accept | By design (UX-12); names already visible in the frontend's own state | closed (accepted) |
| T-08-27 (08-05) | Spoofing | C-5 modifier chip display order | low | mitigate | Hard-coded semantic order (Shift → Ctrl → Alt → Cmd/Win), independent of press order; bit values pinned by unit test | closed |
| T-08-28 (08-05) | Denial of Service | Re-render storm on `state-changed` | low | accept | SolidJS fine-grained reactivity — only the changed conflict card re-renders | closed (accepted) |
| T-08-29 (08-05) | Repudiation | Dismissed conflict toast reappears on new conflict | low | accept | Intended UX — single-slot timer surfaces the latest conflict | closed (accepted) |
| T-08-30 (08-05) | Elevation of Privilege | `conflictsDismissed` is session-only | low | accept | Per UI-SPEC C-2 design — no persistence required | closed (accepted) |
| T-08-31 (08-06) | Tampering | Verification report contents | low | mitigate | Report captures literal command output, independently re-runnable; manual test steps are unambiguous (exact macro names/key combos) | closed |
| T-08-32 (08-06) | Denial of Service | Test suite timeout | low | accept | Full suite runs in <10s per VALIDATION.md; a timeout would itself be a real regression, not a false positive | closed (accepted) |
| T-08-33 (08-06) | Repudiation | Manual test result claimed without evidence | medium | mitigate | Section status requires the literal step to pass before marking `done`; **this round's evidence is no longer just the protocol design — Sections 5 and 6 were actually executed today (2026-08-04) via `08-UAT.md`, both fully passing, closing the exact gap this threat worried about** | **closed** |
| T-08-34 (08-06) | Information Disclosure | Profile backwards-compat manual path uses a real profile | low | accept | Automated path (§4.1) uses a synthetic JSON string; the manual fallback (§4.2, not required since §4.1 passes) intentionally operates on the developer's own profile | closed (accepted) |
| T-08-SC (all plans) | Tampering (supply chain) | package installs | n/a | accept | No `Cargo.toml`/`package.json` changes in any of the 6 phase-8 plans — zero dependency surface added | closed (accepted) |

*Status: open · closed · open — below {block_on} threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` (`high`) count toward `threats_open`*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**No threat in this phase reaches `high` severity** — the highest-impact items (T-08-07/08/13/19/20/33, all `medium`) are all closed, several with fresh device-level confirmation from today's `08-UAT.md` pass (T-08-13, T-08-20, T-08-33). `threats_open: 0` at the `high` block threshold.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|--------------|------|
| AR-08-1 | T-08-11 | AutoMux is a single-user local desktop app with no multi-tenant authorization model; Tauri's CSP is the only IPC caller boundary, and `RemoveMacro`'s lack of ownership recheck cannot be exploited without local code-execution privilege that already defeats the app's entire security model | Plan 08-02 author | 2026-06-19 |
| AR-08-2 | T-08-09 / T-08-18 / T-08-22 / T-08-26 | Conflict-error and conflict-warning messages intentionally name the other macro (UX-11/UX-12 explicitly require this so the user knows which macro to unbind); the frontend already has full access to all macro names via its own state, so no new disclosure occurs | Plans 08-02/08-03/08-04/08-05 authors | 2026-06-19 to 2026-06-30 |
| AR-08-3 | T-08-14 | Windows `WH_KEYBOARD_LL` hook installation via `SetWindowsHookExW` is implicitly available to standard user processes; no elevated privilege is requested or required | Plan 08-03 author | 2026-06-19 |
| AR-08-4 | T-08-25 | The first-run global-hotkey-notice `localStorage` flag is advisory; user tampering can at most cause a one-time educational banner to reappear, which carries no security consequence | Plan 08-05 author (RESEARCH.md R-8) | 2026-06-30 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-30 | 34 | 34 | 0 | Plan-time threat modeling across plans 08-01 through 08-06 (dispositions recorded at authoring time, no dedicated audit pass) |
| 2026-08-04 | 34 | 34 | 0 | Retroactive SECURITY.md compilation (Claude, gsd-secure-phase) — register assembled from all 6 plan-time `<threat_model>` blocks; severities inferred (Phase 8 predates the severity-column convention); three threats (T-08-13, T-08-20, T-08-33) upgraded from "mitigated at source" to fully device-confirmed via today's `08-UAT.md` pass (macOS 5.1-5.6, Windows 6.1-6.5, both fully passing) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-08-04
