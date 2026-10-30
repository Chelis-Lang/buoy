//! Focused process controls for generator success and failure paths.

#![allow(clippy::expect_used, clippy::too_many_lines)]

#[cfg(feature = "coverage-controls")]
use buoy::check::{
    ArtifactAttachmentFact, LogicalMountFact, MachineReport, MigrationReport, OpaqueSubjectFact,
    RequirementSubjectFact, render_human,
};
#[cfg(feature = "coverage-controls")]
use buoy::cli::artifact_binding_real_io_coverage_control;
#[cfg(feature = "coverage-controls")]
use buoy::core::LogicalMountId;
#[cfg(feature = "coverage-controls")]
use buoy::migration::migration_active_error_control;
#[cfg(feature = "coverage-controls")]
use buoy::oracle::{CancellationToken, run_oracles_at_roots_controlled};
#[cfg(feature = "coverage-controls")]
use buoy::repository::{
    MountMappings, RepositoryConfig, repository_active_error_control,
    snapshot_mounted_repository_after_roots_open_control,
    snapshot_repository_after_root_open_control, snapshot_repository_fault_control,
};
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type TestResult<T> = Result<T, Box<dyn Error>>;

#[cfg(feature = "coverage-controls")]
#[allow(clippy::drop_non_drop, dropping_copy_types)]
fn shell_error_translation_controls() {
    drop(std::hint::black_box(repository_active_error_control()));
    drop(std::hint::black_box(migration_active_error_control()));
    assert!(std::hint::black_box(
        artifact_binding_real_io_coverage_control()
    ));
    let requirement_subject = RequirementSubjectFact {
        id: String::from("COVERAGE-REQUIREMENT"),
        subject: String::from("xxh3-128:22222222222222222222222222222222"),
        atom_revision: String::from("xxh3-128:33333333333333333333333333333333"),
        path: String::from("project/spec.md"),
        scopes: vec![String::from("coverage.scope")],
    };
    let mut later_subject = requirement_subject.clone();
    later_subject.id = String::from("COVERAGE-REQUIREMENT-LATER");
    assert!(requirement_subject < later_subject);
    let human = render_human(&MachineReport {
        command: String::from("coverage"),
        graph: String::from("xxh3-128:00000000000000000000000000000000"),
        diagnostics: Vec::new(),
        migration: MigrationReport::default(),
        verdicts: std::collections::BTreeMap::new(),
        impacts: Vec::new(),
        mounts: vec![LogicalMountFact {
            id: String::from("project"),
            prefix: String::from("project"),
        }],
        requirement_subjects: vec![requirement_subject],
        opaque_subjects: vec![OpaqueSubjectFact {
            path: String::from("project/artifact.bin"),
            subject: String::from("xxh3-128:00000000000000000000000000000000"),
        }],
        artifact_attachments: vec![ArtifactAttachmentFact {
            relation: String::from("COVERAGE-RELATION"),
            artifact: String::from("project/artifact.bin"),
            subject: String::from("xxh3-128:00000000000000000000000000000000"),
            record: String::from("xxh3-128:11111111111111111111111111111111"),
            fresh: true,
        }],
    });
    assert!(human.contains("requirement subject COVERAGE-REQUIREMENT"));
    assert!(human.contains("artifact attachment COVERAGE-RELATION"));

    let oracle_config = RepositoryConfig::parse(
        br#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[[selectors]]
root = "spec"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "optional"

[[oracles]]
id = "ORC-SECOND"
program = "builtin:pass"
timeout_ms = 1000

[[oracles]]
id = "ORC-FIRST"
program = "builtin:pass"
timeout_ms = 1000
"#,
    )
    .expect("coverage oracle configuration");
    let current = std::env::current_dir().expect("coverage current directory");
    let receipts = run_oracles_at_roots_controlled(
        oracle_config.oracles(),
        &[current.clone(), current],
        2,
        &CancellationToken::new(),
    )
    .expect("coverage mounted oracles");
    assert_eq!(receipts.len(), 2);

    let mounted_root = tempfile::TempDir::new().expect("coverage mounted root");
    fs::create_dir(mounted_root.path().join("src")).expect("coverage mounted source root");
    fs::write(mounted_root.path().join("src/control.bin"), b"control")
        .expect("coverage mounted source");
    let mounted_config = RepositoryConfig::parse(include_bytes!(
        "../fixtures/logical-mount-artifact-bindings/v1/configs/one-mount.toml"
    ))
    .expect("coverage mounted configuration");
    let mappings = MountMappings::new(vec![(
        LogicalMountId::new("project").expect("coverage mount ID"),
        mounted_root.path().to_path_buf(),
    )])
    .expect("coverage mount mappings");
    assert!(
        snapshot_mounted_repository_after_roots_open_control(
            &mappings,
            &mounted_config,
            std::thread::yield_now,
        )
        .is_ok()
    );

    let single_root = tempfile::TempDir::new().expect("coverage single root");
    fs::create_dir(single_root.path().join("spec")).expect("coverage specification root");
    fs::write(single_root.path().join("spec/control.md"), b"control")
        .expect("coverage specification");
    let single_config = RepositoryConfig::parse(
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
    .expect("coverage single configuration");
    assert!(
        snapshot_repository_after_root_open_control(
            single_root.path(),
            &single_config,
            std::thread::yield_now,
        )
        .is_ok()
    );
    assert!(
        snapshot_repository_fault_control(single_root.path(), &single_config, "selector-stat")
            .is_err()
    );
    assert!(
        snapshot_repository_fault_control(single_root.path(), &single_config, "selector-type")
            .is_err()
    );
    assert!(MountMappings::parse(&[String::from("project=")]).is_err());
    assert!(MountMappings::parse(&[String::from("Bad=path")]).is_err());
}

fn coverage_binary(name: &str) -> TestResult<PathBuf> {
    let executable = std::env::current_exe()?;
    let examples = executable
        .parent()
        .ok_or_else(|| io::Error::other("coverage harness has no parent directory"))?;
    let debug = examples
        .parent()
        .ok_or_else(|| io::Error::other("coverage harness has no target directory"))?;
    Ok(debug.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)))
}

fn write(root: &Path, relative: &str, content: &[u8]) -> TestResult<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn run(binary: &Path, root: &Path, arguments: &[&str]) -> TestResult<Output> {
    Ok(Command::new(binary)
        .current_dir(root)
        .args(arguments)
        .output()?)
}

fn require_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn require_failure(output: &Output) {
    assert!(!output.status.success(), "command unexpectedly succeeded");
}

fn seed_property_evidence(root: &Path) -> TestResult<()> {
    for path in [
        "rust-toolchain.toml",
        "tests/hegel_properties.rs",
        "tests/xxh3_hegel_properties.rs",
        "tests/hegel_controls.rs",
        "tests/logical_mount_hegel_properties.rs",
        "tests/publication_hegel_properties.rs",
        "tests/requirement_subject_inventory.rs",
        "tests/bounded_mount_binding_model.rs",
        "crates/buoy-core/src/algorithms.rs",
        "crates/buoy-core/src/value.rs",
        "crates/buoy-core/src/mount.rs",
        "crates/buoy-core/src/attachment.rs",
        "src/adapters/artifact.rs",
        "src/check.rs",
        "src/publication.rs",
        "src/publication_fs.rs",
        "src/publication_models.rs",
        "crates/buoy-core/tests/xxh3_hegel_envelope.rs",
        "crates/buoy-reference/src/lib.rs",
        "fixtures/xxh3/v2/hegel-replay.toml",
        "fixtures/property/v1/replays/intentionally-false.tsv",
    ] {
        write(root, path, format!("fixture={path}\n").as_bytes())?;
    }
    fs::create_dir_all(root.join("assurance/v1"))?;
    Ok(())
}

fn property_evidence_generator_covers_write_stale_and_error_paths() -> TestResult<()> {
    let property_evidence = coverage_binary("generate-property-evidence")?;
    let repository = tempfile::tempdir()?;
    seed_property_evidence(repository.path())?;

    require_success(&run(&property_evidence, repository.path(), &[])?);
    require_success(&run(&property_evidence, repository.path(), &["--check"])?);

    write(
        repository.path(),
        "assurance/v1/property-inventory.tsv",
        b"stale\n",
    )?;
    require_failure(&run(&property_evidence, repository.path(), &["--check"])?);
    require_success(&run(&property_evidence, repository.path(), &[])?);

    fs::remove_file(repository.path().join("assurance/v1/property-runs.tsv"))?;
    require_failure(&run(&property_evidence, repository.path(), &["--check"])?);
    require_failure(&run(&property_evidence, repository.path(), &["invalid"])?);

    fs::remove_file(repository.path().join("tests/hegel_properties.rs"))?;
    require_failure(&run(&property_evidence, repository.path(), &[])?);

    let write_failure = tempfile::tempdir()?;
    seed_property_evidence(write_failure.path())?;
    fs::create_dir(
        write_failure
            .path()
            .join("assurance/v1/hegel-tool-binding.toml"),
    )?;
    require_failure(&run(&property_evidence, write_failure.path(), &[])?);

    for missing_binding in [
        "rust-toolchain.toml",
        "crates/buoy-core/src/value.rs",
        "crates/buoy-reference/src/lib.rs",
        "fixtures/property/v1/replays/intentionally-false.tsv",
    ] {
        let missing = tempfile::tempdir()?;
        seed_property_evidence(missing.path())?;
        fs::remove_file(missing.path().join(missing_binding))?;
        require_failure(&run(&property_evidence, missing.path(), &[])?);
    }
    Ok(())
}

fn property_fixture_generator_covers_write_stale_and_io_errors() -> TestResult<()> {
    let property_fixtures = coverage_binary("generate-property-fixtures")?;
    let repository = tempfile::tempdir()?;
    require_success(&run(&property_fixtures, repository.path(), &[])?);
    require_success(&run(&property_fixtures, repository.path(), &["--check"])?);

    write(
        repository.path(),
        "fixtures/property/v1/manifest.tsv",
        b"stale\n",
    )?;
    require_failure(&run(&property_fixtures, repository.path(), &["--check"])?);
    require_success(&run(&property_fixtures, repository.path(), &[])?);

    fs::remove_file(repository.path().join("fixtures/property/v1/manifest.tsv"))?;
    require_failure(&run(&property_fixtures, repository.path(), &["--check"])?);
    require_failure(&run(&property_fixtures, repository.path(), &["invalid"])?);

    let create_failure = tempfile::tempdir()?;
    write(
        create_failure.path(),
        "fixtures/property/v1/inventories",
        b"not-a-directory\n",
    )?;
    require_failure(&run(&property_fixtures, create_failure.path(), &[])?);

    let write_failure = tempfile::tempdir()?;
    fs::create_dir_all(
        write_failure
            .path()
            .join("fixtures/property/v1/inventories/negative-domain-missing.tsv"),
    )?;
    require_failure(&run(&property_fixtures, write_failure.path(), &[])?);
    Ok(())
}

fn seed_v1(root: &Path) -> TestResult<()> {
    write(
        root,
        "fixtures/v1/models/model.toml",
        b"schema = \"spec-provenance-fixture-cases/v1\"\nid = \"model-sample-v1\"\nid = \"model-already-v2\"\ndigest_algorithm = \"sha256\"\npath = \"fixtures/v1/sample\"\n",
    )?;
    write(root, "fixtures/v1/root.txt", b"spec-provenance.semantic/v1")?;
    Ok(())
}

fn v2_fixture_generator_covers_write_stale_and_io_errors() -> TestResult<()> {
    let v2_fixtures = coverage_binary("generate-v2-fixture-suite")?;
    let repository = tempfile::tempdir()?;
    require_success(&run(&v2_fixtures, repository.path(), &["--self-test"])?);
    seed_v1(repository.path())?;
    require_success(&run(&v2_fixtures, repository.path(), &["--write"])?);
    require_success(&run(&v2_fixtures, repository.path(), &["--check"])?);

    write(repository.path(), "fixtures/v2/extra.txt", b"extra\n")?;
    require_failure(&run(&v2_fixtures, repository.path(), &["--check"])?);
    fs::remove_file(repository.path().join("fixtures/v2/extra.txt"))?;

    write(
        repository.path(),
        "fixtures/v2/models/model.toml",
        b"stale\n",
    )?;
    require_failure(&run(&v2_fixtures, repository.path(), &["--check"])?);
    require_success(&run(&v2_fixtures, repository.path(), &["--write"])?);

    fs::remove_dir_all(repository.path().join("fixtures/v2"))?;
    require_failure(&run(&v2_fixtures, repository.path(), &["--check"])?);
    require_failure(&run(&v2_fixtures, repository.path(), &["invalid"])?);

    let missing_source = tempfile::tempdir()?;
    require_failure(&run(&v2_fixtures, missing_source.path(), &["--write"])?);

    let empty_source = tempfile::tempdir()?;
    fs::create_dir_all(empty_source.path().join("fixtures/v1"))?;
    require_failure(&run(&v2_fixtures, empty_source.path(), &["--check"])?);

    let unreadable_source = tempfile::tempdir()?;
    write(
        unreadable_source.path(),
        "fixtures/v1/invalid.bin",
        &[0xff, 0xfe],
    )?;
    require_failure(&run(&v2_fixtures, unreadable_source.path(), &["--write"])?);

    let create_failure = tempfile::tempdir()?;
    seed_v1(create_failure.path())?;
    write(
        create_failure.path(),
        "fixtures/v2/models",
        b"not-a-directory\n",
    )?;
    require_failure(&run(&v2_fixtures, create_failure.path(), &["--write"])?);

    let write_failure = tempfile::tempdir()?;
    seed_v1(write_failure.path())?;
    fs::create_dir_all(write_failure.path().join("fixtures/v2/models/model.toml"))?;
    require_failure(&run(&v2_fixtures, write_failure.path(), &["--write"])?);
    Ok(())
}

fn coverage_fixture_generator_covers_write_stale_and_io_errors() -> TestResult<()> {
    let coverage_fixtures = coverage_binary("generate-coverage-fixtures")?;
    let repository = tempfile::tempdir()?;
    require_success(&run(&coverage_fixtures, repository.path(), &[])?);
    require_success(&run(&coverage_fixtures, repository.path(), &["--check"])?);

    write(
        repository.path(),
        "fixtures/coverage/v1/manifest.tsv",
        b"stale\n",
    )?;
    require_failure(&run(&coverage_fixtures, repository.path(), &["--check"])?);
    require_success(&run(&coverage_fixtures, repository.path(), &[])?);

    fs::remove_file(repository.path().join("fixtures/coverage/v1/manifest.tsv"))?;
    require_failure(&run(&coverage_fixtures, repository.path(), &["--check"])?);
    require_failure(&run(&coverage_fixtures, repository.path(), &["invalid"])?);

    let create_failure = tempfile::tempdir()?;
    write(
        create_failure.path(),
        "fixtures/coverage/v1/exports",
        b"not-a-directory\n",
    )?;
    require_failure(&run(&coverage_fixtures, create_failure.path(), &[])?);

    let write_failure = tempfile::tempdir()?;
    fs::create_dir_all(
        write_failure
            .path()
            .join("fixtures/coverage/v1/exports/negative-absent-branches.json"),
    )?;
    require_failure(&run(&coverage_fixtures, write_failure.path(), &[])?);
    Ok(())
}

fn casefold_generator_covers_write_stale_and_io_errors() -> TestResult<()> {
    let casefold = coverage_binary("generate-casefold-table")?;
    let source = fs::read("unicode/16.0.0/CaseFolding.txt")?;
    let output = fs::read("crates/buoy-core/src/casefold_table.rs")?;
    let repository = tempfile::tempdir()?;
    write(repository.path(), "unicode/16.0.0/CaseFolding.txt", &source)?;
    write(
        repository.path(),
        "crates/buoy-core/src/casefold_table.rs",
        &output,
    )?;
    require_success(&run(&casefold, repository.path(), &["--check"])?);
    require_success(&run(&casefold, repository.path(), &[])?);

    write(
        repository.path(),
        "crates/buoy-core/src/casefold_table.rs",
        b"stale\n",
    )?;
    require_failure(&run(&casefold, repository.path(), &["--check"])?);
    fs::remove_file(
        repository
            .path()
            .join("crates/buoy-core/src/casefold_table.rs"),
    )?;
    require_failure(&run(&casefold, repository.path(), &["--check"])?);
    require_failure(&run(&casefold, repository.path(), &["invalid"])?);

    let missing_source = tempfile::tempdir()?;
    require_failure(&run(&casefold, missing_source.path(), &["--check"])?);

    let malformed_source = tempfile::tempdir()?;
    write(
        malformed_source.path(),
        "unicode/16.0.0/CaseFolding.txt",
        b"# CaseFolding-16.0.0.txt\nD800; C; 0061; # surrogate\n# EOF\n",
    )?;
    require_failure(&run(&casefold, malformed_source.path(), &[])?);

    let write_failure = tempfile::tempdir()?;
    write(
        write_failure.path(),
        "unicode/16.0.0/CaseFolding.txt",
        &source,
    )?;
    fs::create_dir_all(
        write_failure
            .path()
            .join("crates/buoy-core/src/casefold_table.rs"),
    )?;
    require_failure(&run(&casefold, write_failure.path(), &[])?);
    Ok(())
}

fn validator_process_controls() -> TestResult<()> {
    let repository = std::env::current_dir()?;
    let empty = tempfile::tempdir()?;

    let validator = coverage_binary("check-reference-independence")?;
    require_failure(&run(&validator, empty.path(), &["invalid"])?);
    require_failure(&run(&validator, empty.path(), &[])?);

    let schema = coverage_binary("check-schema-inventory")?;
    require_failure(&run(&schema, empty.path(), &["one", "two"])?);
    require_failure(&run(&schema, empty.path(), &["missing.tsv"])?);
    require_failure(&run(&schema, empty.path(), &["--self-test"])?);
    write(
        empty.path(),
        "invalid-schema.tsv",
        b"category\tparent\tname\ttag\tshape\tcardinality\tcollection\trule\nfield\ttoo-few\n",
    )?;
    require_failure(&run(&schema, empty.path(), &["invalid-schema.tsv"])?);

    let property = coverage_binary("check-property-evidence")?;
    require_failure(&run(&property, repository.as_path(), &["invalid"])?);
    require_failure(&run(&property, empty.path(), &[])?);

    let coverage = coverage_binary("check-exact-coverage")?;
    require_failure(&run(&coverage, empty.path(), &["invalid"])?);
    require_failure(&run(&coverage, empty.path(), &[])?);
    require_success(&run(&coverage, repository.as_path(), &[])?);
    require_success(&run(
        &coverage,
        repository.as_path(),
        &["--check-inventory"],
    )?);
    drop(run(
        &coverage,
        repository.as_path(),
        &[
            "--check-result",
            "fixtures/coverage/v1/results/positive.tsv",
        ],
    )?);
    require_success(&run(
        &coverage,
        repository.as_path(),
        &["--sha256", "Cargo.toml"],
    )?);
    require_success(&run(
        &coverage,
        repository.as_path(),
        &[
            "--binding",
            "process-control",
            "portable",
            "Cargo.toml",
            "assurance/v1/coverage-command.toml",
            "assurance/v1/source-inventory.tsv",
        ],
    )?);
    require_success(&run(
        &coverage,
        repository.as_path(),
        &[
            "--validate-fixture",
            "fixtures/coverage/v1/repository",
            "fixtures/coverage/v1/inventories/positive.tsv",
            "fixtures/coverage/v1/exports/positive.json",
            "fixtures/coverage/v1/metadata/positive.toml",
        ],
    )?);
    drop(run(
        &coverage,
        repository.as_path(),
        &[
            "--normalize",
            "fixtures/coverage/v1/repository",
            "fixtures/coverage/v1/inventories/positive.tsv",
            "fixtures/coverage/v1/metadata/positive.toml",
            "fixtures/coverage/v1/exports/positive.json",
        ],
    )?);
    drop(run(
        &coverage,
        repository.as_path(),
        &[
            "--check-current",
            "fixtures/coverage/v1/repository",
            "fixtures/coverage/v1/inventories/positive.tsv",
            "fixtures/coverage/v1/results/positive.tsv",
            "fixtures/coverage/v1/metadata/positive.toml",
            "fixtures/coverage/v1/exports/positive.json",
        ],
    )?);
    drop(run(
        &coverage,
        repository.as_path(),
        &["fixtures/coverage/v1/exports/positive.json"],
    )?);

    let migration = coverage_binary("check-digest-transition-model")?;
    require_failure(&run(&migration, empty.path(), &["invalid"])?);

    let vectors = coverage_binary("check-xxh3-vectors")?;
    require_failure(&run(&vectors, repository.as_path(), &["invalid"])?);
    require_failure(&run(&vectors, empty.path(), &[])?);

    let fixtures = coverage_binary("check-fixture-manifest")?;
    require_failure(&run(&fixtures, empty.path(), &[])?);
    Ok(())
}

fn main() -> TestResult<()> {
    #[cfg(feature = "coverage-controls")]
    shell_error_translation_controls();
    validator_process_controls()?;
    property_evidence_generator_covers_write_stale_and_error_paths()?;
    property_fixture_generator_covers_write_stale_and_io_errors()?;
    v2_fixture_generator_covers_write_stale_and_io_errors()?;
    coverage_fixture_generator_covers_write_stale_and_io_errors()?;
    casefold_generator_covers_write_stale_and_io_errors()
}
