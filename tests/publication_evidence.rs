#![doc = "Contract tests for deterministic publication reports and evidence."]
#![allow(missing_docs, clippy::expect_used)]

use buoy::external_link_generation::render_slot;
use buoy::publication::{
    PackageIdentity, PublicationEvidence, PublicationReport, render_evidence, render_report,
};

#[test]
fn report_is_canonical() {
    let report = PublicationReport::new("check", "trace-page")
        .with_diagnostic("TPT-Z", "z")
        .with_diagnostic("TPT-A", "a");
    let value: serde_json::Value =
        serde_json::from_slice(&render_report(&report).expect("report")).expect("JSON");
    assert_eq!(value["schema"], "buoy/trace-publication-report/v1");
    assert_eq!(value["diagnostics"][0]["code"], "TPT-A");
}

#[test]
fn evidence_is_historical_and_machine_readable() {
    let evidence = PublicationEvidence::new("pub-1");
    let value: serde_json::Value =
        serde_json::from_slice(&render_evidence(&evidence)).expect("JSON");
    assert_eq!(value["schema"], "buoy/trace-publication-evidence/v1");
    assert_eq!(value["status"], "historical");
    assert!(
        evidence
            .render_markdown()
            .contains("current-state authority")
    );
}

#[test]
fn evidence_carries_the_retained_source_digest_when_present() {
    let evidence =
        PublicationEvidence::new("pub-1").with_source_digest("xxh3-128:aaaaaaaaaaaaaaaa");
    let value: serde_json::Value =
        serde_json::from_slice(&render_evidence(&evidence)).expect("JSON");
    assert_eq!(value["source_digest"], "xxh3-128:aaaaaaaaaaaaaaaa");
    assert!(
        evidence
            .render_markdown()
            .contains("xxh3-128:aaaaaaaaaaaaaaaa")
    );

    let without = PublicationEvidence::new("pub-1");
    let without_value: serde_json::Value =
        serde_json::from_slice(&render_evidence(&without)).expect("JSON");
    assert!(without_value.get("source_digest").is_none());
}

#[test]
fn report_rejects_scope_claims() {
    let report = PublicationReport::new("generate", "page").with_scope_claim("remote repository");
    assert!(render_report(&report).is_err());
}

#[test]
fn committed_publication_goldens_match_their_deterministic_regeneration() {
    let report = PublicationReport::new("check", "trace-page");
    let expected = render_report(&report).expect("report");
    let committed = std::fs::read("examples/expected/publication-operation-report.json")
        .expect("committed report golden");
    assert_eq!(
        committed, expected,
        "regenerate with `cargo run --locked --bin generate-publication-goldens`"
    );

    let evidence = PublicationEvidence::new("consumer-example");
    let expected = render_evidence(&evidence);
    let committed = std::fs::read("examples/expected/publication-evidence.json")
        .expect("committed evidence golden");
    assert_eq!(
        committed, expected,
        "regenerate with `cargo run --locked --bin generate-publication-goldens`"
    );

    let identity = PackageIdentity::new(
        "0.2.0",
        "0000000000000000000000000000000000000000",
        "nightly-2026-04-16",
        vec!["xxh3-128"],
    );
    let expected = identity.render().expect("identity");
    let committed = std::fs::read("examples/expected/package-identity.json")
        .expect("committed package identity golden");
    assert_eq!(
        committed, expected,
        "regenerate with `cargo run --locked --bin generate-publication-goldens`"
    );

    let expected = render_slot(
        "consumer-example-link",
        "EXAMPLE-001",
        "blake3-256:a352b07c143fff7a0aa8e2a57d65fba2ee2d59635fdb20191f68fe93ef21c87f",
        "documents",
        "https://example.invalid/trace/blake3-256/29fce2a3aa4972e09809482995a4f33636a11c2cac45ea87192bf9d49d7e01a5.md",
    );
    let committed = std::fs::read_to_string("examples/expected/publication-external-link-slot.md")
        .expect("committed external-link slot golden");
    assert_eq!(
        committed, expected,
        "regenerate with `cargo run --locked --bin generate-publication-goldens`"
    );
}

#[test]
fn package_identity_has_explicit_features() {
    let identity =
        PackageIdentity::new("0.2.0", "revision", "rustc", vec!["sha-256", "blake3-256"]);
    let value: serde_json::Value =
        serde_json::from_slice(&identity.render().expect("identity")).expect("JSON");
    assert_eq!(value["schema"], "buoy/package-identity/v1");
    assert_eq!(
        value["features"],
        serde_json::json!(["blake3-256", "sha-256"])
    );
}
