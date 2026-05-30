# Phase 4: CI Hardening - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-30
**Phase:** 4-ci-hardening
**Areas discussed:** Universal binary fix, SHA pinning scope, lipo verification in CI

---

## Universal Binary Fix

| Option | Description | Selected |
|--------|-------------|----------|
| Add --target universal-apple-darwin | Add `args: --target universal-apple-darwin` to tauri-action. Standard Tauri approach — installed targets already correct, only the build flag is missing. | ✓ |
| Separate matrix jobs | Split into two macOS jobs (arm64 + x86_64), build each, lipo-merge. More explicit but much more complex. | |

**User's choice:** Add `--target universal-apple-darwin`
**Notes:** Bundle targets left as-is (no explicit `--bundles` restriction — "whatever works best"). Matrix structure kept as-is — user deferred to Claude for implementation detail.

---

## SHA Pinning Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Pin all 4 | actions/checkout, actions/setup-node, dtolnay/rust-toolchain, tauri-apps/tauri-action | |
| Third-party only | Pin only dtolnay/rust-toolchain and tauri-apps/tauri-action. GitHub-owned actions left on floating tags. | ✓ |

**User's choice:** Third-party only (dtolnay + tauri-apps)
**Notes:** SHA + comment annotation chosen (e.g. `# v0`) to preserve human readability.

---

## lipo Verification in CI

| Option | Description | Selected |
|--------|-------------|----------|
| CI step (blocking) | Fails the workflow if artifact is not arm64+x86_64. Automated on every release. | ✓ |
| One-time manual check | Run lipo manually on first post-fix release artifact. | |
| CI step (non-blocking) | Prints output but doesn't fail the build. | |

**User's choice:** CI step (blocking)
**Notes:** Runs after build, before artifact upload. Runs only on macOS leg.

---

## Claude's Discretion

- Bundle format for the tauri-action step (defaults used — no restriction)
- Matrix structure (keep existing `[macos-latest, windows-latest]`)
- Exact conditional expression syntax for platform-specific args
- Exact file path to the built binary inside tauri output directory for `lipo -info`
- SHA resolution via GitHub API at execution time

## Deferred Ideas

- Pinning `actions/checkout` and `actions/setup-node` — user chose third-party-only; can be revisited in future hardening
- Separate PR CI workflow — out of Phase 4 scope
- `cargo audit` integration — not in CI-01/CI-02
