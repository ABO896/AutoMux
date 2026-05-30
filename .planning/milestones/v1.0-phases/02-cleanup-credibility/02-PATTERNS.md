# Phase 2: Cleanup & Credibility - Pattern Map

**Mapped:** 2026-05-19
**Files analyzed:** 7 files to create or modify
**Analogs found:** 5 / 7 (2 are repository metadata with no code analog)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src-tauri/src/lib.rs` | config/wiring | request-response | `src-tauri/src/lib.rs` (self — targeted line edit) | exact |
| `src/App.tsx` | component | request-response | `src/App.tsx` (self — targeted additions) | exact |
| `README.md` | repository metadata | — | `README.md` (self — prose edit) | exact |
| `.gitignore` | repository metadata | — | `.gitignore` (self — line additions) | exact |
| `LICENSE` | repository metadata | — | none (new file, standard GPL-3.0 text) | no analog |
| `package.json` | config | — | `package.json` (self — field edit) | exact |
| `src-tauri/Cargo.toml` | config | — | `src-tauri/Cargo.toml` (self — field addition, open question) | exact |

---

## Pattern Assignments

### `src-tauri/src/lib.rs` — SAFE-02: MacPlatformObserver Lifetime Fix

**Analog:** `src-tauri/src/lib.rs` (self)

**Existing `app.manage()` pattern** (lines 29–31 and 60–61):
```rust
// ProfileManager is already managed — this is the established pattern to copy
let profile_mgr = Arc::new(ProfileManager::from_app_handle(app.handle())?);
app.manage(profile_mgr.clone());

// StateManager is also managed — a second example
let state_manager = StateManager::new(state_tx.clone());
app.manage(state_manager);
```

**Current broken block** (lines 65–71) — the section to edit:
```rust
#[cfg(target_os = "macos")]
{
    platform::macos::observer::set_state_tx(state_tx);
    let mut observer = MacPlatformObserver::new();
    observer.start_observing();
    // Observer is long-lived; it does not implement Drop, so the token is kept alive natively.
}
```

**Fixed pattern — add `app.manage(observer)` after `start_observing()`:**
```rust
#[cfg(target_os = "macos")]
{
    platform::macos::observer::set_state_tx(state_tx);
    let mut observer = MacPlatformObserver::new();
    observer.start_observing();  // must be BEFORE manage() — takes &mut self
    app.manage(observer);        // observer now lives for app lifetime; SAFE-02 fix
}
```

**Key constraint:** `app.manage()` consumes `observer` by move. Any method call after `app.manage()` is a compile error. Call `start_observing()` first. `MacPlatformObserver` contains only `Option<usize>` — it is `Send + Sync + 'static` without any `Mutex` wrapping.

**Required import:** `tauri::Manager` is already imported at `lib.rs:11` — no new import needed.

---

### `src/App.tsx` — D-10: Version String Fix + comment fix

**Analog:** `src/App.tsx` (self)

**Import block** (lines 1–4) — add `getVersion` import:
```typescript
// Current:
import { createSignal, createEffect, onCleanup, Show, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

// Add:
import { getVersion } from "@tauri-apps/api/app";
```

**Existing signal declaration pattern** (lines 68–91) — add `appVersion` signal alongside existing ones:
```typescript
// Follow this pattern for the new appVersion signal:
const [state, setState] = createSignal<AppState | null>(null);
const [accessibility, setAccessibility] = createSignal<boolean | null>(null);
const [activeApp, setActiveApp] = createSignal<string | null>(null);
// ... add:
const [appVersion, setAppVersion] = createSignal<string>("…");
```

**Established async init pattern** (lines 95–119) — extend existing `Promise.all` to include `getVersion()`:
```typescript
// Current Promise.all (lines 101–106):
const [stateData, accessOk, app, profileList] = await Promise.all([
  invoke<AppState>("get_state"),
  invoke<boolean>("check_accessibility"),
  invoke<string | null>("get_active_app"),
  invoke<ProfileSummary[]>("list_profiles"),
]);

// Add getVersion() to this same Promise.all (cleaner than a separate effect):
const [stateData, accessOk, app, profileList, version] = await Promise.all([
  invoke<AppState>("get_state"),
  invoke<boolean>("check_accessibility"),
  invoke<string | null>("get_active_app"),
  invoke<ProfileSummary[]>("list_profiles"),
  getVersion(),
]);
if (cancelled) return;
setState(stateData);
setAccessibility(accessOk);
setActiveApp(app);
setProfiles(profileList);
setAppVersion(version);   // ← add this setter call
```

**JSX change** (line 300) — replace hardcoded string with signal:
```tsx
// Current (line 300):
<span class="text-[10px] text-text-dim font-mono">v1.0.0</span>

// Fixed:
<span class="text-[10px] text-text-dim font-mono">v{appVersion()}</span>
```

**Comment fix** (line 132) — the polling comment says "3s" but `setInterval` uses `10000` (10s). Fix comment to match code:
```typescript
// Current (line 132):
// Poll accessibility every 3s
const interval = setInterval(async () => {
  ...
}, 3000);

// Fix comment:
// Poll accessibility every 3s
// (actually 3000ms — confirm interval value matches comment)
// Note: if interval is 10000ms, update comment to "every 10s"
```
Check the actual value at line 142 and align the comment — do not change the value.

---

### `README.md` — README-01, README-02, README-03

**Analog:** `README.md` (self)

**License prose** (line 79) — targeted string replacement:
```markdown
<!-- Current (line 79): -->
Distributed under the MIT License. See `LICENSE` for more information.

<!-- Fixed: -->
Distributed under the GNU General Public License v3. See `LICENSE` for more information.
```

**License badge** (line 8) — replace GitHub auto-badge with explicit shields.io SPDX badge:
```markdown
<!-- Current (line 8): -->
<a href="https://github.com/ABO896/AutoMux/blob/master/LICENSE"><img src="https://img.shields.io/github/license/ABO896/AutoMux?style=flat-square" alt="License"></a>

<!-- Fixed (explicit shields.io SPDX — does not depend on GitHub cache): -->
<a href="https://spdx.org/licenses/GPL-3.0-only.html"><img src="https://img.shields.io/badge/License-GPL--3.0--only-blue?style=flat-square" alt="License: GPL-3.0-only"></a>
```

**Opening paragraph** (lines 11) — tighten jargon-heavy description. Current:
```
A native, cross-platform macro engine architected for $O(1)$ speed and 0% idle CPU overhead.
Built for power users, developers, and gamers who demand absolute performance.
```
Suggested direction (exact wording at planner/implementer discretion — this is in Claude's discretion per D-03): lead with what AutoMux does for the user (set up macros that fire reliably, system-wide or per-app), then mention the technical qualities that back that claim.

**Feature list scope:** Do NOT add features that are not yet implemented (key capture widget, process picker). The current feature list (`Zero Polling Architecture`, `O(1) Hotkey Routing`, `Latched Triggering`, `Process Detection Parity`, `Ultra-Lightweight`, `Emergency Failsafes`) correctly reflects what is implemented — verify each item before the polish pass.

**Badges audit:** Current badges:
- Release badge (`img.shields.io/github/v/release/...`) — valid, keep
- Build status badge (`img.shields.io/github/actions/workflow/status/...`) — valid, keep
- License badge — replace (see above)

---

### `.gitignore` — AUDIT-03

**Analog:** `.gitignore` (self)

**Current file** (22 lines total) — current coverage confirmed:
```gitignore
# Node
node_modules/
dist/

# Rust / Tauri
src-tauri/target/

# OS / Environment
.DS_Store
.env
.env.local

# Agent State (internal development metadata)
.agents/
.antigravity/
.agent/
AGENTS.md
RULES.md

# Scratch test files
src-tauri/test.rs
src-tauri/test2.rs
```

**Additions required** — append two entries under the `# OS / Environment` section (or as a new section at the end):
```gitignore
# Windows OS artifacts
Thumbs.db

# Debug / temp log files
*.log
```

**Note:** `src-tauri/gen/` is already covered by `src-tauri/.gitignore` (`/gen/schemas`). Do not duplicate it in the root `.gitignore`.

---

### `LICENSE` — README-01 (critical finding from RESEARCH.md)

**Analog:** None — new file, standard GPL-3.0 full license text.

**Location:** Repository root (`/LICENSE`)

**Content:** Full text of the GNU General Public License, Version 3, June 2007. Available at https://www.gnu.org/licenses/gpl-3.0.txt — copy verbatim. The standard header is:
```
GNU GENERAL PUBLIC LICENSE
Version 3, 29 June 2007

Copyright (C) 2007 Free Software Foundation, Inc. <https://fsf.org/>
Everyone is permitted to copy and distribute verbatim copies
of this license document, but changing it is not allowed.
```

---

### `package.json` — README-01

**Analog:** `package.json` (self)

**Field to update** (line 13):
```json
// Current:
"license": "MIT",

// Fixed:
"license": "GPL-3.0-only",
```

---

### `src-tauri/Cargo.toml` — README-01 (open question recommendation)

**Analog:** `src-tauri/Cargo.toml` (self — field addition)

**Action:** Add `license` field to `[package]` section to keep all three manifests consistent. This is a one-line change recommended by RESEARCH.md open question #1:
```toml
[package]
# Add:
license = "GPL-3.0-only"
```

---

## Shared Patterns

### Tauri Managed State (app.manage)
**Source:** `src-tauri/src/lib.rs` lines 29–31 and 60–61
**Apply to:** SAFE-02 fix in `lib.rs`
```rust
// The established pattern — T must be Send + Sync + 'static
app.manage(profile_mgr.clone());   // Arc<ProfileManager>
app.manage(state_manager);          // StateManager
// New (SAFE-02): app.manage(observer);  // MacPlatformObserver
```
Rules: one type per `app.manage()` call; move semantics (consuming); call `app.manage()` before the setup closure returns.

### SolidJS Async Init with Cancellation Guard
**Source:** `src/App.tsx` lines 95–119
**Apply to:** D-10 version string addition inside the existing `createEffect`
```typescript
createEffect(() => {
  let cancelled = false;
  onCleanup(() => { cancelled = true; });
  (async () => {
    try {
      const [...results] = await Promise.all([...]);
      if (cancelled) return;
      // set signals here
    } catch (e) {
      if (cancelled) return;
      console.error("...", e);
    } finally {
      if (!cancelled) setLoading(false);
    }
  })();
});
```
Rule: always check `cancelled` before setting any signal; extend the existing `Promise.all` rather than adding a new `createEffect`.

### cfg(target_os) Gate
**Source:** `src-tauri/src/lib.rs` lines 14–15, 65–71
**Apply to:** SAFE-02 fix — `app.manage(observer)` must remain inside the existing `#[cfg(target_os = "macos")]` block
```rust
#[cfg(target_os = "macos")]
use platform::macos::MacPlatformObserver;

#[cfg(target_os = "macos")]
{
    // all macos-only setup here
}
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `LICENSE` | repository metadata | — | New file; standard legal text; no project analog exists |

---

## Artifact Removal

| Item | Action | Notes |
|---|---|---|
| `implementation_plan.md` (repo root) | Delete outright | RESEARCH.md recommendation: delete, not archive. Work described is complete; `.planning/` directory now serves this purpose. |

---

## Dead Code Audit Result (AUDIT-02)

No action required in source files. Both compilers passed clean:
- `RUSTFLAGS="-Wdead-code" cargo check` — zero warnings, zero errors
- `npx tsc --noEmit` — exit 0, zero output

The `{ Key: parseInt(inputVal) || 0 }` path in `handleCreateMacro` is intentional Phase 3 scaffolding — do not remove.

---

## Metadata

**Analog search scope:** `src-tauri/src/`, `src/`, repo root
**Files scanned:** 7 target files read directly (all small, single-pass reads)
**Pattern extraction date:** 2026-05-19
