---
name: release
description: >-
  Prepare a release of ripwire-broker and of the Claude Code plugin, following "Publishing a
  version" in integrations/claude-code/README.md: the version bump and the gates before the tag,
  then the pins of the published release in a PR. Use when asked to release, publish, cut a
  version, or update the plugin to a new release. It prepares; the tag, the approval and the merge
  are the maintainer's.
---

# Release

The order is fixed and `tests/plugin.rs` holds it. This session has the maintainer's login
(D-192), so every step that publishes is a stop, even though the command would work.

## Before the tag

1. On a branch, raise `version` in `Cargo.toml` to `X.Y.Z` and let `cargo build --locked` fail,
   then `cargo build` to update `Cargo.lock` for this crate only. The code may be ahead of the
   plugin, never behind it.
2. Run `scripts/check.sh all`. It must end with "all PR checks passed".
3. Record the release as a decision (the `decision-entry` skill) and commit.
4. Stop: the PR, the merge and the tag are the maintainer's (below).

## After the release is published

1. Read the release's `SHA256SUMS` without writing anything:
   `gh release download vX.Y.Z --repo aquental/ripwire-broker --pattern SHA256SUMS --output -`.
   It must be published, not a draft: `gh release view vX.Y.Z --json isDraft --jq .isDraft`
   prints `false`.
2. Write `vX.Y.Z` on the first line of `integrations/claude-code/scripts/checksums.txt` and the
   `SHA256SUMS` lines after it; set `version` in `integrations/claude-code/.claude-plugin/plugin.json`
   to `X.Y.Z`. Both live under `integrations/claude-code/`, which the maintainer has approved by
   asking for the release.
3. Run `scripts/check.sh all`. On the PR, the `default` job checks the pins against the release
   (`scripts/plugin-surface.sh`).
4. Write the PR text: the version, what changes for plugin users, and the `SHA256SUMS` it pins.
   Commit. Stop.

## Where it stops

Each of these is in `AGENTS.md` "Ask before" (publishing, the gates, what plugin users run). The
skill hands them to the maintainer, with the exact command, and does not run them:

- **the tag:** `git tag vX.Y.Z` and `git push origin vX.Y.Z` start `release.yml`;
- **the approval of the `release` environment** in the Actions run ("Review deployments");
- **the merge:** opening the PR, `gh pr merge`, any `git push` to `master`;
- **editing a release:** `gh release create` or `gh release edit`, including marking a bad one as
  a prerelease when rolling back ("Rolling back a version" in the plugin README).
