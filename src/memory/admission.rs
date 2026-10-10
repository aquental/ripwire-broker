//! What an automatic observation may hold (PRD jev-mem §5.2): enumerated fields only, sources
//! that pass the `--online` eligibility policy, and nothing shaped like a secret or personal
//! data. In doubt, nothing is kept. A refusal names its reason and never its content.

use super::identity;
use super::model::{
    Assertion, Kind, MAX_CONTENT_BYTES, MAX_DEPENDENTS, MAX_SYMBOLS, MAX_TESTS,
    OBSERVATION_POLICY_VERSION, POLICY_VERSION, Record, Rejected, SCHEMA_VERSION, Source,
    TimestampRole,
};
use crate::online::reader::{Ineligible, WorkspaceReader};

/// Which analysis the broker observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// `context_after_edit`.
    AfterEdit,
    /// `context_before_finish`.
    BeforeFinish,
}

/// What the analysis reported. There is no "tests passed" or "fixed": the broker never knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    AnalysisCompleted,
    AttentionRequired,
}

/// The broker never runs the tests, so their outcome is always unknown to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tests {
    Unknown,
}

/// A name ripwire read from the code and the file it is in; without a name, the file alone.
/// Never a signature, a body or a value (PRD jev-mem §5.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub name: Option<String>,
    /// Relative to the workspace root.
    pub path: String,
}

impl Named {
    pub fn symbol(name: &str, path: &str) -> Self {
        Self {
            name: Some(name.into()),
            path: path.into(),
        }
    }

    pub fn file(path: &str) -> Self {
        Self {
            name: None,
            path: path.into(),
        }
    }
}

/// What the analysis found, by name (`memory-observation/v2`): each list in the order ripwire
/// gave it. Only what ripwire returned; a name the host passed and ripwire did not resolve
/// never gets here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen {
    /// The symbols in scope that ripwire resolved.
    pub symbols: Vec<Named>,
    /// Callers and dependents of what changed.
    pub dependents: Vec<Named>,
    /// Paths of the linked tests, with the test's name when ripwire gave one.
    pub tests: Vec<String>,
    /// The `kind` of each risk, repeated as often as it was found; never its message.
    pub risks: Vec<String>,
    /// The `kind` of each limitation; never its detail.
    pub limitations: Vec<String>,
}

/// What the broker saw, structured: the only input of an automatic memory. No prompt,
/// transcript, diff, file body or shell output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub event_key: String,
    pub event: Event,
    pub outcome: Outcome,
    pub tests: Tests,
    /// Paths relative to the workspace root.
    pub scope: Vec<String>,
    /// Names of the analyses the outcome rests on, such as `quality_delta`.
    pub evidence: Vec<String>,
    pub seen: Seen,
}

/// The store's part of a record: when, in which order and generation, and for how long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub observed_at_ms: u64,
    pub ingest_seq: u64,
    pub generation: u64,
    pub retention_ms: u64,
}

/// Why an observation was not admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// No eligible source to observe.
    Empty,
    Ineligible(Ineligible),
    SecretShaped,
    PersonalData,
    Record(Rejected),
}

impl Refused {
    /// A category for counting; never the refused value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Ineligible(why) => why.as_str(),
            Self::SecretShaped => "secret_shaped",
            Self::PersonalData => "personal_data",
            Self::Record(_) => "record_limit",
        }
    }
}

/// The record for `draft`, or why it is refused whole.
pub fn admit(
    reader: &WorkspaceReader,
    workspace_id: &str,
    draft: &Draft,
    stamp: Stamp,
) -> Result<Record, Refused> {
    // Every identifier, the ones a cap would cut too: one that fails refuses the whole.
    let seen = &draft.seen;
    let named = seen.symbols.iter().chain(&seen.dependents);
    let identifiers = named
        .flat_map(|n| n.name.iter().chain([&n.path]))
        .chain(&seen.tests)
        .chain(&seen.risks)
        .chain(&seen.limitations);
    for text in draft.scope.iter().chain(&draft.evidence).chain(identifiers) {
        if secret_shaped(text) {
            return Err(Refused::SecretShaped);
        }
        if personal_data(text) {
            return Err(Refused::PersonalData);
        }
    }
    let mut snapshots = Vec::new();
    for path in &draft.scope {
        snapshots.push(reader.snapshot(path).map_err(Refused::Ineligible)?);
    }
    snapshots.sort_by(|a, b| a.path.cmp(&b.path));
    snapshots.dedup_by(|a, b| a.path == b.path);
    if snapshots.is_empty() {
        return Err(Refused::Empty);
    }
    let (kind, tool, action) = match draft.event {
        Event::AfterEdit => (
            Kind::EditObservation,
            "context_after_edit",
            "analysis_after_edit",
        ),
        Event::BeforeFinish => (
            Kind::FinishObservation,
            "context_before_finish",
            "analysis_before_finish",
        ),
    };
    let paths: Vec<&str> = snapshots.iter().map(|s| s.path.as_str()).collect();
    let mut record = Record {
        schema_version: SCHEMA_VERSION,
        policy_version: OBSERVATION_POLICY_VERSION.into(),
        node_id: String::new(),
        content_hash: String::new(),
        workspace_id: workspace_id.into(),
        event_key: draft.event_key.clone(),
        kind,
        content: render(draft, &snapshots),
        observed_at_ms: stamp.observed_at_ms,
        ingest_seq: stamp.ingest_seq,
        event_time: None,
        timestamp_role: TimestampRole::Observation,
        temporal_references: vec![],
        entities: paths
            .iter()
            .map(|p| identity::file_entity(workspace_id, p))
            .collect(),
        sources: snapshots
            .iter()
            .map(|s| Source {
                path: s.path.clone(),
                sha256: s.content_hash.clone(),
                symbol: None,
                lines: None,
                verb: tool.into(),
                basis: "broker".into(),
                ripwire_version: None,
            })
            .collect(),
        revision: None,
        assertion: Some(Assertion {
            action: action.into(),
            outcome: outcome_code(draft.outcome).into(),
            scope: paths.join(","),
        }),
        types: Default::default(),
        enrichment: Default::default(),
        derived_from: vec![],
        expires_at_ms: stamp.observed_at_ms.saturating_add(stamp.retention_ms),
        generation: stamp.generation,
    };
    record.content_hash = identity::content_hash(&record);
    record.node_id = identity::node_id(workspace_id, kind, &record.content_hash);
    record.check().map_err(Refused::Record)?;
    Ok(record)
}

fn outcome_code(o: Outcome) -> &'static str {
    match o {
        Outcome::AnalysisCompleted => "analysis_completed",
        Outcome::AttentionRequired => "attention_required",
    }
}

/// `memory-observation/v2`: event, scope, what the analysis found by name, outcome with its
/// origin, evidence and source hashes, in this order. An absence is said as unknown or as
/// none, never as success. A list gives way from its end until the text fits `content`: the
/// tests first, the symbols last.
fn render(draft: &Draft, sources: &[crate::online::reader::Snapshot]) -> String {
    let seen = &draft.seen;
    let (symbols, dependents) = (unique(&seen.symbols), unique(&seen.dependents));
    let tests = unique(&seen.tests);
    let mut keep = [
        symbols.len().min(MAX_SYMBOLS),
        dependents.len().min(MAX_DEPENDENTS),
        tests.len().min(MAX_TESTS),
    ];
    loop {
        let found = format!(
            "Símbolos: {}. Dependentes: {}. Testes ligados: {}. Achados: {}; {}.",
            cut(grouped(&symbols[..keep[0]]), symbols.len() - keep[0]),
            cut(grouped(&dependents[..keep[1]]), dependents.len() - keep[1]),
            cut(tests[..keep[2]].join(", "), tests.len() - keep[2]),
            counted(&seen.risks).map_or("nenhum risco".into(), |k| format!("riscos: {k}")),
            counted(&seen.limitations)
                .map_or("nenhuma limitação".into(), |k| format!("limitações: {k}")),
        );
        let text = render_with(draft, sources, &found);
        let Some(last) = keep.iter().rposition(|n| *n > 0) else {
            return text;
        };
        if text.len() <= MAX_CONTENT_BYTES {
            return text;
        }
        keep[last] -= 1;
    }
}

fn render_with(draft: &Draft, sources: &[crate::online::reader::Snapshot], found: &str) -> String {
    let event = match draft.event {
        Event::AfterEdit => "análise após edição",
        Event::BeforeFinish => "análise antes da conclusão",
    };
    let outcome = match draft.outcome {
        Outcome::AnalysisCompleted => "análise concluída",
        Outcome::AttentionRequired => "atenção requerida",
    };
    let tests = match draft.tests {
        Tests::Unknown => "execução de testes desconhecida",
    };
    let scope: Vec<&str> = sources.iter().map(|s| s.path.as_str()).collect();
    let hashes: Vec<&str> = sources.iter().map(|s| s.content_hash.as_str()).collect();
    let mut evidence = draft.evidence.join(", ");
    if !evidence.is_empty() {
        evidence.push_str("; ");
    }
    format!(
        "Evento: {event}. Escopo: {}. {found} Observado pelo broker: {outcome}; {tests}. \
         Evidência: {evidence}revisão de fonte {}.",
        scope.join(", "),
        hashes.join(", ")
    )
}

/// `list` without its repetitions, in the order given.
fn unique<T: Clone + PartialEq>(list: &[T]) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for item in list {
        if !out.contains(item) {
            out.push(item.clone());
        }
    }
    out
}

/// `a, b (src/x.py); src/y.py`: the names under the file they are in, files in the order they
/// first appear; a file with no name is only its path.
fn grouped(list: &[Named]) -> String {
    let mut files: Vec<(&str, Vec<&str>)> = Vec::new();
    for n in list {
        let at = match files.iter().position(|(path, _)| *path == n.path) {
            Some(at) => at,
            None => {
                files.push((&n.path, vec![]));
                files.len() - 1
            }
        };
        files[at].1.extend(n.name.as_deref());
    }
    let parts: Vec<String> = files
        .iter()
        .map(|(path, names)| match names.is_empty() {
            true => path.to_string(),
            false => format!("{} ({path})", names.join(", ")),
        })
        .collect();
    parts.join("; ")
}

/// A list and how many of its entries were left out (`+3`); an empty one says so.
fn cut(shown: String, left_out: usize) -> String {
    match (shown.is_empty(), left_out) {
        (true, 0) => "nenhum".into(),
        (true, n) => format!("+{n}"),
        (false, 0) => shown,
        (false, n) => format!("{shown} +{n}"),
    }
}

/// Each kind once, in the order first found, with its count when above one; `None` for none.
fn counted(kinds: &[String]) -> Option<String> {
    let parts: Vec<String> = unique(kinds)
        .iter()
        .map(|kind| match kinds.iter().filter(|k| *k == kind).count() {
            1 => kind.clone(),
            n => format!("{kind} ({n})"),
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// Pieces of text split on anything that cannot be part of a token.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "-_.@+".contains(c)))
        .filter(|w| !w.is_empty())
}

/// Conservative: well-known credential prefixes and assignments. It does not promise to catch
/// every secret; the fields it scans are short and enumerated (PRD jev-mem §5.2).
fn secret_shaped(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let keyword = [
        "-----begin",
        "bearer ",
        "api_key",
        "apikey",
        "password=",
        "secret=",
        "token=",
    ]
    .iter()
    .any(|k| lower.contains(k));
    keyword
        || words(text).any(|w| {
            let long = w.len() >= 20;
            (long && w.starts_with("sk-"))
                || (w.starts_with("AKIA")
                    && w.len() >= 20
                    && w[4..20]
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
                || (long
                    && [
                        "ghp_",
                        "gho_",
                        "ghs_",
                        "ghu_",
                        "github_pat_",
                        "xoxb-",
                        "xoxp-",
                    ]
                    .iter()
                    .any(|p| w.starts_with(p)))
                || (w.starts_with("eyJ") && w.matches('.').count() >= 2)
        })
}

/// Identifiable personal data: an e-mail address.
fn personal_data(text: &str) -> bool {
    words(text).any(|w| match w.split_once('@') {
        Some((user, domain)) => !user.is_empty() && domain.contains('.'),
        None => false,
    })
}

/// A note the operator supplies with `memory add` (PRD jev-mem §5.2). Its text is untrusted
/// data: kept verbatim, never followed as an instruction.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Note {
    pub text: String,
    /// Workspace-relative paths the note is about.
    #[serde(default)]
    pub references: Vec<String>,
}

/// The `explicit_note` record for `note`, under the same filters as an observation.
pub fn admit_note(
    reader: &WorkspaceReader,
    workspace_id: &str,
    note: &Note,
    stamp: Stamp,
) -> Result<Record, Refused> {
    for text in std::iter::once(&note.text).chain(&note.references) {
        if secret_shaped(text) {
            return Err(Refused::SecretShaped);
        }
        if personal_data(text) {
            return Err(Refused::PersonalData);
        }
    }
    if note.text.trim().is_empty() {
        return Err(Refused::Empty);
    }
    let mut snapshots = Vec::new();
    for path in &note.references {
        snapshots.push(reader.snapshot(path).map_err(Refused::Ineligible)?);
    }
    snapshots.sort_by(|a, b| a.path.cmp(&b.path));
    snapshots.dedup_by(|a, b| a.path == b.path);
    let mut record = Record {
        schema_version: SCHEMA_VERSION,
        policy_version: POLICY_VERSION.into(),
        node_id: String::new(),
        content_hash: String::new(),
        workspace_id: workspace_id.into(),
        event_key: "operator_supplied".into(),
        kind: Kind::ExplicitNote,
        content: note.text.clone(),
        observed_at_ms: stamp.observed_at_ms,
        ingest_seq: stamp.ingest_seq,
        event_time: None,
        timestamp_role: TimestampRole::Observation,
        temporal_references: vec![],
        entities: snapshots
            .iter()
            .map(|s| identity::file_entity(workspace_id, &s.path))
            .collect(),
        sources: snapshots
            .iter()
            .map(|s| Source {
                path: s.path.clone(),
                sha256: s.content_hash.clone(),
                symbol: None,
                lines: None,
                verb: "memory_add".into(),
                basis: "operator_supplied".into(),
                ripwire_version: None,
            })
            .collect(),
        revision: None,
        assertion: None,
        types: Default::default(),
        enrichment: Default::default(),
        derived_from: vec![],
        expires_at_ms: stamp.observed_at_ms.saturating_add(stamp.retention_ms),
        generation: stamp.generation,
    };
    record.content_hash = identity::content_hash(&record);
    record.node_id = identity::node_id(workspace_id, record.kind, &record.content_hash);
    record.check().map_err(Refused::Record)?;
    Ok(record)
}
