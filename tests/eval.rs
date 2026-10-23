//! Seam 6: the A/B instrument (PRD §16.2–16.4, §17, §23.15; D-116). Pure parts through the
//! library, the run through the `ripwire-eval` binary with a stand-in agent. No real agent, no
//! network: what a real run costs is the user's decision, not the suite's.
mod common;

use ripwire_broker::eval::arm::Arm;
use ripwire_broker::eval::corpus::Corpus;
use ripwire_broker::eval::report::{self, RunRecord, Verdict};
use ripwire_broker::eval::score;
use ripwire_broker::eval::transcript::{self, Summary};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The synthetic `stream-json` fixture, one event every 100 ms of the runner's clock.
fn fixture() -> Summary {
    let path = format!(
        "{}/tests/fixtures/eval/claude_code_stream.jsonl",
        env!("CARGO_MANIFEST_DIR")
    );
    let events: Vec<(u64, Value)> = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .enumerate()
        .map(|(n, l)| (n as u64 * 100, serde_json::from_str(l).unwrap()))
        .collect();
    transcript::summarize(&events)
}

// --- corpus ---

#[test]
fn a_corpus_is_validated_before_anything_runs() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let dir = tempfile::tempdir().unwrap();
    let task = |id: &str, repo: &str, base: &str, files: Value| {
        json!({"id": id, "repo": repo, "base": base, "prompt": "p",
               "reference": {"files": files, "tests": []}})
    };
    let path = dir.path().join("corpus.json");
    let good = repo.path().to_str().unwrap();
    std::fs::write(
        &path,
        json!({"tasks": [
            task("ok", good, &head, json!(["src/auth.py"])),
            task("ok", good, &head, json!(["src/auth.py"])),
            task("no-files", good, &head, json!([])),
            task("no-repo", "/nonexistent/repo", &head, json!(["a"])),
            task("no-base", good, "0000000000000000000000000000000000000000", json!(["a"])),
        ]})
        .to_string(),
    )
    .unwrap();

    let errors = Corpus::load(&path).unwrap().validate().unwrap_err();
    let all = errors.join("\n");
    for id in ["ok", "no-files", "no-repo", "no-base"] {
        assert!(all.contains(&format!("{id}:")), "{id} not reported:\n{all}");
    }

    // A relative repo path is relative to the corpus file, not to the caller's directory.
    let rel = dir.path().join("rel.json");
    let link = dir.path().join("repo");
    std::os::unix::fs::symlink(repo.path(), &link).unwrap();
    std::fs::write(
        &rel,
        json!({"tasks": [task("t1", "repo", &head, json!(["src/auth.py"]))]}).to_string(),
    )
    .unwrap();
    let corpus = Corpus::load(&rel).unwrap();
    assert_eq!(corpus.validate(), Ok(()));
    assert_eq!(corpus.tasks.len(), 1);
    assert_eq!(corpus.repos(), 1);
}

// --- transcript ---

#[test]
fn a_transcript_is_reduced_to_counts() {
    let s = fixture();

    assert!(s.valid(), "{:?}", s.invalid);
    // Grep, `rg` through Bash, and the subagent's Glob are searches; Read and `sed -n` reads.
    assert_eq!(s.calls.search, 3, "{:?}", s.calls);
    assert_eq!(s.calls.read, 2, "{:?}", s.calls);
    assert_eq!(s.calls.edit, 1, "{:?}", s.calls);
    assert_eq!(s.calls.mcp, 1, "{:?}", s.calls);
    assert_eq!(s.calls.other, 2, "Task and pytest: {:?}", s.calls);
    // The result event carries the session's totals.
    assert_eq!(s.tokens.total(), 50 + 2000 + 30000 + 800);
    assert!((s.cost_usd - 0.1234).abs() < 1e-9);
    assert_eq!(s.duration_ms, 12000);
    // The Edit is the 12th event.
    assert_eq!(s.first_edit_ms, Some(1100));
    assert_eq!(s.presented_files, vec!["src/routes.py", "src/auth.py"]);
    assert_eq!(s.presented_tests, vec!["tests/test_auth.py"]);
    assert_eq!(
        s.mcp_result_bytes, 287,
        "the envelope's text, byte for byte"
    );
    assert_eq!(s.mcp_servers, vec!["ripwire-broker"]);
}

#[test]
fn a_transcript_without_a_result_is_invalid_not_zero() {
    let init = json!({"type": "system", "subtype": "init", "tools": [], "mcp_servers": []});
    let no_result = transcript::summarize(&[(0, init)]);
    assert!(!no_result.valid());
    assert!(no_result.invalid.as_deref().unwrap().contains("result"));

    let result = json!({"type": "result", "subtype": "success", "is_error": false,
                        "usage": {"input_tokens": 1, "output_tokens": 1}});
    let no_init = transcript::summarize(&[(0, result.clone())]);
    assert!(no_init.invalid.as_deref().unwrap().contains("init"));

    let garbage = transcript::summarize(&[]);
    assert!(!garbage.valid());

    let failed = json!({"type": "result", "subtype": "error_max_turns", "is_error": true,
                        "usage": {"input_tokens": 1, "output_tokens": 1}});
    let init = json!({"type": "system", "subtype": "init", "tools": [], "mcp_servers": []});
    let s = transcript::summarize(&[(0, init), (1, failed)]);
    assert!(
        s.valid(),
        "a failed task is a measurement, not a broken run"
    );
    assert!(s.is_error);
}

#[test]
fn a_session_the_api_cut_short_is_invalid_not_a_failed_task() {
    // As Claude Code 2.x wrote it with its API unreachable (ECONNREFUSED), trimmed.
    let init = json!({"type": "system", "subtype": "init", "tools": [], "mcp_servers": []});
    let result = json!({"type": "result", "subtype": "success", "is_error": true,
        "terminal_reason": "api_error", "api_error_status": null, "num_turns": 1,
        "result": "API Error: Connection refused — a firewall or proxy may be blocking it (ECONNREFUSED)",
        "usage": {"input_tokens": 0, "output_tokens": 0}});

    let s = transcript::summarize(&[(0, init), (1, result)]);

    let why = s.invalid.expect("the arm did not fail; the API did");
    assert!(why.contains("ECONNREFUSED"), "{why}");
}

#[test]
fn an_arm_contaminated_by_a_foreign_mcp_is_invalid() {
    let s = fixture();
    assert_eq!(Arm::Broker.contamination(&s), None);
    let none = Arm::None
        .contamination(&s)
        .expect("ripwire-broker leaked into `none`");
    assert!(none.contains("ripwire-broker"), "{none}");
    assert!(Arm::Ripwire.contamination(&s).is_some());

    // The arm's own server must have connected, or `broker` silently becomes `none`.
    let init = json!({"type": "system", "subtype": "init", "tools": ["Read"],
                      "mcp_servers": [{"name": "ripwire-broker", "status": "failed"}]});
    let result = json!({"type": "result", "is_error": false, "usage": {}});
    let down = transcript::summarize(&[(0, init), (1, result)]);
    let why = Arm::Broker.contamination(&down).expect("not connected");
    assert!(why.contains("not connected"), "{why}");
}

#[test]
fn arms_name_their_mcp_server_and_nothing_else() {
    let tools = ripwire_broker::eval::arm::Tools {
        broker: "/bin/rb".into(),
        ripwire: "/bin/rw".into(),
        memory_debug_log: false,
    };
    let ws = Path::new("/work");
    let st = Path::new("/state");
    assert_eq!(
        Arm::None.mcp_config(&tools, ws, st),
        json!({"mcpServers": {}})
    );
    let rw = Arm::Ripwire.mcp_config(&tools, ws, st);
    assert_eq!(rw["mcpServers"]["ripwire"]["command"], "/bin/rw");
    assert_eq!(
        rw["mcpServers"]["ripwire"]["args"],
        json!(["/work", "--mcp"])
    );
    let b = Arm::Broker.mcp_config(&tools, ws, st);
    assert_eq!(
        b["mcpServers"]["ripwire-broker"]["args"],
        json!(["serve", "--workspace", "/work", "--ripwire", "/bin/rw"])
    );
    let on = Arm::BrokerOnline.mcp_config(&tools, ws, st);
    let args = on["mcpServers"]["ripwire-broker"]["args"]
        .as_array()
        .unwrap();
    assert!(args.contains(&json!("--online")));
    for a in [Arm::None, Arm::Ripwire, Arm::Broker, Arm::BrokerOnline] {
        assert_eq!(Arm::parse(a.name()), Some(a));
    }
    assert_eq!(Arm::parse("bogus"), None);
}

// --- scoring ---

#[test]
fn a_run_is_scored_against_the_reference_patch() {
    let task: ripwire_broker::eval::corpus::Task = serde_json::from_value(json!({
        "id": "t", "repo": "/r", "base": "b", "prompt": "p",
        "reference": {"files": ["src/auth.py", "src/session.py"], "tests": ["tests/test_auth.py"]}
    }))
    .unwrap();
    let s = fixture();
    let modified = vec!["src/auth.py".to_string(), "notes.txt".to_string()];

    let sc = score::score(&task, &s, &modified, Some(true));

    assert_eq!(sc.file_recall, 0.5);
    assert_eq!(sc.file_precision, Some(0.5));
    assert_eq!(sc.presented_recall, Some(0.5));
    assert_eq!(sc.first_correct_rank, Some(2), "src/auth.py came second");
    assert_eq!(sc.test_recall, Some(1.0));
    assert_eq!(sc.correct, Some(true));
    let mut no_tests = task.clone();
    no_tests.reference.tests.clear();
    assert_eq!(
        score::score(&no_tests, &s, &modified, None).test_recall,
        None,
        "no reference tests: nothing to find, not a full score"
    );

    let nothing = score::score(&task, &Summary::default(), &[], None);
    assert_eq!(nothing.file_recall, 0.0);
    assert_eq!(
        nothing.file_precision, None,
        "nothing modified: precision undefined"
    );
    assert_eq!(nothing.presented_recall, None, "no broker in the arm");
    assert_eq!(nothing.correct, None);
}

#[test]
fn modified_files_come_from_git_including_new_ones() {
    let repo = common::sample_repo();
    std::fs::write(repo.path().join("src/auth.py"), "changed\n").unwrap();
    std::fs::create_dir_all(repo.path().join("src/new")).unwrap();
    std::fs::write(repo.path().join("src/new/token.py"), "x\n").unwrap();

    let mut files = score::modified_files(repo.path(), "HEAD");
    files.sort();
    assert_eq!(files, vec!["src/auth.py", "src/new/token.py"]);
}

// --- bars ---

/// One valid run of `arm` on task `n`, spread over `repos` repositories.
fn rec(n: usize, repos: usize, arm: Arm, tokens: u64, exploratory: u64) -> RunRecord {
    RunRecord {
        task: format!("t{n}"),
        repo: format!("repo{}", n % repos),
        arm: arm.name().into(),
        valid: true,
        tokens,
        search: exploratory,
        correct: Some(true),
        file_recall: 0.8,
        presented_recall: Some(0.7),
        test_recall: Some(0.9),
        ..RunRecord::default()
    }
}

fn corpus(
    tasks: usize,
    repos: usize,
    broker_tokens: u64,
    broker_exploratory: u64,
) -> Vec<RunRecord> {
    (0..tasks)
        .flat_map(|n| {
            let mut b = rec(n, repos, Arm::Broker, broker_tokens, broker_exploratory);
            b.file_recall = 0.9;
            [rec(n, repos, Arm::None, 1000, 100), b]
        })
        .collect()
}

fn verdict(records: &[RunRecord], id: &str) -> Verdict {
    report::bars(records)
        .into_iter()
        .find(|b| b.id == id)
        .unwrap_or_else(|| panic!("no bar {id}"))
        .verdict
}

#[test]
fn the_bars_follow_the_prd_thresholds() {
    // §17.1: at least 35% fewer tokens. Exactly 35% passes; one token short fails.
    assert_eq!(verdict(&corpus(30, 3, 650, 70), "17.1"), Verdict::Pass);
    assert_eq!(verdict(&corpus(30, 3, 651, 70), "17.1"), Verdict::Fail);
    // §17.2: at least 30% fewer exploratory calls.
    assert_eq!(verdict(&corpus(30, 3, 650, 70), "17.2"), Verdict::Pass);
    assert_eq!(verdict(&corpus(30, 3, 650, 71), "17.2"), Verdict::Fail);
    // §17.3, 17.4, 17.5 on the same records.
    let ok = corpus(30, 3, 650, 70);
    assert_eq!(verdict(&ok, "17.3"), Verdict::Pass);
    assert_eq!(verdict(&ok, "17.4"), Verdict::Pass);
    assert_eq!(verdict(&ok, "17.5"), Verdict::Pass);

    // Fewer than 30 tasks or 3 repositories is not a result, whatever the numbers say.
    assert_eq!(
        verdict(&corpus(29, 3, 100, 1), "17.1"),
        Verdict::Insufficient
    );
    assert_eq!(
        verdict(&corpus(30, 2, 100, 1), "17.1"),
        Verdict::Insufficient
    );

    // An invalid run never counts: invalidating every broker run leaves no arm to compare.
    let mut broken = corpus(30, 3, 100, 1);
    for r in broken.iter_mut().filter(|r| r.arm == "broker") {
        r.valid = false;
    }
    assert_eq!(verdict(&broken, "17.1"), Verdict::Insufficient);

    // Cheaper but less correct is not a pass (§17 "uma economia que diminua correção").
    let mut worse = corpus(30, 3, 100, 1);
    for r in worse.iter_mut().filter(|r| r.arm == "broker").take(1) {
        r.correct = Some(false);
    }
    assert_eq!(verdict(&worse, "17.3"), Verdict::Fail);
}

#[test]
fn the_online_bar_is_measured_on_divergent_vocabulary_only() {
    let mut records = vec![];
    for n in 0..30 {
        let divergent = n % 2 == 0;
        let mut off = rec(n, 3, Arm::Broker, 600, 50);
        let mut on = rec(n, 3, Arm::BrokerOnline, 600, 40);
        off.vocabulary_diverges = divergent;
        on.vocabulary_diverges = divergent;
        off.presented_recall = Some(0.5);
        // +10 pp exactly where the vocabulary diverges, nothing elsewhere.
        on.presented_recall = Some(if divergent { 0.6 } else { 0.5 });
        records.extend([off, on]);
    }
    assert_eq!(verdict(&records, "23.15.2"), Verdict::Pass);
    assert_eq!(
        verdict(&records, "23.15.3"),
        Verdict::Pass,
        "50 → 40 is −20%"
    );
    assert_eq!(verdict(&records, "23.15.1"), Verdict::Pass);

    for r in records.iter_mut().filter(|r| r.arm == "broker-online") {
        if r.vocabulary_diverges {
            r.presented_recall = Some(0.59);
        }
    }
    assert_eq!(verdict(&records, "23.15.2"), Verdict::Fail);

    // Without the online arm there is nothing to compare.
    let offline: Vec<_> = records
        .into_iter()
        .filter(|r| r.arm != "broker-online")
        .collect();
    assert_eq!(verdict(&offline, "23.15.2"), Verdict::Insufficient);
}

// --- the binary, end to end ---

/// A stand-in for `claude -p --output-format stream-json`: reads the prompt on stdin, finds
/// its MCP config after `--mcp-config`, announces the server it names, edits a file in its
/// working directory and reports a result. Counts its runs in `$COUNTER`.
fn fake_agent(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("fake-agent");
    common::write_executable(
        &path,
        r##"#!/bin/sh
cat > /dev/null
cfg=""
while [ $# -gt 0 ]; do
  if [ "$1" = "--mcp-config" ]; then cfg="$2"; shift; fi
  shift
done
echo run >> "$COUNTER"
servers='[]'; tools='["Read","Edit"]'
if grep -q '"ripwire-broker"' "$cfg"; then
  servers='[{"name":"ripwire-broker","status":"connected"}]'
  tools='["Read","Edit","mcp__ripwire-broker__context_for_task"]'
elif grep -q '"ripwire"' "$cfg"; then
  servers='[{"name":"ripwire","status":"connected"}]'
  tools='["Read","Edit","mcp__ripwire__explore"]'
fi
echo "{\"type\":\"system\",\"subtype\":\"init\",\"tools\":$tools,\"mcp_servers\":$servers}"
echo '{"type":"assistant","message":{"id":"m1","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"src/auth.py"}}]}}'
echo '{"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"t2","name":"Edit","input":{"file_path":"src/auth.py"}}]}}'
echo "# expired tokens are rejected" >> src/auth.py
echo '{"type":"result","subtype":"success","is_error":false,"duration_ms":5,"total_cost_usd":0.01,"usage":{"input_tokens":10,"cache_read_input_tokens":90,"output_tokens":5}}'
"##,
    );
    path
}

fn eval(args: &[&str], counter: &Path) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ripwire-eval"))
        .args(args)
        .env("COUNTER", counter)
        // The online arm's refusal is asserted without a key: one in the caller's shell runs it.
        .env_remove("RIPWIRE_BROKER_JEV_API_KEY")
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn ripwire_eval_runs_every_arm_and_never_touches_the_source_repo() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let agent = fake_agent(work.path());
    let counter = work.path().join("runs");
    let corpus = work.path().join("corpus.json");
    std::fs::write(
        &corpus,
        json!({"tasks": [{
            "id": "expired-token", "repo": repo.path(), "base": head,
            "prompt": "make login reject expired tokens",
            "reference": {"files": ["src/auth.py"], "tests": ["tests/test_auth.py"]},
            "check": "grep -q 'expired tokens' src/auth.py"
        }]})
        .to_string(),
    )
    .unwrap();
    let out = work.path().join("out");
    let agent_cmd = format!("{} --mcp-config {{mcp_config}}", agent.display());
    let run = |extra: &[&str]| {
        let mut args = vec![
            "run",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--arms",
            "none,ripwire,broker",
            "--agent-cmd",
            &agent_cmd,
        ];
        args.extend(extra);
        eval(&args, &counter)
    };

    let (code, _, err) = run(&[]);
    assert_eq!(code, 0, "{err}");

    let results = std::fs::read_to_string(out.join("results.jsonl")).unwrap();
    let records: Vec<Value> = results
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let arms: Vec<&str> = records.iter().map(|r| r["arm"].as_str().unwrap()).collect();
    assert_eq!(arms, vec!["none", "ripwire", "broker"]);
    for r in &records {
        assert_eq!(r["valid"], true, "{r}");
        assert_eq!(r["correct"], true, "the check ran in the clone: {r}");
        assert_eq!(r["file_recall"], 1.0, "{r}");
        assert_eq!(r["tokens"], 105, "{r}");
        assert_eq!(r["read"], 1, "{r}");
        assert_eq!(r["edit"], 1, "{r}");
    }
    for arm in ["none", "ripwire", "broker"] {
        assert!(
            out.join(format!("transcripts/expired-token__{arm}__1.jsonl"))
                .exists(),
            "{arm}"
        );
    }

    // The source repository is exactly as it was: no edit, no worktree, same HEAD.
    assert_eq!(git(repo.path(), &["status", "--porcelain"]), "");
    assert_eq!(git(repo.path(), &["rev-parse", "HEAD"]), head);
    assert_eq!(git(repo.path(), &["worktree", "list"]).lines().count(), 1);

    // Run again: every (task, arm, repeat) is already in results.jsonl, so nothing reruns.
    let (code, _, err) = run(&[]);
    assert_eq!(code, 0, "{err}");
    let runs = std::fs::read_to_string(&counter).unwrap().lines().count();
    assert_eq!(runs, 3, "resumed, not repeated");
    // A second repeat only runs what is new.
    let (code, _, err) = run(&["--repeats", "2"]);
    assert_eq!(code, 0, "{err}");
    let runs = std::fs::read_to_string(&counter).unwrap().lines().count();
    assert_eq!(runs, 6);

    let (code, md, err) = eval(&["report", "--out", out.to_str().unwrap()], &counter);
    assert_eq!(code, 0, "{err}");
    assert!(md.contains("17.1"), "{md}");
    assert!(
        md.contains("insuficiente"),
        "one task in one repo cannot pass a bar: {md}"
    );

    // The online arm needs the credential, and refuses before running anything.
    let (code, _, err) = eval(
        &[
            "run",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--arms",
            "broker-online",
            "--agent-cmd",
            &agent_cmd,
        ],
        &counter,
    );
    assert_ne!(code, 0);
    assert!(err.contains("RIPWIRE_BROKER_JEV_API_KEY"), "{err}");
    let runs = std::fs::read_to_string(&counter).unwrap().lines().count();
    assert_eq!(runs, 6, "nothing ran");
}

#[test]
fn an_arm_without_memory_never_asks_the_broker_for_memory_status() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let agent = fake_agent(work.path());
    let counter = work.path().join("runs");
    let calls = work.path().join("broker-calls");
    let broker = work.path().join("fake-broker");
    common::write_executable(
        &broker,
        format!("#!/bin/sh\necho \"$*\" >> '{}'\n", calls.display()),
    );
    let corpus = work.path().join("corpus.json");
    std::fs::write(
        &corpus,
        json!({"tasks": [{"id": "t", "repo": repo.path(), "base": head, "prompt": "p",
                          "reference": {"files": ["src/auth.py"]}, "check": "true"}]})
        .to_string(),
    )
    .unwrap();
    let out = work.path().join("out");
    let agent_cmd = format!("{} --mcp-config {{mcp_config}}", agent.display());

    let (code, _, err) = eval(
        &[
            "run",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--arms",
            "none,broker",
            "--agent-cmd",
            &agent_cmd,
            "--broker",
            broker.to_str().unwrap(),
        ],
        &counter,
    );

    assert_eq!(code, 0, "{err}");
    let asked = std::fs::read_to_string(&calls).unwrap_or_default();
    assert!(!asked.contains("memory"), "{asked}");
}

#[test]
fn the_eval_command_line_refuses_what_it_would_ignore_or_misread() {
    let work = tempfile::tempdir().unwrap();
    let counter = work.path().join("runs");
    let missing = work.path().join("missing");
    let missing = missing.to_str().unwrap();

    for help in [&["--help"][..], &["run", "-h"], &["help"]] {
        let (code, out, err) = eval(help, &counter);
        assert_eq!(code, 0, "{help:?}: {err}");
        assert!(out.contains("usage: ripwire-eval"), "{help:?}: {out}");
    }
    for (args, says) in [
        (
            &["run", "--corpus", "c", "--out", "o", "--repeats", "0"][..],
            "--repeats",
        ),
        (
            &[
                "run",
                "--corpus",
                "c",
                "--out",
                "o",
                "--repeats",
                "4294967297",
            ],
            "--repeats",
        ),
        (
            &[
                "run",
                "--corpus",
                "c",
                "--out",
                "o",
                "--arms",
                "none,broker,none",
            ],
            "none",
        ),
        (
            &["run", "--corpus", "c", "--out", "o", "--out", "p"],
            "--out",
        ),
        (&["check", "--corpus", "c", "--out", "o"], "--out"),
        (&["validate", "--corpus", "c", "--json"], "--json"),
        (&["report", "--out", missing], "no results"),
    ] {
        let (code, _, err) = eval(args, &counter);
        assert_ne!(code, 0, "{args:?} accepted");
        assert!(err.contains(says), "{args:?}: {err}");
    }
}

#[test]
fn a_contaminated_run_is_recorded_as_invalid() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    // An agent whose global config leaks a server into every arm.
    let agent = work.path().join("leaky-agent");
    common::write_executable(
        &agent,
        r#"#!/bin/sh
cat > /dev/null
echo '{"type":"system","subtype":"init","tools":["mcp__graft__ask"],"mcp_servers":[{"name":"graft","status":"connected"}]}'
echo '{"type":"result","is_error":false,"usage":{"input_tokens":1,"output_tokens":1}}'
"#,
    );
    let corpus = work.path().join("corpus.json");
    std::fs::write(
        &corpus,
        json!({"tasks": [{"id": "t", "repo": repo.path(), "base": head, "prompt": "p",
                          "reference": {"files": ["src/auth.py"]}}]})
        .to_string(),
    )
    .unwrap();
    let out = work.path().join("out");

    let (code, _, err) = eval(
        &[
            "run",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--arms",
            "none",
            "--agent-cmd",
            agent.to_str().unwrap(),
        ],
        &work.path().join("runs"),
    );

    assert_eq!(code, 0, "{err}");
    let line = std::fs::read_to_string(out.join("results.jsonl")).unwrap();
    let r: Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(r["valid"], false, "{r}");
    assert!(r["invalid"].as_str().unwrap().contains("graft"), "{r}");
}

#[test]
fn the_first_edit_is_the_earliest_one() {
    let edit = |id: &str| {
        json!({"type": "assistant", "message": {"id": id, "content": [
            {"type": "tool_use", "id": id, "name": "Write", "input": {"file_path": "a"}}]}})
    };
    let s = transcript::summarize(&[
        (
            0,
            json!({"type": "system", "subtype": "init", "tools": [], "mcp_servers": []}),
        ),
        (300, edit("e1")),
        (900, edit("e2")),
        (
            1000,
            json!({"type": "result", "is_error": false, "usage": {}}),
        ),
    ]);
    assert_eq!(s.first_edit_ms, Some(300));
    assert_eq!(s.calls.edit, 2);
}

#[test]
fn the_agent_cannot_see_history_after_the_base() {
    // A task taken from history: the fix is a later commit of the same repository. If the
    // agent's clone carries it, `git log --all` hands the agent the answer.
    let repo = common::sample_repo();
    let base = git(repo.path(), &["rev-parse", "HEAD"]);
    std::fs::write(repo.path().join("src/auth.py"), "the answer\n").unwrap();
    // An identity of its own, as `sample_repo` does: CI runners have none configured.
    git(
        repo.path(),
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qam",
            "the reference fix",
        ],
    );
    let work = tempfile::tempdir().unwrap();
    let seen = work.path().join("seen");
    let agent = work.path().join("curious-agent");
    common::write_executable(
        &agent,
        r#"#!/bin/sh
cat > /dev/null
{ git log --all --format=%s; git for-each-ref; cat src/auth.py; } > "$COUNTER"
echo '{"type":"system","subtype":"init","tools":[],"mcp_servers":[]}'
echo '{"type":"result","is_error":false,"usage":{"input_tokens":1,"output_tokens":1}}'
"#,
    );
    let corpus = work.path().join("corpus.json");
    std::fs::write(
        &corpus,
        json!({"tasks": [{"id": "t", "repo": repo.path(), "base": base, "prompt": "p",
                          "reference": {"files": ["src/auth.py"]}}]})
        .to_string(),
    )
    .unwrap();

    let (code, _, err) = eval(
        &[
            "run",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            work.path().join("out").to_str().unwrap(),
            "--arms",
            "none",
            "--agent-cmd",
            agent.to_str().unwrap(),
        ],
        &seen,
    );

    assert_eq!(code, 0, "{err}");
    let seen = std::fs::read_to_string(&seen).unwrap();
    assert!(
        !seen.contains("the reference fix"),
        "the future leaked:\n{seen}"
    );
    assert!(
        !seen.contains("the answer"),
        "the fixed file leaked:\n{seen}"
    );
    assert!(
        seen.contains("def login"),
        "the agent works on the base:\n{seen}"
    );
}

// --- per-task setup, environment and teardown (real repositories need deps and a database) ---

#[test]
fn setup_env_and_teardown_wrap_each_run_and_setup_is_not_an_edit() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let agent = fake_agent(work.path());
    let log = work.path().join("log");
    let corpus = work.path().join("corpus.json");
    std::fs::write(
        &corpus,
        json!({"tasks": [
            {"id": "T-1", "repo": repo.path(), "base": head, "prompt": "p",
             "reference": {"files": ["src/auth.py"]},
             "env": {"RUN_TAG": "db_{run}"},
             // What setup builds (deps, caches) is not something the agent edited.
             "setup": "echo \"setup $RUN_TAG\" >> \"$COUNTER\" && echo built > built.txt",
             "check": "echo \"check $RUN_TAG\" >> \"$COUNTER\" && echo check-said-this",
             "teardown": "echo \"teardown $RUN_TAG\" >> \"$COUNTER\""},
            {"id": "broken-setup", "repo": repo.path(), "base": head, "prompt": "p",
             "reference": {"files": ["src/auth.py"]},
             "setup": "exit 3"}
        ]})
        .to_string(),
    )
    .unwrap();
    let out = work.path().join("out");
    let agent_cmd = format!("{} --mcp-config {{mcp_config}}", agent.display());

    let (code, _, err) = eval(
        &[
            "run",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--arms",
            "none,broker",
            "--agent-cmd",
            &agent_cmd,
        ],
        &log,
    );
    assert_eq!(code, 0, "{err}");

    let lines: Vec<String> = std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    // Run ids are safe for a database name: lowercase, letters, digits and underscores.
    assert_eq!(
        lines,
        vec![
            "setup db_t_1__none__1",
            "run",
            "check db_t_1__none__1",
            "teardown db_t_1__none__1",
            "setup db_t_1__broker__1",
            "run",
            "check db_t_1__broker__1",
            "teardown db_t_1__broker__1",
        ],
        "the broken setup never reached its agent"
    );
    let records: Vec<Value> = std::fs::read_to_string(out.join("results.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    for r in records.iter().filter(|r| r["task"] == "T-1") {
        assert_eq!(
            r["file_precision"], 1.0,
            "built.txt is setup's, not the agent's: {r}"
        );
        assert_eq!(r["correct"], true, "{r}");
    }
    // The check's output sits next to the transcript, for the run that needs explaining.
    let check_log = out.join("transcripts/T-1__none__1.check.log");
    assert!(
        std::fs::read_to_string(&check_log)
            .unwrap()
            .contains("check-said-this")
    );
    let broken: Vec<&Value> = records
        .iter()
        .filter(|r| r["task"] == "broken-setup")
        .collect();
    assert_eq!(broken.len(), 2);
    for r in broken {
        assert_eq!(r["valid"], false, "{r}");
        assert!(r["invalid"].as_str().unwrap().contains("setup"), "{r}");
    }
}

#[test]
fn a_hook_that_ran_in_the_session_invalidates_every_arm() {
    // A repository can commit hooks of its own (e.g. another context tool's): they inject into
    // every arm, `none` included, and no MCP listing would show them.
    let hook = json!({"type": "system", "subtype": "hook_response", "hook_name": "SessionStart:startup",
                      "hook_event": "SessionStart", "output": "context from a tool"});
    let init = json!({"type": "system", "subtype": "init", "tools": [], "mcp_servers": []});
    let result = json!({"type": "result", "is_error": false, "usage": {}});
    let s = transcript::summarize(&[(0, hook), (1, init), (2, result)]);

    assert_eq!(s.hooks, vec!["SessionStart:startup"]);
    for arm in [Arm::None, Arm::Ripwire, Arm::Broker] {
        let why = arm.contamination(&s).expect("a hook ran");
        assert!(why.contains("hook"), "{why}");
    }
}

#[test]
fn the_default_agent_loads_no_settings_and_reports_hooks() {
    let argv: Vec<&str> = ripwire_broker::eval::runner::DEFAULT_AGENT
        .split_whitespace()
        .collect();
    let after = |flag: &str| argv[argv.iter().position(|a| *a == flag).unwrap() + 1];
    // `local` is .claude/settings.local.json, which a fresh repository never has: neither the
    // user's settings nor the ones a repository commits.
    assert_eq!(after("--setting-sources"), "local");
    assert!(argv.contains(&"--strict-mcp-config"));
    assert!(argv.contains(&"--include-hook-events"));
}

// --- a task is only worth running if its check tells the base from the fix ---

#[test]
fn validate_requires_the_check_to_fail_at_the_base_and_pass_at_the_fix() {
    let repo = common::sample_repo();
    let base = git(repo.path(), &["rev-parse", "HEAD"]);
    std::fs::write(
        repo.path().join("src/auth.py"),
        "def login(user, token):\n    raise ValueError('expired')\n",
    )
    .unwrap();
    std::fs::write(repo.path().join("tests/test_expiry.txt"), "hidden test\n").unwrap();
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            "the fix",
        ],
    );
    let fix = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    // The hidden test comes from the fix commit, through {repo} and {fix}.
    let hidden = "git -C {repo} show {fix}:tests/test_expiry.txt > tests/test_expiry.txt && grep -q expired src/auth.py";
    let task = |id: &str, check: &str| {
        json!({"id": id, "repo": repo.path(), "base": base, "fix": fix, "prompt": "p",
               "reference": {"files": ["src/auth.py"]}, "check": check})
    };
    let corpus = work.path().join("corpus.json");
    std::fs::write(
        &corpus,
        json!({"tasks": [
            task("good", hidden),
            task("trivial", "true"),
            task("impossible", "echo boom-at-the-fix >&2; false"),
            {"id": "unverifiable", "repo": repo.path(), "base": base, "prompt": "p",
             "reference": {"files": ["src/auth.py"]}}
        ]})
        .to_string(),
    )
    .unwrap();

    let (code, out, err) = eval(
        &["validate", "--corpus", corpus.to_str().unwrap()],
        &work.path().join("unused"),
    );

    assert_ne!(code, 0, "two tasks are broken: {out}{err}");
    let line = |id: &str| {
        out.lines()
            .find(|l| l.starts_with(&format!("{id}:")))
            .unwrap_or_else(|| panic!("{id} missing:\n{out}"))
            .to_string()
    };
    assert!(line("good").contains("ok"), "{out}");
    assert!(line("trivial").contains("passes at the base"), "{out}");
    assert!(line("impossible").contains("fails at the fix"), "{out}");
    // A failure says where its output went: a check that fails without a trace cannot be fixed.
    let log = work.path().join("validate-logs/impossible.fix.check.log");
    assert!(
        line("impossible").contains("validate-logs/impossible.fix.check.log"),
        "{out}"
    );
    assert!(
        std::fs::read_to_string(&log)
            .unwrap()
            .contains("boom-at-the-fix"),
        "{}",
        log.display()
    );
    assert!(line("unverifiable").contains("no check"), "{out}");
    // The source repository is only read.
    assert_eq!(git(repo.path(), &["status", "--porcelain"]), "");
}

#[test]
fn a_repository_path_with_spaces_and_quotes_reaches_the_check_as_one_word() {
    let sample = common::sample_repo();
    let outer = tempfile::tempdir().unwrap();
    let repo = outer.path().join("it's a repo; true");
    std::fs::rename(sample.path(), &repo).unwrap();
    let base = git(&repo, &["rev-parse", "HEAD"]);
    std::fs::write(repo.join("tests/test_expiry.txt"), "hidden test\n").unwrap();
    git(&repo, &["add", "."]);
    git(
        &repo,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            "the fix",
        ],
    );
    let fix = git(&repo, &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let corpus = work.path().join("corpus.json");
    std::fs::write(
        &corpus,
        json!({"tasks": [{"id": "spaced", "repo": repo, "base": base, "fix": fix, "prompt": "p",
                          "reference": {"files": ["src/auth.py"]},
                          "check": "git -C {repo} show {fix}:tests/test_expiry.txt > /dev/null && test -f tests/test_expiry.txt"}]})
        .to_string(),
    )
    .unwrap();

    let (code, out, err) = eval(
        &["validate", "--corpus", corpus.to_str().unwrap()],
        &work.path().join("unused"),
    );

    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("spaced: ok"), "{out}{err}");
}

#[test]
fn a_context_tool_run_from_the_shell_is_contamination_too() {
    // A repository can ship another tool's index whose README says "run graft ask"; ripwire is on
    // the PATH too. Through Bash, no MCP listing shows either.
    let bash = |id: &str, command: &str| {
        json!({"type": "assistant", "message": {"id": id, "content": [
            {"type": "tool_use", "id": id, "name": "Bash", "input": {"command": command}}]}})
    };
    let init = json!({"type": "system", "subtype": "init", "tools": [], "mcp_servers": []});
    let result = json!({"type": "result", "is_error": false, "usage": {}});
    let session = |command: &str| {
        transcript::summarize(&[
            (0, init.clone()),
            (1, bash("b1", command)),
            (2, result.clone()),
        ])
    };

    let graft = session("cd /work && graft ask \"where is login\" --source");
    for arm in [Arm::None, Arm::Ripwire, Arm::Broker] {
        let why = arm
            .contamination(&graft)
            .expect("graft is never part of an arm");
        assert!(why.contains("graft"), "{why}");
    }
    let ripwire = session("/Users/me/.local/bin/ripwire . --for=\"login\"");
    assert!(Arm::None.contamination(&ripwire).is_some());
    assert!(
        Arm::Broker.contamination(&ripwire).is_some(),
        "the broker arm goes through the broker"
    );
    // Running the binary under development is the work, not a context tool.
    let own = session("cargo build && ./target/debug/ripwire-broker --help | grep ripwire");
    assert_eq!(Arm::None.contamination(&own), None);
    // `rg ripwire` searches for the word; it does not run the tool.
    assert_eq!(Arm::None.contamination(&session("rg -n ripwire src")), None);
}

// --- PR #29 review: the shell guard, the fix commit, and setup files the agent edits too ---

#[test]
fn the_shell_guard_sees_through_assignments_wrappers_and_quotes() {
    let bash = |command: &str| {
        let init = json!({"type": "system", "subtype": "init", "tools": [], "mcp_servers": []});
        let call = json!({"type": "assistant", "message": {"id": "b", "content": [
            {"type": "tool_use", "id": "b", "name": "Bash", "input": {"command": command}}]}});
        let result = json!({"type": "result", "is_error": false, "usage": {}});
        transcript::summarize(&[(0, init), (1, call), (2, result)])
    };
    for hidden in [
        "env graft ask \"where is login\"",
        "FOO=bar ripwire . --for=login",
        "env -i PATH=/usr/bin graft grep login",
        "cd /work && command graft ask x",
        "bash -c \"graft ask 'where is login'\"",
        "sh -c 'cd src; ripwire . --for=x'",
        "time nohup ripwire .",
    ] {
        assert!(
            Arm::None.contamination(&bash(hidden)).is_some(),
            "missed: {hidden}"
        );
    }
    for innocent in [
        "rg \"foo|graft ask\" src",
        "grep -n 'a; ripwire .' notes.txt",
        "echo 'graft && ripwire' > /dev/null",
        "FOO=graft cargo test",
    ] {
        assert_eq!(
            Arm::None.contamination(&bash(innocent)),
            None,
            "false alarm: {innocent}"
        );
    }
}

#[test]
fn the_shell_guard_sees_through_option_values_keywords_and_exec_forms() {
    let bash = |command: &str| {
        let init = json!({"type": "system", "subtype": "init", "tools": [], "mcp_servers": []});
        let call = json!({"type": "assistant", "message": {"id": "b", "content": [
            {"type": "tool_use", "id": "b", "name": "Bash", "input": {"command": command}}]}});
        let result = json!({"type": "result", "is_error": false, "usage": {}});
        transcript::summarize(&[(0, init), (1, call), (2, result)])
    };
    for hidden in [
        "timeout 30 graft ask x",
        "timeout -s KILL -k 5 30s graft ask x",
        "nice -n 5 graft ask x",
        "sudo -u me graft ask x",
        "env -u HOME graft ask x",
        "stdbuf -oL graft ask x",
        "bash -lc 'graft ask x'",
        "if graft ask x; then echo ok; fi",
        "for f in a b; do graft ask $f; done",
        "{ graft ask x; }",
        "! graft ask x",
        "while true; do ripwire .; done",
        "echo login | xargs graft ask",
        "xargs -n 1 graft ask < q.txt",
        "find . -name '*.py' -exec graft ask {} \\;",
        "find . -execdir ripwire . +",
    ] {
        assert!(
            Arm::None.contamination(&bash(hidden)).is_some(),
            "missed: {hidden}"
        );
    }
    for innocent in [
        "timeout 30 cargo test graft",
        "find . -name graft",
        "if [ -f graft ]; then echo graft; fi",
        "bash -lc 'echo graft'",
        "xargs grep -n graft < files.txt",
    ] {
        assert_eq!(
            Arm::None.contamination(&bash(innocent)),
            None,
            "false alarm: {innocent}"
        );
    }
}

#[test]
fn a_task_id_that_is_not_a_plain_file_name_is_refused() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corpus.json");
    let task = |id: &str| {
        json!({"id": id, "repo": repo.path(), "base": head, "prompt": "p",
               "reference": {"files": ["src/auth.py"]}})
    };
    std::fs::write(
        &path,
        json!({"tasks": [task("../escape"), task("a/b"), task(""), task(".hidden"),
                         task("fine-1.2_x")]})
        .to_string(),
    )
    .unwrap();

    let errors = Corpus::load(&path).unwrap().validate().unwrap_err();

    let refused = |id: &str| errors.iter().any(|e| e.starts_with(&format!("{id}: id")));
    for bad in ["../escape", "a/b", "", ".hidden"] {
        assert!(refused(bad), "{bad} accepted: {errors:?}");
    }
    assert!(!refused("fine-1.2_x"), "{errors:?}");
}

#[test]
fn a_fix_that_is_not_a_commit_is_refused_before_anything_runs() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corpus.json");
    std::fs::write(
        &path,
        json!({"tasks": [{"id": "typo", "repo": repo.path(), "base": head,
                          "fix": "0000000000000000000000000000000000000000", "prompt": "p",
                          "reference": {"files": ["src/auth.py"]}}]})
        .to_string(),
    )
    .unwrap();

    let errors = Corpus::load(&path).unwrap().validate().unwrap_err();
    assert!(errors.join("\n").contains("typo: fix"), "{errors:?}");
}

#[test]
fn an_agent_edit_to_a_file_setup_touched_still_counts() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let agent = fake_agent(work.path()); // appends a line to src/auth.py
    let corpus = work.path().join("corpus.json");
    std::fs::write(
        &corpus,
        json!({"tasks": [{"id": "t", "repo": repo.path(), "base": head, "prompt": "p",
                          "reference": {"files": ["src/auth.py"]},
                          // Setup touches the very file the agent will edit, and another one.
                          "setup": "echo '# setup' >> src/auth.py && echo x >> tests/test_auth.py"}]})
        .to_string(),
    )
    .unwrap();
    let out = work.path().join("out");
    let agent_cmd = format!("{} --mcp-config {{mcp_config}}", agent.display());

    let (code, _, err) = eval(
        &[
            "run",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--arms",
            "none",
            "--agent-cmd",
            &agent_cmd,
        ],
        &work.path().join("runs"),
    );

    assert_eq!(code, 0, "{err}");
    let r: Value = serde_json::from_str(
        std::fs::read_to_string(out.join("results.jsonl"))
            .unwrap()
            .trim(),
    )
    .unwrap();
    assert_eq!(
        r["file_recall"], 1.0,
        "the agent's edit to src/auth.py is kept: {r}"
    );
    assert_eq!(
        r["file_precision"], 1.0,
        "setup's own edit is not the agent's: {r}"
    );
}

// --- the memory arms (PRD jev-mem §14; T5.1, D-141) ---

#[test]
fn the_memory_arms_parse_and_start_the_right_server() {
    let tools = ripwire_broker::eval::arm::Tools {
        broker: "/bin/rb".into(),
        ripwire: "/bin/rw".into(),
        memory_debug_log: false,
    };
    let (ws, st) = (Path::new("/work"), Path::new("/state"));
    assert_eq!(Arm::parse("broker-memory"), Some(Arm::BrokerMemory));
    assert_eq!(
        Arm::parse("broker-memory-deterministic"),
        Some(Arm::BrokerMemoryDeterministic)
    );
    let server = |arm: Arm| arm.mcp_config(&tools, ws, st)["mcpServers"]["ripwire-broker"].clone();
    let b = server(Arm::BrokerMemory);
    assert_eq!(
        b["args"],
        json!([
            "serve",
            "--workspace",
            "/work",
            "--ripwire",
            "/bin/rw",
            "--memory"
        ])
    );
    let c = server(Arm::BrokerMemoryDeterministic);
    assert_eq!(
        c["args"],
        json!([
            "serve",
            "--workspace",
            "/work",
            "--ripwire",
            "/bin/rw",
            "--memory",
            "--memory-selection",
            "deterministic"
        ])
    );
    for m in [&b, &c] {
        assert_eq!(
            m["env"],
            json!({"XDG_STATE_HOME": "/state"}),
            "its own store"
        );
    }
    for arm in [Arm::Broker, Arm::BrokerOnline] {
        assert!(server(arm).get("env").is_none(), "{}", arm.name());
    }
    for arm in ripwire_broker::eval::arm::ALL {
        let online = matches!(
            arm,
            Arm::BrokerOnline | Arm::BrokerMemory | Arm::BrokerMemoryDeterministic
        );
        assert_eq!(arm.needs_credential(), online, "{}", arm.name());
    }
}

#[test]
fn contamination_is_detected_for_the_new_arms() {
    let s = fixture();
    for arm in [Arm::BrokerMemory, Arm::BrokerMemoryDeterministic] {
        assert_eq!(arm.contamination(&s), None, "{}", arm.name());
        let init = json!({"type": "system", "subtype": "init", "tools": ["Read"],
                          "mcp_servers": [{"name": "ripwire-broker", "status": "failed"}]});
        let result = json!({"type": "result", "is_error": false, "usage": {}});
        let down = transcript::summarize(&[(0, init), (1, result)]);
        assert!(arm.contamination(&down).unwrap().contains("not connected"));
        let init = json!({"type": "system", "subtype": "init", "tools": ["Read"],
                          "mcp_servers": [{"name": "ripwire-broker", "status": "connected"},
                                          {"name": "ripwire", "status": "connected"}]});
        let result = json!({"type": "result", "is_error": false, "usage": {}});
        let foreign = transcript::summarize(&[(0, init), (1, result)]);
        assert!(arm.contamination(&foreign).unwrap().contains("ripwire"));
    }
}

/// Like `fake_agent`, and logs one line per run before editing: its directory, the store its
/// server was given (`-` without one), and whether the copy already had the edit.
fn logging_agent(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("logging-agent");
    common::write_executable(
        &path,
        r##"#!/bin/sh
prompt=$(cat)
cfg=""
while [ $# -gt 0 ]; do
  if [ "$1" = "--mcp-config" ]; then cfg="$2"; shift; fi
  shift
done
state=$(sed -n 's/.*"XDG_STATE_HOME":"\([^"]*\)".*/\1/p' "$cfg")
before=$(grep -c 'expired tokens' src/auth.py)
dirty=$(git status --porcelain | wc -l | tr -d ' ')
commits=$(git rev-list --count HEAD)
echo "$(pwd -P) ${state:--} $before $dirty $commits" >> "$LOG"
# What the server would leave there.
if [ -n "$state" ]; then mkdir -p "$state/ripwire-broker"; fi
# What a server run with --memory-debug-log would write.
if [ -n "$state" ] && grep -q -- '--memory-debug-log' "$cfg"; then
  mkdir -p "$state/ripwire-broker/memory/w1"
  echo "read kept=0 for: $prompt" >> "$state/ripwire-broker/memory/w1/debug.log"
fi
# A session that dies before its result.
case "$prompt" in *BREAK*) echo '{"type":"system","subtype":"init","tools":[],"mcp_servers":[]}'; exit 0;; esac
servers='[{"name":"ripwire-broker","status":"connected"}]'
echo "{\"type\":\"system\",\"subtype\":\"init\",\"tools\":[\"Read\",\"Edit\"],\"mcp_servers\":$servers}"
# A session that changes nothing.
case "$prompt" in *IDLE*) ;; *)
echo '{"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"t2","name":"Edit","input":{"file_path":"src/auth.py"}}]}}'
echo "# expired tokens are rejected" >> src/auth.py
;; esac
echo '{"type":"result","subtype":"success","is_error":false,"duration_ms":5,"total_cost_usd":0.01,"usage":{"input_tokens":10,"output_tokens":5}}'
"##,
    );
    path
}

/// Runs `ripwire-eval run` over `tasks` with the logging agent; returns each run's log line
/// (directory, store, edit already there) beside its record, in run order.
fn logged_run(tasks: Value, arms: &str, repeats: &str) -> Vec<(Vec<String>, Value)> {
    let work = tempfile::tempdir().unwrap();
    let agent = logging_agent(work.path());
    let (log, corpus, out) = (
        work.path().join("log"),
        work.path().join("corpus.json"),
        work.path().join("out"),
    );
    std::fs::write(&corpus, json!({ "tasks": tasks }).to_string()).unwrap();
    let agent_cmd = format!("{} --mcp-config {{mcp_config}}", agent.display());
    let status = Command::new(env!("CARGO_BIN_EXE_ripwire-eval"))
        .args([
            "run",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--arms",
            arms,
            "--repeats",
            repeats,
            "--agent-cmd",
            &agent_cmd,
        ])
        .env("LOG", &log)
        // The memory arms imply online; the stand-in agent never uses it.
        .env("RIPWIRE_BROKER_JEV_API_KEY", "synthetic-not-a-credential")
        .output()
        .unwrap();
    assert!(status.status.success(), "{status:?}");
    let lines: Vec<Vec<String>> = std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(|l| l.split(' ').map(String::from).collect())
        .collect();
    let records: Vec<Value> = std::fs::read_to_string(out.join("results.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), records.len());
    lines.into_iter().zip(records).collect()
}

fn auth_task(id: &str, repo: &Path, base: &str, sequence: Option<&str>) -> Value {
    let mut t = json!({
        "id": id, "repo": repo, "base": base,
        "prompt": "make login reject expired tokens",
        "reference": {"files": ["src/auth.py"]},
    });
    if let Some(s) = sequence {
        t["sequence"] = json!(s);
    }
    t
}

#[test]
fn each_round_gets_an_isolated_store() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let runs = logged_run(
        json!([auth_task("t", repo.path(), &head, None)]),
        "broker,broker-memory,broker-memory-deterministic",
        "2",
    );
    assert_eq!(runs.len(), 6);
    let mut stores = std::collections::BTreeSet::new();
    for (line, r) in &runs {
        let store = &line[1];
        if r["arm"] == "broker" {
            assert_eq!(store, "-", "no store for an arm without memory");
            continue;
        }
        assert!(stores.insert(store.clone()), "a store of its own: {store}");
        assert!(
            !Path::new(store).exists(),
            "removed with its round: {store}"
        );
        // The round's place holds the copy (`work`) and, beside it, the server's state.
        assert!(
            store.ends_with("/state") && line[0].ends_with("/work"),
            "{line:?}"
        );
    }
    assert_eq!(stores.len(), 4, "two memory arms, two repeats");
}

#[test]
fn a_sequence_runs_in_order_in_one_place_and_shares_its_store() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let runs = logged_run(
        json!([
            auth_task("first", repo.path(), &head, Some("login")),
            auth_task("alone", repo.path(), &head, None),
            auth_task("second", repo.path(), &head, Some("login")),
        ]),
        "broker-memory,broker-memory-deterministic",
        "1",
    );
    let order: Vec<(String, String)> = runs
        .iter()
        .map(|(_, r)| {
            (
                r["task"].as_str().unwrap().into(),
                r["arm"].as_str().unwrap().into(),
            )
        })
        .collect();
    let at = |task: &str, arm: &str| {
        order
            .iter()
            .position(|(t, a)| t == task && a == arm)
            .unwrap_or_else(|| panic!("{task} {arm} ran"))
    };
    for arm in ["broker-memory", "broker-memory-deterministic"] {
        let (first, second) = (at("first", arm), at("second", arm));
        assert_eq!(second, first + 1, "{arm}: in order, one after the other");
        let (a, b) = (&runs[first].0, &runs[second].0);
        assert_eq!(a[0], b[0], "{arm}: the same place, so the same workspace");
        assert_eq!(a[1], b[1], "{arm}: the same store");
        assert_eq!(
            b[2], "0",
            "{arm}: the second session starts from its own base"
        );
        let alone = &runs[at("alone", arm)].0;
        assert_ne!(
            alone[1], a[1],
            "{arm}: a task outside the sequence has its own store"
        );
    }
    let (b, c) = (
        &runs[at("first", "broker-memory")].0,
        &runs[at("first", "broker-memory-deterministic")].0,
    );
    assert_ne!(b[1], c[1], "each arm its own store");
}

// --- the report's memory cost (PRD jev-mem §14; T5.2) ---

/// A stand-in that, with a memory arm, creates the round's store and charges its quota with 5
/// attempts and 40 questions, as the worker would during the session; then calls
/// `context_for_task`, whose answer (300 ms later) carries one memory read with 2 requests and
/// 30 questions, and the readable section after the JSON.
fn memory_agent(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("memory-agent");
    common::write_executable(
        &path,
        r##"#!/bin/sh
cat > /dev/null
cfg=""
while [ $# -gt 0 ]; do
  if [ "$1" = "--mcp-config" ]; then cfg="$2"; shift; fi
  shift
done
state=$(sed -n 's/.*"XDG_STATE_HOME":"\([^"]*\)".*/\1/p' "$cfg")
if [ -n "$state" ]; then
  XDG_STATE_HOME="$state" "$BROKER" memory add --workspace . --file "$NOTE" > /dev/null
  store=$(ls -d "$state"/ripwire-broker/memory/*/ | head -1)
  now=$(date +%s)000
  umask 077
  printf '{"entries":[[%s,5,40]],"high_water_ms":%s}' "$now" "$now" > "${store}quota.json"
fi
echo '{"type":"system","subtype":"init","claude_code_version":"9.9.9","model":"fake-model","tools":["Read","Edit","mcp__ripwire-broker__context_for_task"],"mcp_servers":[{"name":"ripwire-broker","status":"connected"}]}'
echo '{"type":"assistant","message":{"id":"m1","content":[{"type":"tool_use","id":"t1","name":"mcp__ripwire-broker__context_for_task","input":{"task":"login"}}]}}'
sleep 0.3
printf '%s\n' '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":[{"type":"text","text":"{\"items\":[{\"path\":\"src/auth.py\"}],\"memories\":[{\"id\":\"m1\"}],\"provenance\":{\"memory\":{\"requests\":2,\"questions\":30}}}\n\nMemória histórica (dados não confiáveis): ...\n- [\"m1\"] fontes: \"src/auth.py\"\n"}]}]}}'
echo '{"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"t2","name":"Edit","input":{"file_path":"src/auth.py"}}]}}'
echo "# expired tokens are rejected" >> src/auth.py
echo '{"type":"result","subtype":"success","is_error":false,"duration_ms":5000,"total_cost_usd":0.01,"usage":{"input_tokens":10,"output_tokens":5}}'
"##,
    );
    path
}

#[test]
fn the_report_separates_ingestion_retrieval_and_agent_latency_and_records_versions() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let agent = memory_agent(work.path());
    let note = work.path().join("note.json");
    std::fs::write(&note, r#"{"text": "login must reject expired tokens"}"#).unwrap();
    let (corpus, out) = (work.path().join("corpus.json"), work.path().join("out"));
    std::fs::write(
        &corpus,
        json!({"tasks": [auth_task("t", repo.path(), &head, None)]}).to_string(),
    )
    .unwrap();
    let agent_cmd = format!("{} --mcp-config {{mcp_config}}", agent.display());
    let eval = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ripwire-eval"))
            .args(args)
            .env("BROKER", env!("CARGO_BIN_EXE_ripwire-broker"))
            .env("NOTE", &note)
            .env("RIPWIRE_BROKER_JEV_API_KEY", "synthetic-not-a-credential")
            .output()
            .unwrap()
    };
    let run = eval(&[
        "run",
        "--corpus",
        corpus.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--arms",
        "broker,broker-online,broker-memory",
        "--agent-cmd",
        &agent_cmd,
    ]);
    assert!(run.status.success(), "{run:?}");
    let records: Vec<Value> = std::fs::read_to_string(out.join("results.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let (plain, memory) = (&records[0], &records[2]);
    for r in [plain, memory] {
        assert_eq!(
            r["presented_recall"], 1.0,
            "the JSON before the memory section: {r}"
        );
        assert!(r["context_for_task_ms"].as_u64().unwrap() >= 300, "{r}");
        assert_eq!(r["duration_ms"], 5000, "the agent's own latency: {r}");
    }
    for k in [
        "memory_retrieval_requests",
        "memory_ingestion_attempts",
        "memories_delivered",
    ] {
        assert!(
            plain.get(k).is_none(),
            "an arm without memory has no {k}, not a zero"
        );
    }
    assert_eq!(
        (
            &memory["memory_retrieval_requests"],
            &memory["memory_retrieval_questions"],
            &memory["memories_delivered"]
        ),
        (&json!(2), &json!(30), &json!(1))
    );
    assert_eq!(
        (
            &memory["memory_ingestion_attempts"],
            &memory["memory_ingestion_questions"]
        ),
        (&json!(3), &json!(10)),
        "what the store spent, less what the reads did: {memory}"
    );
    // The note `memory add` left is a job the session's worker did not run: the next session's
    // server will, and pay for it there.
    assert_eq!(memory["memory_jobs_left"], 1, "{memory}");
    assert!(plain.get("memory_jobs_left").is_none());

    let versions: Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("versions.json")).unwrap()).unwrap();
    let broker = format!("ripwire-broker {}", env!("CARGO_PKG_VERSION"));
    assert_eq!(versions["ripwire_broker"], broker.as_str());
    assert_eq!(versions["jev_model"], "jev-1.13.0");
    assert_eq!(versions["summarizer"], "none");
    assert!(versions["ripwire"].is_string(), "{versions}");
    assert_eq!(
        memory["agent"], "9.9.9 fake-model",
        "as the session announced it"
    );

    let report = eval(&["report", "--out", out.to_str().unwrap()]);
    let md = String::from_utf8_lossy(&report.stdout);
    assert!(md.contains("## Custo da memória"), "{md}");
    let row = md
        .lines()
        .find(|l| l.starts_with("| broker-memory |") && l.contains("3"))
        .unwrap_or_else(|| panic!("a memory cost row: {md}"));
    let cells: Vec<&str> = row
        .split('|')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect();
    assert_eq!(
        cells,
        [
            "broker-memory",
            "1",
            "2.000",
            "30.000",
            "1.000",
            cells[5],
            "3.000",
            "10.000",
            "1.000",
            "5000.000"
        ],
        "{row}"
    );
    assert!(
        md.contains("jobs pendentes"),
        "the caveat is in the report: {md}"
    );
    // Arm A beside them, for its wait on context_for_task; no memory column, never a zero.
    let section = md.split("## Custo da memória").nth(1).unwrap();
    let section = section.split("\n## ").next().unwrap();
    let a = section
        .lines()
        .find(|l| l.starts_with("| broker-online |"))
        .unwrap_or_else(|| panic!("arm A in the memory cost table: {md}"));
    let a: Vec<&str> = a
        .split('|')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect();
    assert_eq!(a[2], "—", "{a:?}");
    assert_ne!(a[5], "—", "its context_for_task wait: {a:?}");

    let json = eval(&["report", "--out", out.to_str().unwrap(), "--json"]);
    let json: Value = serde_json::from_slice(&json.stdout).unwrap();
    let b = json["memory_cost"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["arm"] == "broker-memory")
        .expect("arm B in the JSON report");
    assert_eq!(b["ingestion_attempts"], 3.0, "{b}");
    assert_eq!(
        json["versions"]["ripwire_broker"],
        versions["ripwire_broker"]
    );
    assert!(md.contains("## Versões") && md.contains(&broker), "{md}");
    assert!(md.contains("- agente: `9.9.9 fake-model`"), "{md}");
}

/// `ripwire-eval run` of `tasks` with the logging agent, in `work` (its corpus, `out/` and
/// `log` there), so that a second call resumes the first.
fn run_in(work: &Path, tasks: Value, arms: &str, extra: &[&str]) -> std::process::Output {
    let agent = logging_agent(work);
    let corpus = work.join("corpus.json");
    std::fs::write(&corpus, json!({ "tasks": tasks }).to_string()).unwrap();
    let agent_cmd = format!("{} --mcp-config {{mcp_config}}", agent.display());
    let mut args = vec![
        "run".to_string(),
        "--corpus".into(),
        corpus.to_string_lossy().into(),
        "--out".into(),
        work.join("out").to_string_lossy().into(),
        "--arms".into(),
        arms.into(),
        "--agent-cmd".into(),
        agent_cmd,
    ];
    args.extend(extra.iter().map(|a| a.to_string()));
    Command::new(env!("CARGO_BIN_EXE_ripwire-eval"))
        .args(&args)
        .env("LOG", work.join("log"))
        .env("RIPWIRE_BROKER_JEV_API_KEY", "synthetic-not-a-credential")
        .output()
        .unwrap()
}

fn logged(work: &Path) -> usize {
    std::fs::read_to_string(work.join("log")).map_or(0, |l| l.lines().count())
}

#[test]
fn a_sequence_partly_recorded_is_refused_not_resumed_in_part() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let two = json!([
        auth_task("first", repo.path(), &head, Some("login")),
        auth_task("second", repo.path(), &head, Some("login")),
    ]);
    assert!(
        run_in(work.path(), two.clone(), "broker-memory", &[])
            .status
            .success()
    );
    assert_eq!(logged(work.path()), 2);
    // Run again: recorded whole, nothing reruns.
    assert!(
        run_in(work.path(), two, "broker-memory", &[])
            .status
            .success()
    );
    assert_eq!(logged(work.path()), 2);

    // A third session joins the sequence: its history would be missing, so the run stops.
    let three = json!([
        auth_task("first", repo.path(), &head, Some("login")),
        auth_task("second", repo.path(), &head, Some("login")),
        auth_task("third", repo.path(), &head, Some("login")),
    ]);
    let out = run_in(work.path(), three, "broker-memory", &[]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("login") && err.contains("third"), "{err}");
    assert_eq!(logged(work.path()), 2, "nothing ran");
}

#[test]
fn a_sequence_stays_in_one_repository() {
    let (a, b) = (common::sample_repo(), common::sample_repo());
    let head = |r: &tempfile::TempDir| git(r.path(), &["rev-parse", "HEAD"]);
    let corpus: Corpus = serde_json::from_value(json!({"tasks": [
        auth_task("one", a.path(), &head(&a), Some("s")),
        auth_task("two", b.path(), &head(&b), Some("s")),
        auth_task("three", a.path(), &head(&a), Some("")),
    ]}))
    .unwrap();
    let errors = corpus.validate().unwrap_err().join("\n");
    assert!(
        errors.contains("two") && errors.contains("sequence s"),
        "{errors}"
    );
    assert!(
        errors.contains("three") && errors.contains("empty sequence"),
        "{errors}"
    );
}

#[test]
fn a_session_after_an_invalid_one_says_its_history_is_incomplete() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let mut first = auth_task("first", repo.path(), &head, Some("login"));
    first["prompt"] = json!("BREAK before the result");
    let runs = logged_run(
        json!([
            first,
            auth_task("second", repo.path(), &head, Some("login"))
        ]),
        "broker-memory",
        "1",
    );
    let (first, second) = (&runs[0].1, &runs[1].1);
    assert_eq!(first["valid"], false, "{first}");
    assert!(first.get("history_incomplete").is_none(), "{first}");
    assert_eq!(second["valid"], true, "{second}");
    assert_eq!(second["history_incomplete"], true, "{second}");
}

#[test]
fn only_a_memory_arm_carries_a_history() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let mut first = auth_task("first", repo.path(), &head, Some("login"));
    first["prompt"] = json!("BREAK before the result");
    let runs = logged_run(
        json!([
            first,
            auth_task("second", repo.path(), &head, Some("login"))
        ]),
        "broker",
        "1",
    );
    let second = &runs[1].1;
    assert_eq!(second["valid"], true, "{second}");
    assert!(second.get("history_incomplete").is_none(), "{second}");
}

#[test]
fn a_session_with_an_incomplete_history_stays_out_of_the_averages() {
    let whole = rec(0, 1, Arm::BrokerMemory, 100, 1);
    let mut partial = rec(1, 1, Arm::BrokerMemory, 900, 1);
    partial.history_incomplete = true;
    let records = [whole, partial];

    let stats = report::arm_stats(&records, Arm::BrokerMemory.name());
    assert_eq!((stats.runs, stats.valid), (2, 1));
    assert_eq!(stats.tokens, Some(100.0));
    let md = report::render(&records);
    assert!(md.contains("histórico incompleto"), "{md}");
    assert!(md.contains("`t1`"), "{md}");
}

#[test]
fn versions_that_change_between_runs_are_refused() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let tasks = json!([auth_task("t", repo.path(), &head, None)]);
    assert!(
        run_in(work.path(), tasks.clone(), "none", &[])
            .status
            .success()
    );
    let other = work.path().join("other-broker");
    common::write_executable(&other, "#!/bin/sh\necho 'ripwire-broker 9.9.9'\n");
    let out = run_in(
        work.path(),
        tasks,
        "none",
        &["--repeats", "2", "--broker", other.to_str().unwrap()],
    );
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("versions.json") && err.contains("9.9.9"),
        "{err}"
    );
    assert_eq!(logged(work.path()), 1, "the second repeat did not run");
}

// --- audit of 2026-10-04 (D-143) ---

#[test]
fn a_corpus_given_by_a_relative_path_still_finds_its_repositories() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(repo.path(), work.path().join("repo")).unwrap();
    std::fs::write(
        work.path().join("corpus.json"),
        json!({"tasks": [auth_task("t", Path::new("repo"), &head, None)]}).to_string(),
    )
    .unwrap();
    // As the README shows it: from the corpus's own directory, by a relative path.
    let agent = logging_agent(work.path());
    let out = Command::new(env!("CARGO_BIN_EXE_ripwire-eval"))
        .current_dir(work.path())
        .args([
            "run",
            "--corpus",
            "corpus.json",
            "--out",
            "out",
            "--arms",
            "broker",
            "--agent-cmd",
            &format!("{} --mcp-config {{mcp_config}}", agent.display()),
        ])
        .env("LOG", work.path().join("log"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let record: Value = serde_json::from_str(
        std::fs::read_to_string(work.path().join("out/results.jsonl"))
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(record["valid"], true, "the copy was made: {record}");
}

/// A stand-in agent that edits the task's file, adds one with a non-ASCII name, and commits
/// both, as an agent allowed to run git may.
fn committing_agent(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("committing-agent");
    common::write_executable(
        &path,
        r##"#!/bin/sh
cat > /dev/null
echo '{"type":"system","subtype":"init","tools":["Read","Edit"],"mcp_servers":[{"name":"ripwire-broker","status":"connected"}]}'
echo "# expired tokens are rejected" >> src/auth.py
mkdir -p docs && echo "notes" > "docs/ação.md"
git add -A && git -c user.email=a@example.invalid -c user.name=agent commit -qm done
echo '{"type":"result","subtype":"success","is_error":false,"duration_ms":5,"total_cost_usd":0.01,"usage":{"input_tokens":10,"output_tokens":5}}'
"##,
    );
    path
}

#[test]
fn edits_the_agent_committed_still_count_and_any_file_name_matches() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let mut task = auth_task("t", repo.path(), &head, None);
    task["reference"]["files"] = json!(["src/auth.py", "docs/ação.md"]);
    std::fs::write(
        work.path().join("corpus.json"),
        json!({ "tasks": [task] }).to_string(),
    )
    .unwrap();
    let agent = committing_agent(work.path());
    let out = Command::new(env!("CARGO_BIN_EXE_ripwire-eval"))
        .args([
            "run",
            "--corpus",
            work.path().join("corpus.json").to_str().unwrap(),
            "--out",
            work.path().join("out").to_str().unwrap(),
            "--arms",
            "broker",
            "--agent-cmd",
            &format!("{} --mcp-config {{mcp_config}}", agent.display()),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let record: Value = serde_json::from_str(
        std::fs::read_to_string(work.path().join("out/results.jsonl"))
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        (&record["file_recall"], &record["file_precision"]),
        (&json!(1.0), &json!(1.0)),
        "measured against the task's base, names exact: {record}"
    );
}

/// #122: a killed process stays a zombie until someone reaps it, and `kill -0` still finds a
/// zombie. Whether it is reaped in time is up to whoever adopted it, not the code under test.
#[test]
fn a_killed_process_not_yet_reaped_counts_as_gone() {
    let mut child = KillOnDrop::spawn("sleep", &["30"]);
    let pid = child.0.id().to_string();
    child.0.kill().unwrap(); // SIGKILL, and no wait() yet: a zombie, reaped on drop
    assert!(
        common::wait_gone(&pid, std::time::Duration::from_secs(1)),
        "a killed process counts as gone even before it is reaped"
    );
}

#[test]
fn a_live_process_is_not_gone() {
    let child = KillOnDrop::spawn("sleep", &["30"]);
    assert!(
        !common::wait_gone(
            &child.0.id().to_string(),
            std::time::Duration::from_millis(200)
        ),
        "a running process is still there"
    );
}

/// `ps -p ''` prints nothing, which reads as gone: an empty pidfile would let the check above
/// pass with nothing killed. A pid that is not a number is a bug in the test, not an answer.
#[test]
#[should_panic(expected = "not a pid")]
fn a_pid_that_is_not_a_number_is_refused() {
    common::wait_gone("", std::time::Duration::from_millis(100));
}

/// A helper process, killed and reaped when dropped, so an assertion that fails cannot leave it
/// behind. It gets no stdio of the test's: holding the test's stdout open is what nextest reports
/// as a `LEAK` (#122), and that must not depend on Drop running.
struct KillOnDrop(std::process::Child);

impl KillOnDrop {
    fn spawn(program: &str, args: &[&str]) -> Self {
        use std::process::Stdio;
        Self(
            Command::new(program)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    }
}

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn a_timeout_ends_the_agent_and_the_check_with_everything_they_started() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    // An agent whose child keeps its stdout open, and a check that leaves a process behind.
    let agent = work.path().join("hanging-agent");
    common::write_executable(
        &agent,
        r##"#!/bin/sh
cat > /dev/null
echo '{"type":"system","subtype":"init","tools":[],"mcp_servers":[]}'
sleep 30
"##,
    );
    let pidfile = work.path().join("check.pid");
    let mut task = auth_task("t", repo.path(), &head, None);
    task["check"] = json!(format!("sleep 30 & echo $! > {}; wait", pidfile.display()));
    std::fs::write(
        work.path().join("corpus.json"),
        json!({ "tasks": [task] }).to_string(),
    )
    .unwrap();
    // A process outside those groups, which no timeout may touch.
    let mut witness = KillOnDrop::spawn("sleep", &["60"]);
    let started = std::time::Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_ripwire-eval"))
        .args([
            "run",
            "--corpus",
            work.path().join("corpus.json").to_str().unwrap(),
            "--out",
            work.path().join("out").to_str().unwrap(),
            "--arms",
            "none",
            "--timeout-s",
            "1",
            "--check-timeout-s",
            "1",
            "--agent-cmd",
            agent.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(15),
        "the run ended at its timeouts, not when the children did: {:?}",
        started.elapsed()
    );
    let pid = std::fs::read_to_string(&pidfile).unwrap();
    assert!(
        common::wait_gone(pid.trim(), std::time::Duration::from_secs(2)),
        "the check's process went with it"
    );
    assert!(
        witness.0.try_wait().unwrap().is_none(),
        "nothing outside the timed-out groups was killed"
    );
}

#[test]
fn an_agent_that_never_reads_a_long_prompt_still_times_out() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    // Larger than a pipe's buffer: writing it all blocks until the agent reads.
    let agent = work.path().join("deaf-agent");
    common::write_executable(&agent, "#!/bin/sh\nexec sleep 30\n");
    let mut task = auth_task("t", repo.path(), &head, None);
    task["prompt"] = json!("x".repeat(1 << 20));
    std::fs::write(
        work.path().join("corpus.json"),
        json!({ "tasks": [task] }).to_string(),
    )
    .unwrap();
    let started = std::time::Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_ripwire-eval"))
        .args([
            "run",
            "--corpus",
            work.path().join("corpus.json").to_str().unwrap(),
            "--out",
            work.path().join("out").to_str().unwrap(),
            "--arms",
            "none",
            "--timeout-s",
            "1",
            "--agent-cmd",
            agent.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(15),
        "the prompt held the run past its timeout: {:?}",
        started.elapsed()
    );
    let results = std::fs::read_to_string(work.path().join("out/results.jsonl")).unwrap();
    assert!(results.contains("timed out"), "{results}");
}

#[test]
fn the_agent_is_never_handed_the_source_repository_or_the_fix() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let task = |env: Value| {
        let mut t = auth_task("t", repo.path(), &head, None);
        t["env"] = env;
        t
    };
    // `env` reaches the agent: with the source repository's path, `git log --all` there is the
    // answer. Commands may name them; `env` may not.
    for (var, value) in [("SRC", "{repo}"), ("ANSWER", "x{fix}y")] {
        let corpus: Corpus =
            serde_json::from_value(json!({"tasks": [task(json!({ var: value }))]})).unwrap();
        let errors = corpus.validate().unwrap_err().join("\n");
        assert!(errors.contains(var) && errors.contains("env"), "{errors}");
    }
    let run_only: Corpus =
        serde_json::from_value(json!({"tasks": [task(json!({"DB": "db_{run}"}))]})).unwrap();
    assert_eq!(run_only.validate(), Ok(()));
}

#[test]
fn the_bars_compare_the_same_tasks_and_skip_what_has_no_reference_tests() {
    // Thirty tasks where the broker saves 30% of the tokens (short of the 35% bar), and one more
    // that only `none` ran validly, an expensive one: unpaired, it would carry the bar.
    let mut records = corpus(30, 3, 700, 70);
    records.push(rec(30, 3, Arm::None, 100_000, 100));
    let mut lost = rec(30, 3, Arm::Broker, 10, 1);
    lost.valid = false;
    records.push(lost);
    assert_eq!(
        verdict(&records, "17.1"),
        Verdict::Fail,
        "compared over the tasks both arms ran validly"
    );

    // A task whose reference names no tests says nothing about finding tests.
    let mut no_tests = rec(0, 1, Arm::Broker, 1, 1);
    no_tests.test_recall = None;
    let mut half = rec(1, 1, Arm::Broker, 1, 1);
    half.test_recall = Some(0.5);
    let stats = report::arm_stats(&[no_tests, half], "broker");
    assert_eq!(stats.test_recall, Some(0.5));

    // A broker run that presented nothing presented a recall of 0, in the table as in the bar;
    // an arm without the broker has no such measure.
    let mut nothing = rec(0, 1, Arm::Broker, 1, 1);
    nothing.presented_recall = None;
    let mut all = rec(1, 1, Arm::Broker, 1, 1);
    all.presented_recall = Some(1.0);
    assert_eq!(
        report::arm_stats(&[nothing, all], "broker").presented_recall,
        Some(0.5)
    );
    let mut plain = rec(0, 1, Arm::None, 1, 1);
    plain.presented_recall = None;
    assert_eq!(report::arm_stats(&[plain], "none").presented_recall, None);
}

// --- sessions of one history (PRD jev-mem §14; T5.3) ---

fn continuing(mut task: Value) -> Value {
    task["continues"] = json!(true);
    task
}

#[test]
fn a_session_that_continues_starts_from_what_the_previous_one_left() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let commits: u32 = git(repo.path(), &["rev-list", "--count", "HEAD"])
        .parse()
        .unwrap();
    let mut idle = continuing(auth_task("third", repo.path(), &head, Some("login")));
    idle["prompt"] = json!("IDLE: nothing to change");
    let runs = logged_run(
        json!([
            auth_task("first", repo.path(), &head, Some("login")),
            continuing(auth_task("second", repo.path(), &head, Some("login"))),
            idle,
        ]),
        "broker,broker-memory",
        "1",
    );
    for arm in ["broker", "broker-memory"] {
        let of: Vec<&(Vec<String>, Value)> = runs.iter().filter(|(_, r)| r["arm"] == arm).collect();
        let (first, second, third) = (of[0], of[1], of[2]);
        assert_eq!(first.0[2], "0", "{arm}: the first starts at its base");
        assert_eq!(second.0[0], first.0[0], "{arm}: the same place");
        assert_eq!(second.0[2], "1", "{arm}: the first session's edit is there");
        assert_eq!(
            second.0[3], "0",
            "{arm}: committed, so the tree starts clean"
        );
        assert_eq!(
            second.0[4],
            (commits + 1).to_string(),
            "{arm}: one commit per earlier session"
        );
        assert_eq!(third.0[2], "2", "{arm}: and the second's");
        assert_eq!(third.0[4], (commits + 2).to_string(), "{arm}");
        // What a session changed is measured from where it started, not from the task's base.
        assert_eq!(second.1["file_recall"], 1.0, "{arm}: {}", second.1);
        assert_eq!(third.1["file_recall"], 0.0, "{arm}: {}", third.1);
        // The broker's arms say whether the agent closed with a note; this stand-in never does.
        assert_eq!(first.1["finish_calls"], 0, "{arm}: {}", first.1);
        assert_eq!(first.1["finish_notes"], 0, "{arm}: {}", first.1);
    }
    let source = git(repo.path(), &["status", "--porcelain"]);
    assert_eq!(source, "", "the source repository is never touched");
}

#[test]
fn a_session_can_only_continue_an_earlier_one_of_its_sequence() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let out = run_in(
        work.path(),
        json!([
            continuing(auth_task("alone", repo.path(), &head, None)),
            continuing(auth_task("opens", repo.path(), &head, Some("s"))),
            continuing(auth_task("follows", repo.path(), &head, Some("s"))),
        ]),
        "broker",
        &[],
    );
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("alone: continues"), "{err}");
    assert!(err.contains("opens: continues"), "{err}");
    assert!(!err.contains("follows"), "{err}");
    assert_eq!(logged(work.path()), 0, "nothing ran");
}

#[test]
fn a_continued_history_is_incomplete_in_any_arm_after_an_invalid_session() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let mut first = auth_task("first", repo.path(), &head, Some("login"));
    first["prompt"] = json!("BREAK before the result");
    let runs = logged_run(
        json!([
            first,
            continuing(auth_task("second", repo.path(), &head, Some("login")))
        ]),
        "broker",
        "1",
    );
    assert_eq!(runs[1].1["history_incomplete"], true, "{}", runs[1].1);
}

/// A stand-in broker: every call goes to `$LOG` as `BROKER <state> <args>`; `memory status`
/// reports one observation waiting until `memory drain` has run.
fn draining_broker(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("draining-broker");
    let drained = dir.join("drained");
    common::write_executable(
        &path,
        format!(
            r##"#!/bin/sh
case "$1" in --version) echo "stand-in 0"; exit 0;; esac
echo "BROKER ${{XDG_STATE_HOME:--}} $*" >> "$LOG"
case "$1 $2" in
  "memory drain") touch '{d}';;
  "memory status")
    pending=1; [ -e '{d}' ] && pending=0
    echo "{{\"attempts_24h\":0,\"questions_24h\":0,\"pending\":$pending,\"jobs_pending\":0}}";;
esac
"##,
            d = drained.display()
        ),
    );
    path
}

#[test]
fn the_classifier_arm_drains_its_memory_before_the_next_session() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let broker = draining_broker(work.path());
    let out = run_in(
        work.path(),
        json!([
            auth_task("first", repo.path(), &head, Some("login")),
            auth_task("second", repo.path(), &head, Some("login")),
        ]),
        "broker-online,broker-memory,broker-memory-deterministic",
        &["--broker", broker.to_str().unwrap()],
    );
    assert!(out.status.success(), "{out:?}");
    let log = std::fs::read_to_string(work.path().join("log")).unwrap();
    // Sessions and drains, in the order they happened.
    let steps: Vec<&str> = log
        .lines()
        .filter(|l| !l.starts_with("BROKER") || l.contains(" memory drain "))
        .collect();
    let drains: Vec<(usize, &&str)> = steps
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("BROKER"))
        .collect();
    assert_eq!(
        drains.len(),
        1,
        "one drain, in the classifier's arm: {steps:#?}"
    );
    let (at, drain) = drains[0];
    // online: 0, 1; memory: 2, drain, 4.
    assert_eq!(at, 3, "between the arm's two sessions: {steps:#?}");
    let session: Vec<&str> = steps[2].split(' ').collect();
    // The agent logs its directory resolved (`pwd -P`: `/private/var` on macOS); the round's
    // place is gone by now, so the two are compared as text.
    let real = |p: &str| p.trim_start_matches("/private").to_string();
    let said: Vec<&str> = drain.split(' ').collect();
    assert_eq!(
        said[2..],
        ["memory", "drain", "--workspace", said[5], "--online"],
        "{drain}"
    );
    assert_eq!(
        real(said[5]),
        real(session[0]),
        "the session's own workspace"
    );
    assert_eq!(said[1], session[1], "and its store");
    let records: Vec<Value> = std::fs::read_to_string(work.path().join("out/results.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let left = |task: &str, arm: &str| {
        records
            .iter()
            .find(|r| r["task"] == task && r["arm"] == arm)
            .map(|r| r["memory_jobs_left"].clone())
            .unwrap()
    };
    assert_eq!(left("first", "broker-memory"), 0, "counted after the drain");
}

/// The arm order of each task·repeat, as recorded, from a run with `extra` flags.
fn arm_orders(extra: &[&str]) -> Vec<Vec<String>> {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let mut args = vec!["--repeats", "6"];
    args.extend(extra);
    let out = run_in(
        work.path(),
        json!([auth_task("t", repo.path(), &head, None)]),
        "none,broker",
        &args,
    );
    assert!(out.status.success(), "{out:?}");
    let records: Vec<Value> = std::fs::read_to_string(work.path().join("out/results.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(records.len(), 12);
    records
        .chunks(2)
        .map(|pair| {
            assert_eq!(pair[0]["repeat"], pair[1]["repeat"], "a repeat runs whole");
            for (i, r) in pair.iter().enumerate() {
                assert_eq!(r["position"], i, "its place in the repeat: {r}");
                let finish = r["arm"] == "broker";
                assert_eq!(r.get("finish_calls").is_some(), finish, "{r}");
            }
            pair.iter()
                .map(|r| r["arm"].as_str().unwrap().to_string())
                .collect()
        })
        .collect()
}

#[test]
fn a_seed_shuffles_the_arms_of_each_repeat_and_gives_the_same_order_again() {
    let given = vec!["none".to_string(), "broker".to_string()];
    let plain = arm_orders(&[]);
    assert!(plain.iter().all(|o| *o == given), "as given: {plain:?}");

    let shuffled = arm_orders(&["--shuffle", "1"]);
    assert!(shuffled.contains(&given), "{shuffled:?}");
    assert!(shuffled.iter().any(|o| *o != given), "{shuffled:?}");
    assert_eq!(
        arm_orders(&["--shuffle", "1"]),
        shuffled,
        "the seed decides"
    );
    assert_ne!(arm_orders(&["--shuffle", "2"]), shuffled, "another seed");
}

#[test]
fn the_note_given_at_the_finish_is_counted() {
    let finish = |id: &str, input: Value| {
        json!({"type": "assistant", "message": {"id": id, "content": [
            {"type": "tool_use", "id": id, "name": "mcp__ripwire-broker__context_before_finish",
             "input": input}]}})
    };
    let session = |calls: Vec<Value>| {
        let mut events = vec![json!({"type": "system", "subtype": "init", "tools": [],
            "mcp_servers": [{"name": "ripwire-broker", "status": "connected"}]})];
        events.extend(calls);
        events.push(
            json!({"type": "result", "subtype": "success", "is_error": false,
            "duration_ms": 5, "total_cost_usd": 0.01,
            "usage": {"input_tokens": 1, "output_tokens": 1}}),
        );
        let timed: Vec<(u64, Value)> = events.into_iter().map(|e| (0, e)).collect();
        transcript::summarize(&timed)
    };
    let none = session(vec![]);
    assert_eq!((none.finish_calls, none.finish_notes), (0, 0));
    let silent = session(vec![finish("a", json!({"changed_files": ["src/a.rs"]}))]);
    assert_eq!((silent.finish_calls, silent.finish_notes), (1, 0));
    let blank = session(vec![finish("a", json!({"note": "  "}))]);
    assert_eq!((blank.finish_calls, blank.finish_notes), (1, 0));
    let said = session(vec![
        finish("a", json!({"note": "In src/a.rs, get() now returns None."})),
        finish("b", json!({})),
    ]);
    assert_eq!((said.finish_calls, said.finish_notes), (2, 1));
}

#[test]
fn the_memory_debug_log_is_asked_of_the_memory_arms_and_kept_beside_the_transcript() {
    let tools = |on| ripwire_broker::eval::arm::Tools {
        broker: "/bin/rb".into(),
        ripwire: "/bin/rw".into(),
        memory_debug_log: on,
    };
    let args = |arm: Arm, on| {
        let cfg = arm.mcp_config(&tools(on), Path::new("/work"), Path::new("/state"));
        cfg["mcpServers"]["ripwire-broker"]["args"].to_string()
    };
    for arm in [Arm::BrokerMemory, Arm::BrokerMemoryDeterministic] {
        assert!(args(arm, true).contains("--memory-debug-log"), "{arm:?}");
        assert!(!args(arm, false).contains("--memory-debug-log"), "{arm:?}");
    }
    assert!(!args(Arm::BrokerOnline, true).contains("--memory-debug-log"));

    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let broker = draining_broker(work.path());
    let out = run_in(
        work.path(),
        json!([
            auth_task("first", repo.path(), &head, Some("login")),
            auth_task("second", repo.path(), &head, Some("login")),
        ]),
        "broker-online,broker-memory",
        &["--broker", broker.to_str().unwrap(), "--memory-debug-log"],
    );
    assert!(out.status.success(), "{out:?}");
    let kept =
        |name: &str| std::fs::read_to_string(work.path().join("out/transcripts").join(name)).ok();
    // The store's log so far, after each session: the second holds the first's lines too.
    assert_eq!(
        kept("first__broker-memory__1.memory.log")
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert_eq!(
        kept("second__broker-memory__1.memory.log")
            .unwrap()
            .lines()
            .count(),
        2
    );
    assert!(kept("first__broker-online__1.memory.log").is_none());
    let log = std::fs::read_to_string(work.path().join("log")).unwrap();
    let drain = log.lines().find(|l| l.contains(" memory drain ")).unwrap();
    assert!(drain.ends_with("--online --memory-debug-log"), "{drain}");
}

// --- Codex (`codex exec --json`) ---

/// A session as Codex 0.161.0 wrote it (trimmed, paths shortened), one event every 100 ms of
/// the runner's clock: items in place of messages, and one `turn.completed` with the usage.
fn codex_session(items: Vec<Value>) -> Vec<(u64, Value)> {
    let mut events = vec![
        json!({"type": "thread.started", "thread_id": "01a12ad6"}),
        json!({"type": "turn.started"}),
    ];
    events.extend(items);
    events.push(
        json!({"type": "turn.completed", "usage": {"input_tokens": 201_966,
        "cached_input_tokens": 175_360, "cache_write_input_tokens": 0, "output_tokens": 859,
        "reasoning_output_tokens": 11}}),
    );
    events
        .into_iter()
        .enumerate()
        .map(|(n, e)| (n as u64 * 100, e))
        .collect()
}

fn codex_item(id: &str, done: bool, body: Value) -> Value {
    let mut item = body;
    item["id"] = json!(id);
    item["status"] = json!(if done { "completed" } else { "in_progress" });
    json!({"type": if done { "item.completed" } else { "item.started" }, "item": item})
}

/// A shell command, started and completed.
fn codex_shell(id: &str, line: &str) -> Vec<Value> {
    let command = format!("/bin/zsh -lc {line}");
    let body = |done: bool| {
        json!({"type": "command_execution", "command": command,
        "aggregated_output": "", "exit_code": if done { json!(0) } else { Value::Null }})
    };
    vec![
        codex_item(id, false, body(false)),
        codex_item(id, true, body(true)),
    ]
}

/// A call to the broker, started and completed with `text` as its answer.
fn codex_call(id: &str, tool: &str, arguments: Value, text: &str) -> Vec<Value> {
    let body = |result: Value| {
        json!({"type": "mcp_tool_call", "server": "ripwire-broker",
        "tool": tool, "arguments": arguments, "result": result, "error": null})
    };
    vec![
        codex_item(id, false, body(Value::Null)),
        codex_item(
            id,
            true,
            body(json!({"content": [{"type": "text", "text": text}], "structured_content": null})),
        ),
    ]
}

const CODEX_ENVELOPE: &str = r#"{"items":[{"path":"src/auth.py"}],"tests":[{"path":"tests/test_auth.py"}],"memories":[{"id":"m1"}],"provenance":{"memory":{"requests":2,"questions":3,"stop_reason":"classified"}}}

Memories: one."#;

#[test]
fn a_codex_transcript_is_reduced_to_the_same_counts() {
    let mut items = vec![json!({"type": "item.completed", "item": {"id": "item_0",
        "type": "agent_message", "text": "I'll update the token lifetime."}})];
    items.extend(codex_shell("item_1", "\"pwd; rg --files -g '*auth*'\""));
    items.extend(codex_call(
        "item_2",
        "context_for_task",
        json!({"task": "Change TOKEN_TTL_SECONDS"}),
        CODEX_ENVELOPE,
    ));
    items.extend(codex_shell("item_3", "'cat src/auth.py'"));
    let change = |done| {
        codex_item(
            "item_4",
            done,
            json!({"type": "file_change",
                "changes": [{"path": "/work/src/auth.py", "kind": "update"}]}),
        )
    };
    items.extend([change(false), change(true)]);
    items.extend(codex_shell(
        "item_5",
        "'python3 -m pytest tests/test_auth.py'",
    ));
    items.extend(codex_call(
        "item_6",
        "context_before_finish",
        json!({"note": "In src/auth.py, TOKEN_TTL_SECONDS went from 3600 to 7200."}),
        "{}",
    ));
    let events = codex_session(items);
    let last = events.last().unwrap().0;

    let s = transcript::summarize(&events);

    assert!(s.valid(), "{:?}", s.invalid);
    assert_eq!(s.agent.as_deref(), Some("codex"));
    // An item is one call, however many events tell of it.
    assert_eq!(
        (
            s.calls.search,
            s.calls.read,
            s.calls.edit,
            s.calls.mcp,
            s.calls.other
        ),
        (1, 1, 1, 2, 1),
        "{:?}",
        s.calls
    );
    // `input_tokens` already holds the cached ones.
    assert_eq!(s.tokens.total(), 201_966 + 859);
    assert_eq!(s.tokens.cache_read, 175_360);
    // Codex reports neither a cost nor a duration: the runner's clock at the end of the turn.
    assert_eq!(s.cost_usd, 0.0);
    assert_eq!(s.duration_ms, last);
    // The file change starts at the 10th event.
    assert_eq!(s.first_edit_ms, Some(900));
    assert_eq!(s.presented_files, vec!["src/auth.py"]);
    assert_eq!(s.presented_tests, vec!["tests/test_auth.py"]);
    assert_eq!(s.mcp_result_bytes, (CODEX_ENVELOPE.len() + 2) as u64);
    // From the call to its answer: one event later.
    assert_eq!((s.context_calls, s.context_ms), (1, 100));
    assert_eq!((s.finish_calls, s.finish_notes), (1, 1));
    let read = s.memory_read.expect("the envelope carried a memory read");
    assert_eq!(
        (read.reads, read.requests, read.questions, read.delivered),
        (1, 2, 3, 1)
    );
    assert_eq!(s.commands.len(), 3);
    // Codex lists no servers: the one a call was answered by is the one that connected.
    assert_eq!(s.mcp_servers, vec!["ripwire-broker"]);
    assert_eq!(Arm::BrokerMemory.contamination(&s), None);
    let none = Arm::None.contamination(&s).expect("a server in `none`");
    assert!(none.contains("ripwire-broker"), "{none}");
}

#[test]
fn a_codex_session_that_never_reached_its_server_does_not_count_for_the_arm() {
    // No call at all: nothing says the server was there.
    let quiet = transcript::summarize(&codex_session(codex_shell("item_1", "'cat src/auth.py'")));
    assert!(quiet.valid(), "{:?}", quiet.invalid);
    let why = Arm::BrokerOnline.contamination(&quiet).expect("absent");
    assert!(why.contains("not connected"), "{why}");
    assert_eq!(Arm::None.contamination(&quiet), None);

    // A call that failed: the server was asked and did not answer.
    let call = |done: bool, status: &str| {
        let mut e = codex_item(
            "item_1",
            done,
            json!({"type": "mcp_tool_call", "server": "ripwire-broker",
                "tool": "context_for_task", "arguments": {"task": "x"}, "result": null,
                "error": {"message": "tool call failed: Method not found"}}),
        );
        e["item"]["status"] = json!(status);
        e
    };
    let failed = transcript::summarize(&codex_session(vec![
        call(false, "in_progress"),
        call(true, "failed"),
    ]));
    assert_eq!(failed.calls.mcp, 1);
    assert_eq!(failed.context_calls, 0, "no answer to time");
    let why = Arm::BrokerOnline.contamination(&failed).expect("failed");
    assert!(why.contains("not connected (failed)"), "{why}");

    // One answer is enough: a later call that fails does not take the server away.
    let mut items = codex_call("item_0", "context_for_task", json!({"task": "x"}), "{}");
    items.extend([call(false, "in_progress"), call(true, "failed")]);
    let answered = transcript::summarize(&codex_session(items));
    assert_eq!(Arm::BrokerOnline.contamination(&answered), None);
}

#[test]
fn a_codex_turn_that_failed_or_never_ended_is_invalid_not_zero() {
    let started = json!({"type": "thread.started", "thread_id": "t"});
    let cut = transcript::summarize(&[(0, started.clone())]);
    assert!(cut.invalid.as_deref().unwrap().contains("result"));

    let failed = json!({"type": "turn.failed",
        "error": {"message": "stream disconnected before completion"}});
    let s = transcript::summarize(&[(0, started), (1, failed)]);
    let why = s.invalid.expect("the arm did not fail; the API did");
    assert!(why.contains("stream disconnected"), "{why}");
}

#[test]
fn a_context_tool_run_from_a_codex_shell_is_contamination_too() {
    let s = transcript::summarize(&codex_session(codex_shell(
        "item_1",
        "'graft ask \"where is the token checked\" --source'",
    )));
    assert_eq!(s.shell_tools, vec!["graft"]);
    assert!(Arm::None.contamination(&s).unwrap().contains("graft"));
}

#[test]
fn the_stop_reason_of_each_memory_read_reaches_the_record() {
    let envelope = |stop: &str| {
        format!(
            r#"{{"items":[],"provenance":{{"memory":{{"requests":1,"questions":1,"stop_reason":"{stop}"}}}}}}"#
        )
    };
    let mut items = codex_call(
        "a",
        "context_for_task",
        json!({"task": "x"}),
        &envelope("budget_omitted"),
    );
    items.extend(codex_call(
        "b",
        "context_after_edit",
        json!({}),
        &envelope("empty"),
    ));
    items.extend(codex_call("c", "context_before_finish", json!({}), "{}"));
    let s = transcript::summarize(&codex_session(items));
    assert_eq!(s.memory_stops, vec!["budget_omitted", "empty"]);
}

#[test]
fn an_arm_is_given_to_codex_as_configuration_overrides() {
    let tools = ripwire_broker::eval::arm::Tools {
        broker: "/bin/r b".into(),
        ripwire: "/bin/rw".into(),
        memory_debug_log: false,
    };
    let (ws, st) = (Path::new("/wo\"rk"), Path::new("/state"));
    assert!(Arm::None.codex_overrides(&tools, ws, st).is_empty());
    // Each value is TOML, each override one argument after its `-c`: no shell, no splitting.
    assert_eq!(
        Arm::Broker.codex_overrides(&tools, ws, st),
        vec![
            "-c",
            r#"mcp_servers.ripwire-broker.command="/bin/r b""#,
            "-c",
            r#"mcp_servers.ripwire-broker.args=["serve","--workspace","/wo\"rk","--ripwire","/bin/rw"]"#,
        ]
    );
    let memory = Arm::BrokerMemory.codex_overrides(&tools, ws, st);
    assert_eq!(
        memory[4..],
        [
            "-c",
            r#"mcp_servers.ripwire-broker.env.XDG_STATE_HOME="/state""#,
            // Codex starts a server without the caller's environment: the credential is named.
            "-c",
            r#"mcp_servers.ripwire-broker.env_vars=["RIPWIRE_BROKER_JEV_API_KEY"]"#,
        ]
    );
    let online = Arm::BrokerOnline.codex_overrides(&tools, ws, st);
    assert_eq!(online.len(), 6);
    assert!(online[5].contains("env_vars"), "{online:?}");
}

/// A stand-in for `codex exec --json`: its arguments go to `$ARGS`, one per line, and it calls
/// the broker once, whose answer says how the memory read stopped.
fn codex_agent(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("codex-agent");
    common::write_executable(
        &path,
        r##"#!/bin/sh
cat > /dev/null
printf '%s\n' "$@" > "$ARGS"
echo '{"type":"thread.started","thread_id":"t"}'
echo '{"type":"item.started","item":{"id":"i1","type":"mcp_tool_call","server":"ripwire-broker","tool":"context_for_task","arguments":{"task":"login"},"result":null,"error":null,"status":"in_progress"}}'
printf '%s\n' '{"type":"item.completed","item":{"id":"i1","type":"mcp_tool_call","server":"ripwire-broker","tool":"context_for_task","arguments":{"task":"login"},"result":{"content":[{"type":"text","text":"{\"items\":[],\"provenance\":{\"memory\":{\"requests\":1,\"questions\":2,\"stop_reason\":\"below_entry\"}}}"}]},"error":null,"status":"completed"}}'
echo '{"type":"item.completed","item":{"id":"i2","type":"file_change","changes":[{"path":"src/auth.py","kind":"update"}],"status":"completed"}}'
echo "# expired tokens are rejected" >> src/auth.py
echo '{"type":"turn.completed","usage":{"input_tokens":10,"cached_input_tokens":4,"output_tokens":5}}'
"##,
    );
    path
}

#[test]
fn a_codex_run_gets_the_arm_on_its_command_line_and_is_recorded_like_any_other() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let work = tempfile::tempdir().unwrap();
    let (agent, broker) = (codex_agent(work.path()), draining_broker(work.path()));
    let corpus = work.path().join("corpus.json");
    let tasks = json!([auth_task("login", repo.path(), &head, None)]);
    std::fs::write(&corpus, json!({ "tasks": tasks }).to_string()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ripwire-eval"))
        .args(["run", "--corpus"])
        .arg(&corpus)
        .arg("--out")
        .arg(work.path().join("out"))
        .args(["--arms", "broker-memory", "--broker"])
        .arg(&broker)
        .arg("--agent-cmd")
        .arg(format!("{} exec {{codex_mcp}} --json", agent.display()))
        .env("ARGS", work.path().join("args"))
        .env("LOG", work.path().join("log"))
        .env("RIPWIRE_BROKER_JEV_API_KEY", "synthetic-not-a-credential")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");

    let args = std::fs::read_to_string(work.path().join("args")).unwrap();
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(args.len(), 10, "{args:?}");
    assert_eq!((args[0], args[1], args[9]), ("exec", "-c", "--json"));
    assert_eq!(
        args[2],
        format!(
            r#"mcp_servers.ripwire-broker.command="{}""#,
            broker.display()
        )
    );
    assert!(
        args[4].contains(r#"args=["serve","--workspace","#),
        "{}",
        args[4]
    );
    assert!(args[4].ends_with(r#""--memory"]"#), "{}", args[4]);
    assert!(args[6].contains("env.XDG_STATE_HOME="), "{}", args[6]);

    let results = std::fs::read_to_string(work.path().join("out/results.jsonl")).unwrap();
    let r: Value = serde_json::from_str(results.lines().next().unwrap()).unwrap();
    assert_eq!(r["valid"], true, "{r}");
    assert_eq!(r["agent"], "codex");
    assert_eq!(r["tokens"], 15);
    assert_eq!(r["edit"], 1);
    assert_eq!(r["memory_stop_reasons"], json!(["below_entry"]));
    assert_eq!(r["memories_delivered"], 0);
}

#[test]
fn an_arm_without_memory_records_no_stop_reasons() {
    let repo = common::sample_repo();
    let head = git(repo.path(), &["rev-parse", "HEAD"]);
    let run = logged_run(
        json!([auth_task("login", repo.path(), &head, None)]),
        "broker",
        "1",
    );
    assert!(
        run[0]
            .1
            .get("memory_stop_reasons")
            .is_none_or(Value::is_null),
        "{}",
        run[0].1
    );
}
