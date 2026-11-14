#!/usr/bin/env bash
# Every check a pull request runs, in the order CI runs them (spec/plan/ai-native-plan.md, step 6).
#
#   scripts/check.sh [all|default|online|plugin|secrets]
#
# `all` (the default) is what "everything" means: every check CI runs on a PR. It refuses to start
# when a tool is missing instead of skipping a target. A single target is allowed and ends by
# saying what it did not run. CI calls one target per job, so this file is where the steps live.
#
# Not here on purpose: `cargo deny` (scheduled, outside PRs, D-108) and the paid live tests.
set -euo pipefail

cd "$(dirname "$0")/.."

# The Claude Code that validates the plugin, pinned: a newer validator reports warnings this one
# does not, and --strict turns them into failures (D-193). Raising it is a decision.
CLAUDE_CODE_VERSION=2.1.292
TARGETS=(default online plugin secrets)

usage() {
    echo "usage: scripts/check.sh [all|default|online|plugin|secrets]" >&2
    exit 2
}

case "${1:-all}" in
    all) selected=("${TARGETS[@]}") ;;
    default | online | plugin | secrets) selected=("$1") ;;
    *) usage ;;
esac
[ "$#" -le 1 ] || usage

full=false
[ "${#selected[@]}" -eq "${#TARGETS[@]}" ] && full=true

# --- tools -------------------------------------------------------------------------------------

missing=()
need() { # tool, what to do about it
    command -v "$1" > /dev/null 2>&1 || missing+=("$1: $2")
}
for target in "${selected[@]}"; do
    case "$target" in
        default | online)
            need cargo "install Rust with rustup (https://rustup.rs)"
            need cargo-nextest "brew install cargo-nextest, or cargo install cargo-nextest --locked"
            ;;
        plugin)
            need npx "install Node.js (npx runs the pinned Claude Code $CLAUDE_CODE_VERSION)"
            ;;
        secrets)
            need gitleaks "brew install gitleaks, or the release pinned in .github/workflows/rust.yml"
            ;;
    esac
done

# Without ripwire the e2e and upstream tests return early and pass having tested nothing
# (`require_ripwire!`). A full run must not count that as a pass.
needs_ripwire=false
for target in "${selected[@]}"; do
    case "$target" in default | online) needs_ripwire=true ;; esac
done
ripwire_missing=false
if $needs_ripwire && ! command -v ripwire > /dev/null 2>&1; then
    ripwire_missing=true
    if $full; then
        missing+=("ripwire is not on PATH; the e2e tests pass empty without it (README.md, Installing ripwire)")
    fi
fi

if [ "${#missing[@]}" -gt 0 ]; then
    echo "check.sh: cannot run '${1:-all}', missing:" >&2
    printf '  %s\n' "${missing[@]}" >&2
    exit 1
fi
if $ripwire_missing; then
    echo "check.sh: warning: ripwire is not on PATH; the e2e and upstream tests will pass without testing anything" >&2
fi

# --- running -----------------------------------------------------------------------------------

step() { # title, command...
    local title=$1 status=0
    shift
    if [ "${GITHUB_ACTIONS:-}" = true ]; then
        echo "::group::$title"
    else
        echo "==> $title"
    fi
    # The group is closed even when the check fails, so its log stays readable; then the failure
    # stops the run (set -e) with the check's own exit status.
    "$@" || status=$?
    if [ "${GITHUB_ACTIONS:-}" = true ]; then
        echo "::endgroup::"
    fi
    return "$status"
}

# The gates below run inside `step`'s `|| status=$?`, where bash ignores set -e for everything
# they call: each failure returns explicitly.
#
# A gate that greps for something that must not be there. grep exits 1 on no match and 2 on an
# error; both read as "nothing found" under `if grep`, so an error is told apart and fails.
must_not_match() { # what (for the error), then grep's arguments
    local what=$1 status=0
    shift
    grep "$@" || status=$?
    case $status in
        0)
            echo "::error::$what" >&2
            return 1
            ;;
        1) return 0 ;;
        *)
            echo "::error::grep failed (exit $status), so this gate proved nothing: $what" >&2
            return 1
            ;;
    esac
}

# CA-10 as a machine-checked gate. Complements `the_build_has_no_network_stack`
# (tests/mcp_surface.rs).
no_network_stack() {
    local tree
    tree=$(cargo tree --locked -e normal) || return
    must_not_match "a network crate reached the default build (CA-10)" \
        -Ei 'reqwest|secrecy|rustls|hyper' <<< "$tree" || return
    echo "no network crate in the default dependency graph"
}

# A recording taken straight from the provider is the likeliest way a secret or real source
# enters the repository. Every fixture is synthetic.
fixtures_stay_synthetic() {
    must_not_match "a fixture above looks like it carries a credential" \
        -rEinl 'Bearer |api_key|authorization|sk-[A-Za-z0-9]{10}' tests/fixtures/ || return
    echo "no credential-shaped string in tests/fixtures/"
}

# Every cargo command takes --locked: Cargo.lock is committed. The two clippy runs are the
# `lint` and `lint-online` aliases in .cargo/config.toml.
run_default() {
    step "Formatting" cargo fmt --all --check
    step "Clippy" cargo lint
    step "Tests" cargo nextest run --locked
    # The property tests again with far more cases than the default 256 (D-109).
    PROPTEST_CASES=4096 step "Property tests, many cases" \
        cargo nextest run --locked -E 'binary(props)'
    # Their case count is set in the file: each case writes files (D-112).
    step "Filesystem property tests" cargo nextest run --locked -E 'binary(props_fs)'
    step "The default build has no network stack (CA-10)" no_network_stack
    step "Fixtures stay synthetic" fixtures_stay_synthetic
}

run_online() {
    step "Clippy (online)" cargo lint-online
    # The #[ignore] tests stay ignored: the three in online_live.rs are paid.
    step "Tests (online)" cargo nextest run --locked --features online
}

# A subshell: the npm setting below stays with these three steps.
run_plugin() (
    claude=(npx -y "@anthropic-ai/claude-code@$CLAUDE_CODE_VERSION")
    # Claude Code installs its native binary in a postinstall script. npm 12 runs install scripts
    # only for packages on its `allow-scripts` list; older npm ignores the setting.
    export npm_config_allow_scripts=@anthropic-ai/claude-code
    step "The plugin" "${claude[@]}" plugin validate --strict integrations/claude-code
    step "The marketplace" "${claude[@]}" plugin validate --strict .
    # The mod's tests through Claude Code's test kit: no session, sign-in or network.
    step "The mod" "${claude[@]}" plugin test integrations/claude-code
)

# The whole history, not the tree: a secret committed and then deleted is still published.
# The allowlist and its reasons are in .gitleaks.toml. --redact keeps a found secret out of the log.
run_secrets() {
    step "Secrets in the history (gitleaks)" \
        gitleaks git --redact --verbose --no-banner --config .gitleaks.toml .
}

for target in "${selected[@]}"; do
    "run_$target"
done

if $full; then
    echo "all PR checks passed"
else
    not_run=()
    for target in "${TARGETS[@]}"; do
        [[ " ${selected[*]} " == *" $target "* ]] || not_run+=("$target")
    done
    $ripwire_missing && not_run+=("the ripwire e2e tests")
    joined=$(printf ', %s' "${not_run[@]}")
    echo "partial coverage: ran ${selected[*]}; did not run ${joined:2}"
fi
