---
phase: 04-ci-hardening
reviewed: 2026-05-30T00:00:00Z
depth: standard
files_reviewed: 1
files_reviewed_list:
  - .github/workflows/release.yml
findings:
  critical: 2
  warning: 3
  info: 0
  total: 5
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-05-30
**Depth:** standard
**Files Reviewed:** 1
**Status:** issues_found

## Summary

The workflow implements CI-01 (universal binary build + lipo verification) and CI-02 (SHA pinning for third-party actions) as specified. The sha-pinning decisions for `dtolnay/rust-toolchain` and `tauri-apps/tauri-action` are correctly implemented; the deliberate non-pinning of `actions/checkout` and `actions/setup-node` is per design decision D-04 and is not flagged here.

Two critical correctness issues were found: the lipo verification step runs after `tauri-action` has already uploaded artifacts to the GitHub release draft, making it incapable of blocking a bad release from being published; and `npm install` is used instead of `npm ci`, breaking build reproducibility on the dependency resolution axis. Three additional warnings cover fragile binary discovery logic, floating runner labels, and a hardcoded release body.

## Critical Issues

### CR-01: lipo verification runs after artifact upload — cannot block a bad release

**File:** `.github/workflows/release.yml:47`
**Issue:** The `tauri-action` step (line 35) both builds the app AND creates the GitHub release draft AND uploads the built artifacts in a single atomic operation. The "Verify universal binary" step at line 47 runs only after all of this has completed. If the build produced an arm64-only binary (the exact regression CI-01 was designed to prevent), the bad artifact is already attached to the draft release before the lipo check runs. A failing lipo check cannot retract an already-uploaded artifact; it only fails the workflow run after the fact.

This means CI-01's success criterion — "workflow fails if the assertion is not met, no arm64-only artifacts can be released silently" — is not actually enforced. A broken universal build would still produce a draft release with the wrong artifact attached, and a human would have to notice the failed check and manually delete the bad artifact.

**Fix:** The verification must run before the artifact-uploading step. The cleanest way to achieve this within tauri-action's constraints is to split build and upload into two steps:

1. Run `tauri-action` without the `tagName`/`releaseName`/`releaseDraft` parameters — this builds artifacts only, does not create a release.
2. Run the lipo verification step.
3. Run a separate upload step (e.g., `tauri-apps/tauri-action` with upload-only arguments, or `softprops/action-gh-release`) only after the lipo check passes.

Alternatively, if the tauri-action version supports a `skipArtifactUpload` or `dryRun` option, use that for the build step, verify, then re-run for upload. Check the pinned SHA's supported inputs for this option.

```yaml
# Step 1: Build only (no release creation)
- name: Build Tauri App
  uses: tauri-apps/tauri-action@84b9d35b5fc46c1e45415bdb6144030364f7ebc5  # v0
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  with:
    args: ${{ matrix.platform == 'macos-latest' && '--target universal-apple-darwin' || '' }}
    # omit tagName/releaseName/releaseDraft to suppress upload

# Step 2: Verify (blocks upload)
- name: Verify universal binary
  if: matrix.platform == 'macos-latest'
  run: |
    set -e
    BINARY=$(find src-tauri/target/universal-apple-darwin/release/bundle \
      -path "*/MacOS/AutoMux" -type f | head -1)
    if [ -z "$BINARY" ]; then
      echo "Error: universal binary not found"
      exit 1
    fi
    ARCHS=$(lipo -archs "$BINARY")
    echo "Architectures: $ARCHS"
    echo "$ARCHS" | grep -q "x86_64" || { echo "Missing x86_64"; exit 1; }
    echo "$ARCHS" | grep -q "arm64"  || { echo "Missing arm64";  exit 1; }

# Step 3: Create release and upload only after verification passes
- name: Create GitHub Release
  uses: tauri-apps/tauri-action@84b9d35b5fc46c1e45415bdb6144030364f7ebc5  # v0
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  with:
    tagName: ${{ github.ref_name }}
    releaseName: "AutoMux ${{ github.ref_name }}"
    releaseBody: "Automated Release from CI"
    releaseDraft: true
    prerelease: false
    args: ${{ matrix.platform == 'macos-latest' && '--target universal-apple-darwin' || '' }}
```

---

### CR-02: `npm install` used instead of `npm ci` — non-reproducible dependency resolution

**File:** `.github/workflows/release.yml:28`
**Issue:** `npm install` is used in the CI pipeline. Unlike `npm ci`, `npm install` does NOT enforce that the installed dependency tree matches `package-lock.json` exactly. It can silently resolve semver ranges to newer minor/patch versions than what was locked, or modify `package-lock.json` in place. This means release builds may install different frontend dependency versions than what was tested, violating reproducibility. A supply-chain attack on any transitive dependency published between developer testing and the release build tag push would be silently adopted.

`npm ci` is the correct tool for CI: it performs a clean install from the lockfile, fails with a non-zero exit if `package-lock.json` is absent or out of sync with `package.json`, and never modifies the lockfile.

**Fix:**
```yaml
- name: Install Frontend Dependencies
  run: npm ci
```

---

## Warnings

### WR-01: `find | head -1` searches too broadly and is non-deterministic

**File:** `.github/workflows/release.yml:51`
**Issue:** The binary discovery command `find src-tauri/target/universal-apple-darwin/release/bundle -name "AutoMux" -type f | head -1` has two problems:

1. `-name "AutoMux"` matches any regular file named `AutoMux` anywhere under the bundle directory, including intermediate objects, debug artifacts, or files placed by the bundler in unexpected locations. The actual macOS app executable lives at a specific path: `AutoMux.app/Contents/MacOS/AutoMux`.
2. `head -1` takes the first result from `find`, whose traversal order is filesystem-dependent (not alphabetical, not guaranteed). If multiple matches exist, a different file is silently selected on each run.

If an intermediate build artifact happens to be found first and happens to contain the right architecture (e.g., it is a single-arch object file that `lipo` can parse), the check would pass while the actual `.app` bundle contains the wrong binary.

**Fix:** Target the known canonical path for the app bundle executable:
```bash
BINARY=$(find src-tauri/target/universal-apple-darwin/release/bundle \
  -path "*/Contents/MacOS/AutoMux" -type f | head -1)
```
This constrains the match to the actual app bundle executable location, making the check deterministic.

---

### WR-02: `lipo -archs` called three times on the same binary

**File:** `.github/workflows/release.yml:57-58`
**Issue:** The verify step calls `lipo -archs "$BINARY"` three times: once for display (line 57), and twice on line 58 (once per `grep -q`). Each call reads the binary from disk. The pattern on line 58 also means that if for any reason the binary disappears or changes between calls (e.g., a race with another build process in a multi-job runner scenario), the three invocations may not be consistent.

More practically, the repeated invocations make the intent harder to read and create unnecessary I/O.

**Fix:** Capture the output once, then grep against the variable:
```bash
ARCHS=$(lipo -archs "$BINARY")
echo "Architectures: $ARCHS"
echo "$ARCHS" | grep -q "x86_64" || { echo "Error: x86_64 slice missing"; exit 1; }
echo "$ARCHS" | grep -q "arm64"  || { echo "Error: arm64 slice missing";  exit 1; }
```
This also improves the error message: the current `&&` chain exits with a non-zero status but prints nothing, making failure diagnosis harder in CI logs.

---

### WR-03: Runner labels are floating — builds are not reproducible across runner fleet updates

**File:** `.github/workflows/release.yml:15`
**Issue:** `macos-latest` and `windows-latest` are mutable labels. GitHub silently redirects them to a new OS image when they update the fleet (e.g., `macos-latest` moved from macOS 13 → 14 → 15 in succession). This can change the Xcode version, macOS SDK version, available system libraries, and default compiler toolchain under a release without any change to the workflow file or any explicit human decision.

For a release pipeline, where artifacts need to be reproducible across tags (e.g., rebuilding an old tag to patch a critical bug), this is a robustness issue: two runs against the same tag at different points in time may produce binaries built with different SDKs.

**Fix:** Pin to a specific runner version:
```yaml
platform: [macos-14, windows-2022]
```
When the team decides to adopt a newer runner image, it becomes an explicit, reviewable commit rather than a silent background change. Update the list intentionally when upgrading.

---

_Reviewed: 2026-05-30_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
