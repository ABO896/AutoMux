---
plan: 02-04
phase: 02-cleanup-credibility
status: complete
commit: c9d81ba
---

## Summary

Replaced hardcoded `v1.0.0` footer with runtime `getVersion()` and confirmed polling-interval comment accuracy.

## Exact Diff Applied

**src/App.tsx — 4 targeted changes:**

1. Added import: `import { getVersion } from "@tauri-apps/api/app";` (after listen import)
2. Added signal: `const [appVersion, setAppVersion] = createSignal<string>("…");`
3. Extended Promise.all to include `getVersion()` as 5th awaitable; destructured as `version`; added `setAppVersion(version)` after the `if (cancelled) return;` guard
4. Replaced `<span …>v1.0.0</span>` with `<span …>v{appVersion()}</span>`

**Polling comment:** Already accurate — comment says "every 3s" and `setInterval` value is `3000`. No change needed.

## Acceptance Criteria

| Check | Result |
|-------|--------|
| `import { getVersion }` count = 1 | ✓ 1 |
| `createSignal<string>` for appVersion = 1 | ✓ 1 |
| `getVersion()` call count ≥ 1 | ✓ 1 (inside Promise.all) |
| `setAppVersion(version)` count = 1 | ✓ 1 |
| `v{appVersion()}` count = 1 | ✓ 1 |
| `>v1.0.0<` count = 0 | ✓ 0 |
| `setAppVersion(version)` after `if (cancelled) return;` | ✓ confirmed by reading |
| No new `createEffect(` added | ✓ count unchanged |
| Polling comment matches setInterval(3000) → "3s" | ✓ already aligned |
| `npx tsc --noEmit` exit 0 | ✓ |
| `npm run build` exit 0 | ✓ built in 385ms |

## Self-Check: PASSED
