//! The repository's own skills (ai-native plan, step 10) and the changelog index the
//! `decision-entry` skill writes to.
//!
//! The skills live in `.agents/skills/<name>/SKILL.md`, with a link from `.claude/skills/` and
//! `.grok/skills/` so each agent finds the same file. An agent session has the maintainer's login
//! (D-192): a skill that publishes, merges or approves on its own would bypass the "Ask before"
//! list, so each one names where it stops.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");
const OWN: [&str; 3] = ["decision-entry", "release", "audit-rust"];

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(ROOT).join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn skill(name: &str) -> String {
    read(&format!(".agents/skills/{name}/SKILL.md"))
}

/// The text between a `## {title}` heading and the next `## ` heading.
fn section(doc: &str, title: &str) -> String {
    let heading = format!("## {title}");
    doc.lines()
        .skip_while(|l| l.trim_end() != heading)
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_own_skill_is_one_file_that_each_agent_finds() {
    for name in OWN {
        let text = skill(name);
        let front: Vec<&str> = text.lines().skip(1).take_while(|l| *l != "---").collect();
        assert!(text.starts_with("---\n"), "{name}: no front matter");
        assert!(
            front.contains(&format!("name: {name}").as_str()),
            "{name}: {front:?}"
        );
        assert!(
            front.iter().any(|l| l.starts_with("description:")),
            "{name}: no description"
        );
        for agent in [".claude", ".grok"] {
            let link = Path::new(ROOT).join(agent).join("skills").join(name);
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
    // Commands with the effects AGENTS.md "Ask before" lists. A skill may name them only in the
    // section where it stops and hands them to the maintainer.
    const EFFECTS: &[&str] = &[
        "git push",
        "git tag",
        "gh pr merge",
        "gh release create",
        "gh release edit",
        "--online",
        "serve --memory",
        "memory forget --all",
        "--write",
    ];
    for name in OWN {
        let text = skill(name);
        let stops = section(&text, "Where it stops");
        assert!(
            stops.contains("AGENTS.md") && stops.contains("Ask before"),
            "{name}: no `## Where it stops` section citing AGENTS.md \"Ask before\""
        );
        let rest: String = text.replace(&stops, "");
        for effect in EFFECTS {
            assert!(
                !rest.contains(effect),
                "{name}: `{effect}` outside the section where it stops"
            );
        }
    }
}

#[test]
fn the_release_skill_prepares_and_hands_over_the_three_human_steps() {
    let release = skill("release");
    let stops = section(&release, "Where it stops");
    for step in ["tag", "`release` environment", "merge"] {
        assert!(
            stops.contains(step),
            "the release skill does not stop at the {step}"
        );
    }
    // What it does prepare, from "Publishing a version" in the plugin README.
    for prepared in [
        "Cargo.toml",
        "scripts/check.sh all",
        "SHA256SUMS",
        "checksums.txt",
        "plugin.json",
    ] {
        assert!(
            release.contains(prepared),
            "the release skill omits {prepared}"
        );
    }
}

#[test]
fn the_decision_entry_skill_writes_what_the_changelog_holds() {
    let entry = skill("decision-entry");
    for rule in [
        "date '+%Y-%m-%d %H:%M'",
        "spec/changelog.md",
        "end of the index",
        "## D-NNN — ",
        "Portuguese",
        "anchor",
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
    for line in [
        "Você é um revisor de código Rust sênior.",
        "PROCURE, NESTA ORDEM",
        "SAÍDA, POR ACHADO",
    ] {
        assert!(audit.contains(line), "the audit-rust skill lost {line:?}");
    }
    // The old prompt is a pointer now, not a second copy.
    let prompt = read("spec/prompt/auditoria-rust.md");
    assert!(
        prompt.contains(".agents/skills/audit-rust/SKILL.md"),
        "{prompt}"
    );
    assert!(
        !prompt.contains("PROCURE, NESTA ORDEM"),
        "two copies of the audit"
    );
}

/// The decision numbers of the changelog's index rows, in order.
fn index_rows(changelog: &str) -> Vec<u32> {
    changelog
        .lines()
        .take_while(|l| !l.starts_with("## "))
        .filter(|l| l.starts_with("| 20"))
        .map(|l| {
            let link = l.rsplit("[D-").next().expect("a [D-NNN] link");
            link[..3].parse().unwrap_or_else(|_| panic!("{l}"))
        })
        .collect()
}

#[test]
fn the_changelog_index_has_one_row_per_decision_in_order() {
    let changelog = read("spec/changelog.md");
    let rows = index_rows(&changelog);
    let sections: Vec<u32> = changelog
        .lines()
        .filter_map(|l| l.strip_prefix("## D-"))
        .map(|l| l[..3].parse().expect("three digits"))
        .collect();
    assert_eq!(
        rows.iter().copied().collect::<BTreeSet<_>>(),
        sections.iter().copied().collect::<BTreeSet<_>>(),
        "a decision without an index row, or a row without a section"
    );
    let expected: Vec<u32> = (1..=*sections.iter().max().expect("decisions")).collect();
    assert_eq!(
        rows, expected,
        "the index is D-001 to the last, ascending, once each"
    );
    // Each section once. Their order in the body is left as written (D-102 sits before D-101);
    // the index is what is read.
    assert_eq!(
        sections.len(),
        expected.len(),
        "a decision with two sections"
    );
    // The convention says so.
    let handoff = read("handoff.md");
    let conventions = section(&handoff, "Convenções");
    assert!(
        conventions.contains("crescente") && !conventions.contains("decrescente"),
        "{conventions}"
    );
}
