#![doc = "Focused contract for `buoy/package-identity/v1` (PBD-003)."]
#![allow(missing_docs, clippy::expect_used)]

use buoy::publication::PackageIdentity;

/// The real `buoy package-identity` CLI command (not just the standalone
/// renderer) exits `0` and emits the frozen machine schema.
#[test]
fn package_identity_command_runs_and_exits_ok() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_buoy"))
        .arg("package-identity")
        .output()
        .expect("package-identity command");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("package-identity JSON");
    assert_eq!(value["schema"], "buoy/package-identity/v1");
    assert!(
        value["source_revision"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
}

#[test]
fn package_identity_renders_the_frozen_machine_schema() {
    let identity = PackageIdentity::new(
        "0.2.0",
        "0000000000000000000000000000000000000000",
        "nightly-2026-04-16",
        vec!["xxh3-128", "blake3-256", "sha-256"],
    );
    let value: serde_json::Value =
        serde_json::from_slice(&identity.render().expect("identity render")).expect("JSON");
    assert_eq!(value["schema"], "buoy/package-identity/v1");
    assert_eq!(value["version"], "0.2.0");
    assert_eq!(
        value["features"],
        serde_json::json!(["blake3-256", "sha-256", "xxh3-128"])
    );
    assert_eq!(
        value["supported_members"],
        serde_json::json!(["blake3-256", "sha-256", "xxh3-128"])
    );
}

#[test]
fn package_identity_rejects_incomplete_declared_fields() {
    let missing_version = PackageIdentity::new("", "revision", "toolchain", vec!["xxh3-128"]);
    assert!(missing_version.render().is_err());

    let missing_source_revision = PackageIdentity::new("0.2.0", "", "toolchain", vec!["xxh3-128"]);
    assert!(missing_source_revision.render().is_err());

    let missing_features = PackageIdentity::new("0.2.0", "revision", "toolchain", vec![]);
    assert!(missing_features.render().is_err());
}

#[test]
fn committed_package_identity_golden_matches_regeneration() {
    let identity = PackageIdentity::new(
        "0.2.0",
        "0000000000000000000000000000000000000000",
        "nightly-2026-04-16",
        vec!["xxh3-128"],
    );
    let expected = identity.render().expect("identity render");
    let committed =
        std::fs::read("examples/expected/package-identity.json").expect("committed golden");
    assert_eq!(
        committed, expected,
        "regenerate with `cargo run --locked --bin generate-publication-goldens`"
    );
}
