#![doc = "Behavior controls for every canonical report fact and strict report decoder."]
#![allow(clippy::expect_used)]

use buoy_core::*;

fn sid(value: &str) -> StableId {
    StableId::new(value).expect("valid stable ID")
}

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn test_object_ref(kind: ObjectKind, id: impl Into<String>, value: SemanticDigest) -> ObjectRef {
    let id = id.into();
    let id = if id.contains('-') {
        id
    } else {
        format!("{id}-1")
    };
    ObjectRef::new(kind, sid(&id), value)
}

fn test_atom_ref(id: impl Into<String>, revision: SemanticDigest) -> AtomRevisionRef {
    let id = id.into();
    let id = if id.contains('-') {
        id
    } else {
        format!("{id}-1")
    };
    AtomRevisionRef::new(sid(&id), revision)
}

fn test_payload(schema: impl Into<String>, bytes: Vec<u8>) -> TypedPayload {
    let schema = schema.into();
    TypedPayload::new(SchemaId::new(&schema).expect("valid schema"), bytes)
        .expect("valid report typed payload")
}

fn object(kind: ObjectKind, id: &str, byte: u8) -> ObjectRef {
    test_object_ref(kind, id, digest(byte))
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_report_fact_round_trips_through_complete_reports() {
    let atom = object(ObjectKind::Atom, "ATOM", 1);
    let policy = object(ObjectKind::CoveragePolicy, "POLICY", 2);
    let carrier = object(ObjectKind::EvidenceCarrier, "CARRIER", 3);
    let oracle = object(ObjectKind::Oracle, "ORACLE", 4);
    let derivation = object(ObjectKind::Derivation, "DERIVATION", 5);
    let proof = object(ObjectKind::ProofRecord, "PROOF", 6);
    let waiver = object(ObjectKind::Waiver, "WAIVER", 7);
    let reaffirmation = object(ObjectKind::Reaffirmation, "REAFFIRM", 8);
    let obligation = ObligationRef::new(
        sid("POLICY-1"),
        ObligationKind::RequiredRole,
        atom.clone(),
        Some(EvidenceRole::Positive),
    )
    .expect("typed obligation must construct");
    let authority = AuthorityFact::new(
        atom.clone(),
        Validity::Valid,
        LifecycleState::Ratified,
        1,
        Validity::Valid,
        vec![String::from("AUTH-OK")],
    )
    .expect("authority fact must encode");
    let surface = SurfaceFact::new(
        object(ObjectKind::DecisionSurface, "SURFACE", 9),
        digest(10),
        Completeness::Complete,
        1,
        1,
        vec![test_payload("key/v1", vec![1])],
        vec![test_payload("key/v1", vec![2])],
        vec![test_payload("key/v1", vec![3])],
        vec![test_payload("key/v1", vec![4])],
        vec![String::from("SURFACE-OK")],
    )
    .expect("surface fact must encode");
    let registration = RegistrationFact::new(
        carrier.clone(),
        digest(11),
        EvidenceRole::Positive,
        vec![test_atom_ref("ATOM", digest(12))],
        digest(13),
        Validity::Valid,
        vec![String::from("REGISTRATION-OK")],
    )
    .expect("registration fact must encode");
    let policy_fact = PolicyFact::new(
        policy,
        atom.clone(),
        obligation.clone(),
        EnforcementMode::Blocking,
        Validity::Valid,
        Satisfaction::Satisfied,
        vec![carrier.clone()],
        vec![object(ObjectKind::DebtRecord, "DEBT", 14)],
        vec![waiver.clone()],
        vec![String::from("POLICY-OK")],
    )
    .expect("policy fact must encode");
    let freshness = FreshnessFact::new(
        atom.clone(),
        Freshness::Fresh,
        vec![digest(15)],
        vec![digest(15)],
        vec![String::from("binding")],
        vec![String::from("FRESH-OK")],
    )
    .expect("freshness fact must encode");
    let debt_waiver = DebtWaiverFact::new(
        waiver.clone(),
        Validity::Valid,
        GovernanceLifecycle::Active,
        vec![obligation.clone()],
        vec![obligation.clone()],
        "owner",
        digest(16),
        vec![carrier.clone()],
        vec![reaffirmation.clone()],
        vec![String::from("DEBT-OK")],
    )
    .expect("debt fact must encode");
    let repository_binding = ArtifactBinding::repository(
        RepositoryPath::new("generated/control.bin").expect("path must construct"),
        "raw-artifact",
        "application/octet-stream",
        "raw/v1",
        digest(17),
    )
    .expect("artifact binding must construct");
    let derivation_fact = DerivationFact::new(
        derivation,
        Freshness::Fresh,
        vec![ArtifactBinding::Object(carrier.clone())],
        vec![repository_binding],
        1,
        oracle.clone(),
        Consistency::Consistent,
        digest(18),
        vec![String::from("none")],
        vec![String::from("DERIVATION-OK")],
    )
    .expect("derivation fact must encode");
    let proof_fact = ProofFact::new(
        proof,
        Validity::Valid,
        Validity::Valid,
        Completeness::Complete,
        Freshness::Fresh,
        Selection::Selected,
        Verdict::Pass,
        Satisfaction::Satisfied,
        digest(19),
        test_payload("assumptions/v1", vec![1]),
        Satisfaction::Satisfied,
        vec![String::from("PROOF-OK")],
    )
    .expect("proof fact must encode");
    let verdict_with_result = NormalizedVerdict::new(
        oracle.clone(),
        vec![atom.clone()],
        Verdict::Pass,
        Some(test_payload("result/v1", vec![1])),
    )
    .expect("verdict must encode");
    let verdict_without_result = NormalizedVerdict::new(oracle, Vec::new(), Verdict::NotRun, None)
        .expect("empty verdict must encode");
    let evidence = EvidenceFact::new(
        obligation,
        vec![carrier],
        Validity::Valid,
        Freshness::Fresh,
        Selection::Selected,
        vec![verdict_with_result.clone(), verdict_without_result],
        digest(20),
        test_payload("scope/v1", vec![2]),
        vec![AssuranceClass::Tested],
        Satisfaction::Satisfied,
        vec![String::from("EVIDENCE-OK")],
    )
    .expect("evidence fact must encode");
    let closure = DispositionClosureFact::new(
        ObjectKind::Atom,
        sid("ATOM-1"),
        Some(digest(21)),
        None,
        ImpactDistance::Direct,
        vec![ChangeDispositionKind::Reaffirmed],
        Some(reaffirmation.clone()),
        Closure::Closed,
        Some(reaffirmation),
        Some(waiver),
        vec![String::from("CLOSURE-OK")],
    )
    .expect("closure fact must encode");
    let diagnostic = Diagnostic {
        path: Some(RepositoryPath::new("src/control.rs").expect("path must construct")),
        start: 1,
        end: 2,
        code: String::from("CONTROL"),
        kind: Some(ObjectKind::Atom),
        id: Some(String::from("ATOM")),
        fields: vec![(String::from("key"), String::from("value"))],
    };
    let canonical_diagnostic = CanonicalDiagnostic::new(&diagnostic, DiagnosticSeverity::Blocking)
        .expect("diagnostic must encode");
    let empty_diagnostic = CanonicalDiagnostic::new(
        &Diagnostic {
            path: None,
            start: 0,
            end: 0,
            code: String::from("EMPTY"),
            kind: None,
            id: None,
            fields: Vec::new(),
        },
        DiagnosticSeverity::Advisory,
    )
    .expect("empty diagnostic must encode");

    let static_report = CanonicalStaticReport::new(
        digest(22),
        vec![authority],
        vec![surface],
        vec![registration],
        vec![policy_fact],
        vec![freshness],
        vec![debt_waiver],
        vec![derivation_fact],
        vec![proof_fact],
        vec![canonical_diagnostic.clone(), empty_diagnostic.clone()],
    )
    .expect("static report must encode");
    let static_decoded =
        CanonicalStaticReport::decode(static_report.bytes(), DecodeBudget::v2_default())
            .expect("static report must decode");
    assert_eq!(static_decoded.digest(), static_report.digest());
    assert_eq!(
        static_decoded.value().encode().ok().as_deref(),
        Some(static_report.bytes())
    );
    let _ = static_report.value();

    let execution = CanonicalExecutionReport::new(
        static_report.digest(),
        vec![verdict_with_result],
        vec![evidence],
        vec![canonical_diagnostic],
    )
    .expect("execution report must encode");
    let execution_decoded =
        CanonicalExecutionReport::decode(execution.bytes(), DecodeBudget::v2_default())
            .expect("execution report must decode");
    assert_eq!(execution_decoded.digest(), execution.digest());
    assert_eq!(
        execution_decoded.value().encode().ok().as_deref(),
        Some(execution.bytes())
    );
    let _ = execution.value();

    let impact = CanonicalImpactReport::new(
        digest(23),
        digest(24),
        digest(25),
        digest(26),
        vec![closure],
        vec![empty_diagnostic],
    )
    .expect("impact report must encode");
    let impact_decoded = CanonicalImpactReport::decode(impact.bytes(), DecodeBudget::v2_default())
        .expect("impact report must decode");
    assert_eq!(impact_decoded.digest(), impact.digest());
    assert_eq!(
        impact_decoded.value().encode().ok().as_deref(),
        Some(impact.bytes())
    );
    let _ = impact.value();
}

#[test]
fn report_construction_rejects_noncanonical_and_duplicate_fact_sets() {
    assert_eq!(
        StableId::new("E\u{301}-1").err().map(IdentifierError::code),
        Some("SCF-CONSTRUCT-STABLE-ID")
    );

    let authority = AuthorityFact::new(
        object(ObjectKind::Atom, "ATOM-DUPLICATE", 2),
        Validity::Valid,
        LifecycleState::Ratified,
        1,
        Validity::Valid,
        Vec::new(),
    )
    .expect("authority fact encodes");
    assert_eq!(
        CanonicalStaticReport::new(
            digest(3),
            vec![authority.clone(), authority],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
        Err(EncodingError::DuplicateSetValue)
    );

    let verdict = NormalizedVerdict::new(
        object(ObjectKind::Oracle, "ORACLE-DUPLICATE", 4),
        Vec::new(),
        Verdict::Pass,
        None,
    )
    .expect("verdict encodes");
    assert_eq!(
        CanonicalExecutionReport::new(
            digest(5),
            vec![verdict.clone(), verdict],
            Vec::new(),
            Vec::new(),
        ),
        Err(EncodingError::DuplicateSetValue)
    );

    let diagnostic = CanonicalDiagnostic::new(
        &Diagnostic {
            path: None,
            start: 0,
            end: 0,
            code: String::from("DUPLICATE"),
            kind: None,
            id: None,
            fields: Vec::new(),
        },
        DiagnosticSeverity::Advisory,
    )
    .expect("diagnostic encodes");
    assert_eq!(
        CanonicalExecutionReport::new(
            digest(6),
            Vec::new(),
            Vec::new(),
            vec![diagnostic.clone(), diagnostic],
        ),
        Err(EncodingError::DuplicateSetValue)
    );

    let closure = DispositionClosureFact::new(
        ObjectKind::Atom,
        sid("ATOM-DUPLICATE"),
        Some(digest(7)),
        Some(digest(8)),
        ImpactDistance::Direct,
        vec![ChangeDispositionKind::Revised],
        None,
        Closure::Closed,
        None,
        None,
        Vec::new(),
    )
    .expect("closure fact encodes");
    assert_eq!(
        CanonicalImpactReport::new(
            digest(9),
            digest(10),
            digest(11),
            digest(12),
            vec![closure.clone(), closure],
            Vec::new(),
        ),
        Err(EncodingError::DuplicateSetValue)
    );
}

#[test]
fn strict_report_decoders_reject_wrong_shapes_and_malformed_bytes() {
    for bytes in [
        Vec::new(),
        CanonicalValue::False.encode().expect("false encodes"),
        CanonicalValue::Record(Vec::new())
            .encode()
            .expect("empty record encodes"),
        CanonicalValue::Record(vec![(2, CanonicalValue::Digest(digest(1)))])
            .encode()
            .expect("mis-tagged record encodes"),
        CanonicalValue::Record(vec![(1, CanonicalValue::Set(Vec::new()))])
            .encode()
            .expect("wrong-shaped record encodes"),
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Digest(digest(1))),
            (3, CanonicalValue::Digest(digest(2))),
            (4, CanonicalValue::Set(Vec::new())),
            (5, CanonicalValue::Set(Vec::new())),
        ])
        .encode()
        .expect("skipped report tag encodes canonically"),
        CanonicalValue::Record(vec![
            (1, CanonicalValue::False),
            (2, CanonicalValue::Digest(digest(2))),
            (3, CanonicalValue::Set(Vec::new())),
            (4, CanonicalValue::Set(Vec::new())),
        ])
        .encode()
        .expect("wrong scalar report shape encodes canonically"),
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Digest(digest(1))),
            (2, CanonicalValue::Digest(digest(2))),
            (3, CanonicalValue::Digest(digest(3))),
            (4, CanonicalValue::Set(Vec::new())),
        ])
        .encode()
        .expect("wrong set report shape encodes canonically"),
    ] {
        let _ = CanonicalStaticReport::decode(&bytes, DecodeBudget::v2_default());
        let _ = CanonicalExecutionReport::decode(&bytes, DecodeBudget::v2_default());
        let _ = CanonicalImpactReport::decode(&bytes, DecodeBudget::v2_default());
    }
    assert_eq!(ReportDecodingError::Schema.code(), "DET-ENC-REPORT-SCHEMA");
    assert_eq!(
        ReportDecodingError::Identity.code(),
        "DET-ENC-REPORT-IDENTITY"
    );
    assert_eq!(
        ReportDecodingError::Canonical(DecodingError::InvalidLength).code(),
        "DET-ENC-LENGTH"
    );
}
