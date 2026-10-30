#![doc = "Behavioral execution of every frozen disposition-protocol fixture."]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::*;
use std::fs;

fn digest(byte: u8) -> SemanticDigest {
    SemanticDigest::xxh3_128([byte; 16])
}

fn sid(value: &str) -> StableId {
    StableId::new(value).expect("valid stable ID")
}

fn payload(schema: &str, bytes: impl Into<Vec<u8>>) -> TypedPayload {
    TypedPayload::new(SchemaId::new(schema).expect("valid schema"), bytes.into())
        .expect("fixture payload")
}

fn approval(authority: &str, path: &str, value: SemanticDigest) -> ApprovalBinding {
    ApprovalBinding::new(
        ScopeId::new(authority).expect("valid scope"),
        RepositoryPath::new(path).expect("approval path"),
        value,
    )
    .expect("approval")
}

fn atom(id: &str, statement: &str) -> Atom {
    Atom::new(
        sid(id),
        LifecycleState::Ratified,
        AtomKind::Constraint,
        vec![ScopeId::new("change.protocol").expect("valid scope")],
        statement,
        Some(approval("maintainers", "reviews/policy.md", digest(90))),
    )
    .expect("atom")
}

fn object_ref(meta: &ObjectMeta) -> ObjectRef {
    ObjectRef::new(meta.kind(), meta.id().clone(), meta.digest())
}

fn revision(roles: Vec<EvidenceRole>) -> CoveragePolicyRevision {
    CoveragePolicyRevision::new(
        payload("policy-scope/v1", vec![1]),
        roles,
        EnforcementMode::Blocking,
        ScopeId::new("disposition-line").expect("valid scope"),
        Vec::new(),
        None,
    )
    .expect("policy revision")
}

fn policy(
    lifecycle: PolicyLifecycle,
    roles: Vec<EvidenceRole>,
    subject: ObjectRef,
    replacements: Vec<ObjectRef>,
) -> CoveragePolicy {
    CoveragePolicy::new(
        sid("POLICY-DISPOSITION"),
        revision(roles),
        approval("maintainers", "reviews/policy.md", digest(90)),
        lifecycle,
        replacements,
        (lifecycle != PolicyLifecycle::Active)
            .then(|| approval("maintainers", "reviews/policy.md", digest(90))),
        vec![subject],
    )
    .expect("coverage policy")
}

fn replacement_policy(subject: ObjectRef) -> CoveragePolicy {
    CoveragePolicy::new(
        sid("POLICY-REPLACEMENT"),
        revision(vec![EvidenceRole::Positive]),
        approval("maintainers", "reviews/policy.md", digest(90)),
        PolicyLifecycle::Active,
        Vec::new(),
        None,
        vec![subject],
    )
    .expect("replacement policy")
}

fn debt(obligations: Vec<ObligationRef>, object: ObjectRef) -> DebtRecord {
    DebtRecord::new(
        sid("DEBT-DISPOSITION"),
        obligations,
        vec![object],
        "Reviewed disposition debt.",
        ScopeId::new("maintainers").expect("valid scope"),
        "Complete every weakened obligation.",
        payload("debt-discharge/v1", vec![1]),
        Vec::new(),
        DebtLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("active debt")
}

fn waiver(obligations: Vec<ObligationRef>, object: ObjectRef) -> Waiver {
    Waiver::new(
        sid("WAIVER-DISPOSITION"),
        obligations,
        vec![object],
        "Reviewed process continuation.",
        ScopeId::new("maintainers").expect("valid scope"),
        payload("waiver-discharge/v1", vec![1]),
        approval("maintainers", "reviews/policy.md", digest(90)),
        Vec::new(),
        WaiverLifecycle::Active,
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("active waiver")
}

fn artifact_index() -> ArtifactIndex {
    ArtifactIndex::new(vec![
        ArtifactEntry::saved_bytes(
            RepositoryPath::new("reviews/policy.md").expect("policy path"),
            RawArtifactMediaType::Markdown,
            digest(90),
        ),
        ArtifactEntry::saved_bytes(
            RepositoryPath::new("reviews/manifest.md").expect("manifest path"),
            RawArtifactMediaType::Markdown,
            digest(91),
        ),
        ArtifactEntry::saved_bytes(
            RepositoryPath::new("reviews/reaffirmation.md").expect("reaffirmation path"),
            RawArtifactMediaType::Markdown,
            digest(92),
        ),
    ])
    .expect("artifact index")
}

struct ProtocolCase {
    base: SpecGraph,
    head: SpecGraph,
    impacts: Vec<Impact>,
    manifest: ChangeManifest,
    artifacts: ArtifactIndex,
    ratchets: Vec<DerivedPolicyRatchet>,
}

fn build_case(kind: ChangeDispositionKind, mutation: &str) -> ProtocolCase {
    let base_atom = atom("ATOM-DISPOSITION", "Base behavior MUST remain governed.");
    let head_atom = atom(
        "ATOM-DISPOSITION",
        "Head behavior MUST remain governed and current.",
    );
    let base_subject = object_ref(base_atom.meta());
    let head_subject = object_ref(head_atom.meta());
    let base_policy = policy(
        PolicyLifecycle::Active,
        vec![EvidenceRole::Positive, EvidenceRole::Negative],
        base_subject,
        Vec::new(),
    );

    let replacement_control = mutation == "replacement";
    let terminal_replacement =
        replacement_control.then(|| replacement_policy(head_subject.clone()));
    let replacement_refs = terminal_replacement
        .as_ref()
        .map(|replacement| vec![object_ref(replacement.meta())])
        .unwrap_or_default();
    let head_policy = if kind == ChangeDispositionKind::Retired && !replacement_control {
        None
    } else {
        Some(policy(
            if replacement_control {
                PolicyLifecycle::Superseded
            } else {
                PolicyLifecycle::Active
            },
            vec![EvidenceRole::Positive],
            head_subject,
            replacement_refs,
        ))
    };

    let mut base = SpecGraph {
        atoms: vec![base_atom],
        coverage_policies: vec![base_policy],
        ..SpecGraph::default()
    };
    let mut head = SpecGraph {
        atoms: vec![head_atom],
        coverage_policies: head_policy.iter().cloned().collect(),
        ..SpecGraph::default()
    };
    if let Some(replacement) = terminal_replacement {
        head.coverage_policies.push(replacement);
    }

    let derived = derive_policy_ratchets(&base, &head).expect("derived weakening");
    let primary = derived
        .iter()
        .find(|ratchet| ratchet.policy_id().as_str() == "POLICY-DISPOSITION")
        .expect("primary policy ratchet");
    let weakened = primary.removed_obligations().to_vec();
    let base_reference = primary
        .base_policy()
        .cloned()
        .expect("base policy reference");
    let head_reference = primary.head_policy().cloned();
    let protocol_ratchets = if kind == ChangeDispositionKind::Reaffirmed
        || (replacement_control
            && matches!(
                kind,
                ChangeDispositionKind::Retired | ChangeDispositionKind::Waived
            )) {
        Vec::new()
    } else {
        derived
    };

    let normal_base = Some(base_reference.digest());
    let normal_head = if kind == ChangeDispositionKind::Retired {
        None
    } else {
        head_reference.as_ref().map(ObjectRef::digest)
    };
    let impact = Impact {
        kind: ObjectKind::CoveragePolicy,
        id: sid(&String::from("POLICY-DISPOSITION")),
        base: normal_base,
        head: if kind == ChangeDispositionKind::Retired && mutation == "head-digest" {
            Some(digest(201))
        } else {
            normal_head
        },
        direct: true,
    };
    let disposition_base = if mutation == "base-digest" {
        Some(digest(200))
    } else {
        normal_base
    };
    let disposition_head = if mutation == "head-digest" && kind != ChangeDispositionKind::Retired {
        Some(digest(201))
    } else {
        normal_head
    };

    let wrong_approval = mutation == "approval";
    let reaffirmation = (kind == ChangeDispositionKind::Reaffirmed).then(|| {
        Reaffirmation::new(
            sid("REAFFIRMATION-DISPOSITION"),
            ObjectKind::CoveragePolicy,
            sid("POLICY-DISPOSITION"),
            disposition_base.expect("reaffirmed base"),
            disposition_head.expect("reaffirmed head"),
            approval(
                if wrong_approval {
                    "reviewers"
                } else {
                    "maintainers"
                },
                "reviews/reaffirmation.md",
                digest(92),
            ),
            "The policy role remains applicable after rereview.",
        )
        .expect("reaffirmation")
    });
    if mutation != "record" {
        head.reaffirmations.extend(reaffirmation.iter().cloned());
    }

    let governance_object = head_reference.clone().unwrap_or(base_reference.clone());
    let active_debt = debt(weakened.clone(), governance_object.clone());
    let active_waiver = waiver(weakened, governance_object);
    let needs_governance =
        kind != ChangeDispositionKind::Reaffirmed && !protocol_ratchets.is_empty();
    if (needs_governance && mutation != "debt")
        || (kind == ChangeDispositionKind::Reaffirmed && mutation == "debt")
    {
        head.debt_records.push(active_debt);
    }
    if (needs_governance && mutation != "waiver")
        || (kind == ChangeDispositionKind::Reaffirmed && mutation == "waiver")
        || (kind == ChangeDispositionKind::Waived && protocol_ratchets.is_empty())
    {
        head.waivers.push(active_waiver.clone());
    }

    let waiver_reference =
        (kind == ChangeDispositionKind::Waived).then(|| object_ref(active_waiver.meta()));
    let reaffirmation_digest = reaffirmation.as_ref().map(|record| record.meta().digest());
    let disposition = ChangeDisposition::new(
        sid("DISPOSITION-CONTROL"),
        ObjectKind::CoveragePolicy,
        sid("POLICY-DISPOSITION"),
        disposition_base,
        disposition_head,
        kind,
        reaffirmation_digest,
        waiver_reference,
        Vec::new(),
    )
    .expect("change disposition");

    let manifest_approval = if kind == ChangeDispositionKind::Reaffirmed {
        None
    } else {
        Some(approval(
            if wrong_approval {
                "reviewers"
            } else {
                "maintainers"
            },
            "reviews/manifest.md",
            if mutation == "record" {
                digest(199)
            } else {
                digest(91)
            },
        ))
    };
    let manifest = ChangeManifest::new(
        sid("CHANGE-DISPOSITION"),
        digest(210),
        digest(211),
        vec![disposition],
        manifest_approval,
    )
    .expect("change manifest");
    if mutation == "applicability" {
        base.reaffirmations.clear();
    }
    ProtocolCase {
        base,
        head,
        impacts: if mutation == "applicability" {
            Vec::new()
        } else {
            vec![impact]
        },
        manifest,
        artifacts: artifact_index(),
        ratchets: protocol_ratchets,
    }
}

#[test]
fn all_frozen_disposition_protocol_fixtures_execute_contextual_validation() {
    let source =
        fs::read_to_string("../../fixtures/hardening/v3/disposition-protocol-fixtures.tsv")
            .expect("disposition fixtures");
    let mut executed = 0;
    for line in source.lines().skip(1) {
        let columns = line.split('\t').collect::<Vec<_>>();
        assert_eq!(columns.len(), 10, "fixture row shape");
        let case_id = columns[1];
        let disposition = match columns[3] {
            "reaffirmed" => ChangeDispositionKind::Reaffirmed,
            "revised" => ChangeDispositionKind::Revised,
            "retired" => ChangeDispositionKind::Retired,
            "waived" => ChangeDispositionKind::Waived,
            other => panic!("unknown disposition {other}"),
        };
        let mutation = columns[4];
        let expected = columns[8];
        let case = build_case(disposition, mutation);
        let context = DispositionProtocolContext::new(
            &case.base,
            &case.head,
            &case.artifacts,
            &case.ratchets,
        );
        let diagnostics = disposition_protocol_closure(&case.impacts, &case.manifest, &context)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>();
        let expected_diagnostics = if expected == "-" {
            Vec::new()
        } else {
            vec![String::from(expected)]
        };
        assert_eq!(diagnostics, expected_diagnostics, "{case_id}");
        let facts = disposition_report_facts(&case.impacts, &case.manifest, &context)
            .expect("orthogonal disposition facts");
        if expected == "-" {
            let report = CanonicalImpactReport::new(
                digest(220),
                digest(221),
                case.manifest.meta().digest(),
                digest(222),
                facts,
                Vec::new(),
            )
            .expect("impact report");
            let CanonicalValue::Record(report_fields) = report.value() else {
                panic!("impact report record");
            };
            let CanonicalValue::Set(facts) = &report_fields[4].1 else {
                panic!("disposition fact set");
            };
            assert_eq!(facts.len(), 1, "{case_id}");
            let CanonicalValue::Record(fields) = &facts[0] else {
                panic!("disposition fact record");
            };
            assert_eq!(
                fields[7].1,
                CanonicalValue::Enum(Closure::Closed as u16, None)
            );
            assert_eq!(
                matches!(&fields[8].1, CanonicalValue::Some(_)),
                disposition == ChangeDispositionKind::Reaffirmed,
                "{case_id} review-only fact"
            );
            assert_eq!(
                matches!(&fields[9].1, CanonicalValue::Some(_)),
                disposition == ChangeDispositionKind::Waived,
                "{case_id} process-only fact"
            );
        }
        executed += 1;
    }
    assert_eq!(executed, 36);

    let case = build_case(ChangeDispositionKind::Reaffirmed, "none");
    let context =
        DispositionProtocolContext::new(&case.base, &case.head, &case.artifacts, &case.ratchets);
    let mut added = case.impacts[0].clone();
    added.id = sid(&String::from("ADDED-CONTROL"));
    added.base = None;
    added.head = Some(digest(230));
    added.direct = false;
    let mut unchanged = case.impacts[0].clone();
    unchanged.id = sid(&String::from("UNCHANGED-CONTROL"));
    unchanged.base = Some(digest(231));
    unchanged.head = Some(digest(231));
    disposition_report_facts(&[added, unchanged], &case.manifest, &context)
        .expect("added and unchanged impact facts");
}
