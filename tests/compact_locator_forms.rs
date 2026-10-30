#![doc = "Production-reader contract for `fixtures/trace-publication/v1/compact-locator/forms.json`."]
#![allow(missing_docs, clippy::expect_used, clippy::panic)]
#![cfg(all(feature = "blake3-256", feature = "xxh3-128"))]

use buoy::adapters::extract_rust_with;
use buoy::core::{ActiveAlgorithm, DigestAlgorithm, RepositoryPath};
use serde_json::Value;

/// Every declared compact-locator form fixture is parsed through the real
/// production Rust adapter, and its accept/reject outcome (and, for a
/// rejection, its exact diagnostic code) is asserted against the fixture's
/// declared expectation. This fixture previously had no production reader.
#[test]
fn every_declared_form_matches_its_production_outcome() {
    let bytes = include_bytes!("../fixtures/trace-publication/v1/compact-locator/forms.json");
    let value: Value = serde_json::from_slice(bytes).expect("fixture JSON");
    assert_eq!(
        value["schema"],
        "buoy-trace-publication-compact-locator-fixtures/v1"
    );
    let cases = value["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty());

    for case in cases {
        let name = case["name"].as_str().expect("case name");
        let attribute = case["attribute"].as_str().expect("attribute source");
        let active_member = case["active_member"].as_str().expect("active member");
        let expected = case["expected"].as_str().expect("expected outcome");

        let algorithm = match active_member {
            "blake3-256" => DigestAlgorithm::Blake3_256,
            "xxh3-128" => DigestAlgorithm::Xxh3_128,
            "sha-256" => DigestAlgorithm::Sha256,
            other => panic!("case {name}: unknown active member {other}"),
        };
        let active = ActiveAlgorithm::new(algorithm).unwrap_or_else(|_| {
            panic!("case {name}: {active_member} must be compiled into this test build")
        });

        let source = format!("{attribute}\npub fn subject() {{}}\n");
        let path = RepositoryPath::new("src/lib.rs").expect("fixture path");
        let output = extract_rust_with(active, &path, source.as_bytes());

        match expected {
            "accept" => {
                assert!(
                    output.diagnostics.is_empty(),
                    "case {name}: expected acceptance, got diagnostics {:?}",
                    output.diagnostics
                );
                assert_eq!(
                    output.bindings.len(),
                    1,
                    "case {name}: an accepted form must resolve exactly one binding"
                );
            }
            "reject" => {
                let expected_diagnostic = case["diagnostic"].as_str().expect("expected diagnostic");
                assert!(
                    output
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == expected_diagnostic),
                    "case {name}: expected diagnostic {expected_diagnostic}, got {:?}",
                    output.diagnostics
                );
                assert!(
                    output.bindings.is_empty(),
                    "case {name}: a rejected form must resolve no binding"
                );
            }
            other => panic!("case {name}: unknown expectation {other}"),
        }
    }
}

/// Task 11.4: `docs/schema/v3/content-addressed-bindings.md`'s documented
/// compact-locator example must itself be algorithm-qualified syntax, not
/// the removed hash-only form CAB-002/CAB-007 now reject. This extracts the
/// exact `#[buoy::implements(...)]` line from the committed doc file and
/// runs it through the real production Rust adapter (not a hand-rolled
/// regex check), so a future doc edit that regresses to hash-only syntax
/// fails this test with the real `CAB-COMPACT-ATTRIBUTE-DIGEST` diagnostic.
#[test]
fn documented_compact_locator_example_is_algorithm_qualified() {
    let doc = include_str!("../docs/schema/v3/content-addressed-bindings.md");
    let start = doc
        .find("#[buoy::implements(\"")
        .expect("doc declares one compact-locator example");
    let line_end = doc[start..].find('\n').expect("attribute line ends");
    let attribute_line = &doc[start..start + line_end];

    let algorithm = DigestAlgorithm::Blake3_256;
    let active =
        ActiveAlgorithm::new(algorithm).expect("blake3-256 must be compiled into this test build");
    let source = format!("{attribute_line}\npub fn subject() {{}}\n");
    let path = RepositoryPath::new("src/lib.rs").expect("fixture path");
    let output = extract_rust_with(active, &path, source.as_bytes());

    assert!(
        output.diagnostics.is_empty(),
        "the documented compact-locator example must parse as algorithm-qualified, got diagnostics {:?}",
        output.diagnostics
    );
    assert_eq!(
        output.bindings.len(),
        1,
        "the documented example must resolve exactly one binding"
    );
}
