# Phase 4: CI Hardening - Context

**Gathered:** 2026-05-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Harden the release CI pipeline on two axes: (1) ensure the macOS build produces a true universal binary (arm64 + x86_64 in a single fat binary), not an arm64-only artifact; (2) pin third-party GitHub Actions to full-length SHAs to prevent supply-chain attacks from floating tag mutation.

**Requirements in scope:** CI-01, CI-02

**Not in scope:** Adding a separate PR/main CI workflow, macOS notarization, signing identity changes, Windows CI changes, dependency audit (`cargo audit`). Do not expand scope.

</domain>

<decisions>
## Implementation Decisions

### Universal Binary Build (CI-01)

- **D-01:** **Add `--target universal-apple-darwin` to tauri-action.** The current workflow installs both `aarch64-apple-darwin` and `x86_64-apple-darwin` Rust targets but does NOT pass the build target to tauri-action, so it builds for the runner's native architecture only (arm64 on current `macos-latest` runners). Fix: add `args: --target universal-apple-darwin` conditionally for the macOS matrix leg.
- **D-02:** **Keep existing matrix structure ([macos-latest, windows-latest]).** No matrix split. Use a conditional expression to pass `--target universal-apple-darwin` only on the macOS leg. Bundle targets stay as-is (no explicit `--bundles` restriction).

### lipo Verification (CI-01 success criterion)

- **D-03:** **Blocking CI step, runs after build and before artifact upload.** Add a `run: lipo -info` step on the macOS leg that finds the built binary inside the tauri output directory and asserts it contains both `arm64` and `x86_64` slices. Workflow fails if the assertion is not met — no arm64-only artifacts can be released silently.

### SHA Pinning (CI-02)

- **D-04:** **Pin third-party actions only** — `dtolnay/rust-toolchain` and `tauri-apps/tauri-action`. GitHub-owned actions (`actions/checkout`, `actions/setup-node`) are trusted and left on their floating tags.
- **D-05:** **SHA + comment annotation.** Format: `uses: tauri-apps/tauri-action@<full-40-char-SHA> # v0`. The tag comment preserves human readability. The planner/executor must look up the current SHA for each pinned tag via the GitHub API before editing the workflow.

### Claude's Discretion

- Exact conditional expression syntax for platform-specific `args` in the tauri-action step (e.g., `${{ matrix.platform == 'macos-latest' && '--target universal-apple-darwin' || '' }}`).
- Exact file path pattern for finding the built `.app` binary to pass to `lipo -info` (depends on tauri-action output structure — look for `src-tauri/target/universal-apple-darwin/release/bundle/` or equivalent).
- Which specific SHAs to resolve at execution time from GitHub API.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — CI-01 and CI-02 definitions (CI & Distribution section); success criteria for this phase
- `.planning/ROADMAP.md` — Phase 4 success criteria (2 acceptance tests: lipo output + SHA replacement)

### CI Workflow (primary artifact)
- `.github/workflows/release.yml` — the ONLY file being modified in this phase; read it in full before planning

### Context
- `.planning/codebase/CONCERNS.md` — documents existing security/tech-debt concerns; read to ensure no overlap with changes

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `.github/workflows/release.yml` — existing matrix strategy (`[macos-latest, windows-latest]`) and `dtolnay/rust-toolchain@stable` targets conditional (`matrix.platform == 'macos-latest' && '...' || ''`) is already the right pattern to extend for platform-conditional build args

### Established Patterns
- Platform conditional in matrix: `${{ matrix.platform == 'macos-latest' && 'value' || '' }}` — already used for `targets` in the rust-toolchain step; use the same pattern for `args` in the tauri-action step
- `GITHUB_TOKEN` is already passed as an environment variable to tauri-action

### Integration Points
- The lipo verification step runs only on the macOS leg (add `if: matrix.platform == 'macos-latest'` condition)
- Tauri universal binary output path: `src-tauri/target/universal-apple-darwin/release/bundle/` — verify this path during execution; the exact binary may be inside a `.app` bundle

</code_context>

<specifics>
## Specific Ideas

- The `lipo -info` check should print the full architecture list and use `grep` or a conditional to assert `x86_64` and `arm64` are both present. A simple pattern: `lipo -info <binary> | grep -E "arm64.*x86_64|x86_64.*arm64"` or `lipo -archs <binary>` then assert both strings appear.
- SHA lookup: `curl -s "https://api.github.com/repos/dtolnay/rust-toolchain/git/refs/tags/stable" | jq -r '.object.sha'` — or equivalent for `tauri-apps/tauri-action@v0`. The executor should do this lookup at execution time, not pin stale cached SHAs.

</specifics>

<deferred>
## Deferred Ideas

- **Pinning `actions/checkout` and `actions/setup-node`** — user chose third-party-only pinning; GitHub-owned actions left on floating tags. Can be revisited in a future hardening pass.
- **Separate PR CI workflow** — no CI runs on PRs currently; adding one is out of Phase 4 scope.
- **`cargo audit` integration** — supply chain check for Rust dependencies; deferred, not in CI-01/CI-02.

</deferred>

---

*Phase: 4-CI Hardening*
*Context gathered: 2026-05-30*
