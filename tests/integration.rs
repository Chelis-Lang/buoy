#![doc = "Isolated repository, CLI exit-class, oracle, and frontend parity tests."]
#![allow(
    clippy::default_trait_access,
    clippy::expect_used,
    clippy::too_many_lines
)]

#[cfg(all(feature = "coverage-controls", unix))]
#[path = "../tools/digest_binding.rs"]
mod content_digest;
#[cfg(all(feature = "coverage-controls", unix))]
#[path = "../tools/execution_evidence.rs"]
mod execution_evidence;

use buoy::adapters::AuthorityExpectation;
use buoy::check::{
    CheckError, MachineReport, check_snapshot, lint_saved_snapshot, lsp_saved_snapshot,
    render_human, render_machine, requirement_subject_facts, verdict_name,
};
#[cfg(feature = "coverage-controls")]
use buoy::check::{
    migration_kind_control, project_adapter_error_control, project_diagnostic_control,
};
use buoy::core::{
    CanonicalValue, EntryKind, LogicalMountId, RawArtifactMediaType, RejectedEntryObservation,
    RepositoryPath, SemanticDigest, Verdict, raw_artifact_digest, semantic_digest,
};
#[cfg(all(feature = "coverage-controls", unix))]
use buoy::core::{canonical_paths, portable_case_fold};
#[cfg(all(feature = "coverage-controls", unix))]
use buoy::glob::PortableGlob;
use buoy::oracle::run_oracle;
#[cfg(unix)]
use buoy::oracle::{
    CancellationToken, OracleSetError, OracleTermination, run_oracle_controlled,
    run_oracles_at_roots_controlled, run_oracles_controlled,
};
#[cfg(all(feature = "coverage-controls", unix))]
use buoy::oracle::{oracle_reader_fault_control, run_oracle_wait_fault_controlled};
#[cfg(all(feature = "coverage-controls", unix))]
use buoy::repository::windows_reparse_attributes_control;
use buoy::repository::{
    MountMappings, OracleCommand, RepositoryConfig, RepositoryError, RepositorySnapshot,
    SnapshotBudgets, SnapshotFile, load_config, snapshot_mounted_repository, snapshot_repository,
};
#[cfg(feature = "coverage-controls")]
use buoy::repository::{canonical_value_set_control, validate_selected_paths_control};
#[cfg(all(feature = "coverage-controls", unix))]
use buoy::repository::{
    read_bounded_control, snapshot_mounted_repository_after_roots_open_control,
    snapshot_repository_after_root_open_control, snapshot_repository_fault_control,
};
use std::collections::BTreeMap;
#[cfg(all(feature = "coverage-controls", unix))]
use std::collections::BTreeSet;
use std::fs;
#[cfg(all(feature = "coverage-controls", unix))]
use std::io::{Error, ErrorKind, Read};
#[cfg(all(feature = "coverage-controls", unix))]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::{Duration, Instant};
use tempfile::TempDir;
#[cfg(all(feature = "coverage-controls", unix))]
use unicode_normalization::UnicodeNormalization as _;

fn mounted_artifact_config(extra: &str) -> RepositoryConfig {
    RepositoryConfig::parse(
        format!(
            r#"schema = "buoy/repository-config/v3"
digest_algorithms = ["xxh3-128"]

[[mounts]]
id = "authority"
prefix = "authority"

[[mounts]]
id = "project"
prefix = "project"

[[selectors]]
mount = "authority"
root = "src"
adapter = "artifact/v1"
include = ["**/*"]
exclude = []
authority_expectation = "forbidden"

[[selectors]]
mount = "project"
root = "src"
adapter = "artifact/v1"
include = ["**/*"]
exclude = []
authority_expectation = "forbidden"
{extra}
"#,
        )
        .as_bytes(),
    )
    .expect("mounted artifact configuration")
}

fn mounted_mappings(project: &Path, authority: &Path) -> MountMappings {
    MountMappings::new(vec![
        (
            LogicalMountId::new("project").expect("project mount ID"),
            project.to_path_buf(),
        ),
        (
            LogicalMountId::new("authority").expect("authority mount ID"),
            authority.to_path_buf(),
        ),
    ])
    .expect("complete mounted roots")
}

const ATOM_BASE: &str = r#"```spec-provenance-atom-v1
id = "SPG-INTEGRATION-001"
state = "ratified"
kind = "behavioral"
scope = ["spec.integration"]
statement = "The static command MUST use only explicit saved repository inputs."

[approval]
authority = "maintainers"
record = "reviews/SPG-INTEGRATION-001.md"
digest = "xxh3-128:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
```
"#;

const ATOM_HEAD: &str = r#"```spec-provenance-atom-v1
id = "SPG-INTEGRATION-001"
state = "ratified"
kind = "behavioral"
scope = ["spec.integration"]
statement = "The static command MUST use only explicit current saved repository inputs."

[approval]
authority = "maintainers"
record = "reviews/SPG-INTEGRATION-001.md"
digest = "xxh3-128:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
```
"#;

fn config(oracles: &str) -> String {
    format!(
        r#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[[selectors]]
root = "spec"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "required"
{oracles}
"#
    )
}

#[cfg(unix)]
#[test]
fn correspondence_index_is_non_authoritative_for_static_identity() {
    let root = TempDir::new().expect("correspondence non-authority root");
    fs::create_dir_all(root.path().join("spec")).expect("spec directory");
    fs::write(root.path().join("spec/requirements.md"), ATOM_BASE).expect("standing spec");
    let parsed = RepositoryConfig::parse(config("").as_bytes()).expect("repository configuration");

    let before_snapshot =
        snapshot_repository(root.path(), &parsed).expect("snapshot without index");
    let before = check_snapshot(&before_snapshot, &parsed).expect("static check without index");

    let index_path = root
        .path()
        .join("assurance/v1/digest-correspondence-index.tsv");
    fs::create_dir_all(index_path.parent().expect("index parent")).expect("index directory");
    fs::write(
        &index_path,
        "stable_id\tobject_kind\tsource_path\tsource_owner\txxh3_128\tblake3_256\tsha_256\n",
    )
    .expect("correspondence index");
    let with_index_snapshot =
        snapshot_repository(root.path(), &parsed).expect("snapshot with index");
    let with_index =
        check_snapshot(&with_index_snapshot, &parsed).expect("static check with index");

    fs::remove_file(&index_path).expect("remove correspondence index");
    let after_snapshot = snapshot_repository(root.path(), &parsed).expect("snapshot after removal");
    let after = check_snapshot(&after_snapshot, &parsed).expect("static check after removal");

    assert_eq!(before_snapshot, with_index_snapshot);
    assert_eq!(before_snapshot, after_snapshot);
    assert_eq!(before, with_index);
    assert_eq!(before, after);
}

#[test]
fn repository_configuration_identity_is_complete_and_order_independent() {
    let baseline = RepositoryConfig::parse(config("").as_bytes()).ok();
    assert!(baseline.is_some());
    let Some(baseline) = baseline else { return };
    let baseline_digest = baseline.semantic_digest().ok();
    assert!(baseline_digest.is_some());
    assert!(baseline.oracles().is_empty());
    assert_eq!(baseline.oracle_concurrency(), 8);
    assert_eq!(
        baseline.budgets(),
        SnapshotBudgets {
            path_depth: 64,
            directory_entries: 100_000,
            selected_files: 10_000,
            file_bytes: 16_777_216,
            total_bytes: 536_870_912,
            graph_objects: 200_000,
            graph_references: 2_000_000,
            diagnostics: 100_000,
            source_nodes: 200_000,
            source_edges: 400_000,
            source_depth: 64,
            source_visited: 200_000,
        }
    );
    assert_eq!(SnapshotBudgets::default(), baseline.budgets());

    let defaulted_oracle = RepositoryConfig::parse(
        config(
            r#"
[[oracles]]
id = "ORC-DEFAULTS"
program = "builtin:pass"
timeout_ms = 1000
"#,
        )
        .as_bytes(),
    )
    .expect("defaulted oracle configuration");
    let defaulted_oracle = &defaulted_oracle.oracles()[0];
    assert_eq!(defaulted_oracle.stdout_limit, 8_388_608);
    assert_eq!(defaulted_oracle.stderr_limit, 8_388_608);
    assert_eq!(defaulted_oracle.combined_output_limit, 16_777_216);
    assert_eq!(defaulted_oracle.raw_log_retention, 1_048_576);
    assert_eq!(defaulted_oracle.working_root, "repository-root");
    assert_eq!(
        defaulted_oracle.result_protocol,
        "spec-provenance-result/v1"
    );

    let variants = [
        config("").replace("**/*.md", "*.md"),
        config("").replace("required", "optional"),
        config("").replace("required", "forbidden"),
        config("").replacen(
            "schema = \"spec-provenance-config/v2\"",
            "schema = \"spec-provenance-config/v2\"\noracle_concurrency = 7",
            1,
        ),
        config(
            r#"
[[oracles]]
id = "ORC-IDENTITY"
program = "builtin:pass"
timeout_ms = 1000
"#,
        ),
    ];
    for variant in variants {
        let parsed = RepositoryConfig::parse(variant.as_bytes()).ok();
        assert!(parsed.is_some());
        assert_ne!(
            parsed.and_then(|value| value.semantic_digest().ok()),
            baseline_digest
        );
    }

    let full_oracle = RepositoryConfig::parse(
        config(
            r#"
[[oracles]]
id = "ORC-FULL"
program = "builtin:pass"
args = ["--control"]
input = "semantic input"
timeout_ms = 1000
derivation = "DERIVATION-CONTROL"
[oracles.environment]
CONTROL = "1"
"#,
        )
        .as_bytes(),
    )
    .expect("full oracle configuration");
    assert_eq!(full_oracle.oracles().len(), 1);
    assert!(full_oracle.semantic_digest().is_ok());

    let first = config("").replace("[\"**/*.md\"]", "[\"**/*.md\", \"*.txt\"]");
    let second = config("").replace("[\"**/*.md\"]", "[\"*.txt\", \"**/*.md\"]");
    let first = RepositoryConfig::parse(first.as_bytes()).ok();
    let second = RepositoryConfig::parse(second.as_bytes()).ok();
    assert!(first.is_some() && second.is_some());
    assert_eq!(
        first.and_then(|value| value.semantic_digest().ok()),
        second.and_then(|value| value.semantic_digest().ok())
    );

    let duplicate = config("").replace("[\"**/*.md\"]", "[\"**/*.md\", \"**/*.md\"]");
    assert_eq!(
        RepositoryConfig::parse(duplicate.as_bytes()),
        Err(RepositoryError::ConfigSchema)
    );
    assert_eq!(
        RepositoryConfig::parse(&vec![b' '; 1_048_577]),
        Err(RepositoryError::ConfigBytesBudget)
    );
}

#[test]
fn snapshot_encoders_match_frozen_field_tables() {
    let path = RepositoryPath::new("spec/example.rs").expect("snapshot path");
    let file = SnapshotFile {
        path: path.clone(),
        bytes: b"fn example() {}\n".to_vec(),
        adapter: String::from("rust/v1"),
        authority_expectation: AuthorityExpectation::Optional,
        selector_root: String::from("spec"),
    };
    let raw_digest = raw_artifact_digest(RawArtifactMediaType::Rust, &file.bytes);
    let markdown = SnapshotFile {
        path: RepositoryPath::new("spec/example.md").expect("Markdown snapshot path"),
        bytes: b"plain Markdown\n".to_vec(),
        adapter: String::from("markdown/v1"),
        authority_expectation: AuthorityExpectation::Optional,
        selector_root: String::from("spec"),
    };
    assert!(markdown.canonical_value().is_ok());
    let file_value = CanonicalValue::Record(vec![
        (1, CanonicalValue::Text(path.as_str().to_string())),
        (2, CanonicalValue::Enum(EntryKind::RegularFile as u16, None)),
        (3, CanonicalValue::Digest(raw_digest)),
    ]);
    assert_eq!(file.canonical_value(), Ok(file_value.clone()));
    assert_eq!(
        file.semantic_digest(),
        semantic_digest(
            "snapshot-file",
            vec![
                (1, CanonicalValue::Text(path.as_str().to_string())),
                (2, CanonicalValue::Enum(1, None)),
                (3, CanonicalValue::Digest(raw_digest)),
            ],
        )
        .map_err(|_| RepositoryError::SnapshotEncoding)
    );

    let rejected_path = RepositoryPath::new("spec/link.rs").expect("rejected path");
    let rejected =
        RejectedEntryObservation::new(rejected_path.clone(), EntryKind::Symlink, "DET-PATH-LINK")
            .expect("rejected observation");
    let config_digest = SemanticDigest::xxh3_128([81; 16]);
    let tool_lock_digest = SemanticDigest::xxh3_128([82; 16]);
    let snapshot = RepositorySnapshot {
        files: vec![file.clone()],
    };
    assert_eq!(
        snapshot.semantic_digest(
            config_digest,
            tool_lock_digest,
            std::slice::from_ref(&rejected),
        ),
        semantic_digest(
            "repository-snapshot",
            vec![
                (1, CanonicalValue::Digest(config_digest)),
                (2, CanonicalValue::Digest(tool_lock_digest)),
                (3, CanonicalValue::Set(vec![file_value])),
                (
                    4,
                    CanonicalValue::Set(vec![CanonicalValue::Record(vec![
                        (1, CanonicalValue::Text(rejected_path.as_str().to_string())),
                        (2, CanonicalValue::Enum(EntryKind::Symlink as u16, None)),
                        (3, CanonicalValue::Text(String::from("DET-PATH-LINK"))),
                    ])]),
                ),
            ],
        )
        .map_err(|_| RepositoryError::SnapshotEncoding)
    );

    let unsupported = SnapshotFile {
        adapter: String::from("unsupported/v1"),
        ..file.clone()
    };
    assert_eq!(
        unsupported.canonical_value(),
        Err(RepositoryError::UnsupportedAdapter)
    );
    assert_eq!(
        unsupported.semantic_digest(),
        Err(RepositoryError::UnsupportedAdapter)
    );
    assert_eq!(
        RepositorySnapshot {
            files: vec![unsupported],
        }
        .semantic_digest(config_digest, tool_lock_digest, &[]),
        Err(RepositoryError::UnsupportedAdapter)
    );
    assert_eq!(
        RepositorySnapshot {
            files: vec![file.clone(), file],
        }
        .semantic_digest(config_digest, tool_lock_digest, &[]),
        Err(RepositoryError::SnapshotEncoding)
    );
    assert_eq!(
        snapshot.semantic_digest(
            config_digest,
            tool_lock_digest,
            &[rejected.clone(), rejected]
        ),
        Err(RepositoryError::SnapshotEncoding)
    );
    #[cfg(feature = "coverage-controls")]
    assert_eq!(
        canonical_value_set_control(vec![CanonicalValue::Record(vec![
            (1, CanonicalValue::False),
            (1, CanonicalValue::True),
        ])]),
        Err(RepositoryError::SnapshotEncoding)
    );
}

#[test]
fn repository_digest_algorithm_configuration_fails_closed() {
    let baseline = config("");
    let missing = baseline.replace(
        "digest_algorithms = [\"xxh3-128\"]",
        "digest_algorithms = []",
    );
    assert_eq!(
        RepositoryConfig::parse(missing.as_bytes()),
        Err(RepositoryError::DigestActiveMissing)
    );
    let multiple = baseline.replace(
        "digest_algorithms = [\"xxh3-128\"]",
        "digest_algorithms = [\"xxh3-128\", \"sha-256\"]",
    );
    assert_eq!(
        RepositoryConfig::parse(multiple.as_bytes()),
        Err(RepositoryError::DigestActiveMultiple)
    );
    let unknown = baseline.replace(
        "digest_algorithms = [\"xxh3-128\"]",
        "digest_algorithms = [\"sha-512\"]",
    );
    assert_eq!(
        RepositoryConfig::parse(unknown.as_bytes()),
        Err(RepositoryError::DigestAlgorithmUnknown)
    );
    #[cfg(not(feature = "sha-256"))]
    {
        let unavailable = baseline.replace(
            "digest_algorithms = [\"xxh3-128\"]",
            "digest_algorithms = [\"sha-256\"]",
        );
        let error = RepositoryConfig::parse(unavailable.as_bytes())
            .expect_err("missing sha-256 feature must fail");
        assert_eq!(error, RepositoryError::DigestAlgorithmUnavailable);
        assert_eq!(error.code(), "DIG-ALGORITHM-UNAVAILABLE");
    }
}

#[test]
fn repository_configuration_rejects_each_closed_invalid_axis() {
    assert_eq!(
        RepositoryConfig::parse(&[0xff]),
        Err(RepositoryError::ConfigUtf8)
    );
    assert_eq!(
        RepositoryConfig::parse(b"not = [toml"),
        Err(RepositoryError::ConfigSyntax)
    );
    for invalid in [
        String::from(
            "schema = \"spec-provenance-config/v2\"\ndigest_algorithms = [\"xxh3-128\"]\nselectors = []\n",
        ),
        config("").replace("include = [\"**/*.md\"]", "include = []"),
        config("").replace("spec-provenance-config/v2", "wrong/v2"),
    ] {
        assert_eq!(
            RepositoryConfig::parse(invalid.as_bytes()),
            Err(RepositoryError::ConfigSchema)
        );
    }
    assert_eq!(
        RepositoryConfig::parse(
            config("")
                .replace("root = \"spec\"", "root = \"/spec\"")
                .as_bytes()
        ),
        Err(RepositoryError::PathPolicy)
    );
    assert_eq!(
        RepositoryConfig::parse(config("").replace("required", "unknown").as_bytes()),
        Err(RepositoryError::ConfigSchema)
    );
    assert_eq!(
        RepositoryConfig::parse(
            config("")
                .replacen(
                    "schema = \"spec-provenance-config/v2\"",
                    "schema = \"spec-provenance-config/v2\"\noracle_concurrency = 0",
                    1,
                )
                .as_bytes(),
        ),
        Err(RepositoryError::OracleConcurrencyBudget)
    );

    let oracle = r#"
[[oracles]]
id = "ORC-CONTROL"
program = "builtin:pass"
timeout_ms = 1000
"#;
    for (needle, replacement, expected) in [
        (
            "timeout_ms = 1000",
            "timeout_ms = 0",
            RepositoryError::OracleTimeoutBudget,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 3600001",
            RepositoryError::OracleTimeoutBudget,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\nstdout_limit = 0",
            RepositoryError::OracleOutputBudget,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\nstdout_limit = 67108865",
            RepositoryError::OracleOutputBudget,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\nstderr_limit = 0",
            RepositoryError::OracleOutputBudget,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\nstderr_limit = 67108865",
            RepositoryError::OracleOutputBudget,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\ncombined_output_limit = 0",
            RepositoryError::OracleOutputBudget,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\ncombined_output_limit = 134217729",
            RepositoryError::OracleOutputBudget,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\nraw_log_retention = 8388609",
            RepositoryError::OracleOutputBudget,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\nresult_protocol = \"wrong/v1\"",
            RepositoryError::OracleProtocol,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\nworking_root = \"wrong-root\"",
            RepositoryError::OracleProtocol,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\n[oracles.environment]\nlower = \"value\"",
            RepositoryError::ConfigSchema,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\n[oracles.environment]\n\"\" = \"value\"",
            RepositoryError::ConfigSchema,
        ),
        (
            "timeout_ms = 1000",
            "timeout_ms = 1000\n[oracles.environment]\n\"A=B\" = \"value\"",
            RepositoryError::ConfigSchema,
        ),
    ] {
        assert_eq!(
            RepositoryConfig::parse(config(&oracle.replace(needle, replacement)).as_bytes()),
            Err(expected)
        );
    }
    let duplicate = format!("{oracle}{oracle}").replace("ORC-CONTROL", "ORC-DUPLICATE");
    assert_eq!(
        RepositoryConfig::parse(config(&duplicate).as_bytes()),
        Err(RepositoryError::ConfigSchema)
    );
    for (field, value, expected) in [
        ("path_depth", 0_u64, RepositoryError::PathDepthBudget),
        (
            "directory_entries",
            0,
            RepositoryError::DirectoryEntriesBudget,
        ),
        ("selected_files", 0, RepositoryError::SelectedFilesBudget),
        ("file_bytes", 0, RepositoryError::FileBytesBudget),
        ("total_bytes", 0, RepositoryError::TotalBytesBudget),
        ("path_depth", 257_u64, RepositoryError::PathDepthBudget),
        (
            "directory_entries",
            1_000_001,
            RepositoryError::DirectoryEntriesBudget,
        ),
        (
            "selected_files",
            100_001,
            RepositoryError::SelectedFilesBudget,
        ),
        ("file_bytes", 268_435_457, RepositoryError::FileBytesBudget),
        (
            "total_bytes",
            4_294_967_297,
            RepositoryError::TotalBytesBudget,
        ),
        ("graph_objects", 0, RepositoryError::GraphObjectsBudget),
        (
            "graph_objects",
            1_000_001,
            RepositoryError::GraphObjectsBudget,
        ),
        (
            "graph_references",
            0,
            RepositoryError::GraphReferencesBudget,
        ),
        (
            "graph_references",
            10_000_001,
            RepositoryError::GraphReferencesBudget,
        ),
        ("diagnostics", 0, RepositoryError::DiagnosticsBudget),
        ("diagnostics", 1_000_001, RepositoryError::DiagnosticsBudget),
        ("source_nodes", 0, RepositoryError::SourceNodesBudget),
        ("source_edges", 0, RepositoryError::SourceEdgesBudget),
        ("source_depth", 0, RepositoryError::SourceDepthBudget),
        ("source_visited", 0, RepositoryError::SourceVisitedBudget),
    ] {
        let mut values = BTreeMap::from([
            ("path_depth", 64_u64),
            ("directory_entries", 100_000),
            ("selected_files", 10_000),
            ("file_bytes", 16_777_216),
            ("total_bytes", 536_870_912),
            ("graph_objects", 200_000),
            ("graph_references", 2_000_000),
            ("diagnostics", 100_000),
            ("source_nodes", 200_000),
            ("source_edges", 400_000),
            ("source_depth", 64),
            ("source_visited", 200_000),
        ]);
        values.insert(field, value);
        let with_budget = format!(
            "{}\n[budgets]\npath_depth = {}\ndirectory_entries = {}\nselected_files = {}\nfile_bytes = {}\ntotal_bytes = {}\ngraph_objects = {}\ngraph_references = {}\ndiagnostics = {}\nsource_nodes = {}\nsource_edges = {}\nsource_depth = {}\nsource_visited = {}\n",
            config(""),
            values["path_depth"],
            values["directory_entries"],
            values["selected_files"],
            values["file_bytes"],
            values["total_bytes"],
            values["graph_objects"],
            values["graph_references"],
            values["diagnostics"],
            values["source_nodes"],
            values["source_edges"],
            values["source_depth"],
            values["source_visited"],
        );
        assert_eq!(
            RepositoryConfig::parse(with_budget.as_bytes()),
            Err(expected)
        );
    }
    let temporary = TempDir::new().expect("configuration I/O controls");
    assert_eq!(
        load_config(&temporary.path().join("missing.toml")),
        Err(RepositoryError::ReadFailure)
    );
    assert_eq!(
        load_config(temporary.path()),
        Err(RepositoryError::ReadFailure)
    );
    let oversized_path = temporary.path().join("oversized.toml");
    fs::write(&oversized_path, vec![b' '; 1_048_577]).expect("oversized config writes");
    assert_eq!(
        load_config(&oversized_path),
        Err(RepositoryError::ConfigBytesBudget)
    );

    let large_input = format!("{oracle}input = {:?}\n", "x".repeat(1_048_577));
    assert_eq!(
        RepositoryConfig::parse(config(&large_input).as_bytes()),
        Err(RepositoryError::ConfigBytesBudget)
    );
}

#[test]
fn repository_error_codes_are_complete_and_stable() {
    for error in [
        RepositoryError::ConfigUtf8,
        RepositoryError::ConfigSyntax,
        RepositoryError::ConfigBytesBudget,
        RepositoryError::ConfigSchema,
        RepositoryError::DigestActiveMissing,
        RepositoryError::DigestActiveMultiple,
        RepositoryError::DigestAlgorithmUnknown,
        RepositoryError::DigestAlgorithmUnavailable,
        RepositoryError::SelectorRoot,
        RepositoryError::LinkEntry,
        RepositoryError::SubmoduleEntry,
        RepositoryError::PathPolicy,
        RepositoryError::SelectionCollision,
        RepositoryError::ReadFailure,
        RepositoryError::UnsupportedAdapter,
        RepositoryError::ArtifactIndex,
        RepositoryError::SnapshotEncoding,
        RepositoryError::GlobSyntax,
        RepositoryError::PathDepthBudget,
        RepositoryError::DirectoryEntriesBudget,
        RepositoryError::SelectedFilesBudget,
        RepositoryError::FileBytesBudget,
        RepositoryError::TotalBytesBudget,
        RepositoryError::GraphObjectsBudget,
        RepositoryError::GraphReferencesBudget,
        RepositoryError::DiagnosticsBudget,
        RepositoryError::SourceNodesBudget,
        RepositoryError::SourceEdgesBudget,
        RepositoryError::SourceDepthBudget,
        RepositoryError::SourceVisitedBudget,
        RepositoryError::OracleTimeoutBudget,
        RepositoryError::OracleOutputBudget,
        RepositoryError::OracleProtocol,
        RepositoryError::OracleConcurrencyBudget,
    ] {
        assert!(!error.code().is_empty());
    }
}

fn oracle_command(
    id: &str,
    program: &str,
    args: Vec<String>,
    timeout_ms: u64,
    derivation: Option<String>,
) -> OracleCommand {
    OracleCommand {
        id: id.to_owned(),
        program: program.to_owned(),
        args,
        environment: BTreeMap::new(),
        working_root: String::from("repository-root"),
        input: None,
        timeout_ms,
        derivation,
        stdout_limit: 8_388_608,
        stderr_limit: 8_388_608,
        combined_output_limit: 16_777_216,
        raw_log_retention: 1_048_576,
        result_protocol: String::from("spec-provenance-result/v1"),
    }
}

fn repository(atom: &str, oracles: &str) -> Option<(TempDir, PathBuf)> {
    let temporary = TempDir::new().ok()?;
    let spec = temporary.path().join("spec");
    fs::create_dir_all(&spec).ok()?;
    fs::write(spec.join("requirements.md"), atom).ok()?;
    let config_path = temporary.path().join("buoy.toml");
    fs::write(&config_path, config(oracles)).ok()?;
    Some((temporary, config_path))
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_buoy"))
}

fn run(arguments: &[&str]) -> Option<Output> {
    let direct = buoy::cli::run(
        &arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect::<Vec<_>>(),
    );
    let output = Command::new(binary()).args(arguments).output().ok()?;
    assert_eq!(output.status.code(), Some(i32::from(direct)));
    Some(output)
}

fn root_text(path: &Path) -> Option<&str> {
    path.to_str()
}

#[test]
fn snapshots_are_cross_root_deterministic_and_frontends_share_semantics() {
    let first = repository(ATOM_BASE, "");
    let second = repository(ATOM_BASE, "");
    assert!(first.is_some() && second.is_some());
    let (Some((first, first_config)), Some((second, second_config))) = (first, second) else {
        return;
    };
    let first_loaded = load_config(&first_config);
    let second_loaded = load_config(&second_config);
    assert!(first_loaded.is_ok() && second_loaded.is_ok());
    let (Ok(first_loaded), Ok(second_loaded)) = (first_loaded, second_loaded) else {
        return;
    };
    let first_snapshot = snapshot_repository(first.path(), &first_loaded);
    let second_snapshot = snapshot_repository(second.path(), &second_loaded);
    assert!(first_snapshot.is_ok() && second_snapshot.is_ok());
    let (Ok(first_snapshot), Ok(second_snapshot)) = (first_snapshot, second_snapshot) else {
        return;
    };
    assert!(first_snapshot.artifact_index().is_ok());
    let mut rust = first_snapshot.clone();
    rust.files[0].adapter = String::from("rust/v1");
    assert!(rust.artifact_index().is_ok());
    let mut unsupported = first_snapshot.clone();
    unsupported.files[0].adapter = String::from("unsupported/v1");
    assert_eq!(
        unsupported.artifact_index(),
        Err(RepositoryError::UnsupportedAdapter)
    );
    assert_eq!(first_snapshot, second_snapshot);
    let first_artifacts = first_snapshot.artifact_index();
    let second_artifacts = second_snapshot.artifact_index();
    assert!(first_artifacts.is_ok() && second_artifacts.is_ok());
    assert_eq!(first_artifacts, second_artifacts);
    assert_eq!(
        first_artifacts
            .as_ref()
            .ok()
            .map(|index| index.entries().len()),
        Some(1)
    );
    let first_result = check_snapshot(&first_snapshot, &first_loaded);
    let second_result = check_snapshot(&second_snapshot, &second_loaded);
    assert!(first_result.is_ok() && second_result.is_ok());
    let (Ok(first_result), Ok(second_result)) = (first_result, second_result) else {
        return;
    };
    assert_eq!(first_result.graph_digest, second_result.graph_digest);
    assert_eq!(first_result.diagnostics, second_result.diagnostics);
    assert_eq!(
        lint_saved_snapshot(&first_snapshot, &first_loaded).ok(),
        lsp_saved_snapshot(&first_snapshot, &first_loaded).ok()
    );

    let first_requirement_subjects =
        requirement_subject_facts(&first_result).expect("first requirement subjects");
    let second_requirement_subjects =
        requirement_subject_facts(&second_result).expect("second requirement subjects");
    let first_report = MachineReport {
        command: String::from("static"),
        graph: first_result.graph_digest.canonical_text(),
        diagnostics: first_result.diagnostics,
        migration: first_result.migration,
        verdicts: BTreeMap::new(),
        impacts: Vec::new(),
        mounts: Vec::new(),
        requirement_subjects: first_requirement_subjects,
        opaque_subjects: Vec::new(),
        artifact_attachments: Vec::new(),
    };
    let second_report = MachineReport {
        command: String::from("static"),
        graph: second_result.graph_digest.canonical_text(),
        diagnostics: second_result.diagnostics,
        migration: second_result.migration,
        verdicts: BTreeMap::new(),
        impacts: Vec::new(),
        mounts: Vec::new(),
        requirement_subjects: second_requirement_subjects,
        opaque_subjects: Vec::new(),
        artifact_attachments: Vec::new(),
    };
    assert_eq!(
        render_machine(&first_report).ok(),
        render_machine(&second_report).ok()
    );

    let (Some(first_root), Some(first_config), Some(second_root), Some(second_config)) = (
        root_text(first.path()),
        root_text(&first_config),
        root_text(second.path()),
        root_text(&second_config),
    ) else {
        return;
    };
    let first_cli = Command::new(binary())
        .args([
            "static",
            "--root",
            first_root,
            "--config",
            first_config,
            "--format",
            "machine",
        ])
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .output();
    let second_cli = Command::new(binary())
        .args([
            "static",
            "--root",
            second_root,
            "--config",
            second_config,
            "--format",
            "machine",
        ])
        .env("LANG", "tr_TR.UTF-8")
        .env("LC_ALL", "tr_TR.UTF-8")
        .env("TZ", "Pacific/Kiritimati")
        .output();
    assert!(first_cli.is_ok() && second_cli.is_ok());
    let (Ok(first_cli), Ok(second_cli)) = (first_cli, second_cli) else {
        return;
    };
    assert!(first_cli.status.success() && second_cli.status.success());
    assert_eq!(first_cli.stdout, second_cli.stdout);
    assert_eq!(first_cli.stderr, second_cli.stderr);
}

#[test]
fn mixed_adapter_output_is_independent_of_snapshot_order() {
    let parsed = RepositoryConfig::parse(
        config("")
            .replace(
                "authority_expectation = \"required\"",
                "authority_expectation = \"optional\"",
            )
            .as_bytes(),
    )
    .expect("mixed-adapter configuration");
    let markdown = SnapshotFile {
        path: RepositoryPath::new("spec/authority.md").expect("Markdown path"),
        bytes: include_bytes!("../fixtures/adapters/markdown/positive.md").to_vec(),
        adapter: String::from("markdown/v1"),
        authority_expectation: AuthorityExpectation::Optional,
        selector_root: String::from("spec"),
    };
    let rust = SnapshotFile {
        path: RepositoryPath::new("spec/attachment.rs").expect("Rust path"),
        bytes: include_bytes!("../fixtures/adapters/rust/positive.rs").to_vec(),
        adapter: String::from("rust/v1"),
        authority_expectation: AuthorityExpectation::Optional,
        selector_root: String::from("spec"),
    };
    let first = check_snapshot(
        &RepositorySnapshot {
            files: vec![markdown.clone(), rust.clone()],
        },
        &parsed,
    )
    .expect("first mixed-adapter order");
    let second = check_snapshot(
        &RepositorySnapshot {
            files: vec![rust, markdown],
        },
        &parsed,
    )
    .expect("second mixed-adapter order");
    assert_eq!(first.graph_digest, second.graph_digest);
    assert_eq!(first.diagnostics, second.diagnostics);
    let report = |result: buoy::check::CheckResult| {
        let requirement_subjects =
            requirement_subject_facts(&result).expect("mixed-adapter requirement subjects");
        MachineReport {
            command: String::from("static"),
            graph: result.graph_digest.canonical_text(),
            diagnostics: result.diagnostics,
            migration: result.migration,
            verdicts: BTreeMap::new(),
            impacts: Vec::new(),
            mounts: Vec::new(),
            requirement_subjects,
            opaque_subjects: Vec::new(),
            artifact_attachments: Vec::new(),
        }
    };
    assert_eq!(
        render_machine(&report(first)).ok(),
        render_machine(&report(second)).ok()
    );
}

#[test]
fn shared_check_covers_all_adapter_projections_and_presentations() {
    let parsed = RepositoryConfig::parse(config("").as_bytes()).expect("check config");
    let optional = RepositoryConfig::parse(
        config("")
            .replace(
                "authority_expectation = \"required\"",
                "authority_expectation = \"optional\"",
            )
            .as_bytes(),
    )
    .expect("optional check config");
    assert!(check_snapshot(&RepositorySnapshot { files: Vec::new() }, &optional).is_ok());
    #[cfg(feature = "coverage-controls")]
    {
        assert_eq!(
            migration_kind_control(buoy::core::ObjectKind::Atom),
            Default::default()
        );
        let projected = project_diagnostic_control(buoy::core::Diagnostic {
            path: Some(RepositoryPath::new("spec/control.md").expect("diagnostic path")),
            start: 1,
            end: 2,
            code: String::from("CONTROL"),
            kind: None,
            id: None,
            fields: vec![(String::from("key"), String::from("value"))],
        });
        assert_eq!(projected.path.as_deref(), Some("spec/control.md"));
        assert_eq!(projected.context, "key=value");
    }
    let file = |path: &str, bytes: &[u8], adapter: &str, expectation| SnapshotFile {
        path: RepositoryPath::new(path).expect("snapshot path"),
        bytes: bytes.to_vec(),
        adapter: adapter.to_owned(),
        authority_expectation: expectation,
        selector_root: String::from("spec"),
    };
    let noncanonical_carrier =
        String::from_utf8_lossy(include_bytes!("../fixtures/adapters/rust/positive.rs"))
            .replace("CAR-ADAPTER-001", "CAR-e\u{301}");
    let noncanonical_result = check_snapshot(
        &RepositorySnapshot {
            files: vec![file(
                "spec/noncanonical.rs",
                noncanonical_carrier.as_bytes(),
                "rust/v1",
                AuthorityExpectation::Optional,
            )],
        },
        &optional,
    )
    .expect("invalid identifiers become stable adapter diagnostics");
    assert_eq!(
        noncanonical_result.diagnostics[0].code,
        "SCF-CONSTRUCT-STABLE-ID"
    );

    let unique_links =
        String::from_utf8_lossy(include_bytes!("../fixtures/adapters/rust/links.rs"))
            .replace("-ADAPTER-001", "-INTEGRATION-001");
    let mut files = vec![
        file(
            "spec/atom.md",
            include_bytes!("../fixtures/adapters/markdown/positive.md"),
            "markdown/v1",
            AuthorityExpectation::Required,
        ),
        file(
            "spec/objects.md",
            include_bytes!("../fixtures/adapters/markdown/objects.md"),
            "markdown/v1",
            AuthorityExpectation::Optional,
        ),
        file(
            "spec/normative.md",
            b"This text MUST become an atom later.",
            "markdown/v1",
            AuthorityExpectation::Optional,
        ),
        file(
            "spec/invalid.md",
            &[0xff],
            "markdown/v1",
            AuthorityExpectation::Optional,
        ),
        file(
            "spec/carrier.rs",
            include_bytes!("../fixtures/adapters/rust/positive.rs"),
            "rust/v1",
            AuthorityExpectation::Optional,
        ),
        file(
            "spec/links.rs",
            unique_links.as_bytes(),
            "rust/v1",
            AuthorityExpectation::Optional,
        ),
        file(
            "spec/invalid.rs",
            &[0xff],
            "rust/v1",
            AuthorityExpectation::Optional,
        ),
    ];
    let objects_copy =
        String::from_utf8_lossy(include_bytes!("../fixtures/adapters/markdown/objects.md"))
            .replace("-ADAPTER-001", "-ADAPTER-COPY");
    files.push(file(
        "spec/objects-copy.md",
        objects_copy.as_bytes(),
        "markdown/v1",
        AuthorityExpectation::Forbidden,
    ));
    let checked = check_snapshot(&RepositorySnapshot { files }, &parsed).expect("shared check");
    assert_eq!(checked.migration.surface_declarations, 3);
    assert_eq!(checked.migration.policy_declarations, 2);
    assert_eq!(checked.migration.debt_declarations, 2);
    assert_eq!(checked.migration.waiver_declarations, 2);
    assert_eq!(checked.migration.derivation_declarations, 3);
    assert_eq!(checked.migration.formal_property_declarations, 2);
    let copy_check = check_snapshot(
        &RepositorySnapshot {
            files: vec![
                file(
                    "spec/copy-a.md",
                    b"nonnormative copy",
                    "markdown/v1",
                    AuthorityExpectation::Optional,
                ),
                file(
                    "spec/copy-b.md",
                    b"nonnormative copy",
                    "markdown/v1",
                    AuthorityExpectation::Optional,
                ),
            ],
        },
        &parsed,
    )
    .expect("copy check");
    assert_eq!(copy_check.migration.possible_untracked_copies, 1);
    assert!(!checked.migration.normative_looking_files.is_empty());
    assert!(!checked.migration.carrier_counts.is_empty());
    assert!(!checked.diagnostics.is_empty());

    let unsupported = RepositorySnapshot {
        files: vec![file(
            "spec/unsupported",
            b"bytes",
            "unsupported/v1",
            AuthorityExpectation::Optional,
        )],
    };
    assert_eq!(
        check_snapshot(&unsupported, &parsed),
        Err(CheckError::UnsupportedAdapter)
    );
    let report = MachineReport {
        command: String::from("control"),
        graph: checked.graph_digest.canonical_text(),
        diagnostics: checked.diagnostics,
        migration: checked.migration,
        verdicts: BTreeMap::from([(String::from("oracle"), String::from("pass"))]),
        impacts: vec![String::from("atom:CONTROL")],
        mounts: Vec::new(),
        requirement_subjects: Vec::new(),
        opaque_subjects: Vec::new(),
        artifact_attachments: Vec::new(),
    };
    let human = render_human(&report);
    assert!(human.contains("oracle oracle: pass"));
    assert!(human.contains("impact: atom:CONTROL"));
    for (verdict, expected) in [
        (Verdict::NotRun, "not-run"),
        (Verdict::Pass, "pass"),
        (Verdict::Fail, "fail"),
        (Verdict::Error, "error"),
        (Verdict::Timeout, "timeout"),
    ] {
        assert_eq!(verdict_name(verdict), expected);
    }
}

#[test]
fn snapshot_rejects_links_and_required_empty_roots() {
    let temporary = TempDir::new().ok();
    assert!(temporary.is_some());
    let Some(temporary) = temporary else { return };
    let spec = temporary.path().join("spec");
    assert!(fs::create_dir_all(&spec).is_ok());
    let config_path = temporary.path().join("config.toml");
    assert!(fs::write(&config_path, config("")).is_ok());
    let loaded = load_config(&config_path);
    assert!(loaded.is_ok());
    let Ok(loaded) = loaded else { return };
    let snapshot = snapshot_repository(temporary.path(), &loaded);
    assert!(snapshot.is_ok());
    let Ok(snapshot) = snapshot else { return };
    assert!(snapshot.artifact_index().is_ok());
    let checked = check_snapshot(&snapshot, &loaded);
    assert!(checked.is_ok());
    let Ok(checked) = checked else { return };
    assert!(
        checked
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPG-ADAPTER-EXPECTED-AUTHORITY-EMPTY")
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = temporary.path().join("outside.md");
        assert!(fs::write(&outside, ATOM_BASE).is_ok());
        assert!(symlink(&outside, spec.join("linked.md")).is_ok());
        assert_eq!(
            snapshot_repository(temporary.path(), &loaded).err(),
            Some(RepositoryError::LinkEntry)
        );
    }
}

#[test]
fn snapshot_rejects_unsupported_adapters_submodules_and_portable_collisions() {
    let missing = TempDir::new().expect("missing snapshot root");
    let parsed = RepositoryConfig::parse(config("").as_bytes()).expect("valid config");
    assert_eq!(
        snapshot_repository(&missing.path().join("absent"), &parsed),
        Err(RepositoryError::SelectorRoot)
    );
    assert_eq!(
        snapshot_repository(missing.path(), &parsed),
        Err(RepositoryError::SelectorRoot)
    );

    #[cfg(all(feature = "coverage-controls", unix))]
    {
        let faults = TempDir::new().expect("snapshot fault root");
        fs::create_dir_all(faults.path().join("spec")).expect("fault spec directory");
        fs::write(faults.path().join("spec/input.md"), ATOM_BASE).expect("fault input");
        assert_eq!(
            snapshot_repository_fault_control(faults.path(), &parsed, "pass")
                .expect("nonmatching fault control must preserve the snapshot")
                .files
                .len(),
            1
        );
        for (fault, expected) in [
            ("selector-path", RepositoryError::PathPolicy),
            ("selector-depth", RepositoryError::PathDepthBudget),
            ("relative-root", RepositoryError::SelectorRoot),
            ("relative-component", RepositoryError::SelectorRoot),
            ("directory-read", RepositoryError::ReadFailure),
            ("directory-entry", RepositoryError::ReadFailure),
            ("entry-path", RepositoryError::PathPolicy),
            ("child-path", RepositoryError::PathPolicy),
            ("stat", RepositoryError::ReadFailure),
            ("file-type", RepositoryError::LinkEntry),
            ("observed-size", RepositoryError::FileBytesBudget),
            ("include-pattern", RepositoryError::GlobSyntax),
            ("exclude-pattern", RepositoryError::GlobSyntax),
            ("file-read", RepositoryError::ReadFailure),
            ("actual-total", RepositoryError::TotalBytesBudget),
            ("expectation", RepositoryError::ConfigSchema),
            ("selected-paths", RepositoryError::SelectionCollision),
        ] {
            assert_eq!(
                snapshot_repository_fault_control(faults.path(), &parsed, fault),
                Err(expected),
                "fault {fault}"
            );
        }
    }

    let selection = TempDir::new().expect("selector branch root");
    fs::create_dir_all(selection.path().join("spec/nested")).expect("nested selector root");
    fs::write(selection.path().join("spec/input.md"), ATOM_BASE).expect("selector input");
    let shallow = RepositoryConfig::parse(
        config("")
            .replace("root = \"spec\"", "root = \"spec/nested\"")
            .replace(
                "authority_expectation = \"required\"",
                "authority_expectation = \"required\"\n[budgets]\npath_depth = 1\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912",
            )
            .as_bytes(),
    )
    .expect("shallow selector config");
    assert_eq!(
        snapshot_repository(selection.path(), &shallow),
        Err(RepositoryError::PathDepthBudget)
    );
    for source in [
        config("").replace("**/*.md", "**/*.rs"),
        config("").replace("exclude = []", "exclude = [\"**/*.md\"]"),
    ] {
        assert!(
            snapshot_repository(
                selection.path(),
                &RepositoryConfig::parse(source.as_bytes()).expect("filter config"),
            )
            .expect("filtered snapshot")
            .files
            .is_empty()
        );
    }
    let duplicate_selector = format!(
        "{}\n[[selectors]]\nroot = \"spec\"\nadapter = \"markdown/v1\"\ninclude = [\"**/*.md\"]\nexclude = []\nauthority_expectation = \"optional\"\n",
        config("")
    );
    assert_eq!(
        snapshot_repository(
            selection.path(),
            &RepositoryConfig::parse(duplicate_selector.as_bytes()).expect("duplicate selector"),
        ),
        Err(RepositoryError::SelectionCollision)
    );

    let unsupported = TempDir::new().expect("unsupported adapter root");
    fs::create_dir_all(unsupported.path().join("spec")).expect("spec directory");
    let unsupported_config = RepositoryConfig::parse(
        config("")
            .replace("markdown/v1", "unsupported/v1")
            .as_bytes(),
    )
    .expect("unsupported adapter remains syntactically valid");
    assert_eq!(
        snapshot_repository(unsupported.path(), &unsupported_config),
        Err(RepositoryError::UnsupportedAdapter)
    );

    let submodule = TempDir::new().expect("submodule root");
    fs::create_dir_all(submodule.path().join("spec/.git")).expect("nested repository marker");
    assert_eq!(
        snapshot_repository(submodule.path(), &parsed),
        Err(RepositoryError::SubmoduleEntry)
    );

    #[cfg(feature = "coverage-controls")]
    {
        assert!(
            validate_selected_paths_control(vec![
                RepositoryPath::new("spec/unique.md").expect("unique path"),
            ])
            .is_ok()
        );
        assert_eq!(
            validate_selected_paths_control(vec![
                RepositoryPath::new("spec/Straße.md").expect("sharp-s path"),
                RepositoryPath::new("spec/STRASSE.md").expect("folded path"),
            ]),
            Err(RepositoryError::SelectionCollision)
        );
    }
}

#[cfg(all(feature = "coverage-controls", unix))]
struct PartialReader {
    bytes: Vec<u8>,
    position: usize,
    interrupted: bool,
}

#[cfg(all(feature = "coverage-controls", unix))]
impl Read for PartialReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::from(ErrorKind::Interrupted));
        }
        if self.position == self.bytes.len() || output.is_empty() {
            return Ok(0);
        }
        output[0] = self.bytes[self.position];
        self.position += 1;
        Ok(1)
    }
}

#[cfg(all(feature = "coverage-controls", unix))]
struct FailingPartialReader(bool);

#[cfg(all(feature = "coverage-controls", unix))]
impl Read for FailingPartialReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if self.0 {
            return Err(Error::from(ErrorKind::PermissionDenied));
        }
        self.0 = true;
        output[0] = b'x';
        Ok(1)
    }
}

#[cfg(all(feature = "coverage-controls", unix))]
const REPOSITORY_FIXTURE_MANIFEST: &str = include_str!("../fixtures/repository/v1/manifest.tsv");
#[cfg(all(feature = "coverage-controls", unix))]
const REPOSITORY_FIXTURE_PATH: &str = "fixtures/repository/v1/manifest.tsv";
#[cfg(all(feature = "coverage-controls", unix))]
const REPOSITORY_EXECUTIONS: &str = "assurance/v1/repository-fixture-executions.tsv";
#[cfg(all(feature = "coverage-controls", unix))]
const REPOSITORY_EXECUTION_TOOLS: [&str; 5] = [
    "tests/integration.rs",
    "src/repository.rs",
    "src/glob.rs",
    "tools/execution_evidence.rs",
    "tools/digest_binding.rs",
];
#[cfg(all(feature = "coverage-controls", unix))]
const REPOSITORY_EXECUTION_CONFIGURATION: [&str; 2] =
    [REPOSITORY_FIXTURE_PATH, "unicode/16.0.0/CaseFolding.txt"];

#[cfg(all(feature = "coverage-controls", unix))]
#[derive(Clone, Copy, Debug)]
struct RepositoryFixture<'a> {
    source: &'a str,
    id: &'a str,
    category: &'a str,
    platform: &'a str,
    operation: &'a str,
    input: &'a str,
    auxiliary: &'a str,
    expected: &'a str,
}

#[cfg(all(feature = "coverage-controls", unix))]
fn repository_fixtures() -> Result<Vec<RepositoryFixture<'static>>, String> {
    let mut lines = REPOSITORY_FIXTURE_MANIFEST.lines();
    let header = lines.next().ok_or_else(|| String::from("missing header"))?;
    if header != "schema\tcase_id\tcategory\tplatform\toperation\tinput\tauxiliary\texpected" {
        return Err(String::from("repository fixture header"));
    }
    lines
        .enumerate()
        .map(|(index, line)| {
            let columns = line.split('\t').collect::<Vec<_>>();
            if columns.len() != 8 || columns[0] != "buoy-repository-fixture/v1" {
                return Err(format!("repository fixture row {}", index + 2));
            }
            Ok(RepositoryFixture {
                source: line,
                id: columns[1],
                category: columns[2],
                platform: columns[3],
                operation: columns[4],
                input: columns[5],
                auxiliary: columns[6],
                expected: columns[7],
            })
        })
        .collect()
}

#[cfg(all(feature = "coverage-controls", unix))]
fn bounded_snapshot_config(field: &str, value: u64) -> Result<RepositoryConfig, String> {
    let mut values = BTreeMap::from([
        ("path_depth", 64_u64),
        ("directory_entries", 100_000),
        ("selected_files", 10_000),
        ("file_bytes", 16_777_216),
        ("total_bytes", 536_870_912),
    ]);
    if values.insert(field, value).is_none() {
        return Err(format!("unknown snapshot budget `{field}`"));
    }
    let budgets = values
        .into_iter()
        .map(|(name, limit)| format!("{name} = {limit}"))
        .collect::<Vec<_>>()
        .join("\n");
    RepositoryConfig::parse(
        config("")
            .replace(
                "authority_expectation = \"required\"",
                &format!("authority_expectation = \"optional\"\n[budgets]\n{budgets}"),
            )
            .as_bytes(),
    )
    .map_err(|error| format!("bounded snapshot config: {}", error.code()))
}

#[cfg(all(feature = "coverage-controls", unix))]
fn bounded_graph_config(field: &str, value: u64) -> Result<RepositoryConfig, String> {
    let mut values = BTreeMap::from([
        ("graph_objects", 200_000_u64),
        ("graph_references", 2_000_000),
        ("diagnostics", 100_000),
    ]);
    if values.insert(field, value).is_none() {
        return Err(format!("unknown graph budget `{field}`"));
    }
    RepositoryConfig::parse(
        config("")
            .replace(
                "authority_expectation = \"required\"",
                &format!(
                    "authority_expectation = \"optional\"\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = {}\ngraph_references = {}\ndiagnostics = {}",
                    values["graph_objects"], values["graph_references"], values["diagnostics"]
                ),
            )
            .as_bytes(),
    )
    .map_err(|error| format!("bounded graph config: {}", error.code()))
}

#[cfg(all(feature = "coverage-controls", unix))]
fn execute_repository_fixture(fixture: RepositoryFixture<'_>) -> Result<String, String> {
    match fixture.operation {
        "glob-match" => {
            let pattern = PortableGlob::parse(fixture.input)
                .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            let path = RepositoryPath::new(fixture.auxiliary)
                .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            assert_eq!(
                pattern.matches(&path),
                fixture.expected == "true",
                "{}",
                fixture.id
            );
        }
        "glob-parse" => {
            let error = PortableGlob::parse(fixture.input)
                .expect_err("negative portable glob fixture must fail");
            assert_eq!(error.code(), fixture.expected, "{}", fixture.id);
        }
        "glob-overlap" => {
            let left = PortableGlob::parse(fixture.input)
                .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            let right = PortableGlob::parse(fixture.auxiliary)
                .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            let expected = fixture.expected == "true";
            assert_eq!(left.overlaps(&right), expected, "{}", fixture.id);
            assert_eq!(right.overlaps(&left), expected, "{} symmetry", fixture.id);
        }
        "selector-overlap" => {
            let temporary = TempDir::new().map_err(|error| error.to_string())?;
            fs::create_dir_all(temporary.path().join("spec")).map_err(|error| error.to_string())?;
            fs::write(temporary.path().join("spec/input.md"), ATOM_BASE)
                .map_err(|error| error.to_string())?;
            let source = format!(
                "schema = \"spec-provenance-config/v2\"\n\
                 digest_algorithms = [\"xxh3-128\"]\n\
                 [[selectors]]\nroot = \"spec\"\nadapter = \"markdown/v1\"\ninclude = [\"{}\"]\nexclude = []\nauthority_expectation = \"optional\"\n\
                 [[selectors]]\nroot = \"spec\"\nadapter = \"markdown/v1\"\ninclude = [\"{}\"]\nexclude = []\nauthority_expectation = \"optional\"\n",
                fixture.input, fixture.auxiliary
            );
            let parsed = RepositoryConfig::parse(source.as_bytes())
                .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            let error = snapshot_repository(temporary.path(), &parsed)
                .expect_err("overlapping selectors must fail");
            assert_eq!(error.code(), fixture.expected, "{}", fixture.id);
        }
        "unicode-corpus" => {
            let source = fs::read_to_string(fixture.input).map_err(|error| error.to_string())?;
            let statuses = fixture.auxiliary.split(',').collect::<BTreeSet<_>>();
            let mut checked = 0_usize;
            for raw in source.lines() {
                let content = raw.split('#').next().unwrap_or_default().trim();
                if content.is_empty() {
                    continue;
                }
                let columns = content.split(';').map(str::trim).collect::<Vec<_>>();
                if columns.len() != 4 {
                    return Err(format!("{}: malformed Unicode row", fixture.id));
                }
                if !statuses.contains(columns[1]) {
                    continue;
                }
                let source_character = u32::from_str_radix(columns[0], 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| format!("{}: invalid Unicode scalar", fixture.id))?;
                let expected = columns[2]
                    .split_ascii_whitespace()
                    .map(|value| {
                        u32::from_str_radix(value, 16)
                            .ok()
                            .and_then(char::from_u32)
                            .ok_or_else(|| format!("{}: invalid fold scalar", fixture.id))
                    })
                    .collect::<Result<String, _>>()?
                    .nfc()
                    .collect::<String>();
                assert_eq!(
                    portable_case_fold(&source_character.to_string()),
                    expected,
                    "{} U+{}",
                    fixture.id,
                    columns[0]
                );
                checked += 1;
            }
            assert_eq!(
                checked,
                fixture
                    .expected
                    .parse::<usize>()
                    .map_err(|error| error.to_string())?,
                "{}",
                fixture.id
            );
        }
        "unicode-collision" => {
            let left = RepositoryPath::new(fixture.input)
                .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            let right = RepositoryPath::new(fixture.auxiliary)
                .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            assert_eq!(
                left.portable_collision_key(),
                right.portable_collision_key()
            );
            assert!(
                canonical_paths(vec![left, right]).is_err(),
                "{}",
                fixture.id
            );
            assert_eq!(fixture.expected, "collision");
        }
        "reserved-path" => {
            let error = RepositoryPath::new(fixture.input)
                .expect_err("Windows reserved-name fixture must fail");
            assert_eq!(error.code(), fixture.expected, "{}", fixture.id);
        }
        "windows-reparse-attributes" => {
            let attributes = u32::from_str_radix(
                fixture.input.strip_prefix("0x").unwrap_or(fixture.input),
                16,
            )
            .map_err(|error| error.to_string())?;
            let result = windows_reparse_attributes_control(attributes);
            if fixture.expected == "pass" {
                assert_eq!(result, Ok(()), "{}", fixture.id);
            } else {
                assert_eq!(
                    result.expect_err("reparse attribute must fail").code(),
                    fixture.expected,
                    "{}",
                    fixture.id
                );
            }
        }
        "windows-platform-blocker" => {
            let source = fs::read_to_string(fixture.input).map_err(|error| error.to_string())?;
            let document = toml::from_str::<toml::Value>(&source)
                .map_err(|error| format!("{}: {error}", fixture.id))?;
            let blocker = document
                .get("blocker")
                .and_then(toml::Value::as_array)
                .and_then(|rows| {
                    rows.iter().find(|row| {
                        row.get("id").and_then(toml::Value::as_str) == Some(fixture.auxiliary)
                    })
                })
                .ok_or_else(|| format!("{}: missing blocker", fixture.id))?;
            assert_eq!(
                blocker
                    .get("current_behavior")
                    .and_then(toml::Value::as_str),
                Some(fixture.expected),
                "{}",
                fixture.id
            );
            assert!(
                blocker
                    .get("required_discharge")
                    .and_then(toml::Value::as_array)
                    .is_some_and(|items| items.iter().any(|item| {
                        item.as_str().is_some_and(|text| {
                            text.contains("reparse-race") && text.contains("on Windows")
                        })
                    })),
                "{} must retain the exact Windows execution prerequisite",
                fixture.id
            );
        }
        "unix-link-swap" => {
            let temporary = TempDir::new().map_err(|error| error.to_string())?;
            let root = temporary.path().join("repository");
            let original = temporary.path().join("opened-repository");
            let attacker = temporary.path().join("attacker");
            fs::create_dir_all(root.join("spec")).map_err(|error| error.to_string())?;
            fs::create_dir_all(attacker.join("spec")).map_err(|error| error.to_string())?;
            fs::write(root.join("spec/input.md"), fixture.input)
                .map_err(|error| error.to_string())?;
            fs::write(attacker.join("spec/input.md"), fixture.auxiliary)
                .map_err(|error| error.to_string())?;
            let parsed =
                RepositoryConfig::parse(config("").replace("required", "optional").as_bytes())
                    .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            let snapshot = snapshot_repository_after_root_open_control(&root, &parsed, || {
                fs::rename(&root, &original).expect("preserve opened fixture root");
                symlink(&attacker, &root).expect("replace fixture root with link");
            })
            .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            assert_eq!(snapshot.files.len(), 1, "{}", fixture.id);
            assert_eq!(snapshot.files[0].bytes, fixture.expected.as_bytes());
        }
        "resource-budget" | "deep-tree" => {
            let temporary = TempDir::new().map_err(|error| error.to_string())?;
            let spec = temporary.path().join("spec");
            let (field, value) = if fixture.operation == "deep-tree" {
                let depth = fixture
                    .input
                    .parse::<usize>()
                    .map_err(|error| error.to_string())?;
                let mut nested = spec.clone();
                for index in 0..depth {
                    nested.push(format!("d{index}"));
                }
                fs::create_dir_all(&nested).map_err(|error| error.to_string())?;
                fs::write(nested.join("deep.md"), ATOM_BASE).map_err(|error| error.to_string())?;
                (
                    "path_depth",
                    fixture
                        .auxiliary
                        .parse::<u64>()
                        .map_err(|error| error.to_string())?,
                )
            } else {
                let nested = spec.join("nested/deep");
                fs::create_dir_all(&nested).map_err(|error| error.to_string())?;
                fs::write(spec.join("a.md"), ATOM_BASE).map_err(|error| error.to_string())?;
                fs::write(spec.join("b.md"), ATOM_BASE).map_err(|error| error.to_string())?;
                fs::write(nested.join("c.md"), ATOM_BASE).map_err(|error| error.to_string())?;
                (
                    fixture.input,
                    fixture
                        .auxiliary
                        .parse::<u64>()
                        .map_err(|error| error.to_string())?,
                )
            };
            let parsed = bounded_snapshot_config(field, value)?;
            let error = snapshot_repository(temporary.path(), &parsed)
                .expect_err("bounded repository fixture must fail");
            assert_eq!(error.code(), fixture.expected, "{}", fixture.id);
        }
        "configuration-budget" => {
            let size = fixture
                .input
                .parse::<usize>()
                .map_err(|error| error.to_string())?;
            let error = RepositoryConfig::parse(&vec![b' '; size])
                .expect_err("oversized configuration fixture must fail");
            assert_eq!(error.code(), fixture.expected, "{}", fixture.id);
        }
        "graph-budget" => {
            let value = fixture
                .auxiliary
                .parse::<u64>()
                .map_err(|error| error.to_string())?;
            let config = bounded_graph_config(fixture.input, value)?;
            let file = |path: &str, bytes: &[u8], adapter: &str| SnapshotFile {
                path: RepositoryPath::new(path).expect("graph budget fixture path"),
                bytes: bytes.to_vec(),
                adapter: adapter.to_owned(),
                authority_expectation: AuthorityExpectation::Optional,
                selector_root: String::from("spec"),
            };
            let snapshot = match fixture.input {
                "graph_objects" => RepositorySnapshot {
                    files: vec![
                        file(
                            "spec/first.md",
                            include_bytes!("../fixtures/adapters/markdown/positive.md"),
                            "markdown/v1",
                        ),
                        file(
                            "spec/second.md",
                            include_bytes!("../fixtures/adapters/markdown/positive.md"),
                            "markdown/v1",
                        ),
                    ],
                },
                "graph_references" => RepositorySnapshot {
                    files: vec![
                        file(
                            "spec/carrier-a.rs",
                            include_bytes!("../fixtures/adapters/rust/positive.rs"),
                            "rust/v1",
                        ),
                        file(
                            "spec/carrier-b.rs",
                            include_bytes!("../fixtures/adapters/rust/positive.rs"),
                            "rust/v1",
                        ),
                    ],
                },
                "diagnostics" => RepositorySnapshot {
                    files: vec![
                        file("spec/first.md", &[0xff], "markdown/v1"),
                        file("spec/second.md", &[0xff], "markdown/v1"),
                    ],
                },
                field => return Err(format!("{}: unknown graph budget `{field}`", fixture.id)),
            };
            let error =
                check_snapshot(&snapshot, &config).expect_err("bounded graph fixture must fail");
            assert_eq!(error.code(), fixture.expected, "{}", fixture.id);
        }
        "partial-read" => {
            let mut reader = PartialReader {
                bytes: fixture.input.as_bytes().to_vec(),
                position: 0,
                interrupted: false,
            };
            let maximum = fixture
                .auxiliary
                .parse::<u64>()
                .map_err(|error| error.to_string())?;
            let bytes = read_bounded_control(&mut reader, maximum, 0)
                .map_err(|error| format!("{}: {}", fixture.id, error.code()))?;
            assert_eq!(
                format!("bytes:{}", String::from_utf8_lossy(&bytes)),
                fixture.expected,
                "{}",
                fixture.id
            );
        }
        "partial-overflow" => {
            let maximum = fixture
                .auxiliary
                .parse::<u64>()
                .map_err(|error| error.to_string())?;
            let error = read_bounded_control(
                &mut std::io::Cursor::new(fixture.input.as_bytes()),
                maximum,
                0,
            )
            .expect_err("overflow fixture must fail");
            assert_eq!(error.code(), fixture.expected, "{}", fixture.id);
        }
        "partial-failure" => {
            let maximum = fixture
                .auxiliary
                .parse::<u64>()
                .map_err(|error| error.to_string())?;
            let error = read_bounded_control(&mut FailingPartialReader(false), maximum, 0)
                .expect_err("partial failure fixture must fail");
            assert_eq!(error.code(), fixture.expected, "{}", fixture.id);
        }
        operation => return Err(format!("{}: unknown operation `{operation}`", fixture.id)),
    }
    Ok(format!(
        "operation={};platform={};observed={}",
        fixture.operation, fixture.platform, fixture.expected
    ))
}

#[cfg(all(feature = "coverage-controls", unix))]
fn repository_execution_record(
    fixture: RepositoryFixture<'_>,
    result: String,
) -> Result<execution_evidence::Record, String> {
    execution_evidence::passing(
        "fixture-operation",
        format!(
            "{REPOSITORY_FIXTURE_PATH}#{}#{}",
            fixture.id, fixture.operation
        ),
        REPOSITORY_FIXTURE_PATH,
        fixture.source.as_bytes(),
        &REPOSITORY_EXECUTION_TOOLS,
        &REPOSITORY_EXECUTION_CONFIGURATION,
        result,
    )
}

#[cfg(all(feature = "coverage-controls", unix))]
#[test]
fn frozen_repository_fixture_manifest_executes_every_task_2_12_category() {
    let fixtures = repository_fixtures().expect("valid repository fixture manifest");
    assert_eq!(fixtures.len(), 36);
    let mut ids = BTreeSet::new();
    let mut categories = BTreeSet::new();
    let mut operations = BTreeSet::new();
    let mut records = Vec::new();
    for fixture in fixtures {
        assert!(ids.insert(fixture.id), "duplicate fixture {}", fixture.id);
        assert!(matches!(fixture.platform, "portable" | "unix"));
        categories.insert(fixture.category);
        operations.insert(fixture.operation);
        let result = execute_repository_fixture(fixture).expect("repository fixture must execute");
        records.push(
            repository_execution_record(fixture, result)
                .expect("repository fixture evidence must bind"),
        );
    }
    let expected =
        execution_evidence::render(&records).expect("repository fixture evidence must render");
    execution_evidence::validate_materialized(REPOSITORY_EXECUTIONS, &expected, false)
        .expect("repository fixture evidence must be current");
    assert_eq!(
        categories,
        BTreeSet::from([
            "deep-tree",
            "partial-io",
            "portable-glob",
            "resource-budget",
            "selector-overlap",
            "unicode-conformance",
            "unix-link-swap",
            "windows-reserved-reparse",
        ])
    );
    assert_eq!(
        operations,
        BTreeSet::from([
            "configuration-budget",
            "deep-tree",
            "glob-match",
            "glob-overlap",
            "glob-parse",
            "graph-budget",
            "partial-failure",
            "partial-overflow",
            "partial-read",
            "reserved-path",
            "resource-budget",
            "selector-overlap",
            "unicode-collision",
            "unicode-corpus",
            "unix-link-swap",
            "windows-platform-blocker",
            "windows-reparse-attributes",
        ])
    );
}

#[cfg(all(feature = "coverage-controls", unix))]
#[test]
#[ignore = "explicit reviewed per-case execution-evidence regeneration only"]
fn write_repository_fixture_execution_evidence() {
    let records = repository_fixtures()
        .expect("valid repository fixture manifest")
        .into_iter()
        .map(|fixture| {
            let result = execute_repository_fixture(fixture)?;
            repository_execution_record(fixture, result)
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("every repository fixture must execute before evidence is written");
    let expected =
        execution_evidence::render(&records).expect("repository fixture evidence must render");
    execution_evidence::validate_materialized(REPOSITORY_EXECUTIONS, &expected, true)
        .expect("repository fixture evidence must write and verify");
}

#[cfg(all(feature = "coverage-controls", unix))]
#[test]
fn bounded_reader_and_descriptor_root_controls_preserve_saved_bytes() {
    let mut partial = PartialReader {
        bytes: b"partial-input".to_vec(),
        position: 0,
        interrupted: false,
    };
    assert_eq!(
        read_bounded_control(&mut partial, 13, 0),
        Ok(b"partial-input".to_vec())
    );
    assert_eq!(
        read_bounded_control(&mut std::io::Cursor::new(b"12345"), 4, 0),
        Err(RepositoryError::FileBytesBudget)
    );
    assert_eq!(
        read_bounded_control(&mut FailingPartialReader(false), 4, 0),
        Err(RepositoryError::ReadFailure)
    );

    let temporary = TempDir::new().expect("descriptor control root");
    let root = temporary.path().join("repository");
    let original = temporary.path().join("opened-repository");
    let attacker = temporary.path().join("attacker");
    fs::create_dir_all(root.join("spec")).expect("original spec");
    fs::create_dir_all(attacker.join("spec")).expect("attacker spec");
    fs::write(root.join("spec/input.md"), b"original").expect("original bytes");
    fs::write(attacker.join("spec/input.md"), b"attacker").expect("attacker bytes");
    let parsed = RepositoryConfig::parse(config("").replace("required", "optional").as_bytes())
        .expect("descriptor config");
    let snapshot = snapshot_repository_after_root_open_control(&root, &parsed, || {
        fs::rename(&root, &original).expect("preserve opened root");
        symlink(&attacker, &root).expect("replace path with attacker link");
    })
    .expect("descriptor-relative snapshot");
    assert_eq!(snapshot.files[0].bytes, b"original");
}

#[test]
fn graph_reference_diagnostic_budgets_block_before_aggregate_retention() {
    for (error, code) in [
        (CheckError::Encoding, "DET-ENC-SNAPSHOT"),
        (CheckError::Configuration, "REPO-CONFIG-SCHEMA"),
        (CheckError::UnsupportedAdapter, "SPG-ADAPTER-UNSUPPORTED"),
        (CheckError::GraphObjectsBudget, "REH-BUDGET-GRAPH-OBJECTS"),
        (
            CheckError::GraphReferencesBudget,
            "REH-BUDGET-GRAPH-REFERENCES",
        ),
        (CheckError::DiagnosticsBudget, "REH-BUDGET-DIAGNOSTICS"),
    ] {
        assert_eq!(error.code(), code);
    }

    let bounded_config = |objects: u64, references: u64, diagnostics: u64| {
        RepositoryConfig::parse(
            config("")
                .replace(
                    "authority_expectation = \"required\"",
                    &format!(
                        "authority_expectation = \"optional\"\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = {objects}\ngraph_references = {references}\ndiagnostics = {diagnostics}"
                    ),
                )
                .as_bytes(),
        )
        .expect("bounded graph configuration")
    };
    let file = |path: &str, bytes: &[u8], adapter: &str| SnapshotFile {
        path: RepositoryPath::new(path).expect("budget fixture path"),
        bytes: bytes.to_vec(),
        adapter: adapter.to_owned(),
        authority_expectation: AuthorityExpectation::Optional,
        selector_root: String::from("spec"),
    };

    let configured_oracles = r#"
[[oracles]]
id = "ORC-BUDGET-A"
program = "builtin:pass"
timeout_ms = 1000

[[oracles]]
id = "ORC-BUDGET-B"
program = "builtin:pass"
timeout_ms = 1000
"#;
    let initial_object_config = RepositoryConfig::parse(
        format!(
            "{}\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = 1\ngraph_references = 100\ndiagnostics = 100\n",
            config(configured_oracles).replace(
                "authority_expectation = \"required\"",
                "authority_expectation = \"optional\""
            )
        )
        .as_bytes(),
    )
    .expect("configured-oracle graph budget");
    assert_eq!(
        check_snapshot(
            &RepositorySnapshot { files: Vec::new() },
            &initial_object_config
        ),
        Err(CheckError::GraphObjectsBudget)
    );

    let derivation_config = RepositoryConfig::parse(
        config(
            r#"
[[oracles]]
id = "ORC-DERIVATION-COVERAGE"
program = "builtin:pass"
timeout_ms = 1000
derivation = "DERIVATION-COVERAGE"
"#,
        )
        .replace(
            "authority_expectation = \"required\"",
            "authority_expectation = \"optional\"",
        )
        .as_bytes(),
    )
    .expect("derivation-oracle coverage configuration");
    assert!(
        check_snapshot(
            &RepositorySnapshot { files: Vec::new() },
            &derivation_config
        )
        .is_ok()
    );

    let atoms = RepositorySnapshot {
        files: vec![
            file(
                "spec/first.md",
                include_bytes!("../fixtures/adapters/markdown/positive.md"),
                "markdown/v1",
            ),
            file(
                "spec/second.md",
                include_bytes!("../fixtures/adapters/markdown/positive.md"),
                "markdown/v1",
            ),
        ],
    };
    let object_error = check_snapshot(&atoms, &bounded_config(1, 100, 100))
        .expect_err("two materialized atoms exceed one-object budget");
    assert_eq!(object_error, CheckError::GraphObjectsBudget);
    assert_eq!(object_error.code(), "REH-BUDGET-GRAPH-OBJECTS");
    assert_eq!(
        check_snapshot(&atoms, &bounded_config(100, 100, 100)),
        Err(CheckError::Encoding),
        "duplicate aggregate graph values cannot encode"
    );
    let carrier = RepositorySnapshot {
        files: vec![
            file(
                "spec/carrier-a.rs",
                include_bytes!("../fixtures/adapters/rust/positive.rs"),
                "rust/v1",
            ),
            file(
                "spec/carrier-b.rs",
                include_bytes!("../fixtures/adapters/rust/positive.rs"),
                "rust/v1",
            ),
        ],
    };
    let reference_error = check_snapshot(&carrier, &bounded_config(100, 1, 100))
        .expect_err("carrier atom and oracle bindings exceed one-reference budget");
    assert_eq!(reference_error, CheckError::GraphReferencesBudget);
    assert_eq!(reference_error.code(), "REH-BUDGET-GRAPH-REFERENCES");
    assert_eq!(
        check_snapshot(
            &RepositorySnapshot {
                files: vec![carrier.files[0].clone()],
            },
            &bounded_config(100, 100, 1),
        ),
        Err(CheckError::DiagnosticsBudget),
        "missing carrier references exceed the retained diagnostic budget"
    );

    let two_required_selectors = RepositoryConfig::parse(
        br#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[[selectors]]
root = "spec-a"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "required"

[[selectors]]
root = "spec-b"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "required"

[budgets]
path_depth = 64
directory_entries = 100000
selected_files = 10000
file_bytes = 16777216
total_bytes = 536870912
graph_objects = 100
graph_references = 100
diagnostics = 1
"#,
    )
    .expect("two-selector diagnostic budget");
    assert_eq!(
        check_snapshot(
            &RepositorySnapshot { files: Vec::new() },
            &two_required_selectors
        ),
        Err(CheckError::DiagnosticsBudget)
    );

    let malformed = RepositorySnapshot {
        files: vec![
            file("spec/first.md", &[0xff], "markdown/v1"),
            file("spec/second.md", &[0xff], "markdown/v1"),
        ],
    };
    let diagnostic_error = check_snapshot(&malformed, &bounded_config(100, 100, 1))
        .expect_err("two adapter diagnostics exceed one-diagnostic budget");
    assert_eq!(diagnostic_error, CheckError::DiagnosticsBudget);
    assert_eq!(diagnostic_error.code(), "REH-BUDGET-DIAGNOSTICS");
}

#[cfg(feature = "coverage-controls")]
#[test]
fn adapter_error_projection_covers_both_stable_check_classes() {
    assert_eq!(
        project_adapter_error_control(buoy::adapter_sdk::AdapterSdkError::UnsupportedAdapter),
        CheckError::UnsupportedAdapter
    );
    assert_eq!(
        project_adapter_error_control(buoy::adapter_sdk::AdapterSdkError::Identity),
        CheckError::Encoding
    );
}

#[test]
fn snapshot_budgets_fail_before_unbounded_retention() {
    let fixture = |budget_lines: &str| {
        format!(
            r#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[[selectors]]
root = "spec"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "optional"

[budgets]
{budget_lines}
"#
        )
    };
    let defaults_except = |field: &str, value: u64| {
        let mut values = BTreeMap::from([
            ("path_depth", 64_u64),
            ("directory_entries", 100_000),
            ("selected_files", 10_000),
            ("file_bytes", 16_777_216),
            ("total_bytes", 536_870_912),
        ]);
        values.insert(field, value);
        values
            .into_iter()
            .map(|(key, value)| format!("{key} = {value}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let cases = [
        ("file_bytes", 1, RepositoryError::FileBytesBudget),
        ("total_bytes", 1, RepositoryError::TotalBytesBudget),
        ("selected_files", 1, RepositoryError::SelectedFilesBudget),
        (
            "directory_entries",
            1,
            RepositoryError::DirectoryEntriesBudget,
        ),
        ("path_depth", 1, RepositoryError::PathDepthBudget),
    ];
    for (field, value, expected) in cases {
        let temporary = TempDir::new().ok();
        assert!(temporary.is_some());
        let Some(temporary) = temporary else { continue };
        let spec = temporary.path().join("spec");
        let nested = spec.join("nested").join("deep");
        assert!(fs::create_dir_all(&nested).is_ok());
        assert!(fs::write(spec.join("a.md"), ATOM_BASE).is_ok());
        assert!(fs::write(spec.join("b.md"), ATOM_BASE).is_ok());
        assert!(fs::write(nested.join("c.md"), ATOM_BASE).is_ok());
        let bytes = fixture(&defaults_except(field, value));
        let config = RepositoryConfig::parse(bytes.as_bytes());
        assert!(config.is_ok());
        let Ok(config) = config else { continue };
        assert_eq!(
            snapshot_repository(temporary.path(), &config),
            Err(expected),
            "budget {field}"
        );
    }

    let invalid = fixture(&defaults_except("path_depth", 257));
    assert_eq!(
        RepositoryConfig::parse(invalid.as_bytes()),
        Err(RepositoryError::PathDepthBudget)
    );
}

#[test]
fn static_cli_has_exact_success_usage_and_input_exit_classes() {
    let repository = repository(ATOM_BASE, "");
    assert!(repository.is_some());
    let Some((repository, config_path)) = repository else {
        return;
    };
    let (Some(root), Some(config)) = (root_text(repository.path()), root_text(&config_path)) else {
        return;
    };
    let output = run(&[
        "static", "--root", root, "--config", config, "--format", "machine",
    ]);
    assert!(output.is_some());
    let Some(output) = output else { return };
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).ok();
    assert!(stdout.is_some());
    assert!(stdout.is_some_and(|value| value.contains("\"command\":\"static\"")));

    let usage = run(&[]);
    assert!(usage.is_some());
    assert_eq!(usage.and_then(|output| output.status.code()), Some(64));
    let invalid = run(&["static", "--root", root, "--config", "missing-config.toml"]);
    assert!(invalid.is_some());
    assert_eq!(invalid.and_then(|output| output.status.code()), Some(70));
}

#[test]
#[allow(clippy::too_many_lines)]
fn change_cli_requires_complete_exact_dispositions() {
    let base = repository(ATOM_BASE, "");
    let head = repository(ATOM_HEAD, "");
    assert!(base.is_some() && head.is_some());
    let (Some((base, base_config)), Some((head, _))) = (base, head) else {
        return;
    };
    let config = load_config(&base_config);
    assert!(config.is_ok());
    let Ok(config) = config else { return };
    let base_snapshot = snapshot_repository(base.path(), &config);
    let head_snapshot = snapshot_repository(head.path(), &config);
    assert!(base_snapshot.is_ok() && head_snapshot.is_ok());
    let (Ok(base_snapshot), Ok(head_snapshot)) = (base_snapshot, head_snapshot) else {
        return;
    };
    let base_result = check_snapshot(&base_snapshot, &config);
    let head_result = check_snapshot(&head_snapshot, &config);
    assert!(base_result.is_ok() && head_result.is_ok());
    let (Ok(base_result), Ok(head_result)) = (base_result, head_result) else {
        return;
    };
    let base_atom = &base_result.graph.atoms[0];
    let head_atom = &head_result.graph.atoms[0];
    let missing_manifest = base.path().join("missing.toml");
    let missing = format!(
        r#"schema = "spec-provenance-change-manifest/v1"
id = "CHANGE-001"
base_graph = "{}"
head_graph = "{}"
dispositions = []
"#,
        base_result.graph_digest, head_result.graph_digest
    );
    assert!(fs::write(&missing_manifest, &missing).is_ok());
    let (Some(base_root), Some(head_root), Some(config_path), Some(missing_path)) = (
        root_text(base.path()),
        root_text(head.path()),
        root_text(&base_config),
        root_text(&missing_manifest),
    ) else {
        return;
    };
    let missing_output = run(&[
        "change",
        "--base",
        base_root,
        "--head",
        head_root,
        "--config",
        config_path,
        "--manifest",
        missing_path,
    ]);
    assert!(missing_output.is_some());
    assert_eq!(
        missing_output.and_then(|output| output.status.code()),
        Some(3)
    );

    let malformed_manifest = base.path().join("malformed.toml");
    let malformed = format!(
        "{missing}\n[approval]\nauthority = \"maintainers\"\nrecord = \"/absolute\"\ndigest = \"{}\"\n",
        base_result.graph_digest,
    );
    assert!(fs::write(&malformed_manifest, malformed).is_ok());
    let Some(malformed_path) = root_text(&malformed_manifest) else {
        return;
    };
    let malformed_output = run(&[
        "change",
        "--base",
        base_root,
        "--head",
        head_root,
        "--config",
        config_path,
        "--manifest",
        malformed_path,
    ]);
    assert!(malformed_output.is_some());
    assert_eq!(
        malformed_output.and_then(|output| output.status.code()),
        Some(70)
    );

    let complete_manifest = base.path().join("complete.toml");
    let complete = format!(
        r#"schema = "spec-provenance-change-manifest/v1"
id = "CHANGE-001"
base_graph = "{}"
head_graph = "{}"

[[dispositions]]
id = "DISP-001"
kind = "atom"
object = "SPG-INTEGRATION-001"
base = "{}"
head = "{}"
disposition = "revised"
"#,
        base_result.graph_digest,
        head_result.graph_digest,
        base_atom.meta().digest(),
        head_atom.meta().digest(),
    );
    assert!(fs::write(&complete_manifest, &complete).is_ok());
    let Some(complete_path) = root_text(&complete_manifest) else {
        return;
    };
    let complete_output = run(&[
        "change",
        "--base",
        base_root,
        "--head",
        head_root,
        "--config",
        config_path,
        "--manifest",
        complete_path,
    ]);
    assert!(complete_output.is_some());
    assert_eq!(
        complete_output.and_then(|output| output.status.code()),
        Some(0)
    );

    let weakening_manifest = base.path().join("weakening.toml");
    let weakening = format!(
        r#"{complete}
[[policy_ratchets]]
id = "POL-001"
base_subjects = ["SPG-INTEGRATION-001"]
head_subjects = []
base_roles = ["positive", "negative"]
head_roles = ["positive"]
base_mode = "blocking"
head_mode = "advisory"
base_ratchet = "line-a"
head_ratchet = "line-b"
migration_approved = false
debt_recorded = false
waiver_recorded = false
"#
    );
    assert!(fs::write(&weakening_manifest, weakening).is_ok());
    let Some(weakening_path) = root_text(&weakening_manifest) else {
        return;
    };
    let weakening_output = run(&[
        "change",
        "--base",
        base_root,
        "--head",
        head_root,
        "--config",
        config_path,
        "--manifest",
        weakening_path,
    ]);
    assert!(weakening_output.is_some());
    assert_eq!(
        weakening_output.and_then(|output| output.status.code()),
        Some(70)
    );
}

#[test]
fn execute_cli_normalizes_partial_fail_error_and_timeout_without_logs() {
    let oracles = r#"
[[oracles]]
id = "ORC-PASS"
program = "builtin:pass"
timeout_ms = 10

[[oracles]]
id = "ORC-NOT-RUN"
program = "builtin:not-run"
timeout_ms = 10

[[oracles]]
id = "ORC-FAIL"
program = "builtin:fail"
timeout_ms = 10

[[oracles]]
id = "ORC-ERROR"
program = "builtin:error"
timeout_ms = 10

[[oracles]]
id = "ORC-TIMEOUT"
program = "builtin:timeout"
timeout_ms = 10
"#;
    let repository = repository(ATOM_BASE, oracles);
    assert!(repository.is_some());
    let Some((repository, config_path)) = repository else {
        return;
    };
    let (Some(root), Some(config_path)) = (root_text(repository.path()), root_text(&config_path))
    else {
        return;
    };
    let output = run(&[
        "execute",
        "--root",
        root,
        "--config",
        config_path,
        "--format",
        "machine",
    ]);
    assert!(output.is_some());
    let Some(output) = output else { return };
    assert_eq!(output.status.code(), Some(4));
    let text = String::from_utf8(output.stdout).unwrap_or_default();
    for verdict in ["pass", "not-run", "fail", "error", "timeout"] {
        assert!(text.contains(verdict));
    }
    assert!(!text.contains("raw_stdout"));
    assert!(!text.contains("raw_stderr"));
}

#[test]
fn effectful_oracle_logs_are_noncanonical_and_derivation_scope_is_retained() {
    let temporary = TempDir::new().ok();
    assert!(temporary.is_some());
    let Some(temporary) = temporary else { return };
    let command = oracle_command(
        "ORC-DERIVATION",
        "builtin:pass",
        Vec::new(),
        1,
        Some(String::from("DERIV-001")),
    );
    let receipt = run_oracle(&command, temporary.path());
    assert_eq!(receipt.verdict, buoy::core::Verdict::Pass);
    assert_eq!(receipt.derivation.as_deref(), Some("DERIV-001"));
    assert!(receipt.raw_stdout.is_empty() && receipt.raw_stderr.is_empty());

    let missing = oracle_command(
        "ORC-MISSING",
        "/definitely/not/a/program",
        Vec::new(),
        1,
        None,
    );
    assert_eq!(
        run_oracle(&missing, temporary.path()).verdict,
        buoy::core::Verdict::Error
    );

    let crash = oracle_command(
        "ORC-CRASH",
        &binary().to_string_lossy(),
        Vec::new(),
        5_000,
        None,
    );
    let crashed = run_oracle(&crash, temporary.path());
    assert_eq!(crashed.verdict, buoy::core::Verdict::Error);
    assert!(!crashed.raw_stderr.is_empty());
}

#[cfg(unix)]
#[test]
#[allow(clippy::too_many_lines)]
fn process_supervisor_requires_protocol_drains_pipes_and_cleans_descendants() {
    let temporary = TempDir::new().ok();
    assert!(temporary.is_some());
    let Some(temporary) = temporary else { return };
    let shell = |id: &str, script: &str, timeout| {
        oracle_command(
            id,
            "/bin/sh",
            vec![String::from("-c"), script.to_owned()],
            timeout,
            None,
        )
    };

    let valid = shell(
        "ORC-PROTOCOL",
        "printf 'spec-provenance-result/v1\\tpass\\n'",
        1_000,
    );
    let valid = run_oracle(&valid, temporary.path());
    assert_eq!(valid.verdict, buoy::core::Verdict::Pass);
    assert_eq!(valid.termination, OracleTermination::Completed);
    assert!(valid.protocol_valid && valid.cleanup_complete);

    let mut explicit_inputs = shell(
        "ORC-INPUTS",
        "read value; if [ \"$value\" = input ] && [ \"$ALLOW\" = yes ]; then printf 'spec-provenance-result/v1\\tpass\\n'; else exit 9; fi",
        1_000,
    );
    explicit_inputs
        .environment
        .insert(String::from("ALLOW"), String::from("yes"));
    explicit_inputs.input = Some(String::from("input\n"));
    let explicit_inputs = run_oracle(&explicit_inputs, temporary.path());
    assert_eq!(explicit_inputs.verdict, buoy::core::Verdict::Pass);
    assert!(explicit_inputs.protocol_valid && explicit_inputs.cleanup_complete);

    for (id, result, expected) in [
        ("ORC-FAIL-PROTOCOL", "fail", Verdict::Fail),
        ("ORC-ERROR-PROTOCOL", "error", Verdict::Error),
        ("ORC-NOT-RUN-PROTOCOL", "not-run", Verdict::NotRun),
    ] {
        let receipt = run_oracle(
            &shell(
                id,
                &format!("printf 'spec-provenance-result/v1\\t{result}\\n'"),
                1_000,
            ),
            temporary.path(),
        );
        assert_eq!(receipt.verdict, expected);
        assert!(receipt.protocol_valid);
    }
    for (id, script) in [
        ("ORC-WRONG-PROTOCOL", "printf 'wrong/v1\\tpass\\n'"),
        (
            "ORC-UNKNOWN-VERDICT",
            "printf 'spec-provenance-result/v1\\tunknown\\n'",
        ),
        ("ORC-NON-UTF8", "printf '\\377\\n'"),
    ] {
        assert_eq!(
            run_oracle(&shell(id, script, 1_000), temporary.path()).termination,
            OracleTermination::MalformedResult
        );
    }

    let malformed = shell("ORC-MALFORMED", "exit 0", 1_000);
    let malformed = run_oracle(&malformed, temporary.path());
    assert_eq!(malformed.verdict, buoy::core::Verdict::Error);
    assert_eq!(malformed.termination, OracleTermination::MalformedResult);
    assert!(!malformed.protocol_valid);

    let abnormal = shell(
        "ORC-ABNORMAL",
        "printf 'spec-provenance-result/v1\\tpass\\n'; exit 7",
        1_000,
    );
    let abnormal = run_oracle(&abnormal, temporary.path());
    assert_eq!(abnormal.verdict, buoy::core::Verdict::Error);
    assert_eq!(abnormal.termination, OracleTermination::AbnormalExit);

    let mut overflow = shell(
        "ORC-OVERFLOW",
        "while :; do printf '0123456789abcdef' >&2; done",
        5_000,
    );
    overflow.stderr_limit = 1_024;
    overflow.combined_output_limit = 2_048;
    overflow.raw_log_retention = 128;
    let overflow = run_oracle(&overflow, temporary.path());
    assert_eq!(overflow.verdict, buoy::core::Verdict::Error);
    assert_eq!(overflow.termination, OracleTermination::OutputOverflow);
    assert!(overflow.cleanup_complete);
    assert!(overflow.raw_stderr.len() <= 128);

    let mut combined_overflow = shell(
        "ORC-COMBINED-OVERFLOW",
        "i=0; while [ $i -lt 200 ]; do printf '0123456789' ; printf '0123456789' >&2; i=$((i+1)); done",
        5_000,
    );
    combined_overflow.stdout_limit = 10_000;
    combined_overflow.stderr_limit = 10_000;
    combined_overflow.combined_output_limit = 1_024;
    combined_overflow.raw_log_retention = 0;
    let combined_overflow = run_oracle(&combined_overflow, temporary.path());
    assert_eq!(
        combined_overflow.termination,
        OracleTermination::OutputOverflow
    );
    assert!(combined_overflow.raw_stdout.is_empty());

    let mut protocol_limit = shell(
        "ORC-PROTOCOL-LIMIT",
        "i=0; while [ $i -lt 500 ]; do printf '0123456789'; i=$((i+1)); done",
        5_000,
    );
    protocol_limit.stdout_limit = 10_000;
    protocol_limit.combined_output_limit = 10_000;
    protocol_limit.raw_log_retention = 0;
    assert_eq!(
        run_oracle(&protocol_limit, temporary.path()).termination,
        OracleTermination::MalformedResult
    );

    let descendants = shell(
        "ORC-DESCENDANTS",
        "while :; do :; done & printf 'spec-provenance-result/v1\\tpass\\n'",
        2_000,
    );
    let started = Instant::now();
    let descendants = run_oracle(&descendants, temporary.path());
    assert_eq!(descendants.verdict, buoy::core::Verdict::Pass);
    assert!(descendants.cleanup_complete);
    assert!(started.elapsed() < Duration::from_secs(2));

    #[cfg(feature = "coverage-controls")]
    {
        assert!(oracle_reader_fault_control());
        let wait_fault = run_oracle_wait_fault_controlled(
            &shell("ORC-WAIT-FAULT", "while :; do :; done", 5_000),
            temporary.path(),
            &CancellationToken::new(),
        );
        assert_eq!(wait_fault.termination, OracleTermination::SupervisorError);
        assert!(wait_fault.cleanup_complete);
    }

    let immediate_cancellation = CancellationToken::new();
    immediate_cancellation.cancel();
    assert_eq!(
        run_oracle_controlled(
            &shell("ORC-IMMEDIATE-CANCEL", "while :; do :; done", 5_000),
            temporary.path(),
            &immediate_cancellation,
        )
        .termination,
        OracleTermination::Cancelled
    );

    let cancellation = CancellationToken::new();
    let request = cancellation.clone();
    let cancellation_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        request.cancel();
    });
    let cancellation_command = shell("ORC-CANCEL", "while :; do :; done", 5_000);
    let cancellation_result =
        run_oracle_controlled(&cancellation_command, temporary.path(), &cancellation);
    assert!(cancellation_thread.join().is_ok());
    assert_eq!(cancellation_result.verdict, buoy::core::Verdict::Error);
    assert_eq!(
        cancellation_result.termination,
        OracleTermination::Cancelled
    );
    assert!(cancellation_result.cleanup_complete);

    assert_eq!(
        run_oracles_controlled(&[], temporary.path(), 1, &CancellationToken::new()),
        Ok(Vec::new())
    );
    let commands = vec![
        oracle_command("ORC-Z", "builtin:pass", Vec::new(), 10, None),
        oracle_command("ORC-A", "builtin:fail", Vec::new(), 10, None),
    ];
    let ordered = run_oracles_controlled(&commands, temporary.path(), 2, &CancellationToken::new());
    assert!(ordered.is_ok());
    assert_eq!(
        ordered.as_ref().ok().map(|receipts| receipts
            .iter()
            .map(|receipt| receipt.id.as_str())
            .collect::<Vec<_>>()),
        Some(vec!["ORC-A", "ORC-Z"])
    );
    assert_eq!(
        run_oracles_controlled(&commands, temporary.path(), 0, &CancellationToken::new(),),
        Err(OracleSetError::Concurrency)
    );
}

#[cfg(unix)]
#[test]
fn process_timeout_is_normalized_without_elapsed_time() {
    let temporary = TempDir::new().ok();
    assert!(temporary.is_some());
    let Some(temporary) = temporary else { return };
    let command = oracle_command(
        "ORC-PROCESS-TIMEOUT",
        "/bin/sh",
        vec![String::from("-c"), String::from("printf secret; sleep 1")],
        1,
        None,
    );
    let receipt = run_oracle(&command, temporary.path());
    assert_eq!(receipt.verdict, buoy::core::Verdict::Timeout);
    assert!(receipt.raw_stdout.is_empty() || receipt.raw_stdout == b"secret");
}

#[cfg(all(feature = "coverage-controls", unix))]
#[test]
fn mount_descriptor_reads() {
    let temporary = TempDir::new().expect("mounted descriptor parent");
    let project = temporary.path().join("project");
    let authority = temporary.path().join("authority");
    let original = temporary.path().join("authority-opened");
    let attacker = temporary.path().join("attacker");
    for root in [&project, &authority, &attacker] {
        fs::create_dir_all(root.join("src")).expect("mounted source directory");
    }
    fs::write(project.join("src/project.bin"), b"project").expect("project bytes");
    fs::write(authority.join("src/authority.bin"), b"original").expect("authority bytes");
    fs::write(attacker.join("src/authority.bin"), b"attacker").expect("attacker bytes");
    let config = mounted_artifact_config("");
    let mappings = mounted_mappings(&project, &authority);
    let snapshot = snapshot_mounted_repository_after_roots_open_control(&mappings, &config, || {
        fs::rename(&authority, &original).expect("preserve opened authority root");
        symlink(&attacker, &authority).expect("replace authority path with link");
    })
    .expect("descriptor-bound mounted snapshot");
    assert_eq!(
        snapshot
            .files
            .iter()
            .find(|file| file.path.as_str() == "authority/src/authority.bin")
            .expect("authority snapshot file")
            .bytes,
        b"original"
    );

    let link_root = temporary.path().join("project-link");
    symlink(&project, &link_root).expect("link-like mount root");
    assert_eq!(
        snapshot_mounted_repository(&mounted_mappings(&link_root, &original), &config,),
        Err(RepositoryError::MountRoot)
    );

    fs::remove_file(&authority).expect("remove replacement link");
    fs::rename(&original, &authority).expect("restore authority path");
    symlink(
        project.join("src/project.bin"),
        project.join("src/link.bin"),
    )
    .expect("link-like selected entry");
    assert_eq!(
        snapshot_mounted_repository(&mounted_mappings(&project, &authority), &config),
        Err(RepositoryError::LinkEntry)
    );
    fs::remove_file(project.join("src/link.bin")).expect("remove selected link");
    fs::create_dir_all(project.join("src/.git")).expect("nested repository marker");
    assert_eq!(
        snapshot_mounted_repository(&mounted_mappings(&project, &authority), &config),
        Err(RepositoryError::SubmoduleEntry)
    );
}

#[cfg(unix)]
#[test]
fn mount_budget_failure() {
    let project = TempDir::new().expect("project mount");
    let authority = TempDir::new().expect("authority mount");
    fs::create_dir_all(project.path().join("src")).expect("project source");
    fs::create_dir_all(authority.path().join("src")).expect("authority source");
    fs::write(project.path().join("src/one.bin"), b"1234").expect("project bytes");
    fs::write(authority.path().join("src/two.bin"), b"5678").expect("authority bytes");
    let config = mounted_artifact_config(
        r"
[budgets]
path_depth = 64
directory_entries = 100
selected_files = 10
file_bytes = 10
total_bytes = 7
graph_objects = 200000
graph_references = 2000000
diagnostics = 100000
source_nodes = 200000
source_edges = 400000
source_depth = 64
source_visited = 200000
",
    );
    assert_eq!(
        snapshot_mounted_repository(&mounted_mappings(project.path(), authority.path()), &config,),
        Err(RepositoryError::TotalBytesBudget)
    );
}

#[cfg(unix)]
#[test]
fn oracle_mount_isolation() {
    let project = TempDir::new().expect("project mount");
    let authority = TempDir::new().expect("authority mount");
    fs::create_dir_all(project.path().join("src")).expect("project source");
    fs::create_dir_all(authority.path().join("src")).expect("authority source");
    fs::write(project.path().join("src/control.bin"), b"control").expect("project input");
    fs::write(project.path().join("project-marker"), b"marker").expect("oracle marker");
    let oracle = r#"
[[oracles]]
id = "ORC-MOUNT-ISOLATION"
program = "/bin/sh"
args = ['-c', 'if [ -f project-marker ]; then printf "spec-provenance-result/v1\\tpass\\n"; else exit 9; fi']
working_root = "project"
timeout_ms = 1000
"#;
    let config = mounted_artifact_config(oracle);
    let mappings = mounted_mappings(project.path(), authority.path());
    let snapshot =
        snapshot_mounted_repository(&mappings, &config).expect("mounted oracle snapshot");
    let before = check_snapshot(&snapshot, &config).expect("static result before oracle");
    let receipts = run_oracles_at_roots_controlled(
        config.oracles(),
        &[project.path().to_path_buf()],
        config.oracle_concurrency(),
        &CancellationToken::new(),
    )
    .expect("mounted oracle execution");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].verdict, Verdict::Pass);
    let after = check_snapshot(&snapshot, &config).expect("static result after oracle");
    assert_eq!(before.graph_digest, after.graph_digest);
    assert_eq!(before.diagnostics, after.diagnostics);

    assert_eq!(
        run_oracles_at_roots_controlled(
            config.oracles(),
            &[],
            config.oracle_concurrency(),
            &CancellationToken::new(),
        ),
        Err(OracleSetError::Worker)
    );
    let invalid = oracle.replace("working_root = \"project\"", "working_root = \"missing\"");
    let invalid_config = format!(
        r#"schema = "buoy/repository-config/v3"
digest_algorithms = ["xxh3-128"]
[[mounts]]
id = "project"
prefix = "project"
[[selectors]]
mount = "project"
root = "src"
adapter = "artifact/v1"
include = ["**/*"]
exclude = []
authority_expectation = "forbidden"
{invalid}
"#,
    );
    assert_eq!(
        RepositoryConfig::parse(invalid_config.as_bytes()),
        Err(RepositoryError::OracleMount)
    );
}
