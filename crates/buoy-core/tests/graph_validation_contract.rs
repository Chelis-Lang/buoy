#![doc = "Graph-level registry, authority, source-map, reference, lifecycle, role, and host-schema contracts."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::*;

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn sid(value: &str) -> StableId {
    StableId::new(value).expect("valid stable ID")
}

fn payload(schema: &str, bytes: Vec<u8>) -> TypedPayload {
    TypedPayload::new(SchemaId::new(schema).expect("valid schema"), bytes)
        .expect("valid graph-contract payload")
}

fn approval() -> ApprovalBinding {
    ApprovalBinding::new(
        ScopeId::new("maintainers").expect("valid scope"),
        RepositoryPath::new("reviews/approval.md").expect("approval path"),
        digest(90),
    )
    .expect("valid approval")
}

fn atom(id: &str, statement: &str) -> Atom {
    Atom::new(
        sid(id),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        statement,
        Some(approval()),
    )
    .expect("valid graph-contract atom")
}

fn namespace(id: &str, prefix: &str, authorities: Vec<&str>) -> IdNamespace {
    IdNamespace::new(
        ScopeId::new(id).expect("valid scope"),
        NamespacePrefix::new(prefix).expect("valid prefix"),
        payload("id-allocation-policy/v1", vec![1]),
        authorities
            .into_iter()
            .map(|value| ScopeId::new(value).expect("valid scope"))
            .collect(),
    )
    .expect("valid graph-contract namespace")
}

fn allocation(
    namespace_id: &str,
    id: &StableId,
    ordinal: u64,
    class: IdAllocationClass,
) -> IdAllocation {
    IdAllocation::new(
        ScopeId::new(namespace_id).expect("valid scope"),
        id.clone(),
        ordinal,
        class,
        digest(40),
    )
    .expect("valid graph-contract allocation")
}

fn approval_artifacts(value: SemanticDigest) -> ArtifactIndex {
    ArtifactIndex::new(vec![
        ArtifactEntry::new(
            RepositoryPath::new("reviews/approval.md").expect("approval artifact path"),
            "raw-artifact",
            "text/markdown",
            "review-record/v1",
            value,
        )
        .expect("approval artifact"),
    ])
    .expect("approval artifact index")
}

fn object_ref(meta: &ObjectMeta) -> ObjectRef {
    ObjectRef::new(meta.kind(), meta.id().clone(), meta.digest())
}

fn graph_codes(graph: &SpecGraph, context: &GraphValidationContext<'_>) -> Vec<String> {
    validate_graph_with_context(graph, context)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn graph_context_validates_registry_approvals_sources_and_context_shape() {
    let live_atom = atom(
        "GRAPH-001",
        "The graph MUST retain exact registry, approval, and source bindings.",
    );
    let graph = SpecGraph {
        atoms: vec![live_atom.clone()],
        ..SpecGraph::default()
    };
    let graph_namespace = namespace("graph.objects", "GRAPH", vec!["reviewers", "maintainers"]);
    let registry = IdRegistry::new(
        vec![graph_namespace.clone()],
        vec![allocation(
            graph_namespace.namespace_id().as_str(),
            live_atom.meta().id(),
            1,
            IdAllocationClass::Live,
        )],
    )
    .expect("graph registry");
    let source_path = RepositoryPath::new("spec/control.md").expect("source path");
    let source_map = SourceMap::new(vec![
        SourceMapEntry::new(
            object_ref(live_atom.meta()),
            vec![SourceRef::new(source_path.clone(), 0, 10).expect("source extent")],
            SourceRole::Authority,
        )
        .expect("source-map entry"),
    ])
    .expect("source map");
    let artifacts = approval_artifacts(digest(90));
    let schemas = vec![String::from("id-allocation-policy/v1")];
    let permissions = vec![
        ApprovalPermission::new("maintainers", ObjectKind::Atom, ApprovalAction::Ratify)
            .expect("atom ratification permission"),
    ];
    let context = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        &[],
        permissions.clone(),
        vec![(source_path.clone(), 10)],
        schemas.clone(),
    )
    .expect("complete graph context");
    assert!(validate_graph_with_context(&graph, &context).is_empty());
    assert_eq!(context.lanes(), GraphValidationLanes::default());
    let registry_only = GraphValidationLanes::none().with(GraphValidationLane::Registry);
    let narrowed = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        &[],
        permissions.clone(),
        vec![(source_path.clone(), 10)],
        schemas.clone(),
    )
    .expect("complete narrowed context")
    .with_lanes(registry_only);
    assert_eq!(narrowed.lanes(), registry_only);
    assert!(validate_graph_with_context(&graph, &narrowed).is_empty());

    assert_eq!(
        GraphValidationContext::new(
            &registry,
            &source_map,
            &artifacts,
            &[],
            permissions.clone(),
            vec![(source_path.clone(), 10), (source_path.clone(), 11)],
            schemas.clone(),
        )
        .err(),
        Some(GraphValidationContextError::DuplicateSourcePath)
    );
    assert_eq!(
        GraphValidationContext::new(
            &registry,
            &source_map,
            &artifacts,
            &[],
            vec![permissions[0].clone(), permissions[0].clone()],
            vec![(source_path.clone(), 10)],
            schemas.clone(),
        )
        .err(),
        Some(GraphValidationContextError::DuplicateApprovalPermission)
    );
    assert_eq!(
        GraphValidationContext::new(
            &registry,
            &source_map,
            &artifacts,
            &[],
            permissions.clone(),
            vec![(source_path.clone(), 10)],
            vec![
                String::from("id-allocation-policy/v1"),
                String::from("id-allocation-policy/v1"),
            ],
        )
        .err(),
        Some(GraphValidationContextError::DuplicateHostSchema)
    );
    assert_eq!(
        GraphValidationContext::new(
            &registry,
            &source_map,
            &artifacts,
            &[],
            permissions.clone(),
            vec![(source_path.clone(), 10)],
            vec![String::from("Bad_Schema")],
        )
        .err(),
        Some(GraphValidationContextError::HostSchemaId)
    );
    assert_eq!(
        ApprovalPermission::new("Bad_Authority", ObjectKind::Atom, ApprovalAction::Ratify).err(),
        Some(GraphValidationContextError::ApprovalAuthorityId)
    );

    let empty_registry = IdRegistry::default();
    let unallocated = GraphValidationContext::new(
        &empty_registry,
        &source_map,
        &artifacts,
        &[],
        permissions.clone(),
        vec![(source_path.clone(), 10)],
        Vec::new(),
    )
    .expect("unallocated control");
    assert!(
        graph_codes(&graph, &unallocated)
            .iter()
            .any(|code| code == "SCF-REGISTRY-UNALLOCATED")
    );

    let terminal_registry = IdRegistry::new(
        vec![graph_namespace.clone()],
        vec![allocation(
            graph_namespace.namespace_id().as_str(),
            live_atom.meta().id(),
            1,
            IdAllocationClass::Terminal,
        )],
    )
    .expect("wrong-class registry");
    let wrong_class = GraphValidationContext::new(
        &terminal_registry,
        &source_map,
        &artifacts,
        &[],
        permissions.clone(),
        vec![(source_path.clone(), 10)],
        schemas.clone(),
    )
    .expect("wrong-class context");
    assert!(
        graph_codes(&graph, &wrong_class)
            .iter()
            .any(|code| code == "SCF-REGISTRY-CLASS")
    );

    let unauthorized_namespace = namespace("graph.objects", "GRAPH", vec!["reviewers"]);
    let unauthorized_registry = IdRegistry::new(
        vec![unauthorized_namespace],
        registry.allocations().to_vec(),
    )
    .expect("unauthorized registry");
    let unauthorized = GraphValidationContext::new(
        &unauthorized_registry,
        &source_map,
        &artifacts,
        &[],
        permissions.clone(),
        vec![(source_path.clone(), 10)],
        schemas.clone(),
    )
    .expect("unauthorized context");
    assert!(
        graph_codes(&graph, &unauthorized)
            .iter()
            .any(|code| code == "SCF-APPROVAL-AUTHORITY")
    );

    let unauthorized_action = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        &[],
        Vec::new(),
        vec![(source_path.clone(), 10)],
        schemas.clone(),
    )
    .expect("unauthorized action context");
    assert!(
        graph_codes(&graph, &unauthorized_action)
            .iter()
            .any(|code| code == "SCF-APPROVAL-ACTION")
    );

    let stale_artifacts = approval_artifacts(digest(91));
    let stale_approval = GraphValidationContext::new(
        &registry,
        &source_map,
        &stale_artifacts,
        &[],
        permissions.clone(),
        vec![(source_path.clone(), 10)],
        schemas.clone(),
    )
    .expect("stale approval context");
    assert!(
        graph_codes(&graph, &stale_approval)
            .iter()
            .any(|code| code == "SCF-APPROVAL-STALE")
    );

    let changed_atom = atom(
        "GRAPH-001",
        "The changed graph MUST retain exact registry, approval, and source bindings.",
    );
    let changed_graph = SpecGraph {
        atoms: vec![changed_atom],
        ..SpecGraph::default()
    };
    assert!(
        graph_codes(&changed_graph, &context)
            .iter()
            .any(|code| code == "SCF-SOURCE-MAP-DANGLING")
    );

    let short_extent = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        &[],
        permissions,
        vec![(source_path, 9)],
        schemas,
    )
    .expect("short source context");
    assert!(
        graph_codes(&graph, &short_extent)
            .iter()
            .any(|code| code == "SCF-SOURCE-MAP-SPAN")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn graph_validation_closes_atom_parsed_role_and_host_schema_references() {
    let live_atom = atom("CTX-ATOM", "Context validation MUST remain exact.");
    let parsed = ParsedItem::new(
        SchemaId::new("rust/v1").expect("valid schema"),
        RepositoryPath::new("src/context.rs").expect("parsed path"),
        SourceItemKind::new("rust/item/v1").expect("source item kind"),
        vec![1, 2, 3],
        None,
    )
    .expect("parsed item");
    let oracle = Oracle::new(
        sid("CTX-ORACLE"),
        OracleKind::Test,
        ToolIdentity::new(
            ScopeId::new("context.runner").expect("valid scope"),
            "1",
            digest(30),
            digest(31),
        )
        .expect("oracle runner"),
        payload("oracle-selection/v1", Vec::new()),
        payload("verdict-normalization/v1", Vec::new()),
        digest(32),
        payload("oracle-scope/v1", Vec::new()),
        Vec::new(),
    )
    .expect("context oracle");
    let carrier = EvidenceCarrier::new(
        sid("CTX-CARRIER"),
        EvidenceRole::Property,
        vec![AtomRevisionRef::new(
            live_atom.meta().id().clone(),
            live_atom.revision(),
        )],
        parsed.digest(),
        object_ref(oracle.meta()),
        digest(33),
        Vec::new(),
        payload("carrier-scope/v1", Vec::new()),
        Vec::new(),
        Vec::new(),
    )
    .expect("context carrier");
    let graph = SpecGraph {
        atoms: vec![live_atom.clone()],
        evidence_carriers: vec![carrier],
        oracles: vec![oracle],
        ..SpecGraph::default()
    };
    let namespace = namespace("context.objects", "CTX", vec!["maintainers"]);
    let allocations = graph
        .object_meta()
        .into_iter()
        .enumerate()
        .map(|(index, meta)| {
            allocation(
                namespace.namespace_id().as_str(),
                meta.id(),
                u64::try_from(index + 1).expect("small ordinal"),
                IdAllocationClass::Live,
            )
        })
        .collect();
    let registry = IdRegistry::new(vec![namespace], allocations).expect("context registry");
    let source_map = SourceMap::new(Vec::new()).expect("empty source map");
    let artifacts = approval_artifacts(digest(90));
    let schemas = vec![
        String::from("id-allocation-policy/v1"),
        String::from("rust/v1"),
        String::from("oracle-selection/v1"),
        String::from("verdict-normalization/v1"),
        String::from("oracle-scope/v1"),
        String::from("carrier-scope/v1"),
    ];
    let permissions = vec![
        ApprovalPermission::new("maintainers", ObjectKind::Atom, ApprovalAction::Ratify)
            .expect("atom ratification permission"),
    ];
    let context = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        core::slice::from_ref(&parsed),
        permissions.clone(),
        Vec::new(),
        schemas.clone(),
    )
    .expect("complete context");
    let codes = graph_codes(&graph, &context);
    assert!(codes.iter().any(|code| code == "SPG-REF-ROLE-INCOMPATIBLE"));
    assert!(!codes.iter().any(|code| code == "SCF-SOURCE-MAP-ITEM-STALE"));
    assert!(!codes.iter().any(|code| code == "SPG-REF-HOST-SCHEMA"));

    let missing_parsed = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        &[],
        permissions.clone(),
        Vec::new(),
        schemas,
    )
    .expect("missing parsed-item control");
    assert!(
        graph_codes(&graph, &missing_parsed)
            .iter()
            .any(|code| code == "SCF-SOURCE-MAP-ITEM-STALE")
    );

    let missing_schema = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        core::slice::from_ref(&parsed),
        permissions,
        Vec::new(),
        vec![
            String::from("id-allocation-policy/v1"),
            String::from("rust/v1"),
        ],
    )
    .expect("missing host-schema control");
    assert!(
        graph_codes(&graph, &missing_schema)
            .iter()
            .any(|code| code == "SPG-REF-HOST-SCHEMA")
    );

    let nested = ParsedItem::new(
        SchemaId::new("rust/v1").expect("valid schema"),
        RepositoryPath::new("src/nested.rs").expect("nested path"),
        SourceItemKind::new("rust/item/v1").expect("source item kind"),
        vec![4, 5, 6],
        Some(digest(250)),
    )
    .expect("nested parsed item");
    let empty_registry = IdRegistry::default();
    let empty_artifacts = ArtifactIndex::default();
    let nested_context = GraphValidationContext::new(
        &empty_registry,
        &source_map,
        &empty_artifacts,
        core::slice::from_ref(&nested),
        Vec::new(),
        Vec::new(),
        vec![String::from("rust/v1")],
    )
    .expect("nested context");
    assert!(
        graph_codes(&SpecGraph::default(), &nested_context)
            .iter()
            .any(|code| code == "SCF-SOURCE-MAP-ENCLOSING")
    );

    let stale_link = ImplementationLink::new(
        sid("CTX-LINK"),
        vec![AtomRevisionRef::new(
            live_atom.meta().id().clone(),
            digest(251),
        )],
        parsed.digest(),
        NavigationKind::Implements,
    )
    .expect("stale link");
    let stale_graph = SpecGraph {
        atoms: vec![live_atom],
        implementation_links: vec![stale_link],
        ..SpecGraph::default()
    };
    assert!(
        validate_graph(&stale_graph)
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-REF-DANGLING-OR-STALE")
    );
}

#[test]
fn graph_history_retains_governance_and_registry_terminal_records() {
    let subject = ObjectRef::new(ObjectKind::Atom, sid("HISTORY-SUBJECT"), digest(60));
    let obligation = ObligationRef::new(
        sid("HISTORY-POLICY"),
        ObligationKind::Authority,
        subject.clone(),
        None,
    )
    .expect("history obligation");
    let revision = CoveragePolicyRevision::new(
        payload("policy-scope/v1", vec![1]),
        Vec::new(),
        EnforcementMode::Blocking,
        ScopeId::new("history").expect("valid scope"),
        Vec::new(),
        None,
    )
    .expect("history policy revision");
    let terminal_policy = CoveragePolicy::new(
        sid("HISTORY-POLICY"),
        revision,
        approval(),
        PolicyLifecycle::Retired,
        Vec::new(),
        Some(approval()),
        Vec::new(),
    )
    .expect("terminal policy");
    let terminal_debt = DebtRecord::new(
        sid("HISTORY-DEBT"),
        vec![obligation.clone()],
        vec![subject.clone()],
        "Reviewed debt rationale.",
        ScopeId::new("maintainers").expect("valid scope"),
        "Complete the missing obligation.",
        payload("debt-discharge/v1", vec![1]),
        Vec::new(),
        DebtLifecycle::Discharged,
        vec![subject],
        Vec::new(),
        Some(approval()),
    )
    .expect("terminal debt");
    let terminal_waiver = Waiver::new(
        sid("HISTORY-WAIVER"),
        vec![obligation],
        Vec::new(),
        "Reviewed waiver rationale.",
        ScopeId::new("maintainers").expect("valid scope"),
        payload("waiver-discharge/v1", vec![1]),
        approval(),
        Vec::new(),
        WaiverLifecycle::Revoked,
        Vec::new(),
        Vec::new(),
        Some(approval()),
    )
    .expect("terminal waiver");
    let base = SpecGraph {
        coverage_policies: vec![terminal_policy],
        debt_records: vec![terminal_debt],
        waivers: vec![terminal_waiver],
        ..SpecGraph::default()
    };
    assert!(
        validate_graph_history(&base, &base, &IdRegistry::default(), &IdRegistry::default(),)
            .is_empty()
    );

    let removed = validate_graph_history(
        &base,
        &SpecGraph::default(),
        &IdRegistry::default(),
        &IdRegistry::default(),
    );
    assert_eq!(
        removed
            .iter()
            .filter(|diagnostic| diagnostic.code == "SCF-LIFECYCLE-TERMINAL-REMOVED")
            .count(),
        3
    );

    let history_namespace = namespace("history.objects", "HISTORY", vec!["maintainers"]);
    let base_registry = IdRegistry::new(
        vec![history_namespace],
        vec![allocation(
            "history.objects",
            &sid("HISTORY-RESERVED"),
            1,
            IdAllocationClass::Terminal,
        )],
    )
    .expect("base history registry");
    assert!(
        validate_graph_history(
            &SpecGraph::default(),
            &SpecGraph::default(),
            &base_registry,
            &IdRegistry::default(),
        )
        .iter()
        .any(|diagnostic| diagnostic.code == "SCF-REGISTRY-HISTORY")
    );

    let independent_fixture = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/hardening/v3/schema-state-fixtures.tsv"
    ));
    for expected in [
        "SCF-APPROVAL-STALE",
        "SCF-APPROVAL-AUTHORITY",
        "SCF-APPROVAL-ACTION",
        "SCF-REGISTRY-HISTORY",
        "SCF-SOURCE-MAP-DANGLING",
        "SCF-SOURCE-MAP-SPAN",
        "SCF-LIFECYCLE-TERMINAL-REMOVED",
    ] {
        assert!(
            independent_fixture.contains(expected),
            "independent fixture omits {expected}"
        );
    }
}

#[test]
fn external_trace_link_accepts_the_current_semantic_atom_revision() {
    let current = atom(
        "TRACE-REVISION-001",
        "The external trace link MUST bind the semantic atom revision.",
    );
    assert_ne!(
        current.revision(),
        current.meta().digest(),
        "the regression requires distinct digest domains"
    );
    let link = ExternalTraceLink::new(
        sid("TRACE-REVISION-LINK"),
        vec![AtomRevisionRef::new(
            current.meta().id().clone(),
            current.revision(),
        )],
        ExternalTraceRelation::Documents,
        ExternalTraceTarget::new("urn:buoy:trace-page:v1").expect("trace target"),
    )
    .expect("current external trace link");
    let graph = SpecGraph {
        atoms: vec![current],
        external_trace_links: vec![link],
        ..SpecGraph::default()
    };

    let diagnostics = validate_graph(&graph);
    assert!(
        diagnostics.is_empty(),
        "a current semantic revision must be fresh: {diagnostics:?}"
    );
}

#[test]
fn external_trace_link_rejects_an_atom_object_digest_as_a_revision() {
    let current = atom(
        "TRACE-OBJECT-001",
        "The external trace link MUST reject an atom object digest.",
    );
    assert_ne!(current.revision(), current.meta().digest());
    let link = ExternalTraceLink::new(
        sid("TRACE-OBJECT-LINK"),
        vec![AtomRevisionRef::new(
            current.meta().id().clone(),
            current.meta().digest(),
        )],
        ExternalTraceRelation::Documents,
        ExternalTraceTarget::new("urn:buoy:trace-page:v1").expect("trace target"),
    )
    .expect("malformed external trace link");
    let graph = SpecGraph {
        atoms: vec![current],
        external_trace_links: vec![link],
        ..SpecGraph::default()
    };

    let diagnostics = validate_graph(&graph);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "SPG-REF-DANGLING-OR-STALE")
            .count(),
        1,
        "an atom object digest must not satisfy semantic revision closure"
    );
}
