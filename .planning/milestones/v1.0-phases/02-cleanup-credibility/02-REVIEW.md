---
phase: 02-cleanup-credibility
reviewed: 2026-05-20T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - .gitignore
  - README.md
  - package.json
  - src-tauri/Cargo.toml
  - src-tauri/src/lib.rs
  - src/App.tsx
  - LICENSE
findings:
  critical: 0
  warning: 2
  info: 4
  total: 6
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-05-20T00:00:00Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

This phase introduced four changes: (1) a GPL-3.0-only LICENSE file, (2) README and badge polish, (3) `.gitignore` additions, and (4) runtime `getVersion()` wiring in `App.tsx` + `MacPlatformObserver` lifetime fix in `lib.rs`.

The two substantive code changes (`app.manage(observer)` and the `getVersion()` addition) are structurally sound. No critical bugs or security vulnerabilities were introduced. The findings below are two warnings and four info items covering an outdated documentation link, a missing project copyright notice in the LICENSE file, the `authors` placeholder in `Cargo.toml`, a missing EOF newline, a write-only signal, and the Windows observer having no equivalent lifetime anchor.

---

## Warnings

### WR-01: README links to Tauri v1 documentation

**File:** `README.md:50`
**Issue:** The "Building from Source" section links to `https://tauri.app/v1/guides/getting-started/prerequisites`. The project uses Tauri 2 (confirmed in `Cargo.toml` and `package.json`). The Tauri v1 and v2 prerequisite pages differ — v2 requires additional platform-specific tools on some OS configurations. A user following this link will receive incorrect setup instructions.
**Fix:** Replace the URL with the Tauri v2 prerequisites page:
```diff
-along with the [Tauri CLI prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites).
+along with the [Tauri CLI prerequisites](https://v2.tauri.app/start/prerequisites/).
```

### WR-02: Windows observer has no lifetime anchor — asymmetric with macOS fix

**File:** `src-tauri/src/lib.rs:73-79`
**Issue:** The macOS `MacPlatformObserver` is now correctly anchored to Tauri managed state via `app.manage(observer)` (SAFE-02). The Windows `WindowsPlatformObserver` is not — the local `observer` variable goes out of scope at the end of the `setup` closure and is dropped. On Windows, if `WindowsPlatformObserver` performs any resource acquisition in `start_observing()` (e.g., a Win32 event hook handle), that resource would be released prematurely. Even if the Win32 hook currently survives via OS-level registration, this is an unanchored pattern that will silently break on any future refactor that adds a `Drop` impl.
```rust
// Current (Windows path — observer dropped at end of setup closure):
let mut observer = platform::windows::WindowsPlatformObserver::new();
observer.start_observing();
// observer dropped here — no lifetime guarantee

// Fix: apply the same anchor pattern as macOS:
let mut observer = platform::windows::WindowsPlatformObserver::new();
observer.start_observing();
app.manage(observer); // anchor to app lifetime
```

---

## Info

### IN-01: LICENSE file missing project copyright notice

**File:** `LICENSE:1`
**Issue:** The LICENSE file contains only the bare GPL-3.0 license text. There is no project-specific copyright notice at the top (e.g., `Copyright (C) 2024-2026 <author name>`). GPL requires copyright holders to be identified. Without this, the LICENSE file is legally incomplete — it establishes the terms but does not assert copyright ownership for this project.
**Fix:** Add a project copyright notice before the GPL text:
```
Copyright (C) 2024-2026 Alvaro Balduz

                    GNU GENERAL PUBLIC LICENSE
...
```

### IN-02: `authors` field in Cargo.toml is a placeholder

**File:** `src-tauri/Cargo.toml:5`
**Issue:** `authors = ["you"]` is the default Cargo init placeholder and was never replaced. This surfaces in `cargo metadata` output and crate registries.
**Fix:**
```toml
authors = ["Alvaro Balduz <alvaro.balduz@gmail.com>"]
```

### IN-03: `.gitignore` missing trailing newline

**File:** `.gitignore:27`
**Issue:** The file ends with `*.log` with no trailing newline character. POSIX defines a text file as ending with a newline. Some tools (e.g., `git diff`, `wc -l`, certain editors) will emit a "No newline at end of file" warning or mis-count lines.
**Fix:** Add a newline after the last line `*.log`.

### IN-04: `setLoading` signal is write-only — dead state

**File:** `src/App.tsx:72`
**Issue:** The `loading` signal getter is intentionally discarded via `const [, setLoading] = createSignal(true)`, and `setLoading(false)` is called in the `finally` block (line 121). Since nothing reads the `loading` value, the signal and its setter are dead code. The `finally` call has no observable effect. `noUnusedLocals` does not catch this because `setLoading` is technically used (written to), but the write is a no-op in terms of UI behaviour.
**Fix:** Either wire the `loading` signal to a UI element (e.g., a spinner during initial fetch), or remove the signal and the `setLoading(false)` call entirely.

---

_Reviewed: 2026-05-20T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
