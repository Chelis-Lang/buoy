#![doc = "Behavioral coverage controls for core algorithms and model values."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy_core::*;
use std::collections::BTreeSet;

#[path = "support/model_constructor_error_controls.rs"]
mod model_constructor_error_controls;

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn sid(value: &str) -> StableId {
    StableId::new(value).expect("valid stable ID")
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

fn object_ref(meta: &ObjectMeta) -> ObjectRef {
    ObjectRef::new(meta.kind(), meta.id().clone(), meta.digest())
}

fn atom_ref(atom: &Atom) -> AtomRevisionRef {
    AtomRevisionRef::new(atom.meta().id().clone(), atom.revision())
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

fn make_atom(id: &str) -> Atom {
    Atom::new(
        sid(id),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        "The invariant MUST remain true.",
        Some(approval()),
    )
    .expect("valid atom")
}

fn make_oracle(id: &str, kind: OracleKind) -> Oracle {
    Oracle::new(
        sid(id),
        kind,
        tool("runner", 10),
        payload("oracle-selection/v1", &[1]),
        payload("oracle-normalization/v1", &[2]),
        digest(11),
        payload("oracle-scope/v1", &[3]),
        vec![String::from("network")],
    )
    .expect("valid oracle")
}

fn proof_fixture(
    id: &str,
    atoms: Vec<AtomRevisionRef>,
    property: ObjectRef,
    implementation: ArtifactBinding,
    assumptions: Vec<ObjectRef>,
    proof_sources: Vec<ArtifactBinding>,
) -> Result<ProofRecord, ConstructionError> {
    ProofRecord::new(
        sid(id),
        atoms,
        property,
        implementation,
        assumptions,
        Vec::new(),
        tool("verifier", 70),
        digest(72),
        Vec::new(),
        Vec::new(),
        proof_sources,
        Verdict::Pass,
        payload("proof-scope/v1", &[1]),
        Vec::new(),
    )
}

#[test]
fn model_accessors_expose_exact_constructed_state_and_error_codes() {
    let permission = ApprovalPermission::new(
        String::from("maintainers"),
        ObjectKind::Atom,
        ApprovalAction::Ratify,
    )
    .expect("valid permission");
    assert_eq!(permission.authority(), "maintainers");
    assert_eq!(permission.kind(), ObjectKind::Atom);
    assert_eq!(permission.action(), ApprovalAction::Ratify);
    for invalid_authority in ["e\u{301}", "bad..id", "Bad", "bad_id"] {
        assert_eq!(
            ApprovalPermission::new(invalid_authority, ObjectKind::Atom, ApprovalAction::Ratify,)
                .err(),
            Some(GraphValidationContextError::ApprovalAuthorityId)
        );
    }

    let selector = TypedSelector::new(
        path("src"),
        SchemaId::new("rust/v1").expect("valid schema"),
        vec![String::from("**/*.rs")],
        vec![String::from("target/**")],
        AuthorityExpectation::Required,
    )
    .expect("valid selector");
    assert_eq!(selector.root().as_str(), "src");
    assert_eq!(selector.adapter_id(), "rust/v1");
    assert_eq!(selector.include_patterns(), ["**/*.rs"]);
    assert_eq!(selector.exclude_patterns(), ["target/**"]);
    assert_eq!(
        selector.authority_expectation(),
        AuthorityExpectation::Required
    );

    let rejected = RejectedEntryObservation::new(
        path("vendor/link"),
        EntryKind::Symlink,
        String::from("SCF-SNAPSHOT-SYMLINK"),
    )
    .expect("valid rejected observation");
    assert_eq!(rejected.path().as_str(), "vendor/link");
    assert_eq!(rejected.kind(), EntryKind::Symlink);
    assert_eq!(rejected.diagnostic_code(), "SCF-SNAPSHOT-SYMLINK");
    assert_ne!(rejected.canonical_value(), CanonicalValue::None);

    let environment = EnvironmentBinding::new(String::from("LC_ALL"), String::from("C"))
        .expect("valid environment binding");
    assert_eq!(environment.name(), "LC_ALL");
    assert_eq!(environment.value(), "C");
    assert_ne!(environment.canonical_value(), CanonicalValue::None);

    let schema_lock = SchemaToolLock::new(
        SchemaId::new("canonical-encoding/v1").expect("valid schema"),
        tool("unicode", 20),
        tool("commonmark", 22),
        tool("toml", 24),
        Some(tool("rust", 26)),
        vec![tool("adapter", 28)],
    )
    .expect("valid schema lock");
    assert_eq!(schema_lock.encoding_schema_id(), "canonical-encoding/v1");
    assert_eq!(schema_lock.unicode_data().id(), "unicode");
    assert_eq!(schema_lock.commonmark_parser().id(), "commonmark");
    assert_eq!(schema_lock.toml_parser().id(), "toml");
    assert_eq!(
        schema_lock.rust_parser().map(ToolIdentity::id),
        Some("rust")
    );
    assert_eq!(schema_lock.additional_tools()[0].id(), "adapter");
    assert_ne!(schema_lock.digest(), digest(0));

    let object_binding = ArtifactBinding::object(ObjectRef::new(
        ObjectKind::Atom,
        sid("COV-ATOM"),
        digest(30),
    ));
    let repository_binding = ArtifactBinding::repository(
        path("artifacts/proof.bin"),
        String::from("raw-artifact"),
        String::from("application/octet-stream"),
        String::from("raw/v1"),
        digest(31),
    )
    .expect("repository binding");
    assert_eq!(object_binding.kind(), ArtifactBindingKind::Object);
    assert!(object_binding.object_reference().is_some());
    assert!(object_binding.repository_artifact().is_none());
    assert_eq!(object_binding.artifact_digest(), digest(30));
    assert!(object_binding.binding_digest().is_ok());
    assert_eq!(
        repository_binding.kind(),
        ArtifactBindingKind::RepositoryArtifact
    );
    assert!(repository_binding.object_reference().is_none());
    assert_eq!(
        repository_binding
            .repository_artifact()
            .map(RepositoryArtifactBinding::path),
        Some(&path("artifacts/proof.bin"))
    );
    assert_eq!(repository_binding.artifact_digest(), digest(31));
    assert!(repository_binding.binding_digest().is_ok());

    assert_eq!(
        ArtifactIndexError::DuplicatePath.code(),
        "SCF-ARTIFACT-DUPLICATE-PATH"
    );
    assert_eq!(
        ArtifactIndexError::PortableCollision.code(),
        "SCF-ARTIFACT-PORTABLE-COLLISION"
    );
    assert_eq!(
        GraphValidationContextError::DuplicateApprovalPermission.code(),
        "SCF-APPROVAL-ACTION-DUPLICATE"
    );
    assert_eq!(
        GraphValidationContextError::ApprovalAuthorityId.code(),
        "SCF-APPROVAL-AUTHORITY"
    );
    assert_eq!(
        GraphValidationContextError::DuplicateSourcePath.code(),
        "SCF-SOURCE-MAP-DUPLICATE"
    );
    assert_eq!(
        GraphValidationContextError::DuplicateHostSchema.code(),
        "SPG-REF-HOST-SCHEMA-DUPLICATE"
    );
    assert_eq!(
        GraphValidationContextError::HostSchemaId.code(),
        "SPG-REF-HOST-SCHEMA-ID"
    );

    let construction_codes = [
        (
            ConstructionError::Encoding(EncodingError::DuplicateSetValue),
            "DET-DUP-SET-VALUE",
        ),
        (ConstructionError::EmptySet, "SCF-CONSTRUCT-FIELD-EMPTY"),
        (ConstructionError::EmptyText, "SCF-CONSTRUCT-FIELD-EMPTY"),
        (
            ConstructionError::NonPositiveGeneration,
            "SCF-CONSTRUCT-GENERATION",
        ),
        (ConstructionError::SourceSpan, "SCF-SOURCE-MAP-SPAN"),
        (
            ConstructionError::ObligationRoleShape,
            "SCF-CONSTRUCT-OBLIGATION-ROLE",
        ),
        (
            ConstructionError::ArtifactBindingShape,
            "SCF-CONSTRUCT-ARTIFACT-BINDING",
        ),
        (
            ConstructionError::OverlappingSets,
            "SCF-CONSTRUCT-SET-OVERLAP",
        ),
        (
            ConstructionError::VerdictResultShape,
            "SCF-CONSTRUCT-VERDICT-RESULT",
        ),
        (
            ConstructionError::StableIdGrammar,
            "SCF-CONSTRUCT-STABLE-ID",
        ),
        (ConstructionError::ScopeIdGrammar, "SCF-CONSTRUCT-SCOPE-ID"),
        (
            ConstructionError::SchemaIdGrammar,
            "SCF-CONSTRUCT-SCHEMA-ID",
        ),
        (
            ConstructionError::NormativeStatement,
            "SCF-CONSTRUCT-NORMATIVE-STATEMENT",
        ),
        (
            ConstructionError::RoleScopeShape,
            "SCF-CONSTRUCT-ROLE-SCOPE",
        ),
        (
            ConstructionError::LocalInvariant,
            "SCF-CONSTRUCT-LOCAL-INVARIANT",
        ),
        (ConstructionError::LifecycleShape, "SCF-LIFECYCLE-SHAPE"),
        (
            ConstructionError::ReferenceKind,
            "SCF-CONSTRUCT-REFERENCE-KIND",
        ),
        (
            ConstructionError::EqualDigests,
            "SCF-CONSTRUCT-DIGEST-EQUALITY",
        ),
        (
            ConstructionError::ChangeDispositionShape,
            "SCF-CONSTRUCT-CHANGE-DISPOSITION",
        ),
    ];
    for (error, expected) in construction_codes {
        assert_eq!(error.code(), expected);
    }
}

#[test]
fn complete_graph_context_exercises_every_object_validation_path() {
    let atom = make_atom("COV-ATOM");
    let atom_reference = atom_ref(&atom);
    let atom_object = object_ref(atom.meta());
    let proposed_atom = Atom::new(
        sid("COV-PROPOSED"),
        LifecycleState::Proposed,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        "The proposed invariant MUST remain explicit.",
        None,
    )
    .expect("valid proposed atom");
    let tombstone = Tombstone::new(
        sid(&String::from("COV-TOMBSTONE")),
        LifecycleState::Superseded,
        digest(32),
        vec![atom_reference.clone()],
        approval(),
    )
    .expect("valid tombstone");

    let policy_revision = CoveragePolicyRevision::new(
        payload("policy-scope/v1", &[1]),
        vec![EvidenceRole::Positive, EvidenceRole::Negative],
        EnforcementMode::Blocking,
        ScopeId::new("coverage-ratchet").expect("valid scope"),
        Vec::new(),
        Some(payload("derivation-requirements/v1", &[1])),
    )
    .expect("valid policy revision");
    let policy = CoveragePolicy::new(
        sid(&String::from("COV-POLICY")),
        policy_revision,
        approval(),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![atom_object.clone()],
    )
    .expect("valid policy");
    let policy_ref = object_ref(policy.meta());

    let member = payload("surface-member/v1", &[1]);
    let surface_domain = DecisionSurfaceDomain::new(
        sid(&String::from("COV-SURFACE")),
        payload("surface-member/v1", &[]),
        payload("surface-disposition/v1", &[]),
        vec![member.clone()],
        1,
    )
    .expect("valid surface domain");
    let disposition = SurfaceDisposition::new(
        member,
        SurfaceDispositionKind::Normative,
        Some(payload("surface-decision/v1", &[1])),
        Some(atom_reference.clone()),
    )
    .expect("valid disposition");
    let not_applicable = SurfaceDisposition::new(
        payload("surface-member/v1", &[2]),
        SurfaceDispositionKind::NotApplicable,
        None,
        None,
    )
    .expect("valid not-applicable row");
    let surface = DecisionSurface::new(
        surface_domain,
        vec![disposition, not_applicable],
        vec![policy_ref],
    )
    .expect("valid surface");

    let parsed = ParsedItemIdentity::new(
        SchemaId::new("rust/v1").expect("valid schema"),
        path("src/lib.rs"),
        SourceItemKind::new("rust/item/v1").expect("valid source item kind"),
        b"fn item() {}".to_vec(),
        None,
    )
    .expect("valid parsed item");
    let implementation = ImplementationLink::new(
        sid(&String::from("COV-IMPLEMENTATION")),
        vec![atom_reference.clone()],
        parsed.digest(),
        NavigationKind::Implements,
    )
    .expect("valid implementation");
    let implementation_ref = object_ref(implementation.meta());

    let subject_node = SourceNode::new(parsed.clone(), Vec::new()).expect("valid subject node");
    let requirement_node = SourceNode::new(
        ParsedItemIdentity::new(
            SchemaId::new("openspec/v1").expect("valid schema"),
            path("openspec/specs/coverage/spec.md"),
            SourceItemKind::new("openspec/requirement/v1").expect("valid source item kind"),
            b"COV-ATOM".to_vec(),
            None,
        )
        .expect("valid parsed requirement"),
        Vec::new(),
    )
    .expect("valid requirement node");
    let binding = ImplementationBinding::new(
        sid(&String::from("COV-BINDING")),
        NavigationKind::Implements,
        subject_node.reference(),
        requirement_node.reference(),
        atom_reference.clone(),
    )
    .expect("valid implementation binding");

    let oracle = make_oracle("COV-ORACLE", OracleKind::Test);
    let oracle_ref = object_ref(oracle.meta());
    let assumption = Assumption::new(
        sid(&String::from("COV-ASSUMPTION")),
        String::from("The environment is deterministic."),
        payload("assumption-scope/v1", &[1]),
        vec![String::from("clock")],
        approval(),
    )
    .expect("valid assumption");
    let assumption_ref = object_ref(assumption.meta());
    let carrier = EvidenceCarrier::new(
        sid(&String::from("COV-CARRIER")),
        EvidenceRole::Positive,
        vec![atom_reference.clone()],
        parsed.digest(),
        oracle_ref.clone(),
        digest(40),
        vec![digest(41)],
        payload("carrier-scope/v1", &[1]),
        vec![assumption_ref.clone()],
        vec![String::from("network")],
    )
    .expect("valid carrier");

    let finite_model = FiniteModel::new(
        sid(&String::from("COV-MODEL")),
        payload("model-state-space/v1", &[1]),
        payload("model-events/v1", &[2]),
        payload("model-transition/v1", &[3]),
        vec![payload("model-state/v1", &[0])],
        payload("model-bound/v1", &[4]),
        vec![atom_reference.clone()],
        vec![assumption_ref.clone()],
        vec![String::from("infinite-domain")],
    )
    .expect("valid finite model");
    let property = FormalProperty::new(
        sid(&String::from("COV-PROPERTY")),
        vec![atom_reference.clone()],
        payload("formal-property/v1", &[1]),
        payload("implementation-abstraction/v1", &[2]),
        vec![assumption_ref.clone()],
        vec![String::from("allocator")],
        approval(),
    )
    .expect("valid property");
    let property_ref = object_ref(property.meta());

    let proof_source = ArtifactBinding::repository(
        path("proofs/proof.smt2"),
        "raw-artifact",
        "text/plain",
        "smt2/v1",
        digest(42),
    )
    .expect("valid proof source");
    let proof = ProofRecord::new(
        sid(&String::from("COV-PROOF")),
        vec![atom_reference.clone()],
        property_ref,
        ArtifactBinding::object(implementation_ref.clone()),
        vec![assumption_ref.clone()],
        vec![tool("translator", 43)],
        tool("verifier", 45),
        digest(47),
        vec![proof_source.clone()],
        vec![proof_source.clone()],
        vec![proof_source],
        Verdict::Pass,
        payload("proof-scope/v1", &[1]),
        vec![String::from("trusted-kernel")],
    )
    .expect("valid proof");

    let derivation = Derivation::new(
        sid(&String::from("COV-DERIVATION")),
        vec![ArtifactBinding::object(atom_object.clone())],
        vec![ArtifactBinding::object(implementation_ref)],
        tool("generator", 48),
        digest(50),
        oracle_ref,
        DerivationRelation::GeneratedFrom,
        payload("derivation-scope/v1", &[1]),
        vec![String::from("comments")],
    )
    .expect("valid derivation");

    let obligation = ObligationRef::new(
        sid(&String::from("COV-POLICY")),
        ObligationKind::Authority,
        atom_object.clone(),
        None,
    )
    .expect("valid obligation");
    let debt = DebtRecord::new(
        sid(&String::from("COV-DEBT")),
        vec![obligation.clone()],
        vec![atom_object.clone()],
        String::from("Evidence is pending."),
        ScopeId::new("maintainers").expect("valid scope"),
        String::from("Add the missing evidence."),
        payload("debt-discharge/v1", &[1]),
        vec![String::from("issue-1")],
        DebtLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("valid debt");
    let waiver = Waiver::new(
        sid(&String::from("COV-WAIVER")),
        vec![obligation],
        vec![atom_object.clone()],
        String::from("Temporary continuation."),
        ScopeId::new("maintainers").expect("valid scope"),
        payload("waiver-discharge/v1", &[1]),
        approval(),
        vec![String::from("issue-2")],
        WaiverLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("valid waiver");
    let reaffirmation = Reaffirmation::new(
        sid("COV-REAFFIRMATION"),
        ObjectKind::Atom,
        sid("COV-ATOM"),
        digest(51),
        atom.meta().digest(),
        approval(),
        String::from("The atom was reread under the new binding."),
    )
    .expect("valid reaffirmation");

    let graph = SpecGraph {
        atoms: vec![atom, proposed_atom],
        tombstones: vec![tombstone],
        decision_surfaces: vec![surface],
        evidence_carriers: vec![carrier],
        implementation_links: vec![implementation],
        implementation_bindings: vec![binding],
        derivations: vec![derivation],
        coverage_policies: vec![policy],
        debt_records: vec![debt],
        waivers: vec![waiver],
        oracles: vec![oracle],
        finite_models: vec![finite_model],
        formal_properties: vec![property],
        assumptions: vec![assumption],
        proof_records: vec![proof],
        reaffirmations: vec![reaffirmation],
        external_trace_links: Vec::new(),
    };

    let source_entry = SourceMapEntry::new(
        object_ref(graph.implementation_links[0].meta()),
        vec![SourceRef::new(path("src/lib.rs"), 0, 12).expect("valid source")],
        SourceRole::Implementation,
    )
    .expect("valid source entry");
    let source_map = SourceMap::new(vec![source_entry]).expect("valid source map");
    let allocations = graph
        .object_meta()
        .into_iter()
        .enumerate()
        .map(|(index, meta)| {
            IdAllocation::new(
                ScopeId::new("coverage").expect("valid scope"),
                meta.id().clone(),
                u64::try_from(index + 1).expect("small fixture ordinal"),
                if meta.kind() == ObjectKind::Tombstone {
                    IdAllocationClass::Terminal
                } else {
                    IdAllocationClass::Live
                },
                digest(u8::try_from(index + 100).expect("small fixture byte")),
            )
            .expect("object allocation")
        })
        .collect();
    let registry = IdRegistry::new(
        vec![
            IdNamespace::new(
                ScopeId::new("coverage").expect("valid scope"),
                NamespacePrefix::new("COV").expect("valid prefix"),
                payload("allocation-policy/v1", &[1]),
                Vec::new(),
            )
            .expect("coverage namespace"),
        ],
        allocations,
    )
    .expect("complete object registry");
    let artifacts = ArtifactIndex::default();
    let parsed_items = vec![parsed];
    let context = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        &parsed_items,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("valid empty-authority context");
    let diagnostics = validate_graph_with_context(&graph, &context);
    let codes = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<BTreeSet<_>>();
    assert!(!codes.contains("SCF-REGISTRY-UNALLOCATED"));
    assert!(!codes.contains("SCF-REGISTRY-CLASS"));
    assert!(codes.contains("SCF-APPROVAL-AUTHORITY"));
    assert!(codes.contains("SCF-APPROVAL-ACTION"));
    assert!(codes.contains("SCF-APPROVAL-STALE"));
    assert!(codes.contains("SCF-SOURCE-MAP-SPAN"));
    assert!(codes.contains("SPG-REF-HOST-SCHEMA"));

    let no_parsed_items = Vec::new();
    let stale_item_context = GraphValidationContext::new(
        &registry,
        &source_map,
        &artifacts,
        &no_parsed_items,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("context without parsed items");
    let stale_item_codes = validate_graph_with_context(&graph, &stale_item_context)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<BTreeSet<_>>();
    assert!(stale_item_codes.contains("SCF-SOURCE-MAP-ITEM-STALE"));

    assert_eq!(graph.object_meta().len(), 17);
    assert!(semantic_graph_digest(&graph).is_ok());
    macro_rules! assert_duplicate_graph_domain_rejected {
        ($field:ident) => {{
            let mut duplicate_graph = graph.clone();
            duplicate_graph
                .$field
                .push(duplicate_graph.$field[0].clone());
            assert_eq!(
                semantic_graph_digest(&duplicate_graph),
                Err(EncodingError::DuplicateSetValue)
            );
        }};
    }
    assert_duplicate_graph_domain_rejected!(atoms);
    assert_duplicate_graph_domain_rejected!(tombstones);
    assert_duplicate_graph_domain_rejected!(decision_surfaces);
    assert_duplicate_graph_domain_rejected!(evidence_carriers);
    assert_duplicate_graph_domain_rejected!(implementation_links);
    assert_duplicate_graph_domain_rejected!(derivations);
    assert_duplicate_graph_domain_rejected!(coverage_policies);
    assert_duplicate_graph_domain_rejected!(debt_records);
    assert_duplicate_graph_domain_rejected!(waivers);
    assert_duplicate_graph_domain_rejected!(oracles);
    assert_duplicate_graph_domain_rejected!(finite_models);
    assert_duplicate_graph_domain_rejected!(formal_properties);
    assert_duplicate_graph_domain_rejected!(assumptions);
    assert_duplicate_graph_domain_rejected!(proof_records);
    assert_duplicate_graph_domain_rejected!(reaffirmations);
    let all_impacts = change_impacts(&SpecGraph::default(), &graph);
    assert_eq!(all_impacts.len(), 17);
    assert!(all_impacts.iter().all(|impact| impact.direct));
    assert_eq!(
        graph.implementation_links[0].navigation(),
        NavigationKind::Implements
    );
    assert_eq!(graph.oracles[0].selection().bytes(), [1]);
    assert_eq!(graph.oracles[0].normalization().bytes(), [2]);
    assert_eq!(graph.oracles[0].configuration(), digest(11));
    assert_eq!(graph.proof_records[0].scope().schema(), "proof-scope/v1");
    assert_eq!(
        graph.formal_properties[0]
            .correspondence_approval()
            .authority(),
        "maintainers"
    );
    assert_eq!(graph.finite_models[0].state_space().bytes(), [1]);
    assert_eq!(graph.finite_models[0].event_alphabet().bytes(), [2]);
    assert_eq!(graph.finite_models[0].transition_relation().bytes(), [3]);
    assert_eq!(graph.finite_models[0].bound().bytes(), [4]);
    assert_eq!(graph.finite_models[0].exclusions(), ["infinite-domain"]);
    assert_eq!(graph.assumptions[0].scope().schema(), "assumption-scope/v1");
    assert_eq!(graph.assumptions[0].exclusions(), ["clock"]);
    assert_eq!(
        graph.reaffirmations[0].review_statement(),
        "The atom was reread under the new binding."
    );
    assert_eq!(graph.debt_records[0].rationale(), "Evidence is pending.");
    assert_eq!(
        graph.debt_records[0].remediation(),
        "Add the missing evidence."
    );
    assert_eq!(graph.debt_records[0].external_references(), ["issue-1"]);
    assert_eq!(graph.waivers[0].rationale(), "Temporary continuation.");
    assert_eq!(graph.waivers[0].external_references(), ["issue-2"]);
}

#[test]
fn remaining_model_variants_preserve_options_references_and_terminal_state() {
    let selector = TypedSelector::new(
        path("src"),
        SchemaId::new("rust/v1").expect("valid schema"),
        vec![String::from("src/**")],
        Vec::new(),
        AuthorityExpectation::Optional,
    )
    .expect("selector");
    let authority = ApprovalAuthority::new(
        ScopeId::new("maintainers").expect("valid scope"),
        vec![payload("approval-action/v1", &[1])],
        vec![ScopeId::new("coverage").expect("valid scope")],
        selector.clone(),
        Some(payload("separation-duty/v1", &[2])),
        approval(),
    )
    .expect("approval authority");
    assert_eq!(authority.authority_id(), "maintainers");
    assert_eq!(authority.allowed_kind_actions()[0].bytes(), [1]);
    assert_eq!(
        authority.allowed_namespaces(),
        [ScopeId::new("coverage").expect("valid scope")]
    );
    assert_eq!(authority.review_record_selector(), &selector);
    assert_eq!(
        authority.separation_of_duty().map(TypedPayload::bytes),
        Some(&[2][..])
    );
    assert_eq!(authority.declaration_approval().authority(), "maintainers");
    assert_ne!(authority.canonical_value(), CanonicalValue::None);

    let namespace = IdNamespace::new(
        ScopeId::new("coverage").expect("valid scope"),
        NamespacePrefix::new("COV").expect("valid prefix"),
        payload("allocation-policy/v1", &[3]),
        vec![ScopeId::new("maintainers").expect("valid scope")],
    )
    .expect("namespace");
    let allocation = IdAllocation::new(
        ScopeId::new("coverage").expect("valid scope"),
        sid(&String::from("COV-ATOM")),
        1,
        IdAllocationClass::Live,
        digest(60),
    )
    .expect("allocation");
    let policy_ref = ObjectRef::new(ObjectKind::CoveragePolicy, sid("COV-POLICY"), digest(61));
    let config = SemanticRepositoryConfig::new(
        SchemaId::new("repository-config/v1").expect("valid schema"),
        vec![selector],
        vec![namespace],
        vec![authority],
        vec![policy_ref],
        vec![tool("semantic", 62)],
    )
    .expect("semantic repository config");
    assert_eq!(config.configuration_schema_id(), "repository-config/v1");
    assert_eq!(config.selectors().len(), 1);
    assert_eq!(config.id_namespaces().len(), 1);
    assert_eq!(config.approval_authorities().len(), 1);
    assert_eq!(config.policy_references().len(), 1);
    assert_eq!(config.semantic_tools()[0].id(), "semantic");
    assert_ne!(config.digest(), digest(0));

    let subject = ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(63));
    let oracle = ObjectRef::new(ObjectKind::Oracle, sid("COV-ORACLE"), digest(64));
    let verdict = NormalizedVerdict::new(
        oracle.clone(),
        vec![subject.clone()],
        Verdict::Fail,
        Some(payload("semantic-result/v1", &[4])),
    )
    .expect("normalized verdict");
    assert_eq!(verdict.oracle(), &oracle);
    assert_eq!(verdict.selected_subjects(), std::slice::from_ref(&subject));
    assert_eq!(verdict.verdict(), Verdict::Fail);
    assert_eq!(
        verdict.semantic_result().map(TypedPayload::bytes),
        Some(&[4][..])
    );
    assert_ne!(verdict.digest(), digest(0));

    let base_policy = ObjectRef::new(ObjectKind::CoveragePolicy, sid("COV-POLICY"), digest(65));
    let head_policy = ObjectRef::new(ObjectKind::CoveragePolicy, sid("COV-POLICY"), digest(66));
    let obligation = ObligationRef::new(
        sid(&String::from("COV-POLICY")),
        ObligationKind::RequiredRole,
        subject.clone(),
        Some(EvidenceRole::Negative),
    )
    .expect("obligation");
    let impact = PolicyRatchetImpact::new(
        ScopeId::new("coverage-ratchet").expect("valid scope"),
        base_policy.clone(),
        head_policy.clone(),
        vec![obligation.clone()],
    )
    .expect("ratchet impact");
    assert_eq!(impact.ratchet_id(), "coverage-ratchet");
    assert_eq!(impact.base_policy(), &base_policy);
    assert_eq!(impact.head_policy(), &head_policy);
    assert_eq!(
        impact.weakened_obligations(),
        std::slice::from_ref(&obligation)
    );

    let terminal_revision = CoveragePolicyRevision::new(
        payload("policy-scope/v1", &[5]),
        vec![EvidenceRole::Negative],
        EnforcementMode::Advisory,
        ScopeId::new("coverage-ratchet").expect("valid scope"),
        Vec::new(),
        None,
    )
    .expect("terminal revision");
    let terminal_policy = CoveragePolicy::new(
        sid(&String::from("COV-POLICY-OLD")),
        terminal_revision,
        approval(),
        PolicyLifecycle::Superseded,
        vec![ObjectRef::new(
            ObjectKind::CoveragePolicy,
            sid("COV-POLICY-NEW"),
            digest(67),
        )],
        Some(approval()),
        vec![subject.clone()],
    )
    .expect("terminal policy");
    assert_eq!(terminal_policy.lifecycle(), PolicyLifecycle::Superseded);
    assert_eq!(terminal_policy.replacements().len(), 1);
    assert!(terminal_policy.terminal_approval().is_some());
    assert_eq!(terminal_policy.subjects(), std::slice::from_ref(&subject));
    assert_eq!(terminal_policy.roles(), [EvidenceRole::Negative]);

    let waiver_ref = ObjectRef::new(ObjectKind::Waiver, sid("COV-WAIVER"), digest(68));
    let disposition = ChangeDisposition::new(
        sid("COV-DISPOSITION"),
        ObjectKind::Atom,
        sid("COV-ATOM"),
        Some(digest(63)),
        Some(digest(69)),
        ChangeDispositionKind::Waived,
        None,
        Some(waiver_ref.clone()),
        Vec::new(),
    )
    .expect("waived disposition");
    assert_eq!(disposition.impacted_kind(), ObjectKind::Atom);
    assert_eq!(disposition.impacted_id().as_str(), "COV-ATOM");
    assert_eq!(disposition.base(), Some(digest(63)));
    assert_eq!(disposition.head(), Some(digest(69)));
    assert_eq!(disposition.disposition(), ChangeDispositionKind::Waived);
    assert_eq!(disposition.reaffirmation_digest(), None);
    assert_eq!(disposition.waiver(), Some(&waiver_ref));
    assert!(disposition.replacements().is_empty());
    let manifest = ChangeManifest::new(
        sid("COV-MANIFEST"),
        digest(70),
        digest(71),
        vec![disposition],
        Some(approval()),
    )
    .expect("change manifest");
    assert_eq!(manifest.base_graph(), digest(70));
    assert_eq!(manifest.head_graph(), digest(71));
    assert_eq!(manifest.dispositions().len(), 1);
    assert!(manifest.approval().is_some());

    assert_eq!(allocation.namespace_id().as_str(), "coverage");
    assert_eq!(allocation.stable_id().as_str(), "COV-ATOM");
    assert_eq!(allocation.ordinal(), 1);
    assert_eq!(allocation.class(), IdAllocationClass::Live);
    assert_eq!(allocation.allocation_digest(), digest(60));
}

#[test]
fn model_constructors_reject_each_remaining_invalid_shape() {
    assert_eq!(
        StableId::new("A").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-STABLE-ID")
    );
    for malformed_id in ["-AB", "Ab-B", "AB--CD"] {
        assert_eq!(
            StableId::new(malformed_id).err().map(IdentifierError::code),
            Some("SCF-CONSTRUCT-STABLE-ID")
        );
    }
    for malformed_schema in [
        ".ab", "a..b", "/v1", "bad//v1", "bad/v", "bad/vx", "bad/v01",
    ] {
        assert_eq!(
            SchemaId::new(malformed_schema)
                .err()
                .map(ConstructionError::from),
            Some(ConstructionError::SchemaIdGrammar)
        );
    }
    assert_eq!(
        SchemaId::new("invalid/1")
            .err()
            .map(ConstructionError::from),
        Some(ConstructionError::SchemaIdGrammar)
    );
    assert_eq!(
        SchemaId::new("invalid/x1")
            .err()
            .map(ConstructionError::from),
        Some(ConstructionError::SchemaIdGrammar)
    );
    assert!(TypedPayload::new(SchemaId::new("schema").expect("valid schema"), Vec::new()).is_ok());

    let selector = TypedSelector::new(
        path("src"),
        SchemaId::new("rust/v1").expect("valid schema"),
        vec![String::from("**/*.rs")],
        Vec::new(),
        AuthorityExpectation::Required,
    )
    .expect("selector");
    assert_eq!(
        ApprovalAuthority::new(
            ScopeId::new("maintainers").expect("valid scope"),
            Vec::new(),
            vec![ScopeId::new("coverage").expect("valid scope")],
            selector.clone(),
            None,
            approval(),
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );
    assert_eq!(
        ApprovalAuthority::new(
            ScopeId::new("maintainers").expect("valid scope"),
            vec![payload("approval-action/v1", &[1])],
            Vec::new(),
            selector.clone(),
            None,
            approval(),
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );
    assert_eq!(
        SemanticRepositoryConfig::new(
            SchemaId::new("repository-config/v1").expect("valid schema"),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );
    assert_eq!(
        SemanticRepositoryConfig::new(
            SchemaId::new("repository-config/v1").expect("valid schema"),
            vec![selector],
            Vec::new(),
            Vec::new(),
            vec![ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(2))],
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    for invalid_name in ["", "lower", "UPPER-INVALID"] {
        assert_eq!(
            EnvironmentBinding::new(invalid_name, "value").err(),
            Some(ConstructionError::LocalInvariant)
        );
    }
    assert_eq!(
        Atom::new(
            sid("COV-ATOM"),
            LifecycleState::Proposed,
            AtomKind::Constraint,
            Vec::new(),
            "The invariant MUST remain true.",
            None,
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );

    assert_eq!(
        ScopeId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_eq!(
        NamespacePrefix::new("bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    for malformed_prefix in ["", "COV--ALT", "E\u{301}"] {
        assert_eq!(
            NamespacePrefix::new(malformed_prefix)
                .err()
                .map(IdentifierError::code),
            Some("SCF-CONSTRUCT-SCOPE-ID")
        );
    }
    assert_eq!(
        ScopeId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    let namespace = IdNamespace::new(
        ScopeId::new("coverage").expect("valid scope"),
        NamespacePrefix::new("COV").expect("valid prefix"),
        payload("allocation-policy/v1", &[1]),
        vec![ScopeId::new("maintainers").expect("valid scope")],
    )
    .expect("namespace");
    assert_eq!(
        ScopeId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_eq!(
        StableId::new("bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-STABLE-ID")
    );
    assert_eq!(
        IdAllocation::new(
            ScopeId::new("coverage").expect("valid scope"),
            sid("COV-ATOM"),
            0,
            IdAllocationClass::Live,
            digest(3)
        )
        .err(),
        Some(RegistryError::Ordinal)
    );
    assert_eq!(
        IdRegistry::new(vec![namespace.clone(), namespace.clone()], Vec::new()).err(),
        Some(RegistryError::Duplicate)
    );
    let wrong_namespace = IdAllocation::new(
        ScopeId::new("other").expect("valid scope"),
        sid("COV-ATOM"),
        1,
        IdAllocationClass::Live,
        digest(3),
    )
    .expect("allocation with independently valid namespace name");
    assert_eq!(
        IdRegistry::new(vec![namespace.clone()], vec![wrong_namespace]).err(),
        Some(RegistryError::Namespace)
    );
    let wrong_prefix = IdAllocation::new(
        ScopeId::new("coverage").expect("valid scope"),
        sid("ALT-ATOM"),
        1,
        IdAllocationClass::Live,
        digest(3),
    )
    .expect("allocation before registry prefix check");
    assert_eq!(
        IdRegistry::new(vec![namespace.clone()], vec![wrong_prefix]).err(),
        Some(RegistryError::Prefix)
    );
    let prefix_without_separator = IdAllocation::new(
        ScopeId::new("coverage").expect("valid scope"),
        sid("COVX-ATOM"),
        1,
        IdAllocationClass::Live,
        digest(3),
    )
    .expect("allocation before separator check");
    assert_eq!(
        IdRegistry::new(vec![namespace.clone()], vec![prefix_without_separator]).err(),
        Some(RegistryError::Prefix)
    );
    let duplicate_prefix_namespace = IdNamespace::new(
        ScopeId::new("other").expect("valid scope"),
        NamespacePrefix::new("COV").expect("valid prefix"),
        payload("allocation-policy/v1", &[1]),
        Vec::new(),
    )
    .expect("namespace with independently valid duplicate prefix");
    assert_eq!(
        IdRegistry::new(
            vec![namespace.clone(), duplicate_prefix_namespace],
            Vec::new()
        )
        .err(),
        Some(RegistryError::Duplicate)
    );
    let allocation_a = IdAllocation::new(
        ScopeId::new("coverage").expect("valid scope"),
        sid("COV-A"),
        1,
        IdAllocationClass::Live,
        digest(36),
    )
    .expect("allocation");
    let allocation_b = IdAllocation::new(
        ScopeId::new("coverage").expect("valid scope"),
        sid("COV-B"),
        1,
        IdAllocationClass::Live,
        digest(37),
    )
    .expect("allocation");
    assert_eq!(
        IdRegistry::new(
            vec![namespace.clone()],
            vec![allocation_a.clone(), allocation_b]
        )
        .err(),
        Some(RegistryError::Duplicate)
    );
    let terminal_registry = IdRegistry::new(
        vec![namespace.clone()],
        vec![
            IdAllocation::new(
                ScopeId::new("coverage").expect("valid scope"),
                sid("COV-A"),
                1,
                IdAllocationClass::Terminal,
                digest(36),
            )
            .expect("terminal allocation"),
        ],
    )
    .expect("terminal registry");
    let live_registry =
        IdRegistry::new(vec![namespace.clone()], vec![allocation_a]).expect("live registry");
    assert_eq!(
        terminal_registry.validate_successor(&live_registry),
        Err(RegistryError::Reuse)
    );
    let changed_binding_registry = IdRegistry::new(
        vec![namespace],
        vec![
            IdAllocation::new(
                ScopeId::new("coverage").expect("valid scope"),
                sid("COV-A"),
                1,
                IdAllocationClass::Terminal,
                digest(38),
            )
            .expect("changed binding"),
        ],
    )
    .expect("changed registry");
    assert_eq!(
        terminal_registry.validate_successor(&changed_binding_registry),
        Err(RegistryError::History)
    );

    let mapped = ObjectRef::new(ObjectKind::ImplementationLink, sid("COV-LINK"), digest(4));
    let entry_a = SourceMapEntry::new(
        mapped.clone(),
        vec![SourceRef::new(path("src/a.rs"), 0, 1).expect("source")],
        SourceRole::Implementation,
    )
    .expect("source entry");
    let entry_b = SourceMapEntry::new(
        mapped,
        vec![SourceRef::new(path("src/b.rs"), 0, 1).expect("source")],
        SourceRole::Implementation,
    )
    .expect("source entry");
    assert_eq!(
        SourceMap::new(vec![entry_a, entry_b]).err(),
        Some(ConstructionError::LocalInvariant)
    );

    let atom_reference = AtomRevisionRef::new(sid("COV-ATOM"), digest(5));
    let artifact_a = ArtifactBinding::repository(
        path("artifacts/a"),
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        digest(6),
    )
    .expect("artifact");
    let artifact_b = ArtifactBinding::repository(
        path("artifacts/b"),
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        digest(7),
    )
    .expect("artifact");
    assert_eq!(
        Derivation::new(
            sid("COV-DERIVATION-EMPTY"),
            Vec::new(),
            vec![artifact_b.clone()],
            tool("generator", 8),
            digest(9),
            ObjectRef::new(ObjectKind::Oracle, sid("COV-ORACLE"), digest(10)),
            DerivationRelation::GeneratedFrom,
            payload("derivation-scope/v1", &[1]),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );
    assert_eq!(
        Derivation::new(
            sid("COV-DERIVATION-EMPTY"),
            vec![artifact_a.clone()],
            Vec::new(),
            tool("generator", 8),
            digest(9),
            ObjectRef::new(ObjectKind::Oracle, sid("COV-ORACLE"), digest(10)),
            DerivationRelation::GeneratedFrom,
            payload("derivation-scope/v1", &[1]),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );
    assert_eq!(
        Derivation::new(
            sid("COV-DERIVATION"),
            vec![artifact_a],
            vec![artifact_b],
            tool("generator", 8),
            digest(9),
            ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(10)),
            DerivationRelation::GeneratedFrom,
            payload("derivation-scope/v1", &[1]),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    let byte_binding = |name: &str, byte: u8| {
        ArtifactBinding::repository(
            path(name),
            "raw-artifact",
            "application/octet-stream",
            "raw/v1",
            digest(byte),
        )
        .expect("byte binding")
    };
    assert_eq!(
        Derivation::new(
            sid("COV-DERIVATION-BYTE"),
            vec![
                byte_binding("artifacts/source-a", 39),
                byte_binding("artifacts/source-b", 40)
            ],
            vec![byte_binding("artifacts/target", 41)],
            tool("generator", 42),
            digest(44),
            ObjectRef::new(ObjectKind::Oracle, sid("COV-ORACLE"), digest(45)),
            DerivationRelation::ByteEqual,
            payload("derivation-scope/v1", &[1]),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );
    assert_eq!(
        Derivation::new(
            sid("COV-DERIVATION-BYTE-TARGETS"),
            vec![byte_binding("artifacts/source", 47)],
            vec![
                byte_binding("artifacts/target-a", 48),
                byte_binding("artifacts/target-b", 49),
            ],
            tool("generator", 50),
            digest(52),
            ObjectRef::new(ObjectKind::Oracle, sid("COV-ORACLE"), digest(53)),
            DerivationRelation::ByteEqual,
            payload("derivation-scope/v1", &[1]),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );

    let revision = CoveragePolicyRevision::new(
        payload("policy-scope/v1", &[1]),
        vec![EvidenceRole::Positive],
        EnforcementMode::Blocking,
        ScopeId::new("coverage-ratchet").expect("valid scope"),
        Vec::new(),
        None,
    )
    .expect("revision");
    let self_policy = ObjectRef::new(ObjectKind::CoveragePolicy, sid("COV-POLICY"), digest(11));
    assert_eq!(
        CoveragePolicy::new(
            sid("COV-POLICY"),
            revision,
            approval(),
            PolicyLifecycle::Superseded,
            vec![self_policy.clone()],
            Some(approval()),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );
    assert_eq!(
        CoveragePolicy::new(
            sid("COV-POLICY-WRONG"),
            CoveragePolicyRevision::new(
                payload("policy-scope/v1", &[1]),
                vec![EvidenceRole::Positive],
                EnforcementMode::Blocking,
                ScopeId::new("coverage-ratchet").expect("valid scope"),
                Vec::new(),
                None,
            )
            .expect("revision"),
            approval(),
            PolicyLifecycle::Superseded,
            vec![ObjectRef::new(
                ObjectKind::Atom,
                sid("COV-ATOM"),
                digest(11)
            )],
            Some(approval()),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    let policy_revision = || {
        CoveragePolicyRevision::new(
            payload("policy-scope/v1", &[1]),
            vec![EvidenceRole::Positive],
            EnforcementMode::Blocking,
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("revision")
    };
    let replacement_policy = ObjectRef::new(
        ObjectKind::CoveragePolicy,
        sid("COV-POLICY-NEW"),
        digest(12),
    );
    let invalid_policy_shapes = [
        CoveragePolicy::new(
            sid("COV-POLICY-ACTIVE"),
            policy_revision(),
            approval(),
            PolicyLifecycle::Active,
            Vec::new(),
            Some(approval()),
            Vec::new(),
        ),
        CoveragePolicy::new(
            sid("COV-POLICY-SUPERSEDED"),
            policy_revision(),
            approval(),
            PolicyLifecycle::Superseded,
            Vec::new(),
            Some(approval()),
            Vec::new(),
        ),
        CoveragePolicy::new(
            sid("COV-POLICY-RETIRED"),
            policy_revision(),
            approval(),
            PolicyLifecycle::Retired,
            vec![replacement_policy],
            Some(approval()),
            Vec::new(),
        ),
    ];
    assert!(
        invalid_policy_shapes
            .into_iter()
            .all(|result| result.err() == Some(ConstructionError::LifecycleShape))
    );

    let obligation = ObligationRef::new(
        sid("COV-POLICY"),
        ObligationKind::Freshness,
        ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(12)),
        None,
    )
    .expect("obligation");
    assert_eq!(
        PolicyRatchetImpact::new(
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(13)),
            self_policy.clone(),
            vec![obligation.clone()],
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    assert_eq!(
        PolicyRatchetImpact::new(
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            self_policy.clone(),
            ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(13)),
            vec![obligation.clone()],
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    assert_eq!(
        PolicyRatchetImpact::new(
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            self_policy.clone(),
            self_policy,
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );

    assert_eq!(
        DebtRecord::new(
            sid("COV-DEBT"),
            Vec::new(),
            Vec::new(),
            "Debt.",
            ScopeId::new("maintainers").expect("valid scope"),
            "Fix it.",
            payload("debt-discharge/v1", &[1]),
            Vec::new(),
            DebtLifecycle::Active,
            Vec::new(),
            Vec::new(),
            None,
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );
    let wrong_debt_replacement = ObjectRef::new(ObjectKind::Waiver, sid("COV-WAIVER"), digest(14));
    assert_eq!(
        DebtRecord::new(
            sid("COV-DEBT"),
            vec![obligation.clone()],
            Vec::new(),
            "Debt.",
            ScopeId::new("maintainers").expect("valid scope"),
            "Fix it.",
            payload("debt-discharge/v1", &[1]),
            Vec::new(),
            DebtLifecycle::Superseded,
            Vec::new(),
            vec![wrong_debt_replacement],
            Some(approval()),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    let self_debt = ObjectRef::new(ObjectKind::DebtRecord, sid("COV-DEBT"), digest(15));
    assert_eq!(
        DebtRecord::new(
            sid("COV-DEBT"),
            vec![obligation.clone()],
            Vec::new(),
            "Debt.",
            ScopeId::new("maintainers").expect("valid scope"),
            "Fix it.",
            payload("debt-discharge/v1", &[1]),
            Vec::new(),
            DebtLifecycle::Superseded,
            Vec::new(),
            vec![self_debt],
            Some(approval()),
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );

    assert_eq!(
        Waiver::new(
            sid("COV-WAIVER"),
            Vec::new(),
            Vec::new(),
            "Waiver.",
            ScopeId::new("maintainers").expect("valid scope"),
            payload("waiver-discharge/v1", &[1]),
            approval(),
            Vec::new(),
            WaiverLifecycle::Active,
            Vec::new(),
            Vec::new(),
            None,
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );
    let wrong_waiver_replacement =
        ObjectRef::new(ObjectKind::DebtRecord, sid("COV-DEBT"), digest(16));
    assert_eq!(
        Waiver::new(
            sid("COV-WAIVER"),
            vec![obligation.clone()],
            Vec::new(),
            "Waiver.",
            ScopeId::new("maintainers").expect("valid scope"),
            payload("waiver-discharge/v1", &[1]),
            approval(),
            Vec::new(),
            WaiverLifecycle::Superseded,
            Vec::new(),
            vec![wrong_waiver_replacement],
            Some(approval()),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    let self_waiver = ObjectRef::new(ObjectKind::Waiver, sid("COV-WAIVER"), digest(17));
    assert_eq!(
        Waiver::new(
            sid("COV-WAIVER"),
            vec![obligation.clone()],
            Vec::new(),
            "Waiver.",
            ScopeId::new("maintainers").expect("valid scope"),
            payload("waiver-discharge/v1", &[1]),
            approval(),
            Vec::new(),
            WaiverLifecycle::Superseded,
            Vec::new(),
            vec![self_waiver],
            Some(approval()),
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );

    let superseded_debt = DebtRecord::new(
        sid("COV-DEBT-OLD"),
        vec![obligation.clone()],
        Vec::new(),
        "Debt replaced.",
        ScopeId::new("maintainers").expect("valid scope"),
        "Track replacement.",
        payload("debt-discharge/v1", &[1]),
        Vec::new(),
        DebtLifecycle::Superseded,
        Vec::new(),
        vec![ObjectRef::new(
            ObjectKind::DebtRecord,
            sid("COV-DEBT-NEW"),
            digest(17),
        )],
        Some(approval()),
    )
    .expect("valid superseded debt");
    assert_eq!(superseded_debt.lifecycle(), DebtLifecycle::Superseded);
    let superseded_waiver = Waiver::new(
        sid("COV-WAIVER-OLD"),
        vec![obligation],
        Vec::new(),
        "Waiver replaced.",
        ScopeId::new("maintainers").expect("valid scope"),
        payload("waiver-discharge/v1", &[1]),
        approval(),
        Vec::new(),
        WaiverLifecycle::Superseded,
        Vec::new(),
        vec![ObjectRef::new(
            ObjectKind::Waiver,
            sid("COV-WAIVER-NEW"),
            digest(18),
        )],
        Some(approval()),
    )
    .expect("valid superseded waiver");
    assert_eq!(superseded_waiver.lifecycle(), WaiverLifecycle::Superseded);
    assert_eq!(
        DebtRecord::new(
            sid("COV-DEBT-INVALID-LIFECYCLE"),
            vec![superseded_debt.obligations()[0].clone()],
            Vec::new(),
            "Invalid debt shape.",
            ScopeId::new("maintainers").expect("valid scope"),
            "Correct the shape.",
            payload("debt-discharge/v1", &[1]),
            Vec::new(),
            DebtLifecycle::Active,
            Vec::new(),
            Vec::new(),
            Some(approval()),
        )
        .err(),
        Some(ConstructionError::LifecycleShape)
    );
    assert_eq!(
        Waiver::new(
            sid("COV-WAIVER-INVALID-LIFECYCLE"),
            vec![superseded_waiver.obligations()[0].clone()],
            Vec::new(),
            "Invalid waiver shape.",
            ScopeId::new("maintainers").expect("valid scope"),
            payload("waiver-discharge/v1", &[1]),
            approval(),
            Vec::new(),
            WaiverLifecycle::Active,
            Vec::new(),
            Vec::new(),
            Some(approval()),
        )
        .err(),
        Some(ConstructionError::LifecycleShape)
    );
    let discharged_waiver = Waiver::new(
        sid("COV-WAIVER-DISCHARGED"),
        vec![superseded_waiver.obligations()[0].clone()],
        Vec::new(),
        "Waiver discharged.",
        ScopeId::new("maintainers").expect("valid scope"),
        payload("waiver-discharge/v1", &[1]),
        approval(),
        Vec::new(),
        WaiverLifecycle::Discharged,
        vec![ObjectRef::new(
            ObjectKind::ProofRecord,
            sid("COV-PROOF"),
            digest(46),
        )],
        Vec::new(),
        Some(approval()),
    )
    .expect("valid discharged waiver");
    assert_eq!(discharged_waiver.lifecycle(), WaiverLifecycle::Discharged);

    let revised_without_pair = ChangeDisposition::new(
        sid("COV-DISPOSITION"),
        ObjectKind::Atom,
        sid("COV-ATOM"),
        Some(digest(18)),
        None,
        ChangeDispositionKind::Revised,
        None,
        None,
        Vec::new(),
    );
    assert_eq!(
        revised_without_pair.err(),
        Some(ConstructionError::ChangeDispositionShape)
    );
    let waiver_reference = ObjectRef::new(ObjectKind::Waiver, sid("COV-WAIVER"), digest(19));
    let invalid_shapes = [
        ChangeDisposition::new(
            sid("COV-DISPOSITION-A"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            Some(digest(18)),
            Some(digest(19)),
            ChangeDispositionKind::Reaffirmed,
            None,
            None,
            Vec::new(),
        ),
        ChangeDisposition::new(
            sid("COV-DISPOSITION-B"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            Some(digest(18)),
            Some(digest(19)),
            ChangeDispositionKind::Revised,
            Some(digest(20)),
            None,
            Vec::new(),
        ),
        ChangeDisposition::new(
            sid("COV-DISPOSITION-C"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            None,
            None,
            ChangeDispositionKind::Retired,
            None,
            None,
            Vec::new(),
        ),
        ChangeDisposition::new(
            sid("COV-DISPOSITION-D"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            None,
            None,
            ChangeDispositionKind::Waived,
            None,
            Some(waiver_reference.clone()),
            Vec::new(),
        ),
        ChangeDisposition::new(
            sid("COV-DISPOSITION-E"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            Some(digest(18)),
            Some(digest(19)),
            ChangeDispositionKind::Waived,
            Some(digest(20)),
            Some(waiver_reference),
            Vec::new(),
        ),
    ];
    assert!(
        invalid_shapes
            .into_iter()
            .all(|result| result.err() == Some(ConstructionError::ChangeDispositionShape))
    );

    let valid_disposition = ChangeDisposition::new(
        sid("COV-DISPOSITION"),
        ObjectKind::Atom,
        sid("COV-ATOM"),
        Some(digest(18)),
        Some(digest(19)),
        ChangeDispositionKind::Revised,
        None,
        None,
        Vec::new(),
    )
    .expect("valid disposition");
    assert_eq!(
        ChangeManifest::new(
            sid("COV-MANIFEST"),
            digest(20),
            digest(20),
            vec![valid_disposition],
            None,
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );
    let duplicate_impact_a = ChangeDisposition::new(
        sid("COV-DISPOSITION-DUP-A"),
        ObjectKind::Atom,
        sid("COV-DUPLICATE"),
        Some(digest(21)),
        Some(digest(22)),
        ChangeDispositionKind::Revised,
        None,
        None,
        Vec::new(),
    )
    .expect("first duplicate-key disposition");
    let duplicate_impact_b = ChangeDisposition::new(
        sid("COV-DISPOSITION-DUP-B"),
        ObjectKind::Atom,
        sid("COV-DUPLICATE"),
        Some(digest(23)),
        Some(digest(24)),
        ChangeDispositionKind::Revised,
        None,
        None,
        Vec::new(),
    )
    .expect("second duplicate-key disposition");
    assert_eq!(
        ChangeManifest::new(
            sid("COV-MANIFEST-DUPLICATE"),
            digest(25),
            digest(26),
            vec![duplicate_impact_a, duplicate_impact_b],
            None,
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );

    assert!(
        ChangeManifest::new(
            sid("COV-MANIFEST-NO-CHANGE"),
            digest(27),
            digest(27),
            Vec::new(),
            None,
        )
        .is_ok()
    );

    assert_eq!(
        Assumption::new(
            sid("COV-ASSUMPTION"),
            "The environment is stable.",
            payload("assumption-scope/v1", &[1]),
            vec![String::new()],
            approval(),
        )
        .err(),
        Some(ConstructionError::EmptyText)
    );

    let surface_member = payload("surface-member/v1", &[1]);
    let surface_domain = DecisionSurfaceDomain::new(
        sid("COV-SURFACE"),
        payload("surface-member/v1", &[]),
        payload("surface-disposition/v1", &[]),
        vec![surface_member.clone()],
        1,
    )
    .expect("surface domain");
    let surface_row = SurfaceDisposition::new(
        surface_member,
        SurfaceDispositionKind::NotApplicable,
        None,
        None,
    )
    .expect("surface row");
    assert_eq!(
        DecisionSurface::new(
            surface_domain,
            vec![surface_row],
            vec![ObjectRef::new(
                ObjectKind::Atom,
                sid("COV-ATOM"),
                digest(21)
            )],
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );

    assert_eq!(
        EvidenceCarrier::new(
            sid("COV-CARRIER"),
            EvidenceRole::Positive,
            vec![atom_reference.clone()],
            digest(22),
            ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(23)),
            digest(24),
            Vec::new(),
            payload("carrier-scope/v1", &[1]),
            Vec::new(),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    assert_eq!(
        EvidenceCarrier::new(
            sid("COV-CARRIER-ASSUMPTION"),
            EvidenceRole::Positive,
            vec![atom_reference.clone()],
            digest(22),
            ObjectRef::new(ObjectKind::Oracle, sid("COV-ORACLE"), digest(23)),
            digest(24),
            Vec::new(),
            payload("carrier-scope/v1", &[1]),
            vec![ObjectRef::new(
                ObjectKind::Atom,
                sid("COV-ATOM"),
                digest(25)
            )],
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );

    assert_eq!(
        FormalProperty::new(
            sid("COV-PROPERTY"),
            vec![atom_reference.clone()],
            payload("formal-property/v1", &[1]),
            payload("implementation-abstraction/v1", &[1]),
            vec![ObjectRef::new(
                ObjectKind::Atom,
                sid("COV-ATOM"),
                digest(25)
            )],
            Vec::new(),
            approval(),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );

    let valid_property =
        ObjectRef::new(ObjectKind::FormalProperty, sid("COV-PROPERTY"), digest(26));
    let valid_implementation = || {
        ArtifactBinding::object(ObjectRef::new(
            ObjectKind::ImplementationLink,
            sid("COV-IMPLEMENTATION"),
            digest(27),
        ))
    };
    let valid_assumption =
        ObjectRef::new(ObjectKind::Assumption, sid("COV-ASSUMPTION"), digest(28));
    let proof_source = || {
        ArtifactBinding::repository(
            path("proofs/proof.smt2"),
            "raw-artifact",
            "text/plain",
            "smt2/v1",
            digest(31),
        )
        .expect("proof source")
    };
    assert_eq!(
        proof_fixture(
            "COV-PROOF-NO-ATOMS",
            Vec::new(),
            valid_property.clone(),
            valid_implementation(),
            Vec::new(),
            vec![proof_source()],
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );
    assert_eq!(
        proof_fixture(
            "COV-PROOF-NO-SOURCE",
            vec![atom_reference.clone()],
            valid_property.clone(),
            valid_implementation(),
            Vec::new(),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::EmptySet)
    );
    assert_eq!(
        proof_fixture(
            "COV-PROOF-ASSUMPTION",
            vec![atom_reference.clone()],
            valid_property.clone(),
            valid_implementation(),
            vec![ObjectRef::new(
                ObjectKind::Atom,
                sid("COV-ATOM"),
                digest(29)
            )],
            vec![proof_source()],
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    assert_eq!(
        proof_fixture(
            "COV-PROOF-IMPLEMENTATION",
            vec![atom_reference.clone()],
            valid_property,
            ArtifactBinding::object(ObjectRef::new(
                ObjectKind::Atom,
                sid("COV-ATOM"),
                digest(30)
            ),),
            vec![valid_assumption],
            vec![proof_source()],
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );

    assert_eq!(
        ProofRecord::new(
            sid("COV-PROOF"),
            vec![atom_reference.clone()],
            ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(26)),
            ArtifactBinding::object(ObjectRef::new(
                ObjectKind::ImplementationLink,
                sid("COV-IMPLEMENTATION"),
                digest(27),
            ),),
            Vec::new(),
            Vec::new(),
            tool("verifier", 28),
            digest(30),
            Vec::new(),
            Vec::new(),
            vec![
                ArtifactBinding::repository(
                    path("proofs/proof.smt2"),
                    "raw-artifact",
                    "text/plain",
                    "smt2/v1",
                    digest(31),
                )
                .expect("proof source")
            ],
            Verdict::Pass,
            payload("proof-scope/v1", &[1]),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );

    let wrong_replacement = ObjectRef::new(ObjectKind::Waiver, sid("COV-WAIVER"), digest(32));
    assert_eq!(
        ChangeDisposition::new(
            sid("COV-DISPOSITION-WRONG"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            Some(digest(33)),
            Some(digest(34)),
            ChangeDispositionKind::Revised,
            None,
            None,
            vec![wrong_replacement],
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    let self_replacement = ObjectRef::new(ObjectKind::Atom, sid("COV-ATOM"), digest(35));
    assert_eq!(
        ChangeDisposition::new(
            sid("COV-DISPOSITION-SELF"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            Some(digest(33)),
            Some(digest(34)),
            ChangeDispositionKind::Revised,
            None,
            None,
            vec![self_replacement],
        )
        .err(),
        Some(ConstructionError::LocalInvariant)
    );

    assert_eq!(atom_reference.id().as_str(), "COV-ATOM");
}
