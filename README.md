<div align="center">
  <img src="docs/assets/icon-256.png" width="128" height="128" alt="AutoMux icon">

  <h1>AutoMux</h1>
  <p><strong>The Zero-Footprint Automation Engine</strong></p>

  <p>
    <a href="https://github.com/ABO896/AutoMux/releases"><img src="https://img.shields.io/github/v/release/ABO896/AutoMux?style=flat-square&color=6366f1" alt="Release"></a>
    <a href="https://github.com/ABO896/AutoMux/actions"><img src="https://img.shields.io/github/actions/workflow/status/ABO896/AutoMux/release.yml?style=flat-square" alt="Build Status"></a>
    <a href="https://spdx.org/licenses/GPL-3.0-only.html"><img src="https://img.shields.io/badge/License-GPL--3.0--only-blue?style=flat-square" alt="License: GPL-3.0-only"></a>
    <img src="https://img.shields.io/badge/macOS-12%2B-black?style=flat-square" alt="macOS 12+">
    <img src="https://img.shields.io/badge/Windows-x64-0078D4?style=flat-square" alt="Windows x64">
  </p>

  <p>AutoMux lets you define multi-step macros — clicks, key presses, timing — and run them system-wide or scoped to a specific app, on both <b>macOS</b> and <b>Windows</b>. It's built for gamers, productivity power users, and anyone automating repetitive mouse and keyboard tasks. The engine is event-driven with a bare-metal Rust core: zero polling, sub-millisecond dispatch, and roughly 60 MB RAM.</p>
</div>

<br/>

## ✨ Features

- **🎯 Zero Polling Architecture** — Completely event-driven. The engine sleeps at 0% CPU until the OS pushes an explicit notification. No busy-waiting. No wasted cycles.
- **⚡ $O(1)$ Hotkey Routing** — Trigger resolutions use constant-time `HashMap` lookups, entirely stripping out lock contention on the hot path for sub-millisecond input injection.
- **🔀 Parallel Macro Execution** — Triggering one macro never blocks, queues, or cancels another. Multiple macros run genuinely concurrently on both platforms, with per-macro card indicators (firing / waiting / held / disabled) so concurrent state is visible at a glance.
- **🔄 Latched Triggering** — Interval-based **Pulse** firing and continuous **Hold** latches, composed into multi-step sequences.
- **🛡️ Process Detection Parity** — Context-aware macros automatically engage or disengage based on the active application window, natively on both macOS and Windows.
- **🎹 Hotkey Conflict Safety** — Full practical key range (letters, numbers, F1–F12, modifier combinations). Binding a key already claimed by another macro raises an explicit conflict error instead of silently overwriting it.
- **📁 Macro Profiles** — Save, load, and delete named macro sets as JSON. Your working set is auto-saved to a `default` profile and restored on launch.
- **🎨 Light / Dark / Follow-OS Theming** — Redesigned sidebar UI with an Apple-inspired layout on macOS and a matching modern shell on Windows.
- **🪶 Ultra-Lightweight** — Roughly 60 MB resident (~15 MB Rust backend plus the Tauri WebView), thanks to a bare-metal core and no bundled browser engine.
- **🛑 Emergency Failsafes** — <kbd>⌘</kbd><kbd>⇧</kbd><kbd>Q</kbd> on macOS, <kbd>Ctrl</kbd><kbd>⇧</kbd><kbd>Q</kbd> on Windows, checked before every configurable hotkey so it can never be suppressed. An Input Tracking Registry guarantees held keys and buttons are flushed, so a Hold sequence can't leave an input stuck.

## 🧠 Technical Excellence

AutoMux isn't just another autoclicker; it is a meticulously engineered desktop automation tool built to respect system resources.

### The Zero-Polling Guarantee

Traditional macro tools poll the operating system's window manager on an interval (e.g. every 50 ms) to determine the active application. This burns CPU cycles and drains battery life.

AutoMux uses **100% push-based OS events**:

- **macOS:** `NSWorkspaceDidActivateApplicationNotification` via `objc2`, with a `CGEventTap` for input observation.
- **Windows:** `SetWinEventHook` subscribed to `EVENT_SYSTEM_FOREGROUND`, with a low-level keyboard hook.

### The Two-Phase Dispatcher

The OS-level hook is decoupled from the macro executor. A single `Scheduler` task owns every timer and only ever emits an `ActionReady` signal; a single `StateActor` task owns all mutable state, validates targeting, and is the sole place input is ever injected. The main OS event loop is never blocked, so macros stay reliable under heavy system load.

## 🚀 Quick Start

### Installation

1. Head to the [Releases](https://github.com/ABO896/AutoMux/releases) page.
2. Download the installer for your OS — `.dmg` (macOS, universal: Apple Silicon + Intel) or `.exe` (Windows x64).
3. Install and run.

**macOS permissions.** AutoMux needs **Accessibility** (System Settings → Privacy & Security → Accessibility) to inject input and observe the active window; some setups additionally prompt for **Input Monitoring**. The app detects and reports missing permissions in-app rather than failing silently.

**Unsigned builds.** Releases are not notarized and Windows builds are not code-signed, so the first launch shows a warning. On macOS: right-click the app → **Open** → **Open**. On Windows: **More info** → **Run anyway**. If you previously granted Accessibility to an older build, macOS may ask you to re-add AutoMux after an upgrade — the app detects this identity change and tells you.

### Building from Source

Requires [Rust](https://rustup.rs/) (stable), [Node.js](https://nodejs.org/) 24+, and the [Tauri 2 prerequisites](https://tauri.app/start/prerequisites/) for your platform.

```bash
# Clone the repository
git clone https://github.com/ABO896/AutoMux.git
cd AutoMux

# Install frontend dependencies
npm install

# Run the developer instance
npm run tauri dev

# Build the release binaries
npm run tauri build
```

Run the Rust test suite with `cd src-tauri && cargo test`.

## 🏗️ Architecture

| Layer | Responsibility | Location |
|---|---|---|
| UI | SolidJS + Tailwind; renders state, calls commands | `src/` |
| IPC | Translates Tauri commands into `Intent` messages | `src-tauri/src/ipc/` |
| StateActor | Owns `AppState`, gates and performs all input injection | `src-tauri/src/state/` |
| Scheduler | Owns every timer; emits `ActionReady`, never injects | `src-tauri/src/scheduler/` |
| Platform | `CGEvent` / `SendInput` injection and active-app observation | `src-tauri/src/platform/` |
| Persistence | JSON macro profiles | `src-tauri/src/persistence.rs` |

Built with **Tauri 2**, **Rust** (2021 edition), and **SolidJS**.

## 🤝 Contributing

Contributions are welcome — new trigger modes, frontend polish, or OS-native integration work.

1. Fork the project
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a pull request

## 📄 License

Distributed under the GNU General Public License v3.0 only. See [`LICENSE`](LICENSE) for details.
