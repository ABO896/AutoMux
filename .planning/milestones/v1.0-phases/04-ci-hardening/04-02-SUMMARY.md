---
phase: 04-ci-hardening
plan: "02"
subsystem: ci
tags:
  - ci
  - github-actions
  - macos
  - universal-binary
  - tauri
  - lipo

dependency_graph:
  requires:
    - phase: 04-01
      provides: SHA-pinned dtolnay/rust-toolchain and tauri-apps/tauri-action references in release.yml
  provides:
    - Platform-conditional --target universal-apple-darwin arg on tauri-action step
    - Blocking lipo verification step that fails CI if either arm64 or x86_64 slice is absent
  affects:
    - .github/workflows/release.yml

tech-stack:
  added: []
  patterns:
    - "Platform-conditional ternary on args field: ${{ matrix.platform == 'macos-latest' && '<value>' || '' }}"
    - "Step-level if gate: if: matrix.platform == 'macos-latest' for macOS-only steps"
    - "lipo -archs + grep -q blocking assertion pattern for universal binary verification"

key-files:
  created: []
  modified:
    - .github/workflows/release.yml

key-decisions:
  - "args ternary uses the byte-identical expression shape already established on the targets field (line 33) — no step-level if, both platforms run the same tauri-action step"
  - "lipo -archs used over lipo -info: -archs returns a space-separated list (x86_64 arm64) that is reliable for grep -q; -info has prose prefix that risks false positives"
  - "set -e + empty-binary guard added to verify step: catches find returning no results before the misleading lipo error occurs"
  - "Verify step is the final step in the job: tauri-action's artifact upload happens inside the build step; a failing verify step marks the job failed and any drafted release as failed"

patterns-established:
  - "Platform conditional args: use ternary on with.args rather than step-level if — both platforms must run the same tauri-action step"
  - "Architecture verification: lipo -archs | grep -q both slices; chained && ensures non-zero exit on missing slice"

requirements-completed:
  - CI-01

duration: 2min
completed: "2026-05-30"
---

# Phase 04 Plan 02: Universal Binary CI Enforcement Summary

**macOS release leg now builds a fat universal binary (arm64 + x86_64) via --target universal-apple-darwin, with a blocking lipo verification step that fails CI if either architecture slice is absent.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-05-30T16:06:27Z
- **Completed:** 2026-05-30T16:08:33Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Added `args: ${{ matrix.platform == 'macos-latest' && '--target universal-apple-darwin' || '' }}` to the `Build and Release Tauri App` step — macOS leg now tells tauri-action to build a universal binary; Windows leg receives an empty string preserving existing behavior
- Added `Verify universal binary` step gated by `if: matrix.platform == 'macos-latest'` that runs `lipo -archs` against the built binary and fails the workflow if either the `arm64` or `x86_64` slice is missing
- All Plan 01 SHA pins (dtolnay/rust-toolchain, tauri-apps/tauri-action) remain intact — no regressions introduced

## Task Commits

Each task was committed atomically:

1. **Task 1: Add platform-conditional --target universal-apple-darwin arg to tauri-action step** - `8fa0c0e` (feat)
2. **Task 2: Add blocking lipo verification step on the macOS leg after the build** - `c6ed327` (feat)

## Exact Final Form of Added Content

**Task 1 — args field added to tauri-action with block (10-space indent, after prerelease):**

```yaml
          args: ${{ matrix.platform == 'macos-latest' && '--target universal-apple-darwin' || '' }}
```

**Task 2 — Verify universal binary step (final step in job):**

```yaml
      - name: Verify universal binary
        if: matrix.platform == 'macos-latest'
        run: |
          set -e
          BINARY=$(find src-tauri/target/universal-apple-darwin/release/bundle -name "AutoMux" -type f | head -1)
          if [ -z "$BINARY" ]; then
            echo "Error: universal binary not found under src-tauri/target/universal-apple-darwin/release/bundle"
            exit 1
          fi
          echo "Checking binary: $BINARY"
          lipo -archs "$BINARY"
          lipo -archs "$BINARY" | grep -q "x86_64" && lipo -archs "$BINARY" | grep -q "arm64"
```

## Verification Results

All acceptance criteria confirmed against committed release.yml:

- `args:` field present with exact expression: VERIFIED (grep -F confirmed)
- `args:` field located within 12 lines of tauri-action uses line: VERIFIED (awk check)
- Indentation of args field: 10 spaces: VERIFIED (python inspection)
- `name: Verify universal binary` step present: VERIFIED
- Step gated with `if: matrix.platform == 'macos-latest'`: VERIFIED
- `set -e` at top of run block: VERIFIED
- `find src-tauri/target/universal-apple-darwin/release/bundle` path: VERIFIED
- Empty-binary guard with `exit 1`: VERIFIED
- `lipo -archs "$BINARY" | grep -q "x86_64" && lipo -archs "$BINARY" | grep -q "arm64"` blocking assertion: VERIFIED
- Verify step (line 47) is after Build step (line 35): VERIFIED
- Valid YAML (ruby YAML.load_file): VERIFIED
- Plan 01 SHA pins still intact (dtolnay and tauri-action): VERIFIED
- Task 1 args ternary intact after Task 2 edit: VERIFIED

## Files Created/Modified

- `.github/workflows/release.yml` — Added args ternary to tauri-action with block and new Verify universal binary step as final step in the release job

## Decisions Made

- `lipo -archs` used over `lipo -info` — `-archs` returns a space-separated list (`x86_64 arm64`) reliable for `grep -q`; `-info` prepends "Architectures in the fat file:" prose that could produce false positives
- Both platforms run the same tauri-action step with the ternary handling per-platform behavior — no step-level `if:` added to the tauri-action step (consistent with PATTERNS.md rule)
- `set -e` + explicit empty-binary guard: `set -e` catches lipo/find failures; the explicit guard provides a human-readable error when the bundle path layout changes

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

Minor: Several grep/awk verification check patterns from the plan's `<verify>` block produced false negatives due to shell quoting and awk range termination edge cases. The actual file content was verified correct via alternative checks (grep -F, python inspection, ruby YAML parse). These were false negatives in the verification command expressions, not issues with the implementation.

## Live CI Verification (Deferred)

Final CI-01 success criterion requires a live release tag push to confirm the macOS leg's CI log shows `lipo -archs` output containing both `x86_64` and `arm64` and the workflow completes green. This is the documented Phase 4 blocker from STATE.md — it cannot be validated locally and is expected to be verified on the next release tag push post-merge.

## Known Stubs

None.

## Threat Flags

No new security surface introduced. Changes are scoped to CI workflow logic only (build target arg + post-build verification shell commands against workflow's own build output).

## Next Phase Readiness

- CI-01 requirement satisfied at the workflow level; live verification pending next release tag
- Phase 4 has no further plans — phase is complete pending live CI confirmation
- Phase 5 (distribution/signing) can proceed — the universal binary foundation is now in place in CI

---
*Phase: 04-ci-hardening*
*Completed: 2026-05-30*
