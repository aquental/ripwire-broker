# ripwire-broker — Claude Code plugin

Budgeted, task-oriented code context for Claude Code, from the local
[ripwire-broker](../../README.md) MCP server over [ripwire](https://github.com/redhat-et/ripwire).
The plugin ships, in one versioned package: the MCP server (three read-only tools), the hooks that
inject context on the first prompt, after edits and before finishing, the `ripwire-broker` skill,
and the consent options for the online mode and memory.

Tested with Claude Code 2.1.292, on macOS and Linux (the broker is Unix only).

## Install

```sh
claude plugin marketplace add aquental/ripwire-broker
claude plugin install ripwire-broker@aquental
```

The plugin does not contain the broker binary (it is Rust, built per platform). Install the release
the plugin pins, once, and again after an update that pins a new one, with the plugin's
`scripts/install-binary.sh`. The first session after installing prints the exact command, with the
path of the installed copy; in the layout observed with Claude Code 2.1.285 (not documented) it is

```sh
sh ~/.claude/plugins/cache/aquental/ripwire-broker/<version>/scripts/install-binary.sh
```

The script downloads the asset for your machine from the GitHub release named in
`scripts/checksums.txt`, checks it against the SHA-256 pinned there, and only then installs it into
the plugin's data directory, `~/.claude/plugins/data/ripwire-broker-aquental/bin/<version>/`.
A mismatch installs nothing. `--prune` removes the versions the plugin no longer pins. Nothing is
downloaded unless you run it: at the start of each session the plugin only **checks**, and when the
pinned binary or `ripwire` is missing it says so in one line, which Claude passes on to you.

Alternatives: put a `ripwire-broker` on `PATH`
(`cargo install --git https://github.com/aquental/ripwire-broker --tag v0.1.0 --features online`), or
point the `binary` option at one. Either runs whatever its version, with a warning on stderr when
it is not the one the plugin pins.

`ripwire` itself (≥ 0.6.4) is not part of the plugin: it must be on `PATH`.

## Options

Set them in the dialog Claude Code shows when the plugin is enabled, or with
`claude plugin configure ripwire-broker@aquental`. `claude plugin install` itself never asks.

| Option | Default | What it does |
|---|---|---|
| `online` | off | Starts the server with `--online`: previews and eligible excerpts of the workspace go to the Jev provider. Its description in the dialog is the same consent text `ripwire-broker install --online` prints. |
| `memory` | off | Starts the server with `--memory` (implies online) and the hooks with `--memory`: observations of this workspace are kept locally between sessions, and the eligible ones sent to Jev. The hooks only write locally; they never make HTTP requests. |
| `jev_api_key` | empty | The Jev key, kept in the platform's secure credential store, never in `settings.json`. Empty: the server uses `RIPWIRE_BROKER_JEV_API_KEY` from the environment Claude Code starts from. |
| `incremental` | **on** | The server leaves out, per session, what it already delivered. The hooks dedup on their own; this covers what the agent asks the server directly. |
| `every_prompt` | off | The prompt hook injects context on every prompt, not only the first of the session. |
| `gate` | off | The `Stop` hook holds the turn once when the final check asks for attention. |
| `binary` | empty | A `ripwire-broker` to run instead of the pinned release or the one on `PATH`. |

## Names

The server is `broker` inside the plugin, so Claude Code calls it `plugin:ripwire-broker:broker`
(`/mcp` shows it that way) and its tools are:

- `mcp__plugin_ripwire-broker_broker__context_for_task`
- `mcp__plugin_ripwire-broker_broker__context_after_edit`
- `mcp__plugin_ripwire-broker_broker__context_before_finish`

A project `.mcp.json` written by `ripwire-broker install` registers the server as `ripwire-broker`,
with tools named `mcp__ripwire-broker__<tool>`. Permission rules, `allowedTools` and hook matchers
written for those names do not match the plugin's: write them for the names above, or use
`mcp__plugin_ripwire-broker_broker__*`.

Use the plugin **or** `ripwire-broker install claude-code`, not both: with both, the workspace has
two servers and two sets of hooks. To move to the plugin, remove the `ripwire-broker` entry from
the project's `.mcp.json` and the broker's hooks from `.claude/settings.json`.

## What runs, and where

- **Server:** `scripts/broker serve`, which picks the binary (the `binary` option, then the pinned
  release, then `PATH`), turns the options into flags, and serves the project directory
  (`CLAUDE_PROJECT_DIR`, or the directory Claude Code started it in).
- **Hooks:** `UserPromptSubmit`, `PostToolUse` (on `Edit|Write|MultiEdit|NotebookEdit|Bash`) and
  `Stop` run `scripts/broker hook claude-code <event>` in exec form, with a 60 s timeout. They follow
  each event's `cwd`; a missing binary makes them pass silently rather than block the session.
  `SessionStart` runs `scripts/broker check` (10 s): one line when something is missing, nothing
  otherwise.
- **State:** where the broker always keeps it, `$XDG_STATE_HOME/ripwire-broker`. The plugin writes
  only the binary, into its data directory, and only when you run `install-binary.sh`.
- **Status line:** a plugin cannot set Claude Code's `statusLine`. Use
  `ripwire-broker install claude-code --workspace DIR --statusline` for it.

## The mod

The plugin is also a [mod](https://code.claude.com/docs/en/plugins/mods/overview): `hooks/hooks.json`
names a hooks module, `hooks/register.ts`, beside the classic hooks. Where mods load, the module
answers the same three moments through the MCP server the session already has connected, instead of
starting a process per event; where they do not load (an older Claude Code, `--bare`,
`--safe-mode`, `disableAllHooks`), the classic hooks run as before. Never both: on `session.start`
the mod sets `RIPWIRE_BROKER_MOD_ACTIVE=1`, and `scripts/broker` then leaves every classic hook
silent.

Tested with Claude Code 2.1.292; the documentation asks for 2.1.287 or later. The mods API can
change between versions without notice.

What it does more than the classic hooks:

- **Memory through the hooks.** With the `memory` option, the server's `context_for_task` reads the
  memories of the workspace, and the mod's first-prompt context carries them; the classic hooks
  only collect.
- **No process per event.** One MCP call per moment, on the connection the session already has.
- **The band above the prompt**, on the terminal and in the Desktop app:
  `rw-brkr · hooks on · última: atenção · inj 2 · [jev:3] · [mem: retr 1, stor 2] · (online)`,
  the classic status line's labels. **`/ripwire-status`** prints the same line anywhere.

And less:

- `ripwire-broker hook-stats` and `hook-log` count only sessions run by the classic hooks, and the
  classic `statusline` shows `hooks sem dados` in a session with the mod.
- The band has no `não reenviados`: the mod does not know what the server left out.

Why `incremental` is on by default: the server leaves out, per session, what it already delivered.
The classic hooks kept their own record of what they had injected; the mod keeps none, so the
server's is the one that holds. Turning it off makes the mod's answers repeat items and risks it has
already given.

Before you install it, this is everything the module hooks and calls, as `claude plugin validate`
reads it from the source (it runs with your permissions, like any code you install):

```text
hooks: session.start, prompt.submit, tool.call{tool=Edit|Write|MultiEdit|NotebookEdit|Bash}, turn.complete, ui.render{component=AbovePrompt}, command.run{command=ripwire-status}
calls: $.clock.now, $.command.register, $.env.get (via serverView), $.env.set, $.fs.list (via serverView), $.fs.read (via serverView), $.fs.stat (via serverView, snapshot), $.mcp.call (via ask), $.mcp.connect (via ask), $.process.run (via snapshot), $.prompt.submit, $.session.cwd, $.state.get, $.state.set, $.ui.log, $.ui.resolve
env writes: RIPWIRE_BROKER_MOD_ACTIVE
env reads: HOME, XDG_STATE_HOME
```

`$.process.run` is `git rev-parse --show-toplevel` and `git status --porcelain` around a Bash
command, to tell what it changed (only until Claude Code reports that itself); `$.fs` reads the
server's status files under the broker's state directory and stats the dirty files of the work tree;
`$.prompt.submit` is the finish gate's one extra turn, with the `gate` option only.

Its tests run without a session, sign-in or network:
`claude plugin test integrations/claude-code`.

## Publishing a version

The order is fixed, and `tests/plugin.rs` holds it:

1. Raise `version` in `Cargo.toml`; run the gates.
2. Tag `vX.Y.Z` on `master`. The release workflow (`.github/workflows/release.yml`) runs the
   gates of `rust.yml`, then **waits for an approval** of the `release` environment (Actions, the
   run, "Review deployments") before it writes anything. That holds once the maintainer has set
   the environment's required reviewer on GitHub; until then GitHub creates it unprotected and
   nothing waits. Approved, it refuses a tag that is not
   the `Cargo.toml` version, builds
   `ripwire-broker-vX.Y.Z-<target>.tar.gz` (with `ripwire-broker` and `ripwire-eval`, built with
   `--features online`) for `aarch64-apple-darwin`, `x86_64-apple-darwin`,
   `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, and publishes them with
   `SHA256SUMS`. The release stays a draft until every target is up; the job summary of `publish`
   has the lines for step 3.
3. Write `vX.Y.Z` on the first line of `scripts/checksums.txt` and one `sha256  asset` line per
   target; set `.claude-plugin/plugin.json` `version` to `X.Y.Z` (the tests require both to agree,
   and the plugin never to be ahead of `Cargo.toml`). On the PR, the `default` job downloads the
   release's `SHA256SUMS` and fails unless the lines are the same and the release is published
   (`scripts/plugin-surface.sh`); any PR that touches this directory or `.claude-plugin/` also gets
   a warning in its job summary. Both are a check next to the review, not a replacement.
4. `claude plugin validate --strict integrations/claude-code` and `claude plugin validate --strict .`;
   commit. Users get it with `claude plugin update ripwire-broker@aquental`, then run
   `install-binary.sh` again (the previous release is never run: only the pinned version counts).

## Rolling back a version

A bad release goes back the way it came: through the pins. Nothing here deletes a release.

**The maintainer**, in a PR:

1. Put the previous release back in `scripts/checksums.txt`: its tag on the first line and the
   lines of its `SHA256SUMS`. Set `.claude-plugin/plugin.json` `version` to the same version. The
   tests require both to agree, and the `default` job checks the lines against the release's
   `SHA256SUMS` (`scripts/plugin-surface.sh`). `Cargo.toml` stays where it is: the code may be
   ahead of the plugin, never behind it.
2. Mark the bad release: `gh release edit vX.Y.Z --prerelease`. It is a signal to people; nothing
   refuses a prerelease (the pins and `install-binary.sh` go by tag), so the review of the next
   PR that pins a version is what keeps it out.
3. Merge. The marketplace serves `master`, so users get the previous version from the next update.

**The user:**

1. `claude plugin update ripwire-broker@aquental`. It fetches the marketplace itself.
2. If you set the `binary` option, clear the `binary` option (or point it at the pinned release):
   `/plugin configure ripwire-broker@aquental`. While it is set, it wins over everything below.
3. `sh install-binary.sh` again, from the plugin's `scripts/`. If that version is still installed
   from before, it runs at once and this step only checks it again. If it is not (an earlier
   `--prune` removed it), the plugin runs a `ripwire-broker` found on `PATH`, with a warning, and
   otherwise refuses: the server and the hooks say "the binary for version X.Y.Z is not
   installed" with the command to run, and the session start says the same in one line.
4. Restart the session: the server and the hooks pick up the plugin's new version only then.

**What does not go back on its own.** The plugin runs, in this order, the `binary` option, the
pinned release under the plugin's data directory, and `ripwire-broker` on `PATH`. Someone who set
the `binary` option, or who has no pinned release installed and an old `ripwire-broker` on `PATH`,
keeps running that binary after the rollback. The plugin says so on stderr ("… is version 0.2.0;
the plugin pins 0.1.0. Running it anyway.").

To see what runs, ask the plugin's own resolver for the version. From a terminal, give it the
plugin's data directory, which Claude Code only exports to hooks and the server:

```sh
CLAUDE_PLUGIN_DATA="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/plugins/data/ripwire-broker-aquental" \
  sh "<the plugin's directory>/scripts/broker" --version
```

This check does not see the `binary` option either, which Claude Code also passes only to the
server and the hooks, and the terminal's `PATH` may not be Claude Code's. If you set the option,
that is the binary that runs; add `CLAUDE_PLUGIN_OPTION_BINARY=<its value>` to the command to see
what the plugin makes of it. `ripwire-broker doctor` does not say which `ripwire-broker` the plugin
chose. `sh install-binary.sh --prune` removes the other installed versions.

**A memory store the older binary cannot read.** A store written by a newer schema is refused by
an older binary: `memory status` prints a line of zeros, then `store unavailable: unknown_schema`,
and nothing in the store is changed. The server is meant to keep giving the structural context,
with the memory reported as limited (`docs/jev-mem-prd.md` §12). `memory forget --all` does not get
out of it: it revokes collection and drops the pending observations, then fails on the same
schema, and the snapshot stays. What works:

- run a binary that knows the schema (roll forward instead of back); or
- stop the server and the hooks (close the sessions), then move the workspace's store,
  `<state-dir>/memory/<workspace id>/`, out of the state dir as a backup. The next run starts an
  empty store, with collection on. The backup keeps the text of every memory: delete it when it is
  no longer needed.

Only schema 1 exists today, and every write records it: this is for the first schema change.
