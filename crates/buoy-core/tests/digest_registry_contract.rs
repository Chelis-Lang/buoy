#![doc = "Closed digest registry, encoding, and active-dispatch contracts."]
#![allow(clippy::expect_used)]

#[cfg(feature = "coverage-controls")]
use buoy_core::encoding_error_control;
use buoy_core::{
    ActiveAlgorithm, ActiveAlgorithmError, ArtifactBindingAttachment,
    ArtifactBindingTransactionStage, AtomRevisionRef, CanonicalValue, DecodeBudget,
    DigestAlgorithm, DigestAssuranceClass, EncodingError, ImplementationBinding,
    ImplementationBindingRef, LogicalMount, LogicalMountId, NavigationKind, ParsedItemIdentity,
    RepositoryPath, SchemaId, SemanticDigest, SourceItemKind, SourceNode, SourceNodeRef, StableId,
    artifact_binding_transaction_state, mount_relative_path, project_logical_path, semantic_digest,
    semantic_digest_observation, semantic_digest_v2, validate_artifact_attachment,
};

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[test]
#[allow(clippy::too_many_lines)]
fn feature_profiles_cover_default_dispatch_and_error_translation() {
    for algorithm in DigestAlgorithm::ALL {
        assert!(!algorithm.cargo_feature().is_empty());
        std::hint::black_box(algorithm.is_available());
        let _ = std::hint::black_box(ActiveAlgorithm::new(algorithm));
    }
    let _ = std::hint::black_box(ActiveAlgorithm::build_default());
    assert_eq!(
        ActiveAlgorithmError::Unavailable(DigestAlgorithm::Xxh3_128).code(),
        "DIG-ALGORITHM-UNAVAILABLE"
    );
    assert_eq!(
        EncodingError::UnavailableAlgorithm.code(),
        "DIG-ALGORITHM-UNAVAILABLE"
    );
    assert_eq!(
        SemanticDigest::parse_canonical_binary(&[]).expect_err("short digest must reject"),
        buoy_core::DigestBinaryError::Length
    );
    assert!(
        CanonicalValue::decode(&[12, 0, 0, 0, 0, 0, 0, 0, 0], DecodeBudget::v2_default(),).is_err()
    );
    let mut mismatched_digest_payload = vec![12];
    mismatched_digest_payload.extend_from_slice(&21_u64.to_be_bytes());
    mismatched_digest_payload.extend_from_slice(&2_u16.to_be_bytes());
    mismatched_digest_payload.extend_from_slice(&16_u16.to_be_bytes());
    mismatched_digest_payload.extend_from_slice(&[0_u8; 17]);
    assert!(
        CanonicalValue::decode(&mismatched_digest_payload, DecodeBudget::v2_default()).is_err()
    );
    let _ = std::hint::black_box(semantic_digest_v2("coverage-control/v1", Vec::new()));
    let _ = std::hint::black_box(semantic_digest("coverage-control/v1", Vec::new()));
    let _ = std::hint::black_box(semantic_digest_observation(
        "coverage-control/v1",
        Vec::new(),
    ));
    let mount = LogicalMount::new(
        LogicalMountId::new("project").expect("mount ID"),
        RepositoryPath::new("project").expect("mount prefix"),
    );
    let relative = RepositoryPath::new("src/lib.rs").expect("relative path");
    let projected = project_logical_path(&mount, &relative).expect("logical projection");
    assert_eq!(mount_relative_path(&mount, &projected), Some(relative));
    assert_eq!(mount_relative_path(&mount, mount.prefix()), None);
    for failure in [
        ArtifactBindingTransactionStage::RemoveUncommittedObject,
        ArtifactBindingTransactionStage::RetryCleanup,
    ] {
        let state =
            artifact_binding_transaction_state(true, Some(b"prior"), b"next", Some(failure));
        assert!(state.object_present);
        assert!(!state.committed);
    }
    #[cfg(feature = "coverage-controls")]
    {
        assert_eq!(
            encoding_error_control(),
            Err(EncodingError::UnavailableAlgorithm)
        );
        assert_eq!(
            buoy_core::encoding_digest_error_control(),
            Err(EncodingError::UnavailableAlgorithm)
        );
        if let Ok(attachment) = ArtifactBindingAttachment::new(
            StableId::new("COVERAGE-ATTACHMENT").expect("relation"),
            RepositoryPath::new("artifact.bin").expect("artifact path"),
            SourceNodeRef::new(SemanticDigest::xxh3_128([1; 16])),
            ImplementationBindingRef::new(SemanticDigest::xxh3_128([2; 16])),
        ) {
            std::hint::black_box(attachment.record());
            std::hint::black_box(attachment.digest());
        }
        if let Ok(active) = ActiveAlgorithm::build_default() {
            let artifact_path = RepositoryPath::new("generic.bin").expect("artifact path");
            let item = ParsedItemIdentity::new(
                SchemaId::new("artifact.opaque-bytes/v1").expect("artifact schema"),
                artifact_path.clone(),
                SourceItemKind::new("artifact/file/v1").expect("artifact kind"),
                b"generic-artifact".to_vec(),
                None,
            )
            .expect("artifact item");
            let node = SourceNode::new(item, Vec::new()).expect("artifact node");
            let requirement = SourceNodeRef::new(
                active
                    .compute(active.algorithm(), b"generic-requirement")
                    .expect("requirement digest"),
            );
            let relation = StableId::new("COVERAGE-GENERIC-ATTACHMENT").expect("relation");
            let binding = ImplementationBinding::new_with(
                active,
                relation.clone(),
                NavigationKind::Implements,
                node.reference(),
                requirement,
                AtomRevisionRef::new(
                    StableId::new("COVERAGE-GENERIC-REQUIREMENT").expect("requirement ID"),
                    requirement.digest(),
                ),
            )
            .expect("binding");
            let attachment = ArtifactBindingAttachment::new_with(
                active,
                relation,
                artifact_path,
                node.reference(),
                binding.record(),
            )
            .expect("attachment");
            assert!(validate_artifact_attachment(&attachment, &binding, &node).is_ok());
        }
        #[cfg(feature = "xxh3-128")]
        {
            let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("XXH3 member");
            let artifact_path = RepositoryPath::new("artifact.bin").expect("artifact path");
            let item = ParsedItemIdentity::new(
                SchemaId::new("artifact.opaque-bytes/v1").expect("artifact schema"),
                artifact_path.clone(),
                SourceItemKind::new("artifact/file/v1").expect("artifact kind"),
                b"artifact".to_vec(),
                None,
            )
            .expect("artifact item");
            let node = SourceNode::new(item, Vec::new()).expect("artifact node");
            let requirement = SourceNodeRef::new(SemanticDigest::xxh3_128([7; 16]));
            let relation = StableId::new("COVERAGE-VALID-ATTACHMENT").expect("relation");
            let binding = ImplementationBinding::new_with(
                active,
                relation.clone(),
                NavigationKind::Implements,
                node.reference(),
                requirement,
                AtomRevisionRef::new(
                    StableId::new("COVERAGE-VALID-REQUIREMENT").expect("requirement ID"),
                    requirement.digest(),
                ),
            )
            .expect("binding");
            let attachment = ArtifactBindingAttachment::new_with(
                active,
                relation,
                artifact_path,
                node.reference(),
                binding.record(),
            )
            .expect("attachment");
            assert!(validate_artifact_attachment(&attachment, &binding, &node).is_ok());
            for (subject, record) in [
                (
                    SourceNodeRef::new(SemanticDigest::sha_256([3; 32])),
                    ImplementationBindingRef::new(SemanticDigest::xxh3_128([4; 16])),
                ),
                (
                    SourceNodeRef::new(SemanticDigest::xxh3_128([5; 16])),
                    ImplementationBindingRef::new(SemanticDigest::sha_256([6; 32])),
                ),
            ] {
                assert!(
                    ArtifactBindingAttachment::new_with(
                        active,
                        StableId::new("COVERAGE-MIXED-ATTACHMENT").expect("relation"),
                        RepositoryPath::new("artifact.bin").expect("artifact path"),
                        subject,
                        record,
                    )
                    .is_err()
                );
            }
        }
    }
}

#[test]
fn registry_contract_is_closed_and_reserves_obsolete_tags() {
    assert_eq!(DigestAlgorithm::ALL.len(), 3);
    assert_eq!(DigestAlgorithm::from_tag(0), None);
    assert_eq!(DigestAlgorithm::from_tag(1), None);
    assert_eq!(
        DigestAlgorithm::from_tag(2),
        Some(DigestAlgorithm::Xxh3_128)
    );
    assert_eq!(
        DigestAlgorithm::from_tag(3),
        Some(DigestAlgorithm::Blake3_256)
    );
    assert_eq!(DigestAlgorithm::from_tag(4), Some(DigestAlgorithm::Sha256));
    assert_eq!(DigestAlgorithm::Xxh3_128.output_length(), 16);
    assert_eq!(DigestAlgorithm::Blake3_256.output_length(), 32);
    assert_eq!(DigestAlgorithm::Sha256.output_length(), 32);
    assert_eq!(
        DigestAlgorithm::Xxh3_128.assurance_class(),
        DigestAssuranceClass::ChangeDetection
    );
    assert_eq!(
        DigestAlgorithm::Blake3_256.assurance_class(),
        DigestAssuranceClass::CollisionResistant
    );
    assert_eq!(
        DigestAlgorithm::Sha256.assurance_class(),
        DigestAssuranceClass::CollisionResistant
    );
}

#[test]
fn every_registry_member_round_trips_exact_lengths() {
    for algorithm in DigestAlgorithm::ALL {
        let byte = match algorithm {
            DigestAlgorithm::Xxh3_128 => 2_u8,
            DigestAlgorithm::Blake3_256 => 3_u8,
            DigestAlgorithm::Sha256 => 4_u8,
        };
        let bytes = vec![byte; algorithm.output_length()];
        let digest = SemanticDigest::from_bytes(algorithm, &bytes).expect("registry length");
        assert_eq!(digest.bytes(), bytes);
        assert_eq!(
            SemanticDigest::parse_canonical(&digest.canonical_text()),
            Ok(digest)
        );
        assert_eq!(
            SemanticDigest::parse_canonical_binary(&digest.canonical_binary()),
            Ok(digest)
        );
        let mut short = bytes;
        short.pop();
        assert!(SemanticDigest::from_bytes(algorithm, &short).is_err());
    }
}

#[test]
fn semantic_comparison_rejects_cross_algorithm_values() {
    let xxh3 = SemanticDigest::xxh3_128([0_u8; 16]);
    let blake3 = SemanticDigest::blake3_256([0_u8; 32]);
    let other_xxh3 = SemanticDigest::xxh3_128([1_u8; 16]);

    assert_eq!(xxh3.checked_eq(&xxh3), Ok(true));
    assert_eq!(xxh3.checked_eq(&other_xxh3), Ok(false));
    assert_eq!(xxh3.checked_cmp(&other_xxh3), Ok(core::cmp::Ordering::Less));
    let error = xxh3
        .checked_eq(&blake3)
        .expect_err("cross-algorithm equality must fail");
    assert_eq!(error.code(), "DIG-ALGORITHM-MISMATCH");
    assert!(xxh3.checked_cmp(&blake3).is_err());
}

#[test]
fn parser_rejects_obsolete_unknown_and_noncanonical_values() {
    let payload = "00".repeat(32);
    for value in [
        format!("SHA-256:{payload}"),
        format!("sha-512:{payload}"),
        format!("sha-256:{}", "GG".repeat(32)),
        format!("sha-256:{}", "00".repeat(31)),
    ] {
        assert!(SemanticDigest::parse_canonical(&value).is_err());
    }
    for tag in [0_u16, 1, 9] {
        let mut binary = Vec::from([12]);
        binary.extend_from_slice(&36_u64.to_be_bytes());
        binary.extend_from_slice(&tag.to_be_bytes());
        binary.extend_from_slice(&32_u16.to_be_bytes());
        binary.extend_from_slice(&[0_u8; 32]);
        assert!(SemanticDigest::parse_canonical_binary(&binary).is_err());
    }
}

#[test]
#[cfg(all(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256"))]
fn official_known_answer_vectors_match_each_adapter() {
    let vectors = [
        (
            DigestAlgorithm::Xxh3_128,
            b"".as_slice(),
            "99aa06d3014798d86001c324468d497f",
        ),
        (
            DigestAlgorithm::Blake3_256,
            b"".as_slice(),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        ),
        (
            DigestAlgorithm::Sha256,
            b"".as_slice(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            DigestAlgorithm::Sha256,
            b"abc".as_slice(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
    ];
    for (algorithm, input, expected) in vectors {
        let active = ActiveAlgorithm::new(algorithm).expect("all adapters are available");
        let digest = active
            .compute(algorithm, input)
            .expect("active adapter computes");
        assert_eq!(lower_hex(digest.bytes()), expected);
    }
}

#[test]
#[cfg(all(feature = "xxh3-128", feature = "sha-256"))]
fn active_member_rejects_inactive_computation() {
    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("xxh3 feature");
    let error = active
        .compute(DigestAlgorithm::Sha256, b"preimage")
        .expect_err("inactive member must fail");
    assert_eq!(error.code(), "DIG-INACTIVE-ALGORITHM");
}
