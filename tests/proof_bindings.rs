#![doc = "Exact artifact bindings for selected production Verus proofs."]

use buoy::core::{CanonicalValue, SemanticDigest, semantic_digest};
use toml::Value;

fn artifact(media_type: &str, bytes: &[u8]) -> Option<SemanticDigest> {
    semantic_digest(
        "raw-artifact",
        vec![
            (1, CanonicalValue::Text(String::from(media_type))),
            (2, CanonicalValue::Bytes(bytes.to_vec())),
        ],
    )
    .ok()
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for component in path {
        current = current.get(*component)?;
    }
    current.as_str()
}

#[test]
fn selected_verus_proof_bindings_are_current() {
    let implementation = artifact(
        "text/x-rust",
        include_bytes!("../crates/buoy-core/src/algorithms.rs"),
    );
    let proof = artifact(
        "text/x-rust-verus",
        include_bytes!("../proofs/core/integrity.rs"),
    );
    let negative = artifact(
        "text/x-rust-verus",
        include_bytes!("../proofs/core/integrity_negative.rs"),
    );
    assert!(implementation.is_some() && proof.is_some() && negative.is_some());
    let (Some(implementation), Some(proof), Some(negative)) = (implementation, proof, negative)
    else {
        return;
    };

    let record = toml::from_str::<Value>(include_str!("../proofs/core/proof-record.toml")).ok();
    let derivation =
        toml::from_str::<Value>(include_str!("../proofs/core/derivation-record.toml")).ok();
    assert!(record.is_some() && derivation.is_some());
    let (Some(record), Some(derivation)) = (record, derivation) else {
        return;
    };
    assert_eq!(
        string_at(&record, &["implementation", "digest"]),
        Some(implementation.canonical_text().as_str())
    );
    assert_eq!(
        string_at(&record, &["proof_source", "digest"]),
        Some(proof.canonical_text().as_str())
    );
    assert_eq!(
        string_at(&record, &["negative_source", "digest"]),
        Some(negative.canonical_text().as_str())
    );
    assert_eq!(
        string_at(&record, &["derivation"]),
        Some("DERIV-CORE-INTEGRITY-REPRESENTATION-V2")
    );
    assert_eq!(
        string_at(&record, &["verifier", "version"]),
        Some("0.2026.07.12.0b42f4c")
    );
    assert_eq!(
        string_at(&record, &["negative_source", "expected_diagnostic"]),
        Some("postcondition not satisfied")
    );
    assert_eq!(
        string_at(&derivation, &["source", "digest"]),
        Some(implementation.canonical_text().as_str())
    );
    assert_eq!(
        string_at(&derivation, &["target", "digest"]),
        Some(proof.canonical_text().as_str())
    );
    assert_eq!(
        string_at(&derivation, &["assurance_effect"]),
        Some(
            "The derivation makes duplicated representation explicit; it does not prove translation equivalence or add an assurance class."
        )
    );
    assert_eq!(
        string_at(&derivation, &["consistency_oracle", "command"]),
        Some("devenv shell -- buoy-core-proofs")
    );
}
