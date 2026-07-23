---
created: 2026-07-23T00:00:00Z
title: LoadProfile failure wipes all macros (memory + disk)
area: general
severity: blocker
files:
  - src-tauri/src/state/mod.rs:800-845
---

## Problem

`Intent::LoadProfile` (`src-tauri/src/state/mod.rs:800-845`) calls `self.state.macros.clear()` unconditionally at the top of the handler, BEFORE the disk read. On an `Err` from `profile_mgr.load_profile` (missing file, corrupted JSON, transient I/O), `state.macros` is never restored, and the trailing unconditional `self.auto_save_default().await` then persists the emptied state to `default.json`.

Net effect: a single failed "Load Profile" click destroys every configured macro in memory and on disk, with only a transient toast as evidence.

Severity=Critical per 09-REVIEW.md CR-01, confirmed real via direct source read in 09-VERIFICATION.md (2026-07-22T21:30:00Z, "Out-of-Scope Findings" #1).

out-of-scope-for=Phase 9: it is a persistence/error-handling defect in an unrelated intent (bulk profile replacement), not a "macro B triggering cancels macro A" interaction; `git log -S` shows the clear-before-load shape predates Phase 9 (v1.0 MVP commit 2c6bc64); no 09-* plan touched this handler; no later ROADMAP phase addresses it.

## Solution

Only clear/mutate `state.macros` AFTER a successful disk read — load into a local and swap in only on `Ok`; on `Err` leave existing macros untouched and do not run `auto_save_default()` on the failure path. Fix sketch is in 09-REVIEW.md CR-01.

Recommend a regression test proving a failed load preserves prior macros and does not overwrite `default.json`.
