# Development Rules & Constraints

## UI & Aesthetics
- **Framework**: SolidJS + TailwindCSS.
- **Design Language**: Minimal, utilitarian, high information density. Reference: Linear, Raycast, Superhuman.
- **Color Palette**: Monochrome heavy, subtle borders, deep dark mode. Avoid "gamer" aesthetics (no neon, no rgb).
- **Navigation**: Keyboard-first. Every action must be accessible via keyboard shortcuts or a command palette.
- **Component Size**: Keep components small and focused. No giant god-components.

## Backend & Systems
- **Framework**: Tauri with a Rust backend.
- **Concurrency**: Use `tokio` for async scheduling.
- **State Management**: Use the Actor pattern in Rust to avoid Mutex lock contention. Communicate via channels.
- **Timers**: Use `tokio::time::interval` with `MissedTickBehavior::Skip` to avoid timer drift and stampeding.
- **Safety First**: 
  - Ensure injected inputs cannot trigger our own listeners (infinite loop).
  - Implement a hardcoded, unchangeable Emergency Stop hotkey.
  - Implement key-release cleanup on task cancellation to prevent sticky modifier keys.

## What NOT to do
- No React or Electron.
- No polling architecture for active window detection; use OS event hooks.
- No arbitrary `unwrap()` in Rust; handle errors gracefully and propagate to the UI if necessary.
- Do not bloat the UI with unused Tailwind classes or unnecessary npm dependencies.
