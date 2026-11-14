# ripwire-broker — working in this repository

A local MCP server in Rust between a coding agent (Claude Code, Codex) and ripwire: three tools
that return structural context within a token budget, never writing to the workspace. Optional
`--online` (remote classifier) and `--memory` modes; the repository root is also a Claude Code
plugin marketplace. `integrations/codex/AGENTS.md` is product content, what the README's Codex
setup tells a user to add to their own `AGENTS.md`, not an instruction for working here.

## Ask before

Ask the maintainer first, even when the command would work. Each line is an effect, not a flag:
a new flag with the same effect is covered.

- **Publishing:** pushing or creating a `v*` tag, creating or editing a release, `gh pr merge`,
  `git push` to `master`, approving the `release` environment, because a tag runs
  `.github/workflows/release.yml`, users download what it publishes, and the marketplace serves
  `master`.
- **Changing what plugin users run:** any edit under `integrations/claude-code/` (not only
  `integrations/claude-code/scripts/checksums.txt`: also `integrations/claude-code/hooks/register.ts`,
  `integrations/claude-code/scripts/broker`, `integrations/claude-code/.mcp.json`,
  `integrations/claude-code/scripts/install-binary.sh`) or to `.claude-plugin/marketplace.json`,
  because the marketplace serves `master` and it runs with the user's permissions.
- **Spending money or sending content to a provider:** anything that reaches the classifier:
  `--online`, `serve --memory` (it implies online on the server), the plugin's `online`/`memory`
  options, `memory drain --online`, `doctor --jev-probe`, the `--ignored` tests of
  `tests/online_live.rs`, and `ripwire-eval` runs with paid agents, because each call is billed
  and most send workspace content out.
- **Writing outside the repository:** `install … --write` anywhere but a temporary directory,
  because it edits the host's real configuration.
- **Deleting real state:** `memory forget --all` on a state dir that is not a test one, because
  it is irreversible and also revokes collection.
- **Touching the gates:** renaming jobs or editing check steps in `.github/workflows/rust.yml`,
  editing what `scripts/check.sh` or `.cargo/config.toml` runs, branch protection, rulesets,
  environments, `deny.toml`, the gitleaks allowlist (plan step 3), because each one can turn a
  required check green without checking anything.

## Credentials

The session runs as the maintainer, with their GitHub login (keyring, `repo` scope) in reach
([D-192](spec/changelog.md#d-192--credencial-do-agente-risco-aceito): accepted risk, no
isolation). `git push` to `master`, tags, releases, `gh pr merge` and environment approvals are
always the maintainer's, even though the command would succeed. Commit locally; the maintainer
pushes and opens the PR.

## Verify

```sh
scripts/check.sh            # every check a PR runs, in CI's order; refuses if a tool is missing
scripts/check.sh default    # one target (also online, plugin, secrets); ends "partial coverage"
cargo lint                  # the default job's clippy; cargo lint-online for the online build
```

`scripts/check.sh all` needs `cargo`, `cargo-nextest`, `npx`, `gitleaks` and `ripwire` on `PATH`; without
ripwire the e2e tests pass having tested nothing. The `plugin` target runs the Claude Code version
pinned in `scripts/check.sh` through `npx`, not the installed one. Opt-in suites (local model,
live classifier): `README.md` §"Tests".

## Rules

- Every decision gets a D-NNN entry in `spec/changelog.md`, in Portuguese: an index row and a
  section at the end.
- No network crate in the default build (CA-10): `reqwest` and friends stay behind the `online`
  feature, and `scripts/check.sh default` fails otherwise.
- Never rename the CI jobs `default` and `online`: they are the required checks on `master`
  (D-107).
- TDD through the public seam: a red test in `tests/` first, never a test of a private function;
  a mutation proves the test bites.
- Fixtures in `tests/fixtures/` stay synthetic: no credential, no private source; the ripwire
  recordings come from a scratch repository, the hook recordings use placeholders.

Code, comments, commits and PRs are in English; `spec/` is in Portuguese. The repository is
public: nothing from private corpus repositories goes into a commit.

## Where things are

- `handoff.md` §"Como verificar": what CI runs and what the ignored tests need.
- `handoff.md` §"Onde está o quê": the map of `src/`, `tests/`, `spec/`, `integrations/`.
- `handoff.md` §"Armadilhas já pisadas": traps that already cost a wrong conclusion (flaky
  memory tests under load, pipes hiding exit codes, protected `master`).
- `handoff.md` §"Convenções": the conventions above, with the changelog's index order.
- `spec/changelog.md`: every decision and its reason (D-NNN); it wins over any other document.
- `spec/ripwire-broker-mcp.md`: the PRD, with acceptance criteria CA-01 to CA-10.
- `spec/plan/`: one plan per phase; `spec/plan/ai-native-plan.md` is the one in progress.
