---
id: 260806-krh
type: quick
status: in-progress
date: 2026-08-06
---

# Quick Task 260806-krh: Wire up new app icon

Replace the stock Tauri icon with the AutoMux cursor/lightning mark
(`~/Desktop/AutoMuxIcon2.png`, 2048x2048, opaque full-bleed).

## Decision (locked)

Source art is a hard-edged opaque square. macOS does **not** auto-mask `.icns`,
so it must be pre-masked. Apply an Apple-style squircle (superellipse, n=5)
with standard macOS padding (~10% inset), keeping the dark background *inside*
the shape. The same masked art feeds the Windows `.ico`.

## Tasks

### 1. Build masked master
- files: `docs/icon-src/AutoMuxIcon2-masked.png` (new)
- action: Downscale source to 1024x1024, composite through a superellipse alpha
  mask inset ~10%, output RGBA PNG.
- verify: corners fully transparent (alpha 0), centre opaque.
- done: masked master exists at 1024x1024 RGBA.

### 2. Generate icon set
- files: `src-tauri/icons/*`
- action: `npx tauri icon docs/icon-src/AutoMuxIcon2-masked.png`
- verify: `icon.icns`, `icon.ico`, `32x32.png`, `128x128.png`, `128x128@2x.png`
  all regenerated with new mtimes and non-stock content.
- done: all five files referenced by `bundle.icon` exist and are new.

### 3. Verify config
- files: `src-tauri/tauri.conf.json`
- action: Confirm `bundle.icon` lists the five generated paths. No edit expected.
- verify: `node -e` read of `bundle.icon` matches generated files on disk.
- done: every `bundle.icon` entry resolves to an existing file.
