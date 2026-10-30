#![doc = "Seeded properties that bind the reviewed trace order to production."]
#![cfg(any(feature = "blake3-256", feature = "sha-256"))]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use buoy_core::{
    ActiveAlgorithm, Atom, AtomKind, AtomRevisionRef, BindingAttachment, DigestAlgorithm,
    ImplementationBinding, ImplementationBindingRef, LifecycleState, NavigationKind,
    ParsedItemIdentity, RepositoryPath, SchemaId, ScopeId, SemanticDigest, SourceAuthority,
    SourceBudgets, SourceChildRef, SourceDepthBudget, SourceEdgeBudget, SourceEdgeKind,
    SourceGraph, SourceItemKind, SourceNode, SourceNodeBudget, SourceVisitedBudget, StableId,
    StoredBinding, TraceContext, TraceHistory, TraceRow, TraceSelector, TraceState,
    source_root_closure, trace_query,
};
use hegel::generators as gs;
use hegel::{Hegel, TestCase};

#[path = "../../../tests/support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

/// Widest relation count one drawn snapshot builds.
const RELATION_LIMIT: usize = 4;

const fn collision_resistant_member() -> DigestAlgorithm {
    if cfg!(feature = "blake3-256") {
        DigestAlgorithm::Blake3_256
    } else {
        DigestAlgorithm::Sha256
    }
}

fn active() -> ActiveAlgorithm {
    ActiveAlgorithm::new(collision_resistant_member())
        .expect("this build contains one collision-resistant adapter")
}

fn budgets() -> SourceBudgets {
    SourceBudgets::new(
        SourceNodeBudget::new(64).expect("positive node budget"),
        SourceEdgeBudget::new(64).expect("positive edge budget"),
        SourceDepthBudget::new(16).expect("positive depth budget"),
        SourceVisitedBudget::new(64).expect("positive visited budget"),
    )
}

fn item(schema: &str, path: &str, kind: &str, payload: &[u8]) -> ParsedItemIdentity {
    ParsedItemIdentity::new_with(
        active(),
        SchemaId::new(schema).expect("valid schema"),
        RepositoryPath::new(path).expect("valid path"),
        SourceItemKind::new(kind).expect("valid source item kind"),
        payload.to_vec(),
        None,
    )
    .expect("valid parsed item")
}

fn leaf(identity: ParsedItemIdentity) -> SourceNode {
    SourceNode::new_with(active(), identity, Vec::new()).expect("valid leaf node")
}

fn requirement_kind() -> SourceItemKind {
    SourceItemKind::new("openspec/requirement/v1").expect("valid source item kind")
}

fn make_atom() -> Atom {
    Atom::new_with(
        active(),
        StableId::new("CAT-001").expect("valid stable ID"),
        LifecycleState::Proposed,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        "The catalog MUST render every entry.",
        None,
    )
    .expect("valid atom")
}

/// One drawn relation before production values.
#[derive(Clone, Copy, Debug)]
struct Drawn {
    relation: u8,
    path: u8,
    payload: u8,
    attached: bool,
}

fn draw_relations(test_case: &TestCase) -> Vec<Drawn> {
    let count = test_case.draw(
        gs::integers::<usize>()
            .min_value(1)
            .max_value(RELATION_LIMIT),
    );
    (0..count)
        .map(|index| Drawn {
            // A small relation and path alphabet forces ties, so the later key
            // components decide the order rather than the relation alone.
            relation: test_case.draw(gs::integers::<u8>().min_value(0).max_value(1)),
            path: test_case.draw(gs::integers::<u8>().min_value(0).max_value(1)),
            payload: u8::try_from(index).expect("bounded relation index"),
            attached: test_case.draw(gs::booleans()),
        })
        .collect()
}

/// One complete production snapshot built from the drawn relations.
struct Snapshot {
    graph: SourceGraph,
    closure: buoy_core::SourceTraversal,
    authorities: Vec<SourceAuthority>,
    attachments: Vec<BindingAttachment>,
    store: Vec<StoredBinding>,
}

fn build(drawn: &[Drawn]) -> Snapshot {
    let atom = make_atom();
    let requirement = leaf(item(
        "openspec/v1",
        "openspec/specs/catalog/spec.md",
        "openspec/requirement/v1",
        b"CAT-001",
    ));
    let document = SourceNode::new_with(
        active(),
        item(
            "openspec/v1",
            "openspec/specs/catalog/spec.md",
            "openspec/document/v1",
            b"catalog",
        ),
        vec![SourceChildRef::new(
            SourceEdgeKind::Contains,
            0,
            requirement.reference(),
        )],
    )
    .expect("valid document node");

    let mut nodes = vec![document.clone(), requirement.clone()];
    let mut roots = vec![document.reference()];
    let mut attachments = Vec::new();
    let mut store = Vec::new();
    for entry in drawn {
        let path = if entry.path == 0 {
            "src/a.rs"
        } else {
            "src/b.rs"
        };
        let subject = leaf(item("rust/v1", path, "rust/item/v1", &[entry.payload]));
        let relation = if entry.relation == 0 {
            "CAT-ALPHA"
        } else {
            "CAT-BETA"
        };
        let record = ImplementationBinding::new_with(
            active(),
            StableId::new(relation).expect("valid stable ID"),
            NavigationKind::Implements,
            subject.reference(),
            requirement.reference(),
            AtomRevisionRef::new(atom.meta().id().clone(), atom.revision()),
        )
        .expect("valid implementation binding");
        if entry.attached {
            attachments.push(BindingAttachment::new(subject.reference(), record.record()));
        }
        roots.push(subject.reference());
        nodes.push(subject);
        store.push(StoredBinding::new(record.record(), record));
    }

    let graph = SourceGraph::new_with(active(), nodes, Vec::new(), roots, budgets())
        .expect("valid source graph");
    let closure = source_root_closure(&graph).expect("bounded root closure");
    Snapshot {
        graph,
        closure,
        authorities: vec![SourceAuthority::new(
            requirement.reference(),
            AtomRevisionRef::new(atom.meta().id().clone(), atom.revision()),
        )],
        attachments,
        store,
    }
}

/// Independent canonical key, written from the reviewed design.
fn reference_key(row: &TraceRow) -> (u8, String, String, Option<String>, String, String) {
    (
        u8::from(!row.state().attached()),
        row.relation().as_str().to_owned(),
        row.atom().id().as_str().to_owned(),
        row.implementation_path()
            .map(|path| path.as_str().to_owned()),
        row.implementation().canonical_text(),
        row.record().canonical_text(),
    )
}

#[test]
fn hegel_trace_rows_match_the_independent_canonical_order() {
    Hegel::new(|test_case: TestCase| {
        let drawn = draw_relations(&test_case);
        let snapshot = build(&drawn);
        let context = TraceContext::new(
            &snapshot.graph,
            &snapshot.closure,
            requirement_kind(),
            &snapshot.authorities,
            &snapshot.attachments,
            &snapshot.store,
        )
        .expect("valid trace context");
        let projection = trace_query(
            &context,
            &TraceSelector::RequirementId(StableId::new("CAT-001").expect("valid stable ID")),
            TraceHistory::IncludeHistory,
        )
        .expect("one requirement selects every drawn relation");

        assert_eq!(projection.rows().len(), drawn.len());
        let keys = projection
            .rows()
            .iter()
            .map(reference_key)
            .collect::<Vec<_>>();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "production must emit the canonical order");

        // Current rows precede historical rows.
        let boundary = projection
            .rows()
            .iter()
            .position(|row| row.state() == TraceState::Historical);
        if let Some(boundary) = boundary {
            assert!(
                projection.rows()[boundary..]
                    .iter()
                    .all(|row| row.state() == TraceState::Historical)
            );
        }
    })
    .settings(settings(414))
    .run();
}

#[test]
fn hegel_trace_rows_are_store_permutation_independent() {
    Hegel::new(|test_case: TestCase| {
        let drawn = draw_relations(&test_case);
        let snapshot = build(&drawn);
        let selector =
            TraceSelector::RequirementId(StableId::new("CAT-001").expect("valid stable ID"));

        let forward = TraceContext::new(
            &snapshot.graph,
            &snapshot.closure,
            requirement_kind(),
            &snapshot.authorities,
            &snapshot.attachments,
            &snapshot.store,
        )
        .expect("valid trace context");
        let first = trace_query(&forward, &selector, TraceHistory::IncludeHistory)
            .expect("one requirement selects every drawn relation");

        let mut store = snapshot.store.clone();
        store.reverse();
        let mut attachments = snapshot.attachments.clone();
        attachments.reverse();
        let reversed = TraceContext::new(
            &snapshot.graph,
            &snapshot.closure,
            requirement_kind(),
            &snapshot.authorities,
            &attachments,
            &store,
        )
        .expect("valid trace context");
        let second = trace_query(&reversed, &selector, TraceHistory::IncludeHistory)
            .expect("one requirement selects every drawn relation");

        assert_eq!(
            first.rows(),
            second.rows(),
            "a store permutation must not change the report"
        );
        assert_eq!(first.graph(), second.graph());
    })
    .settings(settings(415))
    .run();
}

#[test]
fn hegel_trace_history_selection_only_removes_historical_rows() {
    Hegel::new(|test_case: TestCase| {
        let drawn = draw_relations(&test_case);
        let snapshot = build(&drawn);
        let context = TraceContext::new(
            &snapshot.graph,
            &snapshot.closure,
            requirement_kind(),
            &snapshot.authorities,
            &snapshot.attachments,
            &snapshot.store,
        )
        .expect("valid trace context");
        let selector =
            TraceSelector::RequirementId(StableId::new("CAT-001").expect("valid stable ID"));

        let complete = trace_query(&context, &selector, TraceHistory::IncludeHistory)
            .expect("one requirement selects every drawn relation");
        let current = trace_query(&context, &selector, TraceHistory::CurrentOnly);

        let attached = complete
            .rows()
            .iter()
            .filter(|row| row.state().attached())
            .cloned()
            .collect::<Vec<_>>();
        match current {
            Ok(current) => {
                assert_eq!(
                    current.rows(),
                    attached.as_slice(),
                    "history selection removes only historical rows"
                );
                assert!(
                    current
                        .rows()
                        .iter()
                        .all(|row| row.state() != TraceState::Historical)
                );
            }
            Err(error) => {
                assert!(
                    attached.is_empty(),
                    "a current query fails only when no attachment remains: {error:?}"
                );
            }
        }

        // A retained record never becomes current, so every historical row keeps
        // an empty blocker set and never turns a query into a static blocker.
        for row in complete.rows() {
            if row.state() == TraceState::Historical {
                assert!(row.diagnostics().is_empty());
                assert!(!row.freshness().attachment());
            }
        }
    })
    .settings(settings(416))
    .run();
}

#[test]
fn hegel_trace_exact_record_selectors_survive_history() {
    Hegel::new(|test_case: TestCase| {
        let drawn = draw_relations(&test_case);
        let snapshot = build(&drawn);
        let context = TraceContext::new(
            &snapshot.graph,
            &snapshot.closure,
            requirement_kind(),
            &snapshot.authorities,
            &snapshot.attachments,
            &snapshot.store,
        )
        .expect("valid trace context");

        for member in &snapshot.store {
            let selector = TraceSelector::Record(member.locator());
            for history in [TraceHistory::CurrentOnly, TraceHistory::IncludeHistory] {
                let projection = trace_query(&context, &selector, history)
                    .expect("an exact record selector always resolves");
                assert_eq!(projection.rows().len(), 1);
                assert_eq!(projection.rows()[0].record(), member.locator());
            }
        }

        // An absent record still fails, so the selector is not a wildcard.
        let absent = ImplementationBindingRef::new(
            buoy_core::semantic_digest_with(active(), "absent-record", Vec::new())
                .expect("valid digest domain"),
        );
        if snapshot
            .store
            .iter()
            .all(|member| member.locator() != absent)
        {
            assert!(
                trace_query(
                    &context,
                    &TraceSelector::Record(absent),
                    TraceHistory::IncludeHistory
                )
                .is_err()
            );
        }
    })
    .settings(settings(417))
    .run();
}

/// Keeps the unused-import lint honest for the digest helper.
#[test]
fn trace_digest_helper_addresses_the_selected_member() {
    let digest: SemanticDigest =
        buoy_core::semantic_digest_with(active(), "control", Vec::new()).expect("valid domain");
    assert_eq!(digest.algorithm(), collision_resistant_member());
}
