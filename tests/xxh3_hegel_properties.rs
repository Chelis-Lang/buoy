//! Fixed-seed Hegel properties for the registered XXH3-128 member.

#![allow(clippy::expect_used)]

use buoy::core::{ActiveAlgorithm, DigestAlgorithm, SemanticDigest};
use hegel::generators as gs;
use hegel::{Hegel, TestCase};

#[path = "support/hegel_settings.rs"]
mod hegel_settings;
use hegel_settings::settings;

fn active() -> ActiveAlgorithm {
    ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("the default feature provides XXH3-128")
}

fn digest(bytes: &[u8]) -> SemanticDigest {
    active()
        .compute(DigestAlgorithm::Xxh3_128, bytes)
        .expect("the active member computes its own digest")
}

fn canonical_preimage(domain: &str, fields: &[(u16, &[u8])]) -> Vec<u8> {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(b"SPEC-PROVENANCE\0");
    preimage.extend_from_slice(&3_u16.to_be_bytes());
    preimage.extend_from_slice(&(domain.len() as u64).to_be_bytes());
    preimage.extend_from_slice(domain.as_bytes());
    preimage.extend_from_slice(&(fields.len() as u64).to_be_bytes());
    for (tag, value) in fields {
        preimage.extend_from_slice(&tag.to_be_bytes());
        preimage.extend_from_slice(&(value.len() as u64).to_be_bytes());
        preimage.extend_from_slice(value);
    }
    preimage
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MigrationRecord {
    canonical_source: Vec<u8>,
    digest: Option<SemanticDigest>,
}

fn migrate(records: &[MigrationRecord]) -> Vec<MigrationRecord> {
    records
        .iter()
        .map(|record| MigrationRecord {
            canonical_source: record.canonical_source.clone(),
            digest: Some(digest(&record.canonical_source)),
        })
        .collect()
}

#[test]
fn hegel_xxh3_one_shot_is_deterministic() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(4096));
        assert_eq!(digest(&bytes), digest(&bytes));
    })
    .settings(settings(101))
    .run();
}

#[test]
fn hegel_xxh3_repeated_dispatch_matches() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(4096));
        let first = active()
            .compute(DigestAlgorithm::Xxh3_128, &bytes)
            .expect("active dispatch");
        let second = active()
            .compute(DigestAlgorithm::Xxh3_128, &bytes)
            .expect("active dispatch");
        assert_eq!(first, second);
    })
    .settings(settings(102))
    .run();
}

#[test]
fn hegel_xxh3_text_and_binary_forms_round_trip() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(1024));
        let value = digest(&bytes);
        assert_eq!(
            SemanticDigest::parse_canonical(&value.canonical_text()),
            Ok(value)
        );
        assert_eq!(
            SemanticDigest::parse_canonical_binary(&value.canonical_binary()),
            Ok(value)
        );
    })
    .settings(settings(103))
    .run();
}

#[test]
fn hegel_xxh3_is_sensitive_to_domain_field_and_order_changes() {
    Hegel::new(|test_case: TestCase| {
        let sample = test_case.draw(gs::integers::<u8>());
        let first = [sample, 0];
        let changed = [sample ^ 1, 0];
        let second = [sample, 1];
        let baseline = canonical_preimage("domain-a", &[(1, &first), (2, &second)]);
        let domain_change = canonical_preimage("domain-b", &[(1, &first), (2, &second)]);
        let field_change = canonical_preimage("domain-a", &[(1, &changed), (2, &second)]);
        let order_change = canonical_preimage("domain-a", &[(2, &second), (1, &first)]);
        let baseline_digest = digest(&baseline);
        assert_ne!(baseline_digest, digest(&domain_change));
        assert_ne!(baseline_digest, digest(&field_change));
        assert_ne!(baseline_digest, digest(&order_change));
    })
    .settings(settings(104))
    .run();
}

#[test]
fn hegel_xxh3_parser_rejects_noncanonical_forms() {
    Hegel::new(|test_case: TestCase| {
        let bytes = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(1024));
        let selector = test_case.draw(gs::integers::<u8>());
        let canonical = digest(&bytes).canonical_text();
        let malformed = match selector % 5 {
            0 => canonical.to_ascii_uppercase(),
            1 => canonical.replacen("xxh3-128:", "sha256:", 1),
            2 => canonical[..canonical.len() - 1].to_owned(),
            3 => format!("{canonical}0"),
            _ => format!("xxh3-128:g{}", &canonical[10..]),
        };
        assert!(SemanticDigest::parse_canonical(&malformed).is_err());
    })
    .settings(settings(105))
    .run();
}

#[test]
fn hegel_xxh3_complete_migration_is_idempotent() {
    Hegel::new(|test_case: TestCase| {
        let first = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(512));
        let second = test_case.draw(gs::vecs(gs::integers::<u8>()).max_size(512));
        let records = vec![
            MigrationRecord {
                canonical_source: first,
                digest: None,
            },
            MigrationRecord {
                canonical_source: second,
                digest: None,
            },
        ];
        let migrated = migrate(&records);
        assert_eq!(migrate(&migrated), migrated);
    })
    .settings(settings(106))
    .run();
}
