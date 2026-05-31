# Phase 2: Cleanup & Credibility - Research

**Researched:** 2026-05-19
**Domain:** Repository hygiene, Rust resource lifetime, Tauri managed state, SolidJS async pattern
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Change "MIT License" to "GNU GPL v3" (or "GNU General Public License v3") everywhere it appears in README prose (README-01).
- **D-02:** Add a `GPL-3.0-only` shields.io badge to the README. Use the shields.io SPDX identifier `GPL-3.0-only`. Remove or replace the existing broken license badge (README-02).
- **D-03:** README-03 scope is a **polish + appeal** pass — not a full rewrite. No new top-level sections added. Four areas in scope: opening paragraph, feature list accuracy, platform info, badges audit.
- **D-04:** No screenshot or demo GIF in Phase 2.
- **D-05:** Store `MacPlatformObserver` in Tauri managed state using `app.manage()` in `src-tauri/src/lib.rs`. Observer must live for the full application lifetime.
- **D-06:** No new IPC command for SAFE-02. Goal is correct resource lifetime only.
- **D-07:** Remove `implementation_plan.md` from repository root (AUDIT-01). Planner's choice: move to `archive/` or delete outright.
- **D-08:** Dead code audit scope is **source files only** — `src-tauri/src/` and `src/`. Do NOT audit Cargo dependencies.
- **D-09:** `.gitignore` additions: add coverage for `src-tauri/gen/`, `*.log` / debug temp files, and OS artifacts. Inspect current `.gitignore` first.
- **D-10:** Replace hardcoded `v1.0.0` in `src/App.tsx:291` with runtime `getVersion()` from `@tauri-apps/api/app`.

### Claude's Discretion

- Exact README section order and wording for appeal/clarity improvements.
- Whether to move `implementation_plan.md` to `archive/` or delete it.
- Exact `.gitignore` entry format, ordering, and whether to add grouping comments.
- What constitutes "dead" in the TypeScript source — run compiler to enumerate.

### Deferred Ideas (OUT OF SCOPE)

- Cargo dep audit (`cocoa` 0.26, `objc` 0.2 removal) — defer to later phase.
- Windows hook initialization error surfacing.
- Duplicate trigger key validation.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| README-01 | README prose corrected from "MIT License" to "GNU GPL v3" | Found: line 79, README.md. `package.json` also declares `"license": "MIT"` — both need updating. No `LICENSE` file exists — needs to be created. |
| README-02 | Correct GPL-3.0 badge added to README (shields.io SPDX identifier `GPL-3.0-only`) | Found: current badge is GitHub license auto-badge which will reflect whatever the repo declares. Needs explicit shields.io SPDX badge. |
| README-03 | README copy improved for appeal and clarity | Found: feature list references macOS+Windows process detection already implemented. No Phase 3 features mentioned. Opening para is over-engineered jargon-heavy. |
| SAFE-02 | MacPlatformObserver resource leak fixed; `stop_observing` no longer dead code | Found: `_observer_token: Option<usize>` in observer.rs:443. Struct is `Send+Sync` (usize field). `app.manage()` requires `T: Send + Sync + 'static`. No wrapping needed. |
| AUDIT-01 | `implementation_plan.md` removed from repo root | Found: file confirmed present at repo root. |
| AUDIT-02 | Dead/unused source files identified and removed | Found: `cargo check` and `tsc --noEmit` both pass clean with zero warnings. Rust compiler (`RUSTFLAGS="-Wdead-code"`) emits no warnings. No dead source files found — see findings. |
| AUDIT-03 | Debug artifacts and temp files removed and added to `.gitignore` | Found: `src-tauri/gen/schemas` already covered by `src-tauri/.gitignore`. Root `.gitignore` is missing `Thumbs.db` and `*.log`. `.DS_Store` is already present. |
</phase_requirements>

---

## Summary

Phase 2 is a precision cleanup phase with seven small, independent tasks. All changes are surgically scoped with no new features. The phase breaks into four distinct sub-domains:

**README/legal (README-01, README-02, README-03):** The README currently declares "MIT License" in its prose (line 79). No `LICENSE` file exists in the repository. `package.json` also declares `"license": "MIT"`. The requirements mandate GPL-3.0-only. All three locations need updating — the prose, the `package.json` metadata, and a LICENSE file needs to be created. The existing GitHub license badge auto-reflects repo metadata and will update automatically once the license is corrected. An explicit shields.io SPDX badge for `GPL-3.0-only` must also be added.

**MacPlatformObserver lifetime fix (SAFE-02):** The `MacPlatformObserver` is currently created and immediately dropped at the end of the `setup` closure in `lib.rs:66-71`. The struct contains `Option<usize>` (raw pointer stored as integer), which means it is automatically `Send + Sync + 'static` — no `Mutex` wrapping is required. The fix is a single `app.manage(observer)` call after `observer.start_observing()`, behind the existing `#[cfg(target_os = "macos")]` gate.

**Dead code and artifact removal (AUDIT-01, AUDIT-02, AUDIT-03):** `implementation_plan.md` is confirmed present in the repo root and must be removed or archived. Both `cargo check` (with `RUSTFLAGS="-Wdead-code"`) and `tsc --noEmit` pass completely clean — zero unused code warnings. AUDIT-02 is effectively "verify clean, document result." For AUDIT-03, `src-tauri/gen/schemas` is already covered by `src-tauri/.gitignore`. The root `.gitignore` is missing `Thumbs.db` and `*.log` entries — `.DS_Store` is already present.

**Version string (D-10):** `src/App.tsx:300` hardcodes `v1.0.0` while `tauri.conf.json` and `package.json` both declare `1.1.0`. The `getVersion()` API from `@tauri-apps/api/app` returns a `Promise<string>` and the correct import path is `import { getVersion } from '@tauri-apps/api/app'`. The existing `createEffect` + async IIFE pattern in `App.tsx:95-119` is the established model to follow.

**Primary recommendation:** All tasks are independent and can be executed in any order. Start with the dead code audit verification (AUDIT-02) to confirm cleanliness, then the SAFE-02 lifetime fix (highest safety value), then README/legal corrections.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Observer lifetime management | API / Backend (Rust) | — | `app.manage()` is a Tauri Rust API; observer lifecycle is backend-owned |
| Version display | Frontend Server (SolidJS) | API / Backend | `getVersion()` is a frontend JS API call; the backend just exposes the version from `tauri.conf.json` |
| README/legal text | Repository metadata | — | Static file edits; no runtime tier |
| `.gitignore` coverage | Repository metadata | — | Static file edits; no runtime tier |
| Dead code removal | Both (Rust backend + TS frontend) | — | Compiler-verified source changes |

---

## Standard Stack

### Core (all tools already in project)
| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| `@tauri-apps/api/app` | ^2 (project) / 2.11.0 (registry) | `getVersion()` frontend call | `[VERIFIED: npm registry]` — already in project deps, no install needed |
| `tauri::Manager` | 2.x | `app.manage()` Rust API | `[VERIFIED: Context7 /websites/rs_tauri_2_9_5]` — already imported in `lib.rs:11` |
| `cargo check` / `clippy` | stable | Dead code audit | `[VERIFIED: codebase grep]` — both run clean |
| `tsc --noEmit` | ~5.6.2 | TypeScript dead code audit | `[VERIFIED: codebase run]` — exits 0, no warnings |

### No New Dependencies
This phase requires zero new dependencies. All needed APIs are already in the project.

---

## Architecture Patterns

### System Architecture Diagram

```
README.md (static)
  ├── MIT → GPL-3.0-only prose fix (line 79)
  ├── License badge → shields.io SPDX badge
  └── Appeal pass (features, platform, badges)

src/App.tsx
  └── v1.0.0 hardcoded → createEffect + getVersion() async call

src-tauri/src/lib.rs (setup closure)
  ├── [macos] observer.start_observing()
  └── [macos] app.manage(observer)  ← NEW: keeps observer alive

.gitignore (root)
  └── Add: Thumbs.db, *.log

repo root
  └── implementation_plan.md → remove/archive
```

### Pattern 1: Tauri Managed State (SAFE-02)

**What:** Store any `T: Send + Sync + 'static` value in Tauri's state registry for application-lifetime access.

**When to use:** When a resource must live beyond the closure where it was created and potentially be accessed from commands later.

**Pattern:**
```rust
// Source: https://docs.rs/tauri/2.9.5/tauri/struct.Builder.html [VERIFIED: Context7]
app.manage(my_value);  // T must be Send + Sync + 'static

// In a command (if needed later):
#[tauri::command]
fn my_cmd(state: tauri::State<MyType>) { ... }
```

**MacPlatformObserver-specific application:**
```rust
// Source: src-tauri/src/lib.rs [VERIFIED: codebase]
#[cfg(target_os = "macos")]
{
    platform::macos::observer::set_state_tx(state_tx);
    let mut observer = MacPlatformObserver::new();
    observer.start_observing();
    app.manage(observer);  // Keeps alive for app lifetime; SAFE-02 fix
}
```

**Why `MacPlatformObserver` is compatible:** The struct contains only `Option<usize>`. `usize` is `Send + Sync` in Rust (it is `Copy`). No `Mutex` wrapping needed. `[VERIFIED: codebase inspection of observer.rs:440-444]`

### Pattern 2: SolidJS Async Init for Version String (D-10)

**What:** Read an async value at component mount and store in a signal.

**Established pattern in codebase (`App.tsx:95-119`):**
```typescript
// Source: src/App.tsx [VERIFIED: codebase]
createEffect(() => {
  let cancelled = false;
  onCleanup(() => { cancelled = true; });
  (async () => {
    try {
      const version = await getVersion();
      if (cancelled) return;
      setAppVersion(version);
    } catch (e) {
      if (cancelled) return;
      console.error("Failed to get version:", e);
    }
  })();
});
```

**Alternative:** Add `getVersion()` to the existing `Promise.all` in the initial data fetch effect (`App.tsx:101`). This is cleaner — one fewer effect.

**Import:**
```typescript
import { getVersion } from '@tauri-apps/api/app';
// Source: https://v2.tauri.app/reference/javascript/api/namespaceapp/ [CITED]
```

### Pattern 3: shields.io SPDX Badge

**What:** Display a license badge using the shields.io static SPDX badge endpoint.

**URL format:**
```markdown
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
```

Or using shields.io SPDX identifier (as mandated by D-02):
```markdown
[![License: GPL-3.0-only](https://img.shields.io/badge/License-GPL--3.0--only-blue)](https://spdx.org/licenses/GPL-3.0-only.html)
```

**Current broken badge (README.md:8):** Uses GitHub's automatic license badge which reads from the repo's license metadata. Until the repository declares GPL-3.0, this badge will show "MIT" or "None".

**`[ASSUMED]`:** The exact shields.io URL format for `GPL-3.0-only` SPDX — the SPDX identifier is correct, but the exact color/style is Claude's discretion.

### Anti-Patterns to Avoid

- **Do not** call `app.manage()` with a type that is already managed — Tauri will return `false` (not panic, but the second call is silently ignored). Each type can only be managed once. `[VERIFIED: Context7]`
- **Do not** access `MacPlatformObserver` from a `#[tauri::command]` in this phase (D-06: no new IPC command). The managed state is for lifetime only.
- **Do not** add `getVersion()` as a separate Rust IPC command — the JS API already exposes it natively through the Tauri core app plugin.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Reading app version | Custom IPC command returning version string | `getVersion()` from `@tauri-apps/api/app` | Built into Tauri core; always in sync with `tauri.conf.json` |
| Resource lifetime management | `Arc<Mutex<>>` stored in a global static | `app.manage()` | Already the established pattern in this codebase; thread-safe by design |
| License badge | Custom SVG badge or hardcoded image | shields.io SPDX endpoint | Maintained service, standard format |

---

## Critical Finding: Missing LICENSE File

**This was NOT noted in CONTEXT.md but is blocking for README-01/02.**

A `LICENSE` file does not exist in the repository root. `[VERIFIED: filesystem inspection]`

The current state:
- `README.md:79` — declares "MIT License"
- `package.json` — declares `"license": "MIT"`
- `src-tauri/Cargo.toml` — no `license` field
- No `LICENSE` file anywhere in the repository

The REQUIREMENTS mandate GPL-3.0-only. The plan must include:
1. Create `LICENSE` file with GPL-3.0-only text
2. Update `package.json` `"license"` field to `"GPL-3.0-only"`
3. Update `README.md` prose from "MIT License" to "GNU General Public License v3"
4. Update/replace README badge

The GitHub auto-license badge (`https://img.shields.io/github/license/ABO896/AutoMux?style=flat-square`) will only reflect the correct license after the `LICENSE` file is committed and GitHub detects it. Until then, the explicit shields.io SPDX badge (D-02) is the reliable display.

---

## Dead Code Audit Results (AUDIT-02)

### Rust Layer

`cargo check` (dev profile) and `RUSTFLAGS="-Wdead-code" cargo check` both complete with **zero warnings** and **zero errors**. `[VERIFIED: codebase run]`

`cargo clippy` also completes with **zero warnings**. `[VERIFIED: codebase run]`

**Conclusion:** No unused Rust source files or dead functions surfaced by the compiler. AUDIT-02 for the Rust layer is a documentation task: record that the audit was performed and no dead code was found.

### TypeScript / SolidJS Layer

`npx tsc --noEmit` exits with **code 0, zero output**. `[VERIFIED: codebase run]`

`noUnusedLocals: true` and `noUnusedParameters: true` are both enabled in `tsconfig.json`. The compiler would surface any unused locals or parameters.

**Conclusion:** No dead TypeScript code detected. AUDIT-02 for the TS layer is a documentation task.

**One known structural dead-code pattern (from CONCERNS.md):** The `{ Key: parseInt(inputVal) || 0 }` path in `handleCreateMacro` is effectively unreachable from the UI (the dropdown never produces a non-mouse value), but the code is syntactically reachable — `tsc` does not flag it as dead. This is intentional scaffolding for Phase 3 (UX-01). Leave it; removing it would break Phase 3. `[VERIFIED: CONCERNS.md + codebase]`

---

## .gitignore Gap Analysis (AUDIT-03)

### Root `.gitignore` current coverage:
```
node_modules/     ✓
dist/             ✓
src-tauri/target/ ✓
.DS_Store         ✓
.env              ✓
.env.local        ✓
.agents/ etc.     ✓ (agent metadata)
src-tauri/test.rs ✓ (scratch files)
```

### `src-tauri/.gitignore` current coverage:
```
/target/          ✓
/gen/schemas      ✓  (already covered — CONTEXT.md said to add this, but it's already there)
```

### Gaps requiring additions to root `.gitignore`:
| Pattern | Missing | Reason |
|---------|---------|--------|
| `Thumbs.db` | YES | Windows OS artifact; not covered |
| `*.log` | YES | Debug/temp log files; not covered |
| `src-tauri/gen/` | NO | Already covered by `src-tauri/.gitignore` |
| `.DS_Store` | NO | Already present |

**Plan implication:** AUDIT-03 only needs two additions to the root `.gitignore`: `Thumbs.db` and `*.log`. The `src-tauri/gen/` coverage D-09 mentions is already in place via `src-tauri/.gitignore`.

---

## Common Pitfalls

### Pitfall 1: Double-managing a type in Tauri

**What goes wrong:** Calling `app.manage(observer)` when `observer` has already been managed returns `false` silently.

**Why it happens:** Only one instance of each type can be managed. On macOS the `MacPlatformObserver` is only managed once (behind `#[cfg]`), so this is not a real risk here.

**How to avoid:** Only call `app.manage()` once per type per application lifetime. `[VERIFIED: Context7]`

---

### Pitfall 2: `MacPlatformObserver` borrowed-after-move

**What goes wrong:** `observer.start_observing()` takes `&mut self`. After `app.manage(observer)`, the binding is moved. Calling any method on `observer` after `app.manage()` is a compile error.

**How to avoid:** Call `start_observing()` before `app.manage()`. The correct order:
```rust
let mut observer = MacPlatformObserver::new();
observer.start_observing();  // must be BEFORE manage()
app.manage(observer);        // consumes observer; can't use after this
```

---

### Pitfall 3: README license badge shows wrong value after merge

**What goes wrong:** The GitHub auto-badge (`img.shields.io/github/license/...`) reads from GitHub's license detection. It takes 30–60 minutes after a LICENSE file commit for GitHub to update its detection cache.

**How to avoid:** The explicit shields.io SPDX badge (`img.shields.io/badge/License-GPL--3.0--only-blue`) does not depend on GitHub's detection — it is hardcoded. Use it alongside or instead of the auto-badge. `[ASSUMED: GitHub cache timing — based on training knowledge]`

---

### Pitfall 4: `package.json` license field left as "MIT"

**What goes wrong:** The `package.json` `"license"` field is still `"MIT"`. npm audits and tools that read `package.json` will continue to report the package as MIT-licensed even after the README and LICENSE file are updated.

**How to avoid:** Update `"license": "MIT"` to `"license": "GPL-3.0-only"` in `package.json` as part of README-01 task. `[VERIFIED: codebase inspection]`

---

### Pitfall 5: `getVersion()` called outside async context

**What goes wrong:** Forgetting to `await` `getVersion()` results in a Promise object being stored in the signal, rendering as `[object Promise]` in the UI.

**How to avoid:** Always await inside an `async` function and only set signal value after the promise resolves. Follow the established `createEffect` + async IIFE + `cancelled` flag pattern in the codebase.

---

## Code Examples

### SAFE-02: MacPlatformObserver Lifetime Fix

```rust
// Source: src-tauri/src/lib.rs [VERIFIED: codebase] — current broken pattern
#[cfg(target_os = "macos")]
{
    platform::macos::observer::set_state_tx(state_tx);
    let mut observer = MacPlatformObserver::new();
    observer.start_observing();
    // BROKEN: observer drops here — stop_observing is dead code
}

// Fixed pattern (add app.manage):
#[cfg(target_os = "macos")]
{
    platform::macos::observer::set_state_tx(state_tx);
    let mut observer = MacPlatformObserver::new();
    observer.start_observing();
    app.manage(observer);  // observer now lives for app lifetime
}
```

### D-10: Version String Fix

```typescript
// Source: src/App.tsx pattern [VERIFIED: codebase] — add to existing initial fetch
import { getVersion } from '@tauri-apps/api/app';
// Source: https://v2.tauri.app/reference/javascript/api/namespaceapp/ [CITED]

// Add signal:
const [appVersion, setAppVersion] = createSignal<string>("…");

// Add to existing Promise.all in the initial createEffect (App.tsx:101):
const [stateData, accessOk, app, profileList, version] = await Promise.all([
  invoke<AppState>("get_state"),
  invoke<boolean>("check_accessibility"),
  invoke<string | null>("get_active_app"),
  invoke<ProfileSummary[]>("list_profiles"),
  getVersion(),  // ← add this
]);
setAppVersion(version);

// In JSX, replace:
<span class="text-[10px] text-text-dim font-mono">v1.0.0</span>
// With:
<span class="text-[10px] text-text-dim font-mono">v{appVersion()}</span>
```

### AUDIT-03: .gitignore Additions

```gitignore
# Windows OS artifacts
Thumbs.db

# Debug / temp log files
*.log
```

### README-02: Shields.io GPL Badge

```markdown
[![License: GPL-3.0-only](https://img.shields.io/badge/License-GPL--3.0--only-blue)](https://spdx.org/licenses/GPL-3.0-only.html)
```

---

## Runtime State Inventory

> Not applicable — this is not a rename/refactor/migration phase. All changes are source file edits and repository metadata updates. No runtime state stores need migration.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo check` / `clippy` | AUDIT-02 Rust audit | ✓ | stable | — |
| `npx tsc` | AUDIT-02 TS audit | ✓ | ~5.6.2 | — |
| `@tauri-apps/api` | D-10 getVersion | ✓ | ^2 in deps | — |
| `tauri::Manager` | SAFE-02 app.manage | ✓ | 2.x in deps | — |

No missing dependencies.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust inline tests); no frontend test framework |
| Config file | None for frontend; inline `#[cfg(test)]` for Rust |
| Quick run command | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Full suite command | `cargo test --manifest-path src-tauri/Cargo.toml` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Automated? |
|--------|----------|-----------|-------------------|------------|
| README-01 | README contains no "MIT License" string | manual / grep | `grep -c "MIT License" README.md` (expect 0) | ✓ grep |
| README-02 | GPL-3.0-only badge present | manual / grep | `grep -c "GPL" README.md` (expect ≥1) | ✓ grep |
| README-03 | README clarity pass complete | manual review | — | manual only |
| SAFE-02 | MacPlatformObserver stored via app.manage | compile check | `cargo check --manifest-path src-tauri/Cargo.toml` | ✓ |
| AUDIT-01 | `implementation_plan.md` absent | filesystem check | `test ! -f implementation_plan.md && echo PASS` | ✓ shell |
| AUDIT-02 | Zero dead code warnings | compile check | `RUSTFLAGS="-Wdead-code" cargo check --manifest-path src-tauri/Cargo.toml 2>&1 \| grep -c "warning:"` (expect 0) + `npx tsc --noEmit` | ✓ |
| AUDIT-03 | `.gitignore` covers Thumbs.db and *.log | grep | `grep -E "Thumbs.db\|\.log" .gitignore` | ✓ grep |

### Wave 0 Gaps

None — existing test infrastructure (cargo test, tsc) covers phase verification. No new test files needed. Phase 2 is correctness/cleanup; behavioral tests are not required beyond compile-time checks.

---

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | no | No new input paths added |
| V6 Cryptography | no | — |

No new attack surface introduced. SAFE-02 closes a resource leak (not a security vulnerability). The `app.manage()` API is internal Rust — no user-facing input.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | GitHub's license detection cache takes 30–60 minutes to update after a LICENSE file commit | Pitfall 3 | Negligible — the explicit shields.io badge (D-02) does not depend on GitHub cache |
| A2 | shields.io `GPL-3.0-only` SPDX badge URL format is `img.shields.io/badge/License-GPL--3.0--only-blue` | Code Examples | Low — if URL doesn't render, adjust parameters; shields.io supports many formats |

---

## Open Questions (RESOLVED)

1. **Should the Cargo.toml gain a `license` field?**
   - What we know: `package.json` has `"license": "MIT"` (needs updating). `Cargo.toml` has no `license` field.
   - What's unclear: Is adding `license = "GPL-3.0-only"` to `Cargo.toml` in scope for this phase?
   - RESOLVED: Yes — add `license = "GPL-3.0-only"` to `Cargo.toml`. Implemented in plan 02-01 Task 1.

2. **Move or delete `implementation_plan.md`?**
   - What we know: D-07 leaves this to Claude's discretion.
   - What's unclear: Whether preserving history matters.
   - RESOLVED: Delete outright. Implemented in plan 02-02 Task 1.

3. **Polling comment mismatch (line 132, not in scope but trivial)**
   - What we know: Comment says "Poll every 3s" but `setInterval` uses 10000ms.
   - What's unclear: Whether fixing the comment falls under "README clarity pass" or scope creep.
   - RESOLVED: Fix the comment while working on App.tsx for D-10. Implemented in plan 02-04 Task 1.

---

## Sources

### Primary (HIGH confidence)
- Codebase direct inspection — all file paths and line numbers verified in this session
- `[VERIFIED: codebase run]` — cargo check, cargo clippy, tsc --noEmit all executed
- Context7 `/websites/rs_tauri_2_9_5` — `app.manage()` API contract, `Send + Sync + 'static` requirement
- Context7 `/tauri-apps/tauri-docs` — Tauri managed state pattern

### Secondary (MEDIUM confidence)
- https://v2.tauri.app/reference/javascript/api/namespaceapp/ — `getVersion()` function signature and import path `[CITED]`

### Tertiary (LOW confidence)
- shields.io badge URL format `[ASSUMED]` — training knowledge; verify by loading badge URL in browser before commit

---

## Metadata

**Confidence breakdown:**
- SAFE-02 (observer lifetime): HIGH — struct fields verified, `app.manage()` pattern verified in Context7, existing usage in lib.rs confirmed
- README fixes: HIGH — all locations verified in codebase; license file absence confirmed
- Dead code audit (AUDIT-02): HIGH — compiler tools executed with confirmed clean output
- .gitignore gaps (AUDIT-03): HIGH — `check-ignore` verified what's missing vs present
- getVersion() pattern (D-10): HIGH — Context7 + official docs confirm API

**Research date:** 2026-05-19
**Valid until:** 2026-06-19 (stable APIs — no fast-moving dependencies)
