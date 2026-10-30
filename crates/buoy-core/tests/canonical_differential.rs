#![doc = "Exact production/reference canonical encoding and strict-decoder conformance."]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::{
    CanonicalValue, DecodeBudget, DecodingError, SemanticDigest, semantic_digest_observation,
};
use buoy_reference::Value;

fn cases() -> Vec<(CanonicalValue, Value)> {
    let digest = [0xa5; 16];
    vec![
        (CanonicalValue::False, Value::False),
        (CanonicalValue::True, Value::True),
        (CanonicalValue::U64(u64::MAX), Value::Unsigned(u64::MAX)),
        (CanonicalValue::I64(i64::MIN), Value::Signed(i64::MIN)),
        (
            CanonicalValue::Bytes(vec![0, 1, 255]),
            Value::Bytes(vec![0, 1, 255]),
        ),
        (
            CanonicalValue::Text(String::from("é")),
            Value::Text(String::from("é")),
        ),
        (
            CanonicalValue::Record(vec![
                (1, CanonicalValue::False),
                (2, CanonicalValue::U64(7)),
            ]),
            Value::Record(vec![(1, Value::False), (2, Value::Unsigned(7))]),
        ),
        (
            CanonicalValue::Sequence(vec![CanonicalValue::True, CanonicalValue::False]),
            Value::Sequence(vec![Value::True, Value::False]),
        ),
        (
            CanonicalValue::Set(vec![CanonicalValue::U64(2), CanonicalValue::U64(1)]),
            Value::Set(vec![Value::Unsigned(2), Value::Unsigned(1)]),
        ),
        (CanonicalValue::None, Value::None),
        (
            CanonicalValue::Some(Box::new(CanonicalValue::Text(String::from("value")))),
            Value::Some(Box::new(Value::Text(String::from("value")))),
        ),
        (CanonicalValue::Enum(7, None), Value::Enum(7, None)),
        (
            CanonicalValue::Digest(SemanticDigest::xxh3_128(digest)),
            Value::Digest(2, digest.to_vec()),
        ),
        (
            CanonicalValue::Digest(SemanticDigest::blake3_256([0xb6; 32])),
            Value::Digest(3, vec![0xb6; 32]),
        ),
        (
            CanonicalValue::Digest(SemanticDigest::sha_256([0xc7; 32])),
            Value::Digest(4, vec![0xc7; 32]),
        ),
    ]
}

#[test]
fn every_type_tag_has_exact_independent_bytes_and_round_trip() {
    for (production, reference) in cases() {
        let production_bytes = production.encode();
        let reference_bytes = reference.encode();
        assert_eq!(
            production_bytes.as_ref().ok(),
            reference_bytes.as_ref().ok()
        );
        let Some(bytes) = production_bytes.ok() else {
            continue;
        };
        let production_decoded = CanonicalValue::decode(&bytes, DecodeBudget::v2_default());
        assert_eq!(
            production_decoded
                .and_then(|value| value.encode().map_err(|_| DecodingError::InvalidSet)),
            Ok(bytes.clone())
        );
        let reference_decoded = Value::decode(&bytes, 128, 5_000_000);
        assert_eq!(
            reference_decoded.and_then(|value| value.encode()),
            Ok(bytes)
        );
    }
}

#[test]
fn production_and_reference_v3_envelopes_match() {
    let production = semantic_digest_observation(
        "canonical-differential",
        vec![(1, CanonicalValue::Bytes(vec![0, 1, 2, 255]))],
    );
    let reference = buoy_reference::semantic_preimage(
        2,
        "canonical-differential",
        vec![(1, Value::Bytes(vec![0, 1, 2, 255]))],
    );
    assert_eq!(
        production
            .ok()
            .map(|value| value.canonical_preimage().to_vec()),
        reference.ok()
    );
}

#[test]
fn strict_decoder_rejects_trailing_unknown_length_nfc_set_option_digest_and_budgets() {
    let mut trailing = CanonicalValue::False.encode().unwrap_or_default();
    trailing.push(0);
    assert_eq!(
        CanonicalValue::decode(&trailing, DecodeBudget::v2_default()),
        Err(DecodingError::TrailingBytes)
    );

    assert_eq!(
        CanonicalValue::decode(&[13, 0, 0, 0, 0, 0, 0, 0, 0], DecodeBudget::v2_default()),
        Err(DecodingError::UnknownTypeTag)
    );
    assert_eq!(
        CanonicalValue::decode(&[0, 0, 0, 0, 0, 0, 0, 0, 1, 0], DecodeBudget::v2_default()),
        Err(DecodingError::InvalidLength)
    );

    let non_nfc = [5, 0, 0, 0, 0, 0, 0, 0, 3, b'e', 0xcc, 0x81];
    assert_eq!(
        CanonicalValue::decode(&non_nfc, DecodeBudget::v2_default()),
        Err(DecodingError::NonCanonicalText)
    );

    let element = CanonicalValue::U64(1).encode().unwrap_or_default();
    let mut unordered_set = vec![8];
    let payload_length = 8 + element.len() * 2;
    unordered_set.extend_from_slice(&(payload_length as u64).to_be_bytes());
    unordered_set.extend_from_slice(&2_u64.to_be_bytes());
    unordered_set.extend_from_slice(&element);
    unordered_set.extend_from_slice(&element);
    assert_eq!(
        CanonicalValue::decode(&unordered_set, DecodeBudget::v2_default()),
        Err(DecodingError::InvalidSet)
    );

    let malformed_option = [10, 0, 0, 0, 0, 0, 0, 0, 0];
    assert!(CanonicalValue::decode(&malformed_option, DecodeBudget::v2_default()).is_err());

    let mut digest = CanonicalValue::Digest(SemanticDigest::xxh3_128([1; 16]))
        .encode()
        .unwrap_or_default();
    digest[10] = 3;
    assert_eq!(
        CanonicalValue::decode(&digest, DecodeBudget::v2_default()),
        Err(DecodingError::InvalidDigest)
    );

    let valid = CanonicalValue::Some(Box::new(CanonicalValue::False))
        .encode()
        .unwrap_or_default();
    assert_eq!(
        CanonicalValue::decode(
            &valid,
            DecodeBudget {
                max_bytes: 1024,
                max_nesting: 1,
                max_values: 10,
            },
        ),
        Err(DecodingError::NestingBudget)
    );
    assert_eq!(
        CanonicalValue::decode(
            &valid,
            DecodeBudget {
                max_bytes: 1,
                max_nesting: 128,
                max_values: 10,
            },
        ),
        Err(DecodingError::ByteBudget)
    );
}
