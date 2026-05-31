# Technology Stack

**Analysis Date:** 2026-05-15

## Languages

**Primary:**
- TypeScript ~5.6.2 - Frontend UI (`src/`)
- Rust 2021 edition - Backend/core engine (`src-tauri/src/`)

**Secondary:**
- CSS - UI styling (`src/App.css`)
- HTML - App shell (`index.html`)

## Runtime

**Environment:**
- Node.js 24 (CI-pinned via `actions/setup-node@v4`)
- Rust stable toolchain (managed via `dtolnay/rust-toolchain@stable`)

**Package Manager:**
- npm
- Lockfile: `package-lock.json` present

## Frameworks

**Core:**
- Tauri 2 - Desktop app shell and IPC bridge; ties Rust backend to web frontend (`src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`)
- SolidJS 1.9.3 - Reactive UI framework (`src/`)

**Build/Dev:**
- Vite 6.0.3 - Frontend bundler and dev server (`vite.config.ts`)
- vite-plugin-solid 2.11.0 - SolidJS Vite integration (`vite.config.ts`)
- Tailwind CSS 4.3.0 via `@tailwindcss/vite` plugin (`vite.config.ts`)
- tauri-build 2 - Rust build script for Tauri (`src-tauri/Cargo.toml` `[build-dependencies]`)

**Testing:**
- No frontend test framework detected
- Rust unit tests in `src-tauri/src/persistence.rs` (inline `#[cfg(test)]` module, standard `cargo test`)

## Key Dependencies

**Critical:**
- `@tauri-apps/api` ^2 - Frontend ↔ Rust IPC (`invoke`, `listen` for events)
- `@tauri-apps/plugin-opener` ^2 - Shell/URL open capability
- `tauri-plugin-opener` 2 - Rust-side opener plugin
- `tokio` 1 (rt-multi-thread, sync, time, macros) - Async runtime powering the scheduler and state actor
- `serde` + `serde_json` 1 - JSON serialization for IPC data types and profile persistence
- `uuid` 1 (v4, serde) - Unique IDs for macro configs

**Platform — macOS:**
- `core-graphics` 0.24.0 - CGEvent tap for input injection
- `core-foundation` 0.10.0 / `core-foundation-sys` 0.8.0 - macOS CF API bindings
- `objc` 0.2.7 / `objc2` 0.6.4 - Objective-C runtime interop
- `objc2-app-kit` 0.3.2 / `objc2-foundation` 0.3.2 - AppKit/Foundation bindings
- `cocoa` 0.26.1 - Higher-level Cocoa bindings
- `block2` 0.6.2 - Objective-C block support

**Platform — Windows:**
- `windows` 0.61 with features: `Win32_UI_Input_KeyboardAndMouse`, `Win32_UI_WindowsAndMessaging`, `Win32_Foundation`, `Win32_System_Threading`, `Win32_UI_Accessibility` - Win32 API for input injection and window tracking

## Configuration

**Environment:**
- `TAURI_DEV_HOST` env var: optional; enables remote device dev mode in `vite.config.ts`
- `GITHUB_TOKEN` secret: used in CI release workflow (`release.yml`) — not an app runtime secret

**Build:**
- `vite.config.ts` - Frontend build config; dev server on port 1420 (strict)
- `tsconfig.json` - TypeScript compiler; strict mode enabled, `jsxImportSource: solid-js`
- `tsconfig.node.json` - TypeScript config for Vite/Node tooling
- `src-tauri/tauri.conf.json` - Tauri app config (window dimensions, bundle targets, CSP, macOS entitlements)
- `src-tauri/Cargo.toml` - Rust crate manifest with platform-conditional dependencies

## Platform Requirements

**Development:**
- Node.js 24+
- Rust stable toolchain
- macOS: Accessibility permissions required at runtime (checked via `AXIsProcessTrusted`)
- macOS minimum deployment: 12.0 (Monterey)

**Production:**
- Bundle targets: `dmg` (macOS), `nsis` installer (Windows)
- macOS: signed builds require `signingIdentity` in `tauri.conf.json` (currently `null` — unsigned)
- Windows: code signing requires `certificateThumbprint` (currently `null` — unsigned); timestamp URL set to `http://timestamp.digicert.com`
- App identifier: `com.alvaro.automux`

---

*Stack analysis: 2026-05-15*
