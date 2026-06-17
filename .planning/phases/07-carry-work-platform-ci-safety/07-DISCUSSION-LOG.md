# Phase 7: carry-work-platform-ci-safety - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-17
**Phase:** 07-carry-work-platform-ci-safety
**Areas discussed:** Input Monitoring detection (COMPAT-04), TCC identity change (COMPAT-05), Auto-save error UI (ERR-01), CI resilience (CI-03 + CI-05)

---

## Input Monitoring detection (COMPAT-04)

### Q1: When should the app check for Input Monitoring?

| Option | Description | Selected |
|--------|-------------|----------|
| At startup, alongside Accessibility | Check both on launch using the same polling pattern from Phase 5 | ✓ |
| Reactively — only when a hotkey bind fails | Less noisy on launch, but discovery requires hotkey failure first | |
| On a separate manual check the user triggers | Lowest startup friction but worst discoverability | |

**User's choice:** At startup, alongside Accessibility

---

### Q2: How should the Input Monitoring status be surfaced?

| Option | Description | Selected |
|--------|-------------|----------|
| Second indicator alongside Accessibility | Add an 'Input Monitoring' row next to the existing Accessibility indicator | |
| A single combined 'Permissions' section listing both | Replace the solo Accessibility indicator with a grouped section | ✓ |
| Only show it when denied — hide it when granted | Minimal clutter; Input Monitoring appears only as an alert | |

**User's choice:** A single combined 'Permissions' section listing both

---

### Q3: When the user clicks 'Grant' for Input Monitoring, what should happen?

| Option | Description | Selected |
|--------|-------------|----------|
| Open System Settings → Privacy → Input Monitoring directly | Deep-link via opener plugin, same pattern as Accessibility | ✓ |
| Show a step-by-step modal with instructions, then open Settings | More guided but inconsistent with Accessibility flow | |
| You decide | Planner decides based on opener plugin capabilities | |

**User's choice:** Open System Settings → Privacy → Input Monitoring directly

---

### Q4: After granting, how should the app detect it?

| Option | Description | Selected |
|--------|-------------|----------|
| Same 3s polling as Accessibility — extend existing poll | Consistent with Phase 5 pattern; no new IPC commands | ✓ |
| A separate button the user clicks to re-check | Explicit but inconsistent with Accessibility auto-detection | |
| Require an app restart to re-check | Simplest backend but worst UX | |

**User's choice:** Same 3s polling as Accessibility — extend the existing poll

---

### Q5: macOS 26-only or all supported versions?

| Option | Description | Selected |
|--------|-------------|----------|
| macOS 26+ only | Input Monitoring requirement is a macOS 26 behavioral change | |
| All supported macOS versions | Consistent across versions | |
| You decide | Researcher determines if kTCCServiceListenEvent was required on older macOS | ✓ |

**User's choice:** You decide — delegated to researcher

---

### Q6: If Input Monitoring is denied, block engine or warn?

| Option | Description | Selected |
|--------|-------------|----------|
| Warn only — engine can start, hotkeys won't fire | Mouse macros still work; only global hotkey activation is affected | ✓ |
| Block the engine until Input Monitoring is granted | Overly restrictive — mouse macros work without Input Monitoring | |
| You decide | Planner determines which functionality requires Input Monitoring | |

**User's choice:** Warn only — engine can still start, hotkeys just won't fire

---

## TCC identity change (COMPAT-05)

### Q1: How should AutoMux detect a TCC identity change?

| Option | Description | Selected |
|--------|-------------|----------|
| Persist a 'was-ever-granted' flag, check on startup | If denied and flag exists → show identity-change message | ✓ |
| Compare stored app version to current version | Version bump as proxy for identity change | |
| Don't detect — improve 'not granted' message copy | Better error text, no detection logic | |

**User's choice:** Persist a 'was-ever-granted' flag, check on startup

---

### Q2: Where should the flag be stored?

| Option | Description | Selected |
|--------|-------------|----------|
| App's data directory via Tauri's path API (app_data_dir()) | Survives reinstalls and upgrades; ~/Library/Application Support/com.alvaro.automux/ | ✓ |
| Alongside the macro profiles JSON | Reuse existing persistence layer | |
| UserDefaults / NSUserDefaults via objc2 | Platform-native but macOS-only and more implementation work | |

**User's choice:** App's data directory via Tauri's path API

---

### Q3: What should the UI show when identity-change is detected?

| Option | Description | Selected |
|--------|-------------|----------|
| Replace generic 'not granted' text with a specific 'update detected' message | Same button, same flow, copy explains why it happened | ✓ |
| Show a modal before the main window loads | More prominent but adds startup interruption | |
| Show a banner in addition to the permission indicator | Double signal — hard to miss but noisy | |

**User's choice:** Replace the generic 'not granted' text with a specific 'update detected' message

---

### Q4: Does the same detection logic apply to Input Monitoring?

| Option | Description | Selected |
|--------|-------------|----------|
| Both permissions — unified flag covers Accessibility and Input Monitoring | TCC invalidates all grants simultaneously on identity change | ✓ |
| Accessibility only — Input Monitoring is macOS 26+ and less established | Simpler scope | |
| You decide | Planner determines which permissions are affected | |

**User's choice:** Both permissions — unified flag covers Accessibility and Input Monitoring

---

## Auto-save error UI (ERR-01)

### Q1: What style of notification for auto-save failure?

| Option | Description | Selected |
|--------|-------------|----------|
| Toast that auto-dismisses after ~5 seconds | Non-blocking; informs user briefly | |
| Persistent inline banner at the top of the macro list | Stays visible until dismissed; appropriate for data loss scenario | ✓ |
| Status indicator in the titlebar or footer | Subtle; easy to miss | |

**User's choice:** Persistent inline banner at the top of the macro list

---

### Q2: Should the banner show the raw error message?

| Option | Description | Selected |
|--------|-------------|----------|
| User-friendly copy only — e.g., 'Failed to save changes. Check available disk space.' | Raw Rust error is not user-readable | ✓ |
| Show both: friendly copy + collapsible technical detail | More debug-friendly but heavier UI | |
| You decide | Planner picks based on event payload | |

**User's choice:** User-friendly copy only

---

### Q3: What happens when the user dismisses the banner?

| Option | Description | Selected |
|--------|-------------|----------|
| Dismiss and clear — re-appears if another auto-save error fires | User always informed of latest failure | ✓ |
| Dismiss permanently until app restart | Quieter; second failure goes unreported | |
| No dismiss button — clears when next auto-save succeeds | Self-healing UI; may linger confusingly | |

**User's choice:** Dismiss and clear — re-appears if another auto-save error fires

---

## CI resilience (CI-03 + CI-05)

### Q1 (CI-03): When updater signature is missing, fail or skip?

| Option | Description | Selected |
|--------|-------------|----------|
| Fail CI with a clear error | Make absence visible | ✓ (initial) |
| Skip with a visible warning log | Warning but not failure | |
| Make the step conditional on a CI secret | Signed vs unsigned build separation | |

**Notes:** After clarification — AutoMux uses ad-hoc signing; no `.sig` file is ever generated. "Fail CI" on a never-generated artifact would break every build. Revised question followed.

---

### Q1b (CI-03 revised): Given ad-hoc signing generates no .sig file — what's the right fix?

| Option | Description | Selected |
|--------|-------------|----------|
| Remove the updater signature upload step from CI entirely | No attempt = no misleading skip; comment to re-add at v3 | ✓ |
| Make the step conditional: only run when TAURI_PRIVATE_KEY secret is present | Clean separation of signed/unsigned builds | |
| Keep the step but add ::warning:: annotation | More visible skip, no behavior change | |

**User's choice:** Remove the updater signature upload step from CI entirely

---

### Q2 (CI-05): How should the binary artifact be discovered?

| Option | Description | Selected |
|--------|-------------|----------|
| Use tauri-action's official output variable (${{ steps.tauri.outputs.artifactPaths }}) | Follows artifact wherever tauri-action places it | ✓ |
| Use find to search by extension (.dmg, .exe) | Broader net; could match wrong files | |
| Use a glob on the known tauri output directory | More specific than find; still fragile to path prefix changes | |

**User's choice:** Use tauri-action's official output variable

---

## Claude's Discretion

- Whether Input Monitoring check applies to macOS 26+ only or all supported versions (researcher determines)
- Exact copy for the "AutoMux was updated" identity-change message in the Permissions section
- Exact copy for the auto-save error banner
- Whether `check_accessibility_permissions` is extended or a separate `check_input_monitoring` function is added in `platform/macos/mod.rs`

## Deferred Ideas

None — discussion stayed within phase scope.
