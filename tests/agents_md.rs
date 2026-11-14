//! `AGENTS.md`, the file an agent loads at the start of a session (ai-native plan, steps 1, 2
//! and 4), and the `handoff.md` it points to.
//!
//! The file routes: every path it cites must exist, every section it points to must have that
//! title, and every command it shows must run. A router that points at a renamed section or a
//! command that is gone sends the agent the wrong way, quietly. The "Ask before" list is the only
//! guard on what an agent with the maintainer's login can do (D-192), so its six categories are
//! checked by name and by example.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(ROOT).join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The text between a `## {title}` heading and the next `## ` heading.
fn section(doc: &str, title: &str) -> String {
    let heading = format!("## {title}");
    let start = doc
        .lines()
        .position(|l| l.trim_end() == heading)
        .unwrap_or_else(|| panic!("no `{heading}` section"));
    doc.lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with("## "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `code span` in a text, outside fenced blocks.
fn code_spans(doc: &str) -> Vec<String> {
    let mut spans = Vec::new();
    let mut fenced = false;
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let mut parts = line.split('`');
        parts.next();
        while let (Some(inside), Some(_)) = (parts.next(), parts.next()) {
            spans.push(inside.to_string());
        }
    }
    spans
}

/// The lines of the fenced blocks, without the fences.
fn fenced_lines(doc: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut fenced = false;
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        } else if fenced {
            lines.push(line.to_string());
        }
    }
    lines
}

/// A code span that names a file or directory of this repository: no spaces, no glob, not a
/// flag, and either a path with a slash or a file name with an extension.
fn is_repo_path(span: &str) -> bool {
    const EXTENSIONS: &[&str] = &[".md", ".rs", ".toml", ".json", ".sh", ".yml", ".ts"];
    !span.is_empty()
        && !span.contains([' ', '*', '<', '>', '|', '='])
        && !span.starts_with(['-', '@', '$', '~'])
        && !span.starts_with("http")
        && (span.contains('/') || EXTENSIONS.iter().any(|ext| span.ends_with(ext)))
}

/// The targets `scripts/check.sh` accepts, from its usage line.
fn check_targets() -> BTreeSet<String> {
    let script = read("scripts/check.sh");
    let usage = script
        .lines()
        .find_map(|l| l.split("usage: scripts/check.sh [").nth(1))
        .expect("the usage line of check.sh");
    usage
        .split(']')
        .next()
        .expect("closing bracket")
        .split('|')
        .map(str::to_string)
        .collect()
}

/// The aliases of `.cargo/config.toml`.
fn cargo_aliases() -> BTreeSet<String> {
    let config = read(".cargo/config.toml");
    config
        .lines()
        .skip_while(|l| l.trim() != "[alias]")
        .skip(1)
        .take_while(|l| !l.starts_with('['))
        .filter_map(|l| l.split_once('=').map(|(name, _)| name.trim().to_string()))
        .filter(|name| !name.starts_with('#'))
        .collect()
}

/// Every `D-NNN` number a text cites.
fn decisions_cited(doc: &str) -> Vec<u32> {
    doc.match_indices("D-")
        .filter_map(|(i, _)| {
            let digits: String = doc[i + 2..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            (digits.len() == 3).then(|| digits.parse().expect("three digits"))
        })
        .collect()
}

#[test]
fn agents_md_fits_the_size_an_agent_reads_whole() {
    let size = read("AGENTS.md").len();
    assert!(
        size <= 6_000,
        "AGENTS.md is {size} bytes; the plan's limit is 6,000"
    );
}

#[test]
fn claude_md_only_imports_agents_md() {
    assert_eq!(read("CLAUDE.md"), "@AGENTS.md\n");
}

#[test]
fn every_path_agents_md_cites_exists() {
    let agents = read("AGENTS.md");
    let paths: Vec<String> = code_spans(&agents)
        .into_iter()
        .filter(|s| is_repo_path(s))
        .collect();
    assert!(
        paths.len() >= 10,
        "too few paths found, the parser broke: {paths:?}"
    );
    let missing: Vec<&String> = paths
        .iter()
        .filter(|p| !Path::new(ROOT).join(p.as_str()).exists())
        .collect();
    assert!(
        missing.is_empty(),
        "AGENTS.md cites paths that do not exist: {missing:?}"
    );
}

#[test]
fn every_section_agents_md_points_to_has_that_title() {
    // A pointer is written `file.md` §"Title", the title exactly as the heading has it.
    let agents = read("AGENTS.md");
    const MARK: &str = "` §\"";
    let mut pointers = Vec::new();
    for (i, _) in agents.match_indices(MARK) {
        let file = agents[..i].rsplit('`').next().expect("an opening backtick");
        let title = agents[i + MARK.len()..]
            .split('"')
            .next()
            .expect("a closing quote");
        pointers.push((file.to_string(), title.to_string()));
    }
    for wanted in [
        ("handoff.md", "Como verificar"),
        ("handoff.md", "Onde está o quê"),
        ("handoff.md", "Armadilhas já pisadas"),
        ("handoff.md", "Convenções"),
        ("README.md", "Tests"),
    ] {
        assert!(
            pointers
                .iter()
                .any(|(f, t)| (f.as_str(), t.as_str()) == wanted),
            "AGENTS.md does not point to {wanted:?}; pointers: {pointers:?}"
        );
    }
    for (file, title) in &pointers {
        let doc = read(file);
        assert!(
            doc.lines()
                .any(|l| l.starts_with('#') && l.trim_start_matches('#').trim() == title),
            "{file} has no section titled {title:?}"
        );
    }
}

#[test]
fn every_command_agents_md_shows_runs() {
    let agents = read("AGENTS.md");
    let targets = check_targets();
    let aliases = cargo_aliases();
    let commands: Vec<String> = fenced_lines(&agents)
        .into_iter()
        .map(|l| l.split('#').next().unwrap_or("").trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    assert!(
        commands.iter().any(|c| c == "scripts/check.sh"),
        "AGENTS.md does not show the one command: {commands:?}"
    );
    for command in &commands {
        let words: Vec<&str> = command.split_whitespace().collect();
        match words.as_slice() {
            ["scripts/check.sh"] => {}
            ["scripts/check.sh", target] => {
                assert!(
                    targets.contains(*target),
                    "check.sh has no target {target:?}"
                );
            }
            ["cargo", sub, ..] => {
                assert!(
                    aliases.contains(*sub),
                    "cargo has no alias {sub:?}: {aliases:?}"
                );
            }
            _ => panic!("AGENTS.md shows a command this test cannot check: {command:?}"),
        }
    }
    // Every target the script has is named, so a new one (step 3's `secrets`) shows up here.
    let verify = section(&agents, "Verify");
    let words: BTreeSet<&str> = verify
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .collect();
    for target in &targets {
        assert!(
            words.contains(target.as_str()),
            "the Verify section does not name check.sh's target {target:?}"
        );
    }
}

#[test]
fn the_five_costliest_rules_are_written_down() {
    let rules = read("AGENTS.md");
    let rules = section(&rules, "Rules");
    for (rule, words) in [
        (
            "a D-NNN entry per decision",
            &["D-NNN", "spec/changelog.md"][..],
        ),
        ("no network crate in the default build", &["CA-10"]),
        (
            "the required jobs keep their names",
            &["`default`", "`online`", "D-107"],
        ),
        ("TDD through the public seam", &["TDD", "tests/"]),
        ("synthetic fixtures", &["tests/fixtures/", "synthetic"]),
    ] {
        for word in words {
            assert!(rules.contains(word), "the rule {rule:?} lacks {word:?}");
        }
    }
}

#[test]
fn ask_before_is_on_the_first_screen() {
    let agents = read("AGENTS.md");
    let line = agents
        .lines()
        .position(|l| l.trim_end() == "## Ask before")
        .expect("an `## Ask before` section");
    let end = line + section(&agents, "Ask before").lines().count();
    assert!(
        end <= 60,
        "the Ask before list ends on line {end}; a first screen is 60 lines"
    );
}

#[test]
fn ask_before_names_six_effects_each_with_its_reason_and_examples() {
    let agents = read("AGENTS.md");
    let list = section(&agents, "Ask before");
    let items: Vec<&str> = list.split("\n- ").skip(1).collect();
    assert_eq!(items.len(), 6, "six categories, by effect: {items:#?}");
    let expected: [(&str, &[&str]); 6] = [
        (
            "Publishing",
            &["`v*`", "gh pr merge", "`master`", "release"],
        ),
        (
            "Changing what plugin users run",
            &[
                "integrations/claude-code/",
                ".claude-plugin/marketplace.json",
                "checksums.txt",
            ],
        ),
        (
            "Spending money or sending content to a provider",
            &[
                "--online",
                "serve --memory",
                "memory drain --online",
                "doctor --jev-probe",
                "tests/online_live.rs",
                "ripwire-eval",
            ],
        ),
        ("Writing outside the repository", &["--write"]),
        ("Deleting real state", &["memory forget --all"]),
        (
            "Touching the gates",
            &[
                "rust.yml",
                "branch protection",
                "ruleset",
                "environment",
                "gitleaks",
                "deny.toml",
            ],
        ),
    ];
    for (item, (label, examples)) in items.iter().zip(expected) {
        assert!(
            item.starts_with(&format!("**{label}")),
            "expected {label:?} first in: {item}"
        );
        let flat = item.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat.contains(" because ") || flat.contains("Why:"),
            "{label:?} gives no reason: {item}"
        );
        for example in examples {
            assert!(
                flat.contains(example),
                "{label:?} lacks the example {example:?}"
            );
        }
    }
}

#[test]
fn the_credentials_section_says_whose_login_the_session_has() {
    let agents = read("AGENTS.md");
    let credentials = section(&agents, "Credentials");
    let flat = credentials.split_whitespace().collect::<Vec<_>>().join(" ");
    for word in [
        "maintainer",
        "D-192",
        "gh pr merge",
        "tag",
        "release",
        "environment",
    ] {
        assert!(
            flat.contains(word),
            "the Credentials section lacks {word:?}"
        );
    }
}

#[test]
fn agents_md_tells_the_product_agents_md_apart() {
    let agents = read("AGENTS.md");
    assert!(
        agents.contains("`integrations/codex/AGENTS.md`"),
        "AGENTS.md does not mention the product's AGENTS.md"
    );
    assert!(
        Path::new(ROOT)
            .join("integrations/codex/AGENTS.md")
            .exists()
    );
}

#[test]
fn the_handoff_header_is_as_recent_as_anything_it_says() {
    // One dated state: the header names the latest decision the document relies on (plan, step 1).
    let handoff = read("handoff.md");
    let header = handoff
        .lines()
        .take_while(|l| !l.starts_with("## "))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        header.contains("Estado em "),
        "the header has no date: {header}"
    );
    let stated = *decisions_cited(&header)
        .iter()
        .max()
        .expect("the header names a decision");
    let newest = *decisions_cited(&handoff).iter().max().expect("decisions");
    assert_eq!(
        stated, newest,
        "the header stops at D-{stated}; the body cites D-{newest}"
    );
    // The pointer to the changelog does not freeze a range that goes stale.
    assert!(
        !handoff.contains("D-001 a D-"),
        "handoff.md still states the changelog's range"
    );
}

#[test]
fn the_readme_marks_forgetting_everything_as_irreversible() {
    let readme = read("README.md");
    let row = readme
        .lines()
        .find(|l| l.starts_with("| `memory forget"))
        .expect("the memory forget row");
    assert!(row.contains("irreversible"), "{row}");
}
