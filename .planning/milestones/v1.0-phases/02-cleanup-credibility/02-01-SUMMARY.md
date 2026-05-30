---
plan: 02-01
phase: 02-cleanup-credibility
status: complete
commit: 157f61b
---

## Summary

Restored legal correctness and improved README credibility in one shippable commit.

## What Was Built

- **LICENSE**: Created at repository root with full GPL-3.0 verbatim text (35,149 bytes) fetched from gnu.org/licenses/gpl-3.0.txt.
- **package.json**: `"license"` field changed from `"MIT"` to `"GPL-3.0-only"`.
- **src-tauri/Cargo.toml**: Added `license = "GPL-3.0-only"` in the `[package]` section.
- **README.md**:
  - Replaced GitHub auto-license badge with explicit `img.shields.io/badge/License-GPL--3.0--only-blue` badge (no GitHub cache dependency).
  - Rewrote opening paragraph to lead with user value: defines what AutoMux does, names macOS + Windows support, then states technical qualities.
  - Updated license prose from "MIT License" to "GNU General Public License v3".
  - Existing Release and Build-status badges retained.
  - Feature list verified — all bullets reflect implemented behavior; no Phase 3 claims.

## Acceptance Criteria

| Check | Result |
|-------|--------|
| `grep -c "MIT" README.md` = 0 | ✓ 0 |
| `grep -c "GPL-3.0-only badge" README.md` ≥ 1 | ✓ 1 |
| `grep -c "GNU General Public License v3" README.md` ≥ 1 | ✓ 1 |
| macOS + Windows in README | ✓ both present |
| Old GitHub auto-badge absent | ✓ 0 |
| Release + Build badges present | ✓ both present |
| Old jargon lead absent | ✓ 0 |
| `package.json` license = GPL-3.0-only | ✓ |
| `Cargo.toml` license = GPL-3.0-only | ✓ |
| `cargo check` exits 0 | ✓ Finished dev profile |
| LICENSE ≥ 30,000 bytes | ✓ 35,149 bytes |

## Self-Check: PASSED
