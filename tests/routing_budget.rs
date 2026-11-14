//! Tool routing and budget enforcement as one contract.
//!
//! `tests/broker.rs` covers routing and shaping case by case. This target states the two
//! contracts structurally instead:
//!
//! - **Routing** is a table: every intent, every explicit mode, and the exact upstream verb
//!   sequence each produces. A regression in `router::classify`/`router::route` shows up as a
//!   row mismatch rather than as one of many unrelated unit tests failing.
//! - **Budget** is one invariant set applied uniformly to all three tools over the whole legal
//!   range, plus the floor/ceiling boundaries and the documented defaults.
//!
//! `router` is a private module, so routing is observed the way a client observes it: which
//! verbs reach the upstream and what `intent` the envelope reports.

mod common;

use common::fake::FakeUpstream;
use ripwire_broker::broker::{
    Broker, BrokerConfig, EditRequest, FinishRequest, MAX_BUDGET_TOKENS, MIN_BUDGET_TOKENS, Mode,
    TaskRequest,
};
use ripwire_broker::upstream::UpstreamError;
use serde_json::Value;
use std::sync::Arc;

async fn broker(fake: FakeUpstream) -> (Broker, Arc<FakeUpstream>, tempfile::TempDir) {
    let ws = tempfile::tempdir().unwrap();
    let fake = Arc::new(fake);
    let b = Broker::connect(fake.clone(), BrokerConfig::new(ws.path()))
        .await
        .unwrap();
    (b, fake, ws)
}

fn to_json<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}

/// The broker's documented estimate: 4 bytes of serialized JSON per token.
fn serialized_tokens(v: &Value) -> usize {
    serde_json::to_string(v).unwrap().len().div_ceil(4)
}

fn limitation_kinds(out: &Value) -> Vec<String> {
    out["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["kind"].as_str().unwrap().to_string())
        .collect()
}

const PY_TRACE: &str = "Traceback (most recent call last):\n  File \"src/routes.py\", line 9, in home_route\n    return login(req.user, req.token)\n  File \"src/auth.py\", line 7, in login\n    raise ValueError(\"bad token\")\nValueError: bad token";

/// Every verb the routing matrix can reach, each with a recorded answer.
fn routing_fake() -> FakeUpstream {
    FakeUpstream::new()
        .answer("explore", "explore_export_auth")
        .answer("find_symbol", "find_symbol_login")
        .answer("fetch_body", "fetch_body_login")
        .answer("impact", "impact_login")
        .answer("from_trace", "from_trace_login")
        .answer("memory_recall", "memory_recall_auth")
        .answer("situational_awareness", "situational_awareness_edit")
}

fn edit_fake() -> FakeUpstream {
    FakeUpstream::new().answer("situational_awareness", "situational_awareness_files")
}

fn finish_fake() -> FakeUpstream {
    FakeUpstream::new()
        .answer("situational_awareness", "situational_awareness_files")
        .answer("quality_delta", "quality_delta_clean")
        .answer("affected", "affected_auth")
}

// ------------------------------------------------------------------ routing

struct Case {
    task: &'static str,
    mode: Mode,
    intent: &'static str,
    verbs: &'static [&'static str],
}

/// Every row is a (task, mode) → (intent, upstream verbs) contract.
const ROUTES: &[Case] = &[
    // --- auto mode: the router classifies the task text
    Case {
        task: "how are the routes authenticated?",
        mode: Mode::Auto,
        intent: "orient",
        verbs: &["explore"],
    },
    Case {
        // A backticked name is the symbol, and its body is fetched with it.
        task: "how does `login` work?",
        mode: Mode::Auto,
        intent: "symbol",
        verbs: &["find_symbol", "fetch_body"],
    },
    Case {
        // A change request about a named symbol adds its blast radius.
        task: "change the signature of `login` to add a scope parameter",
        mode: Mode::Auto,
        intent: "change",
        verbs: &["find_symbol", "fetch_body", "impact"],
    },
    Case {
        // A change request with no symbol to anchor on explores instead.
        task: "altere a rota de exportação para exigir autenticação",
        mode: Mode::Auto,
        intent: "change",
        verbs: &["explore"],
    },
    Case {
        // A stack trace is the error route, and the trace goes upstream verbatim.
        task: PY_TRACE,
        mode: Mode::Auto,
        intent: "debug",
        verbs: &["from_trace"],
    },
    Case {
        task: "read the ADR on authentication",
        mode: Mode::Auto,
        intent: "docs",
        verbs: &["memory_recall"],
    },
    Case {
        task: "review my changes before I open the PR",
        mode: Mode::Auto,
        intent: "review",
        verbs: &["situational_awareness"],
    },
    // --- an explicit mode overrides what the text would have routed to
    Case {
        task: "how does `login` work?",
        mode: Mode::Orient,
        intent: "orient",
        verbs: &["explore"],
    },
    Case {
        task: PY_TRACE,
        mode: Mode::Debug,
        intent: "debug",
        verbs: &["from_trace"],
    },
    Case {
        task: "how does `login` work?",
        mode: Mode::Review,
        intent: "review",
        verbs: &["situational_awareness"],
    },
    Case {
        // `change` keeps the named symbol, so the symbol route still runs.
        task: "change the signature of `login`",
        mode: Mode::Change,
        intent: "change",
        verbs: &["find_symbol", "fetch_body", "impact"],
    },
    Case {
        // …and with nothing named, it explores, exactly as auto mode would.
        task: "how are the routes authenticated?",
        mode: Mode::Change,
        intent: "change",
        verbs: &["explore"],
    },
];

#[tokio::test]
async fn every_route_reaches_the_verbs_its_intent_requires() {
    for case in ROUTES {
        let (b, fake, _ws) = broker(routing_fake()).await;
        let mut req = TaskRequest::new(case.task);
        req.mode = case.mode;

        let out = to_json(&b.context_for_task(req).await.unwrap());

        assert_eq!(
            out["intent"], case.intent,
            "{:?} with mode {:?}: wrong intent",
            case.task, case.mode
        );
        assert_eq!(
            fake.called(),
            case.verbs,
            "{:?} with mode {:?}: wrong upstream verbs",
            case.task,
            case.mode
        );
        assert_eq!(
            out["provenance"]["upstream_tools"],
            serde_json::json!(case.verbs),
            "{:?}: the envelope must name the same verbs it called",
            case.task
        );
    }
}

#[tokio::test]
async fn a_symbol_the_repository_lacks_falls_back_to_explore() {
    let refused = UpstreamError::Refused("symbol not found".into());
    let (b, fake, _ws) = broker(
        FakeUpstream::new()
            .fail("find_symbol", refused)
            .answer("explore", "explore_export_auth"),
    )
    .await;

    let out = to_json(
        &b.context_for_task(TaskRequest::new("how does `make_widget` work?"))
            .await
            .unwrap(),
    );

    assert_eq!(fake.called(), vec!["find_symbol", "explore"]);
    assert_eq!(out["intent"], "symbol", "the intent is what was asked");
    assert!(
        limitation_kinds(&out).contains(&"symbol_not_found".to_string()),
        "the fallback must say why it explored: {out:#}"
    );
}

#[tokio::test]
async fn debug_without_a_trace_falls_back_to_explore() {
    let refused = UpstreamError::Refused("no frames found in trace".into());
    let (b, fake, _ws) = broker(
        FakeUpstream::new()
            .fail("from_trace", refused)
            .answer("explore", "explore_export_auth"),
    )
    .await;
    let mut req = TaskRequest::new("the test tests/test_auth.py is failing");
    req.mode = Mode::Debug;

    let out = to_json(&b.context_for_task(req).await.unwrap());

    assert_eq!(fake.called(), vec!["from_trace", "explore"]);
    assert_eq!(out["intent"], "debug");
    assert!(
        limitation_kinds(&out).contains(&"no_trace_frames".to_string()),
        "{out:#}"
    );
}

// ------------------------------------------------------------------- budget

/// The budget contract every tool's answer must satisfy, asserted on what a client sees.
fn assert_budget_contract(out: &Value, requested: u32, what: &str) {
    let b = &out["budget"];
    assert_eq!(
        b["requested_tokens"], requested,
        "{what}: the answer must echo the budget it was asked for"
    );

    let shown = b["shown"].as_u64().unwrap();
    let omitted = b["omitted"].as_u64().unwrap();
    let estimated = b["estimated_tokens"].as_u64().unwrap();
    let truncated = b["truncated"].as_bool().unwrap();

    let present = out["items"].as_array().unwrap().len()
        + out["tests"].as_array().unwrap().len()
        + out["risks"].as_array().unwrap().len();
    assert_eq!(
        shown as usize, present,
        "{what}: `shown` must count exactly what the answer carries"
    );
    assert_eq!(
        truncated,
        omitted > 0,
        "{what}: truncated={truncated} with {omitted} omitted"
    );
    assert_eq!(
        b.get("next_step").is_some(),
        truncated,
        "{what}: `next_step` and `truncated` disagree"
    );

    // Documented exception: limitations are never cut (PRD 10.2 #1), so an answer reduced to
    // limitations alone may pass the budget rather than drop a warning. Every answer that
    // still shows content must fit.
    if shown > 0 {
        assert!(
            estimated <= requested as u64,
            "{what}: reports {estimated} tokens against a requested {requested}"
        );
        assert!(
            serialized_tokens(out) <= requested as usize,
            "{what}: serializes to {} tokens against a requested {requested}",
            serialized_tokens(out)
        );
    }
}

/// Budgets spanning the floor, the crowded middle and the ceiling.
fn sweep() -> Vec<u32> {
    vec![
        MIN_BUDGET_TOKENS,
        300,
        400,
        800,
        1500,
        2500,
        10_000,
        MAX_BUDGET_TOKENS,
    ]
}

#[tokio::test]
async fn context_for_task_keeps_the_budget_contract_across_the_range() {
    for budget in sweep() {
        let (b, _fake, _ws) = broker(routing_fake()).await;
        let mut req = TaskRequest::new("how are the routes authenticated?");
        req.budget_tokens = budget;

        let out = to_json(&b.context_for_task(req).await.unwrap());

        assert_budget_contract(&out, budget, &format!("context_for_task at {budget}"));
    }
}

#[tokio::test]
async fn context_after_edit_keeps_the_budget_contract_across_the_range() {
    for budget in sweep() {
        let (b, _fake, _ws) = broker(edit_fake()).await;
        let req = EditRequest {
            budget_tokens: budget,
            ..Default::default()
        };

        let out = to_json(&b.context_after_edit(req).await.unwrap());

        assert_budget_contract(&out, budget, &format!("context_after_edit at {budget}"));
    }
}

#[tokio::test]
async fn context_before_finish_keeps_the_budget_contract_across_the_range() {
    for budget in sweep() {
        let (b, _fake, _ws) = broker(finish_fake()).await;
        let req = FinishRequest {
            budget_tokens: budget,
            ..Default::default()
        };

        let out = to_json(&b.context_before_finish(req).await.unwrap());

        assert_budget_contract(&out, budget, &format!("context_before_finish at {budget}"));
    }
}

#[tokio::test]
async fn the_floor_and_the_ceiling_are_enforced_before_any_upstream_call() {
    // Below the floor: refused, and ripwire is never started for it.
    for budget in [0, 1, MIN_BUDGET_TOKENS - 1] {
        let (b, fake, _ws) = broker(routing_fake()).await;

        let err = b
            .context_for_task(TaskRequest {
                budget_tokens: budget,
                ..TaskRequest::new("how are the routes authenticated?")
            })
            .await
            .unwrap_err();
        assert_eq!(err.error, "invalid_input", "task at {budget}");
        assert!(fake.called().is_empty(), "task at {budget} called upstream");

        let err = b
            .context_after_edit(EditRequest {
                budget_tokens: budget,
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert_eq!(err.error, "invalid_input", "edit at {budget}");

        let err = b
            .context_before_finish(FinishRequest {
                budget_tokens: budget,
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert_eq!(err.error, "invalid_input", "finish at {budget}");
        assert!(
            fake.called().is_empty(),
            "a refused budget must not reach the upstream"
        );
    }

    let over = MAX_BUDGET_TOKENS + 1;
    let (b, fake, _ws) = broker(routing_fake()).await;
    let err = b
        .context_for_task(TaskRequest {
            budget_tokens: over,
            ..TaskRequest::new("how are the routes authenticated?")
        })
        .await
        .unwrap_err();
    assert_eq!(err.error, "invalid_input");
    assert!(
        err.message.contains(&MAX_BUDGET_TOKENS.to_string()),
        "the refusal names the ceiling: {}",
        err.message
    );
    assert!(fake.called().is_empty());
}

#[tokio::test]
async fn the_edges_of_the_legal_range_are_accepted() {
    // Both ends are inside the contract: the floor is a budget, not an error.
    for budget in [MIN_BUDGET_TOKENS, MAX_BUDGET_TOKENS] {
        let (b, _fake, _ws) = broker(routing_fake()).await;
        let mut req = TaskRequest::new("how are the routes authenticated?");
        req.budget_tokens = budget;
        let out = b.context_for_task(req).await.unwrap();
        assert_eq!(out.budget.requested_tokens, budget);

        let (b, _fake, _ws) = broker(edit_fake()).await;
        let out = b
            .context_after_edit(EditRequest {
                budget_tokens: budget,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(out.budget.requested_tokens, budget);

        let (b, _fake, _ws) = broker(finish_fake()).await;
        let out = b
            .context_before_finish(FinishRequest {
                budget_tokens: budget,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(out.budget.requested_tokens, budget);
    }
}

#[test]
fn each_tool_declares_its_documented_default_budget() {
    assert_eq!(TaskRequest::new("t").budget_tokens, 2500);
    assert_eq!(EditRequest::default().budget_tokens, 1500);
    assert_eq!(FinishRequest::default().budget_tokens, 1800);
}

#[tokio::test]
async fn a_tighter_budget_never_shows_more_than_a_looser_one() {
    // Monotonicity for the two tools whose entry set is fixed by the recorded answer: growing
    // the budget can only let more through, never fewer.
    let limits = [
        MIN_BUDGET_TOKENS,
        300,
        400,
        600,
        800,
        1200,
        1800,
        2500,
        10_000,
    ];

    let mut last = 0usize;
    for budget in limits {
        let (b, _fake, _ws) = broker(edit_fake()).await;
        let out = b
            .context_after_edit(EditRequest {
                budget_tokens: budget,
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(
            out.budget.shown >= last,
            "context_after_edit: shown fell from {last} to {} at {budget}",
            out.budget.shown
        );
        last = out.budget.shown;
    }

    let mut last = 0usize;
    for budget in limits {
        let (b, _fake, _ws) = broker(finish_fake()).await;
        let out = b
            .context_before_finish(FinishRequest {
                budget_tokens: budget,
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(
            out.budget.shown >= last,
            "context_before_finish: shown fell from {last} to {} at {budget}",
            out.budget.shown
        );
        last = out.budget.shown;
    }
}

#[tokio::test]
async fn dropping_bodies_never_shows_fewer_items_and_never_carries_one() {
    let budget = 1200;

    let (b, _fake, _ws) = broker(routing_fake()).await;
    let mut with = TaskRequest::new("how are the routes authenticated?");
    with.budget_tokens = budget;
    let with = to_json(&b.context_for_task(with).await.unwrap());

    let (b, _fake, _ws) = broker(routing_fake()).await;
    let mut without = TaskRequest::new("how are the routes authenticated?");
    without.budget_tokens = budget;
    without.include_bodies = false;
    let without = to_json(&b.context_for_task(without).await.unwrap());

    assert!(
        without["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i.get("content").is_none()),
        "no body may survive include_bodies=false: {without:#}"
    );
    let shown = |v: &Value| v["items"].as_array().unwrap().len();
    assert!(
        shown(&without) >= shown(&with),
        "slimming bodies must not shrink what fits: {} vs {}",
        shown(&without),
        shown(&with)
    );
}

#[tokio::test]
async fn dropping_docs_removes_only_doc_roles() {
    let (b, _fake, _ws) = broker(routing_fake()).await;
    let mut req = TaskRequest::new("how are the routes authenticated?");
    req.budget_tokens = 10_000;
    req.include_docs = false;

    let out = to_json(&b.context_for_task(req).await.unwrap());

    let items = out["items"].as_array().unwrap();
    assert!(
        items.iter().all(|i| i["role"] != "doc"),
        "a doc item survived include_docs=false: {items:?}"
    );
    assert!(
        !items.is_empty(),
        "dropping docs must not drop the code evidence"
    );
}
