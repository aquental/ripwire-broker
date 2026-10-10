//! Seam: what the memory admits (PRD jev-mem §5). The record `memory/v1`, its limits, and the
//! admission filters in front of the store.
use ripwire_broker::memory::model::{
    MAX_CONTENT_BYTES, MAX_ENTITIES, MAX_RECORD_BYTES, MAX_SOURCES, MAX_TEMPORAL_REFERENCES,
    Record, Rejected,
};
use serde_json::{Value, json};

/// A valid automatic observation, as JSON: the shape the spool stores.
fn record() -> Value {
    json!({
        "schema_version": 1,
        "policy_version": "memory-policy/v1",
        "node_id": "n".repeat(64),
        "content_hash": "c".repeat(64),
        "workspace_id": "w".repeat(64),
        "event_key": "e".repeat(64),
        "kind": "edit_observation",
        "content": "Evento: análise após edição. Escopo: src/cache.rs.",
        "observed_at_ms": 1_000,
        "ingest_seq": 1,
        "timestamp_role": "observation",
        "entities": [{"id": "x".repeat(64), "kind": "file", "path": "src/cache.rs"}],
        "sources": [{
            "path": "src/cache.rs",
            "sha256": "a".repeat(64),
            "verb": "quality_delta",
            "basis": "ripwire"
        }],
        "expires_at_ms": 2_000,
        "generation": 1
    })
}

fn parse(v: &Value) -> Result<Record, Rejected> {
    Record::parse(&serde_json::to_vec(v).unwrap())
}

#[test]
fn a_record_round_trips_and_an_oversized_one_is_refused_whole() {
    let r = parse(&record()).expect("a valid record is admitted");
    let again = Record::parse(&serde_json::to_vec(&r).unwrap()).unwrap();
    assert_eq!(r, again, "JSON round trip");
    assert_eq!(r.schema_version, 1);
    assert!(r.event_time.is_none(), "never inferred");

    // Absent type scores are unknown, never zero (PRD jev-mem §5.1).
    assert_eq!(r.types.episodic, None);
    assert_eq!(r.types.semantic, None);
    assert_eq!(r.types.procedural, None);
    assert_eq!(r.types.preference, None);

    let mut v = record();
    v["content"] = json!("é".repeat(MAX_CONTENT_BYTES / 2) + "x");
    assert_eq!(parse(&v), Err(Rejected::ContentTooLong), "2.001 bytes");
    v["content"] = json!("x".repeat(MAX_CONTENT_BYTES));
    assert!(parse(&v).is_ok(), "2.000 bytes fit");

    let mut v = record();
    let entity = v["entities"][0].clone();
    v["entities"] = json!(vec![entity; MAX_ENTITIES + 1]);
    assert_eq!(parse(&v), Err(Rejected::TooManyEntities));

    let mut v = record();
    let source = v["sources"][0].clone();
    v["sources"] = json!(vec![source; MAX_SOURCES + 1]);
    assert_eq!(parse(&v), Err(Rejected::TooManySources));

    let mut v = record();
    let reference = json!({"start_ms": 1, "precision": "day", "evidence_id": "s0"});
    v["temporal_references"] = json!(vec![reference.clone(); MAX_TEMPORAL_REFERENCES + 1]);
    assert_eq!(parse(&v), Err(Rejected::TooManyTemporalReferences));
    v["temporal_references"] = json!(vec![reference; MAX_TEMPORAL_REFERENCES]);
    assert!(parse(&v).is_ok(), "eight references fit");

    // Every list within its own cap, the whole still over 16 KiB: refused, not cut.
    let mut v = record();
    let mut source = v["sources"][0].clone();
    source["path"] = json!("p".repeat(1_100));
    v["sources"] = json!(vec![source; MAX_SOURCES]);
    assert!(serde_json::to_vec(&v).unwrap().len() > MAX_RECORD_BYTES);
    assert_eq!(parse(&v), Err(Rejected::RecordTooLarge));

    let r = parse(&record()).unwrap();
    let mut big = r.clone();
    big.content = "x".repeat(MAX_CONTENT_BYTES + 1);
    assert_eq!(big.check(), Err(Rejected::ContentTooLong));
    assert_eq!(
        big.content.len(),
        MAX_CONTENT_BYTES + 1,
        "checking never truncates the evidence"
    );
    let mut wide = r.clone();
    let mut source = wide.sources[0].clone();
    source.path = "p".repeat(1_100);
    wide.sources = vec![source; MAX_SOURCES];
    assert_eq!(
        wide.check(),
        Err(Rejected::RecordTooLarge),
        "a record built in process"
    );

    assert_eq!(Record::parse(b"{"), Err(Rejected::Malformed));
    let mut v = record();
    v["schema_version"] = json!(2);
    assert_eq!(parse(&v), Err(Rejected::UnknownSchema));
}

// ---------------------------------------------------------------- admission (PRD jev-mem §5.2)

use ripwire_broker::memory::admission::{
    self, Draft, Event, Named, Outcome, Refused, Seen, Stamp, Tests,
};
use ripwire_broker::memory::model::{Kind, TimestampRole};
use ripwire_broker::online::reader::{Ineligible, WorkspaceReader};
use std::fs;

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// A workspace with an eligible file and every kind of source the policy refuses. The files
/// are made by hand at fixed names; secret-shaped values are only ever built here (V12).
fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/cache.rs"), "fn get() {}\n").unwrap();
    fs::write(root.join("src/lib.rs"), "mod cache;\n").unwrap();
    fs::write(root.join("src/auth.py"), "TOKEN_TTL_SECONDS = 900\n").unwrap();
    fs::write(root.join(".env"), "A=1\n").unwrap();
    fs::write(root.join(".gitignore"), "out.log\n").unwrap();
    fs::write(root.join("out.log"), "x\n").unwrap();
    fs::write(root.join("blob.bin"), b"\x00\x01\x02").unwrap();
    std::os::unix::fs::symlink("/etc/hosts", root.join("src/link.rs")).unwrap();
    dir
}

fn draft(scope: &[&str]) -> Draft {
    Draft {
        event_key: "session-1/event-1".into(),
        event: Event::AfterEdit,
        outcome: Outcome::AnalysisCompleted,
        tests: Tests::Unknown,
        scope: scope.iter().map(|s| s.to_string()).collect(),
        evidence: vec!["quality_delta".into()],
        seen: Seen::default(),
    }
}

fn stamp() -> Stamp {
    Stamp {
        observed_at_ms: 1_000,
        ingest_seq: 1,
        generation: 1,
        retention_ms: 30 * DAY_MS,
    }
}

#[test]
fn forbidden_sources_are_refused_before_the_spool() {
    let dir = workspace();
    let reader = WorkspaceReader::new(dir.path()).unwrap();
    let ws = "w".repeat(64);
    for (path, why) in [
        (".env", Ineligible::SensitiveName),
        ("out.log", Ineligible::Ignored),
        ("blob.bin", Ineligible::Binary),
        ("src/link.rs", Ineligible::Symlink),
        ("../outside.rs", Ineligible::Outside),
        ("/etc/hosts", Ineligible::Outside),
    ] {
        let got = admission::admit(&reader, &ws, &draft(&["src/cache.rs", path]), stamp());
        assert_eq!(got, Err(Refused::Ineligible(why)), "{path}");
    }
    let ok = admission::admit(&reader, &ws, &draft(&["src/cache.rs"]), stamp()).unwrap();
    assert_eq!(ok.sources.len(), 1);
    assert_eq!(ok.sources[0].path, "src/cache.rs");
    assert!(
        ok.sources[0].sha256.starts_with("sha256:"),
        "hash of the bytes"
    );
    assert_eq!(ok.entities.len(), 1, "one file entity per source");
    assert_eq!(
        admission::admit(&reader, &ws, &draft(&[]), stamp()),
        Err(Refused::Empty)
    );
}

#[test]
fn a_secret_or_pii_shaped_value_rejects_the_whole_record_and_is_only_counted() {
    let dir = workspace();
    let root = dir.path();
    let reader = WorkspaceReader::new(root).unwrap();
    let ws = "w".repeat(64);
    let token = format!("{}{}", "sk-", "a1B2c3D4e5F6g7H8i9J0k1L2");
    let aws = format!("{}{}", "AKIA", "ABCDEFGHIJKLMNOP");
    let github = format!("{}{}", "ghp_", "Ab1".repeat(12));
    let jwt = format!(
        "{}.{}.{}",
        "eyJhbGciOiJIUzI1NiJ9", "eyJzdWIiOiIxIn0", "c2lnbmF0dXJl"
    );
    let assignment = format!("{}={}", "password", "hunter2");
    let email = format!("{}@{}", "maria.silva", "example.com");
    for (i, value) in [&token, &aws, &github, &jwt, &assignment, &email]
        .into_iter()
        .enumerate()
    {
        let name = format!("src/{value}.rs");
        fs::write(root.join(&name), "fn x() {}\n").unwrap();
        let got = admission::admit(&reader, &ws, &draft(&["src/cache.rs", &name]), stamp());
        let want = match i {
            5 => Refused::PersonalData,
            _ => Refused::SecretShaped,
        };
        assert_eq!(got, Err(want), "{i}");
        assert!(
            !format!("{want:?} {}", want.as_str()).contains(value.as_str()),
            "a refusal carries no content"
        );

        let mut d = draft(&["src/cache.rs"]);
        d.evidence = vec![value.clone()];
        assert_eq!(
            admission::admit(&reader, &ws, &d, stamp()),
            Err(want),
            "nor in the evidence"
        );
    }
}

#[test]
fn the_rendering_is_deterministic_and_never_claims_tests_passed() {
    let dir = workspace();
    let reader = WorkspaceReader::new(dir.path()).unwrap();
    let ws = "w".repeat(64);
    let r = admission::admit(&reader, &ws, &draft(&["src/cache.rs"]), stamp()).unwrap();
    let hash = &r.sources[0].sha256;
    assert_eq!(
        r.content,
        format!(
            "Evento: análise após edição. Escopo: src/cache.rs. Símbolos: nenhum. Dependentes: \
             nenhum. Testes ligados: nenhum. Achados: nenhum risco; nenhuma limitação. Observado \
             pelo broker: análise concluída; execução de testes desconhecida. Evidência: \
             quality_delta; revisão de fonte {hash}."
        ),
        "an analysis that named nothing says so, list by list"
    );
    assert_eq!(
        admission::admit(&reader, &ws, &draft(&["src/cache.rs"]), stamp()).unwrap(),
        r,
        "deterministic"
    );
    let ab = admission::admit(
        &reader,
        &ws,
        &draft(&["src/lib.rs", "src/cache.rs"]),
        stamp(),
    )
    .unwrap();
    let ba = admission::admit(
        &reader,
        &ws,
        &draft(&["src/cache.rs", "src/lib.rs", "src/cache.rs"]),
        stamp(),
    )
    .unwrap();
    assert_eq!(
        ab, ba,
        "the order and repetition of the scope do not make another node"
    );
    assert!(
        ab.content.contains("Escopo: src/cache.rs, src/lib.rs."),
        "{}",
        ab.content
    );
    assert_eq!(r.kind, Kind::EditObservation);
    assert_eq!(r.timestamp_role, TimestampRole::Observation);
    assert_eq!(r.event_time, None);
    assert_eq!(r.expires_at_ms, 1_000 + 30 * DAY_MS);
    assert_eq!(r.node_id.len(), 64);
    assert!(r.check().is_ok());

    for event in [Event::AfterEdit, Event::BeforeFinish] {
        for outcome in [Outcome::AnalysisCompleted, Outcome::AttentionRequired] {
            let mut d = draft(&["src/cache.rs"]);
            (d.event, d.outcome) = (event, outcome);
            let text = admission::admit(&reader, &ws, &d, stamp()).unwrap().content;
            for claim in ["passaram", "passed", "corrigido", "fixed", "merge"] {
                assert!(!text.contains(claim), "{text}");
            }
        }
    }
}

// ---------------------------------------------------- memory-observation/v2 (PRD jev-mem §5.2)

fn symbols(names: &[&str], path: &str) -> Vec<Named> {
    names.iter().map(|n| Named::symbol(n, path)).collect()
}

/// The draft of the PRD's example: two symbols ripwire resolved, a linked test, a limitation.
fn example() -> Draft {
    let mut d = draft(&["src/auth.py"]);
    d.evidence = vec!["situational_awareness".into(), "edit_check".into()];
    d.seen = Seen {
        symbols: symbols(&["TOKEN_TTL_SECONDS", "validate_token"], "src/auth.py"),
        dependents: vec![],
        tests: vec!["tests/test_auth.py::test_valid".into()],
        risks: vec![],
        limitations: vec!["counts_floor".into()],
    };
    d
}

#[test]
fn the_v2_rendering_says_what_the_analysis_was_about() {
    let dir = workspace();
    let reader = WorkspaceReader::new(dir.path()).unwrap();
    let ws = "w".repeat(64);
    let r = admission::admit(&reader, &ws, &example(), stamp()).unwrap();
    let hash = &r.sources[0].sha256;
    assert_eq!(
        r.content,
        format!(
            "Evento: análise após edição. Escopo: src/auth.py. Símbolos: TOKEN_TTL_SECONDS, \
             validate_token (src/auth.py). Dependentes: nenhum. Testes ligados: \
             tests/test_auth.py::test_valid. Achados: nenhum risco; limitações: counts_floor. \
             Observado pelo broker: análise concluída; execução de testes desconhecida. \
             Evidência: situational_awareness, edit_check; revisão de fonte {hash}."
        ),
        "the v2 example of PRD jev-mem §5.2, byte for byte"
    );
    assert_eq!(r.policy_version, "memory-policy/v2");
    assert!(
        !r.content.contains("900"),
        "a name, never the value: {}",
        r.content
    );
    assert_eq!(
        admission::admit(&reader, &ws, &example(), stamp()).unwrap(),
        r,
        "deterministic"
    );

    // Dependents keep the order ripwire gave, grouped under the file they are in; a file
    // with no symbol named is only its path. Each kind is said once, with its count.
    let mut d = example();
    d.seen.dependents = vec![
        Named::symbol("home_route", "src/routes.py"),
        Named::symbol("export_report", "src/auth.py"),
        Named::file("tests/test_auth.py"),
        Named::symbol("admin_route", "src/routes.py"),
    ];
    d.seen.risks = vec![
        "contract_change".into(),
        "hotspot".into(),
        "contract_change".into(),
    ];
    d.seen.limitations = vec![];
    let text = admission::admit(&reader, &ws, &d, stamp()).unwrap().content;
    assert!(
        text.contains(
            "Dependentes: home_route, admin_route (src/routes.py); export_report \
             (src/auth.py); tests/test_auth.py. "
        ),
        "{text}"
    );
    assert!(
        text.contains("Achados: riscos: contract_change (2), hotspot; nenhuma limitação. "),
        "{text}"
    );
    assert_ne!(
        admission::admit(&reader, &ws, &d, stamp()).unwrap().node_id,
        r.node_id,
        "what the analysis found is part of the node"
    );
}

#[test]
fn a_list_over_its_cap_is_cut_at_the_end_and_says_how_many_were_left_out() {
    let dir = workspace();
    let reader = WorkspaceReader::new(dir.path()).unwrap();
    let ws = "w".repeat(64);
    let names = |prefix: &str, n: usize| -> Vec<String> {
        (0..n).map(|i| format!("{prefix}{i:02}")).collect()
    };
    let named = |prefix: &str, n: usize| -> Vec<Named> {
        names(prefix, n)
            .iter()
            .map(|s| Named::symbol(s, "src/auth.py"))
            .collect()
    };
    let mut d = example();
    d.seen.symbols = named("sym", 19);
    d.seen.dependents = named("dep", 10);
    d.seen.tests = names("tests/t.py::test", 9);
    let r = admission::admit(&reader, &ws, &d, stamp()).expect("cutting a list refuses nothing");
    let text = &r.content;
    assert!(
        text.contains("sym15 (src/auth.py) +3. ") && !text.contains("sym16"),
        "{text}"
    );
    assert!(
        text.contains("dep07 (src/auth.py) +2. ") && !text.contains("dep08"),
        "{text}"
    );
    assert!(
        text.contains("tests/t.py::test07 +1. ") && !text.contains("test08"),
        "{text}"
    );

    // Within every cap and still over the 2.000 bytes of `content`: the lists give way from
    // the end, tests first, and the record is admitted.
    let long = "x".repeat(150);
    let mut d = example();
    d.seen.symbols = named(&format!("s{long}"), 8);
    d.seen.dependents = named(&format!("d{long}"), 8);
    d.seen.tests = names(&format!("tests/{long}.py::t"), 8);
    let r = admission::admit(&reader, &ws, &d, stamp()).expect("cut to fit, not refused");
    assert!(r.content.len() <= MAX_CONTENT_BYTES, "{}", r.content.len());
    assert!(r.content.contains("Testes ligados: +8. "), "{}", r.content);
    assert!(
        r.content.contains(&format!("s{long}07 (src/auth.py). ")),
        "the symbols are the last to give way: {}",
        r.content
    );
    assert!(r.content.ends_with(&format!("{}.", r.sources[0].sha256)));
}

#[test]
fn an_identifier_shaped_like_a_secret_rejects_the_whole_observation() {
    let dir = workspace();
    let reader = WorkspaceReader::new(dir.path()).unwrap();
    let ws = "w".repeat(64);
    let token = format!("{}{}", "sk-", "a1B2c3D4e5F6g7H8i9J0k1L2");
    let email = format!("{}@{}", "maria.silva", "example.com");
    type Put = fn(&mut Seen, String);
    let places: [Put; 5] = [
        |s, v| s.symbols.push(Named::symbol(&v, "src/auth.py")),
        |s, v| s.symbols.push(Named::symbol("ok", &format!("src/{v}.py"))),
        |s, v| s.dependents.push(Named::symbol(&v, "src/auth.py")),
        |s, v| s.dependents.push(Named::file(&format!("src/{v}.py"))),
        |s, v| s.tests.push(format!("tests/test_auth.py::{v}")),
    ];
    for (i, put) in places.iter().enumerate() {
        for (value, want) in [
            (&token, Refused::SecretShaped),
            (&email, Refused::PersonalData),
        ] {
            let mut d = example();
            // Past the cap too: what would be cut is scanned all the same.
            d.seen.symbols = symbols(&["a"; 16], "src/auth.py");
            d.seen.dependents = symbols(&["b"; 8], "src/auth.py");
            d.seen.tests = vec!["tests/test_auth.py".into(); 8];
            put(&mut d.seen, value.clone());
            assert_eq!(
                admission::admit(&reader, &ws, &d, stamp()),
                Err(want),
                "place {i}"
            );
        }
    }
}

#[test]
fn a_v1_record_is_still_read_as_it_is() {
    let v1 = parse(&record()).expect("no migration: a v1 record stays valid");
    assert_eq!(v1.policy_version, "memory-policy/v1");
    assert_eq!(
        v1.content,
        "Evento: análise após edição. Escopo: src/cache.rs."
    );
    let stored = serde_json::to_value(&v1).unwrap();
    assert_eq!(stored["content"], record()["content"], "never re-rendered");
    assert_eq!(stored["policy_version"], "memory-policy/v1");
}
