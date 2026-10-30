//! Seeded Hegel differentials that invoke production and dependency-independent authorities.

#![allow(
    clippy::expect_used,
    clippy::fn_params_excessive_bools,
    clippy::items_after_statements,
    clippy::similar_names
)]

#[path = "../tools/digest_binding.rs"]
mod content_digest;
#[path = "../tools/execution_evidence.rs"]
mod execution_evidence;

use buoy::core::*;
use buoy_reference::{Value, algorithms as reference};
use hegel::generators as gs;
use hegel::{Hegel, TestCase};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

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
        .expect("valid property typed payload")
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
    .expect("valid property tool identity")
}

fn test_surface_disposition(
    key: TypedPayload,
    disposition: SurfaceDispositionKind,
    payload: Option<TypedPayload>,
    authority: Option<AtomRevisionRef>,
) -> SurfaceDisposition {
    let (payload, authority) = match disposition {
        SurfaceDispositionKind::Normative
        | SurfaceDispositionKind::Prohibited
        | SurfaceDispositionKind::Deferred => (
            Some(payload.unwrap_or_else(|| test_payload("surface-value/v1", vec![1]))),
            Some(authority.unwrap_or_else(|| test_atom_ref(sid("ATOM-SURFACE"), digest(89)))),
        ),
        SurfaceDispositionKind::NotApplicable => (payload, authority),
    };
    SurfaceDisposition::new(key, disposition, payload, authority)
        .expect("valid property surface disposition")
}

fn approval() -> Option<ApprovalBinding> {
    RepositoryPath::new("reviews/property.md")
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
        vec![ScopeId::new("assurance.property").expect("valid scope")],
        statement,
        approval(),
    )
    .ok()
}

fn meta_ref(meta: &ObjectMeta) -> ObjectRef {
    test_object_ref(meta.kind(), meta.id().clone(), meta.digest())
}

fn test_oracle(id: StableId, configuration: SemanticDigest, _refs: Vec<ObjectRef>) -> Oracle {
    let runner_id = format!("{}.runner", id.as_str());
    buoy::core::Oracle::new(
        id,
        OracleKind::Test,
        test_tool(runner_id, "1", configuration, configuration),
        test_payload("oracle-selection/v1", Vec::new()),
        test_payload("verdict-normalization/v1", Vec::new()),
        configuration,
        test_payload("oracle-scope/v1", Vec::new()),
        Vec::new(),
    )
    .expect("complete property oracle")
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
        atoms.push(test_atom_ref(sid("ATOM-IMPLEMENTATION"), digest(69)));
    }
    buoy::core::ImplementationLink::new(id, atoms, parsed_item_digest, NavigationKind::Implements)
        .expect("complete property implementation link")
}

fn test_finite_model(
    id: StableId,
    payload_digest: SemanticDigest,
    references: Vec<ObjectRef>,
) -> FiniteModel {
    let assumptions = references
        .into_iter()
        .filter(|reference| reference.kind() == ObjectKind::Assumption)
        .collect();
    buoy::core::FiniteModel::new(
        id,
        test_payload("model-state-space/v1", payload_digest.bytes().to_vec()),
        test_payload("model-events/v1", Vec::new()),
        test_payload("model-transition/v1", Vec::new()),
        vec![test_payload("model-state/v1", vec![0])],
        test_payload("model-bound/v1", vec![1]),
        Vec::new(),
        assumptions,
        Vec::new(),
    )
    .expect("complete property finite model")
}

fn test_assumption(id: StableId, seed: SemanticDigest, _refs: Vec<ObjectRef>) -> Assumption {
    buoy::core::Assumption::new(
        id,
        "Reviewed property assumption.",
        test_payload("assumption-scope/v1", seed.bytes().to_vec()),
        Vec::new(),
        approval().expect("property assumption approval"),
    )
    .expect("complete property assumption")
}

#[allow(clippy::too_many_arguments)]
fn test_carrier(
    id: StableId,
    parsed_item_digest: SemanticDigest,
    _refs: Vec<ObjectRef>,
    role: EvidenceRole,
    atoms: Vec<AtomRevisionRef>,
    oracle: ObjectRef,
    configuration: SemanticDigest,
    corpora: Vec<SemanticDigest>,
    scope: TypedPayload,
) -> EvidenceCarrier {
    buoy::core::EvidenceCarrier::new(
        id,
        role,
        atoms,
        parsed_item_digest,
        oracle,
        configuration,
        corpora,
        scope,
        Vec::new(),
        Vec::new(),
    )
    .expect("complete property carrier")
}

#[allow(clippy::too_many_arguments)]
fn test_derivation(
    id: StableId,
    configuration: SemanticDigest,
    _refs: Vec<ObjectRef>,
    sources: Vec<ArtifactBinding>,
    targets: Vec<ArtifactBinding>,
    tool: ToolIdentity,
    oracle: ObjectRef,
) -> Derivation {
    buoy::core::Derivation::new(
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
    .expect("complete property derivation")
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
    buoy::core::FormalProperty::new(
        id,
        atoms,
        property,
        test_payload("implementation-abstraction/v1", Vec::new()),
        references,
        Vec::new(),
        approval().expect("property correspondence approval"),
    )
    .expect("complete formal property")
}

#[allow(clippy::too_many_arguments)]
fn test_proof_record(
    id: StableId,
    verifier_configuration: SemanticDigest,
    _refs: Vec<ObjectRef>,
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
    buoy::core::ProofRecord::new(
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
    .expect("complete property proof record")
}

#[allow(clippy::too_many_arguments)]
fn test_policy(
    id: StableId,
    _digest: SemanticDigest,
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
    .expect("complete property policy revision");
    CoveragePolicy::new(
        id,
        revision,
        approval().expect("property policy approval"),
        lifecycle,
        Vec::new(),
        None,
        subjects,
    )
    .expect("complete property policy")
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
        .then(|| test_object_ref(ObjectKind::Waiver, sid("WAIVER-PROPERTY"), digest));
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
    .expect("complete property change disposition")
}

fn test_change_manifest(
    id: StableId,
    _digest: SemanticDigest,
    _references: Vec<ObjectRef>,
    base: SemanticDigest,
    head: SemanticDigest,
    dispositions: Vec<ChangeDisposition>,
) -> ChangeManifest {
    ChangeManifest::new(id, base, head, dispositions, None)
        .expect("complete property change manifest")
}

fn generated_value(bytes: &[u8], depth: u8) -> (CanonicalValue, Value) {
    let selector = bytes.first().copied().unwrap_or(0) % 13;
    let tail = bytes.get(1..).unwrap_or_default();
    let split = tail.len() / 2;
    let (left, right) = tail.split_at(split);
    if depth == 0 {
        return (
            CanonicalValue::U64(u64::from(selector)),
            Value::Unsigned(u64::from(selector)),
        );
    }
    match selector {
        0 => (CanonicalValue::False, Value::False),
        1 => (CanonicalValue::True, Value::True),
        2 => {
            let value = tail
                .iter()
                .fold(0_u64, |value, byte| value.rotate_left(5) ^ u64::from(*byte));
            (CanonicalValue::U64(value), Value::Unsigned(value))
        }
        3 => {
            let value = tail
                .iter()
                .fold(0_i64, |value, byte| value.rotate_left(5) ^ i64::from(*byte));
            (CanonicalValue::I64(value), Value::Signed(value))
        }
        4 => (
            CanonicalValue::Bytes(tail.to_vec()),
            Value::Bytes(tail.to_vec()),
        ),
        5 => {
            let text = tail
                .iter()
                .map(|byte| char::from(b'a' + (*byte % 26)))
                .collect::<String>();
            (CanonicalValue::Text(text.clone()), Value::Text(text))
        }
        6 => {
            let (production, reference) = generated_value(tail, depth - 1);
            (
                CanonicalValue::Record(vec![(1, production)]),
                Value::Record(vec![(1, reference)]),
            )
        }
        7 => {
            let (production_left, reference_left) = generated_value(left, depth - 1);
            let (production_right, reference_right) = generated_value(right, depth - 1);
            (
                CanonicalValue::Sequence(vec![production_left, production_right]),
                Value::Sequence(vec![reference_left, reference_right]),
            )
        }
        8 => {
            let (production_left, reference_left) = generated_value(left, depth - 1);
            let (production_right, reference_right) = generated_value(right, depth - 1);
            (
                CanonicalValue::Set(vec![production_left, production_right]),
                Value::Set(vec![reference_left, reference_right]),
            )
        }
        9 => (CanonicalValue::None, Value::None),
        10 => {
            let (production, reference) = generated_value(tail, depth - 1);
            (
                CanonicalValue::Some(Box::new(production)),
                Value::Some(Box::new(reference)),
            )
        }
        11 => {
            let (production, reference) = generated_value(tail, depth - 1);
            (
                CanonicalValue::Enum(1, Some(Box::new(production))),
                Value::Enum(1, Some(Box::new(reference))),
            )
        }
        _ => {
            let mut digest = [0_u8; 16];
            for (index, byte) in tail.iter().enumerate() {
                digest[index % 16] ^= *byte;
            }
            (
                CanonicalValue::Digest(SemanticDigest::xxh3_128(digest)),
                Value::Digest(2, digest.to_vec()),
            )
        }
    }
}

#[test]
fn hegel_production_and_reference_canonical_sets_agree() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(32));
        let production = CanonicalValue::Set(
            bytes
                .iter()
                .copied()
                .map(|value| CanonicalValue::U64(u64::from(value)))
                .collect(),
        );
        let reference = Value::Set(
            bytes
                .iter()
                .copied()
                .map(|value| Value::Unsigned(u64::from(value)))
                .collect(),
        );
        assert_eq!(production.encode().ok(), reference.encode().ok());
    })
    .settings(settings(1))
    .run();
}

#[test]
fn hegel_recursive_values_round_trip_and_match_the_reference() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(64));
        let (production, reference) = generated_value(&bytes, 4);
        let production_bytes = production.encode();
        let reference_bytes = reference.encode();
        assert_eq!(
            production_bytes.as_ref().ok(),
            reference_bytes.as_ref().ok()
        );
        if let Ok(encoded) = production_bytes {
            let decoded = CanonicalValue::decode(&encoded, DecodeBudget::v2_default());
            assert_eq!(
                decoded.ok().and_then(|value| value.encode().ok()),
                Some(encoded.clone())
            );
            let reference_decoded = Value::decode(&encoded, 128, 1_000_000);
            assert_eq!(
                reference_decoded.ok().and_then(|value| value.encode().ok()),
                Some(encoded)
            );
        }
    })
    .settings(settings(20))
    .run();
}

#[test]
fn hegel_production_and_reference_preimages_agree() {
    Hegel::new(|test_case: TestCase| {
        let field = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(64));
        let production = semantic_digest_observation(
            "property-differential",
            vec![(1, CanonicalValue::Bytes(field.clone()))],
        );
        let reference = buoy_reference::semantic_preimage(
            2,
            "property-differential",
            vec![(1, Value::Bytes(field))],
        );
        assert_eq!(
            production
                .ok()
                .map(|value| value.canonical_preimage().to_vec()),
            reference.ok()
        );
    })
    .settings(settings(2))
    .run();
}

#[test]
fn hegel_production_and_reference_path_policy_agree() {
    const INPUTS: [&str; 12] = [
        "spec/requirements.md",
        "spec/é.md",
        "spec/e\u{301}.md",
        "../outside",
        "/absolute",
        "C:/drive",
        "spec/CON.txt",
        "spec/trailing.",
        "spec//empty",
        "spec/back\\slash",
        "spec/question?.md",
        "",
    ];
    Hegel::new(|test_case: TestCase| {
        let sample = test_case.draw(gs::integers::<u8>());
        let input = INPUTS[usize::from(sample) % INPUTS.len()];
        let production = RepositoryPath::new(input);
        let independent = reference::portable_path(input);
        assert_eq!(production.is_ok(), independent.is_ok());
        if let (Ok(production), Ok(independent)) = (production, independent) {
            assert_eq!(production.as_str(), independent);
        }
    })
    .settings(settings(3))
    .run();
}

#[test]
fn hegel_production_and_reference_path_sets_agree() {
    Hegel::new(|test_case: TestCase| {
        let first = test_case.draw(gs::integers::<u8>());
        let second = test_case.draw(gs::integers::<u8>());
        let name = |sample: u8| match sample % 5 {
            0 => String::from("spec/readme.md"),
            1 => String::from("spec/README.md"),
            2 => String::from("spec/design.md"),
            3 => String::from("src/lib.rs"),
            _ => String::from("src/main.rs"),
        };
        let raw = vec![name(first), name(second)];
        let parsed = raw
            .iter()
            .map(|path| RepositoryPath::new(path))
            .collect::<Result<Vec<_>, _>>();
        let production = parsed.ok().and_then(|paths| canonical_paths(paths).ok());
        let independent = reference::portable_path_set(&raw).ok();
        assert_eq!(production.is_some(), independent.is_some());
        if let (Some(production), Some(independent)) = (production, independent) {
            assert_eq!(
                production
                    .iter()
                    .map(|path| path.as_str().to_owned())
                    .collect::<Vec<_>>(),
                independent
            );
        }
    })
    .settings(settings(4))
    .run();
}

#[test]
fn hegel_production_and_reference_lifecycle_agree() {
    Hegel::new(|test_case: TestCase| {
        let state_tag = u16::from(test_case.draw(gs::integers::<u8>()) % 4) + 1;
        let event_tag = u16::from(test_case.draw(gs::integers::<u8>()) % 3) + 1;
        let state = match state_tag {
            1 => LifecycleState::Proposed,
            2 => LifecycleState::Ratified,
            3 => LifecycleState::Superseded,
            _ => LifecycleState::Withdrawn,
        };
        let event = match event_tag {
            1 => LifecycleEvent::Ratify,
            2 => LifecycleEvent::Withdraw,
            _ => LifecycleEvent::Supersede,
        };
        let production = lifecycle_transition(state, event)
            .ok()
            .map(|value| value as u16);
        assert_eq!(
            production,
            reference::lifecycle_transition(state_tag, event_tag)
        );
    })
    .settings(settings(5))
    .run();
}

#[test]
fn hegel_production_and_reference_governance_lifecycle_agree() {
    Hegel::new(|test_case: TestCase| {
        let option = |sample: u8, variants: u8| (sample % (variants + 1)).checked_sub(0);
        let policy = |tag: u8| match tag {
            1 => Some(PolicyLifecycle::Active),
            2 => Some(PolicyLifecycle::Superseded),
            3 => Some(PolicyLifecycle::Retired),
            _ => None,
        };
        let debt = |tag: u8| match tag {
            1 => Some(DebtLifecycle::Active),
            2 => Some(DebtLifecycle::Discharged),
            3 => Some(DebtLifecycle::Superseded),
            _ => None,
        };
        let waiver = |tag: u8| match tag {
            1 => Some(WaiverLifecycle::Active),
            2 => Some(WaiverLifecycle::Discharged),
            3 => Some(WaiverLifecycle::Revoked),
            4 => Some(WaiverLifecycle::Superseded),
            _ => None,
        };
        let bp = option(test_case.draw(gs::integers::<u8>()), 3).unwrap_or(0);
        let hp = option(test_case.draw(gs::integers::<u8>()), 3).unwrap_or(0);
        let bd = option(test_case.draw(gs::integers::<u8>()), 3).unwrap_or(0);
        let hd = option(test_case.draw(gs::integers::<u8>()), 3).unwrap_or(0);
        let bw = option(test_case.draw(gs::integers::<u8>()), 4).unwrap_or(0);
        let hw = option(test_case.draw(gs::integers::<u8>()), 4).unwrap_or(0);
        let production = governance_transition_valid(
            policy(bp),
            policy(hp),
            debt(bd),
            debt(hd),
            waiver(bw),
            waiver(hw),
        );
        let tag = |value: u8| (value != 0).then_some(u16::from(value));
        assert_eq!(
            production,
            reference::governance_transition_valid(
                tag(bp),
                tag(hp),
                tag(bd),
                tag(hd),
                tag(bw),
                tag(hw),
            )
        );
    })
    .settings(settings(6))
    .run();
}

#[test]
fn hegel_production_and_reference_surface_completeness_agree() {
    Hegel::new(|test_case: TestCase| {
        let Some(authority_atom) = atom("PROP-ATOM", "Every member MUST have authority.") else {
            return;
        };
        let atom_ref = test_atom_ref(
            authority_atom.meta().id().clone(),
            authority_atom.revision(),
        );
        let expected_a = test_payload("key/v1", b"a".to_vec());
        let expected_b = test_payload("key/v1", b"b".to_vec());
        let include_b = test_case.draw(gs::booleans());
        let duplicate_a = test_case.draw(gs::booleans());
        let authority_a = test_case.draw(gs::booleans());
        let authority_b = test_case.draw(gs::booleans());
        let mut rows = vec![test_surface_disposition(
            expected_a.clone(),
            SurfaceDispositionKind::Normative,
            Some(test_payload("payload/v1", vec![1])),
            authority_a.then_some(atom_ref.clone()),
        )];
        let mut independent = vec![(b"a".to_vec(), authority_a, true)];
        if include_b {
            rows.push(test_surface_disposition(
                expected_b.clone(),
                SurfaceDispositionKind::Normative,
                Some(test_payload("payload/v1", vec![2])),
                authority_b.then_some(atom_ref.clone()),
            ));
            independent.push((b"b".to_vec(), authority_b, true));
        }
        if duplicate_a {
            rows.push(test_surface_disposition(
                expected_a.clone(),
                SurfaceDispositionKind::Normative,
                Some(test_payload("payload/v1", vec![3])),
                Some(atom_ref),
            ));
            independent.push((b"a".to_vec(), true, true));
        }
        let Ok(domain) = DecisionSurfaceDomain::new(
            sid("PROP-SURFACE"),
            test_payload("key-schema/v1", vec![1]),
            test_payload("disposition-schema/v1", vec![1]),
            vec![expected_a, expected_b],
            1,
        ) else {
            return;
        };
        let Ok(surface) = DecisionSurface::new(domain, rows, Vec::new()) else {
            return;
        };
        let graph = SpecGraph {
            atoms: vec![authority_atom],
            ..SpecGraph::default()
        };
        assert_eq!(
            surface_completeness(&graph, &surface).complete,
            reference::surface_complete(&[b"a".to_vec(), b"b".to_vec()], &independent)
        );
    })
    .settings(settings(7))
    .run();
}

#[test]
fn hegel_production_and_reference_freshness_agree() {
    Hegel::new(|test_case: TestCase| {
        let atom_current = test_case.draw(gs::booleans());
        let oracle_current = test_case.draw(gs::booleans());
        let Some(authority_atom) = atom("PROP-FRESH", "Every binding MUST be current.") else {
            return;
        };
        let oracle = test_oracle(sid("PROP-ORACLE"), digest(3), Vec::new());
        let atom_revision = if atom_current {
            authority_atom.revision()
        } else {
            digest(4)
        };
        let oracle_digest = if oracle_current {
            oracle.meta().digest()
        } else {
            digest(5)
        };
        let carrier = test_carrier(
            sid("PROP-CARRIER"),
            digest(6),
            Vec::new(),
            EvidenceRole::Positive,
            vec![test_atom_ref(
                authority_atom.meta().id().clone(),
                atom_revision,
            )],
            test_object_ref(
                ObjectKind::Oracle,
                oracle.meta().id().clone(),
                oracle_digest,
            ),
            digest(7),
            Vec::new(),
            test_payload("scope/v1", vec![1]),
        );
        let graph = SpecGraph {
            atoms: vec![authority_atom],
            evidence_carriers: vec![carrier.clone()],
            oracles: vec![oracle],
            ..SpecGraph::default()
        };
        assert_eq!(
            carrier_fresh(&graph, &carrier),
            reference::all_current(&[atom_current, oracle_current])
        );
    })
    .settings(settings(8))
    .run();
}

#[test]
fn hegel_production_and_reference_derivation_freshness_agree() {
    Hegel::new(|test_case: TestCase| {
        let source_current = test_case.draw(gs::booleans());
        let target_current = test_case.draw(gs::booleans());
        let oracle_current = test_case.draw(gs::booleans());
        let source = test_implementation_link(sid("DERIVE-SOURCE"), digest(70), Vec::new());
        let target = test_finite_model(sid("DERIVE-TARGET"), digest(71), Vec::new());
        let oracle = test_oracle(sid("DERIVE-ORACLE"), digest(72), Vec::new());
        let binding = |value: &ObjectMeta, current: bool, stale: u8| {
            ArtifactBinding::Object(test_object_ref(
                value.kind(),
                value.id().clone(),
                if current {
                    value.digest()
                } else {
                    digest(stale)
                },
            ))
        };
        let derivation = test_derivation(
            sid("DERIVE-PROP"),
            digest(73),
            Vec::new(),
            vec![binding(source.meta(), source_current, 74)],
            vec![binding(target.meta(), target_current, 75)],
            test_tool("generator", "1", digest(76), digest(77)),
            test_object_ref(
                ObjectKind::Oracle,
                oracle.meta().id().clone(),
                if oracle_current {
                    oracle.meta().digest()
                } else {
                    digest(78)
                },
            ),
        );
        let graph = SpecGraph {
            implementation_links: vec![source],
            finite_models: vec![target],
            derivations: vec![derivation.clone()],
            oracles: vec![oracle],
            ..SpecGraph::default()
        };
        assert_eq!(
            derivation_fresh(&graph, &derivation),
            reference::all_current(&[source_current, target_current, oracle_current])
        );
    })
    .settings(settings(14))
    .run();
}

#[test]
#[allow(clippy::too_many_lines)]
fn hegel_production_and_reference_proof_freshness_agree() {
    Hegel::new(|test_case: TestCase| {
        let atom_current = test_case.draw(gs::booleans());
        let property_current = test_case.draw(gs::booleans());
        let implementation_current = test_case.draw(gs::booleans());
        let assumption_current = test_case.draw(gs::booleans());
        let verdict_pass = test_case.draw(gs::booleans());
        let Some(authority_atom) = atom("PROOF-ATOM", "The proof binding MUST be current.") else {
            return;
        };
        let property = test_formal_property(
            sid("PROOF-PROPERTY"),
            digest(80),
            Vec::new(),
            vec![test_atom_ref(
                authority_atom.meta().id().clone(),
                authority_atom.revision(),
            )],
            test_payload("property/v1", vec![1]),
            true,
        );
        let implementation =
            test_implementation_link(sid("PROOF-IMPLEMENTATION"), digest(81), Vec::new());
        let assumption = test_assumption(sid("PROOF-ASSUMPTION"), digest(82), Vec::new());
        let implementation_binding = ArtifactBinding::Object(test_object_ref(
            ObjectKind::ImplementationLink,
            implementation.meta().id().clone(),
            if implementation_current {
                implementation.meta().digest()
            } else {
                digest(83)
            },
        ));
        let proof = test_proof_record(
            sid("PROOF-PROP"),
            digest(84),
            Vec::new(),
            vec![test_atom_ref(
                authority_atom.meta().id().clone(),
                if atom_current {
                    authority_atom.revision()
                } else {
                    digest(85)
                },
            )],
            test_object_ref(
                ObjectKind::FormalProperty,
                property.meta().id().clone(),
                if property_current {
                    property.meta().digest()
                } else {
                    digest(86)
                },
            ),
            implementation_binding.clone(),
            vec![test_object_ref(
                ObjectKind::Assumption,
                assumption.meta().id().clone(),
                if assumption_current {
                    assumption.meta().digest()
                } else {
                    digest(87)
                },
            )],
            Vec::new(),
            test_tool("verifier", "1", digest(88), digest(89)),
            Vec::new(),
            Vec::new(),
            vec![implementation_binding],
            if verdict_pass {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
        );
        let graph = SpecGraph {
            atoms: vec![authority_atom],
            implementation_links: vec![implementation],
            formal_properties: vec![property],
            assumptions: vec![assumption],
            proof_records: vec![proof.clone()],
            ..SpecGraph::default()
        };
        assert_eq!(
            proof_fresh(&graph, &proof),
            reference::all_current(&[
                atom_current,
                property_current,
                implementation_current,
                assumption_current,
                verdict_pass,
            ])
        );
    })
    .settings(settings(15))
    .run();
}

#[test]
fn hegel_production_and_reference_role_coverage_agree() {
    Hegel::new(|test_case: TestCase| {
        let positive = test_case.draw(gs::booleans());
        let negative = test_case.draw(gs::booleans());
        let Some(authority_atom) = atom("PROP-ROLE", "Every required role MUST be present.") else {
            return;
        };
        let oracle = test_oracle(sid("ROLE-ORACLE"), digest(10), Vec::new());
        let mut carriers = Vec::new();
        for (role, present, id, byte) in [
            (EvidenceRole::Positive, positive, "ROLE-POS", 11),
            (EvidenceRole::Negative, negative, "ROLE-NEG", 12),
        ] {
            if present {
                carriers.push(test_carrier(
                    sid(id),
                    digest(byte),
                    Vec::new(),
                    role,
                    vec![test_atom_ref(
                        authority_atom.meta().id().clone(),
                        authority_atom.revision(),
                    )],
                    meta_ref(oracle.meta()),
                    digest(13),
                    Vec::new(),
                    test_payload("scope/v1", vec![1]),
                ));
            }
        }
        let policy = test_policy(
            sid("ROLE-POLICY"),
            digest(14),
            Vec::new(),
            digest(15),
            PolicyLifecycle::Active,
            EnforcementMode::Blocking,
            ScopeId::new("role-line").expect("valid scope"),
            vec![meta_ref(authority_atom.meta())],
            vec![EvidenceRole::Positive, EvidenceRole::Negative],
        );
        let graph = SpecGraph {
            atoms: vec![authority_atom],
            evidence_carriers: carriers,
            oracles: vec![oracle],
            ..SpecGraph::default()
        };
        let production = required_role_coverage(&graph, &policy);
        let (present, missing) =
            reference::required_roles(&[1, 2], &[(1, positive), (2, negative)]);
        assert_eq!(
            production
                .present
                .iter()
                .map(|role| *role as u16)
                .collect::<Vec<_>>(),
            present
        );
        assert_eq!(
            production
                .missing
                .iter()
                .map(|role| *role as u16)
                .collect::<Vec<_>>(),
            missing
        );
    })
    .settings(settings(9))
    .run();
}

#[test]
fn hegel_production_and_reference_policy_ratchets_agree() {
    Hegel::new(|test_case: TestCase| {
        let keep_second = test_case.draw(gs::booleans());
        let keep_negative = test_case.draw(gs::booleans());
        let head_blocking = test_case.draw(gs::booleans());
        let same_lineage = test_case.draw(gs::booleans());
        let subject_a = test_object_ref(ObjectKind::Atom, sid("ROLE-A"), digest(20));
        let subject_b = test_object_ref(ObjectKind::Atom, sid("ROLE-B"), digest(21));
        let base = test_policy(
            sid("POLICY-PROP"),
            digest(22),
            Vec::new(),
            digest(23),
            PolicyLifecycle::Active,
            EnforcementMode::Blocking,
            ScopeId::new("line-a").expect("valid scope"),
            vec![subject_a.clone(), subject_b.clone()],
            vec![EvidenceRole::Positive, EvidenceRole::Negative],
        );
        let mut head_subjects = vec![subject_a];
        if keep_second {
            head_subjects.push(subject_b);
        }
        let mut head_roles = vec![EvidenceRole::Positive];
        if keep_negative {
            head_roles.push(EvidenceRole::Negative);
        }
        let head = test_policy(
            sid("POLICY-PROP"),
            digest(24),
            Vec::new(),
            digest(25),
            PolicyLifecycle::Active,
            if head_blocking {
                EnforcementMode::Blocking
            } else {
                EnforcementMode::Advisory
            },
            if same_lineage { "line-a" } else { "line-b" },
            head_subjects,
            head_roles,
        );
        let production = compare_policy(&base, &head);
        let base_subjects = [String::from("ROLE-A"), String::from("ROLE-B")]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut independent_head = BTreeSet::from([String::from("ROLE-A")]);
        if keep_second {
            independent_head.insert(String::from("ROLE-B"));
        }
        let base_roles = BTreeSet::from([1_u16, 2]);
        let mut independent_roles = BTreeSet::from([1_u16]);
        if keep_negative {
            independent_roles.insert(2);
        }
        let independent = reference::policy_ratchet(
            &base_subjects,
            &independent_head,
            &base_roles,
            &independent_roles,
            true,
            head_blocking,
            same_lineage,
        );
        assert_eq!(
            production
                .removed_subjects
                .iter()
                .map(|(_, id)| id.as_str().to_string())
                .collect::<Vec<_>>(),
            independent.0
        );
        assert_eq!(
            production
                .removed_roles
                .iter()
                .map(|role| *role as u16)
                .collect::<Vec<_>>(),
            independent.1
        );
        assert_eq!(production.mode_weakened, independent.2);
        assert_eq!(production.lineage_changed, independent.3);
    })
    .settings(settings(10))
    .run();
}

#[test]
fn hegel_production_and_reference_no_laundering_agree() {
    Hegel::new(|test_case: TestCase| {
        let linked = test_case.draw(gs::booleans());
        let fresh = test_case.draw(gs::booleans());
        let selected = test_case.draw(gs::booleans());
        let verdict_pass = test_case.draw(gs::booleans());
        let exhaustive = test_case.draw(gs::booleans());
        let formal = test_case.draw(gs::booleans());
        let structural = test_case.draw(gs::booleans());
        let reviewed = test_case.draw(gs::booleans());
        let debt = test_case.draw(gs::booleans());
        let waiver = test_case.draw(gs::booleans());
        let reaffirmation = test_case.draw(gs::booleans());
        let verdict = if verdict_pass {
            Verdict::Pass
        } else {
            Verdict::Fail
        };
        let production = aggregate_assurance(&AggregationInput {
            linked,
            required_fresh: fresh,
            required_selected: selected,
            verdicts: vec![verdict],
            exhaustive,
            formal,
            structural,
            reviewed,
            debt,
            waiver,
            reaffirmation,
        });
        let independent = reference::assurance_classes(
            linked,
            fresh,
            selected,
            &[verdict as u16],
            exhaustive,
            formal,
            structural,
            reviewed,
        );
        assert_eq!(
            production
                .iter()
                .map(|class| *class as u16)
                .collect::<Vec<_>>(),
            independent
        );
    })
    .settings(settings(11))
    .run();
}

#[test]
fn hegel_production_and_reference_impact_closure_agree() {
    Hegel::new(|test_case: TestCase| {
        let change_atom = test_case.draw(gs::booleans());
        let change_link = test_case.draw(gs::booleans());
        let Some(base_atom) = atom("IMPACT-ATOM", "The base statement MUST remain current.") else {
            return;
        };
        let head_atom = if change_atom {
            let Some(value) = atom("IMPACT-ATOM", "The changed statement MUST remain current.")
            else {
                return;
            };
            value
        } else {
            base_atom.clone()
        };
        let base_link = test_implementation_link(
            sid("IMPACT-LINK"),
            digest(32),
            vec![meta_ref(base_atom.meta())],
        );
        let head_link = test_implementation_link(
            sid("IMPACT-LINK"),
            digest(if change_link { 33 } else { 32 }),
            vec![meta_ref(base_atom.meta())],
        );
        let atom_direct = base_atom.meta().digest() != head_atom.meta().digest();
        let link_direct = base_link.meta().digest() != head_link.meta().digest();
        let base = SpecGraph {
            atoms: vec![base_atom],
            implementation_links: vec![base_link],
            ..SpecGraph::default()
        };
        let head = SpecGraph {
            atoms: vec![head_atom],
            implementation_links: vec![head_link],
            ..SpecGraph::default()
        };
        let production = change_impacts(&base, &head)
            .into_iter()
            .map(|impact| (impact.id.as_str().to_string(), impact.direct))
            .collect::<BTreeSet<_>>();
        let mut base_reference = BTreeMap::new();
        base_reference.insert(String::from("IMPACT-ATOM"), (0, Vec::new()));
        base_reference.insert(
            String::from("IMPACT-LINK"),
            (0, vec![String::from("IMPACT-ATOM")]),
        );
        let mut head_reference = base_reference.clone();
        head_reference.get_mut("IMPACT-ATOM").expect("atom row").0 = u8::from(atom_direct);
        head_reference.get_mut("IMPACT-LINK").expect("link row").0 = u8::from(link_direct);
        assert_eq!(
            production,
            reference::transitive_impacts(&base_reference, &head_reference)
        );
    })
    .settings(settings(12))
    .run();
}

#[test]
fn hegel_production_and_reference_disposition_closure_agree() {
    Hegel::new(|test_case: TestCase| {
        let disposition_count = usize::from(test_case.draw(gs::integers::<u8>()) % 2);
        let impact = Impact {
            kind: ObjectKind::Atom,
            id: sid(&String::from("DISPOSITION-ATOM")),
            base: Some(digest(91)),
            head: Some(digest(92)),
            direct: true,
        };
        let dispositions = (0..disposition_count)
            .map(|index| {
                test_change_disposition(
                    sid(&format!("DISPOSITION-{index}")),
                    digest(u8::try_from(93 + index).unwrap_or(93)),
                    Vec::new(),
                    ObjectKind::Atom,
                    sid("DISPOSITION-ATOM"),
                    impact.base,
                    impact.head,
                    ChangeDispositionKind::Revised,
                )
            })
            .collect::<Vec<_>>();
        let manifest = test_change_manifest(
            sid("DISPOSITION-MANIFEST"),
            digest(96),
            Vec::new(),
            digest(97),
            digest(98),
            dispositions,
        );
        let production = disposition_closure(core::slice::from_ref(&impact), &manifest).is_empty();
        let independent_dispositions = (0..disposition_count)
            .map(|_| String::from("DISPOSITION-ATOM"))
            .collect::<Vec<_>>();
        assert_eq!(
            production,
            reference::disposition_closed(
                &[String::from("DISPOSITION-ATOM")],
                &independent_dispositions,
            )
        );
    })
    .settings(settings(16))
    .run();
}

#[test]
fn hegel_semantic_graph_digest_is_order_independent() {
    Hegel::new(|test_case: TestCase| {
        let reverse = test_case.draw(gs::booleans());
        let first = test_implementation_link(sid("ORDER-A"), digest(40), Vec::new());
        let second = test_implementation_link(sid("ORDER-B"), digest(41), Vec::new());
        let ordered = if reverse {
            vec![second.clone(), first.clone()]
        } else {
            vec![first.clone(), second.clone()]
        };
        let graph = SpecGraph {
            implementation_links: ordered,
            ..SpecGraph::default()
        };
        let canonical = SpecGraph {
            implementation_links: vec![first, second],
            ..SpecGraph::default()
        };
        assert_eq!(
            semantic_graph_digest(&graph).expect("permuted graph must encode"),
            semantic_graph_digest(&canonical).expect("canonical graph must encode")
        );
    })
    .settings(settings(13))
    .run();
}

#[test]
fn hegel_execution_report_is_order_independent() {
    Hegel::new(|test_case: TestCase| {
        let reverse = test_case.draw(gs::booleans());
        let graph_digest = digest(42);
        let verdicts = if reverse {
            vec![
                (String::from("ORACLE-B"), Verdict::Fail),
                (String::from("ORACLE-A"), Verdict::Pass),
            ]
        } else {
            vec![
                (String::from("ORACLE-A"), Verdict::Pass),
                (String::from("ORACLE-B"), Verdict::Fail),
            ]
        };
        let report = execution_report(graph_digest, verdicts, Vec::new(), Vec::new());
        assert_eq!(
            report.verdicts,
            vec![
                (String::from("ORACLE-A"), Verdict::Pass),
                (String::from("ORACLE-B"), Verdict::Fail)
            ]
        );
    })
    .settings(settings(17))
    .run();
}

#[allow(clippy::too_many_lines)]
fn validate_registered_finite_models(write: bool) {
    const MODEL_IDS: [&str; 15] = [
        "model-artifact-index-freshness-v2",
        "model-atom-lifecycle-v2",
        "model-carrier-derivation-freshness-v2",
        "model-change-impact-closure-v2",
        "model-complete-impact-fixed-point-v2",
        "model-coverage-policy-adoption-v2",
        "model-debt-waiver-reaffirmation-no-laundering-v2",
        "model-decision-surface-completeness-v2",
        "model-governance-no-laundering-hardening-v2",
        "model-policy-weakening-protocol-v2",
        "model-proof-invalidation-hardening-v2",
        "model-proof-invalidation-v2",
        "model-required-role-coverage-v2",
        "model-terminal-governance-history-v2",
        "model-verdict-aggregation-v2",
    ];
    const MODEL_PATHS: [&str; 15] = [
        "fixtures/hardening/v2/models/artifact-index-freshness.toml",
        "fixtures/v2/models/atom-lifecycle.toml",
        "fixtures/v2/models/carrier-derivation-freshness.toml",
        "fixtures/v2/models/change-impact-closure.toml",
        "fixtures/hardening/v2/models/complete-impact-fixed-point.toml",
        "fixtures/v2/models/coverage-policy-adoption.toml",
        "fixtures/v2/models/no-laundering.toml",
        "fixtures/v2/models/decision-surface-completeness.toml",
        "fixtures/hardening/v2/models/governance-no-laundering.toml",
        "fixtures/hardening/v2/models/policy-weakening-protocol.toml",
        "fixtures/hardening/v2/models/proof-invalidation.toml",
        "fixtures/v2/models/proof-invalidation.toml",
        "fixtures/v2/models/required-role-coverage.toml",
        "fixtures/hardening/v2/models/terminal-governance-history.toml",
        "fixtures/v2/models/verdict-aggregation.toml",
    ];
    let registered = [
        include_str!("../fixtures/hardening/v2/models/artifact-index-freshness.toml"),
        include_str!("../fixtures/v2/models/atom-lifecycle.toml"),
        include_str!("../fixtures/v2/models/carrier-derivation-freshness.toml"),
        include_str!("../fixtures/v2/models/change-impact-closure.toml"),
        include_str!("../fixtures/hardening/v2/models/complete-impact-fixed-point.toml"),
        include_str!("../fixtures/v2/models/coverage-policy-adoption.toml"),
        include_str!("../fixtures/v2/models/no-laundering.toml"),
        include_str!("../fixtures/v2/models/decision-surface-completeness.toml"),
        include_str!("../fixtures/hardening/v2/models/governance-no-laundering.toml"),
        include_str!("../fixtures/hardening/v2/models/policy-weakening-protocol.toml"),
        include_str!("../fixtures/hardening/v2/models/proof-invalidation.toml"),
        include_str!("../fixtures/v2/models/proof-invalidation.toml"),
        include_str!("../fixtures/v2/models/required-role-coverage.toml"),
        include_str!("../fixtures/hardening/v2/models/terminal-governance-history.toml"),
        include_str!("../fixtures/v2/models/verdict-aggregation.toml"),
    ];
    for (id, source) in MODEL_IDS.into_iter().zip(registered) {
        assert!(
            source.contains(&format!("id = \"{id}\"")),
            "registered finite-model source mismatch: {id}"
        );
    }

    let lifecycle_states = [
        LifecycleState::Proposed,
        LifecycleState::Ratified,
        LifecycleState::Superseded,
        LifecycleState::Withdrawn,
    ];
    let lifecycle_events = [
        LifecycleEvent::Ratify,
        LifecycleEvent::Withdraw,
        LifecycleEvent::Supersede,
    ];
    for (state_tag, state) in (1_u16..).zip(lifecycle_states) {
        for (event_tag, event) in (1_u16..).zip(lifecycle_events) {
            assert_eq!(
                lifecycle_transition(state, event)
                    .ok()
                    .map(|next| next as u16),
                reference::lifecycle_transition(state_tag, event_tag),
                "atom lifecycle state={state_tag} event={event_tag}"
            );
        }
    }

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
    let policy_tag = |value: Option<PolicyLifecycle>| value.map(|item| item as u16);
    let debt_tag = |value: Option<DebtLifecycle>| value.map(|item| item as u16);
    let waiver_tag = |value: Option<WaiverLifecycle>| value.map(|item| item as u16);
    for base_policy in policies {
        for head_policy in policies {
            for base_debt in debts {
                for head_debt in debts {
                    for base_waiver in waivers {
                        for head_waiver in waivers {
                            assert_eq!(
                                governance_transition_valid(
                                    base_policy,
                                    head_policy,
                                    base_debt,
                                    head_debt,
                                    base_waiver,
                                    head_waiver,
                                ),
                                reference::governance_transition_valid(
                                    policy_tag(base_policy),
                                    policy_tag(head_policy),
                                    debt_tag(base_debt),
                                    debt_tag(head_debt),
                                    waiver_tag(base_waiver),
                                    waiver_tag(head_waiver),
                                )
                            );
                        }
                    }
                }
            }
        }
    }

    for mask in 0_u16..(1 << 11) {
        let flag = |bit: u16| mask & (1 << bit) != 0;
        let verdict = if flag(3) {
            Verdict::Pass
        } else {
            Verdict::Fail
        };
        let production = aggregate_assurance(&AggregationInput {
            linked: flag(0),
            required_fresh: flag(1),
            required_selected: flag(2),
            verdicts: vec![verdict],
            exhaustive: flag(4),
            formal: flag(5),
            structural: flag(6),
            reviewed: flag(7),
            debt: flag(8),
            waiver: flag(9),
            reaffirmation: flag(10),
        });
        assert_eq!(
            production
                .iter()
                .map(|class| *class as u16)
                .collect::<Vec<_>>(),
            reference::assurance_classes(
                flag(0),
                flag(1),
                flag(2),
                &[verdict as u16],
                flag(4),
                flag(5),
                flag(6),
                flag(7),
            ),
            "no-laundering state {mask}"
        );
    }

    for mask in 0_u8..4 {
        let positive = mask & 1 != 0;
        let negative = mask & 2 != 0;
        let authority_atom = atom("MODEL-ROLE", "Every required role MUST be present.")
            .expect("finite role authority");
        let oracle = test_oracle(sid("MODEL-ROLE-ORACLE"), digest(10), Vec::new());
        let mut carriers = Vec::new();
        for (role, present, id, byte) in [
            (EvidenceRole::Positive, positive, "MODEL-ROLE-POS", 11),
            (EvidenceRole::Negative, negative, "MODEL-ROLE-NEG", 12),
        ] {
            if present {
                carriers.push(test_carrier(
                    sid(id),
                    digest(byte),
                    Vec::new(),
                    role,
                    vec![test_atom_ref(
                        authority_atom.meta().id().clone(),
                        authority_atom.revision(),
                    )],
                    meta_ref(oracle.meta()),
                    digest(13),
                    Vec::new(),
                    test_payload("scope/v1", vec![1]),
                ));
            }
        }
        let policy = test_policy(
            sid("MODEL-ROLE-POLICY"),
            digest(14),
            Vec::new(),
            digest(15),
            PolicyLifecycle::Active,
            EnforcementMode::Blocking,
            ScopeId::new("model-role-line").expect("valid scope"),
            vec![meta_ref(authority_atom.meta())],
            vec![EvidenceRole::Positive, EvidenceRole::Negative],
        );
        let graph = SpecGraph {
            atoms: vec![authority_atom],
            evidence_carriers: carriers,
            oracles: vec![oracle],
            ..SpecGraph::default()
        };
        let production = required_role_coverage(&graph, &policy);
        let (present, missing) =
            reference::required_roles(&[1, 2], &[(1, positive), (2, negative)]);
        assert_eq!(
            production
                .present
                .iter()
                .map(|role| *role as u16)
                .collect::<Vec<_>>(),
            present
        );
        assert_eq!(
            production
                .missing
                .iter()
                .map(|role| *role as u16)
                .collect::<Vec<_>>(),
            missing
        );
    }

    for mask in 0_u8..16 {
        let keep_second = mask & 1 != 0;
        let keep_negative = mask & 2 != 0;
        let head_blocking = mask & 4 != 0;
        let same_lineage = mask & 8 != 0;
        let subject_a = test_object_ref(ObjectKind::Atom, sid("MODEL-POLICY-A"), digest(20));
        let subject_b = test_object_ref(ObjectKind::Atom, sid("MODEL-POLICY-B"), digest(21));
        let base = test_policy(
            sid("MODEL-POLICY"),
            digest(22),
            Vec::new(),
            digest(23),
            PolicyLifecycle::Active,
            EnforcementMode::Blocking,
            ScopeId::new("model-line-a").expect("valid scope"),
            vec![subject_a.clone(), subject_b.clone()],
            vec![EvidenceRole::Positive, EvidenceRole::Negative],
        );
        let mut head_subjects = vec![subject_a];
        if keep_second {
            head_subjects.push(subject_b);
        }
        let mut head_roles = vec![EvidenceRole::Positive];
        if keep_negative {
            head_roles.push(EvidenceRole::Negative);
        }
        let head = test_policy(
            sid("MODEL-POLICY"),
            digest(24),
            Vec::new(),
            digest(25),
            PolicyLifecycle::Active,
            if head_blocking {
                EnforcementMode::Blocking
            } else {
                EnforcementMode::Advisory
            },
            if same_lineage {
                "model-line-a"
            } else {
                "model-line-b"
            },
            head_subjects,
            head_roles,
        );
        let production = compare_policy(&base, &head);
        let base_subjects = BTreeSet::from([
            String::from("MODEL-POLICY-A"),
            String::from("MODEL-POLICY-B"),
        ]);
        let mut reference_subjects = BTreeSet::from([String::from("MODEL-POLICY-A")]);
        if keep_second {
            reference_subjects.insert(String::from("MODEL-POLICY-B"));
        }
        let mut reference_roles = BTreeSet::from([1_u16]);
        if keep_negative {
            reference_roles.insert(2);
        }
        let independent = reference::policy_ratchet(
            &base_subjects,
            &reference_subjects,
            &BTreeSet::from([1_u16, 2]),
            &reference_roles,
            true,
            head_blocking,
            same_lineage,
        );
        assert_eq!(production.removed_subjects.len(), independent.0.len());
        assert_eq!(production.removed_roles.len(), independent.1.len());
        assert_eq!(production.mode_weakened, independent.2);
        assert_eq!(production.lineage_changed, independent.3);
    }

    for mask in 0_u8..4 {
        let atom_changed = mask & 1 != 0;
        let link_changed = mask & 2 != 0;
        let base_atom =
            atom("MODEL-IMPACT-ATOM", "The base MUST remain current.").expect("finite impact atom");
        let head_atom = if atom_changed {
            atom("MODEL-IMPACT-ATOM", "The head MUST remain current.")
                .expect("changed finite impact atom")
        } else {
            base_atom.clone()
        };
        let base_link = test_implementation_link(
            sid("MODEL-IMPACT-LINK"),
            digest(32),
            vec![meta_ref(base_atom.meta())],
        );
        let head_link = test_implementation_link(
            sid("MODEL-IMPACT-LINK"),
            digest(if link_changed { 33 } else { 32 }),
            vec![meta_ref(base_atom.meta())],
        );
        let base = SpecGraph {
            atoms: vec![base_atom],
            implementation_links: vec![base_link],
            ..SpecGraph::default()
        };
        let head = SpecGraph {
            atoms: vec![head_atom],
            implementation_links: vec![head_link],
            ..SpecGraph::default()
        };
        let production = change_impacts(&base, &head)
            .into_iter()
            .map(|impact| (impact.id.as_str().to_string(), impact.direct))
            .collect::<BTreeSet<_>>();
        let base_reference = BTreeMap::from([
            (String::from("MODEL-IMPACT-ATOM"), (0, Vec::new())),
            (
                String::from("MODEL-IMPACT-LINK"),
                (0, vec![String::from("MODEL-IMPACT-ATOM")]),
            ),
        ]);
        let mut head_reference = base_reference.clone();
        head_reference
            .get_mut("MODEL-IMPACT-ATOM")
            .expect("impact atom row")
            .0 = u8::from(atom_changed);
        head_reference
            .get_mut("MODEL-IMPACT-LINK")
            .expect("impact link row")
            .0 = u8::from(link_changed);
        assert_eq!(
            production,
            reference::transitive_impacts(&base_reference, &head_reference)
        );
    }

    for disposition_count in 0_usize..=2 {
        let impact = Impact {
            kind: ObjectKind::Atom,
            id: sid(&String::from("MODEL-DISPOSITION-ATOM")),
            base: Some(digest(91)),
            head: Some(digest(92)),
            direct: true,
        };
        let dispositions = (0..disposition_count)
            .map(|index| {
                test_change_disposition(
                    sid(&format!("MODEL-DISPOSITION-{index}")),
                    digest(u8::try_from(93 + index).unwrap_or(93)),
                    Vec::new(),
                    ObjectKind::Atom,
                    sid("MODEL-DISPOSITION-ATOM"),
                    impact.base,
                    impact.head,
                    ChangeDispositionKind::Revised,
                )
            })
            .collect::<Vec<_>>();
        let production = ChangeManifest::new(
            sid("MODEL-DISPOSITION-MANIFEST"),
            digest(97),
            digest(98),
            dispositions,
            None,
        )
        .is_ok_and(|manifest| {
            disposition_closure(core::slice::from_ref(&impact), &manifest).is_empty()
        });
        assert_eq!(
            production,
            reference::disposition_closed(
                &[String::from("MODEL-DISPOSITION-ATOM")],
                &(0..disposition_count)
                    .map(|_| String::from("MODEL-DISPOSITION-ATOM"))
                    .collect::<Vec<_>>(),
            )
        );
    }

    for mask in 0_u8..4 {
        let atom_current = mask & 1 != 0;
        let oracle_current = mask & 2 != 0;
        let authority_atom =
            atom("MODEL-FRESH", "Every binding MUST be current.").expect("finite freshness atom");
        let oracle = test_oracle(sid("MODEL-FRESH-ORACLE"), digest(3), Vec::new());
        let carrier = test_carrier(
            sid("MODEL-FRESH-CARRIER"),
            digest(6),
            Vec::new(),
            EvidenceRole::Positive,
            vec![test_atom_ref(
                authority_atom.meta().id().clone(),
                if atom_current {
                    authority_atom.revision()
                } else {
                    digest(4)
                },
            )],
            test_object_ref(
                ObjectKind::Oracle,
                oracle.meta().id().clone(),
                if oracle_current {
                    oracle.meta().digest()
                } else {
                    digest(5)
                },
            ),
            digest(7),
            Vec::new(),
            test_payload("scope/v1", vec![1]),
        );
        let graph = SpecGraph {
            atoms: vec![authority_atom],
            evidence_carriers: vec![carrier.clone()],
            oracles: vec![oracle],
            ..SpecGraph::default()
        };
        assert_eq!(
            carrier_fresh(&graph, &carrier),
            reference::all_current(&[atom_current, oracle_current])
        );
    }

    for mask in 0_u8..8 {
        let source_current = mask & 1 != 0;
        let target_current = mask & 2 != 0;
        let oracle_current = mask & 4 != 0;
        let source = test_implementation_link(sid("MODEL-DERIVE-SOURCE"), digest(70), Vec::new());
        let target = test_finite_model(sid("MODEL-DERIVE-TARGET"), digest(71), Vec::new());
        let oracle = test_oracle(sid("MODEL-DERIVE-ORACLE"), digest(72), Vec::new());
        let binding = |meta: &ObjectMeta, current: bool, stale: u8| {
            ArtifactBinding::Object(test_object_ref(
                meta.kind(),
                meta.id().clone(),
                if current {
                    meta.digest()
                } else {
                    digest(stale)
                },
            ))
        };
        let derivation = test_derivation(
            sid("MODEL-DERIVATION"),
            digest(73),
            Vec::new(),
            vec![binding(source.meta(), source_current, 74)],
            vec![binding(target.meta(), target_current, 75)],
            test_tool("model-generator", "1", digest(76), digest(77)),
            test_object_ref(
                ObjectKind::Oracle,
                oracle.meta().id().clone(),
                if oracle_current {
                    oracle.meta().digest()
                } else {
                    digest(78)
                },
            ),
        );
        let graph = SpecGraph {
            implementation_links: vec![source],
            finite_models: vec![target],
            derivations: vec![derivation.clone()],
            oracles: vec![oracle],
            ..SpecGraph::default()
        };
        assert_eq!(
            derivation_fresh(&graph, &derivation),
            reference::all_current(&[source_current, target_current, oracle_current])
        );
    }

    for mask in 0_u8..32 {
        let current = |bit: u8| mask & (1 << bit) != 0;
        let authority_atom =
            atom("MODEL-PROOF-ATOM", "The proof MUST remain current.").expect("finite proof atom");
        let property = test_formal_property(
            sid("MODEL-PROOF-PROPERTY"),
            digest(80),
            Vec::new(),
            vec![test_atom_ref(
                authority_atom.meta().id().clone(),
                authority_atom.revision(),
            )],
            test_payload("property/v1", vec![1]),
            true,
        );
        let implementation =
            test_implementation_link(sid("MODEL-PROOF-IMPLEMENTATION"), digest(81), Vec::new());
        let assumption = test_assumption(sid("MODEL-PROOF-ASSUMPTION"), digest(82), Vec::new());
        let implementation_binding = ArtifactBinding::Object(test_object_ref(
            ObjectKind::ImplementationLink,
            implementation.meta().id().clone(),
            if current(2) {
                implementation.meta().digest()
            } else {
                digest(83)
            },
        ));
        let proof = test_proof_record(
            sid("MODEL-PROOF"),
            digest(84),
            Vec::new(),
            vec![test_atom_ref(
                authority_atom.meta().id().clone(),
                if current(0) {
                    authority_atom.revision()
                } else {
                    digest(85)
                },
            )],
            test_object_ref(
                ObjectKind::FormalProperty,
                property.meta().id().clone(),
                if current(1) {
                    property.meta().digest()
                } else {
                    digest(86)
                },
            ),
            implementation_binding.clone(),
            vec![test_object_ref(
                ObjectKind::Assumption,
                assumption.meta().id().clone(),
                if current(3) {
                    assumption.meta().digest()
                } else {
                    digest(87)
                },
            )],
            Vec::new(),
            test_tool("model-verifier", "1", digest(88), digest(89)),
            Vec::new(),
            Vec::new(),
            vec![implementation_binding],
            if current(4) {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
        );
        let graph = SpecGraph {
            atoms: vec![authority_atom],
            implementation_links: vec![implementation],
            formal_properties: vec![property],
            assumptions: vec![assumption],
            proof_records: vec![proof.clone()],
            ..SpecGraph::default()
        };
        assert_eq!(
            proof_fresh(&graph, &proof),
            reference::all_current(&[current(0), current(1), current(2), current(3), current(4),])
        );
    }

    const EVIDENCE: &str = "assurance/v1/finite-model-production-conformance.tsv";
    const TOOLS: [&str; 6] = [
        "tests/hegel_properties.rs",
        "crates/buoy-core/src/algorithms.rs",
        "crates/buoy-core/src/model.rs",
        "crates/buoy-reference/src/lib.rs",
        "tools/execution_evidence.rs",
        "tools/digest_binding.rs",
    ];
    let records = MODEL_IDS
        .into_iter()
        .zip(MODEL_PATHS)
        .map(|(model_id, path)| {
            let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
            let states = source
                .lines()
                .find_map(|line| {
                    line.strip_prefix("expected_state_count = ")
                        .or_else(|| line.strip_prefix("expected_reachable_count = "))
                })
                .ok_or_else(|| format!("{model_id}: missing finite-model bound"))?;
            let ledger_id = path
                .strip_prefix("fixtures/")
                .unwrap_or(path)
                .trim_end_matches(".toml")
                .replace('/', ":");
            execution_evidence::passing(
                "finite-model",
                ledger_id,
                path,
                source.as_bytes(),
                &TOOLS,
                &[path, "tests/support/hegel_settings.rs"],
                format!(
                    "model={model_id};states={states};production-reference=equal;fixed-point=true"
                ),
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("finite-model execution evidence must bind exact sources and authorities");
    let expected = execution_evidence::render(&records)
        .expect("finite-model execution evidence must render deterministically");
    execution_evidence::validate_materialized(EVIDENCE, &expected, write)
        .expect("finite-model production-conformance evidence must be current");
}

#[test]
fn every_registered_finite_model_exhaustively_matches_production() {
    validate_registered_finite_models(false);
}

#[test]
#[ignore = "explicit reviewed finite-model production-conformance regeneration only"]
fn write_finite_model_production_conformance_evidence() {
    validate_registered_finite_models(true);
}
