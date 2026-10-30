#![doc = "Integration contracts for the deterministic provenance core."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy_core::*;

fn sid(value: &str) -> StableId {
    StableId::new(value).expect("valid stable ID")
}

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn test_object_ref(kind: ObjectKind, id: StableId, value: SemanticDigest) -> ObjectRef {
    ObjectRef::new(kind, id, value)
}

fn test_atom_ref(id: StableId, revision: SemanticDigest) -> AtomRevisionRef {
    AtomRevisionRef::new(id, revision)
}

fn test_payload(schema: impl Into<String>, bytes: Vec<u8>) -> TypedPayload {
    let schema = schema.into();
    TypedPayload::new(SchemaId::new(&schema).expect("valid schema"), bytes)
        .expect("valid test typed payload")
}

fn test_tool(
    id: impl Into<String>,
    version: impl Into<String>,
    executable: SemanticDigest,
    configuration: SemanticDigest,
) -> ToolIdentity {
    let id = id
        .into()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    ToolIdentity::new(
        ScopeId::new(&id).expect("valid scope"),
        version,
        executable,
        configuration,
    )
    .expect("valid test tool identity")
}

fn test_surface_disposition(
    key: TypedPayload,
    disposition: SurfaceDispositionKind,
    payload: Option<TypedPayload>,
    authority: Option<AtomRevisionRef>,
) -> SurfaceDisposition {
    SurfaceDisposition::new(key, disposition, payload, authority)
        .expect("valid test surface disposition")
}

fn approval() -> Option<ApprovalBinding> {
    RepositoryPath::new("reviews/approval.md")
        .ok()
        .and_then(|path| {
            ApprovalBinding::new(
                ScopeId::new("maintainers").expect("valid scope"),
                path,
                digest(90),
            )
            .ok()
        })
}

fn atom(id: &str, statement: &str) -> Option<Atom> {
    let statement = if ["MUST", "SHALL", "SHOULD", "MAY"]
        .iter()
        .any(|term| statement.contains(term))
    {
        statement.to_string()
    } else {
        format!("{statement} The atom MUST remain valid.")
    };
    Atom::new(
        sid(id),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        statement,
        approval(),
    )
    .ok()
}

fn tombstone(
    id: &str,
    state: LifecycleState,
    final_revision: SemanticDigest,
    replacements: Vec<AtomRevisionRef>,
) -> Tombstone {
    Tombstone::new(
        sid(id),
        state,
        final_revision,
        replacements,
        approval().expect("canonical terminal approval"),
    )
    .expect("canonical tombstone")
}

fn decision_surface(
    id: &str,
    expected_members: Vec<TypedPayload>,
    dispositions: Vec<SurfaceDisposition>,
) -> DecisionSurface {
    let domain = DecisionSurfaceDomain::new(
        sid(id),
        test_payload("surface-key-schema/v1", vec![1]),
        test_payload("surface-disposition-schema/v1", vec![1]),
        expected_members,
        1,
    )
    .expect("canonical decision-surface domain");
    DecisionSurface::new(domain, dispositions, Vec::new()).expect("canonical decision surface")
}

fn repository_artifact(path: RepositoryPath, value: SemanticDigest) -> Option<ArtifactBinding> {
    ArtifactBinding::repository(
        path,
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        value,
    )
    .ok()
}

fn artifact_entry(path: RepositoryPath, value: SemanticDigest) -> Option<ArtifactEntry> {
    ArtifactEntry::new(
        path,
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        value,
    )
    .ok()
}

fn test_oracle(id: StableId, configuration: SemanticDigest, _references: Vec<ObjectRef>) -> Oracle {
    let runner_id = format!("{}.runner", id.as_str());
    buoy_core::Oracle::new(
        id,
        OracleKind::Test,
        test_tool(runner_id, "1", configuration, configuration),
        test_payload("oracle-selection/v1", Vec::new()),
        test_payload("verdict-normalization/v1", Vec::new()),
        configuration,
        test_payload("oracle-scope/v1", Vec::new()),
        Vec::new(),
    )
    .expect("complete test oracle")
}

fn test_implementation_link(
    id: StableId,
    parsed_item_digest: SemanticDigest,
    references: Vec<ObjectRef>,
) -> ImplementationLink {
    let mut atoms = references
        .into_iter()
        .filter(|reference| reference.kind() == ObjectKind::Atom)
        .map(|reference| test_atom_ref(reference.id().clone(), reference.digest()))
        .collect::<Vec<_>>();
    if atoms.is_empty() {
        atoms.push(test_atom_ref(sid("ATOM-IMPLEMENTATION"), digest(89)));
    }
    buoy_core::ImplementationLink::new(id, atoms, parsed_item_digest, NavigationKind::Implements)
        .expect("complete test implementation link")
}

fn test_finite_model(
    id: StableId,
    payload_digest: SemanticDigest,
    references: Vec<ObjectRef>,
) -> FiniteModel {
    let checked_invariants = references
        .iter()
        .filter(|reference| reference.kind() == ObjectKind::Atom)
        .map(|reference| test_atom_ref(reference.id().clone(), reference.digest()))
        .collect();
    let assumptions = references
        .into_iter()
        .filter(|reference| reference.kind() == ObjectKind::Assumption)
        .collect();
    buoy_core::FiniteModel::new(
        id,
        test_payload("model-state-space/v1", payload_digest.bytes().to_vec()),
        test_payload("model-events/v1", Vec::new()),
        test_payload("model-transition/v1", Vec::new()),
        vec![test_payload("model-state/v1", vec![0])],
        test_payload("model-bound/v1", vec![1]),
        checked_invariants,
        assumptions,
        Vec::new(),
    )
    .expect("complete test finite model")
}

fn test_assumption(
    id: StableId,
    scope_digest: SemanticDigest,
    _references: Vec<ObjectRef>,
) -> Assumption {
    buoy_core::Assumption::new(
        id,
        "Reviewed test assumption.",
        test_payload("assumption-scope/v1", scope_digest.bytes().to_vec()),
        Vec::new(),
        approval().expect("canonical assumption approval"),
    )
    .expect("complete test assumption")
}

#[allow(clippy::too_many_arguments)]
fn test_carrier(
    id: StableId,
    parsed_item_digest: SemanticDigest,
    references: Vec<ObjectRef>,
    role: EvidenceRole,
    atoms: Vec<AtomRevisionRef>,
    oracle: ObjectRef,
    configuration: SemanticDigest,
    corpora: Vec<SemanticDigest>,
    scope: TypedPayload,
) -> EvidenceCarrier {
    let assumptions = references
        .into_iter()
        .filter(|reference| reference.kind() == ObjectKind::Assumption)
        .collect();
    buoy_core::EvidenceCarrier::new(
        id,
        role,
        atoms,
        parsed_item_digest,
        oracle,
        configuration,
        corpora,
        scope,
        assumptions,
        Vec::new(),
    )
    .expect("complete test carrier")
}

#[allow(clippy::too_many_arguments)]
fn test_derivation(
    id: StableId,
    configuration: SemanticDigest,
    _references: Vec<ObjectRef>,
    sources: Vec<ArtifactBinding>,
    targets: Vec<ArtifactBinding>,
    tool: ToolIdentity,
    oracle: ObjectRef,
) -> Derivation {
    buoy_core::Derivation::new(
        id,
        sources,
        targets,
        tool,
        configuration,
        oracle,
        DerivationRelation::GeneratedFrom,
        test_payload("derivation-scope/v1", Vec::new()),
        Vec::new(),
    )
    .expect("complete test derivation")
}

#[allow(clippy::too_many_arguments)]
fn test_formal_property(
    id: StableId,
    _digest: SemanticDigest,
    references: Vec<ObjectRef>,
    atoms: Vec<AtomRevisionRef>,
    property: TypedPayload,
    _approved: bool,
) -> FormalProperty {
    buoy_core::FormalProperty::new(
        id,
        atoms,
        property,
        test_payload("implementation-abstraction/v1", Vec::new()),
        references,
        Vec::new(),
        approval().expect("canonical property approval"),
    )
    .expect("complete test formal property")
}

#[allow(clippy::too_many_arguments)]
fn test_proof_record(
    id: StableId,
    verifier_configuration: SemanticDigest,
    _references: Vec<ObjectRef>,
    atoms: Vec<AtomRevisionRef>,
    property: ObjectRef,
    implementation: ArtifactBinding,
    assumptions: Vec<ObjectRef>,
    translators: Vec<ToolIdentity>,
    verifier: ToolIdentity,
    external_models: Vec<ArtifactBinding>,
    generated: Vec<ArtifactBinding>,
    proof_sources: Vec<ArtifactBinding>,
    verdict: Verdict,
) -> ProofRecord {
    buoy_core::ProofRecord::new(
        id,
        atoms,
        property,
        implementation,
        assumptions,
        translators,
        verifier,
        verifier_configuration,
        external_models,
        generated,
        proof_sources,
        verdict,
        test_payload("proof-scope/v1", Vec::new()),
        Vec::new(),
    )
    .expect("complete test proof record")
}

#[allow(clippy::too_many_arguments)]
fn test_policy(
    id: StableId,
    digest: SemanticDigest,
    _references: Vec<ObjectRef>,
    revision: SemanticDigest,
    lifecycle: PolicyLifecycle,
    mode: EnforcementMode,
    ratchet: impl Into<String>,
    subjects: Vec<ObjectRef>,
    roles: Vec<EvidenceRole>,
) -> CoveragePolicy {
    let ratchet = ratchet.into();
    let revision = CoveragePolicyRevision::new(
        test_payload("policy-scope/v1", revision.bytes().to_vec()),
        roles,
        mode,
        ScopeId::new(&ratchet).expect("valid scope"),
        Vec::new(),
        None,
    )
    .expect("complete test policy revision");
    let (replacements, terminal_approval) = match lifecycle {
        PolicyLifecycle::Active => (Vec::new(), None),
        PolicyLifecycle::Superseded => (
            vec![test_object_ref(
                ObjectKind::CoveragePolicy,
                sid(&format!("{}-REPLACEMENT", id.as_str())),
                digest,
            )],
            approval(),
        ),
        PolicyLifecycle::Retired => (Vec::new(), approval()),
    };
    CoveragePolicy::new(
        id,
        revision,
        approval().expect("active policy approval"),
        lifecycle,
        replacements,
        terminal_approval,
        subjects,
    )
    .expect("complete test policy")
}

fn test_obligation(
    policy_id: StableId,
    subject: ObjectRef,
    required_role: Option<EvidenceRole>,
) -> ObligationRef {
    let kind = if required_role.is_some() {
        ObligationKind::RequiredRole
    } else {
        ObligationKind::Authority
    };
    ObligationRef::new(policy_id, kind, subject, required_role).expect("complete test obligation")
}

fn test_debt(
    id: StableId,
    _digest: SemanticDigest,
    references: Vec<ObjectRef>,
    lifecycle: DebtLifecycle,
    obligations: Vec<ObligationRef>,
) -> DebtRecord {
    let (evidence, replacements, terminal) = match lifecycle {
        DebtLifecycle::Active => (Vec::new(), Vec::new(), None),
        DebtLifecycle::Discharged => (references.clone(), Vec::new(), approval()),
        DebtLifecycle::Superseded => (
            Vec::new(),
            vec![test_object_ref(
                ObjectKind::DebtRecord,
                sid("DEBT-REPLACEMENT"),
                digest(88),
            )],
            approval(),
        ),
    };
    DebtRecord::new(
        id,
        obligations,
        references,
        "Reviewed debt rationale.",
        ScopeId::new("maintainers").expect("valid scope"),
        "Complete the missing obligation.",
        test_payload("debt-discharge/v1", vec![1]),
        Vec::new(),
        lifecycle,
        evidence,
        replacements,
        terminal,
    )
    .expect("complete test debt")
}

fn test_waiver(
    id: StableId,
    _digest: SemanticDigest,
    references: Vec<ObjectRef>,
    lifecycle: WaiverLifecycle,
    obligations: Vec<ObligationRef>,
) -> Waiver {
    let (evidence, replacements, terminal) = match lifecycle {
        WaiverLifecycle::Active => (Vec::new(), Vec::new(), None),
        WaiverLifecycle::Discharged => (references.clone(), Vec::new(), approval()),
        WaiverLifecycle::Revoked => (Vec::new(), Vec::new(), approval()),
        WaiverLifecycle::Superseded => (
            Vec::new(),
            vec![test_object_ref(
                ObjectKind::Waiver,
                sid("WAIVER-REPLACEMENT"),
                digest(87),
            )],
            approval(),
        ),
    };
    Waiver::new(
        id,
        obligations,
        references,
        "Reviewed waiver rationale.",
        ScopeId::new("maintainers").expect("valid scope"),
        test_payload("waiver-discharge/v1", vec![1]),
        approval().expect("active waiver approval"),
        Vec::new(),
        lifecycle,
        evidence,
        replacements,
        terminal,
    )
    .expect("complete test waiver")
}

fn test_reaffirmation(
    id: StableId,
    base: SemanticDigest,
    references: Vec<ObjectRef>,
) -> Reaffirmation {
    let affected = references.into_iter().next().unwrap_or_else(|| {
        let head = if base == digest(19) {
            digest(20)
        } else {
            digest(19)
        };
        test_object_ref(ObjectKind::Atom, sid("ATOM-REAFFIRMED"), head)
    });
    Reaffirmation::new(
        id,
        affected.kind(),
        affected.id().clone(),
        base,
        affected.digest(),
        approval().expect("reaffirmation approval"),
        "The changed object was individually reread.",
    )
    .expect("complete test reaffirmation")
}

#[allow(clippy::too_many_arguments)]
fn test_change_disposition(
    id: StableId,
    digest: SemanticDigest,
    _references: Vec<ObjectRef>,
    impacted_kind: ObjectKind,
    impacted_id: StableId,
    base: Option<SemanticDigest>,
    head: Option<SemanticDigest>,
    disposition: ChangeDispositionKind,
) -> ChangeDisposition {
    let reaffirmation = (disposition == ChangeDispositionKind::Reaffirmed).then_some(digest);
    let waiver = (disposition == ChangeDispositionKind::Waived)
        .then(|| test_object_ref(ObjectKind::Waiver, sid("WAIVER-DISPOSITION"), digest));
    ChangeDisposition::new(
        id,
        impacted_kind,
        impacted_id,
        base,
        head,
        disposition,
        reaffirmation,
        waiver,
        Vec::new(),
    )
    .expect("complete test change disposition")
}

fn test_change_manifest(
    id: StableId,
    _digest: SemanticDigest,
    _references: Vec<ObjectRef>,
    base: SemanticDigest,
    head: SemanticDigest,
    dispositions: Vec<ChangeDisposition>,
) -> ChangeManifest {
    ChangeManifest::new(id, base, head, dispositions, None).expect("complete test change manifest")
}

fn id_namespace(id: &str, prefix: &str, policy_byte: u8) -> IdNamespace {
    IdNamespace::new(
        ScopeId::new(id).expect("valid scope"),
        NamespacePrefix::new(prefix).expect("valid prefix"),
        test_payload("id-allocation-policy/v1", vec![policy_byte]),
        vec![
            ScopeId::new("reviewers").expect("valid scope"),
            ScopeId::new("maintainers").expect("valid scope"),
        ],
    )
    .expect("valid ID namespace")
}

fn id_allocation(
    namespace: &str,
    id: &str,
    ordinal: u64,
    class: IdAllocationClass,
    digest_byte: u8,
) -> IdAllocation {
    IdAllocation::new(
        ScopeId::new(namespace).expect("valid scope"),
        sid(id),
        ordinal,
        class,
        digest(digest_byte),
    )
    .expect("valid ID allocation")
}

#[test]
fn complete_task_3_5_values_are_canonical_and_derive_exact_references() {
    let selector_a = TypedSelector::new(
        RepositoryPath::new("spec/a").expect("selector root"),
        SchemaId::new("markdown/v1").expect("valid schema"),
        vec![String::from("**/*.md")],
        Vec::new(),
        AuthorityExpectation::Required,
    )
    .expect("complete selector");
    let selector_b = TypedSelector::new(
        RepositoryPath::new("spec/b").expect("selector root"),
        SchemaId::new("rust/v1").expect("valid schema"),
        vec![String::from("**/*.rs")],
        vec![String::from("generated/**")],
        AuthorityExpectation::Optional,
    )
    .expect("complete selector");
    let make_revision = |selectors: Vec<TypedSelector>, roles: Vec<EvidenceRole>| {
        CoveragePolicyRevision::new(
            test_payload("policy-scope/v1", vec![1]),
            roles,
            EnforcementMode::Blocking,
            ScopeId::new("ratchet-main").expect("valid scope"),
            selectors,
            Some(test_payload("policy-derivation/v1", vec![2])),
        )
        .expect("complete policy revision")
    };
    let revision_a = make_revision(
        vec![selector_b.clone(), selector_a.clone()],
        vec![EvidenceRole::Negative, EvidenceRole::Positive],
    );
    let revision_b = make_revision(
        vec![selector_a, selector_b],
        vec![EvidenceRole::Positive, EvidenceRole::Negative],
    );
    assert_eq!(revision_a, revision_b);
    assert_eq!(revision_a.authority_selectors().len(), 2);
    assert_eq!(revision_a.required_roles().len(), 2);
    assert_eq!(revision_a.ratchet_id(), "ratchet-main");
    assert!(revision_a.derivation_requirements().is_some());

    // Canonical text order is length-prefixed, so `Z-1` sorts before `AA-1`
    // even though ordinary string ordering would put `AA-1` first.
    let subject_a = test_object_ref(ObjectKind::Atom, sid("Z-1"), digest(101));
    let subject_b = test_object_ref(ObjectKind::Atom, sid("AA-1"), digest(102));
    let policy = CoveragePolicy::new(
        sid("POLICY-COMPLETE"),
        revision_a,
        approval().expect("active policy approval"),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![subject_b.clone(), subject_a.clone()],
    )
    .expect("complete policy");
    assert_eq!(policy.mode(), EnforcementMode::Blocking);
    assert_eq!(policy.ratchet(), "ratchet-main");
    assert_eq!(policy.subjects(), &[subject_a.clone(), subject_b.clone()]);
    assert_eq!(policy.meta().references(), policy.subjects());

    let authority = ObligationRef::new(
        policy.meta().id().clone(),
        ObligationKind::Authority,
        subject_a.clone(),
        None,
    )
    .expect("authority obligation");
    let required_role = ObligationRef::new(
        policy.meta().id().clone(),
        ObligationKind::RequiredRole,
        subject_b.clone(),
        Some(EvidenceRole::Positive),
    )
    .expect("role obligation");
    assert_ne!(authority.digest(), required_role.digest());

    let debt = DebtRecord::new(
        sid("DEBT-COMPLETE"),
        vec![required_role.clone(), authority.clone()],
        vec![subject_a.clone()],
        "The obligation remains visible.",
        ScopeId::new("maintainers").expect("valid scope"),
        "Provide current evidence.",
        test_payload("debt-discharge/v1", vec![1]),
        vec![String::from("ticket:1")],
        DebtLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("complete debt");
    assert_eq!(
        debt.obligations(),
        &[authority.clone(), required_role.clone()]
    );
    assert_eq!(debt.objects(), core::slice::from_ref(&subject_a));
    assert_eq!(debt.meta().references().len(), 2);

    let waiver = Waiver::new(
        sid("WAIVER-COMPLETE"),
        vec![required_role.clone(), authority],
        vec![subject_b.clone()],
        "The process exception remains explicit.",
        ScopeId::new("maintainers").expect("valid scope"),
        test_payload("waiver-discharge/v1", vec![1]),
        approval().expect("active waiver approval"),
        vec![String::from("ticket:2")],
        WaiverLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("complete waiver");
    assert_eq!(waiver.obligations().len(), 2);
    assert_eq!(waiver.objects(), core::slice::from_ref(&subject_b));
    assert!(waiver.terminal_approval().is_none());

    let reaffirmation = Reaffirmation::new(
        sid("REAFFIRM-COMPLETE"),
        ObjectKind::Atom,
        sid("ATOM-POLICY-A"),
        digest(103),
        digest(104),
        approval().expect("reaffirmation approval"),
        "The changed authority was reread.",
    )
    .expect("complete reaffirmation");
    assert_eq!(reaffirmation.affected_object_kind(), ObjectKind::Atom);
    assert_eq!(reaffirmation.meta().references().len(), 1);

    let revised = ChangeDisposition::new(
        sid("DISPOSITION-REVISED"),
        ObjectKind::Atom,
        sid("ATOM-POLICY-A"),
        Some(digest(103)),
        Some(digest(104)),
        ChangeDispositionKind::Revised,
        None,
        None,
        Vec::new(),
    )
    .expect("complete revised disposition");
    let waiver_ref = test_object_ref(
        ObjectKind::Waiver,
        waiver.meta().id().clone(),
        waiver.meta().digest(),
    );
    let waived_disposition = ChangeDisposition::new(
        sid("DISPOSITION-WAIVED"),
        ObjectKind::Atom,
        sid("ATOM-POLICY-B"),
        Some(digest(105)),
        None,
        ChangeDispositionKind::Waived,
        None,
        Some(waiver_ref.clone()),
        Vec::new(),
    )
    .expect("complete waived disposition");
    let manifest_a = ChangeManifest::new(
        sid("CHANGE-COMPLETE"),
        digest(106),
        digest(107),
        vec![waived_disposition.clone(), revised.clone()],
        Some(approval().expect("change approval")),
    )
    .expect("complete manifest");
    let manifest_b = ChangeManifest::new(
        sid("CHANGE-COMPLETE"),
        digest(106),
        digest(107),
        vec![revised, waived_disposition],
        Some(approval().expect("change approval")),
    )
    .expect("complete reordered manifest");
    assert_eq!(manifest_a, manifest_b);
    assert_eq!(manifest_a.dispositions().len(), 2);
    assert!(manifest_a.approval().is_some());
    assert_eq!(manifest_a.meta().references().len(), 2);
}

#[test]
fn complete_task_3_5_values_reject_invalid_terminal_and_variant_shapes() {
    let selector_root = RepositoryPath::new("spec").expect("selector root");
    assert_eq!(
        TypedSelector::new(
            selector_root.clone(),
            SchemaId::new("markdown/v1").expect("valid schema"),
            Vec::new(),
            Vec::new(),
            AuthorityExpectation::Required,
        ),
        Err(ConstructionError::EmptySet)
    );
    assert_eq!(
        TypedSelector::new(
            selector_root,
            SchemaId::new("markdown/v1").expect("valid schema"),
            vec![String::from("**/*.md"), String::from("**/*.md")],
            Vec::new(),
            AuthorityExpectation::Required,
        ),
        Err(ConstructionError::Encoding(
            EncodingError::DuplicateSetValue
        ))
    );

    let revision = || {
        CoveragePolicyRevision::new(
            test_payload("policy-scope/v1", vec![1]),
            vec![EvidenceRole::Positive],
            EnforcementMode::Blocking,
            ScopeId::new("ratchet-main").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("complete policy revision")
    };
    assert_eq!(
        CoveragePolicy::new(
            sid("POLICY-INVALID"),
            revision(),
            approval().expect("active approval"),
            PolicyLifecycle::Active,
            Vec::new(),
            approval(),
            Vec::new(),
        ),
        Err(ConstructionError::LifecycleShape)
    );
    assert_eq!(
        CoveragePolicy::new(
            sid("POLICY-INVALID"),
            revision(),
            approval().expect("active approval"),
            PolicyLifecycle::Superseded,
            vec![test_object_ref(
                ObjectKind::Atom,
                sid("ATOM-WRONG"),
                digest(108)
            )],
            approval(),
            Vec::new(),
        ),
        Err(ConstructionError::ReferenceKind)
    );

    let subject = test_object_ref(ObjectKind::Atom, sid("ATOM-INVALID"), digest(109));
    assert_eq!(
        ObligationRef::new(
            sid("POLICY-INVALID"),
            ObligationKind::RequiredRole,
            subject.clone(),
            None,
        ),
        Err(ConstructionError::ObligationRoleShape)
    );
    let obligation = ObligationRef::new(
        sid("POLICY-INVALID"),
        ObligationKind::Authority,
        subject,
        None,
    )
    .expect("complete obligation");
    assert_eq!(
        DebtRecord::new(
            sid("DEBT-INVALID"),
            vec![obligation.clone()],
            Vec::new(),
            "rationale",
            ScopeId::new("owner").expect("valid scope"),
            "remediation",
            test_payload("debt-discharge/v1", vec![1]),
            Vec::new(),
            DebtLifecycle::Discharged,
            Vec::new(),
            Vec::new(),
            approval(),
        ),
        Err(ConstructionError::LifecycleShape)
    );
    assert_eq!(
        Waiver::new(
            sid("WAIVER-INVALID"),
            vec![obligation],
            Vec::new(),
            "rationale",
            ScopeId::new("owner").expect("valid scope"),
            test_payload("waiver-discharge/v1", vec![1]),
            approval().expect("active waiver approval"),
            Vec::new(),
            WaiverLifecycle::Active,
            Vec::new(),
            Vec::new(),
            approval(),
        ),
        Err(ConstructionError::LifecycleShape)
    );
    assert_eq!(
        Reaffirmation::new(
            sid("REAFFIRM-INVALID"),
            ObjectKind::Atom,
            sid("ATOM-INVALID"),
            digest(110),
            digest(110),
            approval().expect("review approval"),
            "reread",
        ),
        Err(ConstructionError::EqualDigests)
    );
    assert_eq!(
        ChangeDisposition::new(
            sid("DISPOSITION-INVALID"),
            ObjectKind::Atom,
            sid("ATOM-INVALID"),
            Some(digest(111)),
            None,
            ChangeDispositionKind::Revised,
            None,
            None,
            Vec::new(),
        ),
        Err(ConstructionError::ChangeDispositionShape)
    );
    assert_eq!(
        StableId::new("").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-STABLE-ID")
    );
}

#[test]
fn complete_task_3_4_values_derive_references_and_ignore_set_input_order() {
    let oracle = test_oracle(sid("ORACLE-ORDER"), digest(70), Vec::new());
    let oracle_ref = test_object_ref(
        ObjectKind::Oracle,
        oracle.meta().id().clone(),
        oracle.meta().digest(),
    );
    let atom_a = test_atom_ref(sid("ATOM-ORDER-A"), digest(71));
    let atom_b = test_atom_ref(sid("ATOM-ORDER-B"), digest(72));
    let assumption_a = test_object_ref(ObjectKind::Assumption, sid("ASSUMPTION-A"), digest(73));
    let assumption_b = test_object_ref(ObjectKind::Assumption, sid("ASSUMPTION-B"), digest(74));
    let construct = |atoms: Vec<AtomRevisionRef>,
                     corpora: Vec<SemanticDigest>,
                     assumptions: Vec<ObjectRef>,
                     exclusions: Vec<String>| {
        EvidenceCarrier::new(
            sid("CARRIER-ORDER"),
            EvidenceRole::Property,
            atoms,
            digest(75),
            oracle_ref.clone(),
            digest(76),
            corpora,
            test_payload("scope.quantified/v1", vec![1]),
            assumptions,
            exclusions,
        )
        .expect("complete order-independent carrier")
    };
    let first = construct(
        vec![atom_b.clone(), atom_a.clone()],
        vec![digest(78), digest(77)],
        vec![assumption_b.clone(), assumption_a.clone()],
        vec![String::from("z-exclusion"), String::from("a-exclusion")],
    );
    let second = construct(
        vec![atom_a, atom_b],
        vec![digest(77), digest(78)],
        vec![assumption_a.clone(), assumption_b.clone()],
        vec![String::from("a-exclusion"), String::from("z-exclusion")],
    );
    assert_eq!(first, second);
    assert_eq!(first.meta().digest(), second.meta().digest());
    assert_eq!(first.meta().references().len(), 3);
    for reference in [&oracle_ref, &assumption_a, &assumption_b] {
        assert!(first.meta().references().contains(reference));
    }

    let verdict_first = NormalizedVerdict::new(
        oracle_ref.clone(),
        vec![assumption_b.clone(), assumption_a.clone()],
        Verdict::Pass,
        None,
    )
    .expect("first normalized verdict");
    let verdict_second = NormalizedVerdict::new(
        oracle_ref,
        vec![assumption_a, assumption_b],
        Verdict::Pass,
        None,
    )
    .expect("second normalized verdict");
    assert_eq!(verdict_first, verdict_second);
    assert_eq!(verdict_first.digest(), verdict_second.digest());
}

#[test]
fn id_registry_constructs_canonical_permanent_reservations() {
    let namespace = id_namespace("spec.graph", "SPG", 1);
    assert_eq!(namespace.namespace_id().as_str(), "spec.graph");
    assert_eq!(namespace.prefix().as_str(), "SPG");
    assert_eq!(
        namespace.approval_authorities(),
        &[String::from("reviewers"), String::from("maintainers")]
    );
    assert_eq!(namespace.allocation_policy().bytes(), &[1]);

    let first = id_allocation("spec.graph", "SPG-001", 1, IdAllocationClass::Live, 2);
    let second = id_allocation("spec.graph", "SPG-002", 2, IdAllocationClass::Live, 3);
    assert_eq!(first.namespace_id().as_str(), "spec.graph");
    assert_eq!(first.stable_id().as_str(), "SPG-001");
    assert_eq!(first.ordinal(), 1);
    assert_eq!(first.class(), IdAllocationClass::Live);
    assert_eq!(first.allocation_digest(), digest(2));

    let base =
        IdRegistry::new(vec![namespace.clone()], vec![first.clone()]).expect("valid base registry");
    let appended = base
        .with_allocation(second.clone())
        .expect("append-only allocation");
    assert_eq!(appended.namespaces(), core::slice::from_ref(&namespace));
    assert_eq!(appended.allocations(), &[first.clone(), second.clone()]);
    assert_eq!(
        appended.reservation(&sid("SPG-001")),
        Some(IdAllocationClass::Live)
    );
    assert_eq!(appended.reservation(&sid("SPG-MISSING")), None);
    assert_eq!(base.validate_successor(&appended), Ok(()));

    let permuted = IdRegistry::new(vec![namespace.clone()], vec![second, first.clone()])
        .expect("input order is nonsemantic");
    assert_eq!(appended, permuted);
    assert_eq!(appended.digest(), permuted.digest());
    assert!(appended.digest().is_ok());

    let terminal = IdRegistry::new(
        vec![namespace],
        vec![
            id_allocation("spec.graph", "SPG-001", 1, IdAllocationClass::Terminal, 2),
            id_allocation("spec.graph", "SPG-002", 2, IdAllocationClass::Live, 3),
        ],
    )
    .expect("live allocation may become terminal");
    assert_eq!(appended.validate_successor(&terminal), Ok(()));
    assert_eq!(
        terminal.reservation(&sid("SPG-001")),
        Some(IdAllocationClass::Terminal)
    );
}

#[test]
fn id_registry_rejects_invalid_allocations_and_history_rewrites() {
    assert_eq!(
        IdAllocation::new(
            ScopeId::new("spec.graph").expect("valid scope"),
            sid("SPG-001"),
            0,
            IdAllocationClass::Live,
            digest(1),
        )
        .expect_err("ordinal zero must reject")
        .code(),
        "SCF-REGISTRY-ORDINAL"
    );
    assert_eq!(
        IdNamespace::new(
            ScopeId::new("spec.graph").expect("valid scope"),
            NamespacePrefix::new("SPG").expect("valid prefix"),
            test_payload("id-allocation-policy/v1", vec![1]),
            vec![
                ScopeId::new("maintainers").expect("valid scope"),
                ScopeId::new("maintainers").expect("valid scope"),
            ],
        )
        .expect_err("duplicate authorities must reject")
        .code(),
        "SCF-REGISTRY-DUPLICATE"
    );

    let namespace = id_namespace("spec.graph", "SPG", 1);
    let live = id_allocation("spec.graph", "SPG-001", 1, IdAllocationClass::Live, 2);
    for (allocation, expected) in [
        (
            id_allocation("unknown", "SPG-002", 2, IdAllocationClass::Live, 3),
            "SCF-REGISTRY-NAMESPACE",
        ),
        (
            id_allocation("spec.graph", "DET-002", 2, IdAllocationClass::Live, 3),
            "SCF-REGISTRY-PREFIX",
        ),
    ] {
        assert_eq!(
            IdRegistry::new(vec![namespace.clone()], vec![allocation])
                .expect_err("invalid allocation must reject")
                .code(),
            expected
        );
    }
    for duplicate in [
        id_allocation("spec.graph", "SPG-001", 2, IdAllocationClass::Live, 3),
        id_allocation("spec.graph", "SPG-002", 1, IdAllocationClass::Live, 3),
    ] {
        assert_eq!(
            IdRegistry::new(vec![namespace.clone()], vec![live.clone(), duplicate],)
                .expect_err("duplicate allocation identity must reject")
                .code(),
            "SCF-REGISTRY-DUPLICATE"
        );
    }

    let base =
        IdRegistry::new(vec![namespace.clone()], vec![live.clone()]).expect("valid live registry");
    let removed = IdRegistry::new(vec![namespace.clone()], Vec::new())
        .expect("empty current registry is locally valid");
    assert_eq!(
        base.validate_successor(&removed)
            .expect_err("allocation removal must reject")
            .code(),
        "SCF-REGISTRY-HISTORY"
    );
    let changed_record = IdRegistry::new(
        vec![namespace.clone()],
        vec![id_allocation(
            "spec.graph",
            "SPG-001",
            1,
            IdAllocationClass::Live,
            4,
        )],
    )
    .expect("changed allocation is locally valid");
    let changed_ordinal = IdRegistry::new(
        vec![namespace.clone()],
        vec![id_allocation(
            "spec.graph",
            "SPG-001",
            2,
            IdAllocationClass::Live,
            2,
        )],
    )
    .expect("changed ordinal is locally valid");
    let reassigned_namespace = IdRegistry::new(
        vec![id_namespace("spec.other", "SPG", 1)],
        vec![id_allocation(
            "spec.other",
            "SPG-001",
            1,
            IdAllocationClass::Live,
            2,
        )],
    )
    .expect("reassigned namespace is locally valid");
    for rewrite in [&changed_record, &changed_ordinal, &reassigned_namespace] {
        assert_eq!(
            base.validate_successor(rewrite)
                .expect_err("allocation history rewrite must reject")
                .code(),
            "SCF-REGISTRY-HISTORY"
        );
    }

    let terminal = IdRegistry::new(
        vec![namespace.clone()],
        vec![id_allocation(
            "spec.graph",
            "SPG-001",
            1,
            IdAllocationClass::Terminal,
            2,
        )],
    )
    .expect("valid terminal reservation");
    let reused =
        IdRegistry::new(vec![namespace], vec![live]).expect("locally valid live allocation");
    assert_eq!(
        terminal
            .validate_successor(&reused)
            .expect_err("terminal-to-live reuse must reject")
            .code(),
        "SCF-REGISTRY-REUSE"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn schema_value_surface_is_constructible() {
    let atom = atom("SPG-001", "The graph MUST close references.");
    assert!(atom.is_some());
    let Some(atom) = atom else { return };
    let atom_ref = test_atom_ref(atom.meta().id().clone(), atom.revision());
    let oracle = test_oracle(sid("ORC-001"), digest(2), Vec::new());
    let oracle_ref = test_object_ref(
        ObjectKind::Oracle,
        oracle.meta().id().clone(),
        oracle.meta().digest(),
    );
    let assumption = test_assumption(sid("ASM-001"), digest(16), Vec::new());
    let assumption_ref = test_object_ref(
        ObjectKind::Assumption,
        assumption.meta().id().clone(),
        assumption.meta().digest(),
    );
    let carrier = test_carrier(
        sid("CAR-001"),
        digest(3),
        vec![assumption_ref.clone()],
        EvidenceRole::Positive,
        vec![atom_ref.clone()],
        oracle_ref.clone(),
        digest(4),
        vec![digest(5)],
        test_payload("scope.exact/v1", vec![1]),
    );
    let implementation = test_implementation_link(
        sid("IMPL-001"),
        digest(6),
        vec![test_object_ref(
            ObjectKind::Atom,
            atom.meta().id().clone(),
            atom.revision(),
        )],
    );
    let path = RepositoryPath::new("generated/table.bin").ok();
    assert!(path.is_some());
    let Some(path) = path else { return };
    let tool = test_tool("generator", "1", digest(7), digest(8));
    let Some(generated_binding) = repository_artifact(path, digest(10)) else {
        return;
    };
    let derivation = test_derivation(
        sid("DERIV-001"),
        digest(9),
        Vec::new(),
        vec![ArtifactBinding::Object(test_object_ref(
            ObjectKind::EvidenceCarrier,
            carrier.meta().id().clone(),
            carrier.meta().digest(),
        ))],
        vec![generated_binding.clone()],
        tool.clone(),
        oracle_ref.clone(),
    );
    let subject = test_object_ref(
        ObjectKind::Atom,
        atom.meta().id().clone(),
        atom.meta().digest(),
    );
    let policy = test_policy(
        sid("POL-001"),
        digest(11),
        Vec::new(),
        digest(12),
        PolicyLifecycle::Active,
        EnforcementMode::Blocking,
        ScopeId::new("adoption-line").expect("valid scope"),
        vec![subject.clone()],
        vec![EvidenceRole::Positive],
    );
    let obligation = test_obligation(
        policy.meta().id().clone(),
        subject,
        Some(EvidenceRole::Positive),
    );
    let debt = test_debt(
        sid("DEBT-001"),
        digest(13),
        Vec::new(),
        DebtLifecycle::Active,
        vec![obligation.clone()],
    );
    let waiver = test_waiver(
        sid("WAIVER-001"),
        digest(14),
        Vec::new(),
        WaiverLifecycle::Active,
        vec![obligation],
    );
    let model = test_finite_model(
        sid("MODEL-001"),
        digest(15),
        vec![
            test_object_ref(ObjectKind::Atom, atom.meta().id().clone(), atom.revision()),
            assumption_ref.clone(),
        ],
    );
    let property = test_formal_property(
        sid("PROP-001"),
        digest(17),
        vec![assumption_ref.clone()],
        vec![atom_ref.clone()],
        test_payload("property/v1", vec![2]),
        true,
    );
    let proof = test_proof_record(
        sid("PROOF-001"),
        digest(18),
        Vec::new(),
        vec![atom_ref.clone()],
        test_object_ref(
            ObjectKind::FormalProperty,
            property.meta().id().clone(),
            property.meta().digest(),
        ),
        ArtifactBinding::Object(test_object_ref(
            ObjectKind::ImplementationLink,
            implementation.meta().id().clone(),
            implementation.meta().digest(),
        )),
        vec![assumption_ref],
        vec![tool.clone()],
        tool,
        Vec::new(),
        Vec::new(),
        vec![ArtifactBinding::Object(test_object_ref(
            ObjectKind::ImplementationLink,
            implementation.meta().id().clone(),
            implementation.meta().digest(),
        ))],
        Verdict::Pass,
    );
    let reaffirmation = test_reaffirmation(
        sid("REAFFIRM-001"),
        digest(19),
        vec![test_object_ref(
            ObjectKind::Atom,
            atom.meta().id().clone(),
            atom.meta().digest(),
        )],
    );
    let surface_key = test_payload("surface-key/v1", vec![1]);
    let surface = decision_surface(
        "SURFACE-VALID",
        vec![surface_key.clone()],
        vec![test_surface_disposition(
            surface_key,
            SurfaceDispositionKind::Normative,
            Some(test_payload("surface-value/v1", vec![1])),
            Some(atom_ref.clone()),
        )],
    );

    assert_eq!(oracle.oracle_kind(), OracleKind::Test);
    assert_eq!(oracle.runner().id(), "orc-001.runner");
    assert_eq!(oracle.configuration(), digest(2));
    assert!(oracle.exclusions().is_empty());
    assert_eq!(carrier.role(), EvidenceRole::Positive);
    assert_eq!(carrier.parsed_item_digest(), digest(3));
    assert_eq!(carrier.atoms(), core::slice::from_ref(&atom_ref));
    assert_eq!(carrier.corpora(), &[digest(5)]);
    assert_eq!(carrier.assumptions().len(), 1);
    assert!(carrier.exclusions().is_empty());
    assert_eq!(carrier.meta().references().len(), 2);
    assert!(carrier.meta().references().contains(carrier.oracle()));
    assert_eq!(implementation.navigation(), NavigationKind::Implements);
    assert_eq!(implementation.parsed_item_digest(), digest(6));
    assert_eq!(
        generated_binding.kind(),
        ArtifactBindingKind::RepositoryArtifact
    );
    assert_eq!(generated_binding.artifact_digest(), digest(10));
    assert!(generated_binding.binding_digest().is_ok());
    assert_eq!(derivation.configuration(), digest(9));
    assert_eq!(derivation.relation(), DerivationRelation::GeneratedFrom);
    assert_eq!(derivation.relation_scope().schema(), "derivation-scope/v1");
    assert!(derivation.exclusions().is_empty());
    assert_eq!(derivation.meta().references().len(), 2);
    assert!(derivation.meta().references().contains(derivation.oracle()));
    assert_eq!(model.initial_states().len(), 1);
    assert_eq!(model.checked_invariants(), core::slice::from_ref(&atom_ref));
    assert_eq!(model.assumptions().len(), 1);
    assert_eq!(assumption.statement(), "Reviewed test assumption.");
    assert_eq!(assumption.approval().authority(), "maintainers");
    assert_eq!(property.atoms(), core::slice::from_ref(&atom_ref));
    assert_eq!(property.property().schema(), "property/v1");
    assert_eq!(
        property.implementation_abstraction().schema(),
        "implementation-abstraction/v1"
    );
    assert_eq!(property.assumptions().len(), 1);
    assert!(property.exclusions().is_empty());
    assert!(property.approved());
    assert_eq!(proof.verifier_configuration(), digest(18));
    assert_eq!(proof.scope().schema(), "proof-scope/v1");
    assert!(proof.exclusions().is_empty());
    assert_eq!(proof.meta().references().len(), 3);
    assert!(proof.meta().references().contains(proof.property()));
    let normalized = NormalizedVerdict::new(
        oracle_ref,
        vec![test_object_ref(
            ObjectKind::EvidenceCarrier,
            carrier.meta().id().clone(),
            carrier.meta().digest(),
        )],
        Verdict::Pass,
        Some(test_payload("semantic-result/v1", vec![1])),
    )
    .expect("complete normalized verdict");
    assert_eq!(normalized.verdict(), Verdict::Pass);
    assert_eq!(normalized.selected_subjects().len(), 1);
    assert_eq!(
        normalized.semantic_result().map(TypedPayload::bytes),
        Some(&[1][..])
    );
    assert_ne!(normalized.digest(), digest(0));

    let graph = SpecGraph {
        atoms: vec![atom],
        evidence_carriers: vec![carrier],
        implementation_links: vec![implementation],
        derivations: vec![derivation],
        coverage_policies: vec![policy],
        debt_records: vec![debt],
        waivers: vec![waiver],
        oracles: vec![oracle],
        finite_models: vec![model],
        formal_properties: vec![property],
        assumptions: vec![assumption],
        proof_records: vec![proof],
        reaffirmations: vec![reaffirmation],
        decision_surfaces: vec![surface],
        ..SpecGraph::default()
    };
    assert_eq!(graph.object_meta().len(), 14);
    assert_eq!(
        graph
            .object_meta()
            .into_iter()
            .map(ObjectMeta::kind)
            .collect::<Vec<_>>(),
        vec![
            ObjectKind::Atom,
            ObjectKind::DecisionSurface,
            ObjectKind::EvidenceCarrier,
            ObjectKind::ImplementationLink,
            ObjectKind::Derivation,
            ObjectKind::CoveragePolicy,
            ObjectKind::DebtRecord,
            ObjectKind::Waiver,
            ObjectKind::Oracle,
            ObjectKind::FiniteModel,
            ObjectKind::FormalProperty,
            ObjectKind::Assumption,
            ObjectKind::ProofRecord,
            ObjectKind::Reaffirmation,
        ]
    );
    assert_eq!(graph.atoms[0].atom_kind(), AtomKind::Constraint);
    let approval_binding = graph.atoms[0].approval().expect("ratified atom approval");
    assert_eq!(approval_binding.authority(), "maintainers");
    assert_eq!(approval_binding.record().as_str(), "reviews/approval.md");
    assert_eq!(approval_binding.digest(), digest(90));
    assert_eq!(graph.atoms[0].scopes(), &[String::from("spec.graph")]);
    assert_eq!(
        graph.atoms[0].statement(),
        "The graph MUST close references."
    );
    assert_eq!(graph.evidence_carriers[0].configuration(), digest(4));
    assert_eq!(graph.evidence_carriers[0].corpora(), &[digest(5)]);
    assert_eq!(
        graph.evidence_carriers[0].scope().schema(),
        "scope.exact/v1"
    );
    assert_eq!(
        graph.derivations[0].tool(),
        &test_tool("generator", "1", digest(7), digest(8))
    );
    assert_ne!(graph.coverage_policies[0].revision(), digest(0));
    assert_eq!(
        graph.coverage_policies[0].revision_value().scope().bytes(),
        digest(12).bytes()
    );
    assert_eq!(
        graph.coverage_policies[0].lifecycle(),
        PolicyLifecycle::Active
    );
    assert_eq!(graph.debt_records[0].lifecycle(), DebtLifecycle::Active);
    assert_eq!(graph.debt_records[0].obligations().len(), 1);
    assert_eq!(graph.debt_records[0].owner(), "maintainers");
    assert_eq!(graph.waivers[0].lifecycle(), WaiverLifecycle::Active);
    assert_eq!(graph.waivers[0].obligations().len(), 1);
    assert_eq!(graph.waivers[0].owner(), "maintainers");
    assert_eq!(
        graph.reaffirmations[0].affected_object_id().as_str(),
        "SPG-001"
    );
    assert_eq!(graph.formal_properties[0].atoms().len(), 1);
    assert_eq!(
        graph.formal_properties[0].property().schema(),
        "property/v1"
    );
    assert_eq!(
        graph.decision_surfaces[0].domain().surface_id().as_str(),
        "SURFACE-VALID"
    );
    assert_eq!(graph.proof_records[0].translators().len(), 1);
    assert_eq!(
        graph.proof_records[0].verifier(),
        &test_tool("generator", "1", digest(7), digest(8))
    );
    assert_eq!(
        validate_graph(&graph)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        vec![String::from("SPG-DERIVATION-STALE")]
    );
    assert!(semantic_graph_digest(&graph).is_ok());
    let stale_proof_graph = SpecGraph {
        proof_records: vec![graph.proof_records[0].clone()],
        ..SpecGraph::default()
    };
    assert_eq!(
        validate_graph(&stale_proof_graph)[0].code,
        "FPR-BIND-STALE-OR-INCOMPLETE"
    );

    let mut changed = graph.clone();
    changed.atoms[0] = Atom::new(
        sid("SPG-001"),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        "The changed graph MUST still close references.",
        approval(),
    )
    .expect("changed atom remains canonical");
    let impacted = change_impacts(&graph, &changed);
    for expected in [
        "CAR-001",
        "SURFACE-VALID",
        "MODEL-001",
        "PROP-001",
        "PROOF-001",
    ] {
        assert!(
            impacted.iter().any(|impact| impact.id == sid(expected)),
            "missing transitive impact {expected}"
        );
    }

    let mut changed_assumption = graph.clone();
    changed_assumption.assumptions[0] = test_assumption(sid("ASM-001"), digest(97), Vec::new());
    let assumption_impacts = change_impacts(&graph, &changed_assumption);
    for expected in ["CAR-001", "MODEL-001", "PROP-001", "PROOF-001"] {
        assert!(
            assumption_impacts
                .iter()
                .any(|impact| impact.id == sid(expected) && !impact.direct),
            "missing assumption-dependent impact {expected}"
        );
    }

    let mut changed_policy = graph.clone();
    changed_policy.coverage_policies[0] = test_policy(
        sid("POL-001"),
        digest(11),
        Vec::new(),
        digest(98),
        PolicyLifecycle::Active,
        EnforcementMode::Blocking,
        ScopeId::new("adoption-line").expect("valid scope"),
        graph.coverage_policies[0].subjects().to_vec(),
        graph.coverage_policies[0].roles().to_vec(),
    );
    let policy_impacts = change_impacts(&graph, &changed_policy);
    for expected in ["DEBT-001", "WAIVER-001"] {
        assert!(
            policy_impacts
                .iter()
                .any(|impact| impact.id == sid(expected) && !impact.direct),
            "missing obligation-dependent impact {expected}"
        );
    }
}

#[test]
fn newly_registered_nested_values_match_frozen_field_tables() {
    let environment = EnvironmentBinding::new("CONTROL_1", "exact").expect("environment");
    assert_eq!(
        environment.canonical_value(),
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Text(String::from("CONTROL_1"))),
            (2, CanonicalValue::Text(String::from("exact"))),
        ])
    );
    assert_eq!(
        EnvironmentBinding::new("lower", "exact"),
        Err(ConstructionError::LocalInvariant)
    );

    let rejected_path = RepositoryPath::new("spec/link.md").expect("rejected path");
    let rejected =
        RejectedEntryObservation::new(rejected_path.clone(), EntryKind::Symlink, "DET-PATH-LINK")
            .expect("rejected observation");
    assert_eq!(
        rejected.canonical_value(),
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Text(rejected_path.as_str().to_string())),
            (2, CanonicalValue::Enum(EntryKind::Symlink as u16, None)),
            (3, CanonicalValue::Text(String::from("DET-PATH-LINK"))),
        ])
    );

    let selector = TypedSelector::new(
        RepositoryPath::new("reviews").expect("selector root"),
        SchemaId::new("markdown/v1").expect("valid schema"),
        vec![String::from("**/*.md")],
        Vec::new(),
        AuthorityExpectation::Required,
    )
    .expect("selector");
    let action = test_payload("approval.action/v1", vec![1]);
    let declaration_approval = approval().expect("approval");
    let authority = ApprovalAuthority::new(
        ScopeId::new("maintainers").expect("valid scope"),
        vec![action.clone()],
        vec![ScopeId::new("spg").expect("valid scope")],
        selector,
        None,
        declaration_approval.clone(),
    )
    .expect("authority");
    assert_eq!(authority.authority_id(), "maintainers");
    assert_eq!(
        authority.allowed_kind_actions(),
        core::slice::from_ref(&action)
    );
    assert_eq!(authority.allowed_namespaces(), &[String::from("spg")]);
    assert!(authority.separation_of_duty().is_none());
    assert_eq!(authority.declaration_approval(), &declaration_approval);

    let tool_value = |tool: &ToolIdentity| {
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Text(tool.id().to_string())),
            (2, CanonicalValue::Text(tool.version().to_string())),
            (3, CanonicalValue::Digest(tool.executable())),
            (4, CanonicalValue::Digest(tool.configuration())),
        ])
    };
    let unicode = test_tool("unicode", "16", digest(51), digest(52));
    let commonmark = test_tool("commonmark", "1", digest(53), digest(54));
    let toml = test_tool("toml", "1", digest(55), digest(56));
    let rust = test_tool("rust", "1", digest(57), digest(58));
    let lock = SchemaToolLock::new(
        SchemaId::new("spec-provenance/v2").expect("valid schema"),
        unicode.clone(),
        commonmark.clone(),
        toml.clone(),
        Some(rust.clone()),
        Vec::new(),
    )
    .expect("schema tool lock");
    assert_eq!(
        lock.digest(),
        semantic_digest(
            "schema-tool-lock",
            vec![
                (1, CanonicalValue::Text(String::from("spec-provenance/v2"))),
                (2, tool_value(&unicode)),
                (3, tool_value(&commonmark)),
                (4, tool_value(&toml)),
                (5, CanonicalValue::Some(Box::new(tool_value(&rust)))),
                (6, CanonicalValue::Set(Vec::new())),
            ],
        )
        .expect("schema lock digest")
    );

    let semantic_config = SemanticRepositoryConfig::new(
        SchemaId::new("spec-provenance.config/v2").expect("valid schema"),
        vec![authority.review_record_selector().clone()],
        vec![id_namespace("spg", "SPG", 59)],
        vec![authority],
        vec![test_object_ref(
            ObjectKind::CoveragePolicy,
            sid("POL-CONFIG"),
            digest(60),
        )],
        vec![unicode],
    )
    .expect("semantic repository config");
    assert_eq!(
        semantic_config.configuration_schema_id(),
        "spec-provenance.config/v2"
    );
    assert_eq!(semantic_config.selectors().len(), 1);
    assert_eq!(semantic_config.id_namespaces().len(), 1);
    assert_eq!(semantic_config.approval_authorities().len(), 1);
    assert_eq!(semantic_config.policy_references().len(), 1);
    assert_eq!(semantic_config.semantic_tools().len(), 1);
    assert_ne!(semantic_config.digest(), digest(0));
}

#[test]
fn graph_and_artifact_index_encoders_match_frozen_field_tables() {
    let Some(atom) = atom("SPG-ENCODER", "The graph MUST encode every object domain.") else {
        return;
    };
    let approval = atom.approval().expect("ratified atom approval");
    let atom_value = CanonicalValue::Record(vec![
        (
            1,
            CanonicalValue::Text(atom.meta().id().as_str().to_string()),
        ),
        (2, CanonicalValue::Digest(atom.revision())),
        (
            3,
            CanonicalValue::Enum(LifecycleState::Ratified as u16, None),
        ),
        (
            4,
            CanonicalValue::Some(Box::new(CanonicalValue::Record(vec![
                (1, CanonicalValue::Text(approval.authority().to_string())),
                (
                    2,
                    CanonicalValue::Text(approval.record().as_str().to_string()),
                ),
                (3, CanonicalValue::Digest(approval.digest())),
            ]))),
        ),
    ]);
    let graph = SpecGraph {
        atoms: vec![atom],
        ..SpecGraph::default()
    };
    let mut graph_fields = (1_u16..=16)
        .map(|tag| (tag, CanonicalValue::Set(Vec::new())))
        .collect::<Vec<_>>();
    graph_fields[0] = (1, CanonicalValue::Set(vec![atom_value]));
    assert_eq!(
        semantic_graph_digest(&graph),
        semantic_digest("spec-graph-semantic", graph_fields)
    );
    assert_eq!(
        complete_graph_digest(digest(61), digest(62), digest(63), digest(64)),
        semantic_digest(
            "spec-graph-complete",
            vec![
                (1, CanonicalValue::Digest(digest(61))),
                (2, CanonicalValue::Digest(digest(62))),
                (3, CanonicalValue::Digest(digest(63))),
                (4, CanonicalValue::Digest(digest(64))),
            ],
        )
    );

    let reference_value = |reference: &ObjectRef| {
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Enum(reference.kind() as u16, None)),
            (2, CanonicalValue::Text(reference.id().as_str().to_string())),
            (3, CanonicalValue::Digest(reference.digest())),
        ])
    };
    let direct = test_object_ref(ObjectKind::Atom, sid("SPG-DIRECT"), digest(65));
    let transitive = test_object_ref(
        ObjectKind::EvidenceCarrier,
        sid("CAR-TRANSITIVE"),
        digest(66),
    );
    let base_policy = test_object_ref(ObjectKind::CoveragePolicy, sid("POL-BASE"), digest(67));
    let head_policy = test_object_ref(ObjectKind::CoveragePolicy, sid("POL-HEAD"), digest(68));
    let obligation = ObligationRef::new(
        sid("POL-HEAD"),
        ObligationKind::Authority,
        direct.clone(),
        None,
    )
    .expect("obligation");
    let ratchet = PolicyRatchetImpact::new(
        ScopeId::new("ratchet.v1").expect("valid scope"),
        base_policy.clone(),
        head_policy.clone(),
        vec![obligation.clone()],
    )
    .expect("ratchet impact");
    assert!(
        PolicyRatchetImpact::new(
            ScopeId::new("ratchet.v1").expect("valid scope"),
            base_policy.clone(),
            base_policy.clone(),
            vec![obligation.clone()],
        )
        .is_ok()
    );
    assert_eq!(
        PolicyRatchetImpact::new(
            ScopeId::new("ratchet.v1").expect("valid scope"),
            base_policy.clone(),
            head_policy.clone(),
            Vec::new(),
        ),
        Err(ConstructionError::LocalInvariant)
    );
    let obligation_value = CanonicalValue::Record(vec![
        (1, CanonicalValue::Text(String::from("POL-HEAD"))),
        (
            2,
            CanonicalValue::Enum(ObligationKind::Authority as u16, None),
        ),
        (3, reference_value(&direct)),
        (4, CanonicalValue::None),
    ]);
    assert_eq!(
        impact_set_digest(
            vec![direct.clone()],
            vec![transitive.clone()],
            vec![ratchet]
        ),
        semantic_digest(
            "impact-set",
            vec![
                (1, CanonicalValue::Set(vec![reference_value(&direct)])),
                (2, CanonicalValue::Set(vec![reference_value(&transitive)])),
                (
                    3,
                    CanonicalValue::Set(vec![CanonicalValue::Record(vec![
                        (1, CanonicalValue::Text(String::from("ratchet.v1"))),
                        (2, reference_value(&base_policy)),
                        (3, reference_value(&head_policy)),
                        (4, CanonicalValue::Set(vec![obligation_value])),
                    ])]),
                ),
            ],
        )
    );
    assert_eq!(
        impact_set_digest(vec![direct.clone()], vec![direct], Vec::new()),
        Err(EncodingError::DuplicateSetValue)
    );

    let path = RepositoryPath::new("generated/table.bin").expect("canonical path");
    let entry = ArtifactEntry::saved_bytes(path.clone(), RawArtifactMediaType::Rust, digest(71));
    let index = ArtifactIndex::new(vec![entry]).expect("artifact index");
    assert_eq!(
        index.digest(),
        semantic_digest(
            "artifact-index",
            vec![(
                1,
                CanonicalValue::Set(vec![CanonicalValue::Record(vec![
                    (1, CanonicalValue::Text(path.as_str().to_string())),
                    (2, CanonicalValue::Text(String::from("raw-artifact"))),
                    (3, CanonicalValue::Text(String::from("text/x-rust"))),
                    (4, CanonicalValue::Text(String::from("saved-bytes/v1"))),
                    (5, CanonicalValue::Digest(digest(71))),
                ])]),
            )],
        )
    );
}

#[test]
fn lifecycle_is_closed_and_terminal() {
    assert_eq!(
        lifecycle_transition(LifecycleState::Proposed, LifecycleEvent::Ratify),
        Ok(LifecycleState::Ratified)
    );
    assert_eq!(
        lifecycle_transition(LifecycleState::Ratified, LifecycleEvent::Supersede),
        Ok(LifecycleState::Superseded)
    );
    assert_eq!(
        lifecycle_transition(LifecycleState::Proposed, LifecycleEvent::Withdraw),
        Ok(LifecycleState::Withdrawn)
    );
    assert_eq!(
        lifecycle_transition(LifecycleState::Ratified, LifecycleEvent::Withdraw),
        Ok(LifecycleState::Withdrawn)
    );
    for terminal in [LifecycleState::Superseded, LifecycleState::Withdrawn] {
        for event in [
            LifecycleEvent::Ratify,
            LifecycleEvent::Withdraw,
            LifecycleEvent::Supersede,
        ] {
            assert_eq!(
                lifecycle_transition(terminal, event),
                Err(LifecycleError::IllegalTransition)
            );
        }
    }
    assert!(!governance_transition_valid(
        Some(PolicyLifecycle::Retired),
        Some(PolicyLifecycle::Active),
        None,
        None,
        None,
        None
    ));

    let retired = atom("SPG-003", "A retired ID MUST remain reserved.");
    assert!(retired.is_some());
    let Some(retired) = retired else { return };
    let retained_tombstone = tombstone(
        "SPG-003",
        LifecycleState::Withdrawn,
        retired.revision(),
        Vec::new(),
    );
    assert_eq!(retained_tombstone.meta().kind(), ObjectKind::Tombstone);
    let base = SpecGraph {
        atoms: vec![retired.clone()],
        ..SpecGraph::default()
    };
    let unrelated_tombstone = tombstone(
        "SPG-UNRELATED",
        LifecycleState::Withdrawn,
        digest(98),
        Vec::new(),
    );
    let wrong_revision_tombstone =
        tombstone("SPG-003", LifecycleState::Withdrawn, digest(97), Vec::new());
    let head = SpecGraph {
        tombstones: vec![
            unrelated_tombstone.clone(),
            wrong_revision_tombstone,
            retained_tombstone.clone(),
        ],
        ..SpecGraph::default()
    };
    assert!(validate_atom_history(&base, &head).is_empty());
    let superseded_head = SpecGraph {
        tombstones: vec![
            unrelated_tombstone,
            tombstone(
                "SPG-003",
                LifecycleState::Superseded,
                base.atoms[0].revision(),
                vec![test_atom_ref(sid("SPG-004"), digest(101))],
            ),
        ],
        ..SpecGraph::default()
    };
    assert!(validate_atom_history(&base, &superseded_head).is_empty());
    let reused = SpecGraph {
        atoms: vec![
            atom("SPG-UNRELATED-ATOM", "Unrelated atom.").expect("canonical unrelated atom"),
            retired,
        ],
        tombstones: vec![retained_tombstone],
        ..SpecGraph::default()
    };
    assert!(!validate_atom_history(&head, &reused).is_empty());
    let retained_with_unrelated_atom = SpecGraph {
        atoms: vec![atom("SPG-OTHER", "Other atom.").expect("canonical other atom")],
        tombstones: head.tombstones.clone(),
        ..SpecGraph::default()
    };
    assert!(validate_atom_history(&head, &retained_with_unrelated_atom).is_empty());
    assert_eq!(
        validate_atom_history(&head, &SpecGraph::default())[0].code,
        "SPG-AUTH-TERMINAL-HISTORY"
    );
    let mismatched_tombstone = SpecGraph {
        tombstones: vec![tombstone(
            "SPG-003",
            LifecycleState::Superseded,
            digest(103),
            vec![test_atom_ref(sid("SPG-004"), digest(104))],
        )],
        ..SpecGraph::default()
    };
    assert_eq!(
        validate_atom_history(&head, &mismatched_tombstone)[0].code,
        "SPG-AUTH-TERMINAL-HISTORY"
    );

    let Some(prior) = atom("SPG-PRIOR", "Prior atom.") else {
        return;
    };
    let illegal_current = Atom::new(
        sid("SPG-PRIOR"),
        LifecycleState::Proposed,
        AtomKind::Constraint,
        vec![ScopeId::new("spec.graph").expect("valid scope")],
        "The atom MUST NOT regress.",
        None,
    )
    .expect("canonical regressed atom");
    let prior_graph = SpecGraph {
        atoms: vec![prior.clone()],
        ..SpecGraph::default()
    };
    let illegal_graph = SpecGraph {
        atoms: vec![illegal_current],
        ..SpecGraph::default()
    };
    let legal_graph = SpecGraph {
        atoms: vec![prior.clone()],
        ..SpecGraph::default()
    };
    assert!(validate_atom_history(&prior_graph, &legal_graph).is_empty());
    assert_eq!(
        validate_atom_history(&prior_graph, &illegal_graph)[0].code,
        "SPG-AUTH-ILLEGAL-TRANSITION"
    );
    assert_eq!(
        validate_atom_history(&prior_graph, &SpecGraph::default())[0].code,
        "SPG-AUTH-MISSING-TOMBSTONE"
    );
}

#[test]
fn governance_transition_relation_is_exhaustively_exercised() {
    let policies = [
        None,
        Some(PolicyLifecycle::Active),
        Some(PolicyLifecycle::Superseded),
        Some(PolicyLifecycle::Retired),
    ];
    let debts = [
        None,
        Some(DebtLifecycle::Active),
        Some(DebtLifecycle::Discharged),
        Some(DebtLifecycle::Superseded),
    ];
    let waivers = [
        None,
        Some(WaiverLifecycle::Active),
        Some(WaiverLifecycle::Discharged),
        Some(WaiverLifecycle::Revoked),
        Some(WaiverLifecycle::Superseded),
    ];
    let policy_valid = policies
        .iter()
        .flat_map(|base| policies.iter().map(move |head| (*base, *head)))
        .filter(|(base, head)| {
            governance_transition_valid(
                *base,
                *head,
                Some(DebtLifecycle::Active),
                Some(DebtLifecycle::Active),
                Some(WaiverLifecycle::Active),
                Some(WaiverLifecycle::Active),
            )
        })
        .count();
    let debt_valid = debts
        .iter()
        .flat_map(|base| debts.iter().map(move |head| (*base, *head)))
        .filter(|(base, head)| {
            governance_transition_valid(
                Some(PolicyLifecycle::Active),
                Some(PolicyLifecycle::Active),
                *base,
                *head,
                Some(WaiverLifecycle::Active),
                Some(WaiverLifecycle::Active),
            )
        })
        .count();
    let waiver_valid = waivers
        .iter()
        .flat_map(|base| waivers.iter().map(move |head| (*base, *head)))
        .filter(|(base, head)| {
            governance_transition_valid(
                Some(PolicyLifecycle::Active),
                Some(PolicyLifecycle::Active),
                Some(DebtLifecycle::Active),
                Some(DebtLifecycle::Active),
                *base,
                *head,
            )
        })
        .count();
    assert_eq!((policy_valid, debt_valid, waiver_valid), (6, 6, 8));
}

#[test]
fn canonical_encoding_is_unambiguous_and_permutation_invariant() {
    let one = CanonicalValue::Record(vec![
        (1, CanonicalValue::Text(String::from("ab"))),
        (2, CanonicalValue::Text(String::from("c"))),
    ]);
    let two = CanonicalValue::Record(vec![
        (1, CanonicalValue::Text(String::from("a"))),
        (2, CanonicalValue::Text(String::from("bc"))),
    ]);
    assert_ne!(one.encode().ok(), two.encode().ok());

    let ordered = CanonicalValue::Set(vec![CanonicalValue::U64(1), CanonicalValue::U64(2)]);
    let permuted = CanonicalValue::Set(vec![CanonicalValue::U64(2), CanonicalValue::U64(1)]);
    assert_eq!(ordered.encode().ok(), permuted.encode().ok());
    assert_eq!(
        CanonicalValue::U64(1).encode().ok(),
        Some(vec![2, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 1])
    );
    assert_eq!(
        CanonicalValue::Set(vec![CanonicalValue::U64(1), CanonicalValue::U64(1)])
            .encode()
            .err(),
        Some(EncodingError::DuplicateSetValue)
    );

    let golden = atom("DET-004", "Canonical fields MUST be unambiguous.");
    assert!(golden.is_some());
    let Some(golden) = golden else { return };
    assert_eq!(
        golden.revision().canonical_text(),
        "xxh3-128:4a07be79938bf8cd6caca2be3dea0c32"
    );
}

#[test]
fn path_policy_is_portable_and_collision_closed() {
    assert!(RepositoryPath::new("spec/requirements.md").is_ok());
    assert_eq!(
        RepositoryPath::new("../outside").err(),
        Some(PathError::InvalidSegment)
    );
    assert_eq!(
        RepositoryPath::new("spec/CON.txt").err(),
        Some(PathError::ReservedBasename)
    );
    let lower = RepositoryPath::new("spec/readme.md").ok();
    let upper = RepositoryPath::new("spec/README.md").ok();
    assert!(lower.is_some() && upper.is_some());
    let (Some(lower), Some(upper)) = (lower, upper) else {
        return;
    };
    assert_eq!(
        canonical_paths(vec![lower, upper]).err(),
        Some(PathSetError::PortableCollision)
    );
}

#[test]
fn surface_completeness_reports_missing_unknown_duplicate_and_authority() {
    let atom = atom("SPG-008", "Every member MUST have one disposition.");
    assert!(atom.is_some());
    let Some(atom) = atom else { return };
    let atom_ref = test_atom_ref(atom.meta().id().clone(), atom.revision());
    let expected_a = test_payload("key/v1", b"a".to_vec());
    let expected_b = test_payload("key/v1", b"b".to_vec());
    let unknown = test_payload("key/v1", b"c".to_vec());
    let payload = Some(test_payload("disposition/v1", vec![1]));
    let surface = decision_surface(
        "SURFACE-001",
        vec![expected_a.clone(), expected_b],
        vec![
            test_surface_disposition(
                expected_a.clone(),
                SurfaceDispositionKind::Normative,
                payload.clone(),
                Some(atom_ref.clone()),
            ),
            test_surface_disposition(
                unknown,
                SurfaceDispositionKind::Prohibited,
                payload,
                Some(atom_ref.clone()),
            ),
        ],
    );
    let graph = SpecGraph {
        atoms: vec![atom],
        ..SpecGraph::default()
    };
    assert_eq!(surface.domain().surface_id().as_str(), "SURFACE-001");
    let result = surface_completeness(&graph, &surface);
    assert!(!result.complete);
    assert_eq!(result.missing, vec![test_payload("key/v1", b"b".to_vec())]);
    assert!(result.duplicate.is_empty());
    assert_eq!(result.unknown, vec![test_payload("key/v1", b"c".to_vec())]);

    let schema_mismatch = decision_surface(
        "SURFACE-SCHEMA-MISMATCH",
        vec![test_payload("key/v1", b"same".to_vec())],
        vec![test_surface_disposition(
            test_payload("other-key/v1", b"same".to_vec()),
            SurfaceDispositionKind::NotApplicable,
            None,
            None,
        )],
    );
    let mismatch = surface_completeness(&graph, &schema_mismatch);
    assert_eq!(
        mismatch.missing,
        vec![test_payload("key/v1", b"same".to_vec())]
    );
    assert_eq!(
        mismatch.unknown,
        vec![test_payload("other-key/v1", b"same".to_vec())]
    );

    let isolated_key = test_payload("key/v1", b"isolated".to_vec());
    let missing_only = decision_surface("SURFACE-MISSING", vec![isolated_key.clone()], Vec::new());
    let missing_result = surface_completeness(&graph, &missing_only);
    assert!(!missing_result.missing.is_empty());
    let duplicate_domain = DecisionSurfaceDomain::new(
        sid("SURFACE-DUPLICATE"),
        test_payload("key-schema/v1", Vec::new()),
        test_payload("disposition-vocabulary/v1", Vec::new()),
        vec![isolated_key.clone()],
        1,
    )
    .expect("duplicate-control domain");
    let duplicate_row = test_surface_disposition(
        isolated_key.clone(),
        SurfaceDispositionKind::NotApplicable,
        None,
        None,
    );
    let different_duplicate_row = test_surface_disposition(
        isolated_key.clone(),
        SurfaceDispositionKind::NotApplicable,
        Some(test_payload("reason/v1", vec![1])),
        None,
    );
    assert_eq!(
        DecisionSurface::new(
            duplicate_domain,
            vec![duplicate_row, different_duplicate_row],
            Vec::new(),
        ),
        Err(ConstructionError::LocalInvariant)
    );
    let unknown_only = decision_surface(
        "SURFACE-UNKNOWN",
        vec![test_payload("key/v1", b"expected".to_vec())],
        vec![test_surface_disposition(
            isolated_key,
            SurfaceDispositionKind::NotApplicable,
            None,
            None,
        )],
    );
    let unknown_result = surface_completeness(&graph, &unknown_only);
    assert!(!unknown_result.unknown.is_empty());

    assert_eq!(
        SurfaceDisposition::new(
            test_payload("key/v1", b"a".to_vec()),
            SurfaceDispositionKind::Normative,
            None,
            None,
        ),
        Err(ConstructionError::LocalInvariant)
    );
    assert_eq!(
        SurfaceDisposition::new(
            test_payload("key/v1", b"payload".to_vec()),
            SurfaceDispositionKind::Normative,
            None,
            Some(atom_ref),
        ),
        Err(ConstructionError::LocalInvariant)
    );

    let not_applicable_key = test_payload("key/v1", b"n/a".to_vec());
    let not_applicable_surface = decision_surface(
        "SURFACE-NOT-APPLICABLE",
        vec![not_applicable_key.clone()],
        vec![test_surface_disposition(
            not_applicable_key,
            SurfaceDispositionKind::NotApplicable,
            None,
            None,
        )],
    );
    assert!(surface_completeness(&graph, &not_applicable_surface).complete);
}

#[test]
fn freshness_roles_and_no_laundering_are_fail_closed() {
    let atom = atom("EVD-003", "A carrier MUST bind the current revision.");
    assert!(atom.is_some());
    let Some(atom) = atom else { return };
    let oracle = test_oracle(sid("ORC-001"), digest(22), Vec::new());
    let oracle_ref = test_object_ref(
        ObjectKind::Oracle,
        oracle.meta().id().clone(),
        oracle.meta().digest(),
    );
    let carrier = test_carrier(
        sid("CAR-001"),
        digest(23),
        Vec::new(),
        EvidenceRole::Positive,
        vec![test_atom_ref(atom.meta().id().clone(), atom.revision())],
        oracle_ref,
        digest(24),
        vec![digest(25)],
        test_payload("scope.exact/v1", vec![1]),
    );
    let policy = test_policy(
        sid("POL-001"),
        digest(26),
        Vec::new(),
        digest(27),
        PolicyLifecycle::Active,
        EnforcementMode::Blocking,
        ScopeId::new("line").expect("valid scope"),
        vec![test_object_ref(
            ObjectKind::Atom,
            atom.meta().id().clone(),
            atom.meta().digest(),
        )],
        vec![EvidenceRole::Positive, EvidenceRole::Negative],
    );
    let graph = SpecGraph {
        atoms: vec![atom],
        evidence_carriers: vec![carrier.clone()],
        oracles: vec![oracle],
        ..SpecGraph::default()
    };
    assert!(carrier_fresh(&graph, &carrier));
    assert_eq!(
        required_role_coverage(&graph, &policy).missing,
        vec![EvidenceRole::Negative]
    );
    let mut stale_role_graph = graph.clone();
    stale_role_graph.evidence_carriers = vec![test_carrier(
        sid("CAR-STALE-ROLE"),
        digest(27),
        Vec::new(),
        EvidenceRole::Positive,
        vec![test_atom_ref(sid("EVD-003"), digest(28))],
        test_object_ref(ObjectKind::Oracle, sid("ORC-001"), digest(22)),
        digest(29),
        Vec::new(),
        test_payload("scope.exact/v1", vec![1]),
    )];
    assert!(
        required_role_coverage(&stale_role_graph, &policy)
            .missing
            .contains(&EvidenceRole::Positive)
    );
    let classes = aggregate_assurance(&AggregationInput {
        linked: true,
        required_fresh: true,
        required_selected: true,
        verdicts: vec![Verdict::Fail],
        exhaustive: false,
        formal: false,
        structural: false,
        reviewed: false,
        debt: true,
        waiver: true,
        reaffirmation: true,
    });
    assert_eq!(classes, vec![AssuranceClass::Linked]);
    let complete_input = AggregationInput {
        linked: true,
        required_fresh: true,
        required_selected: true,
        verdicts: vec![Verdict::Pass],
        exhaustive: true,
        formal: true,
        structural: true,
        reviewed: true,
        debt: false,
        waiver: false,
        reaffirmation: false,
    };
    assert_eq!(
        aggregate_assurance(&complete_input),
        vec![
            AssuranceClass::Linked,
            AssuranceClass::Tested,
            AssuranceClass::ExhaustivelyChecked,
            AssuranceClass::FormallyProved,
            AssuranceClass::StructurallyEnforced,
            AssuranceClass::ReviewOnly,
        ]
    );
    for incomplete in [
        AggregationInput {
            linked: false,
            ..complete_input.clone()
        },
        AggregationInput {
            required_fresh: false,
            ..complete_input.clone()
        },
        AggregationInput {
            required_selected: false,
            ..complete_input.clone()
        },
        AggregationInput {
            verdicts: Vec::new(),
            ..complete_input.clone()
        },
        AggregationInput {
            verdicts: vec![Verdict::Fail],
            ..complete_input.clone()
        },
    ] {
        assert!(!aggregate_assurance(&incomplete).contains(&AssuranceClass::Tested));
    }
}

#[test]
#[allow(clippy::panic)]
fn governance_report_facts_keep_live_and_terminal_obligations_unsatisfied() {
    let Some(atom) = atom("GOVERNANCE-ATOM", "Governance MUST never satisfy evidence.") else {
        return;
    };
    let subject = test_object_ref(
        ObjectKind::Atom,
        atom.meta().id().clone(),
        atom.meta().digest(),
    );
    let obligation = test_obligation(
        sid("GOVERNANCE-POLICY"),
        subject.clone(),
        Some(EvidenceRole::Negative),
    );
    let graph = SpecGraph {
        atoms: vec![atom],
        debt_records: vec![
            test_debt(
                sid("DEBT-ACTIVE-FACT"),
                digest(115),
                vec![subject.clone()],
                DebtLifecycle::Active,
                vec![obligation.clone()],
            ),
            test_debt(
                sid("DEBT-DISCHARGED-FACT"),
                digest(116),
                vec![subject.clone()],
                DebtLifecycle::Discharged,
                vec![obligation.clone()],
            ),
        ],
        waivers: vec![
            test_waiver(
                sid("WAIVER-ACTIVE-FACT"),
                digest(117),
                vec![subject.clone()],
                WaiverLifecycle::Active,
                vec![obligation.clone()],
            ),
            test_waiver(
                sid("WAIVER-DISCHARGED-FACT"),
                digest(118),
                vec![subject],
                WaiverLifecycle::Discharged,
                vec![obligation],
            ),
        ],
        ..SpecGraph::default()
    };
    let approval_path = RepositoryPath::new("reviews/approval.md").expect("approval path");
    let approval_entry = artifact_entry(approval_path, digest(90)).expect("approval artifact");
    let artifacts = ArtifactIndex::new(vec![approval_entry]).expect("approval index");
    let facts = governance_report_facts(&graph, &artifacts).expect("governance facts");
    assert_eq!(facts.len(), 4);
    let mut permuted = graph.clone();
    permuted.debt_records.reverse();
    permuted.waivers.reverse();
    assert_eq!(
        governance_report_facts(&permuted, &artifacts).expect("permuted governance facts"),
        facts
    );
    let report = CanonicalStaticReport::new(
        digest(119),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        facts,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("static governance report");
    let CanonicalValue::Record(fields) = report.value() else {
        panic!("static report record");
    };
    let CanonicalValue::Set(facts) = &fields[6].1 else {
        panic!("governance fact set");
    };
    assert_eq!(facts.len(), 4);
    for fact in facts {
        let CanonicalValue::Record(fields) = fact else {
            panic!("governance fact record");
        };
        assert_eq!(fields[3].1, fields[4].1);
        assert!(!matches!(&fields[4].1, CanonicalValue::Set(values) if values.is_empty()));
    }
}

#[test]
#[allow(clippy::panic, clippy::too_many_lines)]
fn repository_artifact_freshness_requires_an_explicit_exact_index() {
    let Some(source_path) = RepositoryPath::new("generated/source.bin").ok() else {
        return;
    };
    let Some(proof_path) = RepositoryPath::new("proofs/proof.rs").ok() else {
        return;
    };
    let source_digest = digest(100);
    let proof_digest = digest(101);
    let oracle = test_oracle(sid("ARTIFACT-ORACLE"), digest(102), Vec::new());
    let target = test_implementation_link(sid("ARTIFACT-TARGET"), digest(103), Vec::new());
    let Some(source_binding) = repository_artifact(source_path.clone(), source_digest) else {
        return;
    };
    let derivation = test_derivation(
        sid("ARTIFACT-DERIVATION"),
        digest(104),
        Vec::new(),
        vec![source_binding],
        vec![ArtifactBinding::Object(test_object_ref(
            ObjectKind::ImplementationLink,
            target.meta().id().clone(),
            target.meta().digest(),
        ))],
        test_tool("artifact-generator", "1", digest(105), digest(106)),
        test_object_ref(
            ObjectKind::Oracle,
            oracle.meta().id().clone(),
            oracle.meta().digest(),
        ),
    );
    let graph = SpecGraph {
        implementation_links: vec![target.clone()],
        derivations: vec![derivation.clone()],
        oracles: vec![oracle],
        ..SpecGraph::default()
    };
    assert!(!derivation_fresh(&graph, &derivation));
    let Some(exact_entry) = artifact_entry(source_path.clone(), source_digest) else {
        return;
    };
    let exact = ArtifactIndex::new(vec![exact_entry]);
    assert!(exact.is_ok());
    let Some(exact) = exact.ok() else { return };
    assert_eq!(exact.entries().len(), 1);
    assert_eq!(exact.entries()[0].raw_domain(), "raw-artifact");
    assert_eq!(exact.entries()[0].media_type(), "application/octet-stream");
    assert_eq!(exact.entries()[0].representation(), "raw/v1");
    assert_eq!(exact.entries()[0].raw_digest(), source_digest);
    assert!(derivation_fresh_with_artifacts(&graph, &derivation, &exact));
    let Some(duplicate_path) = RepositoryPath::new("generated/copy.bin").ok() else {
        return;
    };
    let Some(first_path) = RepositoryPath::new("generated/source.bin").ok() else {
        return;
    };
    let Some(first_duplicate) = artifact_entry(first_path, source_digest) else {
        return;
    };
    let Some(second_duplicate) = artifact_entry(duplicate_path, source_digest) else {
        return;
    };
    let duplicates = ArtifactIndex::new(vec![first_duplicate, second_duplicate]);
    assert!(duplicates.is_ok());
    assert_eq!(
        duplicates
            .as_ref()
            .ok()
            .map(|index| index.paths_for_digest(source_digest).len()),
        Some(2)
    );
    let Some(stale_entry) = artifact_entry(source_path, digest(107)) else {
        return;
    };
    let stale = ArtifactIndex::new(vec![stale_entry]);
    assert!(stale.is_ok());
    let Some(stale) = stale.ok() else { return };
    assert!(!derivation_fresh_with_artifacts(
        &graph,
        &derivation,
        &stale
    ));
    let Some(media_path) = RepositoryPath::new("generated/source.bin").ok() else {
        return;
    };
    let media_mismatch = ArtifactEntry::new(
        media_path,
        "raw-artifact",
        "text/plain",
        "raw/v1",
        source_digest,
    );
    assert!(media_mismatch.is_ok());
    let Some(media_mismatch) = media_mismatch.ok() else {
        return;
    };
    let media_index = ArtifactIndex::new(vec![media_mismatch]);
    assert!(media_index.is_ok());
    let Some(media_index) = media_index.ok() else {
        return;
    };
    assert!(!derivation_fresh_with_artifacts(
        &graph,
        &derivation,
        &media_index
    ));
    let Some(invalid_path) = RepositoryPath::new("generated/invalid.bin").ok() else {
        return;
    };
    assert_eq!(
        ArtifactEntry::new(
            invalid_path,
            "other-domain",
            "application/octet-stream",
            "raw/v1",
            source_digest,
        )
        .err(),
        Some(ConstructionError::ArtifactBindingShape)
    );

    let Some(authority_atom) = atom("ARTIFACT-PROOF-ATOM", "Proof artifacts MUST be exact.") else {
        return;
    };
    let atom_ref = test_atom_ref(
        authority_atom.meta().id().clone(),
        authority_atom.revision(),
    );
    let property = test_formal_property(
        sid("ARTIFACT-PROPERTY"),
        digest(108),
        Vec::new(),
        vec![atom_ref.clone()],
        test_payload("property/v1", vec![1]),
        true,
    );
    let Some(proof_binding) = repository_artifact(proof_path.clone(), proof_digest) else {
        return;
    };
    let translator = test_tool("translator", "1", digest(112), digest(113));
    let verifier = test_tool("verifier", "1", digest(110), digest(111));
    let proof = test_proof_record(
        sid("ARTIFACT-PROOF"),
        digest(109),
        Vec::new(),
        vec![atom_ref],
        test_object_ref(
            ObjectKind::FormalProperty,
            property.meta().id().clone(),
            property.meta().digest(),
        ),
        proof_binding.clone(),
        Vec::new(),
        vec![translator.clone()],
        verifier.clone(),
        Vec::new(),
        Vec::new(),
        vec![proof_binding],
        Verdict::Pass,
    );
    let proof_graph = SpecGraph {
        atoms: vec![authority_atom],
        formal_properties: vec![property],
        proof_records: vec![proof.clone()],
        ..SpecGraph::default()
    };
    assert!(!proof_fresh(&proof_graph, &proof));
    let proof_entry = ArtifactEntry::new(
        proof_path,
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        proof_digest,
    );
    assert!(proof_entry.is_ok());
    let Some(proof_entry) = proof_entry.ok() else {
        return;
    };
    let proof_index = ArtifactIndex::new(vec![proof_entry]);
    assert!(proof_index.is_ok());
    let Some(proof_index) = proof_index.ok() else {
        return;
    };
    assert!(proof_fresh_with_artifacts(
        &proof_graph,
        &proof,
        &proof_index
    ));
    let expectation = |configuration, scope, exclusions, correspondence, negative| {
        ProofExpectation::new(
            proof.meta().id().clone(),
            configuration,
            scope,
            exclusions,
            correspondence,
            negative,
        )
    };
    let exact_expectation = expectation(
        proof.verifier_configuration(),
        proof.scope().clone(),
        proof.exclusions().to_vec(),
        true,
        true,
    );
    let exact_context = ProofInvalidationContext::new(
        &proof_index,
        vec![translator.clone(), verifier.clone()],
        vec![exact_expectation.clone()],
    )
    .expect("exact proof invalidation context");
    assert!(proof_fresh_with_context(
        &proof_graph,
        &proof,
        &exact_context
    ));
    assert!(proof_invalidation_reasons(&proof_graph, &proof, &exact_context).is_empty());
    let exact_fact = proof_report_fact(
        &proof_graph,
        &proof,
        &exact_context,
        test_payload("proof-assumptions-exclusions/v1", Vec::new()),
    )
    .expect("exact proof fact");
    let exact_report = CanonicalStaticReport::new(
        digest(120),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![exact_fact],
        Vec::new(),
    )
    .expect("proof static report");
    let CanonicalValue::Record(report_fields) = exact_report.value() else {
        panic!("proof report record");
    };
    let CanonicalValue::Set(proof_facts) = &report_fields[8].1 else {
        panic!("proof fact set");
    };
    let CanonicalValue::Record(proof_fields) = &proof_facts[0] else {
        panic!("proof fact record");
    };
    assert_eq!(
        proof_fields[4].1,
        CanonicalValue::Enum(Freshness::Fresh as u16, None)
    );
    assert_eq!(
        proof_fields[10].1,
        CanonicalValue::Enum(Satisfaction::Satisfied as u16, None)
    );
    assert_eq!(
        ProofInvalidationContext::new(
            &proof_index,
            vec![verifier.clone(), verifier.clone()],
            vec![exact_expectation.clone()],
        )
        .err(),
        Some(ProofInvalidationContextError::DuplicateTool)
    );
    assert_eq!(
        ProofInvalidationContext::new(
            &proof_index,
            vec![translator.clone(), verifier.clone()],
            vec![exact_expectation.clone(), exact_expectation],
        )
        .err(),
        Some(ProofInvalidationContextError::DuplicateExpectation)
    );
    for (tools, expectation, code) in [
        (
            vec![translator.clone()],
            expectation(
                proof.verifier_configuration(),
                proof.scope().clone(),
                proof.exclusions().to_vec(),
                true,
                true,
            ),
            "FPR-BIND-VERIFIER",
        ),
        (
            vec![verifier.clone()],
            expectation(
                proof.verifier_configuration(),
                proof.scope().clone(),
                proof.exclusions().to_vec(),
                true,
                true,
            ),
            "FPR-BIND-TRANSLATOR",
        ),
        (
            vec![translator.clone(), verifier.clone()],
            expectation(
                digest(114),
                proof.scope().clone(),
                proof.exclusions().to_vec(),
                true,
                true,
            ),
            "FPR-BIND-VERIFIER-CONFIGURATION",
        ),
        (
            vec![translator.clone(), verifier.clone()],
            expectation(
                proof.verifier_configuration(),
                test_payload("proof-scope/v1", vec![2]),
                proof.exclusions().to_vec(),
                true,
                true,
            ),
            "FPR-BIND-SCOPE",
        ),
        (
            vec![translator.clone(), verifier.clone()],
            expectation(
                proof.verifier_configuration(),
                proof.scope().clone(),
                vec![String::from("missing exclusion")],
                true,
                true,
            ),
            "FPR-BIND-EXCLUSIONS",
        ),
        (
            vec![translator.clone(), verifier.clone()],
            expectation(
                proof.verifier_configuration(),
                proof.scope().clone(),
                proof.exclusions().to_vec(),
                false,
                true,
            ),
            "FPR-BIND-CORRESPONDENCE",
        ),
        (
            vec![translator, verifier],
            expectation(
                proof.verifier_configuration(),
                proof.scope().clone(),
                proof.exclusions().to_vec(),
                true,
                false,
            ),
            "FPR-VERDICT-NEGATIVE-CONTROL",
        ),
    ] {
        let context = ProofInvalidationContext::new(&proof_index, tools, vec![expectation])
            .expect("stale proof context");
        let reasons = proof_invalidation_reasons(&proof_graph, &proof, &context);
        assert!(reasons.iter().any(|reason| reason == code));
        assert!(!proof_fresh_with_context(&proof_graph, &proof, &context));
    }
}

#[test]
fn policy_ratchet_detects_every_weakening_axis() {
    let subject_a = test_object_ref(ObjectKind::Atom, sid("EVD-001"), digest(1));
    let subject_b = test_object_ref(ObjectKind::Atom, sid("EVD-002"), digest(2));
    let base = test_policy(
        sid("POL-001"),
        digest(28),
        Vec::new(),
        digest(29),
        PolicyLifecycle::Active,
        EnforcementMode::Blocking,
        ScopeId::new("line-a").expect("valid scope"),
        vec![subject_a.clone(), subject_b],
        vec![EvidenceRole::Positive, EvidenceRole::Negative],
    );
    let head = test_policy(
        sid("POL-001"),
        digest(30),
        Vec::new(),
        digest(31),
        PolicyLifecycle::Active,
        EnforcementMode::Advisory,
        ScopeId::new("line-b").expect("valid scope"),
        vec![subject_a],
        vec![EvidenceRole::Positive],
    );
    let comparison = compare_policy(&base, &head);
    assert!(comparison.weakened());
    assert_eq!(comparison.removed_subjects.len(), 1);
    assert_eq!(comparison.removed_roles, vec![EvidenceRole::Negative]);
    assert!(comparison.mode_weakened);
    assert!(comparison.lineage_changed);
    let still_blocking = test_policy(
        sid("POL-001"),
        digest(32),
        Vec::new(),
        digest(33),
        PolicyLifecycle::Active,
        EnforcementMode::Blocking,
        ScopeId::new("line-a").expect("valid scope"),
        base.subjects().to_vec(),
        base.roles().to_vec(),
    );
    assert!(!compare_policy(&base, &still_blocking).mode_weakened);
    let advisory_base = test_policy(
        sid("POL-ADVISORY"),
        digest(34),
        Vec::new(),
        digest(35),
        PolicyLifecycle::Active,
        EnforcementMode::Advisory,
        ScopeId::new("line-a").expect("valid scope"),
        base.subjects().to_vec(),
        base.roles().to_vec(),
    );
    assert!(!compare_policy(&advisory_base, &still_blocking).mode_weakened);
    for isolated in [
        PolicyRatchet {
            removed_subjects: Vec::new(),
            removed_roles: vec![EvidenceRole::Negative],
            mode_weakened: false,
            lineage_changed: false,
        },
        PolicyRatchet {
            removed_subjects: Vec::new(),
            removed_roles: Vec::new(),
            mode_weakened: true,
            lineage_changed: false,
        },
        PolicyRatchet {
            removed_subjects: Vec::new(),
            removed_roles: Vec::new(),
            mode_weakened: false,
            lineage_changed: true,
        },
    ] {
        assert!(isolated.weakened());
    }
    assert!(!PolicyRatchet::default().weakened());
}

#[test]
fn graph_derived_policy_ratchets_materialize_complete_obligations() {
    let Some(base_atom_a) = atom("RATCHET-A", "Policy scope A MUST remain governed.") else {
        return;
    };
    let Some(base_atom_b) = atom("RATCHET-B", "Policy scope B MUST remain governed.") else {
        return;
    };
    let head_atom_a = base_atom_a.clone();
    let subject = |atom: &Atom| {
        test_object_ref(
            ObjectKind::Atom,
            atom.meta().id().clone(),
            atom.meta().digest(),
        )
    };
    let selector = TypedSelector::new(
        RepositoryPath::new("specs").expect("selector root"),
        SchemaId::new("markdown/v1").expect("valid schema"),
        vec![String::from("**/*.md")],
        Vec::new(),
        AuthorityExpectation::Required,
    )
    .expect("authority selector");
    let make_policy = |roles: Vec<EvidenceRole>,
                       mode: EnforcementMode,
                       ratchet: ScopeId,
                       authorities: Vec<TypedSelector>,
                       derivation: Option<TypedPayload>,
                       subjects: Vec<ObjectRef>| {
        let revision = CoveragePolicyRevision::new(
            test_payload("policy-scope/v1", vec![1]),
            roles,
            mode,
            ratchet,
            authorities,
            derivation,
        )
        .expect("policy revision");
        CoveragePolicy::new(
            sid("POLICY-RATCHET"),
            revision,
            approval().expect("policy approval"),
            PolicyLifecycle::Active,
            Vec::new(),
            None,
            subjects,
        )
        .expect("policy")
    };
    let base_policy = make_policy(
        vec![EvidenceRole::Positive, EvidenceRole::Negative],
        EnforcementMode::Blocking,
        ScopeId::new("ratchet-a").expect("valid scope"),
        vec![selector],
        Some(test_payload("derivation-requirement/v1", vec![1])),
        vec![subject(&base_atom_a), subject(&base_atom_b)],
    );
    let head_policy = make_policy(
        vec![EvidenceRole::Positive],
        EnforcementMode::Advisory,
        ScopeId::new("ratchet-b").expect("valid scope"),
        Vec::new(),
        None,
        vec![subject(&head_atom_a)],
    );
    let base = SpecGraph {
        atoms: vec![base_atom_a, base_atom_b],
        coverage_policies: vec![base_policy],
        ..SpecGraph::default()
    };
    let head = SpecGraph {
        atoms: vec![head_atom_a],
        coverage_policies: vec![head_policy],
        ..SpecGraph::default()
    };
    let base_obligations = materialize_policy_obligations(&base, &base.coverage_policies[0])
        .expect("base obligations");
    assert_eq!(base_obligations.len(), 8);
    let ratchets = derive_policy_ratchets(&base, &head).expect("derived policy ratchets");
    assert_eq!(ratchets.len(), 1);
    #[cfg(feature = "coverage-controls")]
    {
        let diagnostics = buoy::cli::ratchet_diagnostics_control(&ratchets);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "CHG-RATCHET-MODE"
                && diagnostic.id.as_deref() == Some("POLICY-RATCHET")
        }));
    }
    let ratchet = &ratchets[0];
    assert_eq!(ratchet.policy_id().as_str(), "POLICY-RATCHET");
    assert_eq!(
        ratchet.classification(),
        PolicyChangeClassification::Weakening
    );
    assert_eq!(ratchet.base_obligations().len(), 8);
    assert_eq!(ratchet.head_obligations().len(), 2);
    assert_eq!(ratchet.removed_obligations().len(), 6);
    assert!(ratchet.added_obligations().is_empty());
    assert!(ratchet.base_policy().is_some() && ratchet.head_policy().is_some());
    assert!(ratchet.impact().is_some());
    assert_eq!(
        ratchet.diagnostics(),
        &[
            String::from("CHG-RATCHET-AUTHORITY"),
            String::from("CHG-RATCHET-DERIVATION"),
            String::from("CHG-RATCHET-LINEAGE"),
            String::from("CHG-RATCHET-MODE"),
            String::from("CHG-RATCHET-ROLE"),
            String::from("CHG-RATCHET-SCOPE"),
        ]
    );

    let added = derive_policy_ratchets(&SpecGraph::default(), &head)
        .expect("new policy is graph-derived strengthening");
    assert_eq!(
        added[0].classification(),
        PolicyChangeClassification::Strengthening
    );
    assert!(added[0].base_policy().is_none());
    assert_eq!(
        derive_policy_ratchets(&head, &head).expect("equivalent graph pair")[0].classification(),
        PolicyChangeClassification::Equivalent
    );
    let mut permuted_base = base.clone();
    permuted_base.atoms.reverse();
    let mut permuted_head = head.clone();
    permuted_head.atoms.reverse();
    assert_eq!(
        derive_policy_ratchets(&permuted_base, &permuted_head)
            .expect("permuted graph-derived ratchets"),
        ratchets
    );

    let duplicate = SpecGraph {
        coverage_policies: vec![
            head.coverage_policies[0].clone(),
            head.coverage_policies[0].clone(),
        ],
        ..head.clone()
    };
    assert_eq!(
        derive_policy_ratchets(&duplicate, &head),
        Err(PolicyAnalysisError::DuplicatePolicy)
    );
    let dangling_policy = make_policy(
        Vec::new(),
        EnforcementMode::Blocking,
        ScopeId::new("ratchet-a").expect("valid scope"),
        Vec::new(),
        None,
        vec![test_object_ref(
            ObjectKind::Atom,
            sid("MISSING-SUBJECT"),
            digest(200),
        )],
    );
    let dangling = SpecGraph {
        coverage_policies: vec![dangling_policy],
        ..SpecGraph::default()
    };
    assert_eq!(
        derive_policy_ratchets(&dangling, &SpecGraph::default()),
        Err(PolicyAnalysisError::DanglingSubject)
    );
    assert_eq!(
        PolicyAnalysisError::DuplicatePolicy.code(),
        "CHG-RATCHET-POLICY-DUPLICATE"
    );
}

#[test]
fn graph_derived_policy_ratchets_cover_surface_and_waiver_dimensions() {
    let member_a = test_payload("surface-member/v1", vec![1]);
    let member_b = test_payload("surface-member/v1", vec![2]);
    let authority = test_atom_ref(sid("SURFACE-AUTHORITY"), digest(42));
    let disposition = |key: TypedPayload| {
        test_surface_disposition(
            key,
            SurfaceDispositionKind::Normative,
            Some(test_payload("surface-value/v1", vec![1])),
            Some(authority.clone()),
        )
    };
    let base_surface = decision_surface(
        "SURFACE-RATCHET",
        vec![member_a.clone(), member_b.clone()],
        vec![disposition(member_a.clone()), disposition(member_b)],
    );
    let head_surface = decision_surface(
        "SURFACE-RATCHET",
        vec![member_a.clone()],
        vec![disposition(member_a)],
    );
    let make_policy = |subject: ObjectRef| {
        let revision = CoveragePolicyRevision::new(
            test_payload("policy-scope/v1", vec![1]),
            Vec::new(),
            EnforcementMode::Blocking,
            ScopeId::new("surface-line").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("surface policy revision");
        CoveragePolicy::new(
            sid("POLICY-SURFACE"),
            revision,
            approval().expect("surface policy approval"),
            PolicyLifecycle::Active,
            Vec::new(),
            None,
            vec![subject],
        )
        .expect("surface policy")
    };
    let base_subject = test_object_ref(
        ObjectKind::DecisionSurface,
        base_surface.meta().id().clone(),
        base_surface.meta().digest(),
    );
    let head_subject = test_object_ref(
        ObjectKind::DecisionSurface,
        head_surface.meta().id().clone(),
        head_surface.meta().digest(),
    );
    let base_policy = make_policy(base_subject);
    let head_policy = make_policy(head_subject.clone());
    let waiver = Waiver::new(
        sid("WAIVER-WILDCARD"),
        vec![
            ObligationRef::new(
                sid("POLICY-SURFACE"),
                ObligationKind::SurfaceCompleteness,
                head_subject.clone(),
                None,
            )
            .expect("surface waiver obligation"),
        ],
        vec![head_subject],
        "Reviewed wildcard continuation.",
        ScopeId::new("maintainers").expect("valid scope"),
        test_payload("waiver-class/v1", b"wildcard".to_vec()),
        approval().expect("waiver approval"),
        Vec::new(),
        WaiverLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("active waiver class");
    let base = SpecGraph {
        decision_surfaces: vec![base_surface],
        coverage_policies: vec![base_policy],
        ..SpecGraph::default()
    };
    let head = SpecGraph {
        decision_surfaces: vec![head_surface],
        coverage_policies: vec![head_policy],
        waivers: vec![waiver],
        ..SpecGraph::default()
    };
    let weakened = derive_policy_ratchets(&base, &head).expect("surface and waiver comparison");
    assert_eq!(
        weakened[0].classification(),
        PolicyChangeClassification::Weakening
    );
    assert_eq!(
        weakened[0].diagnostics(),
        &[
            String::from("CHG-RATCHET-SURFACE"),
            String::from("CHG-RATCHET-WAIVER-CLASS"),
        ]
    );
    assert!(weakened[0].impact().is_some());
    let strengthened = derive_policy_ratchets(&head, &base).expect("narrowing waiver class");
    assert_eq!(
        strengthened[0].classification(),
        PolicyChangeClassification::Strengthening
    );
    assert!(strengthened[0].diagnostics().is_empty());
}

#[test]
fn graph_derived_policy_ratchets_classify_terminal_transitions() {
    let Some(atom) = atom("TERMINAL-SUBJECT", "Terminal policy MUST retain history.") else {
        return;
    };
    let subject = test_object_ref(
        ObjectKind::Atom,
        atom.meta().id().clone(),
        atom.meta().digest(),
    );
    let revision = |roles: Vec<EvidenceRole>| {
        CoveragePolicyRevision::new(
            test_payload("policy-scope/v1", vec![1]),
            roles,
            EnforcementMode::Blocking,
            ScopeId::new("terminal-line").expect("valid scope"),
            Vec::new(),
            None,
        )
        .expect("terminal policy revision")
    };
    let active = CoveragePolicy::new(
        sid("POLICY-TERMINAL"),
        revision(vec![EvidenceRole::Positive]),
        approval().expect("active approval"),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![subject.clone()],
    )
    .expect("active policy");
    let replacement = CoveragePolicy::new(
        sid("POLICY-REPLACEMENT"),
        revision(vec![EvidenceRole::Positive]),
        approval().expect("replacement approval"),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![subject.clone()],
    )
    .expect("replacement policy");
    let replacement_ref = test_object_ref(
        ObjectKind::CoveragePolicy,
        replacement.meta().id().clone(),
        replacement.meta().digest(),
    );
    let superseded = CoveragePolicy::new(
        sid("POLICY-TERMINAL"),
        revision(vec![EvidenceRole::Positive]),
        approval().expect("superseded approval"),
        PolicyLifecycle::Superseded,
        vec![replacement_ref.clone()],
        approval(),
        vec![subject.clone()],
    )
    .expect("superseded policy");
    let base = SpecGraph {
        atoms: vec![atom.clone()],
        coverage_policies: vec![active.clone()],
        ..SpecGraph::default()
    };
    let covered_head = SpecGraph {
        atoms: vec![atom.clone()],
        coverage_policies: vec![superseded.clone(), replacement],
        ..SpecGraph::default()
    };
    let covered = derive_policy_ratchets(&base, &covered_head).expect("covered supersession");
    let terminal = covered
        .iter()
        .find(|ratchet| ratchet.policy_id().as_str() == "POLICY-TERMINAL")
        .expect("terminal ratchet");
    assert_eq!(
        terminal.classification(),
        PolicyChangeClassification::EquivalentReviewed
    );
    assert!(terminal.diagnostics().is_empty());

    let incomplete_replacement = CoveragePolicy::new(
        sid("POLICY-REPLACEMENT"),
        revision(Vec::new()),
        approval().expect("replacement approval"),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![subject.clone()],
    )
    .expect("incomplete replacement policy");
    let incomplete_ref = test_object_ref(
        ObjectKind::CoveragePolicy,
        incomplete_replacement.meta().id().clone(),
        incomplete_replacement.meta().digest(),
    );
    let invalid_supersession = CoveragePolicy::new(
        sid("POLICY-TERMINAL"),
        revision(vec![EvidenceRole::Positive]),
        approval().expect("superseded approval"),
        PolicyLifecycle::Superseded,
        vec![incomplete_ref],
        approval(),
        vec![subject.clone()],
    )
    .expect("shape-valid supersession");
    let incomplete_head = SpecGraph {
        atoms: vec![atom.clone()],
        coverage_policies: vec![invalid_supersession, incomplete_replacement],
        ..SpecGraph::default()
    };
    let incomplete = derive_policy_ratchets(&base, &incomplete_head)
        .expect("incomplete supersession is classified");
    let terminal = incomplete
        .iter()
        .find(|ratchet| ratchet.policy_id().as_str() == "POLICY-TERMINAL")
        .expect("terminal ratchet");
    assert_eq!(
        terminal.classification(),
        PolicyChangeClassification::Weakening
    );
    assert_eq!(
        terminal.diagnostics(),
        &[String::from("CHG-RATCHET-SUPERSESSION")]
    );

    let retired = CoveragePolicy::new(
        sid("POLICY-TERMINAL"),
        revision(vec![EvidenceRole::Positive]),
        approval().expect("retired approval"),
        PolicyLifecycle::Retired,
        Vec::new(),
        approval(),
        vec![subject],
    )
    .expect("retired policy");
    let retired_head = SpecGraph {
        atoms: vec![atom],
        coverage_policies: vec![retired],
        ..SpecGraph::default()
    };
    let retired = derive_policy_ratchets(&base, &retired_head).expect("retirement");
    assert_eq!(
        retired[0].classification(),
        PolicyChangeClassification::EquivalentReviewed
    );
    let removed = derive_policy_ratchets(&base, &SpecGraph::default()).expect("removed policy");
    assert_eq!(
        removed[0].classification(),
        PolicyChangeClassification::Weakening
    );
    assert_eq!(
        removed[0].diagnostics(),
        &[String::from("CHG-RATCHET-RETIREMENT")]
    );
}

#[test]
fn impact_analysis_reaches_fixed_point_and_requires_exact_dispositions() {
    let base_atom = atom("DET-010", "The base graph MUST be explicit.");
    let head_atom = atom("DET-010", "The head graph MUST be explicit and current.");
    assert!(base_atom.is_some() && head_atom.is_some());
    let (Some(base_atom), Some(head_atom)) = (base_atom, head_atom) else {
        return;
    };
    let base_link = test_implementation_link(
        sid("IMPL-010"),
        digest(40),
        vec![test_object_ref(
            ObjectKind::Atom,
            base_atom.meta().id().clone(),
            base_atom.meta().digest(),
        )],
    );
    let head_link = test_implementation_link(
        sid("IMPL-010"),
        digest(41),
        vec![test_object_ref(
            ObjectKind::Atom,
            head_atom.meta().id().clone(),
            head_atom.meta().digest(),
        )],
    );
    let base_model = test_finite_model(
        sid("MODEL-010"),
        digest(42),
        vec![test_object_ref(
            ObjectKind::ImplementationLink,
            base_link.meta().id().clone(),
            base_link.meta().digest(),
        )],
    );
    let head_model = test_finite_model(
        sid("MODEL-010"),
        digest(43),
        vec![test_object_ref(
            ObjectKind::ImplementationLink,
            head_link.meta().id().clone(),
            head_link.meta().digest(),
        )],
    );
    let base = SpecGraph {
        atoms: vec![base_atom],
        implementation_links: vec![base_link],
        finite_models: vec![base_model],
        ..SpecGraph::default()
    };
    let head = SpecGraph {
        atoms: vec![head_atom],
        implementation_links: vec![head_link],
        finite_models: vec![head_model],
        ..SpecGraph::default()
    };
    let impacts = change_impacts(&base, &head);
    assert!(
        impacts
            .iter()
            .any(|impact| impact.id == sid("DET-010") && impact.direct)
    );
    assert!(
        impacts
            .iter()
            .any(|impact| impact.id == sid("IMPL-010") && impact.direct)
    );
    assert!(
        impacts
            .iter()
            .any(|impact| impact.id == sid("MODEL-010") && impact.direct)
    );

    let disposition = test_change_disposition(
        sid("DISPOSITION-010"),
        digest(43),
        Vec::new(),
        ObjectKind::Atom,
        sid("DET-010"),
        Some(digest(45)),
        Some(digest(46)),
        ChangeDispositionKind::Revised,
    );
    assert_eq!(disposition.meta().id().as_str(), "DISPOSITION-010");
    assert_eq!(disposition.meta().kind(), ObjectKind::ChangeDisposition);
    assert_eq!(disposition.impacted_kind(), ObjectKind::Atom);
    assert_eq!(disposition.impacted_id().as_str(), "DET-010");
    assert_eq!(disposition.base(), Some(digest(45)));
    assert_eq!(disposition.head(), Some(digest(46)));
    assert_eq!(disposition.disposition(), ChangeDispositionKind::Revised);
    let manifest = test_change_manifest(
        sid("CHANGE-010"),
        digest(44),
        Vec::new(),
        digest(45),
        digest(46),
        Vec::new(),
    );
    assert_eq!(manifest.meta().id().as_str(), "CHANGE-010");
    assert_eq!(manifest.meta().kind(), ObjectKind::ChangeManifest);
    assert_eq!(manifest.base_graph(), digest(45));
    assert_eq!(manifest.head_graph(), digest(46));
    assert_eq!(
        disposition_closure(&impacts, &manifest).len(),
        impacts.len()
    );
}

#[test]
fn disposition_closure_checks_duplicate_stale_shape_retired_waived_and_extra_rows() {
    let impact = Impact {
        kind: ObjectKind::Atom,
        id: sid(&String::from("ATOM-IMPACT")),
        base: Some(digest(1)),
        head: Some(digest(2)),
        direct: true,
    };
    let revised = test_change_disposition(
        sid("DISP-REVISED"),
        digest(3),
        Vec::new(),
        impact.kind,
        impact.id.clone(),
        impact.base,
        impact.head,
        ChangeDispositionKind::Revised,
    );
    let manifest = test_change_manifest(
        sid("MANIFEST-REVISED"),
        digest(4),
        Vec::new(),
        digest(5),
        digest(6),
        vec![revised.clone()],
    );
    assert!(disposition_closure(core::slice::from_ref(&impact), &manifest).is_empty());

    assert_eq!(
        ChangeManifest::new(
            sid("MANIFEST-DUPLICATE"),
            digest(5),
            digest(6),
            vec![revised.clone(), revised],
            None,
        ),
        Err(ConstructionError::Encoding(
            EncodingError::DuplicateSetValue
        ))
    );

    let stale = test_change_disposition(
        sid("DISP-STALE"),
        digest(8),
        Vec::new(),
        impact.kind,
        impact.id.clone(),
        Some(digest(9)),
        impact.head,
        ChangeDispositionKind::Reaffirmed,
    );
    let extraneous = test_change_disposition(
        sid("DISP-EXTRA"),
        digest(10),
        Vec::new(),
        ObjectKind::Oracle,
        sid("ORACLE-EXTRA"),
        Some(digest(11)),
        Some(digest(12)),
        ChangeDispositionKind::Reaffirmed,
    );
    let invalid = test_change_manifest(
        sid("MANIFEST-INVALID"),
        digest(13),
        Vec::new(),
        digest(5),
        digest(6),
        vec![stale, extraneous],
    );
    let invalid_codes = disposition_closure(core::slice::from_ref(&impact), &invalid)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert!(
        invalid_codes
            .iter()
            .any(|code| code == "CHG-DISPOSITION-STALE-OR-INAPPLICABLE")
    );
    assert!(
        invalid_codes
            .iter()
            .any(|code| code == "CHG-DISPOSITION-INAPPLICABLE")
    );

    let terminal_impacts = vec![
        Impact {
            kind: ObjectKind::Atom,
            id: sid(&String::from("ATOM-RETIRED")),
            base: Some(digest(14)),
            head: None,
            direct: true,
        },
        Impact {
            kind: ObjectKind::Atom,
            id: sid(&String::from("ATOM-WAIVED")),
            base: None,
            head: Some(digest(15)),
            direct: true,
        },
    ];
    let terminal_manifest = test_change_manifest(
        sid("MANIFEST-TERMINAL"),
        digest(16),
        Vec::new(),
        digest(17),
        digest(18),
        vec![
            test_change_disposition(
                sid("DISP-RETIRED"),
                digest(19),
                Vec::new(),
                ObjectKind::Atom,
                sid("ATOM-RETIRED"),
                Some(digest(14)),
                None,
                ChangeDispositionKind::Retired,
            ),
            test_change_disposition(
                sid("DISP-WAIVED"),
                digest(20),
                Vec::new(),
                ObjectKind::Atom,
                sid("ATOM-WAIVED"),
                None,
                Some(digest(15)),
                ChangeDispositionKind::Waived,
            ),
        ],
    );
    assert!(disposition_closure(&terminal_impacts, &terminal_manifest).is_empty());

    let equal_impact = Impact {
        kind: ObjectKind::Atom,
        id: sid(&String::from("ATOM-EQUAL")),
        base: Some(digest(21)),
        head: Some(digest(21)),
        direct: true,
    };
    assert_eq!(
        ChangeDisposition::new(
            sid("DISP-EQUAL"),
            ObjectKind::Atom,
            sid("ATOM-EQUAL"),
            equal_impact.base,
            equal_impact.head,
            ChangeDispositionKind::Reaffirmed,
            Some(digest(25)),
            None,
            Vec::new(),
        ),
        Err(ConstructionError::ChangeDispositionShape)
    );

    for (index, (base, head, kind)) in [
        (None, Some(digest(31)), ChangeDispositionKind::Revised),
        (Some(digest(32)), None, ChangeDispositionKind::Revised),
        (
            Some(digest(33)),
            Some(digest(34)),
            ChangeDispositionKind::Retired,
        ),
        (None, None, ChangeDispositionKind::Retired),
        (None, None, ChangeDispositionKind::Waived),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("ATOM-SHAPE-{index}");
        let reaffirmation = (kind == ChangeDispositionKind::Reaffirmed).then_some(digest(38));
        let waiver = (kind == ChangeDispositionKind::Waived)
            .then(|| test_object_ref(ObjectKind::Waiver, sid("WAIVER-SHAPE"), digest(38)));
        assert_eq!(
            ChangeDisposition::new(
                sid("DISP-SHAPE"),
                ObjectKind::Atom,
                sid(&id),
                base,
                head,
                kind,
                reaffirmation,
                waiver,
                Vec::new(),
            ),
            Err(ConstructionError::ChangeDispositionShape)
        );
    }
    let waived_base_impact = Impact {
        kind: ObjectKind::Atom,
        id: sid(&String::from("ATOM-WAIVED-BASE")),
        base: Some(digest(39)),
        head: None,
        direct: true,
    };
    let waived_base_manifest = test_change_manifest(
        sid("MANIFEST-WAIVED-BASE"),
        digest(40),
        Vec::new(),
        digest(41),
        digest(42),
        vec![test_change_disposition(
            sid("DISP-WAIVED-BASE"),
            digest(43),
            Vec::new(),
            ObjectKind::Atom,
            sid("ATOM-WAIVED-BASE"),
            Some(digest(39)),
            None,
            ChangeDispositionKind::Waived,
        )],
    );
    assert!(
        disposition_closure(
            core::slice::from_ref(&waived_base_impact),
            &waived_base_manifest
        )
        .is_empty()
    );
}

#[test]
fn reports_are_order_independent_and_static_identity_is_execution_independent() {
    let graph = digest(60);
    let report_a = execution_report(
        graph,
        vec![
            (String::from("ORC-B"), Verdict::Fail),
            (String::from("ORC-A"), Verdict::Pass),
        ],
        vec![AssuranceClass::Linked],
        Vec::new(),
    );
    let report_b = execution_report(
        graph,
        vec![
            (String::from("ORC-A"), Verdict::Pass),
            (String::from("ORC-B"), Verdict::Fail),
        ],
        vec![AssuranceClass::Linked],
        Vec::new(),
    );
    assert_eq!(report_a, report_b);
    assert_eq!(report_a.graph, graph);
}

#[test]
fn complete_atom_surface_parsed_item_and_source_map_values_are_canonical() {
    let approval = approval().expect("canonical approval");
    let live_atom = Atom::new(
        sid("ATOM-COMPLETE"),
        LifecycleState::Ratified,
        AtomKind::Behavioral,
        vec![
            ScopeId::new("scope.beta").expect("valid scope"),
            ScopeId::new("scope.alpha").expect("valid scope"),
        ],
        "The implementation MUST preserve complete values.",
        Some(approval.clone()),
    )
    .expect("complete atom");
    assert_eq!(live_atom.state(), LifecycleState::Ratified);
    assert_eq!(live_atom.atom_kind(), AtomKind::Behavioral);
    assert_eq!(
        live_atom.scopes(),
        &[
            ScopeId::new("scope.beta").expect("valid scope"),
            ScopeId::new("scope.alpha").expect("valid scope"),
        ]
    );
    assert_eq!(
        live_atom.revision(),
        semantic_digest(
            "atom-revision",
            vec![
                (1, CanonicalValue::Enum(AtomKind::Behavioral as u16, None)),
                (
                    2,
                    CanonicalValue::Set(vec![
                        CanonicalValue::Text(String::from("scope.beta")),
                        CanonicalValue::Text(String::from("scope.alpha")),
                    ]),
                ),
                (
                    3,
                    CanonicalValue::Text(String::from(
                        "The implementation MUST preserve complete values.",
                    )),
                ),
            ],
        )
        .expect("atom revision digest")
    );

    let replacement = test_atom_ref(sid("ATOM-REPLACEMENT"), digest(2));
    let terminal = Tombstone::new(
        sid("ATOM-COMPLETE"),
        LifecycleState::Superseded,
        live_atom.revision(),
        vec![replacement.clone()],
        approval.clone(),
    )
    .expect("complete tombstone");
    assert_eq!(terminal.approval(), &approval);
    assert_eq!(terminal.replacements(), std::slice::from_ref(&replacement));
    assert_eq!(
        terminal.meta().digest(),
        semantic_digest(
            "tombstone",
            vec![
                (1, CanonicalValue::Text(String::from("ATOM-COMPLETE"))),
                (
                    2,
                    CanonicalValue::Enum(LifecycleState::Superseded as u16, None),
                ),
                (3, CanonicalValue::Digest(live_atom.revision())),
                (
                    4,
                    CanonicalValue::Set(vec![CanonicalValue::Record(vec![
                        (1, CanonicalValue::Text(String::from("ATOM-REPLACEMENT"))),
                        (2, CanonicalValue::Digest(replacement.revision())),
                    ])]),
                ),
                (
                    5,
                    CanonicalValue::Record(vec![
                        (1, CanonicalValue::Text(String::from("maintainers"))),
                        (2, CanonicalValue::Text(String::from("reviews/approval.md")),),
                        (3, CanonicalValue::Digest(digest(90))),
                    ]),
                ),
            ],
        )
        .expect("tombstone digest")
    );

    let member = test_payload("member-key/v1", b"member".to_vec());
    let key_schema = test_payload("member-key-schema/v1", vec![1]);
    let vocabulary = test_payload("disposition-vocabulary/v1", vec![2]);
    let domain = DecisionSurfaceDomain::new(
        sid("SURFACE-COMPLETE"),
        key_schema.clone(),
        vocabulary.clone(),
        vec![member.clone()],
        3,
    )
    .expect("complete domain");
    assert_eq!(domain.surface_id().as_str(), "SURFACE-COMPLETE");
    assert_eq!(domain.member_key_schema(), &key_schema);
    assert_eq!(domain.disposition_vocabulary(), &vocabulary);
    assert_eq!(domain.member_keys(), std::slice::from_ref(&member));
    assert_eq!(domain.generation(), 3);
    assert_eq!(
        domain.digest(),
        semantic_digest(
            "decision-surface-domain",
            vec![
                (1, CanonicalValue::Text(String::from("SURFACE-COMPLETE"))),
                (
                    2,
                    CanonicalValue::Record(vec![
                        (
                            1,
                            CanonicalValue::Text(String::from("member-key-schema/v1")),
                        ),
                        (2, CanonicalValue::Bytes(vec![1])),
                    ]),
                ),
                (
                    3,
                    CanonicalValue::Record(vec![
                        (
                            1,
                            CanonicalValue::Text(String::from("disposition-vocabulary/v1",)),
                        ),
                        (2, CanonicalValue::Bytes(vec![2])),
                    ]),
                ),
                (
                    4,
                    CanonicalValue::Set(vec![CanonicalValue::Record(vec![
                        (1, CanonicalValue::Text(String::from("member-key/v1")),),
                        (2, CanonicalValue::Bytes(b"member".to_vec())),
                    ])]),
                ),
                (5, CanonicalValue::U64(3)),
            ],
        )
        .expect("decision-surface domain digest")
    );

    let controlling_atom = test_atom_ref(live_atom.meta().id().clone(), live_atom.revision());
    let disposition = test_surface_disposition(
        member,
        SurfaceDispositionKind::Normative,
        Some(test_payload("disposition/v1", vec![3])),
        Some(controlling_atom.clone()),
    );
    let disposition_value = CanonicalValue::Record(vec![
        (
            1,
            CanonicalValue::Record(vec![
                (1, CanonicalValue::Text(String::from("member-key/v1"))),
                (2, CanonicalValue::Bytes(b"member".to_vec())),
            ]),
        ),
        (
            2,
            CanonicalValue::Enum(SurfaceDispositionKind::Normative as u16, None),
        ),
        (
            3,
            CanonicalValue::Some(Box::new(CanonicalValue::Record(vec![
                (1, CanonicalValue::Text(String::from("disposition/v1"))),
                (2, CanonicalValue::Bytes(vec![3])),
            ]))),
        ),
        (
            4,
            CanonicalValue::Some(Box::new(CanonicalValue::Record(vec![
                (
                    1,
                    CanonicalValue::Text(controlling_atom.id().as_str().to_string()),
                ),
                (2, CanonicalValue::Digest(controlling_atom.revision())),
            ]))),
        ),
    ]);
    assert_eq!(
        disposition.digest().expect("surface disposition digest"),
        semantic_digest(
            "surface-disposition",
            match disposition_value.clone() {
                CanonicalValue::Record(fields) => fields,
                _ => Vec::new(),
            },
        )
        .expect("expected surface disposition digest")
    );
    let policy = test_object_ref(
        ObjectKind::CoveragePolicy,
        sid("POLICY-COMPLETE"),
        digest(4),
    );
    let surface = DecisionSurface::new(domain.clone(), vec![disposition], vec![policy.clone()])
        .expect("complete decision surface");
    assert_eq!(surface.domain(), &domain);
    assert_eq!(surface.domain_digest(), domain.digest());
    assert_eq!(surface.policies(), std::slice::from_ref(&policy));
    assert_eq!(surface.meta().references(), std::slice::from_ref(&policy));
    assert_eq!(
        surface.meta().digest(),
        semantic_digest(
            "decision-surface",
            vec![
                (1, CanonicalValue::Text(String::from("SURFACE-COMPLETE"))),
                (2, CanonicalValue::Digest(domain.digest())),
                (3, CanonicalValue::Set(vec![disposition_value.clone()])),
                (
                    4,
                    CanonicalValue::Set(vec![CanonicalValue::Record(vec![
                        (
                            1,
                            CanonicalValue::Enum(ObjectKind::CoveragePolicy as u16, None),
                        ),
                        (2, CanonicalValue::Text(policy.id().as_str().to_string())),
                        (3, CanonicalValue::Digest(policy.digest())),
                    ])]),
                ),
            ],
        )
        .expect("decision-surface digest")
    );

    let path = RepositoryPath::new("src/complete.rs").expect("canonical source path");
    let enclosing = ParsedItemIdentity::new(
        SchemaId::new("rust/v1").expect("valid schema"),
        path.clone(),
        SourceItemKind::new("rust/item/v1").expect("valid source item kind"),
        b"mod complete".to_vec(),
        None,
    )
    .expect("enclosing parsed item");
    let parsed = ParsedItem::new(
        SchemaId::new("rust/v1").expect("valid schema"),
        path.clone(),
        SourceItemKind::new("rust/item/v1").expect("valid source item kind"),
        b"fn complete() {}".to_vec(),
        Some(enclosing.digest()),
    )
    .expect("nested parsed item");
    assert_eq!(parsed.language_schema_id(), "rust/v1");
    assert_eq!(parsed.path(), &path);
    assert_eq!(parsed.kind().as_str(), "rust/item/v1");
    assert_eq!(parsed.semantic_bytes(), b"fn complete() {}");
    assert_eq!(parsed.enclosing_item(), Some(enclosing.digest()));
    assert_eq!(
        parsed.digest(),
        semantic_digest(
            "parsed-item-identity",
            vec![
                (1, CanonicalValue::Text(String::from("rust/v1"))),
                (2, CanonicalValue::Text(String::from("src/complete.rs")),),
                (3, CanonicalValue::Text(String::from("rust/item/v1"))),
                (4, CanonicalValue::Bytes(b"fn complete() {}".to_vec())),
                (
                    5,
                    CanonicalValue::Some(Box::new(CanonicalValue::Digest(enclosing.digest(),))),
                ),
            ],
        )
        .expect("parsed-item digest")
    );

    let source = SourceRef::new(path, 7, 23).expect("valid original-byte span");
    let mapped = test_object_ref(
        ObjectKind::Atom,
        live_atom.meta().id().clone(),
        live_atom.meta().digest(),
    );
    let entry = SourceMapEntry::new(mapped.clone(), vec![source.clone()], SourceRole::Authority)
        .expect("complete source-map entry");
    assert_eq!(entry.object(), &mapped);
    assert_eq!(entry.sources(), std::slice::from_ref(&source));
    assert_eq!(entry.role(), SourceRole::Authority);
    let entry_value = CanonicalValue::Record(vec![
        (
            1,
            CanonicalValue::Record(vec![
                (1, CanonicalValue::Enum(ObjectKind::Atom as u16, None)),
                (2, CanonicalValue::Text(mapped.id().as_str().to_string())),
                (3, CanonicalValue::Digest(mapped.digest())),
            ]),
        ),
        (
            2,
            CanonicalValue::Set(vec![CanonicalValue::Record(vec![
                (1, CanonicalValue::Text(source.path().as_str().to_string())),
                (2, CanonicalValue::U64(source.start())),
                (3, CanonicalValue::U64(source.end())),
            ])]),
        ),
        (3, CanonicalValue::Enum(SourceRole::Authority as u16, None)),
    ]);
    assert_eq!(
        entry.digest(),
        semantic_digest(
            "source-map-entry",
            match entry_value.clone() {
                CanonicalValue::Record(fields) => fields,
                _ => Vec::new(),
            },
        )
        .expect("source-map-entry digest")
    );
    let source_map = SourceMap::new(vec![entry.clone()]).expect("complete source map");
    assert_eq!(source_map.entries(), &[entry]);
    assert_eq!(
        source_map.digest(),
        semantic_digest(
            "source-map",
            vec![(1, CanonicalValue::Set(vec![entry_value]))],
        )
        .expect("source-map digest")
    );
}

#[test]
fn complete_surface_and_source_values_reject_invalid_local_shapes() {
    let key = test_payload("key/v1", vec![1]);
    let key_schema = test_payload("key-schema/v1", vec![1]);
    let vocabulary = test_payload("vocabulary/v1", vec![1]);
    assert_eq!(
        DecisionSurfaceDomain::new(
            sid("SURFACE-ZERO"),
            key_schema.clone(),
            vocabulary.clone(),
            vec![key.clone()],
            0,
        ),
        Err(ConstructionError::NonPositiveGeneration)
    );
    assert_eq!(
        DecisionSurfaceDomain::new(
            sid("SURFACE-EMPTY"),
            key_schema.clone(),
            vocabulary.clone(),
            Vec::new(),
            1,
        ),
        Err(ConstructionError::EmptySet)
    );
    assert_eq!(
        DecisionSurfaceDomain::new(
            sid("SURFACE-DUPLICATE"),
            key_schema,
            vocabulary,
            vec![key.clone(), key],
            1,
        ),
        Err(ConstructionError::Encoding(
            EncodingError::DuplicateSetValue
        ))
    );

    let path = RepositoryPath::new("src/span.rs").expect("canonical source path");
    assert_eq!(
        SourceRef::new(path.clone(), 2, 1),
        Err(ConstructionError::SourceSpan)
    );
    let source = SourceRef::new(path, 1, 2).expect("valid source span");
    let object = test_object_ref(ObjectKind::Atom, sid("ATOM-SPAN"), digest(5));
    assert_eq!(
        SourceMapEntry::new(object.clone(), Vec::new(), SourceRole::Authority),
        Err(ConstructionError::EmptySet)
    );
    assert_eq!(
        SourceMapEntry::new(
            object.clone(),
            vec![source.clone(), source.clone()],
            SourceRole::Authority,
        ),
        Err(ConstructionError::Encoding(
            EncodingError::DuplicateSetValue
        ))
    );
    let entry = SourceMapEntry::new(object, vec![source], SourceRole::Authority)
        .expect("valid source-map entry");
    assert_eq!(
        SourceMap::new(vec![entry.clone(), entry]),
        Err(ConstructionError::Encoding(
            EncodingError::DuplicateSetValue
        ))
    );
    let empty = SourceMap::new(Vec::new()).expect("explicit empty source map");
    assert!(empty.entries().is_empty());
    assert_eq!(
        empty.digest(),
        semantic_digest("source-map", vec![(1, CanonicalValue::Set(Vec::new()))],)
            .expect("empty source-map digest")
    );
}

#[test]
fn model_constructors_reject_each_public_invalid_shape() {
    assert_eq!(
        Atom::new(
            sid("ATOM-DUPLICATE-SCOPE"),
            LifecycleState::Proposed,
            AtomKind::Constraint,
            vec![
                ScopeId::new("scope").expect("valid scope"),
                ScopeId::new("scope").expect("valid scope"),
            ],
            "The system MUST behave.",
            None,
        ),
        Err(ConstructionError::Encoding(
            EncodingError::DuplicateSetValue
        ))
    );
    assert!(
        Atom::new(
            sid("ATOM-DISTINCT-SCOPES"),
            LifecycleState::Proposed,
            AtomKind::Constraint,
            vec![
                ScopeId::new("a").expect("valid scope"),
                ScopeId::new("b").expect("valid scope"),
            ],
            "The system MUST behave.",
            None,
        )
        .is_ok()
    );
    assert_eq!(
        Atom::new(
            sid("ATOM-NONCANONICAL-STATEMENT"),
            LifecycleState::Proposed,
            AtomKind::Constraint,
            vec![ScopeId::new("scope").expect("valid scope")],
            "e\u{301}",
            None,
        ),
        Err(ConstructionError::Encoding(EncodingError::NonCanonicalText))
    );
    assert_eq!(
        StableId::new("A\u{301}TOM-NONCANONICAL-ID")
            .err()
            .map(IdentifierError::code),
        Some("SCF-CONSTRUCT-STABLE-ID")
    );

    let path = RepositoryPath::new("artifacts/output.bin").expect("canonical path");
    for (domain, media_type, representation) in [
        ("wrong-domain", "application/octet-stream", "raw/v1"),
        ("raw-artifact", "", "raw/v1"),
        ("raw-artifact", "application/octet-stream", ""),
    ] {
        assert_eq!(
            RepositoryArtifactBinding::new(
                path.clone(),
                domain,
                media_type,
                representation,
                digest(220),
            ),
            Err(ConstructionError::ArtifactBindingShape)
        );
        assert_eq!(
            ArtifactEntry::new(
                path.clone(),
                domain,
                media_type,
                representation,
                digest(220),
            ),
            Err(ConstructionError::ArtifactBindingShape)
        );
    }
    let duplicate = ArtifactEntry::new(
        path.clone(),
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        digest(221),
    )
    .expect("valid entry");
    assert_eq!(
        ArtifactIndex::new(vec![duplicate.clone(), duplicate]),
        Err(ArtifactIndexError::DuplicatePath)
    );
    let upper = ArtifactEntry::new(
        RepositoryPath::new("artifacts/OUTPUT.bin").expect("canonical uppercase path"),
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        digest(222),
    )
    .expect("valid uppercase entry");
    let lower = ArtifactEntry::new(
        path.clone(),
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        digest(223),
    )
    .expect("valid lowercase entry");
    assert_eq!(
        ArtifactIndex::new(vec![upper, lower]),
        Err(ArtifactIndexError::PortableCollision)
    );
    let representation_mismatch = ArtifactIndex::new(vec![
        ArtifactEntry::new(
            path.clone(),
            "raw-artifact",
            "application/octet-stream",
            "other/v1",
            digest(223),
        )
        .expect("alternate representation entry"),
    ])
    .expect("representation index");
    let exact_binding = RepositoryArtifactBinding::new(
        path,
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        digest(223),
    )
    .expect("exact repository binding");
    assert!(!representation_mismatch.contains_binding(&exact_binding));

    let subject = test_object_ref(ObjectKind::Atom, sid("ATOM-SUBJECT"), digest(224));
    assert_eq!(
        ObligationRef::new(
            sid("POLICY-AUTHORITY"),
            ObligationKind::Authority,
            subject.clone(),
            None,
        )
        .expect("complete authority obligation")
        .kind(),
        ObligationKind::Authority
    );
    assert_eq!(
        ObligationRef::new(
            sid("POLICY-INVALID"),
            ObligationKind::RequiredRole,
            subject.clone(),
            None,
        ),
        Err(ConstructionError::ObligationRoleShape)
    );
    assert!(
        ObligationRef::new(
            sid("POLICY-AUTHORITY"),
            ObligationKind::Authority,
            subject.clone(),
            None,
        )
        .is_ok()
    );
    assert_eq!(
        ObligationRef::new(
            sid("POLICY-INVALID"),
            ObligationKind::Authority,
            subject,
            Some(EvidenceRole::Positive),
        ),
        Err(ConstructionError::ObligationRoleShape)
    );

    let oracle_ref = test_object_ref(ObjectKind::Oracle, sid("ORACLE-SHAPE"), digest(225));
    assert_eq!(
        ImplementationLink::new(
            sid("IMPLEMENTATION-EMPTY"),
            Vec::new(),
            digest(226),
            NavigationKind::Implements,
        ),
        Err(ConstructionError::EmptySet)
    );
    assert_eq!(
        Assumption::new(
            sid("ASSUMPTION-EMPTY"),
            "",
            test_payload("scope/v1", Vec::new()),
            Vec::new(),
            approval().expect("assumption approval"),
        ),
        Err(ConstructionError::EmptyText)
    );
    assert_eq!(
        FiniteModel::new(
            sid("MODEL-EMPTY"),
            test_payload("state-space/v1", Vec::new()),
            test_payload("events/v1", Vec::new()),
            test_payload("transition/v1", Vec::new()),
            Vec::new(),
            test_payload("bound/v1", vec![1]),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
        Err(ConstructionError::EmptySet)
    );
    assert_eq!(
        FormalProperty::new(
            sid("PROPERTY-EMPTY"),
            Vec::new(),
            test_payload("property/v1", Vec::new()),
            test_payload("abstraction/v1", Vec::new()),
            Vec::new(),
            Vec::new(),
            approval().expect("property approval"),
        ),
        Err(ConstructionError::EmptySet)
    );
    assert_eq!(
        NormalizedVerdict::new(
            oracle_ref,
            Vec::new(),
            Verdict::NotRun,
            Some(test_payload("result/v1", Vec::new())),
        ),
        Err(ConstructionError::VerdictResultShape)
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn constructor_diagnostics_cover_grammar_nfc_kind_scope_options_and_local_invariants() {
    assert_eq!(
        StableId::new("lower-ID").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-STABLE-ID")
    );
    assert_eq!(
        StableId::new("A\u{301}TOM-ID")
            .err()
            .map(IdentifierError::code),
        Some("SCF-CONSTRUCT-STABLE-ID")
    );
    assert_eq!(
        ScopeId::new("Maintainers").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_eq!(
        SchemaId::new("Bad_Schema").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCHEMA-ID")
    );
    assert_eq!(
        ScopeId::new("Runner").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_eq!(
        ToolIdentity::new(
            ScopeId::new("runner").expect("valid scope"),
            "",
            digest(3),
            digest(4)
        ),
        Err(ConstructionError::EmptyText)
    );

    assert_eq!(
        Atom::new(
            sid("ATOM-NONNORMATIVE"),
            LifecycleState::Proposed,
            AtomKind::Constraint,
            vec![ScopeId::new("scope").expect("valid scope")],
            "No normative keyword is present.",
            None,
        ),
        Err(ConstructionError::NormativeStatement)
    );
    assert_eq!(
        ScopeId::new("Bad_Scope").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCOPE-ID")
    );
    assert_eq!(
        Tombstone::new(
            sid("ATOM-SELF-REPLACED"),
            LifecycleState::Superseded,
            digest(5),
            vec![test_atom_ref(sid("ATOM-SELF-REPLACED"), digest(6))],
            approval().expect("terminal approval"),
        ),
        Err(ConstructionError::LocalInvariant)
    );
    assert_eq!(
        SchemaId::new("Bad_Adapter")
            .err()
            .map(IdentifierError::code),
        Some("SCF-CONSTRUCT-SCHEMA-ID")
    );
    assert_eq!(
        SurfaceDisposition::new(
            test_payload("key/v1", vec![1]),
            SurfaceDispositionKind::Deferred,
            Some(test_payload("reason/v1", vec![1])),
            None,
        ),
        Err(ConstructionError::LocalInvariant)
    );

    let source = SourceRef::new(
        RepositoryPath::new("spec/source.md").expect("source path"),
        0,
        1,
    )
    .expect("source span");
    assert_eq!(
        SourceMapEntry::new(
            test_object_ref(ObjectKind::Oracle, sid("ORACLE-SOURCE"), digest(7)),
            vec![source],
            SourceRole::Authority,
        ),
        Err(ConstructionError::ReferenceKind)
    );
    assert_eq!(
        NormalizedVerdict::new(
            test_object_ref(ObjectKind::Atom, sid("ATOM-NOT-ORACLE"), digest(8)),
            Vec::new(),
            Verdict::NotRun,
            None,
        ),
        Err(ConstructionError::ReferenceKind)
    );

    let oracle = test_object_ref(ObjectKind::Oracle, sid("ORACLE-SCOPE"), digest(9));
    assert_eq!(
        EvidenceCarrier::new(
            sid("CARRIER-SCOPE"),
            EvidenceRole::Positive,
            vec![test_atom_ref(sid("ATOM-SCOPE"), digest(10))],
            digest(11),
            oracle.clone(),
            digest(12),
            Vec::new(),
            test_payload("spec-provenance.scope.quantified-property/v1", Vec::new()),
            Vec::new(),
            Vec::new(),
        ),
        Err(ConstructionError::RoleScopeShape)
    );
    assert_eq!(
        FiniteModel::new(
            sid("MODEL-WRONG-ASSUMPTION"),
            test_payload("state-space/v1", Vec::new()),
            test_payload("events/v1", Vec::new()),
            test_payload("transition/v1", Vec::new()),
            vec![test_payload("state/v1", vec![0])],
            test_payload("bound/v1", vec![1]),
            Vec::new(),
            vec![test_object_ref(
                ObjectKind::Oracle,
                sid("ORACLE-WRONG"),
                digest(13)
            )],
            Vec::new(),
        ),
        Err(ConstructionError::ReferenceKind)
    );
    assert_eq!(
        Derivation::new(
            sid("DERIVATION-BYTE-EQUAL"),
            vec![
                ArtifactBinding::Object(test_object_ref(
                    ObjectKind::Atom,
                    sid("ATOM-A"),
                    digest(14)
                )),
                ArtifactBinding::Object(test_object_ref(
                    ObjectKind::Atom,
                    sid("ATOM-B"),
                    digest(15)
                )),
            ],
            vec![ArtifactBinding::Object(test_object_ref(
                ObjectKind::Atom,
                sid("ATOM-C"),
                digest(16),
            ))],
            test_tool("generator", "1", digest(17), digest(18)),
            digest(19),
            oracle,
            DerivationRelation::ByteEqual,
            test_payload("derivation-scope/v1", Vec::new()),
            Vec::new(),
        ),
        Err(ConstructionError::LocalInvariant)
    );

    let first = ChangeDisposition::new(
        sid("DISPOSITION-FIRST"),
        ObjectKind::Atom,
        sid("ATOM-IMPACT"),
        Some(digest(20)),
        Some(digest(21)),
        ChangeDispositionKind::Revised,
        None,
        None,
        Vec::new(),
    )
    .expect("first disposition");
    let second = ChangeDisposition::new(
        sid("DISPOSITION-SECOND"),
        ObjectKind::Atom,
        sid("ATOM-IMPACT"),
        Some(digest(20)),
        Some(digest(21)),
        ChangeDispositionKind::Revised,
        None,
        None,
        Vec::new(),
    )
    .expect("second disposition");
    assert_eq!(
        ChangeManifest::new(
            sid("CHANGE-EQUAL-GRAPHS"),
            digest(22),
            digest(22),
            vec![first.clone()],
            None,
        ),
        Err(ConstructionError::LocalInvariant)
    );
    assert_eq!(
        ChangeManifest::new(
            sid("CHANGE-DUPLICATE-IMPACT"),
            digest(22),
            digest(23),
            vec![first, second],
            None,
        ),
        Err(ConstructionError::LocalInvariant)
    );

    for (error, code) in [
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
    ] {
        assert_eq!(error.code(), code);
    }
    assert_eq!(
        ArtifactIndexError::DuplicatePath.code(),
        "SCF-ARTIFACT-DUPLICATE-PATH"
    );
}

#[test]
fn freshness_and_reference_predicates_exercise_each_short_circuit_axis() {
    let Some(atom) = atom("ATOM-LIVE", "Live atom.") else {
        return;
    };
    let oracle = test_oracle(sid("ORACLE-LIVE"), digest(230), Vec::new());
    let atom_ref = test_atom_ref(atom.meta().id().clone(), atom.revision());
    let oracle_ref = test_object_ref(
        ObjectKind::Oracle,
        oracle.meta().id().clone(),
        oracle.meta().digest(),
    );
    let graph = SpecGraph {
        atoms: vec![atom.clone()],
        oracles: vec![oracle.clone()],
        ..SpecGraph::default()
    };
    let carrier = |id: &str, atoms: Vec<AtomRevisionRef>, oracle: ObjectRef| {
        test_carrier(
            sid(id),
            digest(231),
            Vec::new(),
            EvidenceRole::Positive,
            atoms,
            oracle,
            digest(232),
            Vec::new(),
            test_payload("scope/v1", Vec::new()),
        )
    };
    assert_eq!(
        EvidenceCarrier::new(
            sid("CARRIER-EMPTY"),
            EvidenceRole::Positive,
            Vec::new(),
            digest(231),
            oracle_ref.clone(),
            digest(232),
            Vec::new(),
            test_payload("scope/v1", Vec::new()),
            Vec::new(),
            Vec::new(),
        ),
        Err(ConstructionError::EmptySet)
    );
    assert!(!carrier_fresh(
        &graph,
        &carrier(
            "CARRIER-STALE-ATOM",
            vec![test_atom_ref(atom.meta().id().clone(), digest(233))],
            oracle_ref.clone(),
        )
    ));
    assert!(!carrier_fresh(
        &graph,
        &carrier(
            "CARRIER-MISSING-ATOM",
            vec![test_atom_ref(sid("ATOM-MISSING"), digest(233))],
            oracle_ref.clone(),
        )
    ));
    assert!(!carrier_fresh(
        &graph,
        &carrier(
            "CARRIER-STALE-ORACLE",
            vec![atom_ref.clone()],
            test_object_ref(ObjectKind::Oracle, oracle.meta().id().clone(), digest(234)),
        )
    ));
    assert!(carrier_fresh(
        &graph,
        &carrier("CARRIER-LIVE", vec![atom_ref.clone()], oracle_ref.clone(),)
    ));

    let source = ArtifactBinding::Object(test_object_ref(
        ObjectKind::Atom,
        atom.meta().id().clone(),
        atom.meta().digest(),
    ));
    let target = ArtifactBinding::Object(oracle_ref.clone());
    let tool = test_tool("tool", "1", digest(235), digest(236));
    let derivation = |id: &str,
                      sources: Vec<ArtifactBinding>,
                      targets: Vec<ArtifactBinding>,
                      oracle: ObjectRef| {
        test_derivation(
            sid(id),
            digest(237),
            Vec::new(),
            sources,
            targets,
            tool.clone(),
            oracle,
        )
    };
    assert_eq!(
        Derivation::new(
            sid("DERIVATION-NO-SOURCE"),
            Vec::new(),
            vec![target.clone()],
            tool.clone(),
            digest(237),
            oracle_ref.clone(),
            DerivationRelation::GeneratedFrom,
            test_payload("derivation-scope/v1", Vec::new()),
            Vec::new(),
        ),
        Err(ConstructionError::EmptySet)
    );
    assert!(!derivation_fresh(
        &graph,
        &derivation(
            "DERIVATION-STALE-BINDING",
            vec![ArtifactBinding::Object(test_object_ref(
                ObjectKind::Atom,
                atom.meta().id().clone(),
                digest(238),
            ))],
            vec![target.clone()],
            oracle_ref.clone(),
        )
    ));
    assert!(!derivation_fresh(
        &graph,
        &derivation(
            "DERIVATION-STALE-ORACLE",
            vec![source.clone()],
            vec![target.clone()],
            test_object_ref(ObjectKind::Oracle, oracle.meta().id().clone(), digest(239)),
        )
    ));
    assert_eq!(
        Derivation::new(
            sid("DERIVATION-SAME-ENDS"),
            vec![source.clone()],
            vec![source.clone()],
            tool.clone(),
            digest(237),
            oracle_ref.clone(),
            DerivationRelation::GeneratedFrom,
            test_payload("derivation-scope/v1", Vec::new()),
            Vec::new(),
        ),
        Err(ConstructionError::OverlappingSets)
    );
    let live_derivation = derivation(
        "DERIVATION-LIVE",
        vec![source.clone()],
        vec![target.clone()],
        oracle_ref.clone(),
    );
    assert!(derivation_fresh(&graph, &live_derivation));
    let mut graph_with_derivation = graph.clone();
    graph_with_derivation.derivations = vec![live_derivation];
    assert!(
        !validate_graph(&graph_with_derivation)
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-DERIVATION-STALE")
    );

    let implementation =
        test_implementation_link(sid("IMPLEMENTATION-LIVE"), digest(248), Vec::new());
    let assumption = test_assumption(sid("ASSUMPTION-LIVE"), digest(249), Vec::new());
    let property = test_formal_property(
        sid("PROPERTY-LIVE"),
        digest(250),
        Vec::new(),
        vec![atom_ref.clone()],
        test_payload("property/v1", vec![1]),
        true,
    );
    let property_ref = test_object_ref(
        ObjectKind::FormalProperty,
        property.meta().id().clone(),
        property.meta().digest(),
    );
    let implementation_binding = ArtifactBinding::Object(test_object_ref(
        ObjectKind::ImplementationLink,
        implementation.meta().id().clone(),
        implementation.meta().digest(),
    ));
    let assumption_ref = test_object_ref(
        ObjectKind::Assumption,
        assumption.meta().id().clone(),
        assumption.meta().digest(),
    );
    let proof = |id: &str,
                 atoms: Vec<AtomRevisionRef>,
                 property: ObjectRef,
                 implementation: ArtifactBinding,
                 assumptions: Vec<ObjectRef>,
                 proof_sources: Vec<ArtifactBinding>,
                 verdict: Verdict| {
        test_proof_record(
            sid(id),
            digest(251),
            Vec::new(),
            atoms,
            property,
            implementation,
            assumptions,
            Vec::new(),
            tool.clone(),
            Vec::new(),
            Vec::new(),
            proof_sources,
            verdict,
        )
    };
    let valid_proof = proof(
        "PROOF-LIVE",
        vec![atom_ref.clone()],
        property_ref.clone(),
        implementation_binding.clone(),
        vec![assumption_ref.clone()],
        vec![implementation_binding.clone()],
        Verdict::Pass,
    );
    let proof_graph = SpecGraph {
        atoms: vec![atom.clone()],
        implementation_links: vec![implementation.clone()],
        formal_properties: vec![property.clone()],
        assumptions: vec![assumption.clone()],
        proof_records: vec![valid_proof.clone()],
        ..SpecGraph::default()
    };
    assert!(proof_fresh(&proof_graph, &valid_proof));
    assert_eq!(
        ProofRecord::new(
            sid("PROOF-NO-ATOMS"),
            Vec::new(),
            property_ref.clone(),
            implementation_binding.clone(),
            vec![assumption_ref.clone()],
            Vec::new(),
            tool.clone(),
            digest(251),
            Vec::new(),
            Vec::new(),
            vec![implementation_binding.clone()],
            Verdict::Pass,
            test_payload("proof-scope/v1", Vec::new()),
            Vec::new(),
        ),
        Err(ConstructionError::EmptySet)
    );
    let stale_inputs = vec![
        proof(
            "PROOF-STALE-ATOM",
            vec![test_atom_ref(atom.meta().id().clone(), digest(252))],
            property_ref.clone(),
            implementation_binding.clone(),
            vec![assumption_ref.clone()],
            vec![implementation_binding.clone()],
            Verdict::Pass,
        ),
        proof(
            "PROOF-STALE-PROPERTY",
            vec![atom_ref.clone()],
            test_object_ref(
                ObjectKind::FormalProperty,
                property.meta().id().clone(),
                digest(253),
            ),
            implementation_binding.clone(),
            vec![assumption_ref.clone()],
            vec![implementation_binding.clone()],
            Verdict::Pass,
        ),
        proof(
            "PROOF-STALE-IMPLEMENTATION",
            vec![atom_ref.clone()],
            property_ref.clone(),
            ArtifactBinding::Object(test_object_ref(
                ObjectKind::ImplementationLink,
                implementation.meta().id().clone(),
                digest(254),
            )),
            vec![assumption_ref.clone()],
            vec![implementation_binding.clone()],
            Verdict::Pass,
        ),
        proof(
            "PROOF-STALE-ASSUMPTION",
            vec![atom_ref.clone()],
            property_ref.clone(),
            implementation_binding.clone(),
            vec![test_object_ref(
                ObjectKind::Assumption,
                assumption.meta().id().clone(),
                digest(255),
            )],
            vec![implementation_binding.clone()],
            Verdict::Pass,
        ),
        proof(
            "PROOF-STALE-SOURCE",
            vec![atom_ref.clone()],
            property_ref.clone(),
            implementation_binding.clone(),
            vec![assumption_ref.clone()],
            vec![ArtifactBinding::Object(test_object_ref(
                ObjectKind::ImplementationLink,
                implementation.meta().id().clone(),
                digest(200),
            ))],
            Verdict::Pass,
        ),
        proof(
            "PROOF-FAIL",
            vec![atom_ref.clone()],
            property_ref.clone(),
            implementation_binding.clone(),
            vec![assumption_ref.clone()],
            vec![implementation_binding.clone()],
            Verdict::Fail,
        ),
    ];
    for stale in &stale_inputs {
        assert!(!proof_fresh(&proof_graph, stale));
    }
    let assumption_live = test_object_ref(
        ObjectKind::Assumption,
        assumption.meta().id().clone(),
        assumption.meta().digest(),
    );
    let referenced = SpecGraph {
        atoms: vec![atom],
        oracles: vec![oracle],
        assumptions: vec![assumption.clone()],
        finite_models: vec![
            test_finite_model(
                sid("MODEL-VALID"),
                digest(240),
                vec![assumption_live.clone()],
            ),
            test_finite_model(
                sid("MODEL-WRONG-ID"),
                digest(242),
                vec![test_object_ref(
                    ObjectKind::Assumption,
                    sid("ASSUMPTION-MISSING"),
                    assumption.meta().digest(),
                )],
            ),
            test_finite_model(
                sid("MODEL-WRONG-DIGEST"),
                digest(243),
                vec![test_object_ref(
                    ObjectKind::Assumption,
                    assumption.meta().id().clone(),
                    digest(244),
                )],
            ),
        ],
        ..SpecGraph::default()
    };
    let codes = validate_graph(&referenced)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert!(codes.iter().any(|code| code == "SPG-REF-DANGLING-OR-STALE"));
    for invalid_id in ["lower-ID", "NOHYPHEN", "BAD-_"] {
        assert_eq!(
            StableId::new(invalid_id).err().map(IdentifierError::code),
            Some("SCF-CONSTRUCT-STABLE-ID")
        );
    }
}

#[test]
fn graph_validation_reports_each_malformed_public_shape() {
    let Some(review_approval) = approval() else {
        return;
    };
    assert_eq!(
        Atom::new(
            sid("BAD-ATOM"),
            LifecycleState::Proposed,
            AtomKind::Constraint,
            vec![ScopeId::new("scope").expect("valid scope")],
            "Proposed atoms MUST NOT carry approval.",
            Some(review_approval),
        ),
        Err(ConstructionError::LifecycleShape)
    );
    assert_eq!(
        Atom::new(
            sid("RATIFIED-NO-APPROVAL"),
            LifecycleState::Ratified,
            AtomKind::Constraint,
            vec![ScopeId::new("scope").expect("valid scope")],
            "Ratified atoms MUST carry approval.",
            None,
        ),
        Err(ConstructionError::LifecycleShape)
    );
    assert_eq!(
        Atom::new(
            sid("TERMINAL-ATOM"),
            LifecycleState::Withdrawn,
            AtomKind::Constraint,
            vec![ScopeId::new("scope").expect("valid scope")],
            "Terminal states MUST use tombstones.",
            None,
        ),
        Err(ConstructionError::LifecycleShape)
    );
    let reused = Atom::new(
        sid("REUSED-ID"),
        LifecycleState::Proposed,
        AtomKind::Constraint,
        vec![ScopeId::new("scope").expect("valid scope")],
        "Tombstoned IDs MUST stay reserved.",
        None,
    )
    .expect("valid proposed reused-ID control");
    let dangling = test_object_ref(ObjectKind::Oracle, sid("ORC-MISSING"), digest(201));
    let invalid_link =
        test_implementation_link(sid("LINK-DUPLICATE"), digest(202), vec![dangling.clone()]);
    let duplicate_link =
        test_implementation_link(sid("LINK-DUPLICATE"), digest(202), vec![dangling.clone()]);
    let reused_tombstone = tombstone(
        "REUSED-ID",
        LifecycleState::Withdrawn,
        reused.revision(),
        Vec::new(),
    );
    assert_eq!(
        Tombstone::new(
            sid("TOMB-PROPOSED"),
            LifecycleState::Proposed,
            digest(205),
            Vec::new(),
            approval().expect("terminal approval"),
        ),
        Err(ConstructionError::LifecycleShape)
    );
    assert_eq!(
        Tombstone::new(
            sid("TOMB-SUPERSEDED"),
            LifecycleState::Superseded,
            digest(207),
            Vec::new(),
            approval().expect("terminal approval"),
        ),
        Err(ConstructionError::LifecycleShape)
    );
    let incomplete_surface = decision_surface(
        "SURFACE-INCOMPLETE",
        vec![test_payload("surface-key/v1", vec![1])],
        Vec::new(),
    );
    let stale_carrier = test_carrier(
        sid("CARRIER-STALE"),
        digest(210),
        Vec::new(),
        EvidenceRole::Positive,
        vec![test_atom_ref(sid("ATOM-MISSING"), digest(209))],
        dangling.clone(),
        digest(211),
        Vec::new(),
        test_payload("scope/v1", Vec::new()),
    );
    let stale_derivation = test_derivation(
        sid("DERIVATION-STALE"),
        digest(212),
        Vec::new(),
        vec![ArtifactBinding::Object(dangling.clone())],
        vec![ArtifactBinding::Object(test_object_ref(
            ObjectKind::Oracle,
            sid("ORC-OTHER"),
            digest(215),
        ))],
        test_tool("tool", "1", digest(213), digest(214)),
        dangling,
    );
    let graph = SpecGraph {
        atoms: vec![reused],
        tombstones: vec![reused_tombstone],
        decision_surfaces: vec![incomplete_surface],
        evidence_carriers: vec![stale_carrier],
        implementation_links: vec![invalid_link, duplicate_link],
        derivations: vec![stale_derivation],
        ..SpecGraph::default()
    };

    let codes = validate_graph(&graph)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    for expected in [
        "DET-DUP-OBJECT-KEY",
        "EVD-FRESH-CARRIER",
        "SPG-AUTH-RETIRED-ID-REUSE",
        "SPG-DERIVATION-STALE",
        "SPG-REF-DANGLING-OR-STALE",
        "SPG-SURFACE-INCOMPLETE",
    ] {
        assert!(
            codes.iter().any(|code| code == expected),
            "missing {expected}"
        );
    }
}
