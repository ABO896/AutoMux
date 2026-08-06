---
id: 260806-krh
type: quick
status: complete
date: 2026-08-06
commit: 257f0e7
---

# Quick Task 260806-krh: Wire up new app icon — Summary

**Status:** complete

## What was done

1. **Masked master** — `docs/icon-src/AutoMuxIcon2-masked.png` (1024x1024 RGBA).
   Source downscaled from 2048x2048 and composited through a superellipse
   (n=5) alpha mask at 10% inset, 4x supersampled edge.
2. **Icon set regenerated** — `npx tauri icon` (tauri-cli 2.11.1) rewrote all
   50 files under `src-tauri/icons/` (desktop, Windows Store, iOS, Android).
3. **Config verified** — `bundle.icon` in `tauri.conf.json` already listed the
   five correct paths; no edit required.

## Deviation from request

The request said "use as-is if possible." The source was a full-bleed **opaque**
square — macOS does not auto-mask `.icns`, so shipping it unmodified would put a
hard-edged black tile in the Dock. Squircle masking was confirmed with the user
before proceeding.

## Verification

| Check | Result |
|---|---|
| Masked master corners transparent | ✓ alpha 0 at all four corners, 255 at centre |
| Generator ✦ watermark removed | ✓ 0 bright pixels in bottom-right quadrant (clipped by inset) |
| All 5 `bundle.icon` paths resolve | ✓ 32x32, 128x128, 128x128@2x, icon.icns, icon.ico |
| `.icns` well-formed | ✓ `ic09` type, 1024x1024 |
| `.ico` well-formed | ✓ 6 entries: 16/24/32/48/64/256 |
| `tauri.conf.json` parses | ✓ |

Not run: a full `tauri build`. Icons are bundle-time assets only and do not
affect Rust/TS compilation; the format checks above cover what the bundler reads.

## Follow-up

`docs/ICON-SPEC.md` still describes the app as shipping the stock Tauri icon and
claims macOS "applies a rounded-square mask" to `.icns` (it does not). Worth
correcting if the spec is kept.
