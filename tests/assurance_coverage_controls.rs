#![doc = "Behavior controls for assurance executables that are otherwise invoked outside Cargo's test harness."]
#![allow(
    clippy::expect_used,
    clippy::needless_borrow,
    clippy::needless_match,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::unnecessary_wraps
)]

#[cfg(feature = "coverage-controls")]
use buoy::adapter_sdk::{AdapterDescriptor, AdapterSdkError, SourceProduct};
#[cfg(not(any(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256")))]
use buoy::adapters::AuthorityExpectation;
#[cfg(feature = "coverage-controls")]
use buoy::adapters::{binding_attachment_key_locator, extract_binding_in_mount};
#[cfg(feature = "coverage-controls")]
use buoy::check::{
    ArtifactAttachmentFact, LogicalMountFact, MachineReport, MigrationReport, OpaqueSubjectFact,
    RequirementSubjectFact, render_human,
};
#[cfg(feature = "coverage-controls")]
use buoy::cli::{
    artifact_binding_prepare_coverage_control, artifact_binding_real_io_coverage_control,
};
#[cfg(feature = "coverage-controls")]
use buoy::hashing_policy::scan_hashing_policy_fault_control;
use buoy::hashing_policy::{
    evaluate_hashing_policy_fixture, inspect_resolved_dependency_features, scan_hashing_policy,
};
#[cfg(feature = "coverage-controls")]
use buoy::oracle::{CancellationToken, run_oracles_at_roots_controlled};
use buoy::product_identity;
#[cfg(feature = "coverage-controls")]
use buoy::product_identity::{
    classify_control, scan_control, self_test_fault_control, validate_corpus_control,
    validate_frozen_mutation_control, validate_planning_control,
    validate_snapshot_manifest_control,
};
#[cfg(feature = "coverage-controls")]
use buoy::repository::{
    MountMappings, snapshot_mounted_repository,
    snapshot_mounted_repository_after_roots_open_control,
    snapshot_mounted_repository_fault_control, snapshot_repository,
    snapshot_repository_after_root_open_control, snapshot_repository_fault_control,
};
use buoy::repository::{RepositoryConfig, RepositoryError};
#[cfg(not(any(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256")))]
use buoy::repository::{RepositorySnapshot, SnapshotFile};
#[cfg(feature = "coverage-controls")]
use buoy_core::{
    ActiveAlgorithm, AtomRevisionRef, BindingAttachment, DigestAlgorithm, ImplementationBinding,
    ImplementationBindingRef, LogicalMountId, NavigationKind, RepositoryPath, SemanticDigest,
    SourceItemKind, SourceNodeRef, StableId, StoredBinding,
};
#[cfg(not(any(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256")))]
use buoy_core::{
    DigestAlgorithm, EncodingError, RepositoryPath, SemanticDigest, semantic_digest,
    semantic_digest_observation,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use tempfile::TempDir;

static REPOSITORY_SCAN_LOCK: Mutex<()> = Mutex::new(());

#[cfg(feature = "coverage-controls")]
#[test]
fn active_algorithm_error_controls_translate_unavailable_members() {
    assert_eq!(
        buoy::migration::migration_active_error_control(),
        Err(buoy::migration::MigrationError::MixedVersion)
    );
    assert_eq!(
        buoy::repository::repository_active_error_control(),
        Err(RepositoryError::DigestAlgorithmUnavailable)
    );
}

#[cfg(feature = "coverage-controls")]
#[test]
fn logical_mount_production_coverage_controls_run_in_the_workspace_image() {
    assert!(artifact_binding_real_io_coverage_control());
    let active = ActiveAlgorithm::new(DigestAlgorithm::Xxh3_128).expect("XXH3 member");
    let unrelated = RepositoryPath::new("unrelated.toml").expect("unrelated path");
    let object =
        RepositoryPath::new("buoy/objects/v1/xxh3-128/00000000000000000000000000000000.toml")
            .expect("object path");
    let attachment =
        RepositoryPath::new("buoy/attachments/v1/xxh3-128/00000000000000000000000000000000.toml")
            .expect("attachment path");
    assert!(
        !extract_binding_in_mount(active, None, &unrelated, b"invalid")
            .diagnostics
            .is_empty()
    );
    assert!(
        !extract_binding_in_mount(active, None, &object, &[0xff])
            .diagnostics
            .is_empty()
    );
    assert!(
        !extract_binding_in_mount(active, None, &attachment, &[0xff])
            .diagnostics
            .is_empty()
    );
    let mount_prefix = RepositoryPath::new("project").expect("mount prefix");
    assert_eq!(
        binding_attachment_key_locator(&mount_prefix, Some(&mount_prefix)),
        None
    );
    assert!(
        SourceProduct::from_stored_binding(None)
            .bindings()
            .is_empty()
    );
    let dangling = BindingAttachment::new(
        SourceNodeRef::new(SemanticDigest::xxh3_128([8; 16])),
        ImplementationBindingRef::new(SemanticDigest::xxh3_128([9; 16])),
    );
    assert!(
        SourceProduct::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![dangling],
            Vec::new(),
        )
        .is_err()
    );
    let subject = SourceNodeRef::new(SemanticDigest::xxh3_128([1; 16]));
    let requirement = SourceNodeRef::new(SemanticDigest::xxh3_128([2; 16]));
    let record = ImplementationBinding::new_with(
        active,
        StableId::new("COVERAGE-STORED-BINDING").expect("binding ID"),
        NavigationKind::Implements,
        subject,
        requirement,
        AtomRevisionRef::new(
            StableId::new("COVERAGE-REQUIREMENT").expect("requirement ID"),
            requirement.digest(),
        ),
    )
    .expect("stored binding");
    assert_eq!(
        SourceProduct::from_stored_binding(Some(StoredBinding::new(record.record(), record)))
            .bindings()
            .len(),
        1
    );
    for error in [
        AdapterSdkError::Identity,
        AdapterSdkError::Capability,
        AdapterSdkError::Configuration,
        AdapterSdkError::DuplicateAdapter,
        AdapterSdkError::UnsupportedAdapter,
        AdapterSdkError::Materialization,
        AdapterSdkError::UndeclaredSourceItemKind,
        AdapterSdkError::Encoding,
    ] {
        assert!(!error.code().is_empty());
    }
    for version in ["", "-1", "1-", "release", "1.0.0+reviewed"] {
        let _ = AdapterDescriptor::new(
            "coverage/v1",
            "coverage-parser",
            version,
            "coverage/v1",
            "coverage.configuration/v1",
            vec![String::from("coverage.syntax/v1")],
            vec![SourceItemKind::new("coverage/item/v1").expect("coverage item kind")],
        );
    }
    let report = MachineReport {
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
        requirement_subjects: vec![RequirementSubjectFact {
            id: String::from("COVERAGE-REQUIREMENT"),
            subject: String::from("xxh3-128:22222222222222222222222222222222"),
            atom_revision: String::from("xxh3-128:33333333333333333333333333333333"),
            path: String::from("project/spec.md"),
            scopes: vec![String::from("coverage.scope")],
        }],
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
    };
    let human = render_human(&report);
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

[[oracles]]
id = "ORC-MIDDLE"
program = "builtin:pass"
timeout_ms = 1000
"#,
    )
    .expect("coverage oracle configuration");
    let current = std::env::current_dir().expect("current directory");
    let receipts = run_oracles_at_roots_controlled(
        oracle_config.oracles(),
        &[current.clone(), current.clone(), current],
        1,
        &CancellationToken::new(),
    )
    .expect("mounted oracle set");
    assert_eq!(receipts.len(), 3);
    assert!(
        run_oracles_at_roots_controlled(
            oracle_config.oracles(),
            &[std::env::current_dir().expect("mismatched root")],
            1,
            &CancellationToken::new(),
        )
        .is_err()
    );

    let mounted_root = TempDir::new().expect("mounted root");
    fs::create_dir(mounted_root.path().join("src")).expect("mounted source root");
    fs::write(mounted_root.path().join("src/control.bin"), b"control").expect("mounted source");
    let mounted_config = RepositoryConfig::parse(include_bytes!(
        "../fixtures/logical-mount-artifact-bindings/v1/configs/one-mount.toml"
    ))
    .expect("mounted configuration");
    let project_id = LogicalMountId::new("project").expect("mount ID");
    let mappings = MountMappings::new(vec![(
        project_id.clone(),
        mounted_root.path().to_path_buf(),
    )])
    .expect("mount mappings");
    assert_eq!(
        MountMappings::new(vec![
            (project_id.clone(), mounted_root.path().to_path_buf()),
            (project_id.clone(), mounted_root.path().to_path_buf()),
        ]),
        Err(RepositoryError::MountMappingDuplicate)
    );
    let missing_mappings = MountMappings::new(Vec::new()).expect("missing mappings");
    assert_eq!(
        missing_mappings.validate(&mounted_config),
        Err(RepositoryError::MountMissing)
    );
    let extra_mappings = MountMappings::new(vec![
        (project_id, mounted_root.path().to_path_buf()),
        (
            LogicalMountId::new("extra").expect("extra mount ID"),
            mounted_root.path().to_path_buf(),
        ),
    ])
    .expect("extra mappings");
    assert_eq!(
        extra_mappings.validate(&mounted_config),
        Err(RepositoryError::MountExtra)
    );
    assert!(
        snapshot_mounted_repository_after_roots_open_control(
            &mappings,
            &mounted_config,
            std::thread::yield_now,
        )
        .is_ok()
    );
    assert!(
        snapshot_mounted_repository_fault_control(&mappings, &mounted_config, "selected-paths",)
            .is_err()
    );
    let barnacle = Path::new("fixtures/logical-mount-artifact-bindings/v1/barnacle");
    let barnacle_source =
        fs::read_to_string(barnacle.join("buoy.toml")).expect("Barnacle coverage configuration");
    let barnacle_config = RepositoryConfig::parse(barnacle_source.as_bytes())
        .expect("Barnacle coverage configuration");
    let barnacle_mappings = MountMappings::new(vec![
        (
            LogicalMountId::new("project").expect("project mount"),
            barnacle.join("project"),
        ),
        (
            LogicalMountId::new("authority").expect("authority mount"),
            barnacle.join("authority"),
        ),
    ])
    .expect("Barnacle coverage mappings");
    let barnacle_snapshot = snapshot_mounted_repository(&barnacle_mappings, &barnacle_config)
        .expect("Barnacle coverage snapshot");
    let mut bounded_source = barnacle_source.clone();
    bounded_source.push_str(
        "\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = 1\ngraph_references = 2000000\ndiagnostics = 200000\nsource_nodes = 200000\nsource_edges = 400000\nsource_depth = 64\nsource_visited = 200000\n",
    );
    let bounded_config =
        RepositoryConfig::parse(bounded_source.as_bytes()).expect("bounded Barnacle configuration");
    assert!(artifact_binding_prepare_coverage_control(
        &barnacle_snapshot,
        &barnacle_config,
        &bounded_config,
    ));
    let branch_source = barnacle_source.replace(
        "[[selectors]]\nmount = \"project\"\nroot = \"buoy\"",
        "[[selectors]]\nmount = \"project\"\nroot = \"aaa\"\nadapter = \"binding/v1\"\ninclude = [\"**/*.toml\"]\nexclude = []\nauthority_expectation = \"forbidden\"\n\n[[selectors]]\nmount = \"project\"\nroot = \"buoy\"",
    );
    let branch_config =
        RepositoryConfig::parse(branch_source.as_bytes()).expect("branch coverage configuration");
    assert!(artifact_binding_prepare_coverage_control(
        &barnacle_snapshot,
        &branch_config,
        &bounded_config,
    ));
    assert_eq!(
        snapshot_mounted_repository(&missing_mappings, &mounted_config),
        Err(RepositoryError::MountMissing)
    );
    assert_eq!(
        snapshot_mounted_repository_fault_control(
            &missing_mappings,
            &mounted_config,
            "selected-paths",
        ),
        Err(RepositoryError::MountMissing)
    );

    let single_root = TempDir::new().expect("single root");
    fs::create_dir(single_root.path().join("spec")).expect("specification root");
    fs::write(single_root.path().join("spec/control.md"), b"control").expect("specification");
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
    .expect("single configuration");
    assert_eq!(
        mappings.validate(&single_config),
        Err(RepositoryError::RootMountMixed)
    );
    assert_eq!(
        snapshot_repository(single_root.path(), &mounted_config),
        Err(RepositoryError::RootMountMixed)
    );
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
    assert!(
        snapshot_repository_fault_control(single_root.path(), &single_config, "selector-depth")
            .is_err()
    );
    assert!(
        snapshot_repository_fault_control(single_root.path(), &single_config, "directory-read")
            .is_err()
    );
    assert!(
        snapshot_repository_fault_control(single_root.path(), &single_config, "selected-paths")
            .is_err()
    );
    assert!(MountMappings::parse(&[String::from("project=")]).is_err());
    assert!(MountMappings::parse(&[String::from("=path")]).is_err());
    assert!(MountMappings::parse(&[String::from("Bad=path")]).is_err());
}

#[cfg(feature = "coverage-controls")]
#[test]
fn snapshot_encoding_control_reaches_the_injected_integration_path() {
    let config = RepositoryConfig::parse(
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
    .expect("encoding control configuration");
    let snapshot = buoy::repository::RepositorySnapshot {
        files: vec![buoy::repository::SnapshotFile {
            path: buoy_core::RepositoryPath::new("spec/control.md").expect("control path"),
            bytes: include_bytes!("../fixtures/adapters/markdown/positive.md").to_vec(),
            adapter: String::from("markdown/v1"),
            authority_expectation: buoy::adapters::AuthorityExpectation::Optional,
            selector_root: String::from("spec"),
        }],
    };
    assert_eq!(
        buoy::check::check_snapshot_encoding_control(&snapshot, &config),
        Err(buoy::check::CheckError::Encoding)
    );
}

#[test]
fn repository_configuration_controls_cover_legacy_schema_and_glob_rejection() {
    let base = "schema = \"spec-provenance-config/v2\"\n\
                digest_algorithms = [\"xxh3-128\"]\n\
                [[selectors]]\n\
                root = \"spec\"\n\
                adapter = \"markdown/v1\"\n\
                include = [\"**/*.md\"]\n\
                exclude = []\n\
                authority_expectation = \"optional\"\n";
    assert_eq!(
        RepositoryConfig::parse(base.replace("config/v2", "config/v1").as_bytes()),
        Err(RepositoryError::ConfigSchema)
    );
    assert_eq!(
        RepositoryConfig::parse(base.replace("**/*.md", "[invalid").as_bytes()),
        Err(RepositoryError::GlobSyntax)
    );
    #[cfg(feature = "coverage-controls")]
    assert!(matches!(
        buoy::repository::repository_config_unavailable_control(base.as_bytes()),
        Err(RepositoryError::DigestAlgorithmUnavailable)
    ));
}

#[cfg(not(any(feature = "xxh3-128", feature = "blake3-256", feature = "sha-256")))]
#[test]
fn repository_default_digest_paths_fail_when_the_build_has_no_member() {
    let config = b"schema = \"spec-provenance-config/v2\"\n\
                   digest_algorithms = [\"xxh3-128\"]\n\
                   [[selectors]]\n\
                   root = \"spec\"\n\
                   adapter = \"markdown/v1\"\n\
                   include = [\"**/*.md\"]\n\
                   exclude = []\n\
                   authority_expectation = \"optional\"\n";
    assert_eq!(
        RepositoryConfig::parse(config),
        Err(RepositoryError::DigestAlgorithmUnavailable)
    );
    let file = SnapshotFile {
        path: RepositoryPath::new("spec/control.md").expect("repository path"),
        bytes: b"control".to_vec(),
        adapter: String::from("markdown/v1"),
        authority_expectation: AuthorityExpectation::Optional,
        selector_root: String::from("spec"),
    };
    assert_eq!(
        file.canonical_value(),
        Err(RepositoryError::DigestAlgorithmUnavailable)
    );
    assert_eq!(
        file.semantic_digest(),
        Err(RepositoryError::DigestAlgorithmUnavailable)
    );

    let snapshot = RepositorySnapshot { files: vec![file] };
    let digest = SemanticDigest::parse_canonical("xxh3-128:00000000000000000000000000000000")
        .expect("canonical digest");
    assert_eq!(
        snapshot.semantic_digest(digest, digest, &[]),
        Err(RepositoryError::DigestAlgorithmUnavailable)
    );
    assert_eq!(
        snapshot.artifact_index(),
        Err(RepositoryError::DigestAlgorithmUnavailable)
    );
    assert_eq!(
        semantic_digest("coverage-control/v1", Vec::new()),
        Err(EncodingError::UnavailableAlgorithm)
    );
    assert!(matches!(
        semantic_digest_observation("coverage-control/v1", Vec::new()),
        Err(EncodingError::UnavailableAlgorithm)
    ));
    assert_eq!(
        buoy::publication::page_content_digest(DigestAlgorithm::Xxh3_128, b"content"),
        Err(buoy::publication::PublicationError::DigestMemberUnavailable)
    );
    assert_eq!(
        buoy::publication::publication_basis_digest(DigestAlgorithm::Xxh3_128, b"basis"),
        Err(buoy::publication::PublicationError::DigestMemberUnavailable)
    );
    assert_eq!(
        RepositoryError::PlatformBlocker.code(),
        "REH-PLATFORM-WINDOWS-SNAPSHOT-BLOCKED"
    );
}

#[test]
fn product_identity_assurance_paths_execute_inside_the_instrumented_harness() {
    let _scan_guard = REPOSITORY_SCAN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    product_identity::self_test(Path::new(".")).expect("product-identity controls must pass");
    #[cfg(feature = "coverage-controls")]
    for fault in [
        "create-scratch",
        "write-scratch",
        "generate-scratch",
        "cleanup-scratch",
        "execution-evidence-scratch-create",
        "execution-evidence-fixture-directory",
        "execution-evidence-invalid-manifest-write",
        "execution-evidence-real-manifest-write",
        "execution-evidence-full-render",
        "execution-evidence-scratch-cleanup",
    ] {
        assert!(
            self_test_fault_control(Path::new("."), fault).is_err(),
            "{fault}"
        );
    }
    for argument in ["--check-corpus", "--check-planning"] {
        product_identity::run_cli(&[String::from(argument)], Path::new("."))
            .expect("product-identity check must pass");
    }
    assert!(product_identity::run_cli(&[String::from("--invalid")], Path::new(".")).is_err());
}

#[cfg(feature = "coverage-controls")]
#[test]
fn product_identity_external_parser_controls_reject_malformed_rows() {
    let temporary = TempDir::new().expect("product parser control root");
    let manifest = temporary
        .path()
        .join("assurance/product-identity/v1/pre-rename-snapshots.tsv");
    fs::create_dir_all(manifest.parent().expect("snapshot manifest parent"))
        .expect("snapshot manifest directory");
    fs::write(
        manifest,
        "schema\tsource_path\tsnapshot_path\treplay\nmalformed\n",
    )
    .expect("malformed snapshot manifest");
    assert!(validate_snapshot_manifest_control(temporary.path()).is_err());

    let corpus = fs::read_to_string("assurance/product-identity/v1/pre-rename-corpus.tsv")
        .expect("frozen corpus");
    let header = corpus.lines().next().expect("frozen corpus header");
    let first_row = corpus.lines().nth(1).expect("frozen corpus row");
    let mut fields = first_row.split('\t').map(str::to_owned).collect::<Vec<_>>();
    let digest = fields[4].clone();
    fields[1] = String::from("wrong-id");
    for malformed in [
        corpus.replacen(first_row, "malformed", 1),
        header.to_owned(),
        corpus.replacen(first_row, &fields.join("\t"), 1),
        corpus.replacen(&digest, "xxh3-128:00000000000000000000000000000000", 1),
    ] {
        assert!(validate_corpus_control(Path::new("."), &malformed).is_err());
    }
}

#[cfg(feature = "coverage-controls")]
#[test]
fn product_identity_scanner_and_classification_controls_cover_closed_rules() {
    let _scan_guard = REPOSITORY_SCAN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temporary = TempDir::new().expect("scanner controls root");
    fs::write(temporary.path().join("binary"), [0xff]).expect("binary control");
    fs::write(
        temporary.path().join("text"),
        "prefix spec-provenance/v2 suffix\n",
    )
    .expect("text control");
    assert_eq!(
        scan_control(
            temporary.path(),
            &[Path::new("missing").to_path_buf()],
            "git",
        )
        .expect_err("missing scanner input")
        .code(),
        "PID-SCAN-READ"
    );
    assert_eq!(
        scan_control(
            temporary.path(),
            &[
                Path::new("binary").to_path_buf(),
                Path::new("text").to_path_buf(),
            ],
            "git",
        )
        .expect("mixed scanner inputs"),
        1
    );
    assert!(scan_control(Path::new("."), &[], "repository").expect("repository scan") > 0);
    assert!(scan_control(Path::new("."), &[], "git").expect("direct Git scan") > 0);
    assert_eq!(
        scan_control(temporary.path(), &[], "missing-buoy-git-control")
            .expect_err("missing git executable")
            .code(),
        "PID-SCAN-GIT"
    );
    assert_eq!(
        scan_control(temporary.path(), &[], "repository")
            .expect_err("non-repository git root")
            .code(),
        "PID-SCAN-GIT"
    );

    for (path, context, lexeme, expected) in [
        (
            "docs/schema/x",
            "spec-provenance",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "fixtures/v1/x",
            "spec-provenance",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "fixtures/v2/x",
            "spec-provenance",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "fixtures/xxh3/v2/migration/x",
            "spec-provenance",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "fixtures/adapters/rust/x",
            "spec-provenance",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "tests/adapters.rs",
            "spec_provenance::x",
            "spec_provenance",
            "frozen-v1-identifier",
        ),
        (
            "src/adapter_sdk.rs",
            "spec_provenance::x",
            "spec_provenance",
            "frozen-v1-identifier",
        ),
        (
            "src/adapters/x",
            "spec-provenance",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "crates/buoy-x/x",
            "SPEC-PROVENANCE",
            "SPEC-PROVENANCE",
            "frozen-v1-identifier",
        ),
        (
            "tests/xxh3_x.rs",
            "SPEC-PROVENANCE",
            "SPEC-PROVENANCE",
            "frozen-v1-identifier",
        ),
        (
            "tools/check_xxh3_vectors.rs",
            "spec-provenance",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "other",
            "spec-provenance/v1",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "crates/buoy-x/x",
            "spec-provenance",
            "spec-provenance",
            "rename-product",
        ),
        (
            "tests/xxh3_x.rs",
            "spec-provenance",
            "spec-provenance",
            "rename-product",
        ),
        (
            "other",
            "spec-provenance/v2",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "other",
            "spec-provenance-v1",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "other",
            "spec-provenance-v2",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "other",
            "spec-provenance.semantic/v3",
            "spec-provenance",
            "frozen-v1-identifier",
        ),
        (
            "Cargo.lock",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "nested/Cargo.lock",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "proofs/spikes/aeneas/generated/x",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "proofs/spikes/aeneas/integrity.llbc",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "src/product_identity.rs",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "tests/product_identity.rs",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "tests/assurance_coverage_controls.rs",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "tests/migration.rs",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "assurance/v1/source-inventory.tsv",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "assurance/v1/coverage-result.tsv",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "assurance/supply-chain/v1/closure-inventory.tsv",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "docs/assurance/quality-assurance-status.md",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "fixtures/chelis-provenance/v1/inventory.tsv",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "fixtures/quality/v1/positive-all-states.toml",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "tests/chelis_preimplementation.rs",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "tools/check_reference_independence.rs",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "tools/check_property_evidence.rs",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "tools/check_schema_inventory.rs",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "fixtures/product-identity/x",
            "spec-provenance",
            "spec-provenance",
            "generated-or-external",
        ),
        (
            "openspec/changes/archive/x",
            "spec-provenance",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "openspec/changes/rename-spec-provenance-to-buoy/x",
            "spec-provenance",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "openspec/changes/archive/2026-08-02-add-buoy-quality-assurance-tracking/tasks.md",
            "spec-provenance",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "docs/migration/x",
            "spec-provenance",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "openspec/specs/product-identity/spec.md",
            "spec-provenance",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "assurance/product-identity/x",
            "spec-provenance",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "assurance/v1/red-baseline.toml",
            "spec-provenance",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "other",
            "establish-spec-provenance-system",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "other",
            "harden-spec-provenance-production-readiness",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "other",
            "openspec/changes/archive/x spec-provenance",
            "spec-provenance",
            "historical-reference",
        ),
        (
            "crates/spec-provenance/x",
            "spec-provenance",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "crates/spec-provenance-core/x",
            "spec-provenance",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "other",
            "compatibility-alias spec-provenance",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "other",
            "compatibility-facade spec-provenance",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "other",
            "upgrade-fixture spec-provenance",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "docs/release/buoy-rename-checklist.md",
            "spec-provenance",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "nix/devenv/scripts/core.nix",
            "cargo test --package spec-provenance",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "nix/devenv/scripts/core.nix",
            "unrelated",
            "spec-provenance",
            "rename-product",
        ),
        (
            "nix/devenv/tasks.nix",
            "spec-provenance:check",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "tools/check_devenv_task_graph.rs",
            "task.name.starts_with(\"spec-provenance:\")",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "devenv.nix",
            "spec-provenance",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "Cargo.toml",
            "crates/spec-provenance",
            "spec-provenance",
            "compatibility-alias",
        ),
        (
            "devenv.nix",
            "unrelated",
            "spec-provenance",
            "rename-product",
        ),
        (
            "Cargo.toml",
            "spec-provenance",
            "spec-provenance",
            "rename-product",
        ),
        (
            "docs/current.md",
            "spec-provenance",
            "spec-provenance",
            "rename-product",
        ),
    ] {
        assert_eq!(
            classify_control(path, context, 1, lexeme),
            expected,
            "{path}"
        );
    }
}

#[cfg(feature = "coverage-controls")]
#[test]
fn product_identity_full_check_reaches_current_inventory_success() {
    let _scan_guard = REPOSITORY_SCAN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let scratch = tempfile::Builder::new()
        .prefix(".buoy-product-check-")
        .tempdir_in(".")
        .expect("product check root");
    let copy = |path: &str| {
        let target = scratch.path().join(path);
        fs::create_dir_all(target.parent().expect("copied parent")).expect("copied directory");
        fs::copy(path, target).expect("copied control source");
    };
    for path in [
        "assurance/product-identity/v1/pre-rename-corpus.tsv",
        "assurance/product-identity/v1/pre-rename-snapshots.tsv",
        "assurance/product-identity/v1/snapshots/canonical-vectors-v2.hex",
        "assurance/product-identity/v1/snapshots/protocol-inventory-v2.hex",
        "assurance/product-identity/v1/pre-rename-occurrences.tsv",
        "fixtures/product-identity/v1/manifest.tsv",
        "fixtures/product-identity/v1/frozen-identifier-mutations.tsv",
        "fixtures/product-identity/v1/positive/naming-inventory.tsv",
        "fixtures/product-identity/v1/negative/duplicate.tsv",
        "fixtures/product-identity/v1/negative/broad-exclusion.tsv",
        "fixtures/product-identity/v1/negative/wrong-class.tsv",
        "fixtures/product-identity/v1/negative/modified-frozen.tsv",
        "fixtures/product-identity/v1/negative/missing.tsv",
        "fixtures/product-identity/v1/negative/expired-alias.tsv",
        "assurance/product-identity/v1/evidence-ledger.tsv",
        "assurance/product-identity/v1/coverage-replay.toml",
    ] {
        copy(path);
    }
    let corpus = fs::read_to_string("assurance/product-identity/v1/pre-rename-corpus.tsv")
        .expect("corpus manifest");
    for path in corpus
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').nth(3))
    {
        copy(path);
    }
    let frozen = fs::read_to_string("fixtures/product-identity/v1/frozen-identifier-mutations.tsv")
        .expect("frozen manifest");
    assert!(validate_frozen_mutation_control(Path::new("."), &frozen).is_ok());
    for path in frozen
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').nth(3))
    {
        copy(path);
    }
    let write = |path: &str, text: &str| {
        let target = scratch.path().join(path);
        fs::create_dir_all(target.parent().expect("written parent")).expect("written directory");
        fs::write(target, text).expect("written control source");
    };
    write(
        "openspec/changes/archive/2026-07-19-add-cargo-llvm-cov-hegel-rs-testing-requirements/design.md",
        "map once to hardening tasks 2.10, 2.19, 2.21, and 2.22\n",
    );
    write(
        "openspec/changes/archive/2026-07-19-add-cargo-llvm-cov-hegel-rs-testing-requirements/tasks.md",
        "devenv tasks run buoy:coverage\ndevenv tasks run buoy:properties\n",
    );
    write(
        "openspec/changes/archive/2026-08-02-harden-spec-provenance-production-readiness/tasks.md",
        "maps to one P0 ledger entry\n- [ ] 2.10 control\n- [ ] 2.19 control\n- [ ] 2.21 control\n- [ ] 2.22 control\n",
    );
    write(
        "openspec/changes/archive/2026-08-28-integrate-spec-provenance-with-chelis/design.md",
        "published Buoy shell/core interfaces\n",
    );
    write(
        "assurance/v1/source-inventory.tsv",
        "target_id\tpath\tkind\tplatform\tcoverage_class\tdisposition\trationale\ncontrol\tcrates/buoy-core/src/algorithms.rs\tproduction-source\tportable\tworkspace\troot\tcontrol\ncontrol\tcrates/buoy-reference/src/lib.rs\tproduction-source\tportable\tworkspace\troot\tcontrol\ncontrol\tsrc/bin/buoy.rs\tproduction-source\tportable\tworkspace\troot\tcontrol\ncontrol\tsrc/product_identity.rs\tproduction-source\tportable\tworkspace\troot\tcontrol\n",
    );
    write("assurance/v1/property-runs.tsv", "header\n");
    write(
        "proofs/core/proof-record.toml",
        "path = \"crates/buoy-core/src/algorithms.rs\"\n",
    );
    write(
        "proofs/core/derivation-record.toml",
        "command = \"devenv shell -- buoy-core-proofs\"\n",
    );

    product_identity::run_cli(&[String::from("--check-corpus")], scratch.path())
        .expect("preserved pre-rename snapshots");
    product_identity::freeze_pre_rename(scratch.path())
        .expect("snapshot-aware synthetic frozen inventory");
    let inventory_snapshot = scratch
        .path()
        .join("assurance/product-identity/v1/snapshots/protocol-inventory-v2.hex");
    let inventory_snapshot_bytes =
        fs::read(&inventory_snapshot).expect("saved inventory snapshot control");
    fs::remove_file(&inventory_snapshot).expect("remove inventory snapshot control");
    assert_eq!(
        product_identity::freeze_pre_rename(scratch.path())
            .expect_err("snapshot-aware freeze must reject a missing snapshot")
            .code(),
        "PID-CORPUS-READ"
    );
    fs::write(&inventory_snapshot, inventory_snapshot_bytes)
        .expect("restore inventory snapshot control");
    let vector_snapshot = scratch
        .path()
        .join("assurance/product-identity/v1/snapshots/canonical-vectors-v2.hex");
    let vector_snapshot_bytes = fs::read(&vector_snapshot).expect("saved vector snapshot");
    fs::write(&vector_snapshot, "not-hex").expect("corrupt vector snapshot");
    assert_eq!(
        product_identity::run_cli(&[String::from("--check-corpus")], scratch.path())
            .expect_err("corrupt snapshot")
            .code(),
        "PID-CANONICAL-PARITY"
    );
    fs::write(&vector_snapshot, vector_snapshot_bytes).expect("restore vector snapshot");

    let current_inventory = scratch.path().join("schema/v2/inventory.tsv");
    let current_inventory_bytes = fs::read(&current_inventory).expect("saved current inventory");
    let current_inventory_header = String::from_utf8(current_inventory_bytes.clone())
        .expect("UTF-8 inventory")
        .lines()
        .next()
        .expect("inventory header")
        .to_owned();
    fs::write(&current_inventory, format!("{current_inventory_header}\n"))
        .expect("remove frozen inventory rows");
    assert_eq!(
        product_identity::run_cli(&[String::from("--check-corpus")], scratch.path())
            .expect_err("removed frozen inventory row")
            .code(),
        "PID-CANONICAL-PARITY"
    );
    fs::write(&current_inventory, current_inventory_bytes).expect("restore current inventory");

    let snapshot_manifest = scratch
        .path()
        .join("assurance/product-identity/v1/pre-rename-snapshots.tsv");
    let snapshot_manifest_text =
        fs::read_to_string(&snapshot_manifest).expect("saved snapshot manifest");
    let duplicate_binding = snapshot_manifest_text
        .lines()
        .nth(1)
        .expect("snapshot binding");
    fs::write(
        &snapshot_manifest,
        format!("{snapshot_manifest_text}{duplicate_binding}\n"),
    )
    .expect("duplicate snapshot binding");
    assert_eq!(
        product_identity::run_cli(&[String::from("--check-corpus")], scratch.path())
            .expect_err("duplicate snapshot binding")
            .code(),
        "PID-CANONICAL-PARITY"
    );
    assert_eq!(
        product_identity::freeze_pre_rename(scratch.path())
            .expect_err("freeze must reject a duplicate snapshot binding")
            .code(),
        "PID-CANONICAL-PARITY"
    );
    fs::write(&snapshot_manifest, &snapshot_manifest_text).expect("restore snapshot manifest");

    let inventory_snapshot_bytes = fs::read(&inventory_snapshot).expect("saved inventory snapshot");
    fs::remove_file(&inventory_snapshot).expect("remove inventory snapshot");
    assert_eq!(
        product_identity::run_cli(&[String::from("--check-corpus")], scratch.path())
            .expect_err("missing inventory snapshot")
            .code(),
        "PID-CORPUS-READ"
    );
    fs::write(&inventory_snapshot, inventory_snapshot_bytes).expect("restore inventory snapshot");

    for path in [
        "assurance/product-identity/v1/pre-rename-snapshots.tsv",
        "assurance/product-identity/v1/snapshots/canonical-vectors-v2.hex",
        "assurance/product-identity/v1/snapshots/protocol-inventory-v2.hex",
    ] {
        fs::remove_file(scratch.path().join(path)).expect("remove snapshot bootstrap control");
    }
    product_identity::freeze_pre_rename(scratch.path()).expect("synthetic frozen inventory");
    product_identity::generate_current_inventory(scratch.path())
        .expect("synthetic current inventory");
    product_identity::check(scratch.path()).expect("complete product identity check");
    product_identity::run_cli(&[String::from("--freeze-pre-rename")], scratch.path())
        .expect("freeze CLI control");
    product_identity::run_cli(&[String::from("--generate-inventory")], scratch.path())
        .expect("generate CLI control");
    product_identity::run_cli(&[String::from("--check")], scratch.path())
        .expect("check CLI control");
    product_identity::run_cli(&[], scratch.path()).expect("default check CLI control");

    for (relative, expected) in [
        (
            "fixtures/product-identity/v1/manifest.tsv",
            "PID-FIXTURE-READ",
        ),
        (
            "fixtures/product-identity/v1/frozen-identifier-mutations.tsv",
            "PID-FROZEN-READ",
        ),
        (
            "fixtures/product-identity/v1/positive/naming-inventory.tsv",
            "PID-FIXTURE-READ",
        ),
        (
            "fixtures/product-identity/v1/negative/duplicate.tsv",
            "PID-FIXTURE-READ",
        ),
        (
            "fixtures/product-identity/v1/negative/broad-exclusion.tsv",
            "PID-FIXTURE-READ",
        ),
        (
            "fixtures/product-identity/v1/negative/wrong-class.tsv",
            "PID-FIXTURE-READ",
        ),
        (
            "fixtures/product-identity/v1/negative/modified-frozen.tsv",
            "PID-FIXTURE-READ",
        ),
        (
            "fixtures/product-identity/v1/negative/missing.tsv",
            "PID-FIXTURE-READ",
        ),
        (
            "fixtures/product-identity/v1/negative/expired-alias.tsv",
            "PID-FIXTURE-READ",
        ),
        ("assurance/v1/source-inventory.tsv", "PID-EVIDENCE-READ"),
        ("assurance/v1/property-runs.tsv", "PID-EVIDENCE-READ"),
        ("proofs/core/proof-record.toml", "PID-EVIDENCE-READ"),
        ("proofs/core/derivation-record.toml", "PID-EVIDENCE-READ"),
        (
            "assurance/product-identity/v1/evidence-ledger.tsv",
            "PID-EVIDENCE-READ",
        ),
        (
            "assurance/product-identity/v1/coverage-replay.toml",
            "PID-EVIDENCE-READ",
        ),
    ] {
        let path = scratch.path().join(relative);
        let bytes = fs::read(&path).expect("saved self-test input");
        fs::remove_file(&path).expect("remove self-test input");
        assert_eq!(
            product_identity::self_test(scratch.path())
                .expect_err("missing self-test input")
                .code(),
            expected,
            "{relative}"
        );
        fs::write(path, bytes).expect("restore self-test input");
    }
    for relative in [
        "fixtures/product-identity/v1/manifest.tsv",
        "fixtures/product-identity/v1/frozen-identifier-mutations.tsv",
        "fixtures/product-identity/v1/positive/naming-inventory.tsv",
        "fixtures/product-identity/v1/negative/wrong-class.tsv",
        "fixtures/product-identity/v1/negative/modified-frozen.tsv",
        "fixtures/product-identity/v1/negative/missing.tsv",
        "assurance/product-identity/v1/evidence-ledger.tsv",
    ] {
        let path = scratch.path().join(relative);
        let bytes = fs::read(&path).expect("saved malformed self-test input");
        fs::write(&path, b"invalid\n").expect("malformed self-test input");
        assert!(
            product_identity::self_test(scratch.path()).is_err(),
            "{relative}"
        );
        fs::write(path, bytes).expect("restore malformed self-test input");
    }

    let blocked_root = scratch.path().join("blocked-root");
    fs::write(&blocked_root, b"file").expect("blocked root file");
    assert_eq!(
        product_identity::generate_current_inventory(&blocked_root)
            .expect_err("file cannot own inventory parent")
            .code(),
        "PID-WRITE-PATH"
    );
    assert_eq!(
        product_identity::freeze_pre_rename(&blocked_root)
            .expect_err("file cannot own frozen parent")
            .code(),
        "PID-WRITE-PATH"
    );

    let current_output = scratch
        .path()
        .join("assurance/product-identity/v1/naming-inventory.tsv");
    let current_output_bytes = fs::read(&current_output).expect("saved current output");
    fs::remove_file(&current_output).expect("remove current output");
    fs::create_dir(&current_output).expect("directory at current output");
    assert_eq!(
        product_identity::generate_current_inventory(scratch.path())
            .expect_err("directory current output")
            .code(),
        "PID-WRITE-INVENTORY"
    );
    fs::remove_dir(&current_output).expect("remove current output directory");
    fs::write(&current_output, current_output_bytes).expect("restore current output");

    let corpus_path = scratch
        .path()
        .join("assurance/product-identity/v1/pre-rename-corpus.tsv");
    let corpus_bytes = fs::read(&corpus_path).expect("saved corpus");
    fs::remove_file(&corpus_path).expect("remove corpus");
    fs::create_dir(&corpus_path).expect("directory at corpus output");
    assert_eq!(
        product_identity::freeze_pre_rename(scratch.path())
            .expect_err("directory corpus output")
            .code(),
        "PID-WRITE-CORPUS"
    );
    fs::remove_dir(&corpus_path).expect("remove corpus output directory");
    fs::write(&corpus_path, &corpus_bytes).expect("restore corpus after write control");
    fs::remove_file(&corpus_path).expect("remove corpus");
    assert_eq!(
        product_identity::run_cli(&[String::from("--check-corpus")], scratch.path())
            .expect_err("missing corpus")
            .code(),
        "PID-CORPUS-READ"
    );
    fs::write(&corpus_path, &corpus_bytes).expect("restore corpus");

    let first_corpus_source = corpus
        .lines()
        .nth(1)
        .and_then(|line| line.split('\t').nth(3))
        .expect("first corpus source");
    let first_corpus_path = scratch.path().join(first_corpus_source);
    let first_corpus_bytes = fs::read(&first_corpus_path).expect("saved corpus source");
    fs::remove_file(&first_corpus_path).expect("remove corpus source");
    assert_eq!(
        product_identity::run_cli(&[String::from("--check-corpus")], scratch.path())
            .expect_err("missing corpus source")
            .code(),
        "PID-CORPUS-READ"
    );
    fs::write(&first_corpus_path, first_corpus_bytes).expect("restore corpus source");

    let frozen_path = scratch
        .path()
        .join("fixtures/product-identity/v1/frozen-identifier-mutations.tsv");
    let frozen_bytes = fs::read(&frozen_path).expect("saved frozen manifest");
    fs::remove_file(&frozen_path).expect("remove frozen manifest");
    assert_eq!(
        product_identity::run_cli(&[String::from("--check-corpus")], scratch.path())
            .expect_err("missing frozen manifest")
            .code(),
        "PID-FROZEN-READ"
    );
    fs::write(&frozen_path, frozen_bytes).expect("restore frozen manifest");

    let pre_path = scratch
        .path()
        .join("assurance/product-identity/v1/pre-rename-occurrences.tsv");
    let pre_text = fs::read_to_string(&pre_path).expect("saved pre-rename inventory");
    fs::remove_file(&pre_path).expect("remove pre-rename inventory");
    fs::create_dir(&pre_path).expect("directory at frozen inventory output");
    assert_eq!(
        product_identity::freeze_pre_rename(scratch.path())
            .expect_err("directory frozen inventory output")
            .code(),
        "PID-WRITE-INVENTORY"
    );
    fs::remove_dir(&pre_path).expect("remove frozen inventory output directory");
    fs::write(&pre_path, &pre_text).expect("restore frozen inventory after write control");
    fs::remove_file(&pre_path).expect("remove pre-rename inventory");
    assert_eq!(
        product_identity::check(scratch.path())
            .expect_err("missing pre-rename inventory")
            .code(),
        "PID-INVENTORY-READ"
    );
    fs::write(
        &pre_path,
        "schema\tid\tpath\tline\tcolumn\tform\tlexeme\tcontext_digest\tdisposition\trationale\towner\trule\n",
    )
    .expect("empty pre-rename inventory");
    assert_eq!(
        product_identity::check(scratch.path())
            .expect_err("empty pre-rename inventory")
            .code(),
        "PID-INVENTORY-MISSING"
    );
    fs::write(&pre_path, "invalid\n").expect("malformed pre-rename inventory");
    assert_eq!(
        product_identity::check(scratch.path())
            .expect_err("malformed pre-rename inventory")
            .code(),
        "PID-INVENTORY-SCHEMA"
    );
    fs::write(&pre_path, pre_text).expect("restore pre-rename inventory");

    let current_path = scratch
        .path()
        .join("assurance/product-identity/v1/naming-inventory.tsv");
    let current_text = fs::read_to_string(&current_path).expect("saved current inventory");
    fs::remove_file(&current_path).expect("remove current inventory");
    assert_eq!(
        product_identity::check(scratch.path())
            .expect_err("missing current inventory")
            .code(),
        "PID-INVENTORY-READ"
    );
    fs::write(&current_path, "invalid\n").expect("malformed current inventory");
    assert_eq!(
        product_identity::check(scratch.path())
            .expect_err("malformed current inventory")
            .code(),
        "PID-INVENTORY-SCHEMA"
    );
    fs::write(
        &current_path,
        current_text.replacen("frozen-v1-identifier", "rename-product", 1),
    )
    .expect("stale current identity");
    assert_eq!(
        product_identity::check(scratch.path())
            .expect_err("stale product identity")
            .code(),
        "PID-STALE-PRODUCT-IDENTITY"
    );
    fs::write(
        &current_path,
        current_text.replacen(
            "versioned protocol or canonical identifier is byte-frozen",
            "changed rationale",
            1,
        ),
    )
    .expect("stale current inventory");
    assert_eq!(
        product_identity::check(scratch.path())
            .expect_err("stale current inventory")
            .code(),
        "PID-INVENTORY-STALE"
    );
    fs::write(&current_path, current_text).expect("restore current inventory");

    let source_inventory_path = scratch.path().join("assurance/v1/source-inventory.tsv");
    let source_inventory_text =
        fs::read_to_string(&source_inventory_path).expect("synthetic source inventory");
    fs::write(
        &source_inventory_path,
        source_inventory_text.replace(
            "crates/buoy-core/src/algorithms.rs",
            "missing-required-source",
        ),
    )
    .expect("missing source binding");
    product_identity::generate_current_inventory(scratch.path())
        .expect("inventory with missing source binding");
    assert_eq!(
        product_identity::check(scratch.path())
            .expect_err("missing source evidence binding")
            .code(),
        "PID-EVIDENCE-SOURCE-MISSING"
    );
    fs::write(&source_inventory_path, source_inventory_text)
        .expect("restore synthetic source inventory");
    product_identity::generate_current_inventory(scratch.path())
        .expect("inventory with restored source binding");

    for (relative, needle, replacement) in [
        (
            "proofs/core/proof-record.toml",
            "path = \"crates/buoy-core/src/algorithms.rs\"",
            "path = \"missing-proof-source\"",
        ),
        (
            "proofs/core/derivation-record.toml",
            "command = \"devenv shell -- buoy-core-proofs\"",
            "command = \"missing-proof-command\"",
        ),
    ] {
        let path = scratch.path().join(relative);
        let text = fs::read_to_string(&path).expect("saved proof binding");
        fs::write(&path, text.replace(needle, replacement)).expect("missing proof binding");
        product_identity::generate_current_inventory(scratch.path())
            .expect("inventory with missing proof binding");
        assert_eq!(
            product_identity::check(scratch.path())
                .expect_err("missing proof binding")
                .code(),
            "PID-EVIDENCE-PROOF-STALE"
        );
        fs::write(&path, text).expect("restore proof binding");
        product_identity::generate_current_inventory(scratch.path())
            .expect("inventory with restored proof binding");
    }

    let fixture_manifest_path = scratch
        .path()
        .join("fixtures/product-identity/v1/manifest.tsv");
    let fixture_manifest_text =
        fs::read_to_string(&fixture_manifest_path).expect("saved fixture manifest");
    fs::write(&fixture_manifest_path, "invalid\n").expect("malformed fixture manifest");
    product_identity::generate_current_inventory(scratch.path())
        .expect("inventory with malformed fixture manifest");
    assert_eq!(
        product_identity::check(scratch.path())
            .expect_err("malformed fixture manifest")
            .code(),
        "PID-FIXTURE-SCHEMA"
    );
    fs::write(&fixture_manifest_path, fixture_manifest_text).expect("restore fixture manifest");
    product_identity::generate_current_inventory(scratch.path())
        .expect("inventory with restored fixture manifest");

    for relative in [
        "fixtures/product-identity/v1/manifest.tsv",
        "openspec/changes/archive/2026-07-19-add-cargo-llvm-cov-hegel-rs-testing-requirements/design.md",
        "openspec/changes/archive/2026-07-19-add-cargo-llvm-cov-hegel-rs-testing-requirements/tasks.md",
        "openspec/changes/archive/2026-08-02-harden-spec-provenance-production-readiness/tasks.md",
        "openspec/changes/archive/2026-08-28-integrate-spec-provenance-with-chelis/design.md",
        "assurance/v1/source-inventory.tsv",
        "assurance/v1/property-runs.tsv",
        "proofs/core/proof-record.toml",
        "proofs/core/derivation-record.toml",
        "assurance/product-identity/v1/evidence-ledger.tsv",
        "assurance/product-identity/v1/coverage-replay.toml",
    ] {
        let path = scratch.path().join(relative);
        let bytes = fs::read(&path).expect("saved staged check input");
        fs::remove_file(&path).expect("remove staged check input");
        product_identity::generate_current_inventory(scratch.path())
            .expect("inventory without staged input");
        let expected = if relative == "fixtures/product-identity/v1/manifest.tsv" {
            "PID-FIXTURE-READ"
        } else if relative.starts_with("openspec/") {
            "PID-PLANNING-READ"
        } else {
            "PID-EVIDENCE-READ"
        };
        assert_eq!(
            product_identity::check(scratch.path())
                .expect_err("missing staged check input")
                .code(),
            expected,
            "{relative}"
        );
        fs::write(path, bytes).expect("restore staged check input");
        product_identity::generate_current_inventory(scratch.path())
            .expect("inventory with restored staged input");
    }

    let outside = TempDir::new().expect("non-repository control root");
    for path in corpus
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').nth(3))
    {
        let target = outside.path().join(path);
        fs::create_dir_all(target.parent().expect("outside corpus parent"))
            .expect("outside corpus directory");
        fs::copy(path, target).expect("outside corpus source");
    }
    for path in [
        "fixtures/product-identity/v1/manifest.tsv",
        "fixtures/product-identity/v1/frozen-identifier-mutations.tsv",
        "fixtures/product-identity/v1/positive/naming-inventory.tsv",
    ] {
        let target = outside.path().join(path);
        fs::create_dir_all(target.parent().expect("outside fixture parent"))
            .expect("outside fixture directory");
        fs::copy(path, target).expect("outside fixture source");
    }
    for path in frozen
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').nth(3))
    {
        let target = outside.path().join(path);
        fs::create_dir_all(target.parent().expect("outside frozen parent"))
            .expect("outside frozen directory");
        fs::copy(path, target).expect("outside frozen source");
    }
    assert_eq!(
        product_identity::self_test(outside.path())
            .expect_err("self-test outside Git")
            .code(),
        "PID-SCAN-GIT"
    );
    assert_eq!(
        product_identity::freeze_pre_rename(outside.path())
            .expect_err("freeze outside Git")
            .code(),
        "PID-SCAN-GIT"
    );
    let outside_pre = outside
        .path()
        .join("assurance/product-identity/v1/pre-rename-occurrences.tsv");
    fs::create_dir_all(outside_pre.parent().expect("outside inventory parent"))
        .expect("outside inventory directory");
    fs::copy(
        "assurance/product-identity/v1/pre-rename-occurrences.tsv",
        &outside_pre,
    )
    .expect("outside pre-rename inventory");
    fs::write(
        outside
            .path()
            .join("assurance/product-identity/v1/naming-inventory.tsv"),
        "schema\tid\tpath\tline\tcolumn\tform\tlexeme\tcontext_digest\tdisposition\trationale\towner\trule\n",
    )
    .expect("outside current inventory");
    assert_eq!(
        product_identity::check(outside.path())
            .expect_err("check outside Git")
            .code(),
        "PID-SCAN-GIT"
    );
    assert_eq!(
        product_identity::generate_current_inventory(outside.path())
            .expect_err("generate outside Git")
            .code(),
        "PID-SCAN-GIT"
    );
}

#[cfg(feature = "coverage-controls")]
#[test]
fn product_identity_planning_controls_reject_each_substitution() {
    let focused_design = fs::read_to_string(
        "openspec/changes/archive/2026-07-19-add-cargo-llvm-cov-hegel-rs-testing-requirements/design.md",
    )
    .expect("focused design");
    let focused_tasks = fs::read_to_string(
        "openspec/changes/archive/2026-07-19-add-cargo-llvm-cov-hegel-rs-testing-requirements/tasks.md",
    )
    .expect("focused tasks");
    let hardening_tasks = fs::read_to_string(
        "openspec/changes/archive/2026-08-02-harden-spec-provenance-production-readiness/tasks.md",
    )
    .expect("hardening tasks");
    let chelis_design = fs::read_to_string(
        "openspec/changes/archive/2026-08-28-integrate-spec-provenance-with-chelis/design.md",
    )
    .expect("Chelis design");
    assert!(
        validate_planning_control(
            &focused_design,
            &focused_tasks,
            &hardening_tasks,
            &chelis_design,
        )
        .is_ok()
    );
    for stale in [
        "Spec Provenance",
        "spec-provenance:coverage",
        "spec-provenance:properties",
        "spec-provenance-core",
        "spec_provenance_core",
    ] {
        assert_eq!(
            validate_planning_control(
                &format!("{stale}\n{focused_design}"),
                &focused_tasks,
                &hardening_tasks,
                &chelis_design,
            )
            .expect_err("old planning identity")
            .code(),
            "PID-PLANNING-OLD-PRIMARY"
        );
    }
    assert_eq!(
        validate_planning_control("", &focused_tasks, &hardening_tasks, &chelis_design)
            .expect_err("missing focused mapping")
            .code(),
        "PID-PLANNING-MAPPING-MISSING"
    );
    assert_eq!(
        validate_planning_control(
            &focused_design,
            &focused_tasks,
            &hardening_tasks.replace("maps to one P0 ledger entry", "missing mapping"),
            &chelis_design,
        )
        .expect_err("missing hardening mapping")
        .code(),
        "PID-PLANNING-MAPPING-MISSING"
    );
    for task in ["2.10", "2.19", "2.21", "2.22"] {
        let missing = hardening_tasks
            .replace(&format!("- [ ] {task} "), "- [ ] CONTROL ")
            .replace(&format!("- [x] {task} "), "- [x] CONTROL ");
        assert_eq!(
            validate_planning_control(&focused_design, &focused_tasks, &missing, &chelis_design,)
                .expect_err("missing hardening task")
                .code(),
            "PID-PLANNING-TASK-MISSING"
        );
    }
    let incomplete_focused = focused_tasks.replace("- [x] 6.5 ", "- [ ] 6.5 ");
    assert_eq!(
        validate_planning_control(
            &focused_design,
            &incomplete_focused,
            &hardening_tasks,
            &chelis_design,
        )
        .expect_err("focused completion must precede mapped hardening completion")
        .code(),
        "PID-PLANNING-FALSE-COMPLETION"
    );
    for (tasks, chelis) in [
        (
            focused_tasks.replace("devenv tasks run buoy:coverage", "missing-coverage"),
            chelis_design.clone(),
        ),
        (
            focused_tasks.replace("devenv tasks run buoy:properties", "missing-properties"),
            chelis_design.clone(),
        ),
        (
            focused_tasks.clone(),
            chelis_design.replace("published Buoy shell/core interfaces", "missing interface"),
        ),
    ] {
        assert_eq!(
            validate_planning_control(&focused_design, &tasks, &hardening_tasks, &chelis)
                .expect_err("missing planning target")
                .code(),
            "PID-PLANNING-BUOY-TARGET-MISSING"
        );
    }
}

#[test]
fn hashing_policy_assurance_paths_execute_inside_the_instrumented_harness() {
    let _scan_guard = REPOSITORY_SCAN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let source = fs::read_to_string("fixtures/xxh3/v2/policy-cases.tsv")
        .expect("policy fixture manifest must read");
    for line in source.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        let fixture = fs::read_to_string(fields[4]).expect("policy fixture must read");
        let findings = evaluate_hashing_policy_fixture(fields[3], fields[4], &fixture)
            .expect("policy fixture must parse");
        let expected = fields[6];
        if expected == "-" {
            assert!(findings.is_empty());
        } else {
            assert_eq!(
                findings
                    .iter()
                    .map(|finding| finding.code)
                    .collect::<BTreeSet<_>>(),
                expected.split(',').collect::<BTreeSet<_>>()
            );
        }
    }
    let wrong_adapter = evaluate_hashing_policy_fixture(
        "core",
        "crates/buoy-core/src/value/hash/xxh3_128.rs",
        "use sha2::Digest;",
    )
    .expect("wrong-adapter control must parse");
    assert_eq!(wrong_adapter.len(), 1);
    assert_eq!(wrong_adapter[0].code, "DIG-HASH-WRONG-ADAPTER");

    for (role, source) in [
        ("core", "not valid Rust"),
        ("rust", "not valid Rust"),
        ("manifest", "not = [valid"),
        ("transitive-features", "not-json"),
        ("unknown", "valid"),
    ] {
        assert!(evaluate_hashing_policy_fixture(role, "control", source).is_err());
    }
    assert!(inspect_resolved_dependency_features("not-json").is_err());
    assert!(inspect_resolved_dependency_features("{}").is_err());
    assert!(
        inspect_resolved_dependency_features(
            r#"{
                "packages": [
                    {},
                    {"id": 1, "name": "invalid-id"},
                    {"id": "missing-name"},
                    {"id": "invalid-name", "name": 1},
                    {"id": "sha2-id", "name": "sha2"},
                    {"id": "buoy-id", "name": "buoy"},
                    {"id": "other-id", "name": "other"}
                ],
                "resolve": {
                    "nodes": [
                        {"id": "sha2-id", "features": ["sha2"]},
                        {"id": "buoy-id", "features": ["sha-256", "sha2-extra"]},
                        {"id": "other-id", "features": ["blake3-extra"]},
                        {"name": "blake3", "features": ["blake3"]},
                        {"name": "featureless"}
                    ]
                }
            }"#,
        )
        .expect("resolved metadata control must parse")
        .len()
            >= 2
    );
    let scan_root = TempDir::new().expect("hashing scan root");
    let write_scan = |path: &str, source: &str| {
        let target = scan_root.path().join(path);
        fs::create_dir_all(target.parent().expect("scan parent")).expect("scan directory");
        fs::write(target, source).expect("scan source");
    };
    write_scan(
        "src/control.rs",
        "fn control() { let _ = sha2::Sha256::default(); }\n",
    );
    write_scan(
        "Cargo.toml",
        "[package]\nname = \"control\"\nversion = \"0.0.0\"\n[dependencies]\nsha2 = \"1\"\n",
    );
    write_scan(
        "docs/control.md",
        "SHA-256 is the active Buoy identity\nDigest equality establishes authenticity.\n",
    );
    write_scan("active.toml", "digest = \"sha256:control\"\n");
    write_scan("active.json", "{\"digest\":\"blake3:control\"}\n");
    for path in [
        "docs/schema/v1/history.md",
        "docs/schema/v1/history.toml",
        "schema/v1/history.toml",
        "assurance/v1/history.tsv",
        "fixtures/hardening/v1/history.txt",
        "openspec/changes/archive/history.md",
        "fixtures/xxh3/v2/policy/negative/history.toml",
        "fixtures/xxh3/v2/external/history.toml",
        "fixtures/xxh3/v2/malformed-digest-vectors.tsv",
        "fixtures/xxh3/v2/migration/complete-v1-input.toml",
    ] {
        write_scan(path, "digest = \"sha256:historical\"\n");
    }
    for ignored in [
        ".git",
        ".devenv",
        ".worktrees",
        "target",
        "result",
        "node_modules",
    ] {
        write_scan(&format!("{ignored}/ignored.rs"), "fn sha256() {}\n");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink("missing-target", scan_root.path().join("dangling"))
        .expect("dangling scanner control");
    let scan_findings = scan_hashing_policy(scan_root.path()).expect("synthetic hashing scan");
    assert!(
        scan_findings
            .iter()
            .any(|finding| finding.role == "rust-source")
    );
    assert!(
        scan_findings
            .iter()
            .any(|finding| finding.role == "manifest")
    );
    assert!(
        scan_findings
            .iter()
            .any(|finding| finding.role == "documentation")
    );
    assert!(
        scan_findings
            .iter()
            .any(|finding| finding.code == "DIG-UNSUPPORTED-ASSURANCE-CLAIM")
    );
    assert!(
        scan_findings
            .iter()
            .any(|finding| finding.detail == "sha256:")
    );
    assert!(
        scan_findings
            .iter()
            .any(|finding| finding.detail == "blake3:")
    );
    assert!(scan_hashing_policy(&scan_root.path().join("missing")).is_err());
    write_scan("src/control.rs", "not valid Rust");
    assert!(scan_hashing_policy(scan_root.path()).is_err());
    write_scan("src/control.rs", "fn control() {}\n");
    write_scan("Cargo.toml", "not = [valid");
    assert!(scan_hashing_policy(scan_root.path()).is_err());
    write_scan(
        "Cargo.toml",
        "[package]\nname = \"control\"\nversion = \"0.0.0\"\n",
    );
    #[cfg(feature = "coverage-controls")]
    for fault in [
        "collect-entry",
        "nested-entry",
        "rust-read",
        "manifest-read",
        "markdown-read",
        "structured-read",
    ] {
        assert!(
            scan_hashing_policy_fault_control(scan_root.path(), fault).is_err(),
            "{fault}"
        );
    }
    assert!(
        inspect_resolved_dependency_features(
            r#"{"packages":[{"name":"hidden","features":["sha2","blake3"]}]}"#
        )
        .expect("metadata control must parse")
        .len()
            >= 2
    );
    assert!(
        scan_hashing_policy(Path::new("."))
            .expect("repository hashing policy must scan")
            .is_empty()
    );
}

fn run_binary(path: &str, arguments: &[&str]) -> std::process::Output {
    Command::new(path)
        .args(arguments)
        .output()
        .expect("assurance binary must execute")
}

#[test]
fn assurance_binary_entry_points_execute_success_and_usage_paths() {
    let _scan_guard = REPOSITORY_SCAN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let product = env!("CARGO_BIN_EXE_buoy-product-identity");
    assert!(run_binary(product, &["--self-test"]).status.success());
    assert!(!run_binary(product, &["--invalid"]).status.success());

    let hashing = env!("CARGO_BIN_EXE_buoy-hashing-policy");
    assert!(run_binary(hashing, &["--self-test"]).status.success());
    assert!(run_binary(hashing, &[]).status.success());
    assert!(!run_binary(hashing, &["--invalid"]).status.success());
    for control in [
        "--finding-control",
        "--error-control",
        "--self-test-error-control",
    ] {
        assert!(!run_binary(hashing, &[control]).status.success());
    }

    let migrate = env!("CARGO_BIN_EXE_buoy-migrate");
    assert!(
        run_binary(
            migrate,
            &[
                "leaves",
                "fixtures/xxh3/v2/migration/complete-v1-input.toml",
            ]
        )
        .status
        .success()
    );
    assert!(
        run_binary(
            migrate,
            &[
                "complete",
                "fixtures/xxh3/v2/migration/complete-v1-input.toml"
            ]
        )
        .status
        .success()
    );
    assert!(!run_binary(migrate, &[]).status.success());
    assert!(
        !run_binary(migrate, &["leaves", "missing-migration-input"])
            .status
            .success()
    );
    let malformed = TempDir::new().expect("malformed migration root");
    let malformed_path = malformed.path().join("input.toml");
    fs::write(&malformed_path, "invalid").expect("malformed migration input");
    let malformed_text = malformed_path.to_string_lossy();
    assert!(
        !run_binary(migrate, &["leaves", &malformed_text])
            .status
            .success()
    );
    assert!(
        !run_binary(migrate, &["complete", &malformed_text])
            .status
            .success()
    );
    assert!(
        !run_binary(migrate, &["unknown", "Cargo.toml"])
            .status
            .success()
    );
}
