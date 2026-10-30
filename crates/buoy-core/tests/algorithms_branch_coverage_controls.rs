#![doc = "Asserted branch controls for graph, proof, impact, and disposition algorithms."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::*;

fn sid(value: &str) -> StableId {
    StableId::new(value).expect("valid stable ID")
}

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn path(value: &str) -> RepositoryPath {
    RepositoryPath::new(value).expect("valid repository path")
}

fn payload(schema: &str, bytes: &[u8]) -> TypedPayload {
    TypedPayload::new(SchemaId::new(schema).expect("valid schema"), bytes.to_vec())
        .expect("valid typed payload")
}

fn approval() -> ApprovalBinding {
    ApprovalBinding::new(
        ScopeId::new("maintainers").expect("valid scope"),
        path("reviews/approval.md"),
        digest(90),
    )
    .expect("valid approval")
}

fn tool(id: &str, byte: u8) -> ToolIdentity {
    ToolIdentity::new(
        ScopeId::new(id).expect("valid scope"),
        "1.0",
        digest(byte),
        digest(byte.wrapping_add(1)),
    )
    .expect("valid tool")
}

fn object_ref(meta: &ObjectMeta) -> ObjectRef {
    ObjectRef::new(meta.kind(), meta.id().clone(), meta.digest())
}

fn repository_binding(byte: u8) -> ArtifactBinding {
    ArtifactBinding::repository(
        path("proofs/proof.smt2"),
        "raw-artifact",
        "text/plain",
        "smt2/v1",
        digest(byte),
    )
    .expect("valid repository binding")
}

#[test]
fn remaining_identifier_conversion_and_encoding_errors_are_reachable() {
    for invalid in [
        "https:///path",
        "https://a..b/path",
        "https://a-.b/path",
        "https://a_b/path",
        "https://a.com#fragment",
        "https://a.com/a\\b",
        "https://a.com/a b",
        "https://a.com/a\u{7}b",
        "urn:a:b",
        "urn:a::v1",
        "urn:a:b:v",
    ] {
        assert!(ExternalTraceTarget::new(invalid).is_err(), "{invalid}");
    }
    assert!(ExternalTraceTarget::new("urn:a:b/x:v1").is_ok());
    assert_eq!(
        IdentifierError::SourceItemKindGrammar.code(),
        "SPG-SOURCE-ITEM-KIND"
    );
    assert_eq!(
        IdentifierError::ExternalTraceTargetGrammar.code(),
        "SPG-EXTERNAL-TRACE-TARGET"
    );
    let scope = ScopeId::new("coverage-scope").expect("valid scope");
    assert!(scope.eq("coverage-scope"));

    for (error, code) in [
        (IdentifierError::ScopeIdGrammar, "SCF-CONSTRUCT-SCOPE-ID"),
        (
            IdentifierError::SourceItemKindGrammar,
            "SPG-SOURCE-ITEM-KIND",
        ),
        (
            IdentifierError::ExternalTraceTargetGrammar,
            "SPG-EXTERNAL-TRACE-TARGET",
        ),
    ] {
        assert_eq!(ConstructionError::from(error).code(), code);
    }
    assert_eq!(
        semantic_digest_v2(
            "coverage-control",
            vec![
                (1, CanonicalValue::Text(String::from("first"))),
                (1, CanonicalValue::Text(String::from("duplicate"))),
            ],
        ),
        Err(EncodingError::InvalidRecordTags)
    );

    let atom = Atom::new(
        sid("TRACE-ATOM"),
        LifecycleState::Proposed,
        AtomKind::Behavioral,
        vec![ScopeId::new("trace").expect("valid scope")],
        "The trace target MUST remain exact.",
        None,
    )
    .expect("valid atom");
    let link = ExternalTraceLink::new(
        sid("TRACE-LINK"),
        vec![AtomRevisionRef::new(
            atom.meta().id().clone(),
            atom.revision(),
        )],
        ExternalTraceRelation::Documents,
        ExternalTraceTarget::new("urn:buoy:document:v1").expect("valid target"),
    )
    .expect("valid trace link");
    let graph = SpecGraph {
        atoms: vec![atom],
        external_trace_links: vec![link.clone(), link],
        ..SpecGraph::default()
    };
    assert_eq!(
        semantic_graph_digest(&graph),
        Err(EncodingError::DuplicateSetValue)
    );
}

#[test]
fn generic_model_constructor_controls_reject_each_policy_and_selector_shape() {
    assert_eq!(
        TypedSelector::new(
            path("src"),
            SchemaId::new("rust/v1").expect("valid schema"),
            Vec::new(),
            Vec::new(),
            AuthorityExpectation::Required,
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );

    let revision = || {
        CoveragePolicyRevision::new(
            payload("policy-scope/v1", &[1]),
            vec![EvidenceRole::Positive],
            EnforcementMode::Blocking,
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("valid policy revision")
    };
    let wrong_kind = ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(91));
    assert_eq!(
        CoveragePolicy::new(
            sid(&String::from("COV-POLICY")),
            revision(),
            approval(),
            PolicyLifecycle::Superseded,
            vec![wrong_kind],
            Some(approval()),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    let self_reference = ObjectRef::new(ObjectKind::CoveragePolicy, sid("COV-POLICY"), digest(92));
    assert_eq!(
        CoveragePolicy::new(
            sid(&String::from("COV-POLICY")),
            revision(),
            approval(),
            PolicyLifecycle::Superseded,
            vec![self_reference],
            Some(approval()),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );
    assert_eq!(
        CoveragePolicy::new(
            sid(&String::from("COV-POLICY")),
            revision(),
            approval(),
            PolicyLifecycle::Active,
            Vec::new(),
            Some(approval()),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::LifecycleShape)
    );
}

#[test]
fn proof_invalidation_reports_all_independent_stale_dimensions() {
    let translator = tool("translator", 1);
    let verifier = tool("verifier", 3);
    let proof = ProofRecord::new(
        sid(&String::from("COV-PROOF")),
        vec![AtomRevisionRef::new(sid("COV-ATOM"), digest(5))],
        ObjectRef::new(ObjectKind::FormalProperty, sid("COV-PROPERTY"), digest(6)),
        ArtifactBinding::object(ObjectRef::new(
            ObjectKind::ImplementationLink,
            sid("COV-IMPLEMENTATION"),
            digest(7),
        )),
        vec![ObjectRef::new(
            ObjectKind::Assumption,
            sid("COV-ASSUMPTION"),
            digest(8),
        )],
        vec![translator.clone()],
        verifier.clone(),
        digest(9),
        vec![repository_binding(10)],
        vec![repository_binding(11)],
        vec![repository_binding(12)],
        Verdict::Fail,
        payload("proof-scope/v1", &[1]),
        vec![String::from("trusted-kernel")],
    )
    .expect("valid stale proof record");
    let artifacts = ArtifactIndex::default();
    let mismatching_expectation = ProofExpectation::new(
        sid("COV-PROOF"),
        digest(13),
        payload("proof-scope/v1", &[2]),
        vec![String::from("different-exclusion")],
        false,
        false,
    );
    let context = ProofInvalidationContext::new(
        &artifacts,
        Vec::new(),
        vec![mismatching_expectation.clone()],
    )
    .expect("valid proof context");
    assert_eq!(
        proof_invalidation_reasons(&SpecGraph::default(), &proof, &context),
        [
            "FPR-BIND-ASSUMPTION",
            "FPR-BIND-ATOM",
            "FPR-BIND-CORRESPONDENCE",
            "FPR-BIND-EXCLUSIONS",
            "FPR-BIND-EXTERNAL-MODEL",
            "FPR-BIND-GENERATED",
            "FPR-BIND-IMPLEMENTATION",
            "FPR-BIND-PROOF-SOURCE",
            "FPR-BIND-PROPERTY",
            "FPR-BIND-SCOPE",
            "FPR-BIND-TRANSLATOR",
            "FPR-BIND-VERIFIER",
            "FPR-BIND-VERIFIER-CONFIGURATION",
            "FPR-VERDICT-NEGATIVE-CONTROL",
            "FPR-VERDICT-NONPASS",
        ]
    );
    assert!(!proof_fresh_with_context(
        &SpecGraph::default(),
        &proof,
        &context
    ));

    let no_expectation = ProofInvalidationContext::new(
        &artifacts,
        vec![translator.clone(), verifier.clone()],
        Vec::new(),
    )
    .expect("valid proof context without expectation");
    assert!(
        proof_invalidation_reasons(&SpecGraph::default(), &proof, &no_expectation)
            .contains(&String::from("FPR-BIND-EXPECTATION"))
    );

    assert_eq!(
        ProofInvalidationContext::new(
            &artifacts,
            vec![translator.clone(), translator],
            Vec::new(),
        )
        .err(),
        Some(ProofInvalidationContextError::DuplicateTool)
    );
    assert_eq!(
        ProofInvalidationContext::new(
            &artifacts,
            Vec::new(),
            vec![mismatching_expectation.clone(), mismatching_expectation],
        )
        .err(),
        Some(ProofInvalidationContextError::DuplicateExpectation)
    );
}

#[test]
fn schema_impacts_cover_object_domains_errors_and_transitive_fixed_point() {
    let catalog_edge = SCHEMA_IMPACT_EDGES[0];
    assert!(!catalog_edge.id().is_empty());
    assert!(!catalog_edge.source().is_empty());
    assert!(!catalog_edge.target().is_empty());

    let object_domains = [
        "atom-object",
        "tombstone",
        "decision-surface",
        "evidence-carrier",
        "implementation-link",
        "derivation",
        "coverage-policy-object",
        "debt-record",
        "waiver",
        "oracle",
        "finite-model",
        "formal-property",
        "assumption",
        "proof-record",
        "parsed-item-identity",
        "reaffirmation",
        "change-manifest",
        "change-disposition",
    ];
    for kind in object_domains {
        let target = SchemaImpactKey::new(kind, format!("{kind}-id")).expect("known domain");
        let dependency = SchemaImpactDependency::new("repository-config-policies", target.clone())
            .expect("object-reference polymorphism accepts every object domain");
        assert_eq!(dependency.edge_id(), "repository-config-policies");
        assert_eq!(dependency.target(), &target);
        assert_eq!(target.kind(), kind);
        assert_eq!(target.id(), format!("{kind}-id"));
    }

    let atom_key = SchemaImpactKey::new("atom-object", "COV-ATOM").expect("atom key");
    let config_key = SchemaImpactKey::new("repository-config", "repository").expect("config key");
    let dependency = SchemaImpactDependency::new("repository-config-policies", atom_key.clone())
        .expect("config dependency");
    let base_atom =
        SchemaImpactNode::new(atom_key.clone(), digest(20), Vec::new()).expect("base atom node");
    let head_atom =
        SchemaImpactNode::new(atom_key.clone(), digest(21), Vec::new()).expect("head atom node");
    let config = SchemaImpactNode::new(config_key.clone(), digest(22), vec![dependency.clone()])
        .expect("config node");
    assert_eq!(config.key(), &config_key);
    assert_eq!(config.digest(), digest(22));
    assert_eq!(config.dependencies(), std::slice::from_ref(&dependency));

    let base = SchemaImpactIndex::new(vec![base_atom, config.clone()]).expect("base index");
    let head = SchemaImpactIndex::new(vec![head_atom, config]).expect("head index");
    assert_eq!(base.len(), 2);
    assert!(!base.is_empty());
    assert!(base.get(&atom_key).is_some());
    let impacts = schema_change_impacts(&base, &head);
    assert_eq!(impacts.len(), 2);
    let atom_impact = impacts
        .iter()
        .find(|impact| impact.key() == &atom_key)
        .expect("direct atom impact");
    assert_eq!(atom_impact.base(), Some(digest(20)));
    assert_eq!(atom_impact.head(), Some(digest(21)));
    assert!(atom_impact.direct());
    let config_impact = impacts
        .iter()
        .find(|impact| impact.key() == &config_key)
        .expect("transitive config impact");
    assert_eq!(config_impact.base(), Some(digest(22)));
    assert_eq!(config_impact.head(), Some(digest(22)));
    assert!(!config_impact.direct());

    assert_eq!(
        SchemaImpactKey::new("unknown-domain", "id").err(),
        Some(SchemaImpactError::UnknownKind)
    );
    assert_eq!(
        SchemaImpactKey::new("atom-object", "").err(),
        Some(SchemaImpactError::InvalidKey)
    );
    assert_eq!(
        SchemaImpactKey::new("atom-object", "e\u{301}").err(),
        Some(SchemaImpactError::InvalidKey)
    );
    assert_eq!(
        SchemaImpactDependency::new("unknown-edge", atom_key.clone()).err(),
        Some(SchemaImpactError::UnknownEdge)
    );
    let wrong_target = SchemaImpactKey::new("tool-identity", "tool").expect("tool key");
    assert_eq!(
        SchemaImpactDependency::new("repository-config-policies", wrong_target).err(),
        Some(SchemaImpactError::TargetKind)
    );
    assert_eq!(
        SchemaImpactNode::new(atom_key.clone(), digest(23), vec![dependency.clone()]).err(),
        Some(SchemaImpactError::SourceKind)
    );
    assert_eq!(
        SchemaImpactNode::new(
            config_key.clone(),
            digest(24),
            vec![dependency.clone(), dependency.clone()],
        )
        .err(),
        Some(SchemaImpactError::DuplicateDependency)
    );
    let duplicate_node =
        SchemaImpactNode::new(atom_key.clone(), digest(25), Vec::new()).expect("node");
    assert_eq!(
        SchemaImpactIndex::new(vec![duplicate_node.clone(), duplicate_node]).err(),
        Some(SchemaImpactError::DuplicateNode)
    );
    let dangling =
        SchemaImpactNode::new(config_key, digest(26), vec![dependency]).expect("dangling node");
    assert_eq!(
        SchemaImpactIndex::new(vec![dangling]).err(),
        Some(SchemaImpactError::DanglingDependency)
    );
    assert!(SchemaImpactIndex::default().is_empty());

    let error_codes = [
        (SchemaImpactError::UnknownKind, "CHG-IMPACT-KIND-UNKNOWN"),
        (SchemaImpactError::InvalidKey, "CHG-IMPACT-KEY-INVALID"),
        (SchemaImpactError::UnknownEdge, "CHG-IMPACT-EDGE-UNKNOWN"),
        (SchemaImpactError::TargetKind, "CHG-IMPACT-EDGE-TARGET"),
        (SchemaImpactError::SourceKind, "CHG-IMPACT-EDGE-SOURCE"),
        (
            SchemaImpactError::DuplicateDependency,
            "CHG-IMPACT-EDGE-DUPLICATE",
        ),
        (
            SchemaImpactError::DuplicateNode,
            "CHG-IMPACT-NODE-DUPLICATE",
        ),
        (
            SchemaImpactError::DanglingDependency,
            "CHG-IMPACT-EDGE-DANGLING",
        ),
    ];
    for (error, expected) in error_codes {
        assert_eq!(error.code(), expected);
    }
}

fn disposition(
    id: &str,
    base: Option<SemanticDigest>,
    head: Option<SemanticDigest>,
    kind: ChangeDispositionKind,
) -> ChangeDisposition {
    let reaffirmation = (kind == ChangeDispositionKind::Reaffirmed).then(|| digest(40));
    let waiver = (kind == ChangeDispositionKind::Waived)
        .then(|| ObjectRef::new(ObjectKind::Waiver, sid("COV-WAIVER"), digest(41)));
    ChangeDisposition::new(
        sid(&format!("COV-{id}")),
        ObjectKind::Atom,
        sid(&format!("COV-{id}")),
        base,
        head,
        kind,
        reaffirmation,
        waiver,
        Vec::new(),
    )
    .expect("valid disposition")
}

fn manifest(id: &str, dispositions: Vec<ChangeDisposition>) -> ChangeManifest {
    ChangeManifest::new(
        sid(&format!("COV-MANIFEST-{id}")),
        digest(50),
        digest(51),
        dispositions,
        Some(approval()),
    )
    .expect("valid manifest")
}

#[test]
fn disposition_closure_distinguishes_missing_duplicate_stale_and_extra_rows() {
    let base_graph = SpecGraph::default();
    let head_graph = SpecGraph::default();
    let artifacts = ArtifactIndex::default();
    let ratchets = Vec::new();
    let protocol_context =
        DispositionProtocolContext::new(&base_graph, &head_graph, &artifacts, &ratchets);
    assert!(std::ptr::eq(
        std::ptr::from_ref(protocol_context.base()),
        std::ptr::from_ref(&base_graph)
    ));
    assert!(std::ptr::eq(
        std::ptr::from_ref(protocol_context.head()),
        std::ptr::from_ref(&head_graph)
    ));
    assert!(protocol_context.ratchets().is_empty());

    let changed = Impact {
        kind: ObjectKind::Atom,
        id: sid(&String::from("COV-CHANGED")),
        base: Some(digest(52)),
        head: Some(digest(53)),
        direct: true,
    };
    let missing = disposition_closure(
        std::slice::from_ref(&changed),
        &manifest("MISSING", Vec::new()),
    );
    assert_eq!(missing[0].code, "CHG-DISPOSITION-MISSING");

    let valid = disposition(
        "CHANGED",
        Some(digest(52)),
        Some(digest(53)),
        ChangeDispositionKind::Revised,
    );
    assert!(
        disposition_closure(
            std::slice::from_ref(&changed),
            &manifest("VALID", vec![valid.clone()])
        )
        .is_empty()
    );
    let no_policy_diagnostics = disposition_protocol_closure(
        core::slice::from_ref(&changed),
        &manifest("NO-POLICY", vec![valid.clone()]),
        &protocol_context,
    );
    assert!(
        no_policy_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CHG-DISPOSITION-RECORD")
    );
    assert!(
        ChangeManifest::new(
            sid("COV-MANIFEST-DUPLICATE"),
            digest(50),
            digest(51),
            vec![valid.clone(), valid],
            Some(approval()),
        )
        .is_err()
    );

    let stale = disposition(
        "CHANGED",
        Some(digest(54)),
        Some(digest(55)),
        ChangeDispositionKind::Reaffirmed,
    );
    let stale_diagnostics = disposition_closure(&[changed], &manifest("STALE", vec![stale]));
    assert_eq!(
        stale_diagnostics[0].code,
        "CHG-DISPOSITION-STALE-OR-INAPPLICABLE"
    );

    let extra = disposition(
        "EXTRA",
        Some(digest(56)),
        None,
        ChangeDispositionKind::Retired,
    );
    let extra_diagnostics = disposition_closure(&[], &manifest("EXTRA", vec![extra]));
    assert_eq!(extra_diagnostics[0].code, "CHG-DISPOSITION-INAPPLICABLE");

    let waived_impact = Impact {
        kind: ObjectKind::Atom,
        id: sid(&String::from("COV-WAIVED")),
        base: Some(digest(57)),
        head: None,
        direct: true,
    };
    let waived = disposition(
        "WAIVED",
        Some(digest(57)),
        None,
        ChangeDispositionKind::Waived,
    );
    assert!(disposition_closure(&[waived_impact], &manifest("WAIVED", vec![waived])).is_empty());
}

#[test]
fn policy_materialization_emits_authority_surface_freshness_and_role_obligations() {
    let atom = Atom::new(
        sid("COV-ATOM"),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        "The invariant MUST remain true.",
        Some(approval()),
    )
    .expect("atom");
    let member = payload("surface-member/v1", &[1]);
    let surface = DecisionSurface::new(
        DecisionSurfaceDomain::new(
            sid("COV-SURFACE"),
            payload("surface-member/v1", &[]),
            payload("surface-disposition/v1", &[]),
            vec![member.clone()],
            1,
        )
        .expect("surface domain"),
        vec![
            SurfaceDisposition::new(
                member,
                SurfaceDispositionKind::Normative,
                Some(payload("surface-decision/v1", &[1])),
                Some(AtomRevisionRef::new(
                    atom.meta().id().clone(),
                    atom.revision(),
                )),
            )
            .expect("surface disposition"),
        ],
        Vec::new(),
    )
    .expect("surface");
    let stale_authority = surface_completeness(&SpecGraph::default(), &surface);
    assert!(!stale_authority.complete);
    assert_eq!(stale_authority.unauthorized.len(), 1);
    let unknown_surface = DecisionSurface::new(
        surface.domain().clone(),
        vec![
            SurfaceDisposition::new(
                payload("surface-member/v1", &[2]),
                SurfaceDispositionKind::NotApplicable,
                None,
                None,
            )
            .expect("unknown surface row"),
        ],
        Vec::new(),
    )
    .expect("surface with unknown member");
    let unknown = surface_completeness(&SpecGraph::default(), &unknown_surface);
    assert_eq!(unknown.missing.len(), 1);
    assert_eq!(unknown.unknown.len(), 1);
    assert!(unknown.duplicate.is_empty());

    let stale_surface_graph = SpecGraph {
        decision_surfaces: vec![surface.clone()],
        ..SpecGraph::default()
    };
    assert!(
        validate_graph(&stale_surface_graph)
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-REF-DANGLING-OR-STALE")
    );

    let surface_reference = ObjectRef::new(
        ObjectKind::DecisionSurface,
        surface.meta().id().clone(),
        surface.meta().digest(),
    );
    let policy = CoveragePolicy::new(
        sid(&String::from("COV-POLICY")),
        CoveragePolicyRevision::new(
            payload("policy-scope/v1", &[1]),
            vec![EvidenceRole::Positive],
            EnforcementMode::Blocking,
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            vec![
                TypedSelector::new(
                    path("src"),
                    SchemaId::new("rust/v1").expect("valid schema"),
                    vec![String::from("**/*.rs")],
                    Vec::new(),
                    AuthorityExpectation::Required,
                )
                .expect("authority selector"),
            ],
            Some(payload("derivation-requirements/v1", &[1])),
        )
        .expect("policy revision"),
        approval(),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![surface_reference],
    )
    .expect("policy");
    let graph = SpecGraph {
        atoms: vec![atom],
        decision_surfaces: vec![surface],
        coverage_policies: vec![policy.clone()],
        ..SpecGraph::default()
    };
    let registry = IdRegistry::default();
    let source_map = SourceMap::new(Vec::new()).expect("source map");
    let artifacts = ArtifactIndex::default();
    let parsed_items = Vec::new();
    let context = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        &parsed_items,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("validation context");
    assert!(
        validate_graph_with_context(&graph, &context)
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-REF-HOST-SCHEMA")
    );

    let obligations = materialize_policy_obligations(&graph, &policy).expect("obligations");
    let kinds = obligations
        .iter()
        .map(ObligationRef::kind)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        kinds,
        [
            ObligationKind::Authority,
            ObligationKind::DerivationConsistency,
            ObligationKind::Freshness,
            ObligationKind::RequiredRole,
            ObligationKind::SurfaceAuthority,
            ObligationKind::SurfaceCompleteness,
        ]
        .into_iter()
        .collect()
    );
    assert!(matches!(
        materialize_policy_obligations(&SpecGraph::default(), &policy),
        Err(PolicyAnalysisError::DanglingSubject)
    ));
    assert_eq!(
        PolicyAnalysisError::DuplicatePolicy.code(),
        "CHG-RATCHET-POLICY-DUPLICATE"
    );
    assert_eq!(
        PolicyAnalysisError::DanglingSubject.code(),
        "CHG-RATCHET-SCOPE-DANGLING"
    );
    let converted = PolicyAnalysisError::from(ConstructionError::EmptyText);
    assert_eq!(converted.code(), "CHG-RATCHET-MATERIALIZATION");
}

fn policy(id: &str, lifecycle: PolicyLifecycle) -> CoveragePolicy {
    let (replacements, terminal_approval) = match lifecycle {
        PolicyLifecycle::Active => (Vec::new(), None),
        PolicyLifecycle::Superseded => (
            vec![ObjectRef::new(
                ObjectKind::CoveragePolicy,
                sid("COV-POLICY-NEW"),
                digest(70),
            )],
            Some(approval()),
        ),
        PolicyLifecycle::Retired => (Vec::new(), Some(approval())),
    };
    CoveragePolicy::new(
        sid(id),
        CoveragePolicyRevision::new(
            payload("policy-scope/v1", &[1]),
            vec![EvidenceRole::Positive],
            EnforcementMode::Blocking,
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("policy revision"),
        approval(),
        lifecycle,
        replacements,
        terminal_approval,
        Vec::new(),
    )
    .expect("policy")
}

fn subject_policy(id: &str, roles: Vec<EvidenceRole>, subject: ObjectRef) -> CoveragePolicy {
    CoveragePolicy::new(
        sid(id),
        CoveragePolicyRevision::new(
            payload("policy-scope/v1", &[1]),
            roles,
            EnforcementMode::Blocking,
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("policy revision"),
        approval(),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![subject],
    )
    .expect("subject policy")
}

fn obligation() -> ObligationRef {
    ObligationRef::new(
        sid(&String::from("COV-POLICY-ACTIVE")),
        ObligationKind::Freshness,
        ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(71)),
        None,
    )
    .expect("obligation")
}

#[test]
fn graph_history_retains_active_and_terminal_policy_debt_and_waiver_records() {
    let active_policy = policy("COV-POLICY-ACTIVE", PolicyLifecycle::Active);
    let terminal_policy = policy("COV-POLICY-RETIRED", PolicyLifecycle::Retired);
    let evidence = ObjectRef::new(ObjectKind::ProofRecord, sid("COV-PROOF"), digest(72));
    let active_debt = DebtRecord::new(
        sid("COV-DEBT-ACTIVE"),
        vec![obligation()],
        Vec::new(),
        "Pending evidence.",
        ScopeId::new("maintainers").expect("valid scope"),
        "Provide evidence.",
        payload("debt-discharge/v1", &[1]),
        Vec::new(),
        DebtLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("active debt");
    let terminal_debt = DebtRecord::new(
        sid("COV-DEBT-DISCHARGED"),
        vec![obligation()],
        Vec::new(),
        "Evidence supplied.",
        ScopeId::new("maintainers").expect("valid scope"),
        "Complete.",
        payload("debt-discharge/v1", &[1]),
        Vec::new(),
        DebtLifecycle::Discharged,
        vec![evidence.clone()],
        Vec::new(),
        Some(approval()),
    )
    .expect("discharged debt");
    let active_waiver = Waiver::new(
        sid("COV-WAIVER-ACTIVE"),
        vec![obligation()],
        Vec::new(),
        "Temporary waiver.",
        ScopeId::new("maintainers").expect("valid scope"),
        payload("waiver-discharge/v1", &[1]),
        approval(),
        Vec::new(),
        WaiverLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("active waiver");
    let terminal_waiver = Waiver::new(
        sid("COV-WAIVER-REVOKED"),
        vec![obligation()],
        Vec::new(),
        "Waiver revoked.",
        ScopeId::new("maintainers").expect("valid scope"),
        payload("waiver-discharge/v1", &[1]),
        approval(),
        Vec::new(),
        WaiverLifecycle::Revoked,
        Vec::new(),
        Vec::new(),
        Some(approval()),
    )
    .expect("revoked waiver");
    let base = SpecGraph {
        coverage_policies: vec![active_policy, terminal_policy],
        debt_records: vec![active_debt, terminal_debt],
        waivers: vec![active_waiver, terminal_waiver],
        ..SpecGraph::default()
    };
    let retained = base.clone();
    assert!(
        validate_graph_history(
            &base,
            &retained,
            &IdRegistry::default(),
            &IdRegistry::default(),
        )
        .is_empty()
    );

    let allocations = base
        .object_meta()
        .into_iter()
        .enumerate()
        .map(|(index, meta)| {
            let class = if meta.id().as_str().ends_with("ACTIVE") {
                IdAllocationClass::Live
            } else {
                IdAllocationClass::Terminal
            };
            IdAllocation::new(
                ScopeId::new("coverage").expect("valid scope"),
                meta.id().clone(),
                u64::try_from(index + 1).expect("small fixture ordinal"),
                class,
                digest(u8::try_from(index + 80).expect("small fixture byte")),
            )
            .expect("allocation")
        })
        .collect();
    let registry = IdRegistry::new(
        vec![
            IdNamespace::new(
                ScopeId::new("coverage").expect("valid scope"),
                NamespacePrefix::new("COV").expect("valid prefix"),
                payload("allocation-policy/v1", &[1]),
                vec![ScopeId::new("maintainers").expect("valid scope")],
            )
            .expect("namespace"),
        ],
        allocations,
    )
    .expect("registry");
    let source_map = SourceMap::new(Vec::new()).expect("source map");
    let artifacts = ArtifactIndex::default();
    let context = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        &[],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("validation context");
    let context_diagnostics = validate_graph_with_context(&base, &context);
    assert!(
        !context_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SCF-REGISTRY-CLASS")
    );
    assert!(
        context_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SCF-APPROVAL-ACTION")
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
        6
    );

    let duplicate_head_policy = policy("COV-POLICY-DUPLICATE", PolicyLifecycle::Active);
    assert_eq!(
        derive_policy_ratchets(
            &SpecGraph::default(),
            &SpecGraph {
                coverage_policies: vec![duplicate_head_policy.clone(), duplicate_head_policy,],
                ..SpecGraph::default()
            },
        ),
        Err(PolicyAnalysisError::DuplicatePolicy)
    );

    let dangling_subject = ObjectRef::new(ObjectKind::Atom, sid("COV-MISSING-SUBJECT"), digest(71));
    let dangling_head_policy = subject_policy(
        "COV-POLICY-DANGLING",
        vec![EvidenceRole::Positive],
        dangling_subject.clone(),
    );
    assert_eq!(
        derive_policy_ratchets(
            &SpecGraph::default(),
            &SpecGraph {
                coverage_policies: vec![dangling_head_policy],
                ..SpecGraph::default()
            },
        ),
        Err(PolicyAnalysisError::DanglingSubject)
    );

    let dangling_replacement = subject_policy(
        "COV-POLICY-ZZZ",
        vec![EvidenceRole::Positive],
        dangling_subject,
    );
    let superseded_with_dangling_replacement = CoveragePolicy::new(
        sid("COV-POLICY-OLD"),
        CoveragePolicyRevision::new(
            payload("policy-scope/v1", &[1]),
            vec![EvidenceRole::Positive],
            EnforcementMode::Blocking,
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("policy revision"),
        approval(),
        PolicyLifecycle::Superseded,
        vec![object_ref(dangling_replacement.meta())],
        Some(approval()),
        Vec::new(),
    )
    .expect("superseded policy");
    assert_eq!(
        derive_policy_ratchets(
            &SpecGraph {
                coverage_policies: vec![policy("COV-POLICY-OLD", PolicyLifecycle::Active)],
                ..SpecGraph::default()
            },
            &SpecGraph {
                coverage_policies: vec![superseded_with_dangling_replacement, dangling_replacement,],
                ..SpecGraph::default()
            },
        ),
        Err(PolicyAnalysisError::DanglingSubject)
    );

    let base_policy = policy("COV-POLICY-OLD", PolicyLifecycle::Active);
    let head_policy = policy("COV-POLICY-OLD", PolicyLifecycle::Superseded);
    let ratchets = derive_policy_ratchets(
        &SpecGraph {
            coverage_policies: vec![base_policy],
            ..SpecGraph::default()
        },
        &SpecGraph {
            coverage_policies: vec![head_policy],
            ..SpecGraph::default()
        },
    )
    .expect("dangling replacement is a conservative weakening, not an analysis failure");
    assert_eq!(
        ratchets[0].classification(),
        PolicyChangeClassification::Weakening
    );
    assert!(
        ratchets[0]
            .diagnostics()
            .contains(&String::from("CHG-RATCHET-SUPERSESSION"))
    );
}

#[test]
fn disposition_protocol_covers_subject_authority_missing_records_and_ratchets() {
    let base_atom = Atom::new(
        sid("COV-SUBJECT"),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("change.protocol").expect("valid scope")],
        "The base behavior MUST remain governed.",
        Some(approval()),
    )
    .expect("base atom");
    let head_atom = Atom::new(
        sid("COV-SUBJECT"),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("change.protocol").expect("valid scope")],
        "The head behavior MUST remain governed and current.",
        Some(approval()),
    )
    .expect("head atom");
    let base = SpecGraph {
        atoms: vec![base_atom.clone()],
        coverage_policies: vec![subject_policy(
            "COV-POLICY",
            vec![EvidenceRole::Positive, EvidenceRole::Negative],
            object_ref(base_atom.meta()),
        )],
        ..SpecGraph::default()
    };
    let head = SpecGraph {
        atoms: vec![head_atom.clone()],
        coverage_policies: vec![subject_policy(
            "COV-POLICY",
            vec![EvidenceRole::Positive],
            object_ref(head_atom.meta()),
        )],
        ..SpecGraph::default()
    };
    let ratchets = derive_policy_ratchets(&base, &head).expect("subject ratchet");
    assert_eq!(
        ratchets[0].classification(),
        PolicyChangeClassification::Weakening
    );
    assert!(!ratchets[0].removed_obligations().is_empty());
    let impact = Impact {
        kind: ObjectKind::Atom,
        id: sid(&String::from("COV-SUBJECT")),
        base: Some(base_atom.meta().digest()),
        head: Some(head_atom.meta().digest()),
        direct: true,
    };
    let revised = ChangeDisposition::new(
        sid("COV-DISPOSITION-SUBJECT"),
        ObjectKind::Atom,
        sid("COV-SUBJECT"),
        impact.base,
        impact.head,
        ChangeDispositionKind::Revised,
        None,
        None,
        Vec::new(),
    )
    .expect("subject disposition");
    let approved_manifest = ChangeManifest::new(
        sid("COV-MANIFEST-SUBJECT"),
        digest(80),
        digest(81),
        vec![revised.clone()],
        Some(approval()),
    )
    .expect("subject manifest");
    let artifacts = ArtifactIndex::default();
    let context = DispositionProtocolContext::new(&base, &head, &artifacts, &ratchets);
    let diagnostics =
        disposition_protocol_closure(core::slice::from_ref(&impact), &approved_manifest, &context);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CHG-DISPOSITION-DEBT")
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CHG-DISPOSITION-WAIVER")
    );

    let advisory_policy = CoveragePolicy::new(
        sid("COV-POLICY"),
        CoveragePolicyRevision::new(
            payload("policy-scope/v1", &[1]),
            vec![EvidenceRole::Positive, EvidenceRole::Negative],
            EnforcementMode::Advisory,
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("advisory revision"),
        approval(),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![object_ref(base_atom.meta())],
    )
    .expect("advisory base policy");
    let advisory_base = SpecGraph {
        atoms: vec![base_atom.clone()],
        coverage_policies: vec![advisory_policy],
        ..SpecGraph::default()
    };
    let advisory_context =
        DispositionProtocolContext::new(&advisory_base, &head, &artifacts, &ratchets);
    let advisory_diagnostics = disposition_protocol_closure(
        core::slice::from_ref(&impact),
        &approved_manifest,
        &advisory_context,
    );
    assert!(
        advisory_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "CHG-DISPOSITION-WAIVER")
    );

    let fallback_context = DispositionProtocolContext::new(&base, &head, &artifacts, &[]);
    assert!(
        disposition_protocol_closure(
            core::slice::from_ref(&impact),
            &approved_manifest,
            &fallback_context,
        )
        .iter()
        .any(|diagnostic| diagnostic.code == "CHG-DISPOSITION-RECORD")
    );

    let missing_replacement =
        ObjectRef::new(ObjectKind::Atom, sid("COV-MISSING-REPLACEMENT"), digest(82));
    let revised_with_missing_replacement = ChangeDisposition::new(
        sid("COV-DISPOSITION-REPLACEMENT"),
        ObjectKind::Atom,
        sid("COV-SUBJECT"),
        impact.base,
        impact.head,
        ChangeDispositionKind::Revised,
        None,
        None,
        vec![missing_replacement],
    )
    .expect("disposition with explicit replacement");
    let replacement_manifest = ChangeManifest::new(
        sid("COV-MANIFEST-REPLACEMENT"),
        digest(83),
        digest(84),
        vec![revised_with_missing_replacement],
        Some(approval()),
    )
    .expect("replacement manifest");
    assert!(
        disposition_protocol_closure(
            core::slice::from_ref(&impact),
            &replacement_manifest,
            &fallback_context,
        )
        .iter()
        .any(|diagnostic| diagnostic.code == "CHG-DISPOSITION-REPLACEMENT")
    );

    let no_approval_manifest = ChangeManifest::new(
        sid("COV-MANIFEST-NO-APPROVAL"),
        digest(85),
        digest(86),
        vec![revised],
        None,
    )
    .expect("manifest without approval");
    assert!(
        disposition_protocol_closure(
            core::slice::from_ref(&impact),
            &no_approval_manifest,
            &context,
        )
        .iter()
        .any(|diagnostic| diagnostic.code == "CHG-DISPOSITION-APPROVAL")
    );

    let reaffirmed = ChangeDisposition::new(
        sid("COV-DISPOSITION-REAFFIRMED"),
        ObjectKind::Atom,
        sid("COV-SUBJECT"),
        impact.base,
        impact.head,
        ChangeDispositionKind::Reaffirmed,
        Some(digest(87)),
        None,
        Vec::new(),
    )
    .expect("reaffirmed disposition");
    let reaffirmed_manifest = ChangeManifest::new(
        sid("COV-MANIFEST-REAFFIRMED"),
        digest(88),
        digest(89),
        vec![reaffirmed],
        None,
    )
    .expect("reaffirmed manifest");
    assert!(
        disposition_protocol_closure(
            core::slice::from_ref(&impact),
            &reaffirmed_manifest,
            &context,
        )
        .iter()
        .any(|diagnostic| diagnostic.code == "CHG-DISPOSITION-APPLICABILITY")
    );

    let applicable_obligation = ObligationRef::new(
        sid("COV-POLICY"),
        ObligationKind::Freshness,
        object_ref(head_atom.meta()),
        None,
    )
    .expect("applicable waiver obligation");
    let applicable_waiver = Waiver::new(
        sid("COV-WAIVER-APPLICABLE"),
        vec![applicable_obligation],
        vec![object_ref(head_atom.meta())],
        "Temporary process waiver.",
        ScopeId::new("maintainers").expect("valid scope"),
        payload("waiver-discharge/v1", &[1]),
        approval(),
        Vec::new(),
        WaiverLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("applicable active waiver");
    let applicable_waiver_ref = object_ref(applicable_waiver.meta());
    let head_with_waiver = SpecGraph {
        waivers: vec![applicable_waiver],
        ..head.clone()
    };
    let current_artifacts = ArtifactIndex::new(vec![ArtifactEntry::saved_bytes(
        path("reviews/approval.md"),
        RawArtifactMediaType::Markdown,
        digest(90),
    )])
    .expect("current approval artifact");
    let applicable_disposition = ChangeDisposition::new(
        sid("COV-DISPOSITION-WAIVER-APPLICABLE"),
        ObjectKind::Atom,
        sid("COV-SUBJECT"),
        impact.base,
        impact.head,
        ChangeDispositionKind::Waived,
        None,
        Some(applicable_waiver_ref),
        Vec::new(),
    )
    .expect("applicable waived disposition");
    let applicable_manifest = ChangeManifest::new(
        sid("COV-MANIFEST-WAIVER-APPLICABLE"),
        digest(88),
        digest(89),
        vec![applicable_disposition],
        Some(approval()),
    )
    .expect("applicable waiver manifest");
    let applicable_context =
        DispositionProtocolContext::new(&base, &head_with_waiver, &current_artifacts, &[]);
    let applicable_diagnostics = disposition_protocol_closure(
        core::slice::from_ref(&impact),
        &applicable_manifest,
        &applicable_context,
    );
    assert!(
        applicable_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "CHG-DISPOSITION-WAIVER")
    );

    let empty_manifest = ChangeManifest::new(
        sid("COV-MANIFEST-EMPTY"),
        digest(90),
        digest(91),
        Vec::new(),
        None,
    )
    .expect("empty manifest");
    assert!(
        disposition_protocol_closure(&[], &empty_manifest, &context)
            .iter()
            .any(|diagnostic| diagnostic.code == "CHG-DISPOSITION-MISSING")
    );
}
