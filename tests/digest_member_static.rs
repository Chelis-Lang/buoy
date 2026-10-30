//! End-to-end static-check parity across all active digest members.

#![cfg(all(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256"))]
#![allow(clippy::expect_used)]

use buoy::check::check_snapshot;
use buoy::repository::{RepositoryConfig, RepositorySnapshot};
use std::collections::BTreeSet;

fn configuration(member: &str) -> RepositoryConfig {
    let source = format!(
        "schema = \"spec-provenance-config/v2\"\n\
         digest_algorithms = [\"{member}\"]\n\
         [[selectors]]\n\
         root = \"spec\"\n\
         adapter = \"markdown/v1\"\n\
         include = [\"**/*.md\"]\n\
         exclude = []\n\
         authority_expectation = \"optional\"\n"
    );
    RepositoryConfig::parse(source.as_bytes()).expect("all-member test configuration")
}

#[test]
fn active_member_changes_only_digest_identity_for_the_same_static_snapshot() {
    let snapshot = RepositorySnapshot { files: Vec::new() };
    let results = ["xxh3-128", "blake3-256", "sha-256"]
        .map(|member| check_snapshot(&snapshot, &configuration(member)).expect("static check"));

    for result in &results[1..] {
        assert_eq!(result.graph, results[0].graph);
        assert_eq!(result.diagnostics, results[0].diagnostics);
        assert_eq!(result.migration, results[0].migration);
    }
    assert_eq!(
        results[0].graph_digest.algorithm().canonical_name(),
        "xxh3-128"
    );
    assert_eq!(
        results[1].graph_digest.algorithm().canonical_name(),
        "blake3-256"
    );
    assert_eq!(
        results[2].graph_digest.algorithm().canonical_name(),
        "sha-256"
    );
    assert_eq!(
        results[0]
            .graph_digest
            .algorithm()
            .assurance_class()
            .canonical_name(),
        "change-detection"
    );
    assert_eq!(
        results[1]
            .graph_digest
            .algorithm()
            .assurance_class()
            .canonical_name(),
        "collision-resistant"
    );
    assert_eq!(
        results[2]
            .graph_digest
            .algorithm()
            .assurance_class()
            .canonical_name(),
        "collision-resistant"
    );
    assert_eq!(
        results
            .iter()
            .map(|result| result.graph_digest.canonical_text())
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
}
