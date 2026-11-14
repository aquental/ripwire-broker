//! The repository's own skills (ai-native plan, step 10) and the changelog index the
//! `decision-entry` skill writes to.
//!
//! The skills live in `.agents/skills/<name>/SKILL.md`, with a symlink from `.claude/skills/` and
//! `.grok/skills/` so each agent finds the same file. An agent session has the maintainer's login
//! (D-192): a skill that publishes, merges, approves or changes what plugin users run on its own
//! would bypass the "Ask before" list, so each one names where it stops and says it does not run
//! those steps.
#![cfg(unix)] // The links are Unix symlinks.

mod common;

use common::markdown::{section_lines, slug, without_comments};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(ROOT).join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The skills born here: every directory of `.agents/skills/` that `skills-lock.json` does not
/// pin to an outside source. A new one is tested without being listed anywhere.
fn own_skills() -> Vec<String> {
    let lock = read("skills-lock.json");
    let mut own: Vec<String> = fs::read_dir(Path::new(ROOT).join(".agents/skills"))
        .expect(".agents/skills")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|name| !lock.contains(&format!("\"{name}\":")))
        .collect();
    own.sort();
    own
}

fn skill(name: &str) -> String {
    without_comments(&read(&format!(".agents/skills/{name}/SKILL.md")))
}

/// The skill without its front matter: what it tells the agent to do. The description is for
/// choosing the skill, and must not satisfy a check of the steps.
fn body(name: &str) -> String {
    let text = skill(name);
    let mut parts = text.splitn(3, "---\n");
    parts.next();
    parts.next();
    parts.next().expect("a closed front matter").to_string()
}

/// The skill split in two: the `Where it stops` section, and everything else.
fn stops_and_rest(text: &str) -> (String, String) {
    let range = section_lines(text, "Where it stops");
    let lines: Vec<&str> = text.lines().collect();
    let stops = lines[range.clone()].join("\n");
    let rest = lines[..range.start]
        .iter()
        .chain(&lines[range.end..])
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    (stops, rest)
}

#[test]
fn the_own_skills_are_the_three_the_plan_names() {
    assert_eq!(own_skills(), ["audit-rust", "decision-entry", "release"]);
}

#[test]
fn every_own_skill_is_one_file_that_each_agent_finds() {
    for name in own_skills() {
        let text = skill(&name);
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("---"), "{name}: no front matter");
        let front: Vec<&str> = lines.by_ref().take_while(|l| *l != "---").collect();
        assert!(
            lines.next().is_some(),
            "{name}: the front matter is not closed"
        );
        assert!(
            front.contains(&format!("name: {name}").as_str()),
            "{name}: {front:?}"
        );
        assert!(
            front.iter().any(|l| l.starts_with("description:")),
            "{name}: no description"
        );
        for agent in [".claude", ".grok"] {
            let link = Path::new(ROOT).join(agent).join("skills").join(&name);
            let target = fs::read_link(&link).unwrap_or_else(|e| panic!("{}: {e}", link.display()));
            assert_eq!(
                target,
                PathBuf::from(format!("../../.agents/skills/{name}")),
                "{}",
                link.display()
            );
            assert!(
                link.join("SKILL.md").is_file(),
                "{} is dangling",
                link.display()
            );
        }
    }
}

#[test]
fn every_own_skill_stops_where_the_ask_before_list_says() {
    // What AGENTS.md "Ask before" lists, as the words a step would use. A skill may name them only
    // in its `Where it stops` section, which says it does not run them.
    const EFFECTS: &[&str] = &[
        "git push",
        "git tag",
        "gh pr merge",
        "gh pr create",
        "gh release create",
        "gh release edit",
        "--online",
        "serve --memory",
        "--jev-probe",
        "online_live",
        "ripwire-eval run",
        "memory forget --all",
        "--write",
        "integrations/claude-code/",
        ".claude-plugin/",
        "rust.yml",
        "deny.toml",
        ".gitleaks",
        "approv",
    ];
    for name in own_skills() {
        // The body: the description, for choosing the skill, may name what the skill hands over.
        let (stops, rest) = stops_and_rest(&body(&name));
        assert!(
            stops.contains("AGENTS.md") && stops.contains("Ask before"),
            "{name}: the stop section does not cite AGENTS.md \"Ask before\""
        );
        assert!(
            stops.contains("does not run"),
            "{name}: the stop section does not say the skill does not run these"
        );
        for word in ["executes", "yourself", "without asking", "runs these"] {
            assert!(
                !stops.contains(word),
                "{name}: the stop section says {word:?}"
            );
        }
        let rest = rest.to_lowercase();
        for effect in EFFECTS {
            assert!(
                !rest.contains(&effect.to_lowercase()),
                "{name}: `{effect}` outside the section where it stops"
            );
        }
    }
}

#[test]
fn the_release_skill_prepares_and_hands_over_the_human_steps() {
    let release = body("release");
    let (stops, _) = stops_and_rest(&release);
    // Line breaks are not meaning: compare the flattened text.
    let release = release.split_whitespace().collect::<Vec<_>>().join(" ");
    let stops = stops.split_whitespace().collect::<Vec<_>>().join(" ");
    for step in [
        "git switch master && git pull --ff-only && git tag vX.Y.Z",
        "`release` environment",
        "merge",
        "plugin's pins",
    ] {
        assert!(
            stops.contains(step),
            "the release skill does not stop at {step}"
        );
    }
    // What it prepares, in the order of "Publishing a version".
    let at = |needle: &str| {
        release
            .find(needle)
            .unwrap_or_else(|| panic!("the release skill omits {needle:?}"))
    };
    assert!(at("Cargo.toml") < at("scripts/check.sh all"));
    assert!(at("## Before the tag") < at("SHA256SUMS"));
    for prepared in [
        "checksums.txt",
        "plugin.json",
        "the PR text: the version, what changes for plugin users",
    ] {
        at(prepared);
    }
}

#[test]
fn the_decision_entry_skill_writes_what_the_changelog_holds() {
    let entry = body("decision-entry");
    for rule in [
        "date '+%Y-%m-%d %H:%M'",
        "spec/changelog.md",
        "at the end of the index: after the last",
        "Never invent or round it.",
        "at the end of the file",
        "## D-NNN — ",
        "in Portuguese",
        "other than `-` and `_`",
    ] {
        assert!(
            entry.contains(rule),
            "the decision-entry skill does not say {rule:?}"
        );
    }
}

#[test]
fn the_rust_audit_lives_in_one_place() {
    let audit = skill("audit-rust");
    // The whole prompt: its four blocks and the twelve things to look for.
    for line in [
        "Você é um revisor de código Rust sênior.",
        "CONTEXTO",
        "REGRAS",
        "PROCURE, NESTA ORDEM",
        "SAÍDA, POR ACHADO",
    ] {
        assert!(audit.contains(line), "the audit-rust skill lost {line:?}");
    }
    for n in 1..=12 {
        assert!(
            audit.lines().any(|l| l.starts_with(&format!("{n}. "))),
            "the audit-rust skill lost item {n}"
        );
    }
    // The old prompt is a pointer now, not a second copy.
    let prompt = read("spec/prompt/auditoria-rust.md");
    assert!(
        prompt.contains(".agents/skills/audit-rust/SKILL.md"),
        "{prompt}"
    );
    assert!(prompt.lines().count() <= 6, "{prompt}");
    for block in ["CONTEXTO", "REGRAS", "PROCURE", "SAÍDA"] {
        assert!(!prompt.contains(block), "a second copy: {block}");
    }
}

/// The index rows: every line of the table before the first section, each
/// `| YYYY-MM-DD HH:MM | summary | [D-NNN](#anchor) |`, as (number, anchor).
fn index_rows(changelog: &str) -> Vec<(u32, String)> {
    changelog
        .lines()
        .take_while(|l| !l.starts_with("## "))
        .filter(|l| l.starts_with('|'))
        .filter(|l| *l != "| Data | Resumo | Referência |" && *l != "| --- | --- | --- |")
        .map(|l| {
            let date = l.get(2..18).unwrap_or_default();
            let shape = date.len() == 16
                && date.char_indices().all(|(i, c)| match i {
                    4 | 7 => c == '-',
                    10 => c == ' ',
                    13 => c == ':',
                    _ => c.is_ascii_digit(),
                })
                && l.get(18..21) == Some(" | ");
            assert!(shape, "not `| YYYY-MM-DD HH:MM | …`: {l}");
            let reference = l
                .trim_end()
                .strip_suffix(" |")
                .and_then(|r| r.rsplit(" | ").next())
                .unwrap_or_else(|| panic!("no reference column: {l}"));
            let (number, anchor) = reference
                .strip_prefix("[D-")
                .and_then(|r| r.split_once("](#"))
                .and_then(|(n, a)| Some((n.parse().ok()?, a.strip_suffix(')')?.to_string())))
                .unwrap_or_else(|| panic!("the reference is not [D-NNN](#anchor): {l}"));
            (number, anchor)
        })
        .collect()
}

#[test]
fn the_changelog_index_has_one_row_per_decision_in_order() {
    let changelog = read("spec/changelog.md");
    let rows = index_rows(&changelog);
    let sections: Vec<(u32, String)> = changelog
        .lines()
        .filter_map(|l| l.strip_prefix("## "))
        .filter(|h| h.starts_with("D-"))
        .map(|h| (h[2..5].parse().expect("three digits"), slug(h)))
        .collect();
    let last = sections.iter().map(|(n, _)| *n).max().expect("decisions");
    let numbers: Vec<u32> = rows.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        numbers,
        (1..=last).collect::<Vec<_>>(),
        "the index is D-001 to the last, ascending, once each"
    );
    // Each row's link reaches its own section.
    let anchors: std::collections::BTreeMap<u32, &String> =
        sections.iter().map(|(n, a)| (*n, a)).collect();
    for (number, anchor) in &rows {
        assert_eq!(
            Some(&anchor),
            anchors.get(number),
            "the row of D-{number:03} links elsewhere"
        );
    }
    // Each section once, new ones at the end: the body is in order except where it was written
    // out of order before the index was (D-102 sits before D-101).
    let body: Vec<u32> = sections.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        body.iter().collect::<BTreeSet<_>>().len(),
        body.len(),
        "a decision with two sections"
    );
    let mut expected: Vec<u32> = (1..=last).collect();
    expected.swap(100, 101);
    assert_eq!(body, expected, "a section out of place");
    // The convention says so.
    let handoff = read("handoff.md");
    let range = section_lines(&handoff, "Convenções");
    let conventions = handoff.lines().collect::<Vec<_>>()[range].join("\n");
    assert!(
        conventions.contains("O índice é crescente, do D-001 ao último")
            && !conventions.contains("decrescente"),
        "{conventions}"
    );
}
