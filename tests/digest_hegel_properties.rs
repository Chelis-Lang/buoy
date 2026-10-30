//! Fixed-seed properties for every registered digest member.

#![cfg(all(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256"))]
#![allow(clippy::expect_used)]

use buoy::core::{
    ActiveAlgorithm, CanonicalValue, DigestAlgorithm, SemanticDigest, semantic_digest_with,
};
use hegel::generators as gs;
use hegel::{Hegel, TestCase};

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

fn active(algorithm: DigestAlgorithm) -> ActiveAlgorithm {
    ActiveAlgorithm::new(algorithm).expect("all-member property build")
}

fn all_members() -> [DigestAlgorithm; 3] {
    DigestAlgorithm::ALL
}

#[test]
fn hegel_each_member_round_trips_in_text_and_binary() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(4096));
        for algorithm in all_members() {
            let digest = active(algorithm)
                .compute(algorithm, &bytes)
                .expect("active member computes");
            assert_eq!(
                SemanticDigest::parse_canonical(&digest.canonical_text()),
                Ok(digest)
            );
            assert_eq!(
                SemanticDigest::parse_canonical_binary(&digest.canonical_binary()),
                Ok(digest)
            );
        }
    })
    .settings(settings(301))
    .run();
}

#[test]
fn hegel_digest_output_is_root_and_set_permutation_independent() {
    Hegel::new(|test_case: TestCase| {
        let first = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(256));
        let second = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(256));
        for algorithm in all_members() {
            let active = active(algorithm);
            let left = semantic_digest_with(
                active,
                "root-independent",
                vec![(
                    1,
                    CanonicalValue::Set(vec![
                        CanonicalValue::Bytes(first.clone()),
                        CanonicalValue::Bytes(second.clone()),
                    ]),
                )],
            );
            let right = semantic_digest_with(
                active,
                "root-independent",
                vec![(
                    1,
                    CanonicalValue::Set(vec![
                        CanonicalValue::Bytes(second.clone()),
                        CanonicalValue::Bytes(first.clone()),
                    ]),
                )],
            );
            assert_eq!(left, right);
        }
    })
    .settings(settings(302))
    .run();
}

#[test]
fn hegel_preimage_changes_bind_domain_field_and_sequence_order() {
    Hegel::new(|test_case: TestCase| {
        let byte = test_case.draw(gs::integers::<u8>());
        let first = CanonicalValue::Bytes(vec![byte, 0]);
        let changed = CanonicalValue::Bytes(vec![byte ^ 1, 0]);
        let second = CanonicalValue::Bytes(vec![byte, 1]);
        for algorithm in all_members() {
            let active = active(algorithm);
            let baseline = semantic_digest_with(
                active,
                "domain-a",
                vec![(
                    1,
                    CanonicalValue::Sequence(vec![first.clone(), second.clone()]),
                )],
            )
            .expect("baseline encodes");
            let domain_change = semantic_digest_with(
                active,
                "domain-b",
                vec![(
                    1,
                    CanonicalValue::Sequence(vec![first.clone(), second.clone()]),
                )],
            )
            .expect("domain change encodes");
            let field_change = semantic_digest_with(
                active,
                "domain-a",
                vec![(
                    1,
                    CanonicalValue::Sequence(vec![changed.clone(), second.clone()]),
                )],
            )
            .expect("field change encodes");
            let order_change = semantic_digest_with(
                active,
                "domain-a",
                vec![(
                    1,
                    CanonicalValue::Sequence(vec![second.clone(), first.clone()]),
                )],
            )
            .expect("order change encodes");
            assert_ne!(baseline, domain_change);
            assert_ne!(baseline, field_change);
            assert_ne!(baseline, order_change);
        }
    })
    .settings(settings(303))
    .run();
}

#[test]
fn hegel_parser_rejects_noncanonical_member_forms() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(1024));
        let selector = test_case.draw(gs::integers::<u8>());
        for algorithm in all_members() {
            let digest = active(algorithm)
                .compute(algorithm, &bytes)
                .expect("active member computes");
            let canonical = digest.canonical_text();
            let malformed = match selector % 5 {
                0 => canonical.to_ascii_uppercase(),
                1 => canonical.replacen(algorithm.canonical_name(), "sha-512", 1),
                2 => canonical[..canonical.len() - 1].to_owned(),
                3 => format!("{canonical}0"),
                _ => format!(
                    "{}:g{}",
                    algorithm.canonical_name(),
                    &canonical[algorithm.canonical_name().len() + 2..]
                ),
            };
            assert!(SemanticDigest::parse_canonical(&malformed).is_err());
        }
    })
    .settings(settings(304))
    .run();
}

#[test]
fn hegel_regeneration_is_idempotent_for_each_member() {
    Hegel::new(|test_case: TestCase| {
        let source = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(2048));
        for algorithm in all_members() {
            let active = active(algorithm);
            let first = active
                .compute(algorithm, &source)
                .expect("first regeneration");
            let second = active
                .compute(algorithm, &source)
                .expect("second regeneration");
            assert_eq!(first, second);
        }
    })
    .settings(settings(305))
    .run();
}
