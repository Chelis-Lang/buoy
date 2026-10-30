#![doc = "Focused controls for every public canonical value rejection and accessor."]
#![allow(clippy::expect_used, clippy::too_many_lines)]

#[path = "algorithms_model_coverage_controls.rs"]
mod model_coverage_controls;

use buoy_core::*;

fn envelope(tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![tag];
    bytes.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn envelope_with_length(tag: u8, length: u64, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![tag];
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn digest_binary_display_and_error_codes_are_complete() {
    let digest = SemanticDigest::xxh3_128([0xabu8; 16]);
    assert_eq!(digest.to_string(), digest.canonical_text());

    let mut wrong_tag = digest.canonical_binary();
    wrong_tag[0] = 11;
    assert_eq!(
        SemanticDigest::parse_canonical_binary(&wrong_tag),
        Err(DigestBinaryError::TypeTag)
    );
    let mut wrong_length = digest.canonical_binary();
    wrong_length[8] = 19;
    assert_eq!(
        SemanticDigest::parse_canonical_binary(&wrong_length),
        Err(DigestBinaryError::Length)
    );
    assert_eq!(
        SemanticDigest::parse_canonical_binary(&wrong_length[..28]),
        Err(DigestBinaryError::Length)
    );
    let mut wrong_digest_length = digest.canonical_binary();
    wrong_digest_length[12] = 15;
    assert_eq!(
        SemanticDigest::parse_canonical_binary(&wrong_digest_length),
        Err(DigestBinaryError::Length)
    );

    assert_eq!(DigestBinaryError::TypeTag.code(), "DET-DIGEST-TYPE-TAG");
    assert_eq!(DigestBinaryError::Algorithm.code(), "DET-DIGEST-ALGORITHM");
    assert_eq!(DigestBinaryError::Length.code(), "DET-DIGEST-LENGTH");
    assert_eq!(
        EncodingError::InvalidRecordTags.code(),
        "DET-ENC-RECORD-TAGS"
    );
    assert_eq!(EncodingError::DuplicateSetValue.code(), "DET-DUP-SET-VALUE");
    assert_eq!(EncodingError::NonCanonicalText.code(), "DET-ENC-TEXT-NFC");
    for (error, expected) in [
        (DecodingError::ByteBudget, "DET-ENC-BUDGET-BYTES"),
        (DecodingError::NestingBudget, "DET-ENC-BUDGET-NESTING"),
        (DecodingError::ValueBudget, "DET-ENC-BUDGET-VALUES"),
        (DecodingError::UnknownTypeTag, "DET-ENC-UNKNOWN-TYPE-TAG"),
        (DecodingError::InvalidLength, "DET-ENC-LENGTH"),
        (DecodingError::NonCanonicalText, "DET-ENC-TEXT-NFC"),
        (DecodingError::InvalidRecord, "DET-ENC-RECORD-TAGS"),
        (DecodingError::InvalidSequence, "DET-ENC-SEQUENCE"),
        (DecodingError::InvalidSet, "DET-ORDER-SET"),
        (DecodingError::InvalidOption, "DET-ENC-OPTION"),
        (DecodingError::InvalidEnum, "DET-ENC-ENUM"),
        (DecodingError::InvalidDigest, "DET-ENC-DIGEST"),
        (DecodingError::TrailingBytes, "DET-ENC-TRAILING-BYTES"),
    ] {
        assert_eq!(error.code(), expected);
    }
}

#[test]
fn canonical_encoder_and_decoder_cover_nested_failures_and_budgets() {
    assert_eq!(
        CanonicalValue::Record(vec![(2, CanonicalValue::False), (1, CanonicalValue::True),])
            .encode(),
        Err(EncodingError::InvalidRecordTags)
    );
    assert_eq!(
        CanonicalValue::Enum(
            1,
            Some(Box::new(CanonicalValue::Text(String::from("e\u{301}")))),
        )
        .encode(),
        Err(EncodingError::NonCanonicalText)
    );
    assert_eq!(
        CanonicalValue::Sequence(vec![CanonicalValue::Text(String::from("e\u{301}"))]).encode(),
        Err(EncodingError::NonCanonicalText)
    );
    assert_eq!(
        semantic_digest("e\u{301}", Vec::new()),
        Err(EncodingError::NonCanonicalText)
    );
    assert_eq!(
        semantic_digest(
            "valid-domain",
            vec![(2, CanonicalValue::False), (1, CanonicalValue::True),],
        ),
        Err(EncodingError::InvalidRecordTags)
    );

    let false_bytes = CanonicalValue::False.encode().expect("false encodes");
    assert_eq!(
        CanonicalValue::decode(&[0], DecodeBudget::v2_default()),
        Err(DecodingError::InvalidLength)
    );
    assert_eq!(
        CanonicalValue::decode(&envelope(5, &[0xff]), DecodeBudget::v2_default()),
        Err(DecodingError::NonCanonicalText)
    );
    assert_eq!(
        CanonicalValue::decode(&envelope(11, &[]), DecodeBudget::v2_default()),
        Err(DecodingError::InvalidEnum)
    );
    assert_eq!(
        CanonicalValue::decode(
            &false_bytes,
            DecodeBudget {
                max_bytes: u64::MAX,
                max_nesting: u16::MAX,
                max_values: 0,
            },
        ),
        Err(DecodingError::ValueBudget)
    );
    assert_eq!(
        CanonicalValue::decode(&envelope_with_length(4, 1, &[]), DecodeBudget::v2_default(),),
        Err(DecodingError::InvalidLength)
    );

    for tag in [1, 2, 3] {
        assert_eq!(
            CanonicalValue::decode(&envelope(tag, &[0]), DecodeBudget::v2_default()),
            Err(DecodingError::InvalidLength)
        );
    }
    for (algorithm, length) in [(3_u16, 16_u16), (2_u16, 15_u16)] {
        let mut payload = algorithm.to_be_bytes().to_vec();
        payload.extend_from_slice(&length.to_be_bytes());
        payload.extend_from_slice(&[0_u8; 16]);
        assert_eq!(
            CanonicalValue::decode(&envelope(12, &payload), DecodeBudget::v2_default()),
            Err(DecodingError::InvalidDigest)
        );
    }

    let mut zero_variant = 0_u16.to_be_bytes().to_vec();
    zero_variant.extend_from_slice(&CanonicalValue::None.encode().expect("none encodes"));
    assert_eq!(
        CanonicalValue::decode(&envelope(11, &zero_variant), DecodeBudget::v2_default()),
        Err(DecodingError::InvalidEnum)
    );
    let mut malformed_variant = 1_u16.to_be_bytes().to_vec();
    malformed_variant.extend_from_slice(&envelope(13, &[]));
    assert_eq!(
        CanonicalValue::decode(
            &envelope(11, &malformed_variant),
            DecodeBudget::v2_default(),
        ),
        Err(DecodingError::InvalidEnum)
    );
    let mut scalar_variant = 1_u16.to_be_bytes().to_vec();
    scalar_variant.extend_from_slice(&false_bytes);
    assert_eq!(
        CanonicalValue::decode(&envelope(11, &scalar_variant), DecodeBudget::v2_default(),),
        Err(DecodingError::InvalidEnum)
    );
    let enum_some = CanonicalValue::Enum(1, Some(Box::new(CanonicalValue::U64(7))));
    assert_eq!(
        CanonicalValue::decode(
            &enum_some.encode().expect("enum some encodes"),
            DecodeBudget::v2_default(),
        ),
        Ok(enum_some)
    );
    assert_eq!(
        CanonicalValue::decode(&envelope(9, &[0]), DecodeBudget::v2_default()),
        Err(DecodingError::InvalidOption)
    );
    let mut trailing_option = false_bytes.clone();
    trailing_option.push(0);
    assert_eq!(
        CanonicalValue::decode(&envelope(10, &trailing_option), DecodeBudget::v2_default()),
        Err(DecodingError::InvalidOption)
    );
    let mut trailing_enum = 1_u16.to_be_bytes().to_vec();
    trailing_enum.extend_from_slice(&CanonicalValue::None.encode().expect("none encodes"));
    trailing_enum.push(0);
    assert_eq!(
        CanonicalValue::decode(&envelope(11, &trailing_enum), DecodeBudget::v2_default()),
        Err(DecodingError::InvalidEnum)
    );

    assert_eq!(
        CanonicalValue::decode(&envelope(6, &[]), DecodeBudget::v2_default()),
        Err(DecodingError::InvalidLength)
    );
    assert_eq!(
        CanonicalValue::decode(
            &envelope(6, &1_u64.to_be_bytes()),
            DecodeBudget::v2_default(),
        ),
        Err(DecodingError::InvalidRecord)
    );
    let mut missing_sequence_value = 1_u64.to_be_bytes().to_vec();
    assert_eq!(
        CanonicalValue::decode(
            &envelope(7, &missing_sequence_value),
            DecodeBudget::v2_default(),
        ),
        Err(DecodingError::InvalidLength)
    );
    missing_sequence_value[7] = 9;
    assert_eq!(
        CanonicalValue::decode(
            &envelope(7, &missing_sequence_value),
            DecodeBudget {
                max_bytes: u64::MAX,
                max_nesting: u16::MAX,
                max_values: 100,
            },
        ),
        Err(DecodingError::ValueBudget)
    );

    let mut zero_tag_record = 1_u64.to_be_bytes().to_vec();
    zero_tag_record.extend_from_slice(&0_u16.to_be_bytes());
    zero_tag_record.extend_from_slice(&false_bytes);
    assert_eq!(
        CanonicalValue::decode(&envelope(6, &zero_tag_record), DecodeBudget::v2_default()),
        Err(DecodingError::InvalidRecord)
    );

    let mut empty_count_with_extra = 0_u64.to_be_bytes().to_vec();
    empty_count_with_extra.push(0);
    for (tag, expected) in [
        (6, DecodingError::InvalidRecord),
        (7, DecodingError::InvalidSequence),
        (8, DecodingError::InvalidSet),
    ] {
        assert_eq!(
            CanonicalValue::decode(
                &envelope(tag, &empty_count_with_extra),
                DecodeBudget::v2_default(),
            ),
            Err(expected)
        );
    }

    assert_eq!(
        CanonicalValue::decode(
            &envelope(7, &2_u64.to_be_bytes()),
            DecodeBudget {
                max_bytes: u64::MAX,
                max_nesting: u16::MAX,
                max_values: 1,
            },
        ),
        Err(DecodingError::ValueBudget)
    );

    let mut duplicate_tag_record = 2_u64.to_be_bytes().to_vec();
    for _ in 0..2 {
        duplicate_tag_record.extend_from_slice(&1_u16.to_be_bytes());
        duplicate_tag_record.extend_from_slice(&false_bytes);
    }
    assert_eq!(
        CanonicalValue::decode(
            &envelope(6, &duplicate_tag_record),
            DecodeBudget::v2_default(),
        ),
        Err(DecodingError::InvalidRecord)
    );

    let mut record_unknown = 1_u64.to_be_bytes().to_vec();
    record_unknown.extend_from_slice(&1_u16.to_be_bytes());
    record_unknown.extend_from_slice(&envelope(13, &[]));
    assert_eq!(
        CanonicalValue::decode(&envelope(6, &record_unknown), DecodeBudget::v2_default()),
        Err(DecodingError::UnknownTypeTag)
    );
    let mut record_bad_child = 1_u64.to_be_bytes().to_vec();
    record_bad_child.extend_from_slice(&1_u16.to_be_bytes());
    record_bad_child.extend_from_slice(&envelope(0, &[0]));
    assert_eq!(
        CanonicalValue::decode(&envelope(6, &record_bad_child), DecodeBudget::v2_default(),),
        Err(DecodingError::InvalidRecord)
    );
}

#[cfg(feature = "coverage-controls")]
#[test]
fn injected_observation_control_rejects_a_forced_full_digest_collision() {
    let digest = SemanticDigest::xxh3_128([42; 16]);
    assert_eq!(
        validate_digest_observations_control(vec![
            (digest, b"left".to_vec()),
            (digest, b"right".to_vec()),
        ]),
        Err(DigestObservationError::ObservedCollision)
    );
}

#[test]
fn observations_and_path_rejections_expose_stable_public_results() {
    let observations = observe_canonical_preimages(vec![b"alpha".to_vec(), b"beta".to_vec()])
        .expect("distinct observations are unique");
    assert_eq!(observations.len(), 2);
    assert_eq!(observations[0].canonical_preimage(), b"alpha");
    assert_eq!(
        observations[0].digest().algorithm(),
        DigestAlgorithm::Xxh3_128
    );
    assert_eq!(
        observe_canonical_preimages(vec![b"same".to_vec(), b"same".to_vec()]),
        Err(DigestObservationError::DuplicateCanonicalPreimage)
    );
    assert_eq!(
        DigestObservationError::DuplicateCanonicalPreimage.code(),
        "DET-DUP-CANONICAL-PREIMAGE"
    );

    assert!(RepositoryPath::new("a").is_ok());
    assert!(RepositoryPath::new("COM10/file").is_ok());
    assert!(RepositoryPath::new("COM0/file").is_ok());
    for (input, expected) in [
        ("", PathError::NotRelative),
        ("/root", PathError::NotRelative),
        ("trailing/", PathError::NotRelative),
        ("bad\\path", PathError::ForbiddenCharacter),
        ("bad\0path", PathError::ForbiddenCharacter),
        ("C:drive", PathError::NotRelative),
        ("bad./path", PathError::ForbiddenCharacter),
        ("bad?/path", PathError::ForbiddenCharacter),
        ("bad\u{1}/path", PathError::ForbiddenCharacter),
        ("a//b", PathError::InvalidSegment),
        ("./path", PathError::InvalidSegment),
        ("../path", PathError::InvalidSegment),
        ("CON/file", PathError::ReservedBasename),
        ("COM1/file", PathError::ReservedBasename),
        ("LPT1/file", PathError::ReservedBasename),
    ] {
        assert_eq!(RepositoryPath::new(input), Err(expected));
    }
    for (error, expected) in [
        (PathError::NotRelative, "DET-PATH-NOT-RELATIVE"),
        (PathError::InvalidSegment, "DET-PATH-SEGMENT"),
        (PathError::ForbiddenCharacter, "DET-PATH-FORBIDDEN"),
        (PathError::ReservedBasename, "DET-PATH-RESERVED"),
    ] {
        assert_eq!(error.code(), expected);
    }
    let duplicate = RepositoryPath::new("same/path").expect("canonical path");
    assert_eq!(
        canonical_paths(vec![duplicate.clone(), duplicate]),
        Err(PathSetError::CanonicalCollision)
    );
}
