#![doc = "Buoy package, dependency, compatibility-window, and CLI identity tests."]
#![allow(clippy::expect_used)]

#[path = "../tools/digest_binding.rs"]
mod content_digest;
#[path = "../tools/execution_evidence.rs"]
mod execution_evidence;

use buoy::core::{CanonicalValue, SemanticDigest};
use buoy_reference::Value;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const ROLLBACK_FIXTURES: &str = "fixtures/product-identity/v1/rollback.tsv";
const ROLLBACK_EXECUTIONS: &str = "assurance/v1/product-identity-rollback-executions.tsv";
const ROLLBACK_TOOLS: [&str; 3] = [
    "tests/product_identity.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
const ROLLBACK_CONFIGURATION: [&str; 1] = [ROLLBACK_FIXTURES];

fn compatibility_cargo() -> Command {
    let mut command = Command::new(env!("CARGO"));
    for variable in [
        "CARGO_LLVM_COV",
        "CARGO_LLVM_COV_SHOW_ENV",
        "LLVM_PROFILE_FILE",
        "RUSTC_WRAPPER",
        "RUSTFLAGS",
        "RUSTDOCFLAGS",
        "__CARGO_LLVM_COV_RUSTC_WRAPPER",
        "__CARGO_LLVM_COV_RUSTC_WRAPPER_CRATE_NAMES",
        "__CARGO_LLVM_COV_RUSTC_WRAPPER_PRE_EXISTING",
        "__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS",
    ] {
        command.env_remove(variable);
    }
    command
}

fn metadata() -> Option<JsonValue> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn package_dependencies<'a>(metadata: &'a JsonValue, name: &str) -> Vec<&'a str> {
    metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|package| package["name"].as_str() == Some(name))
        .and_then(|package| package["dependencies"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|dependency| dependency["name"].as_str())
        .collect()
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn decode_snapshot_hex(source: &[u8]) -> Option<Vec<u8>> {
    let digits = source
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if digits.is_empty() || digits.len() % 2 != 0 {
        return None;
    }
    digits
        .chunks_exact(2)
        .map(|pair| Some((hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?))
        .collect()
}

fn frozen_artifact(path: &str) -> Option<Vec<u8>> {
    let manifest =
        fs::read_to_string("assurance/product-identity/v1/pre-rename-snapshots.tsv").ok()?;
    for line in manifest.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() == 4 && fields[1] == path {
            return fs::read(fields[2])
                .ok()
                .and_then(|bytes| decode_snapshot_hex(&bytes));
        }
    }
    fs::read(path).ok()
}

#[test]
fn primary_package_crate_and_reference_identities_compile() {
    assert_eq!(env!("CARGO_PKG_NAME"), "buoy");
    assert_eq!(
        CanonicalValue::Digest(SemanticDigest::xxh3_128([1; 16]))
            .encode()
            .ok()
            .and_then(|bytes| Value::decode(&bytes, 128, 10).ok())
            .and_then(|value| value.encode().ok()),
        CanonicalValue::Digest(SemanticDigest::xxh3_128([1; 16]))
            .encode()
            .ok()
    );
}

#[test]
fn workspace_dependency_graph_points_one_way_to_buoy_core() {
    let metadata = metadata();
    assert!(metadata.is_some());
    let Some(metadata) = metadata else { return };
    let packages = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|package| package["name"].as_str())
        .collect::<Vec<_>>();
    for expected in [
        "buoy",
        "buoy-core",
        "buoy-reference",
        "spec-provenance",
        "spec-provenance-core",
    ] {
        assert!(packages.contains(&expected), "missing package {expected}");
    }
    assert!(package_dependencies(&metadata, "buoy").contains(&"buoy-core"));
    assert!(package_dependencies(&metadata, "spec-provenance").contains(&"buoy"));
    assert!(package_dependencies(&metadata, "spec-provenance-core").contains(&"buoy-core"));
    assert!(!package_dependencies(&metadata, "buoy-core").contains(&"buoy"));
    assert!(!package_dependencies(&metadata, "buoy-core").contains(&"spec-provenance"));
}

#[test]
fn compatibility_window_is_machine_readable_and_expires_at_point_three() {
    let text = fs::read_to_string("assurance/product-identity/v1/compatibility.toml").ok();
    assert!(text.is_some());
    let parsed = text
        .as_deref()
        .and_then(|source| toml::from_str::<toml::Value>(source).ok());
    assert!(parsed.is_some());
    let Some(parsed) = parsed else { return };
    assert_eq!(
        parsed.get("alias_window").and_then(toml::Value::as_str),
        Some(">=0.2.0,<0.3.0")
    );
    assert_eq!(
        parsed
            .get("alias_removal_release")
            .and_then(toml::Value::as_str),
        Some("0.3.0")
    );
    assert_eq!(
        parsed
            .get("aliases_are_authoritative")
            .and_then(toml::Value::as_bool),
        Some(false)
    );
}

#[test]
fn clean_install_and_package_upgrade_fixtures_compile() {
    for manifest in [
        "fixtures/product-identity/v1/positive/clean-buoy-shell/Cargo.toml",
        "fixtures/product-identity/v1/positive/clean-buoy-core/Cargo.toml",
        "fixtures/product-identity/v1/positive/upgrade-spec-provenance/Cargo.toml",
        "fixtures/product-identity/v1/positive/upgrade-spec-provenance-core/Cargo.toml",
    ] {
        let output = Command::new(env!("CARGO"))
            .args([
                "check",
                "--quiet",
                "--offline",
                "--manifest-path",
                manifest,
                "--target-dir",
                "target/product-identity-install-fixtures",
            ])
            .output()
            .ok();
        assert!(
            output.as_ref().is_some_and(|value| value.status.success()),
            "fixture failed: {manifest}: {}",
            output
                .as_ref()
                .map_or_else(String::new, |value| String::from_utf8_lossy(&value.stderr)
                    .into_owned())
        );
        let lock = PathBuf::from(manifest).with_file_name("Cargo.lock");
        let _ = fs::remove_file(lock);
    }
}

fn validate_rollback_execution_evidence(write: bool) -> Result<(), String> {
    let source = fs::read_to_string(ROLLBACK_FIXTURES).map_err(|error| error.to_string())?;
    let mut records = Vec::new();
    let mut digest_mismatches = Vec::new();
    for line in source.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err(String::from("product-identity rollback fixture shape"));
        }
        let bytes = frozen_artifact(fields[3])
            .ok_or_else(|| format!("unknown frozen artifact {}", fields[3]))?;
        let actual = content_digest::bytes_binding(&bytes);
        if actual != fields[4] {
            digest_mismatches.push(format!(
                "{} expected={} actual={actual}",
                fields[1], fields[4]
            ));
        }
        records.push(execution_evidence::passing(
            "fixture-operation",
            format!("{ROLLBACK_FIXTURES}#{}#{}", fields[1], fields[2]),
            ROLLBACK_FIXTURES,
            line.as_bytes(),
            &ROLLBACK_TOOLS,
            &ROLLBACK_CONFIGURATION,
            format!("operation={};observed={actual}", fields[2]),
        )?);
    }
    if !digest_mismatches.is_empty() {
        return Err(format!(
            "rollback digest differs: {}",
            digest_mismatches.join(", ")
        ));
    }
    let expected = execution_evidence::render(&records)?;
    execution_evidence::validate_materialized(ROLLBACK_EXECUTIONS, &expected, write)
}

#[test]
fn rollback_fixture_preserves_frozen_artifact_bytes() {
    validate_rollback_execution_evidence(false)
        .expect("rollback execution evidence must be current");
}

#[test]
#[ignore = "explicit reviewed per-case execution-evidence regeneration only"]
fn write_rollback_fixture_execution_evidence() {
    validate_rollback_execution_evidence(true)
        .expect("rollback execution evidence must be written from passing executions");
}

#[test]
fn product_identity_execution_evidence_cli_is_current_and_fails_closed_when_stale() {
    use buoy::product_identity;
    use std::path::Path;

    // `fixture_execution_evidence` only reads the fixture manifest and
    // writes the committed execution-evidence artifact underneath `root`;
    // its tool/configuration authorities are real repository paths resolved
    // relative to the process's current working directory, not `root`. An
    // isolated scratch root therefore only needs to carry those two
    // root-relative files to safely exercise the write/check round trip
    // without mutating the real committed artifact under `cargo test`'s
    // default parallel execution.
    const MANIFEST: &str = "fixtures/product-identity/v1/manifest.tsv";
    const EXECUTIONS: &str = "assurance/v1/product-identity-fixture-executions.tsv";

    let scratch = tempfile::tempdir().expect("execution-evidence CLI scratch root");
    let root = scratch.path();
    for relative in [MANIFEST, EXECUTIONS] {
        let destination = root.join(relative);
        fs::create_dir_all(
            destination
                .parent()
                .expect("execution-evidence CLI scratch path has a parent"),
        )
        .expect("execution-evidence CLI scratch directory must be writable");
        fs::copy(relative, &destination).expect("copy repository file into scratch checkout");
    }

    // Happy path: the committed per-case execution-evidence artifact must be
    // byte-current with a deterministic regeneration from the live fixture
    // manifest, through both the direct function and the CLI dispatch.
    product_identity::fixture_execution_evidence(root, false)
        .expect("committed product-identity execution evidence must be current");
    product_identity::run_cli(&[String::from("--check-execution-evidence")], root)
        .expect("check-execution-evidence through CLI parser");

    let executions_path = root.join(EXECUTIONS);
    let original = fs::read_to_string(&executions_path)
        .expect("read committed execution evidence in scratch checkout");

    // Fail-closed: a corrupted/stale artifact must be rejected, not
    // silently repaired, when only checking.
    fs::write(&executions_path, "stale\n")
        .expect("corrupt execution evidence for control in scratch checkout");
    let error = product_identity::fixture_execution_evidence(root, false)
        .expect_err("stale execution evidence must fail closed");
    assert!(error.to_string().starts_with("PID-EXECUTION-EVIDENCE"));
    let cli_error = product_identity::run_cli(&[String::from("--check-execution-evidence")], root)
        .expect_err("stale execution evidence must fail closed through the CLI");
    assert!(cli_error.to_string().starts_with("PID-EXECUTION-EVIDENCE"));

    // Round trip: the reviewed write flag regenerates the exact committed
    // content from the same deterministic render.
    product_identity::run_cli(&[String::from("--write-execution-evidence")], root)
        .expect("write-execution-evidence through CLI parser");
    let rewritten = fs::read_to_string(&executions_path)
        .expect("read rewritten execution evidence in scratch checkout");
    assert_eq!(rewritten, original);
    product_identity::fixture_execution_evidence(root, false)
        .expect("execution evidence must be current after rewrite");

    // Usage: an unrecognized argument still falls through to the stable
    // usage diagnostic, independent of any execution-evidence handling.
    assert!(product_identity::run_cli(&[String::from("--invalid")], Path::new(".")).is_err());
}

#[test]
fn point_three_removal_fixture_removes_aliases_but_retains_readers() {
    let source = fs::read_to_string("fixtures/product-identity/v1/removal-0.3.0.toml").ok();
    let parsed = source
        .as_deref()
        .and_then(|value| toml::from_str::<toml::Value>(value).ok());
    assert!(parsed.is_some());
    let Some(parsed) = parsed else { return };
    assert_eq!(
        parsed.get("release").and_then(toml::Value::as_str),
        Some("0.3.0")
    );
    for key in [
        "remove_packages",
        "remove_crates",
        "remove_binaries",
        "remove_task_prefixes",
    ] {
        assert!(
            parsed
                .get(key)
                .and_then(toml::Value::as_array)
                .is_some_and(|rows| !rows.is_empty())
        );
    }
    let readers = parsed
        .get("retain_readers")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(readers, ["spec-provenance/v1", "spec-provenance/v2"]);
}

#[test]
fn primary_cli_uses_buoy_help_and_stable_usage_exit() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_buoy"));
    let output = Command::new(binary).output().ok();
    assert!(output.is_some());
    let Some(output) = output else { return };
    assert_eq!(output.status.code(), Some(64));
    let stderr = String::from_utf8(output.stderr).ok();
    assert!(stderr.is_some());
    assert!(stderr.is_some_and(|text| {
        text.starts_with("usage: buoy static") && !text.contains("usage: spec-provenance")
    }));

    let help = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .arg("--help")
        .output()
        .ok();
    assert!(help.is_some());
    let Some(help) = help else { return };
    assert_eq!(help.status.code(), Some(0));
    assert!(help.stderr.is_empty());
    assert!(
        String::from_utf8(help.stdout.clone())
            .is_ok_and(|text| text.starts_with("usage: buoy static"))
    );

    let alias_target = PathBuf::from("target/product-identity-compat-cli");
    let alias_build = compatibility_cargo()
        .args([
            "build",
            "--quiet",
            "--offline",
            "-p",
            "spec-provenance",
            "--target-dir",
        ])
        .arg(&alias_target)
        .output()
        .ok();
    assert!(alias_build.is_some_and(|build| build.status.success()));
    let alias_binary = alias_target
        .join("debug")
        .join(format!("spec-provenance{}", std::env::consts::EXE_SUFFIX));
    let alias_help = Command::new(&alias_binary).arg("--help").output().ok();
    assert!(alias_help.is_some());
    let Some(alias_help) = alias_help else { return };
    assert_eq!(alias_help.status.code(), help.status.code());
    assert_eq!(alias_help.stdout, help.stdout);
    assert_eq!(
        String::from_utf8(alias_help.stderr).ok().as_deref(),
        Some(
            "warning: `spec-provenance` is deprecated; use `buoy`; compatibility expires at 0.3.0\n"
        )
    );

    let alias_usage = Command::new(alias_binary).output().ok();
    assert!(alias_usage.is_some());
    let Some(alias_usage) = alias_usage else {
        return;
    };
    assert_eq!(alias_usage.status.code(), Some(64));
    assert_eq!(alias_usage.stdout, output.stdout);
    let alias_stderr = String::from_utf8(alias_usage.stderr).ok();
    assert!(alias_stderr.is_some_and(|text| {
        text.starts_with(
            "warning: `spec-provenance` is deprecated; use `buoy`; compatibility expires at 0.3.0\n",
        ) && text.contains(
            "usage: buoy static (--root ROOT | --mount ID=PATH...) --config FILE [--format machine|human]\n",
        )
    }));
}
