//! `scripts/check.sh`, the one command that runs every check a PR runs (ai-native plan, step 6).
//!
//! The script is copied into a sandbox tree and driven with fake tools on a sealed `PATH`: each
//! fake records its arguments and succeeds unless told to fail. That tests the script's own logic
//! (which checks, in which order, what a missing tool does, what a partial run says) without
//! running cargo inside cargo. Every scenario runs under each `bash` on the machine (macOS ships
//! 3.2 in /bin; CI has 5) and must behave the same. The real tools are exercised by CI, which
//! calls the same script.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/check.sh");
const ALL_TOOLS: &[&str] = &[
    "cargo",
    "cargo-nextest",
    "npx",
    "ripwire",
    "gitleaks",
    "git",
];

/// A sealed directory of tools (the fakes asked for plus the few host utilities the script uses)
/// and a tree holding a copy of the script and a clean `tests/fixtures/`.
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
            write_executable(&bin.join(fake), FAKE);
        }
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join("scripts")).expect("scripts");
        fs::copy(SCRIPT, repo.join("scripts/check.sh")).expect("copy check.sh");
        fs::create_dir_all(repo.join("tests/fixtures")).expect("fixtures");
        fs::write(
            repo.join("tests/fixtures/clean.json"),
            "{\"tool\": \"explore\"}\n",
        )
        .expect("fixture");
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

    fn fixture(self, name: &str, body: &str) -> Self {
        let path = self.dir.path().join("repo/tests/fixtures").join(name);
        fs::write(path, body).expect("fixture");
        self
    }

    /// `grep` errors (exit 2) when its arguments contain `marker`, and is the real one otherwise.
    fn grep_errors_on(self, marker: &str) -> Self {
        let grep = self.dir.path().join("bin/grep");
        fs::remove_file(&grep).expect("rm grep");
        let body = format!(
            "#!/bin/bash\ncase \" $* \" in *'{marker}'*) echo 'grep: broken' >&2; exit 2 ;; esac\nexec '{}' \"$@\"\n",
            host_tool("grep").display()
        );
        write_executable(&grep, &body);
        self
    }

    fn log(&self) -> PathBuf {
        self.dir.path().join("calls.log")
    }

    /// Runs the script under every bash on the machine and insists they agree.
    fn run(&self, args: &[&str]) -> Run {
        let mut runs = shells()
            .into_iter()
            .map(|shell| self.run_with(&shell, args));
        let first = runs.next().expect("a bash");
        for other in runs {
            assert_eq!(other, first, "bash versions disagree on {args:?}");
        }
        first
    }

    fn run_with(&self, shell: &Path, args: &[&str]) -> Run {
        let _ = fs::remove_file(self.log());
        let mut cmd = Command::new(shell);
        cmd.arg(self.dir.path().join("repo/scripts/check.sh"))
            .args(args)
            .env_clear()
            .env("PATH", self.dir.path().join("bin"))
            .env("HOME", self.dir.path())
            .env("FAKE_LOG", self.log());
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        let output = cmd.output().expect("run check.sh");
        Run {
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            calls: fs::read_to_string(self.log()).unwrap_or_default(),
        }
    }
}

/// Records `name args` (and `PROPTEST_CASES` and npm's `allow-scripts` when set) and exits 0,
/// unless `FAKE_FAIL` is a substring of the call. `cargo tree` prints `FAKE_TREE`; `git
/// rev-parse` answers `FAKE_INSIDE` and `FAKE_SHALLOW`; `gitleaks` logs `FAKE_GITLEAKS_LOG` to
/// stderr, as the real one does (default: a clean scan of three commits).
const FAKE: &str = r#"#!/bin/bash
call="${0##*/} $*"
printf '%s%s%s\n' "$call" "${PROPTEST_CASES:+ [PROPTEST_CASES=$PROPTEST_CASES]}" \
    "${npm_config_allow_scripts:+ [allow-scripts=$npm_config_allow_scripts]}" >> "$FAKE_LOG"
if [ -n "${FAKE_FAIL:-}" ] && [[ "$call" == *"$FAKE_FAIL"* ]]; then
    echo "fake failure in: $call" >&2
    exit 1
fi
case "$call" in
    "cargo tree --locked -e normal") printf '%s\n' "${FAKE_TREE:-ripwire-broker v0.1.0}" ;;
    "git rev-parse --is-inside-work-tree") echo "${FAKE_INSIDE:-true}" ;;
    "git rev-parse --is-shallow-repository") echo "${FAKE_SHALLOW:-false}" ;;
    "gitleaks git "*)
        printf '%b\n' "${FAKE_GITLEAKS_LOG:-5:24PM INF 3 commits scanned.\n5:24PM INF no leaks found}" >&2
        ;;
esac
exit 0
"#;

#[derive(Debug, PartialEq)]
struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    calls: String,
}

impl Run {
    fn ok(&self) -> bool {
        self.code == Some(0)
    }
    fn both(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
    fn calls(&self) -> Vec<&str> {
        self.calls.lines().collect()
    }
    fn last_line(&self) -> &str {
        self.stdout.lines().last().unwrap_or_default()
    }
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).expect("write");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
}

fn is_executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

fn host_tool(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH");
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| is_executable(p))
        .unwrap_or_else(|| panic!("{name} not on the host PATH"))
}

/// The first bash on `PATH`, plus the system one when it is a different file (macOS: 3.2).
fn shells() -> Vec<PathBuf> {
    let mut shells = vec![host_tool("bash")];
    let system = PathBuf::from("/bin/bash");
    let same = |a: &Path, b: &Path| fs::canonicalize(a).ok() == fs::canonicalize(b).ok();
    if is_executable(&system) && !same(&shells[0], &system) {
        shells.push(system);
    }
    shells
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

const SECRETS_CALLS: &[&str] = &[
    "git rev-parse --is-inside-work-tree",
    "git rev-parse --is-shallow-repository",
    // remerge: a merge's own changes (a conflict resolution, a file only the merge adds) are part
    // of the history too, and plain `git log -p` shows no diff for a merge.
    "gitleaks git --redact --verbose --no-banner --config .gitleaks.toml \
     --gitleaks-ignore-path .gitleaksignore --log-opts=--all --full-history --diff-merges=remerge .",
];

fn every_call() -> Vec<&'static str> {
    [DEFAULT_CALLS, ONLINE_CALLS, PLUGIN_CALLS, SECRETS_CALLS].concat()
}

#[test]
fn all_runs_every_ci_check_in_the_ci_order_and_says_so() {
    let run = Sandbox::all().run(&["all"]);
    assert!(run.ok(), "{}", run.both());
    assert_eq!(run.calls(), every_call());
    assert_eq!(run.last_line(), "all PR checks passed");
}

#[test]
fn no_target_means_all() {
    let run = Sandbox::all().run(&[]);
    assert!(run.ok(), "{}", run.both());
    assert_eq!(run.calls(), every_call());
    assert_eq!(run.last_line(), "all PR checks passed");
}

#[test]
fn each_target_runs_only_its_own_checks_and_reports_partial_coverage() {
    for (target, calls, rest) in [
        ("default", DEFAULT_CALLS, "online, plugin, secrets"),
        ("online", ONLINE_CALLS, "default, plugin, secrets"),
        ("plugin", PLUGIN_CALLS, "default, online, secrets"),
        ("secrets", SECRETS_CALLS, "default, online, plugin"),
    ] {
        let run = Sandbox::all().run(&[target]);
        assert!(run.ok(), "{target}: {}", run.both());
        assert_eq!(run.calls(), calls, "{target}");
        assert_eq!(
            run.last_line(),
            format!("partial coverage: ran {target}; did not run {rest}")
        );
        assert!(!run.stdout.contains("all PR checks passed"), "{target}");
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
    assert!(!run.stdout.contains("passed"), "{}", run.stdout);
}

#[test]
fn a_network_crate_in_the_default_graph_fails_ca10() {
    let run = Sandbox::all()
        .env("FAKE_TREE", "ripwire-broker v0.1.0\n└── reqwest v0.12.0")
        .run(&["default"]);
    assert!(!run.ok());
    assert!(
        run.stderr
            .contains("::error::a network crate reached the default build (CA-10)"),
        "{}",
        run.both()
    );
}

#[test]
fn a_failing_cargo_tree_fails_ca10_instead_of_reading_an_empty_graph() {
    let run = Sandbox::all()
        .env("FAKE_FAIL", "cargo tree")
        .run(&["default"]);
    assert!(!run.ok(), "{}", run.both());
    assert!(!run.stdout.contains("no network crate"), "{}", run.stdout);
    assert_eq!(
        run.calls().last().copied(),
        Some("cargo tree --locked -e normal")
    );
}

#[test]
fn a_credential_shaped_fixture_fails_the_fixture_guard() {
    let run = Sandbox::all()
        .fixture("recorded.json", "{\"api_key\": \"synthetic\"}\n")
        .run(&["default"]);
    assert!(!run.ok(), "{}", run.both());
    assert!(
        run.stderr
            .contains("::error::a fixture above looks like it carries a credential"),
        "{}",
        run.both()
    );
    assert!(run.stdout.contains("recorded.json"), "{}", run.stdout);
}

// grep exits 2 on an error and 1 on no match; `if grep` reads both as "nothing found", which
// would turn a broken guard into a passing one. Each gate is broken on its own.
#[test]
fn the_ca10_gate_fails_closed_when_grep_errors() {
    let run = Sandbox::all().grep_errors_on("-Ei").run(&["default"]);
    assert!(!run.ok(), "{}", run.both());
    assert!(
        run.stderr.contains(
            "grep failed (exit 2), so this gate proved nothing: a network crate reached the default build"
        ),
        "{}",
        run.both()
    );
}

#[test]
fn the_fixture_gate_fails_closed_when_grep_errors() {
    let run = Sandbox::all()
        .grep_errors_on("tests/fixtures/")
        .run(&["default"]);
    assert!(!run.ok(), "{}", run.both());
    assert!(
        run.stderr.contains(
            "grep failed (exit 2), so this gate proved nothing: a fixture above looks like it carries a credential"
        ),
        "{}",
        run.both()
    );
    assert_eq!(run.calls(), DEFAULT_CALLS, "every earlier check ran");
}

#[test]
fn all_fails_up_front_naming_what_to_install_when_a_tool_is_missing() {
    for (tool, hint) in [
        ("cargo-nextest", "cargo-nextest"),
        ("npx", "Node.js"),
        ("cargo", "rustup"),
        ("gitleaks", "brew install gitleaks"),
        ("git", "install git"),
    ] {
        let run = Sandbox::without(tool).run(&["all"]);
        assert!(!run.ok(), "{tool}");
        assert!(run.stderr.contains(hint), "{tool}: {}", run.stderr);
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
    assert_eq!(run.calls(), DEFAULT_CALLS);
    assert_eq!(
        run.last_line(),
        "partial coverage: ran default; did not run online, plugin, secrets"
    );
}

#[test]
fn secrets_needs_only_gitleaks_and_no_ripwire() {
    let run = Sandbox::with(&["gitleaks", "git"]).run(&["secrets"]);
    assert!(run.ok(), "{}", run.both());
    assert_eq!(run.calls(), SECRETS_CALLS);
    assert!(!run.stderr.contains("ripwire"), "{}", run.stderr);
    assert_eq!(
        run.last_line(),
        "partial coverage: ran secrets; did not run default, online, plugin"
    );
}

#[test]
fn a_failing_plugin_check_stops_the_run_before_the_next_target() {
    // run_plugin is a subshell (its npm setting stays inside); a failure in any of its steps must
    // still stop `all`.
    for (failing, call) in [
        ("validate --strict integrations", PLUGIN_CALLS[0]),
        ("validate --strict .", PLUGIN_CALLS[1]),
        ("plugin test", PLUGIN_CALLS[2]),
    ] {
        let run = Sandbox::all().env("FAKE_FAIL", failing).run(&["all"]);
        assert!(!run.ok(), "{failing}: {}", run.both());
        assert_eq!(run.calls().last().copied(), Some(call), "{failing}");
        assert!(!run.stdout.contains("passed"), "{failing}: {}", run.stdout);
    }
}

#[test]
fn secrets_refuses_outside_a_git_work_tree() {
    // gitleaks 8.30 logs `not a git repository` and exits 0 there.
    let run = Sandbox::all().env("FAKE_INSIDE", "false").run(&["secrets"]);
    assert!(!run.ok(), "{}", run.both());
    assert!(run.both().contains("not a git work tree"), "{}", run.both());
    assert!(!run.calls.contains("gitleaks"), "{}", run.calls);
}

#[test]
fn secrets_refuses_a_shallow_clone() {
    // A shallow clone holds part of the history: a secret added and removed before it would pass.
    let run = Sandbox::all().env("FAKE_SHALLOW", "true").run(&["secrets"]);
    assert!(!run.ok(), "{}", run.both());
    assert!(
        run.both().contains("git fetch --unshallow"),
        "{}",
        run.both()
    );
    assert!(!run.calls.contains("gitleaks"), "{}", run.calls);
}

#[test]
fn gitleaks_logging_an_error_fails_even_when_it_exits_zero() {
    // What gitleaks 8.30.1 prints, exit 0, when git writes anything to stderr.
    let log = "5:24PM ERR [git] warning: synthetic\n5:24PM ERR error=\"stderr is not empty\"\n\
               5:24PM INF 373 commits scanned.\n5:24PM INF no leaks found";
    let run = Sandbox::all()
        .env("FAKE_GITLEAKS_LOG", log)
        .run(&["secrets"]);
    assert!(!run.ok(), "{}", run.both());
    assert!(
        run.both().contains("::error::gitleaks logged an error"),
        "{}",
        run.both()
    );
}

#[test]
fn gitleaks_scanning_no_commit_fails() {
    let log = "5:24PM INF 0 commits scanned.\n5:24PM INF no leaks found";
    let run = Sandbox::all()
        .env("FAKE_GITLEAKS_LOG", log)
        .run(&["secrets"]);
    assert!(!run.ok(), "{}", run.both());
    assert!(run.both().contains("scanned no commit"), "{}", run.both());
}

#[test]
fn a_leak_found_by_gitleaks_fails_the_run() {
    let run = Sandbox::all()
        .env("FAKE_FAIL", "gitleaks git")
        .run(&["all"]);
    assert!(!run.ok(), "{}", run.both());
    assert_eq!(run.calls().last().copied(), Some(SECRETS_CALLS[2]));
    assert!(!run.stdout.contains("passed"), "{}", run.stdout);
}

#[test]
fn all_fails_without_ripwire_because_the_e2e_tests_would_pass_empty() {
    let run = Sandbox::without("ripwire").run(&["all"]);
    assert!(!run.ok());
    assert!(
        run.stderr.contains("ripwire is not on PATH"),
        "{}",
        run.stderr
    );
    assert!(run.calls.is_empty(), "{}", run.calls);
}

#[test]
fn a_single_target_without_ripwire_warns_and_counts_it_as_not_run() {
    let run = Sandbox::without("ripwire").run(&["default"]);
    assert!(run.ok(), "{}", run.both());
    assert!(
        run.stderr.contains("ripwire is not on PATH"),
        "{}",
        run.stderr
    );
    assert!(
        run.last_line().contains("the ripwire e2e tests"),
        "{}",
        run.last_line()
    );
}

#[test]
fn an_unknown_target_is_a_usage_error() {
    let run = Sandbox::all().run(&["everything"]);
    assert_eq!(run.code, Some(2), "{}", run.both());
    assert!(run.stderr.contains("usage"), "{}", run.stderr);
    assert!(run.calls.is_empty());
}

#[test]
fn under_github_actions_each_check_is_its_own_log_group() {
    let run = Sandbox::all()
        .env("GITHUB_ACTIONS", "true")
        .run(&["default"]);
    assert!(run.ok(), "{}", run.both());
    let groups = run.stdout.matches("::group::").count();
    assert_eq!(
        groups,
        DEFAULT_CALLS.len() + 1,
        "fmt..CA-10 plus the fixture guard:\n{}",
        run.stdout
    );
    assert_eq!(groups, run.stdout.matches("::endgroup::").count());
}

#[test]
fn a_failing_check_still_closes_its_log_group() {
    let run = Sandbox::all()
        .env("GITHUB_ACTIONS", "true")
        .env("FAKE_FAIL", "nextest run --locked")
        .run(&["default"]);
    assert!(!run.ok());
    assert_eq!(
        run.stdout.matches("::group::").count(),
        run.stdout.matches("::endgroup::").count(),
        "{}",
        run.stdout
    );
}

/// Each job's own lines, from `  <job>:` to the next line at the jobs' indentation.
fn job_block(yml: &str, job: &str) -> Vec<String> {
    let header = format!("  {job}:");
    let is_job_start =
        |l: &str| l.starts_with("  ") && !l.starts_with("   ") && !l[2..].starts_with('#');
    yml.lines()
        .skip_while(|l| *l != header)
        .enumerate()
        .take_while(|(i, l)| *i == 0 || !is_job_start(l))
        .map(|(_, l)| l.to_owned())
        .collect()
}

#[test]
fn ci_jobs_call_the_script_under_their_required_names() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/rust.yml");
    let yml = fs::read_to_string(&path).expect("rust.yml");
    for job in ["default", "online", "plugin", "secrets"] {
        let block = job_block(&yml, job);
        assert!(
            block.contains(&format!("    name: {job}")),
            "job {job} renamed: {block:#?}"
        );
        let calls: Vec<&str> = block
            .iter()
            .filter_map(|l| l.trim().strip_prefix("run: scripts/check.sh "))
            .collect();
        assert_eq!(calls, [job], "job {job} must call its own target");
    }
}

/// A job's lines without comments: a property written in a comment is not a property.
fn code_lines(block: &[String]) -> Vec<String> {
    block
        .iter()
        .map(|l| match l.find(" #") {
            Some(i) if !l[..i].contains('"') => l[..i].trim_end().to_string(),
            _ => l.trim_end().to_string(),
        })
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .collect()
}

#[test]
fn the_secrets_job_scans_the_whole_history_with_a_pinned_gitleaks() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/rust.yml");
    let yml = fs::read_to_string(&path).expect("rust.yml");
    let lines = code_lines(&job_block(&yml, "secrets"));
    let block = lines.join("\n");
    let has = |exact: &str| lines.iter().any(|l| l.trim() == exact);
    let first = |needle: &str| {
        lines
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("no `{needle}` in the job:\n{block}"))
    };
    // A shallow checkout scans one commit: a secret added and removed earlier would pass.
    assert!(has("fetch-depth: 0"), "{block}");
    assert!(has("persist-credentials: false"), "{block}");
    // Nothing turns the job off or its failure into a pass.
    for escape in ["if:", "continue-on-error", "|| true"] {
        assert!(!block.contains(escape), "`{escape}` in the job:\n{block}");
    }
    // The only action is the pinned checkout; gitleaks comes from its release, not an action.
    let uses: Vec<&String> = lines.iter().filter(|l| l.contains("uses:")).collect();
    assert_eq!(uses.len(), 1, "{uses:?}");
    let pin = uses[0].split('@').nth(1).unwrap_or_default();
    assert!(
        uses[0].contains("actions/checkout@") && pin.len() == 40,
        "{uses:?}"
    );
    // A fixed release.
    let version = lines
        .iter()
        .find_map(|l| l.trim().strip_prefix("GITLEAKS_VERSION: "))
        .expect("GITLEAKS_VERSION pinned in the job");
    assert!(
        version.split('.').all(|n| n.parse::<u32>().is_ok()) && version.split('.').count() == 3,
        "not an exact version: {version}"
    );
    let sha = lines
        .iter()
        .find_map(|l| l.trim().strip_prefix("GITLEAKS_SHA256: "))
        .expect("GITLEAKS_SHA256 pinned in the job");
    assert!(
        sha.len() == 64 && sha.chars().all(|c| c.is_ascii_hexdigit()),
        "{sha}"
    );
    assert!(
        block.contains(r#"gh release download "v$GITLEAKS_VERSION""#),
        "{block}"
    );
    // Its hash is checked against the pinned one before it is unpacked, installed or run.
    let check = first("sha256sum -c");
    assert!(
        lines[check].contains(r#"echo "$GITLEAKS_SHA256  $tarball" | sha256sum -c"#),
        "{}",
        lines[check]
    );
    for later in [
        "tar ",
        "install ",
        "gitleaks\" version",
        "scripts/check.sh secrets",
    ] {
        assert!(
            check < first(later),
            "`{later}` comes before the hash check:\n{block}"
        );
    }
}

#[test]
fn the_gitleaks_exceptions_are_pinned_fingerprints_each_with_a_reason() {
    // A regex or path allowlist hides whatever else matches it; a fingerprint names one finding
    // (commit, file, rule, line), so a new secret anywhere is still found.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = fs::read_to_string(root.join(".gitleaks.toml")).expect(".gitleaks.toml");
    let settings: Vec<&str> = config
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    assert_eq!(settings, ["[extend]", "useDefault = true"], "{config}");
    let ignore = fs::read_to_string(root.join(".gitleaksignore")).expect(".gitleaksignore");
    let mut reason = false;
    let mut entries = 0;
    for line in ignore.lines().map(str::trim) {
        // The reason is the comment block right above the fingerprint, not the file's header.
        if line.is_empty() {
            reason = false;
            continue;
        }
        if line.starts_with('#') {
            reason = true;
            continue;
        }
        let parts: Vec<&str> = line.split(':').collect();
        assert!(
            parts.len() == 4
                && parts[0].len() == 40
                && parts[0].chars().all(|c| c.is_ascii_hexdigit())
                && parts[3].parse::<u32>().is_ok()
                && !line.contains('*'),
            "not a commit:file:rule:line fingerprint: {line}"
        );
        assert!(reason, "no comment saying why: {line}");
        reason = false;
        entries += 1;
    }
    assert!(entries > 0, "{ignore}");
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
