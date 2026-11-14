//! `scripts/check.sh`, the one command that runs every check a PR runs (ai-native plan, step 6).
//!
//! The script is driven with fake tools on a sealed `PATH`: each fake records its arguments and
//! succeeds unless told to fail. That tests the script's own logic (which checks, in which order,
//! what a missing tool does, what a partial run says) without running cargo inside cargo. The
//! real tools are exercised by CI, which calls the same script.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/check.sh");
const ALL_TOOLS: &[&str] = &["cargo", "cargo-nextest", "npx", "ripwire"];

/// A sealed directory of tools: the fakes asked for plus the few host utilities the script uses.
struct Sandbox {
    dir: tempfile::TempDir,
    env: Vec<(String, String)>,
}

impl Sandbox {
    fn with(fakes: &[&str]) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = dir.path().join("bin");
        fs::create_dir(&bin).expect("bin");
        for tool in ["grep", "sed", "dirname"] {
            symlink(host_tool(tool), bin.join(tool)).expect("symlink");
        }
        for fake in fakes {
            let path = bin.join(fake);
            fs::write(&path, FAKE).expect("fake");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        Sandbox {
            dir,
            env: Vec::new(),
        }
    }

    fn all() -> Self {
        Self::with(ALL_TOOLS)
    }

    fn without(tool: &str) -> Self {
        let fakes: Vec<&str> = ALL_TOOLS.iter().copied().filter(|t| *t != tool).collect();
        Self::with(&fakes)
    }

    fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    fn log(&self) -> PathBuf {
        self.dir.path().join("calls.log")
    }

    fn run(&self, args: &[&str]) -> Run {
        let mut cmd = Command::new(host_tool("bash"));
        cmd.arg(SCRIPT)
            .args(args)
            .env_clear()
            .env("PATH", self.dir.path().join("bin"))
            .env("HOME", self.dir.path())
            .env("FAKE_LOG", self.log());
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        let output = cmd.output().expect("run check.sh");
        let calls = fs::read_to_string(self.log()).unwrap_or_default();
        Run { output, calls }
    }
}

/// Records `name args` (and `PROPTEST_CASES` and npm's `allow-scripts` when set) and exits 0,
/// unless `FAKE_FAIL` is a substring of the call. `cargo tree` prints `FAKE_TREE`.
const FAKE: &str = r#"#!/bin/bash
call="${0##*/} $*"
printf '%s%s%s\n' "$call" "${PROPTEST_CASES:+ [PROPTEST_CASES=$PROPTEST_CASES]}" \
    "${npm_config_allow_scripts:+ [allow-scripts=$npm_config_allow_scripts]}" >> "$FAKE_LOG"
if [ -n "${FAKE_FAIL:-}" ] && [[ "$call" == *"$FAKE_FAIL"* ]]; then
    echo "fake failure in: $call" >&2
    exit 1
fi
if [ "$call" = "cargo tree --locked -e normal" ]; then
    printf '%s\n' "${FAKE_TREE:-ripwire-broker v0.1.0}"
fi
exit 0
"#;

struct Run {
    output: Output,
    calls: String,
}

impl Run {
    fn ok(&self) -> bool {
        self.output.status.success()
    }
    fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).into_owned()
    }
    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }
    fn both(&self) -> String {
        format!("{}{}", self.stdout(), self.stderr())
    }
    fn calls(&self) -> Vec<&str> {
        self.calls.lines().collect()
    }
}

fn host_tool(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH");
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("{name} not on the host PATH"))
}

const DEFAULT_CALLS: &[&str] = &[
    "cargo fmt --all --check",
    "cargo lint",
    "cargo nextest run --locked",
    "cargo nextest run --locked -E binary(props) [PROPTEST_CASES=4096]",
    "cargo nextest run --locked -E binary(props_fs)",
    "cargo tree --locked -e normal",
];
const ONLINE_CALLS: &[&str] = &[
    "cargo lint-online",
    "cargo nextest run --locked --features online",
];
const PLUGIN_CALLS: &[&str] = &[
    "npx -y @anthropic-ai/claude-code@2.1.292 plugin validate --strict integrations/claude-code [allow-scripts=@anthropic-ai/claude-code]",
    "npx -y @anthropic-ai/claude-code@2.1.292 plugin validate --strict . [allow-scripts=@anthropic-ai/claude-code]",
    "npx -y @anthropic-ai/claude-code@2.1.292 plugin test integrations/claude-code [allow-scripts=@anthropic-ai/claude-code]",
];

#[test]
fn all_runs_every_ci_check_in_the_ci_order_and_says_so() {
    let run = Sandbox::all().run(&["all"]);
    assert!(run.ok(), "{}", run.both());
    let expected: Vec<&str> = [DEFAULT_CALLS, ONLINE_CALLS, PLUGIN_CALLS].concat();
    assert_eq!(run.calls(), expected);
    let last = run.stdout().lines().last().unwrap_or_default().to_owned();
    assert_eq!(last, "all PR checks passed");
}

#[test]
fn no_target_means_all() {
    let run = Sandbox::all().run(&[]);
    assert!(run.ok(), "{}", run.both());
    assert!(
        run.stdout().ends_with("all PR checks passed\n"),
        "{}",
        run.stdout()
    );
}

#[test]
fn each_target_runs_only_its_own_checks_and_reports_partial_coverage() {
    for (target, calls, rest) in [
        ("default", DEFAULT_CALLS, "online, plugin"),
        ("online", ONLINE_CALLS, "default, plugin"),
        ("plugin", PLUGIN_CALLS, "default, online"),
    ] {
        let run = Sandbox::all().run(&[target]);
        assert!(run.ok(), "{target}: {}", run.both());
        assert_eq!(run.calls(), calls, "{target}");
        let last = run.stdout().lines().last().unwrap_or_default().to_owned();
        assert_eq!(
            last,
            format!("partial coverage: ran {target}; did not run {rest}")
        );
        assert!(!run.stdout().contains("all PR checks passed"), "{target}");
    }
}

#[test]
fn a_failing_check_stops_the_run_with_a_nonzero_exit() {
    let run = Sandbox::all()
        .env("FAKE_FAIL", "nextest run --locked")
        .run(&["all"]);
    assert!(!run.ok());
    assert_eq!(
        run.calls().last().copied(),
        Some("cargo nextest run --locked")
    );
    assert!(!run.stdout().contains("passed"), "{}", run.stdout());
}

#[test]
fn a_network_crate_in_the_default_graph_fails_ca10() {
    let run = Sandbox::all()
        .env("FAKE_TREE", "ripwire-broker v0.1.0\n└── reqwest v0.12.0")
        .run(&["default"]);
    assert!(!run.ok());
    assert!(run.both().contains("CA-10"), "{}", run.both());
}

#[test]
fn the_grep_gates_fail_closed_when_grep_itself_errors() {
    // grep exits 2 on an error and 1 on no match; `if grep` reads both as "nothing found", which
    // would turn a broken guard into a passing one.
    let sandbox = Sandbox::all();
    let grep = sandbox.dir.path().join("bin/grep");
    fs::remove_file(&grep).expect("rm grep");
    fs::write(&grep, "#!/bin/bash\necho 'grep: broken' >&2\nexit 2\n").expect("fake grep");
    fs::set_permissions(&grep, fs::Permissions::from_mode(0o755)).expect("chmod");
    let run = sandbox.run(&["default"]);
    assert!(!run.ok(), "{}", run.both());
    assert!(run.both().contains("CA-10"), "{}", run.both());
}

#[test]
fn all_fails_up_front_naming_what_to_install_when_a_tool_is_missing() {
    for (tool, hint) in [
        ("cargo-nextest", "cargo-nextest"),
        ("npx", "Node.js"),
        ("cargo", "rustup"),
    ] {
        let run = Sandbox::without(tool).run(&["all"]);
        assert!(!run.ok(), "{tool}");
        assert!(run.stderr().contains(hint), "{tool}: {}", run.stderr());
        assert!(
            run.calls.is_empty(),
            "{tool}: ran before failing: {}",
            run.calls
        );
    }
}

#[test]
fn default_passes_without_npx_and_says_coverage_is_partial() {
    let run = Sandbox::without("npx").run(&["default"]);
    assert!(run.ok(), "{}", run.both());
    assert!(
        run.stdout().contains("partial coverage"),
        "{}",
        run.stdout()
    );
}

#[test]
fn all_fails_without_ripwire_because_the_e2e_tests_would_pass_empty() {
    let run = Sandbox::without("ripwire").run(&["all"]);
    assert!(!run.ok());
    assert!(
        run.stderr().contains("ripwire is not on PATH"),
        "{}",
        run.stderr()
    );
    assert!(run.calls.is_empty(), "{}", run.calls);
}

#[test]
fn a_single_target_without_ripwire_warns_and_counts_it_as_not_run() {
    let run = Sandbox::without("ripwire").run(&["default"]);
    assert!(run.ok(), "{}", run.both());
    assert!(
        run.stderr().contains("ripwire is not on PATH"),
        "{}",
        run.stderr()
    );
    let last = run.stdout().lines().last().unwrap_or_default().to_owned();
    assert!(last.contains("the ripwire e2e tests"), "{last}");
}

#[test]
fn an_unknown_target_is_a_usage_error() {
    let run = Sandbox::all().run(&["everything"]);
    assert_eq!(run.output.status.code(), Some(2), "{}", run.both());
    assert!(run.stderr().contains("usage"), "{}", run.stderr());
    assert!(run.calls.is_empty());
}

#[test]
fn under_github_actions_each_check_is_its_own_log_group() {
    let run = Sandbox::all()
        .env("GITHUB_ACTIONS", "true")
        .run(&["default"]);
    assert!(run.ok(), "{}", run.both());
    let out = run.stdout();
    let groups = out.matches("::group::").count();
    assert_eq!(
        groups,
        DEFAULT_CALLS.len() + 1,
        "fmt..CA-10 plus the fixture guard:\n{out}"
    );
    assert_eq!(groups, out.matches("::endgroup::").count());
}

#[test]
fn ci_jobs_call_the_script_under_their_required_names() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/rust.yml");
    let yml = fs::read_to_string(&path).expect("rust.yml");
    for job in ["default", "online", "plugin"] {
        assert!(
            yml.contains(&format!("  {job}:\n    name: {job}\n")),
            "job {job} renamed"
        );
        assert!(
            yml.contains(&format!("run: scripts/check.sh {job}\n")),
            "job {job} does not call the script"
        );
    }
}

#[test]
fn cargo_aliases_hold_the_two_clippy_runs_and_nothing_else() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".cargo/config.toml");
    let toml = fs::read_to_string(&path).expect(".cargo/config.toml");
    let tables: Vec<&str> = toml.lines().filter(|l| l.starts_with('[')).collect();
    assert_eq!(tables, ["[alias]"]);
    assert!(toml.contains(r#"lint = "clippy --all-targets --locked -- -D warnings""#));
    assert!(toml.contains(
        r#"lint-online = "clippy --all-targets --locked --features online -- -D warnings""#
    ));
}
