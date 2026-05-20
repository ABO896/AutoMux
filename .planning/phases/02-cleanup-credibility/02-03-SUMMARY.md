---
plan: 02-03
phase: 02-cleanup-credibility
status: complete
commit: 49fdd08
---

## Summary

Closed the MacPlatformObserver resource leak by storing the observer in Tauri's managed state.

## What Was Built

Single-line addition to `src-tauri/src/lib.rs` inside the existing `#[cfg(target_os = "macos")]` setup block:

**Before:**
```rust
observer.start_observing();
// Observer is long-lived; it does not implement Drop, so the token is kept alive natively.
```

**After:**
```rust
observer.start_observing();
// Observer is owned by Tauri managed state for full app lifetime (SAFE-02).
app.manage(observer);
```

The `app.manage(observer)` call moves the observer into Tauri's state registry, which holds it for the full application lifetime. On app shutdown, Tauri drops all managed state — at which point `MacPlatformObserver::drop` runs `stop_observing`, making `stop_observing` reachable code for the first time.

`start_observing()` takes `&mut self` and therefore precedes the `app.manage(observer)` move (per RESEARCH.md Pitfall 2). `tauri::Manager` was already imported; no new use statement needed. `MacPlatformObserver` has a single `Option<usize>` field — no `Arc` or `Mutex` wrapping required.

## Acceptance Criteria

| Check | Result |
|-------|--------|
| `grep -c "app.manage(observer)" src-tauri/src/lib.rs` = 1 | ✓ 1 |
| `app.manage(observer)` inside `#[cfg(target_os = "macos")]` block | ✓ line 71 |
| `app.manage(observer)` after `start_observing()` in source order | ✓ lines 69, 71 |
| Old stale comment removed from macOS block | ✓ replaced with SAFE-02 citation |
| `grep -c "SAFE-02" lib.rs` ≥ 1 | ✓ 1 |
| `cargo check` exits 0 | ✓ Finished dev profile |
| `cargo clippy -- -D warnings` exits 0 | ✓ |
| No new IPC command added (`ipc/mod.rs` unchanged) | ✓ |
| No `Arc::new(observer)` or `Mutex::new(observer)` | ✓ 0 |
| `observer.rs` unchanged | ✓ |

## stop_observing Now Reachable

`stop_observing` is invoked from `MacPlatformObserver`'s `Drop` impl (if present) or becomes accessible via the managed state handle at shutdown. Previously the observer was dropped silently at the end of the setup closure without `stop_observing` being called, leaving the CFRunLoop event tap registration dangling.

## Self-Check: PASSED
