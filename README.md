# AutoMux ⚡ v1.1.0

**AutoMux** is a high-performance, cross-platform desktop automation engine designed for complex macro management and multi-track sequencing. Engineered to run entirely in the background, AutoMux features sub-millisecond precision and an ultra-low resource footprint.

![AutoMux Dashboard](https://raw.githubusercontent.com/ABO896/AutoMux/main/docs/dashboard.png) *(Placeholder)*

## 🚀 Technical Excellence

- **Zero Polling Architecture**: AutoMux relies purely on system-native push events (`NSWorkspaceDidActivateApplicationNotification` on macOS and `SetWinEventHook` on Windows) for detecting target app changes. This fully eliminates CPU-wasting interval polling loops.
- **$O(1)$ Trigger Lookups**: Hot path macro triggering is powered by constant-time Hash Maps, completely avoiding mutex locks and guaranteeing zero latency injection when hotkeys are pressed.
- **Latched Trigger Modes**: Full support for both "Pulse" (one-shot actions) and "Hold" (sustained) trigger modes, enabling advanced hardware emulation seamlessly across the frontend and backend.
- **Multi-Track Sequences**: Run concurrent sustained holds and interleaved intervals. Perfect for gaming (e.g., Minecraft AFK farms) and productivity.
- **Ultra-Low Footprint**: Consumes strictly **< 60MB RAM** and **0% Idle CPU**. Frontend bundle size is an incredible **56KB**.
- **Sub-ms Jitter Control**: Tokio-driven scheduler ensures extreme timing precision (standard deviation ~1.3ms).

## 📥 Installation

AutoMux natively supports **macOS** and **Windows**. Binary releases are built automatically via GitHub Actions.

1. Go to the [Releases](https://github.com/ABO896/AutoMux/releases) page.
2. Download the installer for your OS:
   - **macOS**: Download the `.dmg` file.
   - **Windows**: Download the `.exe` (NSIS) installer.
3. **Important for macOS**: Upon launch, AutoMux will request Accessibility Permissions. This is required for input injection.

## 🛡 Cybersecurity Audit & Safety Architecture

AutoMux interacts with low-level OS APIs to inject synthetic inputs seamlessly. We designed this layer with user safety and network compliance in mind:

### macOS (`CGEventTap`)
- AutoMux utilizes Apple's native `CoreGraphics` API to post events.
- **Dynamic Initialization**: The engine uses a thread-safe atomic guard to ensure the tap is only active if explicitly granted Accessibility permissions.
- **Memory Safety**: `objc2` pointers are correctly retained and dropped during tap lifecycles, ensuring memory safety across process re-evaluation events.

### Windows (`SendInput`)
- Uses the `Win32` API via `windows-rs` to post synthetic key and mouse events.
- Operates entirely in **User-Mode**. This ensures that AutoMux does not conflict with kernel-level anti-cheat software or strict university/enterprise network endpoint protectors (like CrowdStrike).

### Emergency Stop Safety Guard
Both platforms implement a robust **Input Tracking Registry** (a thread-safe `HashSet` of currently held keys). If the "Emergency Stop" hotkey is invoked or the process unexpectedly exits, a flush routine ensures all virtual keys are released (`KeyUp`/`MouseUp`), preventing the OS from becoming locked in an input loop.

## 🛠 Building from Source

Ensure you have Node.js and Rust installed.

```bash
# Clone the repository
git clone https://github.com/ABO896/AutoMux.git
cd automux

# Install frontend dependencies
npm install

# Run in development mode
npm run dev

# Build the release binaries
npm run build
```

---
*Built with Tauri v2, Rust, SolidJS, and Tailwind CSS.*
