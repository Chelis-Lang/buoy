#![doc = "Deterministic bounded fault injection for repository, parser, budget, and process effects."]
#![allow(clippy::expect_used)]

use buoy::adapters::{AuthorityExpectation, extract_markdown, extract_rust};
use buoy::core::{RepositoryPath, Verdict};
#[cfg(all(unix, feature = "coverage-controls"))]
use buoy::oracle::run_oracle_input_fault_controlled;
#[cfg(unix)]
use buoy::oracle::{CancellationToken, OracleTermination, run_oracle, run_oracle_controlled};
#[cfg(all(unix, feature = "coverage-controls"))]
use buoy::repository::snapshot_repository_fault_control;
use buoy::repository::{OracleCommand, RepositoryConfig, RepositoryError};
use std::collections::BTreeMap;
#[cfg(all(unix, feature = "coverage-controls"))]
use std::{fs, path::PathBuf};
#[cfg(any(unix, feature = "coverage-controls"))]
use tempfile::TempDir;

#[cfg(all(unix, feature = "coverage-controls"))]
fn config() -> RepositoryConfig {
    RepositoryConfig::parse(
        br#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]
[[selectors]]
root = "spec"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "optional"
"#,
    )
    .expect("fault configuration")
}

#[test]
fn fault_catalog_names_every_required_injected_failure_class() {
    let catalog = include_str!("../assurance/adversarial/v1/fault-catalog.tsv");
    for category in [
        "filesystem",
        "parser",
        "allocation-budget",
        "process",
        "pipe",
        "result",
        "timeout",
        "cancellation",
    ] {
        assert!(
            catalog
                .lines()
                .any(|line| line.split('\t').nth(2) == Some(category)),
            "missing fault category {category}"
        );
    }
}

#[test]
fn parser_and_allocation_faults_fail_closed() {
    let markdown_path = RepositoryPath::new("spec/fault.md").expect("Markdown fault path");
    let rust_path = RepositoryPath::new("src/fault.rs").expect("Rust fault path");
    assert_eq!(
        extract_markdown(&markdown_path, &[0xff], AuthorityExpectation::Optional).diagnostics[0]
            .code,
        "SPG-ADAPTER-INVALID-UTF8"
    );
    assert_eq!(
        extract_rust(&rust_path, &[0xff]).diagnostics[0].code,
        "SPG-ADAPTER-INVALID-UTF8"
    );
    assert_eq!(
        RepositoryConfig::parse(&vec![b'x'; 1_048_577]),
        Err(RepositoryError::ConfigBytesBudget)
    );
}

#[cfg(all(unix, feature = "coverage-controls"))]
#[test]
fn filesystem_faults_fail_closed() {
    let temporary = TempDir::new().expect("fault root");
    let spec: PathBuf = temporary.path().join("spec");
    fs::create_dir_all(&spec).expect("fault spec directory");
    fs::write(spec.join("input.md"), b"input").expect("fault input");
    for (fault, expected) in [
        ("file-read", RepositoryError::ReadFailure),
        ("actual-total", RepositoryError::TotalBytesBudget),
        ("stat", RepositoryError::ReadFailure),
    ] {
        assert_eq!(
            snapshot_repository_fault_control(temporary.path(), &config(), fault),
            Err(expected),
            "fault {fault}"
        );
    }
}

#[cfg(unix)]
fn command(id: &str, program: &str, args: Vec<String>, timeout_ms: u64) -> OracleCommand {
    OracleCommand {
        id: id.to_string(),
        program: program.to_string(),
        args,
        environment: BTreeMap::new(),
        working_root: String::from("repository-root"),
        input: None,
        timeout_ms,
        derivation: None,
        stdout_limit: 1_024,
        stderr_limit: 1_024,
        combined_output_limit: 2_048,
        raw_log_retention: 64,
        result_protocol: String::from("spec-provenance-result/v1"),
    }
}

#[cfg(unix)]
#[test]
fn process_pipe_result_timeout_and_cancellation_faults_are_bounded() {
    let temporary = TempDir::new().expect("process fault root");
    let missing = command(
        "FAULT-SPAWN",
        "/definitely/not/a/buoy-fault-program",
        Vec::new(),
        100,
    );
    assert_eq!(
        run_oracle(&missing, temporary.path()).termination,
        OracleTermination::SpawnError
    );

    let mut pipe = command(
        "FAULT-PIPE",
        "/bin/sh",
        vec![
            String::from("-c"),
            String::from("while :; do printf 0123456789 >&2; done"),
        ],
        2_000,
    );
    pipe.stderr_limit = 64;
    pipe.combined_output_limit = 128;
    let pipe = run_oracle(&pipe, temporary.path());
    assert_eq!(pipe.termination, OracleTermination::OutputOverflow);
    assert_eq!(pipe.verdict, Verdict::Error);
    assert!(pipe.cleanup_complete);
    assert!(pipe.raw_stderr.len() <= 64);

    let malformed = command(
        "FAULT-RESULT",
        "/bin/sh",
        vec![String::from("-c"), String::from("exit 0")],
        1_000,
    );
    assert_eq!(
        run_oracle(&malformed, temporary.path()).termination,
        OracleTermination::MalformedResult
    );

    let timeout = command(
        "FAULT-TIMEOUT",
        "/bin/sh",
        vec![String::from("-c"), String::from("sleep 1")],
        1,
    );
    let timeout = run_oracle(&timeout, temporary.path());
    assert_eq!(timeout.termination, OracleTermination::TimedOut);
    assert_eq!(timeout.verdict, Verdict::Timeout);
    assert!(timeout.cleanup_complete);

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = run_oracle_controlled(
        &command(
            "FAULT-CANCEL",
            "/bin/sh",
            vec![String::from("-c"), String::from("sleep 1")],
            1_000,
        ),
        temporary.path(),
        &cancellation,
    );
    assert_eq!(cancelled.termination, OracleTermination::Cancelled);
    assert!(cancelled.cleanup_complete);

    #[cfg(feature = "coverage-controls")]
    {
        let mut input_fault = command(
            "FAULT-STDIN-WRITER",
            "/bin/sh",
            vec![
                String::from("-c"),
                String::from("printf 'spec-provenance-result/v1\\tpass\\n'"),
            ],
            1_000,
        );
        input_fault.input = Some(String::from("input"));
        let input_fault = run_oracle_input_fault_controlled(
            &input_fault,
            temporary.path(),
            &CancellationToken::new(),
        );
        assert_eq!(input_fault.termination, OracleTermination::Completed);
        assert!(input_fault.protocol_valid);
        assert!(!input_fault.cleanup_complete);
    }
}
