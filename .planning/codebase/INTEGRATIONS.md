# External Integrations

**Analysis Date:** 2026-05-15

## APIs & External Services

**None at runtime.** AutoMux is a fully offline desktop application. There are no HTTP API calls, cloud service SDKs, or third-party SaaS integrations in the application code.

**Build-time only:**
- `https://fonts.googleapis.com` and `https://fonts.gstatic.com` — referenced in the Tauri CSP (`src-tauri/tauri.conf.json`), allowing Google Fonts to load in the webview. No API key required.
- `http://timestamp.digicert.com` — Windows code-signing timestamp server, used only during production bundling (`src-tauri/tauri.conf.json` `bundle.windows.timestampUrl`). Not used at runtime.

## Data Storage

**Databases:**
- None. No database engine is used.

**File Storage (local):**
- Platform-specific app data directory, managed by Tauri's path API (`src-tauri/src/persistence.rs`):
  - macOS: `~/Library/Application Support/com.alvaro.automux/profiles/`
  - Windows: `%APPDATA%/com.alvaro.automux/profiles/`
- Each saved profile is a JSON file (`<profile-name>.json`) containing macro configurations.
- All file I/O is async via `tokio::fs`.
- Client: `serde_json` for serialization; no ORM.

**Caching:**
- None.

## Authentication & Identity

**Auth Provider:**
- None. No user accounts, login flow, or session management.

**OS-Level Permission (macOS only):**
- macOS Accessibility API (`AXIsProcessTrusted` / `AXIsProcessTrustedWithOptions`) — required for CGEvent tap input injection.
- Checked via `src-tauri/src/platform/macos/mod.rs` → exposed to frontend via `check_accessibility` and `request_accessibility` IPC commands (`src-tauri/src/ipc/mod.rs`).
- Frontend polls this every 10 seconds and prompts the user if not granted (`src/App.tsx`).

## Monitoring & Observability

**Error Tracking:**
- None. No Sentry, Datadog, or similar integration.

**Logs:**
- `eprintln!` statements guarded by `#[cfg(debug_assertions)]` only — debug builds log to stderr. No structured logging framework. No production log output.

## CI/CD & Deployment

**Hosting:**
- GitHub Releases — binaries are published as release assets automatically by CI.

**CI Pipeline:**
- GitHub Actions (`release.yml` at `.github/workflows/release.yml`)
- Trigger: push of a `v*` tag
- Matrix: `macos-latest` and `windows-latest`
- macOS builds both `aarch64-apple-darwin` (Apple Silicon) and `x86_64-apple-darwin` (Intel) targets
- Uses `tauri-apps/tauri-action@v0` to build and publish the release draft
- Secret required: `GITHUB_TOKEN` (standard GitHub Actions secret, no extra setup)

## Environment Configuration

**Required env vars (runtime):**
- None. The application has no runtime environment variable dependencies.

**Optional env vars (build/dev):**
- `TAURI_DEV_HOST` — enables remote HMR in `vite.config.ts` during development; not required for local dev.
- `GITHUB_TOKEN` — CI only, injected automatically by GitHub Actions.

**Secrets location:**
- No application secrets. No `.env` file detected.

## Webhooks & Callbacks

**Incoming:**
- None.

**Outgoing:**
- None.

## IPC (Internal — Frontend ↔ Rust)

While not an external integration, the Tauri IPC bridge is the primary communication channel and warrants documentation:

**Invoke commands** (frontend → Rust, defined in `src-tauri/src/ipc/mod.rs`, registered in `src-tauri/src/lib.rs`):
- `get_state` — returns full `AppState`
- `get_active_app` — returns the currently focused OS application
- `add_macro` / `remove_macro` / `set_macro_enabled` / `set_macro_target_app`
- `set_macro_sequence` / `update_step_interval`
- `toggle_engine`
- `request_accessibility` / `check_accessibility`
- `bind_hotkey` / `unbind_hotkey`
- `save_profile` / `load_profile` / `delete_profile` / `list_profiles`

**Events** (Rust → frontend, emitted by the state actor):
- `state-changed` — payload: full `AppState`; subscribed in `src/App.tsx` via `listen()`

**CSP** (configured in `src-tauri/tauri.conf.json`):
- `connect-src ipc: http://ipc.localhost` — allows Tauri IPC
- `style-src 'self' 'unsafe-inline' https://fonts.googleapis.com`
- `font-src https://fonts.gstatic.com`

---

*Integration audit: 2026-05-15*
