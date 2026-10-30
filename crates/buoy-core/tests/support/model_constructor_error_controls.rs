#![doc = "Behavioral controls for every public model-constructor error boundary."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy_core::*;

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn sid(value: &str) -> StableId {
    StableId::new(value).expect("valid stable ID")
}

fn path(value: &str) -> RepositoryPath {
    RepositoryPath::new(value).expect("valid repository path")
}

fn scope(value: &str) -> ScopeId {
    ScopeId::new(value).expect("valid scope ID")
}

fn schema(value: &str) -> SchemaId {
    SchemaId::new(value).expect("valid schema ID")
}

fn payload(schema_id: &str, byte: u8) -> TypedPayload {
    TypedPayload::new(schema(schema_id), vec![byte]).expect("valid typed payload")
}

fn approval() -> ApprovalBinding {
    ApprovalBinding::new(scope("maintainers"), path("reviews/approval.md"), digest(1))
        .expect("valid approval")
}

fn selector() -> TypedSelector {
    TypedSelector::new(
        path("src"),
        SchemaId::new("rust/v1").expect("valid schema"),
        vec![String::from("**/*.rs")],
        vec![String::from("target/**")],
        AuthorityExpectation::Required,
    )
    .expect("valid selector")
}

fn authority() -> ApprovalAuthority {
    ApprovalAuthority::new(
        ScopeId::new("maintainers").expect("valid scope"),
        vec![payload("approval-action/v1", 1)],
        vec![ScopeId::new("coverage").expect("valid scope")],
        selector(),
        None,
        approval(),
    )
    .expect("valid authority")
}

fn namespace() -> IdNamespace {
    IdNamespace::new(
        ScopeId::new("coverage").expect("valid scope"),
        NamespacePrefix::new("COV").expect("valid prefix"),
        payload("allocation-policy/v1", 1),
        vec![ScopeId::new("maintainers").expect("valid scope")],
    )
    .expect("valid namespace")
}

fn tool(id: &str, byte: u8) -> ToolIdentity {
    ToolIdentity::new(scope(id), "1.0", digest(byte), digest(byte.wrapping_add(1)))
        .expect("valid tool")
}

fn object(kind: ObjectKind, id: &str, byte: u8) -> ObjectRef {
    ObjectRef::new(kind, sid(id), digest(byte))
}

fn atom_revision() -> AtomRevisionRef {
    AtomRevisionRef::new(sid("COV-ATOM"), digest(2))
}

fn repository_binding(name: &str, byte: u8) -> ArtifactBinding {
    ArtifactBinding::repository(
        path(name),
        "raw-artifact",
        "application/octet-stream",
        "saved-bytes/v1",
        digest(byte),
    )
    .expect("valid repository binding")
}

fn obligation() -> ObligationRef {
    ObligationRef::new(
        sid("COV-POLICY"),
        ObligationKind::Freshness,
        object(ObjectKind::Atom, "COV-ATOM", 30),
        None,
    )
    .expect("valid obligation")
}

#[allow(clippy::too_many_arguments)]
fn debt(
    id: &str,
    rationale: &str,
    owner: &str,
    remediation: &str,
    obligations: Vec<ObligationRef>,
    objects: Vec<ObjectRef>,
    external_references: Vec<String>,
    discharge_evidence: Vec<ObjectRef>,
    replacements: Vec<ObjectRef>,
) -> Result<DebtRecord, ConstructionError> {
    DebtRecord::new(
        sid(id),
        obligations,
        objects,
        rationale,
        ScopeId::new(owner).expect("valid scope"),
        remediation,
        payload("debt-discharge/v1", 1),
        external_references,
        DebtLifecycle::Active,
        discharge_evidence,
        replacements,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn waiver(
    id: &str,
    rationale: &str,
    owner: &str,
    obligations: Vec<ObligationRef>,
    objects: Vec<ObjectRef>,
    external_references: Vec<String>,
    discharge_evidence: Vec<ObjectRef>,
    replacements: Vec<ObjectRef>,
) -> Result<Waiver, ConstructionError> {
    Waiver::new(
        sid(id),
        obligations,
        objects,
        rationale,
        ScopeId::new(owner).expect("valid scope"),
        payload("waiver-discharge/v1", 1),
        approval(),
        external_references,
        WaiverLifecycle::Active,
        discharge_evidence,
        replacements,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn proof(
    id: &str,
    atoms: Vec<AtomRevisionRef>,
    assumptions: Vec<ObjectRef>,
    translators: Vec<ToolIdentity>,
    external_models: Vec<ArtifactBinding>,
    generated: Vec<ArtifactBinding>,
    proof_sources: Vec<ArtifactBinding>,
    exclusions: Vec<String>,
) -> Result<ProofRecord, ConstructionError> {
    ProofRecord::new(
        sid(id),
        atoms,
        object(ObjectKind::FormalProperty, "COV-PROPERTY", 31),
        ArtifactBinding::object(object(
            ObjectKind::ImplementationLink,
            "COV-IMPLEMENTATION",
            32,
        )),
        assumptions,
        translators,
        tool("verifier", 33),
        digest(35),
        external_models,
        generated,
        proof_sources,
        Verdict::Pass,
        payload("proof-scope/v1", 1),
        exclusions,
    )
}

fn duplicate<T: Clone>(value: T) -> Vec<T> {
    vec![value.clone(), value]
}

fn assert_duplicate<T>(result: Result<T, ConstructionError>) {
    assert_eq!(
        result.err(),
        Some(ConstructionError::Encoding(
            EncodingError::DuplicateSetValue
        ))
    );
}

#[test]
fn each_reachable_constructor_error_propagates_its_exact_diagnostic() {
    let non_nfc = "e\u{301}";
    let noncanonical = Some(ConstructionError::Encoding(EncodingError::NonCanonicalText));

    assert_eq!(
        ScopeId::new(non_nfc).err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_eq!(
        SchemaId::new(non_nfc).err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCHEMA-ID")
    );

    // Every stable-ID role now parses through `StableId`, so an invalid ID is
    // rejected once here rather than separately inside each model constructor.
    assert_eq!(
        StableId::new("bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-STABLE-ID")
    );
    assert_eq!(
        StableId::new(non_nfc).err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-STABLE-ID")
    );

    assert_duplicate(TypedSelector::new(
        path("src"),
        SchemaId::new("rust/v1").expect("valid schema"),
        vec![String::from("**/*.rs")],
        vec![String::from("target/**"), String::from("target/**")],
        AuthorityExpectation::Required,
    ));

    assert_eq!(
        ScopeId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    for (actions, namespaces) in [
        (
            Vec::new(),
            vec![ScopeId::new("coverage").expect("valid scope")],
        ),
        (vec![payload("approval-action/v1", 1)], Vec::new()),
    ] {
        assert_eq!(
            ApprovalAuthority::new(
                ScopeId::new("maintainers").expect("valid scope"),
                actions,
                namespaces,
                selector(),
                None,
                approval(),
            )
            .err(),
            Some(ConstructionError::EmptySet)
        );
    }
    let action = payload("approval-action/v1", 1);
    assert_duplicate(ApprovalAuthority::new(
        ScopeId::new("maintainers").expect("valid scope"),
        duplicate(action),
        vec![ScopeId::new("coverage").expect("valid scope")],
        selector(),
        None,
        approval(),
    ));
    assert_eq!(
        ScopeId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_duplicate(ApprovalAuthority::new(
        ScopeId::new("maintainers").expect("valid scope"),
        vec![payload("approval-action/v1", 1)],
        vec![scope("coverage"), scope("coverage")],
        selector(),
        None,
        approval(),
    ));

    assert_eq!(
        SchemaId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCHEMA-ID")
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
            vec![selector()],
            Vec::new(),
            Vec::new(),
            vec![object(ObjectKind::Atom, "COV-ATOM", 2)],
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::ReferenceKind)
    );
    assert_duplicate(SemanticRepositoryConfig::new(
        SchemaId::new("repository-config/v1").expect("valid schema"),
        duplicate(selector()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(SemanticRepositoryConfig::new(
        SchemaId::new("repository-config/v1").expect("valid schema"),
        vec![selector()],
        duplicate(namespace()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(SemanticRepositoryConfig::new(
        SchemaId::new("repository-config/v1").expect("valid schema"),
        vec![selector()],
        Vec::new(),
        duplicate(authority()),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(SemanticRepositoryConfig::new(
        SchemaId::new("repository-config/v1").expect("valid schema"),
        vec![selector()],
        Vec::new(),
        Vec::new(),
        duplicate(object(ObjectKind::CoveragePolicy, "COV-POLICY", 3)),
        Vec::new(),
    ));
    assert_duplicate(SemanticRepositoryConfig::new(
        SchemaId::new("repository-config/v1").expect("valid schema"),
        vec![selector()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        duplicate(tool("scanner", 4)),
    ));

    assert_eq!(
        RejectedEntryObservation::new(path("vendor/link"), EntryKind::Symlink, "").err(),
        Some(ConstructionError::EmptyText)
    );
    assert_eq!(
        EnvironmentBinding::new("LC_ALL", non_nfc).err(),
        noncanonical
    );
    for invalid_name in ["", "lower", "BAD-NAME"] {
        assert_eq!(
            EnvironmentBinding::new(invalid_name, "value").err(),
            Some(ConstructionError::LocalInvariant)
        );
    }
    assert_eq!(
        SchemaId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCHEMA-ID")
    );
    assert_duplicate(SchemaToolLock::new(
        SchemaId::new("canonical-encoding/v1").expect("valid schema"),
        tool("unicode", 5),
        tool("commonmark", 7),
        tool("toml", 9),
        None,
        duplicate(tool("adapter", 11)),
    ));

    assert_duplicate(Tombstone::new(
        sid("COV-TOMBSTONE"),
        LifecycleState::Superseded,
        digest(12),
        duplicate(atom_revision()),
        approval(),
    ));

    let member = payload("surface-member/v1", 1);
    assert_duplicate(DecisionSurfaceDomain::new(
        sid("COV-SURFACE"),
        payload("surface-member/v1", 2),
        payload("surface-disposition/v1", 3),
        duplicate(member.clone()),
        1,
    ));
    let domain = DecisionSurfaceDomain::new(
        sid("COV-SURFACE"),
        payload("surface-member/v1", 2),
        payload("surface-disposition/v1", 3),
        vec![member.clone()],
        1,
    )
    .expect("valid surface domain");
    let row = SurfaceDisposition::new(member, SurfaceDispositionKind::NotApplicable, None, None)
        .expect("valid row");
    assert_duplicate(DecisionSurface::new(
        domain,
        vec![row],
        duplicate(object(ObjectKind::CoveragePolicy, "COV-POLICY", 13)),
    ));

    assert_eq!(
        SchemaId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCHEMA-ID")
    );
    assert_duplicate(ImplementationLink::new(
        sid("COV-LINK"),
        duplicate(atom_revision()),
        digest(14),
        NavigationKind::Implements,
    ));
    assert_duplicate(Oracle::new(
        sid("COV-ORACLE"),
        OracleKind::Test,
        tool("runner", 15),
        payload("oracle-selection/v1", 1),
        payload("oracle-normalization/v1", 2),
        digest(17),
        payload("oracle-scope/v1", 3),
        vec![String::from("network"), String::from("network")],
    ));
    assert_duplicate(NormalizedVerdict::new(
        object(ObjectKind::Oracle, "COV-ORACLE", 18),
        duplicate(object(ObjectKind::Atom, "COV-ATOM", 19)),
        Verdict::Pass,
        None,
    ));

    assert_duplicate(Assumption::new(
        sid("COV-ASSUMPTION"),
        "The environment remains stable.",
        payload("assumption-scope/v1", 1),
        vec![String::from("network"), String::from("network")],
        approval(),
    ));

    let initial = payload("model-state/v1", 1);
    assert_duplicate(FiniteModel::new(
        sid("COV-MODEL"),
        payload("model-state-space/v1", 1),
        payload("model-event/v1", 2),
        payload("model-transition/v1", 3),
        duplicate(initial.clone()),
        payload("model-bound/v1", 4),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(FiniteModel::new(
        sid("COV-MODEL"),
        payload("model-state-space/v1", 1),
        payload("model-event/v1", 2),
        payload("model-transition/v1", 3),
        vec![initial.clone()],
        payload("model-bound/v1", 4),
        duplicate(atom_revision()),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(FiniteModel::new(
        sid("COV-MODEL"),
        payload("model-state-space/v1", 1),
        payload("model-event/v1", 2),
        payload("model-transition/v1", 3),
        vec![initial.clone()],
        payload("model-bound/v1", 4),
        Vec::new(),
        duplicate(object(ObjectKind::Assumption, "COV-ASSUMPTION", 20)),
        Vec::new(),
    ));
    assert_duplicate(FiniteModel::new(
        sid("COV-MODEL"),
        payload("model-state-space/v1", 1),
        payload("model-event/v1", 2),
        payload("model-transition/v1", 3),
        vec![initial],
        payload("model-bound/v1", 4),
        Vec::new(),
        Vec::new(),
        vec![String::from("network"), String::from("network")],
    ));
}

#[test]
fn aggregate_model_constructors_reject_each_duplicate_or_invalid_field() {
    assert_eq!(
        Reaffirmation::new(
            sid("COV-REAFFIRMATION"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            digest(1),
            digest(2),
            approval(),
            "",
        )
        .err(),
        Some(ConstructionError::EmptyText)
    );

    let oracle = object(ObjectKind::Oracle, "COV-ORACLE", 3);
    let assumption = object(ObjectKind::Assumption, "COV-ASSUMPTION", 4);
    assert_duplicate(EvidenceCarrier::new(
        sid("COV-CARRIER"),
        EvidenceRole::Positive,
        duplicate(atom_revision()),
        digest(5),
        oracle.clone(),
        digest(6),
        Vec::new(),
        payload("carrier-scope/v1", 1),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(EvidenceCarrier::new(
        sid("COV-CARRIER"),
        EvidenceRole::Positive,
        vec![atom_revision()],
        digest(5),
        oracle.clone(),
        digest(6),
        duplicate(digest(7)),
        payload("carrier-scope/v1", 1),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(EvidenceCarrier::new(
        sid("COV-CARRIER"),
        EvidenceRole::Positive,
        vec![atom_revision()],
        digest(5),
        oracle.clone(),
        digest(6),
        Vec::new(),
        payload("carrier-scope/v1", 1),
        duplicate(assumption.clone()),
        Vec::new(),
    ));
    assert_duplicate(EvidenceCarrier::new(
        sid("COV-CARRIER"),
        EvidenceRole::Positive,
        vec![atom_revision()],
        digest(5),
        oracle.clone(),
        digest(6),
        Vec::new(),
        payload("carrier-scope/v1", 1),
        Vec::new(),
        vec![String::from("network"), String::from("network")],
    ));

    let non_nfc = "e\u{301}";
    let noncanonical = Some(ConstructionError::Encoding(EncodingError::NonCanonicalText));
    assert_eq!(
        RepositoryArtifactBinding::new(
            path("artifacts/a"),
            "raw-artifact",
            non_nfc,
            "saved-bytes/v1",
            digest(8),
        )
        .err(),
        noncanonical
    );
    assert_eq!(
        RepositoryArtifactBinding::new(
            path("artifacts/a"),
            "raw-artifact",
            "application/octet-stream",
            non_nfc,
            digest(8),
        )
        .err(),
        noncanonical
    );
    assert_eq!(
        ArtifactEntry::new(
            path("artifacts/a"),
            "raw-artifact",
            non_nfc,
            "saved-bytes/v1",
            digest(8),
        )
        .err(),
        noncanonical
    );
    assert_eq!(
        ArtifactEntry::new(
            path("artifacts/a"),
            "raw-artifact",
            "application/octet-stream",
            non_nfc,
            digest(8),
        )
        .err(),
        noncanonical
    );

    let source = repository_binding("artifacts/source", 9);
    let target = repository_binding("artifacts/target", 10);
    assert_duplicate(Derivation::new(
        sid("COV-DERIVATION"),
        duplicate(source.clone()),
        vec![target.clone()],
        tool("generator", 11),
        digest(13),
        oracle.clone(),
        DerivationRelation::GeneratedFrom,
        payload("derivation-scope/v1", 1),
        Vec::new(),
    ));
    assert_duplicate(Derivation::new(
        sid("COV-DERIVATION"),
        vec![source.clone()],
        duplicate(target.clone()),
        tool("generator", 11),
        digest(13),
        oracle.clone(),
        DerivationRelation::GeneratedFrom,
        payload("derivation-scope/v1", 1),
        Vec::new(),
    ));
    assert_duplicate(Derivation::new(
        sid("COV-DERIVATION"),
        vec![source],
        vec![target],
        tool("generator", 11),
        digest(13),
        oracle,
        DerivationRelation::GeneratedFrom,
        payload("derivation-scope/v1", 1),
        vec![String::from("network"), String::from("network")],
    ));

    assert_eq!(
        ScopeId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_duplicate(CoveragePolicyRevision::new(
        payload("policy-scope/v1", 1),
        duplicate(EvidenceRole::Positive),
        EnforcementMode::Blocking,
        ScopeId::new("coverage-ratchet").expect("valid scope"),
        Vec::new(),
        None,
    ));
    assert_duplicate(CoveragePolicyRevision::new(
        payload("policy-scope/v1", 1),
        vec![EvidenceRole::Positive],
        EnforcementMode::Blocking,
        ScopeId::new("coverage-ratchet").expect("valid scope"),
        duplicate(selector()),
        None,
    ));
    let revision = || {
        CoveragePolicyRevision::new(
            payload("policy-scope/v1", 1),
            vec![EvidenceRole::Positive],
            EnforcementMode::Blocking,
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("valid revision")
    };
    assert_duplicate(CoveragePolicy::new(
        sid("COV-POLICY"),
        revision(),
        approval(),
        PolicyLifecycle::Superseded,
        duplicate(object(ObjectKind::CoveragePolicy, "COV-REPLACEMENT", 14)),
        Some(approval()),
        Vec::new(),
    ));
    assert_duplicate(CoveragePolicy::new(
        sid("COV-POLICY"),
        revision(),
        approval(),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        duplicate(object(ObjectKind::Atom, "COV-ATOM", 15)),
    ));

    let policy = object(ObjectKind::CoveragePolicy, "COV-POLICY", 17);
    assert_eq!(
        ScopeId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_duplicate(PolicyRatchetImpact::new(
        ScopeId::new("coverage-ratchet").expect("valid scope"),
        policy.clone(),
        policy,
        duplicate(obligation()),
    ));
}

#[test]
fn debt_waiver_proof_and_disposition_fields_reject_exact_bad_inputs() {
    let obligation = obligation();
    let affected = object(ObjectKind::Atom, "COV-ATOM", 40);
    let evidence = object(ObjectKind::ProofRecord, "COV-PROOF", 41);
    let debt_replacement = object(ObjectKind::DebtRecord, "COV-DEBT-NEW", 42);
    let waiver_replacement = object(ObjectKind::Waiver, "COV-WAIVER-NEW", 43);

    assert_eq!(
        debt(
            "COV-DEBT",
            "",
            "maintainers",
            "Remediate it.",
            vec![obligation.clone()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::EmptyText)
    );
    assert_eq!(
        ScopeId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_eq!(
        debt(
            "COV-DEBT",
            "Visible debt.",
            "maintainers",
            "",
            vec![obligation.clone()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::EmptyText)
    );
    assert_duplicate(debt(
        "COV-DEBT",
        "Visible debt.",
        "maintainers",
        "Remediate it.",
        duplicate(obligation.clone()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(debt(
        "COV-DEBT",
        "Visible debt.",
        "maintainers",
        "Remediate it.",
        vec![obligation.clone()],
        duplicate(affected.clone()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(debt(
        "COV-DEBT",
        "Visible debt.",
        "maintainers",
        "Remediate it.",
        vec![obligation.clone()],
        Vec::new(),
        vec![String::from("ticket"), String::from("ticket")],
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(debt(
        "COV-DEBT",
        "Visible debt.",
        "maintainers",
        "Remediate it.",
        vec![obligation.clone()],
        Vec::new(),
        Vec::new(),
        duplicate(evidence.clone()),
        Vec::new(),
    ));
    assert_duplicate(debt(
        "COV-DEBT",
        "Visible debt.",
        "maintainers",
        "Remediate it.",
        vec![obligation.clone()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        duplicate(debt_replacement),
    ));

    assert_eq!(
        waiver(
            "COV-WAIVER",
            "",
            "maintainers",
            vec![obligation.clone()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .err(),
        Some(ConstructionError::EmptyText)
    );
    assert_eq!(
        ScopeId::new("Bad").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_duplicate(waiver(
        "COV-WAIVER",
        "Process waiver.",
        "maintainers",
        duplicate(obligation.clone()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(waiver(
        "COV-WAIVER",
        "Process waiver.",
        "maintainers",
        vec![obligation.clone()],
        duplicate(affected.clone()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(waiver(
        "COV-WAIVER",
        "Process waiver.",
        "maintainers",
        vec![obligation.clone()],
        Vec::new(),
        vec![String::from("ticket"), String::from("ticket")],
        Vec::new(),
        Vec::new(),
    ));
    assert_duplicate(waiver(
        "COV-WAIVER",
        "Process waiver.",
        "maintainers",
        vec![obligation.clone()],
        Vec::new(),
        Vec::new(),
        duplicate(evidence.clone()),
        Vec::new(),
    ));
    assert_duplicate(waiver(
        "COV-WAIVER",
        "Process waiver.",
        "maintainers",
        vec![obligation],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        duplicate(waiver_replacement),
    ));

    assert_duplicate(FormalProperty::new(
        sid("COV-PROPERTY"),
        duplicate(atom_revision()),
        payload("formal-property/v1", 1),
        payload("implementation-abstraction/v1", 2),
        Vec::new(),
        Vec::new(),
        approval(),
    ));
    assert_duplicate(FormalProperty::new(
        sid("COV-PROPERTY"),
        vec![atom_revision()],
        payload("formal-property/v1", 1),
        payload("implementation-abstraction/v1", 2),
        duplicate(object(ObjectKind::Assumption, "COV-ASSUMPTION", 44)),
        Vec::new(),
        approval(),
    ));
    assert_duplicate(FormalProperty::new(
        sid("COV-PROPERTY"),
        vec![atom_revision()],
        payload("formal-property/v1", 1),
        payload("implementation-abstraction/v1", 2),
        Vec::new(),
        vec![String::from("network"), String::from("network")],
        approval(),
    ));

    let proof_source = repository_binding("proofs/proof.smt2", 45);
    assert_duplicate(proof(
        "COV-PROOF",
        duplicate(atom_revision()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![proof_source.clone()],
        Vec::new(),
    ));
    assert_duplicate(proof(
        "COV-PROOF",
        vec![atom_revision()],
        duplicate(object(ObjectKind::Assumption, "COV-ASSUMPTION", 46)),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![proof_source.clone()],
        Vec::new(),
    ));
    assert_duplicate(proof(
        "COV-PROOF",
        vec![atom_revision()],
        Vec::new(),
        duplicate(tool("translator", 47)),
        Vec::new(),
        Vec::new(),
        vec![proof_source.clone()],
        Vec::new(),
    ));
    assert_duplicate(proof(
        "COV-PROOF",
        vec![atom_revision()],
        Vec::new(),
        Vec::new(),
        duplicate(repository_binding("models/external", 49)),
        Vec::new(),
        vec![proof_source.clone()],
        Vec::new(),
    ));
    assert_duplicate(proof(
        "COV-PROOF",
        vec![atom_revision()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        duplicate(repository_binding("generated/proof", 50)),
        vec![proof_source.clone()],
        Vec::new(),
    ));
    assert_duplicate(proof(
        "COV-PROOF",
        vec![atom_revision()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        duplicate(proof_source.clone()),
        Vec::new(),
    ));
    assert_duplicate(proof(
        "COV-PROOF",
        vec![atom_revision()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![proof_source],
        vec![String::from("network"), String::from("network")],
    ));

    assert_duplicate(ChangeDisposition::new(
        sid("COV-DISPOSITION"),
        ObjectKind::Atom,
        sid("COV-ATOM"),
        Some(digest(51)),
        Some(digest(52)),
        ChangeDispositionKind::Revised,
        None,
        None,
        duplicate(object(ObjectKind::Atom, "COV-REPLACEMENT", 53)),
    ));
}

#[test]
fn error_instantiations_also_retain_their_valid_behavior() {
    assert!(
        RejectedEntryObservation::new(
            path("vendor/link"),
            EntryKind::Symlink,
            "SCF-SNAPSHOT-SYMLINK",
        )
        .is_ok()
    );
    assert!(EnvironmentBinding::new("LC_ALL", "C").is_ok());
    assert!(
        SemanticRepositoryConfig::new(
            SchemaId::new("repository-config/v1").expect("valid schema"),
            vec![selector()],
            vec![namespace()],
            vec![authority()],
            vec![object(ObjectKind::CoveragePolicy, "COV-POLICY", 60)],
            vec![tool("scanner", 61)],
        )
        .is_ok()
    );
    assert!(
        SchemaToolLock::new(
            SchemaId::new("canonical-encoding/v1").expect("valid schema"),
            tool("unicode", 62),
            tool("commonmark", 64),
            tool("toml", 66),
            Some(tool("rust", 68)),
            vec![tool("adapter", 70)],
        )
        .is_ok()
    );
    assert!(
        Tombstone::new(
            sid("COV-TOMBSTONE"),
            LifecycleState::Withdrawn,
            digest(72),
            Vec::new(),
            approval(),
        )
        .is_ok()
    );
    assert!(
        ParsedItemIdentity::new(
            SchemaId::new("rust-item/v1").expect("valid schema"),
            path("src/lib.rs"),
            SourceItemKind::new("rust/item/v1").expect("valid source item kind"),
            vec![1],
            None,
        )
        .is_ok()
    );
    assert!(
        ImplementationLink::new(
            sid("COV-LINK"),
            vec![atom_revision()],
            digest(73),
            NavigationKind::Implements,
        )
        .is_ok()
    );
    assert!(
        Oracle::new(
            sid("COV-ORACLE"),
            OracleKind::Test,
            tool("runner", 74),
            payload("oracle-selection/v1", 1),
            payload("oracle-normalization/v1", 2),
            digest(76),
            payload("oracle-scope/v1", 3),
            vec![String::from("network")],
        )
        .is_ok()
    );
    assert!(
        Assumption::new(
            sid("COV-ASSUMPTION"),
            "The environment remains stable.",
            payload("assumption-scope/v1", 1),
            vec![String::from("network")],
            approval(),
        )
        .is_ok()
    );
    assert!(
        FiniteModel::new(
            sid("COV-MODEL"),
            payload("model-state-space/v1", 1),
            payload("model-event/v1", 2),
            payload("model-transition/v1", 3),
            vec![payload("model-state/v1", 4)],
            payload("model-bound/v1", 5),
            vec![atom_revision()],
            vec![object(ObjectKind::Assumption, "COV-ASSUMPTION", 77)],
            vec![String::from("network")],
        )
        .is_ok()
    );
    assert!(
        Reaffirmation::new(
            sid("COV-REAFFIRMATION"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            digest(78),
            digest(79),
            approval(),
            "Reviewed change.",
        )
        .is_ok()
    );
    assert!(
        EvidenceCarrier::new(
            sid("COV-CARRIER"),
            EvidenceRole::Positive,
            vec![atom_revision()],
            digest(80),
            object(ObjectKind::Oracle, "COV-ORACLE", 81),
            digest(82),
            vec![digest(83)],
            payload("carrier-scope/v1", 1),
            vec![object(ObjectKind::Assumption, "COV-ASSUMPTION", 84)],
            vec![String::from("network")],
        )
        .is_ok()
    );

    assert!(
        RepositoryArtifactBinding::new(
            path("artifacts/current"),
            "raw-artifact",
            "application/octet-stream",
            "saved-bytes/v1",
            digest(85),
        )
        .is_ok()
    );
    assert!(
        ArtifactEntry::new(
            path("artifacts/current"),
            "raw-artifact",
            "application/octet-stream",
            "saved-bytes/v1",
            digest(85),
        )
        .is_ok()
    );
    assert!(
        Derivation::new(
            sid("COV-DERIVATION"),
            vec![repository_binding("artifacts/source", 86)],
            vec![repository_binding("artifacts/target", 87)],
            tool("generator", 88),
            digest(90),
            object(ObjectKind::Oracle, "COV-ORACLE", 91),
            DerivationRelation::ByteEqual,
            payload("derivation-scope/v1", 1),
            vec![String::from("network")],
        )
        .is_ok()
    );

    let revision = CoveragePolicyRevision::new(
        payload("policy-scope/v1", 1),
        vec![EvidenceRole::Positive],
        EnforcementMode::Blocking,
        ScopeId::new("coverage-ratchet").expect("valid scope"),
        vec![selector()],
        Some(payload("derivation-requirement/v1", 1)),
    )
    .expect("valid policy revision");
    assert!(
        CoveragePolicy::new(
            sid("COV-POLICY"),
            revision,
            approval(),
            PolicyLifecycle::Active,
            Vec::new(),
            None,
            vec![object(ObjectKind::Atom, "COV-ATOM", 92)],
        )
        .is_ok()
    );
    let policy = object(ObjectKind::CoveragePolicy, "COV-POLICY", 93);
    assert!(
        PolicyRatchetImpact::new(
            ScopeId::new("coverage-ratchet").expect("valid scope"),
            policy.clone(),
            policy,
            vec![obligation()],
        )
        .is_ok()
    );
    assert!(
        debt(
            "COV-DEBT",
            "Visible debt.",
            "maintainers",
            "Remediate it.",
            vec![obligation()],
            vec![object(ObjectKind::Atom, "COV-ATOM", 94)],
            vec![String::from("ticket")],
            Vec::new(),
            Vec::new(),
        )
        .is_ok()
    );
    assert!(
        waiver(
            "COV-WAIVER",
            "Process waiver.",
            "maintainers",
            vec![obligation()],
            vec![object(ObjectKind::Atom, "COV-ATOM", 95)],
            vec![String::from("ticket")],
            Vec::new(),
            Vec::new(),
        )
        .is_ok()
    );
    assert!(
        FormalProperty::new(
            sid("COV-PROPERTY"),
            vec![atom_revision()],
            payload("formal-property/v1", 1),
            payload("implementation-abstraction/v1", 2),
            vec![object(ObjectKind::Assumption, "COV-ASSUMPTION", 96)],
            vec![String::from("network")],
            approval(),
        )
        .is_ok()
    );
    assert!(
        proof(
            "COV-PROOF",
            vec![atom_revision()],
            vec![object(ObjectKind::Assumption, "COV-ASSUMPTION", 97)],
            vec![tool("translator", 98)],
            vec![repository_binding("models/external", 100)],
            vec![repository_binding("generated/proof", 101)],
            vec![repository_binding("proofs/proof.smt2", 102)],
            vec![String::from("network")],
        )
        .is_ok()
    );
    assert!(
        ChangeDisposition::new(
            sid("COV-DISPOSITION"),
            ObjectKind::Atom,
            sid("COV-ATOM"),
            Some(digest(103)),
            Some(digest(104)),
            ChangeDispositionKind::Revised,
            None,
            None,
            vec![object(ObjectKind::Atom, "COV-REPLACEMENT", 105)],
        )
        .is_ok()
    );
}

#[test]
fn impact_set_rejects_duplicates_within_each_independent_domain() {
    let direct = object(ObjectKind::Atom, "COV-DIRECT", 110);
    let transitive = object(ObjectKind::Oracle, "COV-TRANSITIVE", 111);
    let ratchet = PolicyRatchetImpact::new(
        ScopeId::new("coverage-ratchet").expect("valid scope"),
        object(ObjectKind::CoveragePolicy, "COV-POLICY-BASE", 112),
        object(ObjectKind::CoveragePolicy, "COV-POLICY-HEAD", 113),
        vec![obligation()],
    )
    .expect("valid ratchet impact");

    assert_eq!(
        impact_set_digest(vec![direct.clone(), direct], Vec::new(), Vec::new()),
        Err(EncodingError::DuplicateSetValue)
    );
    assert_eq!(
        impact_set_digest(Vec::new(), vec![transitive.clone(), transitive], Vec::new()),
        Err(EncodingError::DuplicateSetValue)
    );
    assert_eq!(
        impact_set_digest(Vec::new(), Vec::new(), vec![ratchet.clone(), ratchet]),
        Err(EncodingError::DuplicateSetValue)
    );
}
