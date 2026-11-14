//! `scripts/plugin-surface.sh`, the PR step that makes a change to what plugin users run visible,
//! and checks the plugin's pinned hashes against the release they name (ai-native plan, step 5).
//!
//! The marketplace serves `master`: a PR that touches `integrations/claude-code/` or
//! `.claude-plugin/` changes what runs with the users' permissions. The step says so in the job
//! summary, and when `checksums.txt` changes it downloads `SHA256SUMS` from the published release
//! named on its first line and requires the same lines. A hash that did not come from a published
//! release fails. The script runs with fake `git` and `gh` here; CI runs it for real on PRs.
#![cfg(unix)]

mod common;

use common::shell::bashes as shells;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/plugin-surface.sh");
const CHECKSUMS: &str = "integrations/claude-code/scripts/checksums.txt";
const WARNING: &str = "This PR changes what the plugin's users run";

const SUMS: &str = "\
1111111111111111111111111111111111111111111111111111111111111111  ripwire-broker-v0.2.0-aarch64-apple-darwin.tar.gz
2222222222222222222222222222222222222222222222222222222222222222  ripwire-broker-v0.2.0-aarch64-unknown-linux-gnu.tar.gz
3333333333333333333333333333333333333333333333333333333333333333  ripwire-broker-v0.2.0-x86_64-apple-darwin.tar.gz
4444444444444444444444444444444444444444444444444444444444444444  ripwire-broker-v0.2.0-x86_64-unknown-linux-gnu.tar.gz
";

/// Logs `name args`, and answers as the real tools would:
/// - `git diff … -- PATHS` prints the lines of `FAKE_CHANGED` under one of PATHS. With rename
///   detection on (no `--no-renames`), `FAKE_RENAMED_FROM` is left out: git lists only the new name.
/// - `gh release view` exits `FAKE_VIEW_EXIT`, and prints `FAKE_DRAFT` (default false) through
///   `--jq .isDraft`, or the JSON object without it.
/// - `gh release download` writes `FAKE_SUMS` to its `--output` and exits `FAKE_DL_EXIT`.
const FAKE: &str = r#"#!/bin/bash
printf '%s\n' "${0##*/} $*" >> "$FAKE_LOG"
case "${0##*/} $1" in
    "git diff")
        renames=true
        for arg in "$@"; do [ "$arg" = --no-renames ] && renames=false; done
        while [ "$#" -gt 0 ] && [ "$1" != -- ]; do shift; done
        shift
        printf '%b' "${FAKE_CHANGED:-}" | while IFS= read -r file; do
            if $renames && [ "$file" = "${FAKE_RENAMED_FROM:-}" ]; then continue; fi
            for path in "$@"; do
                case "$file" in "$path" | "$path"/*) echo "$file" ;; esac
            done
        done
        ;;
    "gh release")
        case "$2" in
            view)
                [ "${FAKE_VIEW_EXIT:-0}" = 0 ] || { echo "release not found" >&2; exit "$FAKE_VIEW_EXIT"; }
                case " $* " in
                    *" --jq .isDraft "*) echo "${FAKE_DRAFT:-false}" ;;
                    *) echo "{\"isDraft\":${FAKE_DRAFT:-false}}" ;;
                esac
                ;;
            download)
                [ "${FAKE_DL_EXIT:-0}" = 0 ] || { echo "download failed" >&2; exit "$FAKE_DL_EXIT"; }
                out=""
                while [ "$#" -gt 0 ]; do
                    if [ "$1" = --output ]; then out=$2; fi
                    shift
                done
                printf '%b' "${FAKE_SUMS:-}" > "$out"
                ;;
        esac
        ;;
esac
"#;

/// The base the step passes; any revision name, so a script that ignores it is caught.
const BASE: &str = "base-sha-1234";

struct Sandbox {
    dir: tempfile::TempDir,
    env: Vec<(String, String)>,
    args: Vec<String>,
}

#[derive(Debug, PartialEq)]
struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    calls: String,
    summary: String,
}

impl Run {
    fn ok(&self) -> bool {
        self.code == Some(0)
    }
    fn both(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
    fn gh_calls(&self) -> Vec<&str> {
        self.calls
            .lines()
            .filter(|l| l.starts_with("gh "))
            .collect()
    }
}

impl Sandbox {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = dir.path().join("bin");
        fs::create_dir(&bin).expect("bin");
        for tool in ["git", "gh"] {
            let path = bin.join(tool);
            fs::write(&path, FAKE).expect("fake");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join("scripts")).expect("scripts");
        fs::copy(SCRIPT, repo.join("scripts/plugin-surface.sh")).expect("copy the script");
        let sandbox = Sandbox {
            dir,
            env: Vec::new(),
            args: vec![BASE.to_string()],
        };
        sandbox.checksums(&format!("v0.2.0\n{SUMS}"))
    }

    fn checksums(self, body: &str) -> Self {
        let path = self.dir.path().join("repo").join(CHECKSUMS);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, body).expect("checksums");
        self
    }

    fn without_checksums(self) -> Self {
        fs::remove_file(self.dir.path().join("repo").join(CHECKSUMS)).expect("rm checksums");
        self
    }

    fn args(mut self, args: &[&str]) -> Self {
        self.args = args.iter().map(ToString::to_string).collect();
        self
    }

    fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    fn changed(self, files: &[&str]) -> Self {
        let list: String = files.iter().map(|f| format!("{f}\\n")).collect();
        self.env("FAKE_CHANGED", &list)
    }

    /// Runs under every bash on the machine and insists they agree.
    fn run(&self) -> Run {
        let mut runs = shells().into_iter().map(|shell| self.run_with(&shell));
        let first = runs.next().expect("a bash");
        for other in runs {
            assert_eq!(other, first, "bash versions disagree");
        }
        first
    }

    fn run_with(&self, shell: &Path) -> Run {
        let log = self.dir.path().join("calls.log");
        let summary = self.dir.path().join("summary.md");
        let _ = fs::remove_file(&log);
        let _ = fs::remove_file(&summary);
        let mut cmd = Command::new(shell);
        cmd.arg(self.dir.path().join("repo/scripts/plugin-surface.sh"))
            .args(&self.args)
            .current_dir(self.dir.path().join("repo"))
            .env_clear()
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.dir.path().join("bin").display()),
            )
            .env("HOME", self.dir.path())
            .env("TMPDIR", self.dir.path())
            .env("FAKE_LOG", &log)
            .env("GITHUB_STEP_SUMMARY", &summary)
            .env("GITHUB_REPOSITORY", "aquental/ripwire-broker")
            .env("FAKE_SUMS", SUMS.replace('\n', "\\n"));
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        let output = cmd.output().expect("run plugin-surface.sh");
        Run {
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            // The temporary directory differs between runs.
            calls: fs::read_to_string(&log)
                .unwrap_or_default()
                .lines()
                .map(|l| match l.split_once(" --output ") {
                    Some((head, _)) => format!("{head} --output <tmp>\n"),
                    None => format!("{l}\n"),
                })
                .collect(),
            summary: fs::read_to_string(&summary).unwrap_or_default(),
        }
    }
}

#[test]
fn a_pr_that_leaves_the_plugin_alone_says_nothing_and_downloads_nothing() {
    let run = Sandbox::new().changed(&["src/broker.rs"]).run();
    assert!(run.ok(), "{}", run.both());
    assert!(run.summary.is_empty(), "{}", run.summary);
    assert!(run.gh_calls().is_empty(), "{}", run.calls);
    assert!(
        run.calls.contains(
            "git diff --no-renames --name-only base-sha-1234 HEAD -- integrations/claude-code .claude-plugin"
        ),
        "{}",
        run.calls
    );
}

#[test]
fn a_pr_that_touches_the_plugin_says_so_in_the_summary_without_blocking() {
    for file in [
        "integrations/claude-code/hooks/register.ts",
        ".claude-plugin/marketplace.json",
    ] {
        let run = Sandbox::new().changed(&[file]).run();
        assert!(run.ok(), "{file}: {}", run.both());
        assert!(run.summary.contains(WARNING), "{file}: {}", run.summary);
        assert!(run.summary.contains(file), "{file}: {}", run.summary);
        assert!(run.stdout.contains("::warning::"), "{file}: {}", run.stdout);
        assert!(run.gh_calls().is_empty(), "{file}: {}", run.calls);
    }
}

#[test]
fn changed_hashes_that_match_the_published_release_pass() {
    let run = Sandbox::new().changed(&[CHECKSUMS]).run();
    assert!(run.ok(), "{}", run.both());
    assert!(run.summary.contains(WARNING), "{}", run.summary);
    let gh = run.gh_calls();
    assert_eq!(gh.len(), 2, "{gh:?}");
    assert!(
        gh[0].starts_with("gh release view v0.2.0 --repo aquental/ripwire-broker --json isDraft"),
        "{gh:?}"
    );
    assert!(
        gh[1].starts_with(
            "gh release download v0.2.0 --repo aquental/ripwire-broker --pattern SHA256SUMS --output "
        ),
        "{gh:?}"
    );
}

#[test]
fn a_hash_the_release_did_not_publish_fails_and_names_the_asset() {
    let forged = SUMS.replacen(&"3".repeat(64), &"5".repeat(64), 1);
    let run = Sandbox::new()
        .checksums(&format!("v0.2.0\n{forged}"))
        .changed(&[CHECKSUMS])
        .run();
    assert!(!run.ok(), "{}", run.both());
    assert!(run.both().contains("::error::"), "{}", run.both());
    assert!(run.both().contains("x86_64-apple-darwin"), "{}", run.both());
}

#[test]
fn a_missing_or_extra_line_fails() {
    let mut lines: Vec<&str> = SUMS.lines().collect();
    lines.pop();
    let missing = format!("v0.2.0\n{}\n", lines.join("\n"));
    let extra = format!(
        "v0.2.0\n{SUMS}{}  ripwire-broker-v0.2.0-riscv.tar.gz\n",
        "6".repeat(64)
    );
    for body in [missing, extra] {
        let run = Sandbox::new().checksums(&body).changed(&[CHECKSUMS]).run();
        assert!(!run.ok(), "{body}: {}", run.both());
    }
}

#[test]
fn a_draft_or_missing_release_fails() {
    let draft = Sandbox::new()
        .env("FAKE_DRAFT", "true")
        .changed(&[CHECKSUMS])
        .run();
    assert!(!draft.ok(), "{}", draft.both());
    assert!(draft.both().contains("draft"), "{}", draft.both());
    let missing = Sandbox::new()
        .env("FAKE_VIEW_EXIT", "1")
        .changed(&[CHECKSUMS])
        .run();
    assert!(!missing.ok(), "{}", missing.both());
    assert!(
        missing.both().contains("no release v0.2.0"),
        "{}",
        missing.both()
    );
}

#[test]
fn a_failed_download_fails_instead_of_comparing_nothing() {
    let run = Sandbox::new()
        .env("FAKE_DL_EXIT", "1")
        .changed(&[CHECKSUMS])
        .run();
    assert!(!run.ok(), "{}", run.both());
    let empty = Sandbox::new()
        .env("FAKE_SUMS", "")
        .changed(&[CHECKSUMS])
        .run();
    assert!(!empty.ok(), "{}", empty.both());
    // No hash pinned and none published would compare equal.
    let nothing = Sandbox::new()
        .checksums("v0.2.0\n")
        .env("FAKE_SUMS", "")
        .changed(&[CHECKSUMS])
        .run();
    assert!(!nothing.ok(), "{}", nothing.both());
}

#[test]
fn a_first_line_that_is_not_a_version_tag_fails_before_any_download() {
    for first in ["", "latest", "v0.2", "v0.2.0; rm -rf /", "0.2.0"] {
        let run = Sandbox::new()
            .checksums(&format!("{first}\n{SUMS}"))
            .changed(&[CHECKSUMS])
            .run();
        assert!(!run.ok(), "{first:?}: {}", run.both());
        assert!(run.gh_calls().is_empty(), "{first:?}: {}", run.calls);
    }
}

#[test]
fn a_failing_git_diff_fails_instead_of_reading_no_change() {
    let dir = Sandbox::new();
    let git = dir.dir.path().join("bin/git");
    fs::write(
        &git,
        "#!/bin/bash\necho 'fatal: bad revision' >&2\nexit 128\n",
    )
    .expect("git");
    let run = dir.run();
    assert!(!run.ok(), "{}", run.both());
}

#[test]
fn the_base_is_the_one_argument() {
    for args in [&[][..], &["a", "b"][..]] {
        let run = Sandbox::new().args(args).changed(&[CHECKSUMS]).run();
        assert_eq!(run.code, Some(2), "{args:?}: {}", run.both());
        assert!(run.calls.is_empty(), "{args:?}: {}", run.calls);
    }
}

#[test]
fn a_renamed_or_deleted_checksums_file_is_still_checked() {
    // With rename detection, `git diff --name-only` lists only the new name.
    let run = Sandbox::new()
        .without_checksums()
        .env("FAKE_RENAMED_FROM", CHECKSUMS)
        .changed(&[CHECKSUMS, "integrations/claude-code/scripts/checksums.old"])
        .run();
    assert!(!run.ok(), "{}", run.both());
    assert!(
        run.both().contains("::error::") && run.both().contains("checksums.txt"),
        "{}",
        run.both()
    );
}

#[test]
fn pinned_lines_in_another_order_pass() {
    let reversed: Vec<&str> = SUMS.lines().rev().collect();
    let run = Sandbox::new()
        .checksums(&format!("v0.2.0\n{}\n", reversed.join("\n")))
        .changed(&[CHECKSUMS])
        .run();
    assert!(run.ok(), "{}", run.both());
}

#[test]
fn a_duplicate_pinned_line_fails() {
    let first = SUMS.lines().next().expect("a line");
    let run = Sandbox::new()
        .checksums(&format!("v0.2.0\n{SUMS}{first}\n"))
        .changed(&[CHECKSUMS])
        .run();
    assert!(!run.ok(), "{}", run.both());
}

#[test]
fn lines_that_are_not_hashes_fail_even_when_both_sides_agree() {
    for (pinned, published) in [
        ("v0.2.0\n\n", "\\n"),
        ("v0.2.0\n \n", " \\n"),
        (
            "v0.2.0\nnot-a-hash  asset.tar.gz\n",
            "not-a-hash  asset.tar.gz\\n",
        ),
    ] {
        let run = Sandbox::new()
            .checksums(pinned)
            .env("FAKE_SUMS", published)
            .changed(&[CHECKSUMS])
            .run();
        assert!(!run.ok(), "{pinned:?}: {}", run.both());
    }
}

#[test]
fn the_default_job_runs_the_step_on_pull_requests_against_the_base() {
    let yml = common::workflow::read("rust.yml");
    let job = common::workflow::job_code(&yml, "default");
    let step: Vec<String> = common::workflow::step(&job, "scripts/plugin-surface.sh")
        .iter()
        .map(|l| l.trim().to_string())
        .collect();
    // Exactly this: only on PRs (only a PR has a base), the commit the PR's merge was built on
    // (the base branch's tip may have moved), fetched alone (the checkout is one commit deep),
    // and nothing that turns a failure into a pass.
    assert_eq!(
        step,
        [
            "- name: Plugin surface",
            "if: github.event_name == 'pull_request'",
            "env:",
            "GH_TOKEN: ${{ github.token }}",
            "BASE_SHA: ${{ github.event.pull_request.base.sha }}",
            "run: |",
            r#"git fetch --no-tags --depth=1 origin "$BASE_SHA""#,
            "scripts/plugin-surface.sh FETCH_HEAD",
        ]
    );
}
