# Phase 4: CI Hardening - Pattern Map

**Mapped:** 2026-05-30
**Files analyzed:** 1
**Analogs found:** 1 / 1

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `.github/workflows/release.yml` | CI/release automation | sequential-steps | `.github/workflows/release.yml` (self — extend existing patterns within the file) | self |

## Pattern Assignments

### `.github/workflows/release.yml` (CI release automation, sequential-steps)

**Analog:** `.github/workflows/release.yml` — the file being modified; all new patterns must extend, not break, the existing structure.

---

#### Platform conditional expression pattern (line 33 — existing, extend this)

This is the canonical pattern already in use for `targets`. Apply the identical expression shape for the `args` field in the tauri-action step.

```yaml
# Existing use — rust-toolchain targets (line 33):
targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}

# New use — tauri-action args (same expression shape):
args: ${{ matrix.platform == 'macos-latest' && '--target universal-apple-darwin' || '' }}
```

Rule: the ternary evaluates the truthy branch on macOS and yields an empty string on Windows, which tauri-action ignores. Copy the expression shape exactly — do not use `if:` at the step level for this because both platforms must use the same tauri-action step.

---

#### `GITHUB_TOKEN` env injection pattern (lines 37-38 — existing, do not change)

```yaml
env:
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

This pattern is already correct and must be preserved as-is on the tauri-action step.

---

#### Platform-gated step pattern (new — for lipo verification)

Use a step-level `if:` condition to run the verification only on the macOS leg. No analog exists in the current file (it has no conditional steps), but the matrix variable used is the same one already established.

```yaml
- name: Verify universal binary
  if: matrix.platform == 'macos-latest'
  run: |
    BINARY=$(find src-tauri/target/universal-apple-darwin/release/bundle -name "AutoMux" -type f | head -1)
    echo "Checking binary: $BINARY"
    lipo -archs "$BINARY"
    lipo -archs "$BINARY" | grep -q "x86_64" && lipo -archs "$BINARY" | grep -q "arm64"
```

Key constraints:
- Step must come AFTER the `Build and Release Tauri App` step and BEFORE any artifact upload step.
- The `find` path is `src-tauri/target/universal-apple-darwin/release/bundle/` — executor must verify the exact binary name inside that tree at execution time.
- The assertion must cause a non-zero exit (workflow failure) if either slice is absent. Using `grep -q` with `&&` achieves this — if either grep fails, the step exits non-zero and the job fails.
- Do NOT use `lipo -info` for the assertion check; `lipo -archs` returns a space-separated list suitable for `grep`.

---

#### SHA pinning pattern (new — for dtolnay/rust-toolchain and tauri-apps/tauri-action)

The two third-party actions currently use floating tags:

```yaml
# Current (floating tags — lines 31, 36):
uses: dtolnay/rust-toolchain@stable
uses: tauri-apps/tauri-action@v0
```

Replace with full 40-character SHAs and a trailing comment preserving the human-readable tag:

```yaml
# Pinned pattern (executor must resolve SHAs at execution time):
uses: dtolnay/rust-toolchain@<SHA>  # stable
uses: tauri-apps/tauri-action@<SHA>  # v0
```

SHA resolution commands (executor runs these, not hardcoded here):
```bash
# dtolnay/rust-toolchain — resolve tag "stable"
curl -s "https://api.github.com/repos/dtolnay/rust-toolchain/git/refs/tags/stable" \
  | jq -r '.object.sha'

# tauri-apps/tauri-action — resolve tag "v0"
curl -s "https://api.github.com/repos/tauri-apps/tauri-action/git/refs/tags/v0" \
  | jq -r '.object.sha'
```

If the tag ref is a tag object (not a commit), the GitHub API returns the tag object SHA. The executor should then dereference to the commit SHA with:
```bash
curl -s "https://api.github.com/repos/<owner>/<repo>/git/tags/<sha>" \
  | jq -r '.object.sha'
```

---

#### Full annotated post-modification structure (reference shape)

The final `release.yml` steps section should follow this ordering:

```yaml
steps:
  - name: Checkout repository
    uses: actions/checkout@v4          # GitHub-owned, left on floating tag

  - name: Setup Node.js
    uses: actions/setup-node@v4        # GitHub-owned, left on floating tag
    with:
      node-version: 24

  - name: Install Frontend Dependencies
    run: npm install

  - name: Setup Rust
    uses: dtolnay/rust-toolchain@<SHA>  # stable
    with:
      targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}

  - name: Build and Release Tauri App
    uses: tauri-apps/tauri-action@<SHA>  # v0
    env:
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    with:
      tagName: ${{ github.ref_name }}
      releaseName: "AutoMux ${{ github.ref_name }}"
      releaseBody: "Automated Release from CI"
      releaseDraft: true
      prerelease: false
      args: ${{ matrix.platform == 'macos-latest' && '--target universal-apple-darwin' || '' }}

  - name: Verify universal binary
    if: matrix.platform == 'macos-latest'
    run: |
      BINARY=$(find src-tauri/target/universal-apple-darwin/release/bundle -name "AutoMux" -type f | head -1)
      echo "Checking binary: $BINARY"
      lipo -archs "$BINARY"
      lipo -archs "$BINARY" | grep -q "x86_64" && lipo -archs "$BINARY" | grep -q "arm64"
```

---

## Shared Patterns

### Platform conditional expression
**Source:** `.github/workflows/release.yml` line 33
**Apply to:** `args` field on tauri-action step (new) — identical ternary shape.
```yaml
${{ matrix.platform == 'macos-latest' && '<macos-value>' || '' }}
```

### Step-level platform gate
**Source:** No existing example in the file — use standard GitHub Actions syntax.
**Apply to:** `Verify universal binary` step only.
```yaml
if: matrix.platform == 'macos-latest'
```

---

## No Analog Found

No files in this phase lack an analog. The single modified file is its own analog; all new patterns are extensions of patterns already present in the file or are standard GitHub Actions syntax with no project-specific precedent needed.

---

## Metadata

**Analog search scope:** `.github/workflows/`
**Files scanned:** 1 (`release.yml`)
**Pattern extraction date:** 2026-05-30
