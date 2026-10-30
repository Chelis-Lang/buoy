#![doc = "Behavioral contracts for the pure read-only trace query."]
#![cfg(any(feature = "blake3-256", feature = "sha-256"))]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use buoy_core::{
    ActiveAlgorithm, ApprovalBinding, Atom, AtomKind, AtomRevisionRef, BindingAttachment,
    DigestAlgorithm, ImplementationBinding, ImplementationBindingRef, LifecycleState,
    NavigationKind, ParsedItemIdentity, RepositoryPath, SchemaId, ScopeId, SemanticDigest,
    SourceAuthority, SourceBudgets, SourceChildRef, SourceDepthBudget, SourceEdgeBudget,
    SourceEdgeKind, SourceGraph, SourceItemKind, SourceNode, SourceNodeBudget, SourceTraversal,
    SourceVisitedBudget, StableId, StoredBinding, TraceContext, TraceError, TraceHistory, TraceRow,
    TraceSelector, TraceState, semantic_digest_with, source_root_closure, trace_query,
};

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

/// Returns the collision-resistant member a build with one such adapter selects.
///
/// One compact locator carries no algorithm agility, so every record identity
/// in these contracts must address one collision-resistant member.
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

/// One digest under the selected member, for values outside the graph.
fn member_digest(domain: &str) -> SemanticDigest {
    semantic_digest_with(active(), domain, Vec::new()).expect("valid digest domain")
}

fn sid(value: &str) -> StableId {
    StableId::new(value).expect("valid stable ID")
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

fn parent(identity: ParsedItemIdentity, children: &[&SourceNode]) -> SourceNode {
    let references = children
        .iter()
        .enumerate()
        .map(|(position, child)| {
            SourceChildRef::new(
                SourceEdgeKind::Contains,
                u32::try_from(position).expect("bounded child position"),
                child.reference(),
            )
        })
        .collect();
    SourceNode::new_with(active(), identity, references).expect("valid parent node")
}

fn requirement_node(id: &[u8]) -> SourceNode {
    leaf(item(
        "openspec/v1",
        "openspec/specs/catalog/spec.md",
        "openspec/requirement/v1",
        id,
    ))
}

fn document_of(requirements: &[&SourceNode]) -> SourceNode {
    parent(
        item(
            "openspec/v1",
            "openspec/specs/catalog/spec.md",
            "openspec/document/v1",
            b"catalog",
        ),
        requirements,
    )
}

fn rust_node(path: &str, payload: &[u8]) -> SourceNode {
    leaf(item("rust/v1", path, "rust/item/v1", payload))
}

fn approval() -> ApprovalBinding {
    ApprovalBinding::new(
        ScopeId::new("maintainers").expect("valid scope"),
        RepositoryPath::new("reviews/approval.md").expect("valid path"),
        member_digest("test-approval"),
    )
    .expect("valid approval")
}

fn make_atom(id: &str, statement: &str) -> Atom {
    Atom::new_with(
        active(),
        sid(id),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        statement,
        Some(approval()),
    )
    .expect("valid atom")
}

fn atom_reference(atom: &Atom) -> AtomRevisionRef {
    AtomRevisionRef::new(atom.meta().id().clone(), atom.revision())
}

fn binding(
    relation: &str,
    subject: &SourceNode,
    requirement: &SourceNode,
    atom: &Atom,
) -> ImplementationBinding {
    ImplementationBinding::new_with(
        active(),
        sid(relation),
        NavigationKind::Implements,
        subject.reference(),
        requirement.reference(),
        atom_reference(atom),
    )
    .expect("valid implementation binding")
}

fn stored(record: &ImplementationBinding) -> StoredBinding {
    StoredBinding::new(record.record(), record.clone())
}

fn attach(subject: &SourceNode, record: &ImplementationBinding) -> BindingAttachment {
    BindingAttachment::new(subject.reference(), record.record())
}

fn graph_of(nodes: Vec<SourceNode>, roots: &[&SourceNode]) -> SourceGraph {
    SourceGraph::new_with(
        active(),
        nodes,
        Vec::new(),
        roots.iter().map(|node| node.reference()).collect(),
        budgets(),
    )
    .expect("valid source graph")
}

fn closure_of(graph: &SourceGraph) -> SourceTraversal {
    source_root_closure(graph).expect("bounded root closure")
}

fn requirement_kind() -> SourceItemKind {
    SourceItemKind::new("openspec/requirement/v1").expect("valid source item kind")
}

/// One complete snapshot with one fresh relation.
struct Fixture {
    graph: SourceGraph,
    closure: SourceTraversal,
    authorities: Vec<SourceAuthority>,
    attachments: Vec<BindingAttachment>,
    store: Vec<StoredBinding>,
    record: ImplementationBinding,
    subject: SourceNode,
    requirement: SourceNode,
}

fn fixture() -> Fixture {
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let requirement = requirement_node(b"CAT-001");
    let document = document_of(&[&requirement]);
    let subject = rust_node("src/lib.rs", b"pub fn render() {}");
    let record = binding("CAT-RENDER", &subject, &requirement, &atom);
    let graph = graph_of(
        vec![document.clone(), requirement.clone(), subject.clone()],
        &[&document, &subject],
    );
    let closure = closure_of(&graph);
    Fixture {
        graph,
        closure,
        authorities: vec![SourceAuthority::new(
            requirement.reference(),
            atom_reference(&atom),
        )],
        attachments: vec![attach(&subject, &record)],
        store: vec![stored(&record)],
        record,
        subject,
        requirement,
    }
}

impl Fixture {
    fn context(&self) -> TraceContext<'_> {
        TraceContext::new(
            &self.graph,
            &self.closure,
            requirement_kind(),
            &self.authorities,
            &self.attachments,
            &self.store,
        )
        .expect("valid trace context")
    }
}

// ---------------------------------------------------------------------------
// Positive contracts
// ---------------------------------------------------------------------------

#[test]
fn trace_every_typed_selector_reaches_one_current_relation() {
    let fixture = fixture();
    let context = fixture.context();
    for selector in [
        TraceSelector::RequirementId(sid("CAT-001")),
        TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceSelector::ImplementationPath(RepositoryPath::new("src/lib.rs").expect("valid path")),
        TraceSelector::Record(fixture.record.record()),
    ] {
        let projection = trace_query(&context, &selector, TraceHistory::CurrentOnly)
            .expect("one current relation");
        assert_eq!(projection.rows().len(), 1, "{selector:?}");
        let row = &projection.rows()[0];
        assert_eq!(row.state(), TraceState::Current, "{selector:?}");
        assert_eq!(row.relation(), &sid("CAT-RENDER"));
        assert_eq!(row.record(), fixture.record.record());
        assert_eq!(row.navigation(), NavigationKind::Implements);
        assert_eq!(row.implementation(), fixture.subject.reference());
        assert_eq!(row.requirement(), fixture.requirement.reference());
        assert_eq!(projection.selector(), &selector);
        assert_eq!(projection.graph(), fixture.graph.digest());
        assert!(!projection.has_stale_attachment(), "{selector:?}");
    }
}

#[test]
fn trace_current_row_reports_every_freshness_fact() {
    let fixture = fixture();
    let projection = trace_query(
        &fixture.context(),
        &TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceHistory::CurrentOnly,
    )
    .expect("one current relation");
    let freshness = projection.rows()[0].freshness();
    assert!(freshness.record());
    assert!(freshness.attachment());
    assert!(freshness.implementation());
    assert!(freshness.requirement());
    assert!(freshness.authority());
    assert!(projection.rows()[0].diagnostics().is_empty());
}

#[test]
fn trace_current_row_carries_both_logical_paths() {
    let fixture = fixture();
    let projection = trace_query(
        &fixture.context(),
        &TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceHistory::CurrentOnly,
    )
    .expect("one current relation");
    let row = &projection.rows()[0];
    assert_eq!(
        row.implementation_path().map(RepositoryPath::as_str),
        Some("src/lib.rs")
    );
    assert_eq!(
        row.requirement_path().map(RepositoryPath::as_str),
        Some("openspec/specs/catalog/spec.md")
    );
}

// ---------------------------------------------------------------------------
// Staleness contracts
// ---------------------------------------------------------------------------

#[test]
fn trace_changed_requirement_makes_the_attachment_stale() {
    let fixture = fixture();
    // The requirement node leaves the current root closure while the record and
    // its attachment stay unchanged.
    let graph = graph_of(
        vec![fixture.requirement.clone(), fixture.subject.clone()],
        &[&fixture.subject],
    );
    let closure = closure_of(&graph);
    let context = TraceContext::new(
        &graph,
        &closure,
        requirement_kind(),
        &fixture.authorities,
        &fixture.attachments,
        &fixture.store,
    )
    .expect("valid trace context");
    let projection = trace_query(
        &context,
        &TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceHistory::CurrentOnly,
    )
    .expect("one stale relation");
    let row = &projection.rows()[0];
    assert_eq!(row.state(), TraceState::Stale);
    assert!(!row.freshness().requirement());
    assert!(row.freshness().implementation());
    assert!(
        row.diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "CAB-REQUIREMENT-STALE"),
        "the row must retain the exact primary blocker"
    );
    assert!(projection.has_stale_attachment());
}

#[test]
fn trace_changed_implementation_makes_the_attachment_stale() {
    let fixture = fixture();
    // The subject node leaves the current root closure. Its attachment stays.
    let graph = graph_of(
        vec![
            document_of(&[&fixture.requirement]),
            fixture.requirement.clone(),
            fixture.subject.clone(),
        ],
        &[&document_of(&[&fixture.requirement])],
    );
    let closure = closure_of(&graph);
    let context = TraceContext::new(
        &graph,
        &closure,
        requirement_kind(),
        &fixture.authorities,
        &fixture.attachments,
        &fixture.store,
    )
    .expect("valid trace context");
    let projection = trace_query(
        &context,
        &TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceHistory::CurrentOnly,
    )
    .expect("one stale relation");
    let row = &projection.rows()[0];
    assert_eq!(row.state(), TraceState::Stale);
    assert!(!row.freshness().implementation());
    assert!(row.freshness().requirement());
    assert!(
        row.diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "CAB-SUBJECT-STALE")
    );
}

// ---------------------------------------------------------------------------
// History contracts
// ---------------------------------------------------------------------------

/// One snapshot whose store keeps an old record that no attachment selects.
fn with_history() -> (Fixture, ImplementationBinding) {
    let mut fixture = fixture();
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let old_subject = rust_node("src/lib.rs", b"pub fn render_old() {}");
    let old = binding("CAT-RENDER", &old_subject, &fixture.requirement, &atom);
    fixture.store.push(stored(&old));
    (fixture, old)
}

#[test]
fn trace_retained_record_stays_historical_without_an_attachment() {
    let (fixture, old) = with_history();
    let context = fixture.context();
    let projection = trace_query(
        &context,
        &TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceHistory::IncludeHistory,
    )
    .expect("one current and one historical record");
    assert_eq!(projection.rows().len(), 2);
    let historical = projection
        .rows()
        .iter()
        .find(|row| row.record() == old.record())
        .expect("the retained record appears");
    assert_eq!(historical.state(), TraceState::Historical);
    assert!(!historical.freshness().attachment());
    assert!(
        historical.diagnostics().is_empty(),
        "a historical record carries no primary blocker"
    );
    assert!(
        !projection.has_stale_attachment(),
        "history never changes a successful exit class"
    );
}

#[test]
fn trace_current_only_selection_hides_historical_rows() {
    let (fixture, old) = with_history();
    let projection = trace_query(
        &fixture.context(),
        &TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceHistory::CurrentOnly,
    )
    .expect("one current relation");
    assert_eq!(projection.rows().len(), 1);
    assert_eq!(projection.rows()[0].state(), TraceState::Current);
    assert!(
        projection
            .rows()
            .iter()
            .all(|row| row.record() != old.record())
    );
}

#[test]
fn trace_exact_record_selector_keeps_its_historical_row() {
    let (fixture, old) = with_history();
    let context = fixture.context();
    for history in [TraceHistory::CurrentOnly, TraceHistory::IncludeHistory] {
        let projection = trace_query(&context, &TraceSelector::Record(old.record()), history)
            .expect("one exact immutable record");
        assert_eq!(projection.rows().len(), 1, "{history:?}");
        assert_eq!(projection.rows()[0].state(), TraceState::Historical);
        assert_eq!(projection.rows()[0].record(), old.record());
        assert!(!projection.has_stale_attachment(), "{history:?}");
    }
}

#[test]
fn trace_retained_record_never_becomes_current_from_a_current_side() {
    // Both bound sides stay current, but no attachment declares the locator.
    let (fixture, old) = with_history();
    let projection = trace_query(
        &fixture.context(),
        &TraceSelector::Record(old.record()),
        TraceHistory::IncludeHistory,
    )
    .expect("one exact immutable record");
    let row = &projection.rows()[0];
    assert_eq!(row.state(), TraceState::Historical);
    assert!(row.freshness().requirement(), "the requirement is current");
    assert!(!row.freshness().attachment(), "no attachment selects it");
}

// ---------------------------------------------------------------------------
// Canonical order and duplicate rejection
// ---------------------------------------------------------------------------

#[test]
fn trace_rows_use_the_canonical_order() {
    let (fixture, _) = with_history();
    let context = fixture.context();
    let projection = trace_query(
        &context,
        &TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceHistory::IncludeHistory,
    )
    .expect("two rows");
    let states = projection
        .rows()
        .iter()
        .map(TraceRow::state)
        .collect::<Vec<_>>();
    assert_eq!(
        states,
        vec![TraceState::Current, TraceState::Historical],
        "current rows precede historical rows"
    );

    // The projection is independent of the caller-supplied store order.
    let mut reversed = fixture.store.clone();
    reversed.reverse();
    let other = TraceContext::new(
        &fixture.graph,
        &fixture.closure,
        requirement_kind(),
        &fixture.authorities,
        &fixture.attachments,
        &reversed,
    )
    .expect("valid trace context");
    let permuted = trace_query(
        &other,
        &TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceHistory::IncludeHistory,
    )
    .expect("two rows");
    assert_eq!(projection.rows(), permuted.rows());
}

#[test]
fn trace_projection_never_repeats_one_record() {
    let (fixture, _) = with_history();
    let projection = trace_query(
        &fixture.context(),
        &TraceSelector::RelationId(sid("CAT-RENDER")),
        TraceHistory::IncludeHistory,
    )
    .expect("two rows");
    let mut records = projection
        .rows()
        .iter()
        .map(|row| row.record().canonical_text())
        .collect::<Vec<_>>();
    records.sort();
    let total = records.len();
    records.dedup();
    assert_eq!(records.len(), total, "each record appears exactly once");
}

// ---------------------------------------------------------------------------
// Negative contracts
// ---------------------------------------------------------------------------

#[test]
fn trace_duplicate_record_locator_fails_closed() {
    let fixture = fixture();
    let store = vec![fixture.store[0].clone(), fixture.store[0].clone()];
    assert_eq!(
        TraceContext::new(
            &fixture.graph,
            &fixture.closure,
            requirement_kind(),
            &fixture.authorities,
            &fixture.attachments,
            &store,
        )
        .expect_err("a repeated locator must fail"),
        TraceError::DuplicateRecord
    );
}

#[test]
fn trace_record_that_contradicts_its_locator_fails_closed() {
    let fixture = fixture();
    let store = vec![StoredBinding::new(
        ImplementationBindingRef::new(member_digest("wrong-locator")),
        fixture.record.clone(),
    )];
    assert_eq!(
        TraceContext::new(
            &fixture.graph,
            &fixture.closure,
            requirement_kind(),
            &fixture.authorities,
            &fixture.attachments,
            &store,
        )
        .expect_err("a contradicted locator must fail"),
        TraceError::RecordDigestMismatch
    );
}

#[test]
fn trace_repeated_attachment_fails_closed() {
    let fixture = fixture();
    let attachments = vec![fixture.attachments[0], fixture.attachments[0]];
    assert_eq!(
        TraceContext::new(
            &fixture.graph,
            &fixture.closure,
            requirement_kind(),
            &fixture.authorities,
            &attachments,
            &fixture.store,
        )
        .expect_err("a repeated attachment must fail"),
        TraceError::DuplicateAttachment
    );
}

#[test]
fn trace_subject_that_claims_two_records_fails_closed() {
    let fixture = fixture();
    let attachments = vec![
        fixture.attachments[0],
        BindingAttachment::new(
            fixture.subject.reference(),
            ImplementationBindingRef::new(member_digest("other-record")),
        ),
    ];
    assert_eq!(
        TraceContext::new(
            &fixture.graph,
            &fixture.closure,
            requirement_kind(),
            &fixture.authorities,
            &attachments,
            &fixture.store,
        )
        .expect_err("two records under one subject must fail"),
        TraceError::AmbiguousAttachment
    );
}

#[test]
fn trace_record_claimed_by_two_subjects_fails_closed() {
    let fixture = fixture();
    let other = rust_node("src/other.rs", b"pub fn other() {}");
    let attachments = vec![
        fixture.attachments[0],
        BindingAttachment::new(other.reference(), fixture.record.record()),
    ];
    assert_eq!(
        TraceContext::new(
            &fixture.graph,
            &fixture.closure,
            requirement_kind(),
            &fixture.authorities,
            &attachments,
            &fixture.store,
        )
        .expect_err("one record under two subjects must fail"),
        TraceError::AmbiguousAttachment
    );
}

#[test]
fn trace_unmatched_selector_reports_one_input_failure() {
    let fixture = fixture();
    let context = fixture.context();
    for selector in [
        TraceSelector::RequirementId(sid("CAT-999")),
        TraceSelector::RelationId(sid("CAT-ABSENT")),
        TraceSelector::ImplementationPath(
            RepositoryPath::new("src/absent.rs").expect("valid path"),
        ),
        TraceSelector::Record(ImplementationBindingRef::new(member_digest(
            "absent-record",
        ))),
    ] {
        assert_eq!(
            trace_query(&context, &selector, TraceHistory::IncludeHistory)
                .expect_err("an unmatched selector must fail"),
            TraceError::SelectorNotFound,
            "{selector:?}"
        );
    }
}

#[test]
fn trace_rejection_carries_one_stable_code() {
    for (error, code) in [
        (TraceError::DuplicateRecord, "TRC-RECORD-DUPLICATE"),
        (TraceError::RecordDigestMismatch, "TRC-RECORD-DIGEST"),
        (TraceError::DuplicateAttachment, "TRC-ATTACHMENT-DUPLICATE"),
        (TraceError::AmbiguousAttachment, "TRC-ATTACHMENT-AMBIGUOUS"),
        (TraceError::SelectorNotFound, "TRC-SELECTOR-NOT-FOUND"),
    ] {
        assert_eq!(error.code(), code);
    }
}

#[test]
fn trace_historical_only_query_stays_successful() {
    // A query that reaches only historical rows carries no primary blocker, so
    // it never turns a successful exit class into a static blocker.
    let (fixture, old) = with_history();
    let projection = trace_query(
        &fixture.context(),
        &TraceSelector::Record(old.record()),
        TraceHistory::IncludeHistory,
    )
    .expect("one historical record");
    assert!(
        projection
            .rows()
            .iter()
            .all(|row| row.state() == TraceState::Historical)
    );
    assert!(!projection.has_stale_attachment());
}

/// A node that is retained but unreachable is not current, so the trace query
/// must not report its record as a current attachment.
#[test]
fn trace_retained_but_unreachable_subject_is_not_current() {
    let fixture = fixture();
    let graph = graph_of(
        vec![
            document_of(&[&fixture.requirement]),
            fixture.requirement.clone(),
            fixture.subject.clone(),
        ],
        &[&document_of(&[&fixture.requirement])],
    );
    let closure = closure_of(&graph);
    assert!(
        !closure.contains(fixture.subject.reference()),
        "the subject must stay outside the current closure"
    );
    let context = TraceContext::new(
        &graph,
        &closure,
        requirement_kind(),
        &fixture.authorities,
        &fixture.attachments,
        &fixture.store,
    )
    .expect("valid trace context");
    let projection = trace_query(
        &context,
        &TraceSelector::Record(fixture.record.record()),
        TraceHistory::CurrentOnly,
    )
    .expect("one exact record");
    assert_eq!(projection.rows()[0].state(), TraceState::Stale);
}

#[test]
fn trace_context_exposes_its_exact_inputs() {
    let fixture = fixture();
    let context = fixture.context();
    assert_eq!(context.store().len(), 1);
    assert_eq!(context.attachments().len(), 1);
    assert_eq!(
        context.store()[0].locator(),
        fixture.record.record(),
        "the context keeps the caller-supplied record"
    );
}

#[test]
fn trace_state_names_are_stable_and_distinct() {
    assert_eq!(TraceState::Current.name(), "current");
    assert_eq!(TraceState::Stale.name(), "stale");
    assert_eq!(TraceState::Historical.name(), "historical");
    assert!(TraceState::Current.attached());
    assert!(TraceState::Stale.attached());
    assert!(!TraceState::Historical.attached());
}

/// Binds the reviewed classification model to the production query.
///
/// The model in `tests/bounded_trace_projection_model.rs` enumerates the same
/// three facts. This control keeps production and that model in agreement for
/// each reachable combination this snapshot can build.
#[test]
fn trace_production_classification_agrees_with_the_model() {
    let fixture = fixture();
    let current_document = document_of(&[&fixture.requirement]);

    for (attached, requirement_current, implementation_current, expected) in [
        (true, true, true, TraceState::Current),
        (true, false, true, TraceState::Stale),
        (true, true, false, TraceState::Stale),
        (false, true, true, TraceState::Historical),
    ] {
        let mut roots: Vec<&SourceNode> = Vec::new();
        if requirement_current {
            roots.push(&current_document);
        }
        if implementation_current {
            roots.push(&fixture.subject);
        }
        let graph = graph_of(
            vec![
                current_document.clone(),
                fixture.requirement.clone(),
                fixture.subject.clone(),
            ],
            &roots,
        );
        let closure = closure_of(&graph);
        let attachments = if attached {
            fixture.attachments.clone()
        } else {
            Vec::new()
        };
        let context = TraceContext::new(
            &graph,
            &closure,
            requirement_kind(),
            &fixture.authorities,
            &attachments,
            &fixture.store,
        )
        .expect("valid trace context");
        let projection = trace_query(
            &context,
            &TraceSelector::Record(fixture.record.record()),
            TraceHistory::IncludeHistory,
        )
        .expect("one exact record");
        assert_eq!(
            projection.rows()[0].state(),
            expected,
            "attached={attached} requirement={requirement_current} implementation={implementation_current}"
        );
    }
}
