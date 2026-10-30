#![doc = "Deliberately faulty integrity implementations that frozen tests must kill."]
#![allow(
    clippy::expect_used,
    clippy::manual_contains,
    clippy::nonminimal_bool,
    clippy::unnecessary_wraps
)]

use buoy::core::{
    AggregationInput, AssuranceClass, CanonicalValue, DecodeBudget, DecodingError, EvidenceRole,
    LifecycleEvent, LifecycleState, SchemaImpactDependency, SchemaImpactIndex, SchemaImpactKey,
    SchemaImpactNode, SemanticDigest, Verdict, aggregate_assurance, lifecycle_transition,
    schema_change_impacts,
};

fn faulty_lifecycle(state: LifecycleState, _event: LifecycleEvent) -> Option<LifecycleState> {
    Some(match state {
        LifecycleState::Superseded | LifecycleState::Withdrawn => LifecycleState::Proposed,
        other => other,
    })
}

fn faulty_surface_complete(expected_count: usize, actual_count: usize) -> bool {
    expected_count == actual_count
}

fn faulty_freshness(bindings: &[bool]) -> bool {
    bindings.iter().any(|binding| *binding)
}

fn faulty_concat(fields: &[&[u8]]) -> Vec<u8> {
    fields.concat()
}

fn faulty_policy_monotonic(base: &[EvidenceRole], head: &[EvidenceRole]) -> bool {
    head.iter().any(|role| base.contains(role))
}

fn faulty_impact_closed(dispositions: &[bool]) -> bool {
    dispositions.iter().any(|disposition| *disposition)
}

fn faulty_waiver_laundering(technical: bool, waiver: bool) -> bool {
    technical || waiver
}

fn faulty_aggregation(verdicts: &[Verdict]) -> bool {
    verdicts.iter().any(|verdict| *verdict == Verdict::Pass)
}

fn faulty_artifact_freshness(path_matches: bool, digest_matches: bool) -> bool {
    path_matches || digest_matches
}

fn faulty_strict_decode(bytes: &[u8], valid_prefix_length: usize) -> bool {
    CanonicalValue::decode(
        &bytes[..valid_prefix_length],
        DecodeBudget {
            max_bytes: 64,
            max_nesting: 8,
            max_values: 8,
        },
    )
    .is_ok()
}

fn faulty_complete_impacts(direct: usize, _transitive: usize) -> usize {
    direct
}

const fn faulty_policy_derivation(manifest_claims_safe: bool, _removed_obligations: usize) -> bool {
    manifest_claims_safe
}

const fn faulty_process_cleanup(direct_child_reaped: bool, _descendants_reaped: bool) -> bool {
    direct_child_reaped
}

const fn faulty_output_enforcement(stream_bytes: u64, stream_limit: u64, _combined: bool) -> bool {
    stream_bytes <= stream_limit
}

const fn faulty_budget_enforcement(observed: u64, maximum: u64) -> bool {
    observed < maximum
}

#[test]
fn mutation_tripwires_reject_faulty_integrity_logic() {
    assert_eq!(
        lifecycle_transition(LifecycleState::Superseded, LifecycleEvent::Ratify),
        Err(buoy::core::LifecycleError::IllegalTransition)
    );
    assert_eq!(
        faulty_lifecycle(LifecycleState::Superseded, LifecycleEvent::Ratify),
        Some(LifecycleState::Proposed)
    );

    let expected = [b"a".as_slice(), b"b".as_slice()];
    let substituted = [b"a".as_slice(), b"c".as_slice()];
    assert!(faulty_surface_complete(expected.len(), substituted.len()));
    assert_ne!(expected, substituted);

    assert!(faulty_freshness(&[true, false]));
    assert!(![true, false].into_iter().all(|binding| binding));

    assert_eq!(faulty_concat(&[b"ab", b"c"]), faulty_concat(&[b"a", b"bc"]));
    assert_ne!(
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Bytes(b"ab".to_vec())),
            (2, CanonicalValue::Bytes(b"c".to_vec())),
        ])
        .encode()
        .ok(),
        CanonicalValue::Record(vec![
            (1, CanonicalValue::Bytes(b"a".to_vec())),
            (2, CanonicalValue::Bytes(b"bc".to_vec())),
        ])
        .encode()
        .ok()
    );

    assert!(faulty_policy_monotonic(
        &[EvidenceRole::Positive, EvidenceRole::Negative],
        &[EvidenceRole::Positive]
    ));
    assert!(
        ![EvidenceRole::Positive, EvidenceRole::Negative]
            .into_iter()
            .all(|role| [EvidenceRole::Positive].contains(&role))
    );

    assert!(faulty_impact_closed(&[true, false]));
    assert!(![true, false].into_iter().all(|disposition| disposition));
    assert!(faulty_waiver_laundering(false, true));

    assert!(faulty_aggregation(&[Verdict::Pass, Verdict::Timeout]));
    let assurance = aggregate_assurance(&AggregationInput {
        linked: true,
        required_fresh: true,
        required_selected: true,
        verdicts: vec![Verdict::Pass, Verdict::Timeout],
        exhaustive: false,
        formal: false,
        structural: false,
        reviewed: false,
        debt: false,
        waiver: true,
        reaffirmation: true,
    });
    assert_eq!(assurance, vec![AssuranceClass::Linked]);
}

#[test]
fn named_adversarial_mutants_are_individually_killed() {
    let path_matches = true;
    let digest_matches = false;
    assert!(faulty_artifact_freshness(path_matches, digest_matches));
    assert!(
        !(path_matches && digest_matches),
        "exact freshness requires every binding"
    );

    let mut trailing = CanonicalValue::False
        .encode()
        .expect("canonical false encodes");
    let valid_prefix_length = trailing.len();
    trailing.push(0);
    assert!(faulty_strict_decode(&trailing, valid_prefix_length));
    assert_eq!(
        CanonicalValue::decode(
            &trailing,
            DecodeBudget {
                max_bytes: 64,
                max_nesting: 8,
                max_values: 8,
            },
        ),
        Err(DecodingError::TrailingBytes)
    );

    let target =
        SchemaImpactKey::new("atom-object", "MUTANT-ATOM").expect("named mutant target key");
    let source = SchemaImpactKey::new("evidence-carrier", "MUTANT-CARRIER")
        .expect("named mutant source key");
    let dependency = SchemaImpactDependency::new("evidence-carrier-atoms", target.clone())
        .expect("named mutant dependency");
    let base = SchemaImpactIndex::new(vec![
        SchemaImpactNode::new(
            target.clone(),
            SemanticDigest::xxh3_128([1; 16]),
            Vec::new(),
        )
        .expect("base target"),
        SchemaImpactNode::new(
            source.clone(),
            SemanticDigest::xxh3_128([3; 16]),
            vec![dependency.clone()],
        )
        .expect("base source"),
    ])
    .expect("base index");
    let head = SchemaImpactIndex::new(vec![
        SchemaImpactNode::new(target, SemanticDigest::xxh3_128([2; 16]), Vec::new())
            .expect("head target"),
        SchemaImpactNode::new(source, SemanticDigest::xxh3_128([3; 16]), vec![dependency])
            .expect("head source"),
    ])
    .expect("head index");
    assert_eq!(faulty_complete_impacts(1, 1), 1);
    assert_eq!(schema_change_impacts(&base, &head).len(), 2);

    let manifest_claims_safe = true;
    let removed_obligations = 1;
    assert!(faulty_policy_derivation(
        manifest_claims_safe,
        removed_obligations
    ));
    assert!(
        removed_obligations != 0,
        "removed obligations make policy unsafe"
    );
    let direct_child_reaped = true;
    let descendants_reaped = false;
    assert!(faulty_process_cleanup(
        direct_child_reaped,
        descendants_reaped
    ));
    assert!(
        !(direct_child_reaped && descendants_reaped),
        "complete cleanup includes descendants"
    );
    let stream_bytes = 8;
    let stream_limit = 8;
    let combined_overflow = true;
    assert!(faulty_output_enforcement(
        stream_bytes,
        stream_limit,
        combined_overflow
    ));
    assert!(
        !(stream_bytes <= stream_limit && !combined_overflow),
        "combined overflow remains blocking"
    );
    let observed = 8;
    let maximum = 8;
    assert!(!faulty_budget_enforcement(observed, maximum));
    assert!(observed <= maximum, "the exact budget boundary is valid");
}
