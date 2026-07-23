---
created: 2026-07-23T00:00:00Z
title: Windows keyboard hook does not filter injected keystrokes
area: general
severity: blocker
files:
  - src-tauri/src/platform/windows/mod.rs:494-540
  - src-tauri/src/platform/macos/observer.rs:279-281
---

## Problem

`hook_callback` (`src-tauri/src/platform/windows/mod.rs:494-540`) never inspects `KBDLLHOOKSTRUCT.flags` for `LLKHF_INJECTED`, unlike the macOS CGEventTap which guards its own injected events (`src-tauri/src/platform/macos/observer.rs:279-281`).

As a result a macro's own `SendInput`-injected keystroke can match another macro's configured hotkey — or the hardcoded Ctrl+Shift+Q emergency-stop combo — and self-trigger a toggle or emergency stop on Windows, directly threatening the Core Value ("execution must be accurate").

Severity=Critical per 09-REVIEW.md CR-02, confirmed real via direct source read in 09-VERIFICATION.md (2026-07-22T21:30:00Z, "Out-of-Scope Findings" #2).

out-of-scope-for=Phase 9: `git log -S` shows the missing filter dates to when the hook was first written (v1.0 commit faa1e1e) and was never touched for filtering by 08-03 or 09-10 (09-10 only removed a redundant registry rebuild in the same function); it is more naturally scoped to Phase 8's Hotkey Reliability & Conflict Safety domain; no later ROADMAP phase addresses it.

## Solution

In `hook_callback`, check `KBDLLHOOKSTRUCT.flags & LLKHF_INJECTED` and, for injected events, skip BOTH the emergency-stop (Ctrl+Shift+Q) check and hotkey matching — mirroring the macOS tap's existing injected-event guard. Fix sketch is in 09-REVIEW.md CR-02.

Recommend a test/note that an injected keystroke matching a bound hotkey does not dispatch a toggle.
