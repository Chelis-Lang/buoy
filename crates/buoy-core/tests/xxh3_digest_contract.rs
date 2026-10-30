//! Frozen v2 digest representation and malformed-input fixture checks.

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::{
    CanonicalValue, DecodeBudget, DecodingError, DigestBinaryError, DigestTextError, ObjectKind,
    ObjectRef, SemanticDigest,
};
use std::fs;

const ROUNDTRIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/xxh3/v2/canonical-digest-roundtrip.tsv"
);
const MALFORMED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/xxh3/v2/malformed-digest-vectors.tsv"
);

#[allow(clippy::expect_used)]
fn sid(value: &str) -> buoy_core::StableId {
    buoy_core::StableId::new(value).expect("valid stable ID")
}

fn test_object_ref(kind: ObjectKind, id: buoy_core::StableId, digest: SemanticDigest) -> ObjectRef {
    ObjectRef::new(kind, id, digest)
}

fn hex_bytes(input: &str) -> Option<Vec<u8>> {
    if input.len() % 2 != 0 {
        return None;
    }
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = char::from(pair[0]).to_digit(16)?;
            let low = char::from(pair[1]).to_digit(16)?;
            u8::try_from((high << 4) | low).ok()
        })
        .collect()
}

#[test]
fn canonical_digest_forms_match_frozen_roundtrip_fixtures() {
    let source = fs::read_to_string(ROUNDTRIP);
    assert!(source.is_ok());
    let Some(source) = source.ok() else {
        return;
    };
    let mut count = 0;
    for line in source.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 7);
        let parsed = SemanticDigest::parse_canonical(fields[5]);
        assert!(parsed.is_ok(), "{}", fields[1]);
        let Some(digest) = parsed.ok() else {
            continue;
        };
        assert_eq!(digest.canonical_text(), fields[5]);
        assert_eq!(hex_bytes(fields[4]).as_deref(), Some(digest.bytes()));
        let binary = hex_bytes(fields[6]);
        assert_eq!(
            binary.as_deref(),
            Some(digest.canonical_binary().as_slice())
        );
        assert_eq!(
            binary
                .as_deref()
                .and_then(|bytes| SemanticDigest::parse_canonical_binary(bytes).ok()),
            Some(digest)
        );
        assert_eq!(
            CanonicalValue::Digest(digest).encode().ok().as_deref(),
            binary.as_deref()
        );
        count += 1;
    }
    assert_eq!(count, 3);
}

#[test]
fn canonical_value_and_reference_ordering_use_complete_v2_digests() {
    let first = SemanticDigest::xxh3_128([0; 16]);
    let second = SemanticDigest::xxh3_128([1; 16]);
    let encoded = CanonicalValue::Set(vec![
        CanonicalValue::Digest(second),
        CanonicalValue::Digest(first),
    ])
    .encode();
    assert!(encoded.is_ok());
    assert_eq!(
        encoded
            .as_deref()
            .ok()
            .and_then(|bytes| { CanonicalValue::decode(bytes, DecodeBudget::v2_default()).ok() }),
        Some(CanonicalValue::Set(vec![
            CanonicalValue::Digest(first),
            CanonicalValue::Digest(second),
        ]))
    );

    let mut references = [
        test_object_ref(ObjectKind::Atom, sid("ATOM-B"), first),
        test_object_ref(ObjectKind::Atom, sid("ATOM-A"), second),
        test_object_ref(ObjectKind::Atom, sid("ATOM-A"), first),
    ];
    references.sort_unstable();
    assert_eq!(references[0].id().as_str(), "ATOM-A");
    assert_eq!(references[0].digest(), first);
    assert_eq!(references[1].id().as_str(), "ATOM-A");
    assert_eq!(references[1].digest(), second);
    assert_eq!(references[2].id().as_str(), "ATOM-B");

    let mut obsolete = Vec::from(&[12_u8][..]);
    obsolete.extend_from_slice(&36_u64.to_be_bytes());
    obsolete.extend_from_slice(&1_u16.to_be_bytes());
    obsolete.extend_from_slice(&32_u16.to_be_bytes());
    obsolete.extend_from_slice(&[0_u8; 32]);
    assert_eq!(
        CanonicalValue::decode(&obsolete, DecodeBudget::v2_default()),
        Err(DecodingError::InvalidDigest)
    );
}

#[test]
fn malformed_digest_fixtures_return_exact_diagnostics() {
    let source = fs::read_to_string(MALFORMED);
    assert!(source.is_ok());
    let Some(source) = source.ok() else {
        return;
    };
    let mut parser_cases = 0;
    for line in source.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5);
        let actual = match fields[2] {
            "text" => SemanticDigest::parse_canonical(fields[3])
                .err()
                .map(DigestTextError::code),
            "binary" => hex_bytes(fields[3]).and_then(|bytes| {
                SemanticDigest::parse_canonical_binary(&bytes)
                    .err()
                    .map(DigestBinaryError::code)
            }),
            "collision" => continue,
            surface => {
                assert_eq!(surface, "text-or-binary");
                None
            }
        };
        assert_eq!(actual, Some(fields[4]), "{}", fields[1]);
        parser_cases += 1;
    }
    assert_eq!(parser_cases, 9);
}
