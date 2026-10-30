#![doc = "Behavioral contracts for content-addressed implementation bindings and their impact."]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use buoy_core::{
    ActiveAlgorithm, ApprovalBinding, Atom, AtomKind, AtomRevisionRef, BindingAttachment,
    BindingValidationContext, CoveragePolicy, CoveragePolicyRevision, Diagnostic, DigestAlgorithm,
    DigestAssuranceClass, EnforcementMode, EvidenceRole, Impact, ImplementationBinding,
    ImplementationBindingRef, LifecycleState, NavigationKind, ObjectKind, ObjectRef,
    ParsedItemIdentity, PolicyLifecycle, Reaffirmation, RepositoryPath, SchemaId, ScopeId,
    SemanticDigest, SourceAuthority, SourceBudgets, SourceChildRef, SourceDepthBudget,
    SourceEdgeBudget, SourceEdgeKind, SourceGraph, SourceItemKind, SourceNode, SourceNodeBudget,
    SourceNodeRef, SourceTraversal, SourceVisitedBudget, SpecGraph, StableId, StoredBinding,
    TypedPayload, change_impacts, implementation_binding_diagnostics, required_role_coverage,
    source_root_closure, validate_graph,
};

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
    ParsedItemIdentity::new(
        SchemaId::new(schema).expect("valid schema"),
        RepositoryPath::new(path).expect("valid path"),
        SourceItemKind::new(kind).expect("valid source item kind"),
        payload.to_vec(),
        None,
    )
    .expect("valid parsed item")
}

fn leaf(identity: ParsedItemIdentity) -> SourceNode {
    SourceNode::new(identity, Vec::new()).expect("valid leaf node")
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
    SourceNode::new(identity, references).expect("valid parent node")
}

fn openspec(kind: &str, payload: &[u8]) -> ParsedItemIdentity {
    item(
        "openspec/v1",
        "openspec/specs/catalog/spec.md",
        kind,
        payload,
    )
}

fn rust_subject(path: &str, payload: &[u8]) -> SourceNode {
    leaf(item("rust/v1", path, "rust/function/v1", payload))
}

/// One `OpenSpec` document, requirement, scenario, and ordered steps.
struct SpecTree {
    document: SourceNode,
    requirement: SourceNode,
    scenario: SourceNode,
    steps: [SourceNode; 3],
}

fn spec_tree(result_step: &[u8]) -> SpecTree {
    let steps = [
        leaf(openspec("openspec/step/v1", b"GIVEN one catalog")),
        leaf(openspec("openspec/step/v1", b"WHEN a client renders it")),
        leaf(openspec("openspec/step/v1", result_step)),
    ];
    let scenario = parent(
        openspec("openspec/scenario/v1", b"renders-every-entry"),
        &[&steps[0], &steps[1], &steps[2]],
    );
    let requirement = parent(
        openspec("openspec/requirement/v1", b"CAT-001"),
        &[&scenario],
    );
    let document = parent(
        openspec("openspec/document/v1", b"catalog"),
        &[&requirement],
    );
    SpecTree {
        document,
        requirement,
        scenario,
        steps,
    }
}

impl SpecTree {
    fn nodes(&self) -> Vec<SourceNode> {
        let mut nodes = alloc_nodes(&[&self.document, &self.requirement, &self.scenario]);
        nodes.extend(self.steps.iter().cloned());
        nodes
    }
}

fn alloc_nodes(nodes: &[&SourceNode]) -> Vec<SourceNode> {
    nodes.iter().map(|node| (*node).clone()).collect()
}

fn approval() -> ApprovalBinding {
    ApprovalBinding::new(
        ScopeId::new("maintainers").expect("valid scope"),
        RepositoryPath::new("reviews/approval.md").expect("valid path"),
        SemanticDigest::xxh3_128([90; 16]),
    )
    .expect("valid approval")
}

fn make_atom(id: &str, statement: &str) -> Atom {
    Atom::new(
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

/// A digest whose registered member differs from the active build member.
fn foreign_digest(reference: SemanticDigest) -> SemanticDigest {
    if reference.algorithm() == DigestAlgorithm::Blake3_256 {
        SemanticDigest::xxh3_128([9; 16])
    } else {
        SemanticDigest::blake3_256([9; 32])
    }
}

fn binding(
    relation: &str,
    subject: &SourceNode,
    requirement: &SourceNode,
    atom: &Atom,
) -> ImplementationBinding {
    ImplementationBinding::new(
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

fn graph_of(nodes: Vec<SourceNode>, roots: &[&SourceNode]) -> SourceGraph {
    SourceGraph::new(
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

fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect()
}

/// The requirement kind that the `OpenSpec` adapter owns and parses for itself.
fn requirement_kind() -> SourceItemKind {
    SourceItemKind::new("openspec/requirement/v1").expect("valid source item kind")
}

/// Validates every attachment against one adapter-supplied binding context.
fn diagnose(
    attachments: &[BindingAttachment],
    store: &[StoredBinding],
    graph: &SourceGraph,
    closure: &SourceTraversal,
    authorities: &[SourceAuthority],
) -> Vec<Diagnostic> {
    implementation_binding_diagnostics(
        attachments,
        store,
        &BindingValidationContext::new(graph, closure, requirement_kind(), authorities),
    )
}

/// Adds the build-dependent digest-class fact to the expected structural facts.
///
/// Validation reports one class fact when any addressed digest is outside the
/// collision-resistant class, so a change-detection build adds that one fact to
/// every attachment.
fn expected(members: &[SemanticDigest], structural: &[&str]) -> Vec<String> {
    let mut all = structural
        .iter()
        .map(|code| (*code).to_string())
        .collect::<Vec<_>>();
    if members.iter().any(|digest| {
        digest.algorithm().assurance_class() != DigestAssuranceClass::CollisionResistant
    }) {
        all.push(String::from("CAB-DIGEST-CLASS"));
    }
    all.sort();
    all
}

#[test]
fn implementation_binding_digest_binds_every_field_under_its_versioned_domain() {
    let tree = spec_tree(b"THEN every entry appears");
    let other_tree = spec_tree(b"THEN every entry appears once");
    let subject = rust_subject("src/catalog.rs", b"fn render(catalog:&Catalog)->String{}");
    let other_subject = rust_subject("src/catalog.rs", b"fn render(catalog:&Catalog)->Text{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let other_atom = make_atom("CAT-002", "The catalog MUST render every entry.");

    let base = binding("REL-CATALOG", &subject, &tree.requirement, &atom);
    assert_eq!(base.meta().kind(), ObjectKind::ImplementationBinding);
    assert_eq!(base.meta().id(), &sid("REL-CATALOG"));
    assert_eq!(base.navigation(), NavigationKind::Implements);
    assert_eq!(base.subject(), subject.reference());
    assert_eq!(base.requirement(), tree.requirement.reference());
    assert_eq!(base.atom(), &atom_reference(&atom));
    assert_eq!(base.record().digest(), base.meta().digest());
    assert!(base.meta().references().is_empty());

    let repeated = binding("REL-CATALOG", &subject, &tree.requirement, &atom);
    assert_eq!(repeated.meta().digest(), base.meta().digest());

    let varied = [
        binding("REL-OTHER", &subject, &tree.requirement, &atom),
        binding("REL-CATALOG", &other_subject, &tree.requirement, &atom),
        binding("REL-CATALOG", &subject, &other_tree.requirement, &atom),
        binding("REL-CATALOG", &subject, &tree.requirement, &other_atom),
        ImplementationBinding::new(
            sid("REL-CATALOG"),
            NavigationKind::Owns,
            subject.reference(),
            tree.requirement.reference(),
            atom_reference(&atom),
        )
        .expect("valid navigation variant"),
    ];
    for variant in &varied {
        assert_ne!(variant.meta().digest(), base.meta().digest());
    }
}

#[test]
fn graph_validation_rejects_a_stale_binding_atom_revision() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let historical = make_atom("CAT-001", "The catalog MUST render every entry.");
    let current = make_atom("CAT-001", "The catalog MUST render every current entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &historical);
    let mut graph = SpecGraph::default();
    graph.atoms.push(current);
    graph.implementation_bindings.push(record);

    let diagnostics = validate_graph(&graph);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SPG-REF-DANGLING-OR-STALE"
            && diagnostic.kind == Some(ObjectKind::ImplementationBinding)
            && diagnostic.id.as_deref() == Some("REL-CATALOG")
    }));
}

#[test]
fn implementation_binding_rejects_references_outside_one_digest_member() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let foreign = foreign_digest(subject.digest());

    assert!(
        ImplementationBinding::new(
            sid("REL-CATALOG"),
            NavigationKind::Implements,
            SourceNodeRef::new(foreign),
            tree.requirement.reference(),
            atom_reference(&atom),
        )
        .is_err()
    );
    assert!(
        ImplementationBinding::new(
            sid("REL-CATALOG"),
            NavigationKind::Implements,
            subject.reference(),
            SourceNodeRef::new(foreign),
            atom_reference(&atom),
        )
        .is_err()
    );
    assert!(
        ImplementationBinding::new(
            sid("REL-CATALOG"),
            NavigationKind::Implements,
            subject.reference(),
            tree.requirement.reference(),
            AtomRevisionRef::new(sid("CAT-001"), foreign),
        )
        .is_err()
    );
    if let Ok(foreign_member) = ActiveAlgorithm::new(foreign.algorithm()) {
        assert!(
            ImplementationBinding::new_with(
                foreign_member,
                sid("REL-CATALOG"),
                NavigationKind::Implements,
                subject.reference(),
                tree.requirement.reference(),
                atom_reference(&atom),
            )
            .is_err(),
            "an explicit member must match all otherwise consistent references"
        );
    }
}

#[test]
fn implementation_binding_locators_parse_only_canonical_digest_text() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &atom);

    let text = record.record().canonical_text();
    assert_eq!(
        ImplementationBindingRef::parse(&text).expect("canonical locator"),
        record.record()
    );
    let uppercase = text.to_uppercase();
    for invalid in ["", "blake3-256", "md5-128:00", "blake3-256:zz", &uppercase] {
        assert!(ImplementationBindingRef::parse(invalid).is_err());
    }
}

#[test]
fn implementation_binding_validation_accepts_one_current_attached_binding() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);
    let attachment = BindingAttachment::new(subject.reference(), record.record());
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];

    let diagnostics = diagnose(
        &[attachment],
        &[stored(&record)],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(
        codes(&diagnostics),
        expected(&[record.meta().digest()], &[])
    );

    let repeated = diagnose(
        &[attachment, attachment],
        &[stored(&record), stored(&record)],
        &graph,
        &closure,
        &[authorities[0].clone(), authorities[0].clone()],
    );
    assert_eq!(repeated, diagnostics);
}

#[test]
fn implementation_binding_validation_reports_an_absent_record() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];

    let diagnostics = diagnose(
        &[BindingAttachment::new(subject.reference(), record.record())],
        &[],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(
        codes(&diagnostics),
        expected(&[record.record().digest()], &["CAB-RECORD-ABSENT"])
    );
}

#[test]
fn implementation_binding_validation_reports_one_ambiguous_locator_under_every_store_order() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &atom);
    let rival = binding("REL-RIVAL", &subject, &tree.requirement, &atom);
    assert_ne!(record, rival);

    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];
    let attachment = BindingAttachment::new(subject.reference(), record.record());
    let disagreeing = StoredBinding::new(record.record(), rival);

    let forward = diagnose(
        &[attachment],
        &[stored(&record), disagreeing.clone()],
        &graph,
        &closure,
        &authorities,
    );
    let reversed = diagnose(
        &[attachment],
        &[disagreeing, stored(&record)],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(forward, reversed);
    assert_eq!(
        codes(&forward),
        expected(&[record.record().digest()], &["CAB-RECORD-AMBIGUOUS"])
    );
    for diagnostic in &forward {
        assert_eq!(
            diagnostic.id, None,
            "an ambiguous locator settles no one relation"
        );
    }
}

#[test]
fn implementation_binding_validation_reports_a_stale_subject() {
    let tree = spec_tree(b"THEN every entry appears");
    let historical = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let current = rust_subject("src/catalog.rs", b"fn render()->Text{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &historical, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.push(historical.clone());
    nodes.push(current.clone());
    let graph = graph_of(nodes, &[&tree.document, &current]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];

    let diagnostics = diagnose(
        &[BindingAttachment::new(
            historical.reference(),
            record.record(),
        )],
        &[stored(&record)],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(
        codes(&diagnostics),
        expected(&[record.meta().digest()], &["CAB-SUBJECT-STALE"])
    );
}

#[test]
fn implementation_binding_validation_reports_a_stale_requirement() {
    let historical = spec_tree(b"THEN every entry appears");
    let current = spec_tree(b"THEN every entry appears exactly once");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &historical.requirement, &atom);

    let mut nodes = historical.nodes();
    nodes.extend(current.nodes());
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&current.document, &subject]);
    let closure = closure_of(&graph);
    let authorities = [
        SourceAuthority::new(historical.requirement.reference(), atom_reference(&atom)),
        SourceAuthority::new(current.requirement.reference(), atom_reference(&atom)),
    ];

    let diagnostics = diagnose(
        &[BindingAttachment::new(subject.reference(), record.record())],
        &[stored(&record)],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(
        codes(&diagnostics),
        expected(&[record.meta().digest()], &["CAB-REQUIREMENT-STALE"])
    );
}

#[test]
fn implementation_binding_validation_reports_each_absent_node_as_its_own_stale_side() {
    let tree = spec_tree(b"THEN every entry appears");
    let absent_tree = spec_tree(b"THEN no entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let absent_subject = rust_subject("src/report.rs", b"fn summarize()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");

    // No historical node is retained: each absent side is unknown to the graph.
    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);
    let atoms = atom_reference(&atom);

    let stale_subject = binding("REL-SUBJECT", &absent_subject, &tree.requirement, &atom);
    assert_eq!(
        codes(&diagnose(
            &[BindingAttachment::new(
                absent_subject.reference(),
                stale_subject.record()
            )],
            &[stored(&stale_subject)],
            &graph,
            &closure,
            &[SourceAuthority::new(tree.requirement.reference(), atoms)],
        )),
        expected(&[stale_subject.meta().digest()], &["CAB-SUBJECT-STALE"])
    );

    let stale_requirement = binding("REL-REQUIREMENT", &subject, &absent_tree.requirement, &atom);
    assert_eq!(
        codes(&diagnose(
            &[BindingAttachment::new(
                subject.reference(),
                stale_requirement.record()
            )],
            &[stored(&stale_requirement)],
            &graph,
            &closure,
            &[SourceAuthority::new(
                absent_tree.requirement.reference(),
                atom_reference(&atom)
            )],
        )),
        expected(
            &[stale_requirement.meta().digest()],
            &["CAB-REQUIREMENT-STALE"]
        )
    );

    let stale_pair = binding("REL-PAIR", &absent_subject, &absent_tree.requirement, &atom);
    assert_eq!(
        codes(&diagnose(
            &[BindingAttachment::new(
                absent_subject.reference(),
                stale_pair.record()
            )],
            &[stored(&stale_pair)],
            &graph,
            &closure,
            &[SourceAuthority::new(
                absent_tree.requirement.reference(),
                atom_reference(&atom)
            )],
        )),
        expected(
            &[stale_pair.meta().digest()],
            &["CAB-REQUIREMENT-STALE", "CAB-SUBJECT-STALE"]
        )
    );
}

#[test]
fn implementation_binding_validation_reports_a_wrong_requirement_kind() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.scenario, &atom);

    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.scenario.reference(),
        atom_reference(&atom),
    )];
    let attachment = BindingAttachment::new(subject.reference(), record.record());

    let diagnostics = diagnose(
        &[attachment],
        &[stored(&record)],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(
        codes(&diagnostics),
        expected(&[record.meta().digest()], &["CAB-REQUIREMENT-KIND"])
    );

    // The expected kind is one adapter-supplied parsed value, not core syntax.
    let scenario_context = BindingValidationContext::new(
        &graph,
        &closure,
        SourceItemKind::new("openspec/scenario/v1").expect("valid source item kind"),
        &authorities,
    );
    assert_eq!(
        codes(&implementation_binding_diagnostics(
            &[attachment],
            &[stored(&record)],
            &scenario_context,
        )),
        expected(&[record.meta().digest()], &[])
    );
}

#[test]
fn implementation_binding_validation_reports_mismatched_atom_authority() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let revised = make_atom("CAT-001", "The catalog MUST render every entry once.");
    let renamed = make_atom("CAT-002", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);
    let attachment = BindingAttachment::new(subject.reference(), record.record());

    for disagreeing in [
        Vec::new(),
        vec![SourceAuthority::new(
            tree.requirement.reference(),
            atom_reference(&revised),
        )],
        vec![SourceAuthority::new(
            tree.requirement.reference(),
            atom_reference(&renamed),
        )],
        vec![SourceAuthority::new(
            tree.scenario.reference(),
            atom_reference(&atom),
        )],
    ] {
        let diagnostics = diagnose(
            &[attachment],
            &[stored(&record)],
            &graph,
            &closure,
            &disagreeing,
        );
        assert_eq!(
            codes(&diagnostics),
            expected(&[record.meta().digest()], &["CAB-AUTHORITY-MISMATCH"])
        );
    }
}

#[test]
fn implementation_binding_validation_reports_ambiguous_authority_under_every_order() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let rival = make_atom("CAT-002", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);
    let attachment = BindingAttachment::new(subject.reference(), record.record());
    let governing = SourceAuthority::new(tree.requirement.reference(), atom_reference(&atom));
    let competing = SourceAuthority::new(tree.requirement.reference(), atom_reference(&rival));

    let forward = diagnose(
        &[attachment],
        &[stored(&record)],
        &graph,
        &closure,
        &[governing.clone(), competing.clone()],
    );
    let reversed = diagnose(
        &[attachment],
        &[stored(&record)],
        &graph,
        &closure,
        &[competing, governing.clone()],
    );
    assert_eq!(forward, reversed);
    assert_eq!(
        codes(&forward),
        expected(&[record.meta().digest()], &["CAB-AUTHORITY-AMBIGUOUS"])
    );

    // One repeated equal authority stays exactly one distinct correspondence.
    assert_eq!(
        codes(&diagnose(
            &[attachment],
            &[stored(&record)],
            &graph,
            &closure,
            &[governing.clone(), governing],
        )),
        expected(&[record.meta().digest()], &[])
    );
}

#[test]
fn implementation_binding_validation_reports_a_relabeled_record_digest() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &atom);
    let rival = binding("REL-RIVAL", &subject, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];

    let relabeled = StoredBinding::new(rival.record(), record.clone());
    let diagnostics = diagnose(
        &[BindingAttachment::new(subject.reference(), rival.record())],
        &[relabeled],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(
        codes(&diagnostics),
        expected(&[record.meta().digest()], &["CAB-RECORD-DIGEST"])
    );
}

#[test]
fn implementation_binding_validation_reports_a_subject_attachment_mismatch() {
    let tree = spec_tree(b"THEN every entry appears");
    let bound = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let attached = rust_subject("src/report.rs", b"fn summarize()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &bound, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.push(bound.clone());
    nodes.push(attached.clone());
    let graph = graph_of(nodes, &[&tree.document, &bound, &attached]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];

    let diagnostics = diagnose(
        &[BindingAttachment::new(
            attached.reference(),
            record.record(),
        )],
        &[stored(&record)],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(
        codes(&diagnostics),
        expected(&[record.meta().digest()], &["CAB-SUBJECT-ATTACHMENT"])
    );
}

#[test]
fn stale_bound_subject_suppresses_the_derivative_attachment_mismatch() {
    let tree = spec_tree(b"THEN every entry appears");
    let stale = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let attached = rust_subject("src/report.rs", b"fn summarize()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &stale, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.extend([stale.clone(), attached.clone()]);
    let graph = graph_of(nodes, &[&tree.document, &attached]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];
    let diagnostics = diagnose(
        &[BindingAttachment::new(
            attached.reference(),
            record.record(),
        )],
        &[stored(&record)],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(
        codes(&diagnostics),
        expected(&[record.meta().digest()], &["CAB-SUBJECT-STALE"])
    );
}

#[test]
fn implementation_binding_validation_separates_every_attachment_that_shares_one_locator() {
    let tree = spec_tree(b"THEN every entry appears");
    let bound = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let attached = rust_subject("src/report.rs", b"fn summarize()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &bound, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.push(bound.clone());
    nodes.push(attached.clone());
    let graph = graph_of(nodes, &[&tree.document, &bound, &attached]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];

    let diagnostics = diagnose(
        &[
            BindingAttachment::new(bound.reference(), record.record()),
            BindingAttachment::new(attached.reference(), record.record()),
        ],
        &[stored(&record)],
        &graph,
        &closure,
        &authorities,
    );
    let attachment_subjects = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "CAB-SUBJECT-ATTACHMENT")
        .map(|diagnostic| diagnostic.fields.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        attachment_subjects,
        [vec![
            (String::from("record"), record.record().canonical_text()),
            (
                String::from("subject"),
                attached.reference().canonical_text()
            ),
        ]],
        "only the mismatched attachment reports its own subject"
    );
    // Each attachment keeps its own facts, so no shared locator collapses them.
    let mut want = expected(&[record.meta().digest()], &[]);
    want.extend(expected(
        &[record.meta().digest()],
        &["CAB-SUBJECT-ATTACHMENT"],
    ));
    want.sort();
    assert_eq!(codes(&diagnostics), want);
}

#[test]
fn implementation_binding_validation_separates_the_compact_locator_digest_class() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);

    let resistant = ImplementationBindingRef::new(SemanticDigest::blake3_256([7; 32]));
    let detection = ImplementationBindingRef::new(SemanticDigest::xxh3_128([7; 16]));
    assert_eq!(
        codes(&diagnose(
            &[BindingAttachment::new(subject.reference(), resistant)],
            &[],
            &graph,
            &closure,
            &[],
        )),
        ["CAB-RECORD-ABSENT"]
    );
    assert_eq!(
        codes(&diagnose(
            &[BindingAttachment::new(subject.reference(), detection)],
            &[],
            &graph,
            &closure,
            &[],
        )),
        ["CAB-DIGEST-CLASS", "CAB-RECORD-ABSENT"]
    );
}

#[test]
fn implementation_binding_validation_reports_one_locator_outside_the_record_digest_member() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &atom);
    let foreign = ImplementationBindingRef::new(foreign_digest(record.meta().digest()));

    let mut nodes = tree.nodes();
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&tree.document, &subject]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];

    let diagnostics = diagnose(
        &[BindingAttachment::new(subject.reference(), foreign)],
        &[StoredBinding::new(foreign, record.clone())],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(
        codes(&diagnostics),
        expected(
            &[foreign.digest(), record.meta().digest()],
            &["CAB-DIGEST-MEMBER", "CAB-RECORD-DIGEST"]
        )
    );
}

#[test]
fn implementation_binding_diagnostics_ignore_every_input_permutation() {
    let tree = spec_tree(b"THEN every entry appears");
    let historical = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let current = rust_subject("src/catalog.rs", b"fn render()->Text{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let stale = binding("REL-CATALOG", &historical, &tree.requirement, &atom);
    let fresh = binding("REL-REPORT", &current, &tree.requirement, &atom);

    let mut nodes = tree.nodes();
    nodes.push(historical.clone());
    nodes.push(current.clone());
    let graph = graph_of(nodes, &[&tree.document, &current]);
    let closure = closure_of(&graph);
    let authorities = [SourceAuthority::new(
        tree.requirement.reference(),
        atom_reference(&atom),
    )];
    let stale_attachment = BindingAttachment::new(historical.reference(), stale.record());
    let fresh_attachment = BindingAttachment::new(current.reference(), fresh.record());

    let forward = diagnose(
        &[stale_attachment, fresh_attachment],
        &[stored(&stale), stored(&fresh)],
        &graph,
        &closure,
        &authorities,
    );
    let reversed = diagnose(
        &[fresh_attachment, stale_attachment],
        &[stored(&fresh), stored(&stale)],
        &graph,
        &closure,
        &authorities,
    );
    assert_eq!(forward, reversed);
    assert!(codes(&forward).contains(&String::from("CAB-SUBJECT-STALE")));
    for diagnostic in &forward {
        let (relation, subject) = match diagnostic.fields.as_slice() {
            [(_, value), _] if *value == stale.record().canonical_text() => {
                ("REL-CATALOG", historical.reference())
            }
            [(_, value), _] if *value == fresh.record().canonical_text() => {
                ("REL-REPORT", current.reference())
            }
            other => panic!("every binding diagnostic binds one exact locator: {other:?}"),
        };
        assert_eq!(
            diagnostic.fields[0].0, "record",
            "every binding diagnostic names its locator first"
        );
        assert_eq!(
            diagnostic.fields[1],
            (String::from("subject"), subject.canonical_text()),
            "every binding diagnostic names its exact attached subject"
        );
        assert!(
            diagnostic.id.is_none() || diagnostic.id == Some(String::from(relation)),
            "a resolved binding diagnostic names its own relation"
        );
        assert!(
            diagnostic.id.is_none() || diagnostic.kind == Some(ObjectKind::ImplementationBinding)
        );
    }
}

#[test]
fn implementation_binding_supplies_navigation_without_any_assurance_class() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &atom);

    let revision = CoveragePolicyRevision::new(
        TypedPayload::new(
            SchemaId::new("coverage-scope/v1").expect("valid schema"),
            vec![1],
        )
        .expect("valid payload"),
        vec![EvidenceRole::Positive],
        EnforcementMode::Blocking,
        ScopeId::new("catalog").expect("valid scope"),
        Vec::new(),
        None,
    )
    .expect("valid policy revision");
    let policy = CoveragePolicy::new(
        sid("POL-CATALOG"),
        revision,
        approval(),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![ObjectRef::new(
            ObjectKind::Atom,
            sid("CAT-001"),
            atom.revision(),
        )],
    )
    .expect("valid coverage policy");

    let graph = SpecGraph {
        atoms: vec![atom],
        coverage_policies: vec![policy.clone()],
        implementation_bindings: vec![record],
        ..SpecGraph::default()
    };
    let coverage = required_role_coverage(&graph, &policy);
    assert_eq!(coverage.present, []);
    assert_eq!(coverage.missing, [EvidenceRole::Positive]);
}

fn reaffirmation(relation: &str, head: SemanticDigest) -> Reaffirmation {
    Reaffirmation::new(
        sid("REV-CATALOG"),
        ObjectKind::ImplementationBinding,
        sid(relation),
        SemanticDigest::xxh3_128([1; 16]),
        head,
        approval(),
        "The reviewer reaffirmed the catalog relation.",
    )
    .expect("valid reaffirmation")
}

fn impact_of<'a>(impacts: &'a [Impact], kind: ObjectKind, id: &str) -> &'a Impact {
    impacts
        .iter()
        .find(|impact| impact.kind == kind && impact.id == sid(id))
        .expect("expected impact key")
}

#[test]
fn binding_impact_follows_one_stable_relation_id_to_dependent_objects() {
    let tree = spec_tree(b"THEN every entry appears");
    let before = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let after = rust_subject("src/catalog.rs", b"fn render()->Text{}");
    let atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let base_record = binding("REL-CATALOG", &before, &tree.requirement, &atom);
    let head_record = binding("REL-CATALOG", &after, &tree.requirement, &atom);
    assert_ne!(base_record.meta().digest(), head_record.meta().digest());

    let review = reaffirmation("REL-CATALOG", SemanticDigest::xxh3_128([2; 16]));
    let base = SpecGraph {
        atoms: vec![atom.clone()],
        implementation_bindings: vec![base_record.clone()],
        reaffirmations: vec![review.clone()],
        ..SpecGraph::default()
    };
    let head = SpecGraph {
        atoms: vec![atom],
        implementation_bindings: vec![head_record.clone()],
        reaffirmations: vec![review],
        ..SpecGraph::default()
    };

    let impacts = change_impacts(&base, &head);
    let relation = impact_of(&impacts, ObjectKind::ImplementationBinding, "REL-CATALOG");
    assert!(relation.direct);
    assert_eq!(relation.base, Some(base_record.meta().digest()));
    assert_eq!(relation.head, Some(head_record.meta().digest()));
    let dependent = impact_of(&impacts, ObjectKind::Reaffirmation, "REV-CATALOG");
    assert!(!dependent.direct);
}

#[test]
fn binding_impact_follows_the_atom_authority_edge_to_every_unchanged_relation() {
    let tree = spec_tree(b"THEN every entry appears");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let base_atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let head_atom = make_atom("CAT-001", "The catalog MUST render every entry once.");
    let record = binding("REL-CATALOG", &subject, &tree.requirement, &base_atom);

    let base = SpecGraph {
        atoms: vec![base_atom.clone()],
        implementation_bindings: vec![record.clone()],
        ..SpecGraph::default()
    };
    let head = SpecGraph {
        atoms: vec![head_atom],
        implementation_bindings: vec![record.clone()],
        ..SpecGraph::default()
    };

    let impacts = change_impacts(&base, &head);
    assert!(impact_of(&impacts, ObjectKind::Atom, "CAT-001").direct);
    let relation = impact_of(&impacts, ObjectKind::ImplementationBinding, "REL-CATALOG");
    assert!(!relation.direct);
    assert_eq!(relation.base, Some(record.meta().digest()));
    assert_eq!(relation.head, Some(record.meta().digest()));
}

#[test]
fn binding_impact_reports_a_stale_relation_rather_than_an_unchanged_one() {
    let historical = spec_tree(b"THEN every entry appears");
    let current = spec_tree(b"THEN every entry appears exactly once");
    let subject = rust_subject("src/catalog.rs", b"fn render()->String{}");
    let base_atom = make_atom("CAT-001", "The catalog MUST render every entry.");
    let head_atom = make_atom("CAT-001", "The catalog MUST render every entry once.");
    let record = binding("REL-CATALOG", &subject, &historical.requirement, &base_atom);

    let base = SpecGraph {
        atoms: vec![base_atom],
        implementation_bindings: vec![record.clone()],
        ..SpecGraph::default()
    };
    let head = SpecGraph {
        atoms: vec![head_atom.clone()],
        implementation_bindings: vec![record.clone()],
        ..SpecGraph::default()
    };
    let impacts = change_impacts(&base, &head);
    assert!(!impact_of(&impacts, ObjectKind::ImplementationBinding, "REL-CATALOG").direct);

    let mut nodes = historical.nodes();
    nodes.extend(current.nodes());
    nodes.push(subject.clone());
    let graph = graph_of(nodes, &[&current.document, &subject]);
    let closure = closure_of(&graph);
    let diagnostics = diagnose(
        &[BindingAttachment::new(subject.reference(), record.record())],
        &[stored(&record)],
        &graph,
        &closure,
        &[SourceAuthority::new(
            current.requirement.reference(),
            atom_reference(&head_atom),
        )],
    );
    assert_eq!(
        codes(&diagnostics),
        expected(&[record.meta().digest()], &["CAB-REQUIREMENT-STALE"])
    );
}
