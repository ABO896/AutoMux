---
phase: 04-ci-hardening
plan: "01"
subsystem: ci
tags:
  - ci
  - github-actions
  - supply-chain
  - security
dependency_graph:
  requires: []
  provides:
    - Pinned third-party action SHAs in release.yml
  affects:
    - .github/workflows/release.yml
tech_stack:
  added: []
  patterns:
    - SHA pinning for third-party GitHub Actions with inline tag comment annotation
key_files:
  created: []
  modified:
    - .github/workflows/release.yml
decisions:
  - "dtolnay/rust-toolchain@stable resolved from branch ref (not a tag): SHA 29eef336d9b2848a0b548edc03f92a220660cdb8"
  - "tauri-apps/tauri-action@v0 is an annotated tag object, dereferenced to commit SHA 84b9d35b5fc46c1e45415bdb6144030364f7ebc5"
  - "actions/checkout@v4 and actions/setup-node@v4 intentionally left on floating tags per D-04"
metrics:
  duration: "2 minutes"
  completed: "2026-05-30T16:03:50Z"
  tasks_completed: 2
  files_modified: 1
---

# Phase 04 Plan 01: SHA Pinning for Third-Party GitHub Actions Summary

**One-liner:** Pinned dtolnay/rust-toolchain and tauri-apps/tauri-action to full 40-char commit SHAs with inline tag comments, closing CI-02 supply-chain risk.

## What Was Done

Replaced two floating-tag references in `.github/workflows/release.yml` with pinned commit SHAs per D-05, resolving SHAs live from the GitHub API at execution time.

### Resolved SHAs (snapshot: 2026-05-30T16:02:16Z)

| Action | Tag | Resolved Commit SHA | Notes |
|--------|-----|---------------------|-------|
| `dtolnay/rust-toolchain` | `stable` | `29eef336d9b2848a0b548edc03f92a220660cdb8` | `stable` is a branch ref, not a tag — SHA resolved from `refs/heads/stable` |
| `tauri-apps/tauri-action` | `v0` | `84b9d35b5fc46c1e45415bdb6144030364f7ebc5` | `v0` is an annotated tag — dereferenced from tag object `fce9c6108b31ea247710505d3aaaa893ee6768d4` to commit SHA |

### Intentionally Not Pinned (D-04)

| Action | Reason |
|--------|--------|
| `actions/checkout@v4` | GitHub-owned action — trusted per D-04; left on floating tag |
| `actions/setup-node@v4` | GitHub-owned action — trusted per D-04; left on floating tag |

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Resolve commit SHAs for dtolnay/rust-toolchain@stable and tauri-apps/tauri-action@v0 | (discovery only — no commit) | None |
| 2 | Pin the two third-party action references in release.yml to resolved SHAs | 3ba8100 | `.github/workflows/release.yml` |

## Verification Results

All acceptance criteria met:

- `uses: dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8  # stable` present
- `uses: tauri-apps/tauri-action@84b9d35b5fc46c1e45415bdb6144030364f7ebc5  # v0` present
- No floating tag references remain for either pinned action
- `actions/checkout@v4` and `actions/setup-node@v4` unchanged
- YAML is valid (verified via Ruby `YAML.load_file`)
- Diff is scoped to exactly 2 changed `uses:` lines (4 diff lines total: 2 removed + 2 added)
- SHA idempotency confirmed: re-resolution yielded identical SHAs on second run

## Deviations from Plan

**1. [Rule 1 - Discovery] dtolnay/rust-toolchain@stable is a branch, not a tag**

- **Found during:** Task 1
- **Issue:** The GitHub API endpoint `refs/tags/stable` returned 404 for `dtolnay/rust-toolchain`. `stable` is a branch ref, not a tag.
- **Fix:** Resolved from `refs/heads/stable` instead. SHA `29eef336d9b2848a0b548edc03f92a220660cdb8` is the current HEAD of that branch — this is the correct SHA to pin (same SHA that `dtolnay/rust-toolchain@stable` resolves to in GitHub Actions).
- **Files modified:** None (discovery task)
- **Commit:** N/A

## Known Stubs

None.

## Threat Flags

No new security surface introduced. This change reduces attack surface by removing floating tag references.

## Self-Check: PASSED

- `.github/workflows/release.yml` exists and contains both pinned SHAs: VERIFIED
- Commit `3ba8100` exists: VERIFIED (`git log --oneline -1` confirmed)
- No files deleted: VERIFIED
