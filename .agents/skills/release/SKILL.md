---
name: release
description: >-
  Prepare a release of ripwire-broker and of its Claude Code plugin, in the order of
  "Publishing a version" in the plugin's README: the version bump and the gates before the tag,
  then, with the maintainer's yes, the plugin's pins from the published release and the PR text.
  Use when asked to release, publish, cut a version, or move the plugin to a new release. It
  prepares; the tag, the approval, the merge and the edit of what plugin users run are the
  maintainer's.
---

# Release

The order is fixed and `tests/plugin.rs` holds it. This session has the maintainer's login
(D-192), so every step that publishes, or that changes what plugin users run, is a stop, even
though the command would work.

## Before the tag

1. On a branch, raise `version` in `Cargo.toml` to `X.Y.Z`, then `cargo build` to update
   `Cargo.lock` for this crate. The code may be ahead of the plugin, never behind it.
2. Run `scripts/check.sh all`. It must end with "all PR checks passed".
3. Record the release as a decision (the `decision-entry` skill) and commit.
4. Hand over the tag (below) and wait for the release to be published.

## After the release is published

1. Check the release is published, not a draft, without writing anything:
   `gh release view vX.Y.Z --repo aquental/ripwire-broker --json isDraft --jq .isDraft` prints
   `false`.
2. Read its `SHA256SUMS`:
   `gh release download vX.Y.Z --repo aquental/ripwire-broker --pattern SHA256SUMS --output -`.
3. Prepare the plugin's pins (below: they need the maintainer's yes) and the PR text: the version,
   what changes for plugin users, and the `SHA256SUMS` lines it pins.

## Where it stops

Each of these is in the list of `AGENTS.md` "Ask before" (publishing, what plugin users run, the
gates). The skill does not run them on its own: it shows the maintainer the exact command or
change and waits.

- **The tag:** `git tag vX.Y.Z` and `git push origin vX.Y.Z` start `release.yml`. The maintainer's.
- **The approval of the `release` environment** in the Actions run ("Review deployments"). The
  maintainer's.
- **The plugin's pins.** Writing `vX.Y.Z` and the `SHA256SUMS` lines in
  `integrations/claude-code/scripts/checksums.txt`, and `X.Y.Z` as `version` in
  `integrations/claude-code/.claude-plugin/plugin.json`, changes what plugin users run once it is
  merged. Ask first; with a yes, write them, run `scripts/check.sh all` (the `default` job will
  check the pins against the release in the PR, `scripts/plugin-surface.sh`), and commit.
- **The PR and the merge:** `gh pr create`, `gh pr merge`, any `git push`. The maintainer's.
- **Editing a release:** `gh release create` or `gh release edit`, including marking a bad one as
  a prerelease when rolling back ("Rolling back a version" in the plugin's README). The
  maintainer's.
