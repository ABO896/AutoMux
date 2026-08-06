---
id: 260806-l0d
type: quick
status: complete
date: 2026-08-06
commit: ee501f6
tag: v2.0.1
---

# Quick Task 260806-l0d: v2.0.1 release prep + README refresh — Summary

**Status:** complete (release draft pending user publish)

## Trigger

User asked whether the new icon (260806-krh) had reached the GitHub release.
It had not: v2.0 artifacts were built 2026-08-04, the icon landed 2026-08-06,
and 5 commits were still unpushed. Downloaded builds carried the stock Tauri
mark.

## What was done

1. **Version bump 2.0.0 → 2.0.1** — `package.json`, `src-tauri/Cargo.toml`,
   `src-tauri/tauri.conf.json`, `src-tauri/Cargo.lock`. `package-lock.json`
   carries an unrelated `0.1.0` and was left alone.
2. **README full refresh** — icon in the header, Features rewritten against
   shipped v2.0 scope, new architecture table, real requirements, unsigned-build
   Gatekeeper/SmartScreen guidance, `docs/assets/icon-256.png` added.
3. **Pushed + tagged** — `master` pushed (`b3f8ea9..ee501f6`), tag `v2.0.1`
   pushed, `release.yml` run `31104695970` started.

## Corrections made during the work

- README claimed macOS/Windows builds were unsigned. `tauri.conf.json` actually
  sets macOS `signingIdentity: "-"` (ad-hoc), Windows `certificateThumbprint:
  null`. Reworded to "not notarized / not code-signed", which is the accurate
  and user-relevant framing.
- Draft README said "your last profile is restored on launch". `lib.rs` sends
  `Intent::LoadProfile("default")` at startup — a fixed profile, not the last
  used. Reworded before commit.
- Prerequisites link pointed at `tauri.app/v1/...` while the project runs
  Tauri 2. Repointed to `tauri.app/start/prerequisites/`.

## Verification

| Check | Result |
|---|---|
| Version fields consistent | ✓ 4 files at 2.0.1, no stray 2.0.0 |
| Rust test suite | ✓ 25 passed, 0 failed |
| README local paths resolve | ✓ 8/8 |
| README external links | ✓ 5/5 return 200 |
| Feature claims traced to source | ✓ emergency-stop combos, profiles IPC, theming, parallel execution |
| master pushed | ✓ 0 commits ahead of origin |
| Tag pushed, CI triggered | ✓ run 31104695970 |

## Open item

`release.yml` sets `releaseDraft: true`, so v2.0.1 lands as a **draft**. The
user must review and publish it for the icon to reach downloads.
