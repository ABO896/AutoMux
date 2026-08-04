---
task: "06-02 Task 1"
built_at: "2026-06-04T21:20Z"
---

# DMG Build Record — Plan 06-02 Task 1

## Build Output

- **DMG:** `src-tauri/target/release/bundle/dmg/AutoMux_1.2.0_aarch64.dmg`
- **Size:** 3.5 MB
- **Architecture:** aarch64 (Apple Silicon)
- **Build command:** `npm run tauri build -- --bundles dmg`
- **Exit code:** 0

## Verification Checks

1. Build exits 0 with no errors — PASS
2. DMG file exists at reported path — PASS (3.5M, dated 2026-06-04)
3. Bundled Info.plist passes `plutil -lint` — PASS (OK)
4. `NSAccessibilityUsageDescription` present in bundled Info.plist — PASS
   - Value: "AutoMux requires Accessibility access to monitor and inject keyboard and mouse events for macro automation."

## Notes

- No signing errors (expected — signingIdentity is null; unsigned build)
- Build includes Plan 06-01 fixes: CGEventTap live probe (403c8ba) and Info.plist (9656597)
- DMG ready for manual installation on macOS 26 Tahoe device for COMPAT verification
