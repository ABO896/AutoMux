<div align="center">
  <h1>⚡ AutoMux</h1>
  <p><strong>The Zero-Footprint Automation Engine</strong></p>

  <p>
    <a href="https://github.com/ABO896/AutoMux/releases"><img src="https://img.shields.io/github/v/release/ABO896/AutoMux?style=flat-square&color=blue" alt="Release"></a>
    <a href="https://github.com/ABO896/AutoMux/actions"><img src="https://img.shields.io/github/actions/workflow/status/ABO896/AutoMux/release.yml?style=flat-square" alt="Build Status"></a>
    <a href="https://github.com/ABO896/AutoMux/blob/master/LICENSE"><img src="https://img.shields.io/github/license/ABO896/AutoMux?style=flat-square" alt="License"></a>
  </p>
  
  <p>A native, cross-platform macro engine architected for <b>$O(1)$ speed</b> and <b>0% idle CPU</b> overhead. Built for power users, developers, and gamers who demand absolute performance.</p>
</div>

<br/>

## ✨ Features

- **🎯 Zero Polling Architecture** — Completely event-driven. The engine sleeps at 0% CPU until the OS pushes an explicit notification. No busy-waiting. No wasted cycles.
- **⚡ $O(1)$ Hotkey Routing** — Trigger resolutions use constant-time `HashMap` lookups, entirely stripping out lock contention on the hot path for sub-millisecond input injection.
- **🔄 Latched Triggering** — Deep support for complex input states including interval-based "Pulse" firing and continuous "Hold" latches.
- **🛡️ Process Detection Parity** — Context-aware macros automatically engage or disengage based on the currently active application window (Native support for both macOS and Windows).
- **🪶 Ultra-Lightweight** — Consumes `<60MB RAM` thanks to a bare-metal Rust core and optimized Tauri frontend.
- **🛑 Emergency Failsafes** — A robust Input Tracking Registry ensures that complex "Hold" sequences are flawlessly flushed to prevent stuck keys during emergency stops.

## 🧠 Technical Excellence

AutoMux isn't just another autoclicker; it is a meticulously engineered desktop automation tool built to respect system resources.

### The Zero-Polling Guarantee
Traditional macro tools poll the operating system's window manager on an interval (e.g., every 50ms) to determine the active application. This burns CPU cycles and drains battery life. 

AutoMux uses **100% push-based OS events**:
- **macOS:** Utilizes `NSWorkspaceDidActivateApplicationNotification` injected via `objc2`.
- **Windows:** Leverages `SetWinEventHook` to subscribe to `EVENT_SYSTEM_FOREGROUND`.

### The Two-Phase Dispatcher
By decoupling the OS-level hook from the actual macro executor (`StateActor`), AutoMux ensures that the main OS event loop is never blocked. Intents are asynchronously routed, enabling reliable macro execution even under heavy system load.

## 🚀 Quick Start

### Installation
1. Head over to the [Releases](https://github.com/ABO896/AutoMux/releases) page.
2. Download the appropriate installer for your OS (`.dmg` for macOS, `.exe` for Windows).
3. Install and run.

*Note for macOS users: AutoMux requires Accessibility permissions to inject keystrokes and monitor active windows.*

### Building from Source

Ensure you have [Rust](https://rustup.rs/) and [Node.js](https://nodejs.org/) installed, along with the [Tauri CLI prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites).

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

## 🤝 Contributing

We welcome contributions! Whether it's adding new Trigger Modes, optimizing the frontend UI, or improving OS-native integrations, feel free to open a PR.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

Distributed under the GNU GPLv3 license. See `LICENSE` for more information.
