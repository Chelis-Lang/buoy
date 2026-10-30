//! Quality-assurance registry fixture and validator contracts.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

const POSITIVE_FIXTURE: &str = "fixtures/quality/v1/positive-all-states.toml";
const CASE_MANIFEST: &str = "fixtures/quality/v1/cases.tsv";

/// Every real repository path that fixture-execution-evidence rendering can
/// read: the frozen fixture registry and case manifest, everything the
/// registry's controls bind as a source/tool/configuration/result/gate
/// authority (`fixtures/quality/v1/repository/*`, `devenv.nix`, the
/// committed bindings/report/tasks documents named inside the fixture), the
/// execution-evidence tool/configuration authorities
/// (`EXECUTION_TOOL_PATHS`/`EXECUTION_CONFIGURATION_PATHS`), and the
/// committed execution-evidence artifact this test mutates. Kept as an
/// explicit list (rather than a whole-repository walk) so the isolated
/// scratch checkout this test builds touches only the handful of files it
/// actually needs, instead of racing this repository's much heavier,
/// unrelated parallel test I/O.
const SCRATCH_COPY_FILES: &[&str] = &[
    POSITIVE_FIXTURE,
    CASE_MANIFEST,
    "devenv.nix",
    "nix/devenv/workflow-policy.toml",
    "fixtures/devenv/v1/valid-tasks.json",
    "assurance/v1/quality-assurance-bindings.tsv",
    "docs/assurance/quality-assurance-status.md",
    "openspec/changes/archive/2026-08-02-add-buoy-quality-assurance-tracking/tasks.md",
    "openspec/changes/archive/2026-08-02-harden-spec-provenance-production-readiness/tasks.md",
    "openspec/changes/archive/2026-08-28-integrate-spec-provenance-with-chelis/tasks.md",
    "openspec/changes/README.md",
    "src/quality.rs",
    "src/bin/buoy-quality.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
    "assurance/v1/quality-fixture-executions.tsv",
    "fixtures/quality/v1/repository/blocker.md",
    "fixtures/quality/v1/repository/configuration.toml",
    "fixtures/quality/v1/repository/evidence.txt",
    "fixtures/quality/v1/repository/formal.rs",
    "fixtures/quality/v1/repository/gate-omission.txt",
    "fixtures/quality/v1/repository/implementation.rs",
    "fixtures/quality/v1/repository/model.toml",
    "fixtures/quality/v1/repository/negative.txt",
    "fixtures/quality/v1/repository/owner.md",
    "fixtures/quality/v1/repository/positive.txt",
    "fixtures/quality/v1/repository/proof.rs",
    "fixtures/quality/v1/repository/result-error.tsv",
    "fixtures/quality/v1/repository/result-fail.tsv",
    "fixtures/quality/v1/repository/result-formal-pass.tsv",
    "fixtures/quality/v1/repository/result-linked-pass.tsv",
    "fixtures/quality/v1/repository/result-registry-pass.tsv",
    "fixtures/quality/v1/repository/result-timeout.tsv",
    "fixtures/quality/v1/repository/result.tsv",
    "fixtures/quality/v1/repository/tool.toml",
];

/// Copy every path in [`SCRATCH_COPY_FILES`] from the real repository
/// checkout rooted at `source` into an isolated scratch checkout rooted at
/// `destination`, preserving their relative layout.
///
/// Fixture-execution-evidence rendering reads a broad set of real repository
/// files (fixture registry result/tool/configuration authorities), so tests
/// that need to safely mutate the committed execution-evidence artifact must
/// operate on an isolated copy rather than the shared on-disk file, which is
/// unsafe under `cargo test`'s default parallel execution.
fn copy_repository_tree(source: &Path, destination: &Path) {
    for relative in SCRATCH_COPY_FILES {
        let source_path = source.join(relative);
        let destination_path = destination.join(relative);
        let parent = destination_path
            .parent()
            .expect("scratch checkout paths always have a parent");
        fs::create_dir_all(parent).expect("scratch repository directory must be writable");
        fs::copy(&source_path, &destination_path)
            .expect("copy repository file into scratch checkout");
    }
}

fn string_set(controls: &[toml::Value], field: &str) -> BTreeSet<String> {
    controls
        .iter()
        .map(|control| {
            control
                .get(field)
                .and_then(toml::Value::as_str)
                .unwrap_or_else(|| panic!("fixture control must have string field {field}"))
                .to_owned()
        })
        .collect()
}

#[test]
fn complete_positive_fixture_covers_every_closed_axis() {
    let text = fs::read_to_string(POSITIVE_FIXTURE).expect("read positive QA fixture");
    let parsed = toml::from_str::<toml::Value>(&text).expect("parse positive QA fixture");
    let controls = parsed
        .get("controls")
        .and_then(toml::Value::as_array)
        .expect("positive QA fixture controls");

    assert_eq!(
        string_set(controls, "category"),
        BTreeSet::from([
            "adversarial-campaign".to_owned(),
            "code-hygiene".to_owned(),
            "deterministic-fixtures".to_owned(),
            "finite-model".to_owned(),
            "formal-verification".to_owned(),
            "ordinary-verification".to_owned(),
            "platform-runtime".to_owned(),
            "registry-governance".to_owned(),
            "reproducibility".to_owned(),
            "source-coverage".to_owned(),
            "structural-enforcement".to_owned(),
            "supply-chain".to_owned(),
            "sampled-properties".to_owned(),
        ])
    );
    assert_eq!(
        string_set(controls, "implementation_state"),
        BTreeSet::from([
            "available".to_owned(),
            "blocked".to_owned(),
            "implemented".to_owned(),
            "planned".to_owned(),
            "retired".to_owned(),
        ])
    );
    assert_eq!(
        string_set(controls, "enforcement_state"),
        BTreeSet::from([
            "advisory".to_owned(),
            "blocking".to_owned(),
            "focused".to_owned(),
            "none".to_owned(),
        ])
    );
    assert_eq!(
        string_set(controls, "evidence_state"),
        BTreeSet::from([
            "absent".to_owned(),
            "current".to_owned(),
            "stale".to_owned(),
        ])
    );
    assert_eq!(
        string_set(controls, "execution_verdict"),
        BTreeSet::from([
            "error".to_owned(),
            "fail".to_owned(),
            "not-run".to_owned(),
            "pass".to_owned(),
            "timeout".to_owned(),
        ])
    );
    assert_eq!(
        string_set(controls, "assurance_class"),
        BTreeSet::from([
            "bounded-exhaustive".to_owned(),
            "formally-proved".to_owned(),
            "linked".to_owned(),
            "none".to_owned(),
            "review-only".to_owned(),
            "sampled".to_owned(),
            "structurally-enforced".to_owned(),
            "tested".to_owned(),
        ])
    );
    assert_eq!(
        string_set(controls, "lifecycle"),
        BTreeSet::from([
            "active".to_owned(),
            "replacement".to_owned(),
            "retired".to_owned(),
        ])
    );

    let ownership_forms = controls
        .iter()
        .map(|control| {
            control
                .get("owner")
                .and_then(|owner| owner.get("kind"))
                .and_then(toml::Value::as_str)
                .expect("fixture owner kind")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        ownership_forms,
        BTreeSet::from(["baseline".to_owned(), "change".to_owned()])
    );
}

#[test]
fn validator_self_test_enforces_every_frozen_fixture() {
    buoy::quality::self_test(Path::new(".")).expect("quality validator self-test");
}

#[test]
fn focused_quality_lane_is_a_single_blocking_phase_child_without_nested_lanes() {
    let task_file = std::env::var("DEVENV_TASK_FILE")
        .unwrap_or_else(|_| "fixtures/devenv/v1/valid-tasks.json".to_owned());
    let task_text = fs::read_to_string(task_file).expect("resolved Devenv task data");
    let tasks = serde_json::from_str::<Vec<serde_json::Value>>(&task_text)
        .expect("resolved Devenv task array");
    let quality_tasks = tasks
        .iter()
        .filter(|task| task["name"].as_str() == Some("buoy:quality"))
        .collect::<Vec<_>>();
    assert_eq!(quality_tasks.len(), 1);
    assert_eq!(
        quality_tasks[0]["after"],
        serde_json::json!(["buoy:gate-wiring"])
    );
    assert!(quality_tasks[0]["status"].is_null());
    assert_eq!(quality_tasks[0]["exec_if_modified"], serde_json::json!([]));

    let scripts =
        fs::read_to_string("nix/devenv/scripts/core.nix").expect("focused quality script module");
    let script = scripts
        .split_once("scripts.buoy-quality = {")
        .and_then(|(_, rest)| rest.split_once("\n  };"))
        .map(|(body, _)| body)
        .expect("focused quality script");
    for child in [
        "buoy-properties",
        "buoy-coverage",
        "buoy-fcis",
        "buoy-core-proofs",
        "cargo fuzz",
        "cargo mutants",
        "verus ",
    ] {
        assert!(
            !script.contains(child),
            "quality lane executes child {child}"
        );
    }
}

#[test]
fn live_generation_cli_and_binary_cover_success_and_usage_contracts() {
    let root = Path::new(".");
    let generated = buoy::quality::generated_artifacts(root).expect("live generated artifacts");
    assert!(generated.bindings.starts_with("schema\tcontrol_id"));
    assert!(
        generated
            .report
            .starts_with("# Quality-assurance control status")
    );
    buoy::quality::check_repository(root).expect("live generated artifacts are current");
    buoy::quality::run_cli(&["--check".to_owned()], root).expect("check through CLI parser");

    let output = tempfile::tempdir().expect("quality output directory");
    let arguments = vec![
        "--generate".to_owned(),
        "--root".to_owned(),
        ".".to_owned(),
        "--output".to_owned(),
        output.path().to_string_lossy().into_owned(),
    ];
    buoy::quality::run_cli(&arguments, root).expect("generate through CLI parser");
    assert!(
        output
            .path()
            .join("quality-assurance-bindings.tsv")
            .is_file()
    );
    assert!(output.path().join("quality-assurance-status.md").is_file());

    for invalid in [
        vec![],
        vec!["--unknown".to_owned()],
        vec!["--check".to_owned(), "--generate".to_owned()],
        vec![
            "--check".to_owned(),
            "--output".to_owned(),
            "unused".to_owned(),
        ],
        vec![
            "--self-test".to_owned(),
            "--output".to_owned(),
            "unused".to_owned(),
        ],
        vec![
            "--check-execution-evidence".to_owned(),
            "--output".to_owned(),
            "unused".to_owned(),
        ],
        vec![
            "--write-execution-evidence".to_owned(),
            "--output".to_owned(),
            "unused".to_owned(),
        ],
        vec!["--check".to_owned(), "--root".to_owned()],
        vec!["--check".to_owned(), "--output".to_owned()],
    ] {
        let error = buoy::quality::run_cli(&invalid, root).expect_err("invalid quality CLI");
        assert!(error.to_string().starts_with("QA-USAGE"));
    }

    let binary = env!("CARGO_BIN_EXE_buoy-quality");
    let success = Command::new(binary)
        .arg("--self-test")
        .status()
        .expect("run quality binary success");
    assert!(success.success());
    for arguments in [vec!["--unknown"], vec!["--check", "--generate"]] {
        let failure = Command::new(binary)
            .args(arguments)
            .output()
            .expect("run quality binary usage failure");
        assert_eq!(failure.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&failure.stderr).starts_with("QA-USAGE"));
    }
}

#[test]
fn fixture_execution_evidence_is_current_and_fails_closed_when_stale() {
    const EXECUTIONS: &str = "assurance/v1/quality-fixture-executions.tsv";

    // This test deliberately corrupts the fixture-execution-evidence
    // artifact to exercise the fail-closed path, then rewrites it. Doing
    // that against the real, shared repository file would be unsafe under
    // `cargo test`'s default parallel execution (another test could observe
    // the corrupted content, or two runs could race on the write). Instead,
    // operate entirely on an isolated scratch copy of the repository.
    let scratch = tempfile::tempdir().expect("scratch repository directory");
    copy_repository_tree(Path::new("."), scratch.path());
    let root = scratch.path();
    let executions_path = root.join(EXECUTIONS);

    // Happy path: the committed per-case execution-evidence artifact must be
    // byte-current with a deterministic regeneration from the live fixtures,
    // and both the direct function and the CLI dispatch must agree.
    buoy::quality::fixture_execution_evidence(root, false)
        .expect("committed fixture execution evidence must be current");
    buoy::quality::run_cli(&["--check-execution-evidence".to_owned()], root)
        .expect("check-execution-evidence through CLI parser");

    let original = fs::read_to_string(&executions_path)
        .expect("read committed fixture execution evidence in scratch checkout");

    // Fail-closed: a corrupted/stale artifact must be rejected, not silently
    // repaired, when only checking.
    fs::write(&executions_path, "stale\n")
        .expect("corrupt fixture execution evidence for control in scratch checkout");
    let error = buoy::quality::fixture_execution_evidence(root, false)
        .expect_err("stale fixture execution evidence must fail closed");
    assert!(error.to_string().starts_with("QA-EXECUTION-EVIDENCE"));
    let cli_error = buoy::quality::run_cli(&["--check-execution-evidence".to_owned()], root)
        .expect_err("stale fixture execution evidence must fail closed through the CLI");
    assert!(cli_error.to_string().starts_with("QA-EXECUTION-EVIDENCE"));

    // Round trip: the reviewed write flag regenerates the exact committed
    // content from the same deterministic render.
    buoy::quality::run_cli(&["--write-execution-evidence".to_owned()], root)
        .expect("write-execution-evidence through CLI parser");
    let rewritten = fs::read_to_string(&executions_path)
        .expect("read rewritten fixture execution evidence in scratch checkout");
    assert_eq!(rewritten, original);
    buoy::quality::fixture_execution_evidence(root, false)
        .expect("fixture execution evidence must be current after rewrite");
}

#[test]
fn quality_registry_remains_outside_the_no_std_core() {
    let core_manifest = fs::read_to_string("crates/buoy-core/Cargo.toml").expect("core manifest");
    let core_library = fs::read_to_string("crates/buoy-core/src/lib.rs").expect("core library");
    assert!(!core_manifest.contains("toml"));
    assert!(!core_manifest.contains("serde"));
    assert!(!core_library.contains("quality"));
    assert!(core_library.contains("#![no_std]"));
}

#[test]
fn negative_fixture_manifest_freezes_expected_diagnostics_and_cardinality() {
    let text = fs::read_to_string(CASE_MANIFEST).expect("read QA fixture manifest");
    let mut case_ids = BTreeSet::new();
    let mut negative_count = 0_usize;
    let mut positive_count = 0_usize;
    let mut covered = String::new();

    for (index, line) in text.lines().enumerate() {
        if index == 0 {
            assert_eq!(
                line,
                "schema\tcase_id\tpolarity\tmutation\texpected_diagnostic\texpected_count\tcovers"
            );
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 7, "manifest row {} shape", index + 1);
        assert_eq!(fields[0], "quality-assurance-fixture-case/v1");
        assert!(
            case_ids.insert(fields[1]),
            "duplicate case ID {}",
            fields[1]
        );
        covered.push_str(fields[6]);
        covered.push('\n');
        match fields[2] {
            "positive" => {
                positive_count += 1;
                assert_eq!(fields[4], "-");
                assert_eq!(fields[5], "0");
            }
            "negative" => {
                negative_count += 1;
                assert!(fields[4].starts_with("QA-"));
                assert_eq!(fields[5], "1");
            }
            other => panic!("unknown fixture polarity {other}"),
        }
    }

    assert_eq!(positive_count, 3);
    assert_eq!(negative_count, 58);
    for required in [
        "missing required field",
        "pinned tool is not implementation",
        "single implementation owner",
        "absolute-root independence",
        "sampled is not exhaustive",
    ] {
        assert!(
            covered.contains(required),
            "missing fixture coverage: {required}"
        );
    }
}
