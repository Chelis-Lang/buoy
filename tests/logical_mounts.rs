//! Requirement controls for logical source mounts and joined snapshots.

#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy::check::check_snapshot;
use buoy::core::{
    LogicalMount, LogicalMountError, LogicalMountId, RepositoryPath, SemanticDigest,
    canonical_logical_mounts, mount_relative_path, project_logical_path,
};
use buoy::repository::{
    MountMappings, RepositoryConfig, RepositoryError, snapshot_mounted_repository,
    snapshot_repository,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const FIXTURE: &str = "fixtures/logical-mount-artifact-bindings/v1";

fn config(name: &str) -> RepositoryConfig {
    RepositoryConfig::parse(
        &fs::read(Path::new(FIXTURE).join("configs").join(name)).expect("configuration fixture"),
    )
    .expect("valid logical mount configuration")
}

fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture file parent")).expect("fixture directory");
    fs::write(path, bytes).expect("fixture file");
}

fn mapping(rows: &[(&str, &Path)]) -> MountMappings {
    MountMappings::new(
        rows.iter()
            .map(|(id, root)| {
                (
                    LogicalMountId::new(id).expect("fixture mount ID"),
                    root.to_path_buf(),
                )
            })
            .collect(),
    )
    .expect("complete fixture mappings")
}

fn two_roots() -> (TempDir, TempDir) {
    let project = TempDir::new().expect("project root");
    let authority = TempDir::new().expect("authority root");
    write(project.path(), "src/control.bin", b"project");
    write(
        authority.path(),
        "openspec/specs/demo/spec.md",
        b"# Demo\n\n## Requirements\n\n### Requirement: OSA-900 Preserve bytes\nThe project MUST preserve bytes.\n\n#### Scenario: Exact\n\n- **GIVEN** bytes\n- **WHEN** a check runs\n- **THEN** bytes remain exact\n",
    );
    (project, authority)
}

#[test]
fn explicit_mount_set() {
    let one = config("one-mount.toml");
    assert!(one.is_mounted());
    assert_eq!(one.mounts().len(), 1);
    assert_eq!(one.mounts()[0].id().as_str(), "project");
    assert_eq!(one.mounts()[0].prefix().as_str(), "project");

    let two = config("two-mounts.toml");
    assert_eq!(
        two.mounts()
            .iter()
            .map(|mount| mount.id().as_str())
            .collect::<Vec<_>>(),
        vec!["authority", "project"]
    );
    assert!(
        !RepositoryConfig::parse(
            &fs::read(Path::new(FIXTURE).join("configs/v2-compatibility.toml"))
                .expect("version 2 fixture"),
        )
        .expect("version 2 compatibility")
        .is_mounted()
    );
}

#[test]
fn version_two_selector_identity_and_directory_semantics_are_unchanged() {
    let base = fs::read_to_string(Path::new(FIXTURE).join("configs/v2-compatibility.toml"))
        .expect("version 2 fixture");
    let combining_acute = char::from_u32(0x301).expect("combining acute accent");
    let decomposed_root = format!("root = \"cafe{combining_acute}\"");
    assert_eq!(
        RepositoryConfig::parse(base.replace("root = \"src\"", &decomposed_root).as_bytes(),),
        Err(RepositoryError::ConfigSchema),
        "version 2 keeps the raw configured root in canonical validation"
    );
    let composed =
        RepositoryConfig::parse(base.replace("root = \"src\"", "root = \"café\"").as_bytes())
            .expect("composed version 2 root");
    let root = TempDir::new().expect("version 2 Unicode root");
    write(root.path(), "café/lib.rs", b"pub fn control() {}\n");
    assert!(snapshot_repository(root.path(), &composed).is_ok());

    let file_root = RepositoryConfig::parse(
        base.replace("root = \"src\"", "root = \"control.rs\"")
            .replace("include = [\"**/*.rs\"]", "include = [\"control.rs\"]")
            .as_bytes(),
    )
    .expect("version 2 file-root configuration");
    fs::write(root.path().join("control.rs"), b"pub fn control() {}\n")
        .expect("version 2 file root");
    assert_eq!(
        snapshot_repository(root.path(), &file_root),
        Err(RepositoryError::SelectorRoot)
    );
}

#[test]
fn physical_path_independence() {
    let first = TempDir::new().expect("first physical root");
    let second = TempDir::new().expect("second physical root");
    write(first.path(), "src/control.bin", b"equal");
    write(second.path(), "src/control.bin", b"equal");
    let config = config("one-mount.toml");
    let first_snapshot =
        snapshot_mounted_repository(&mapping(&[("project", first.path())]), &config)
            .expect("first logical snapshot");
    let second_snapshot =
        snapshot_mounted_repository(&mapping(&[("project", second.path())]), &config)
            .expect("second logical snapshot");
    assert_eq!(first_snapshot, second_snapshot);
    let config_digest = config.semantic_digest().expect("configuration digest");
    let tool = SemanticDigest::xxh3_128([7; 16]);
    assert_eq!(
        first_snapshot
            .semantic_digest_with(config.active_algorithm(), config_digest, tool, &[])
            .expect("first snapshot digest"),
        second_snapshot
            .semantic_digest_with(config.active_algorithm(), config_digest, tool, &[])
            .expect("second snapshot digest")
    );

    let config_path = Path::new(FIXTURE).join("configs/one-mount.toml");
    let run = |root: &Path| {
        Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args([
                "static",
                "--mount",
                &format!("project={}", root.display()),
                "--config",
                config_path.to_str().expect("UTF-8 config path"),
                "--format",
                "machine",
            ])
            .output()
            .expect("mounted static command")
    };
    let first_report = run(first.path());
    let second_report = run(second.path());
    assert!(first_report.status.success() && second_report.status.success());
    assert_eq!(first_report.stdout, second_report.stdout);
    let machine = String::from_utf8(first_report.stdout).expect("machine UTF-8");
    assert!(!machine.contains(&first.path().display().to_string()));
    assert!(!machine.contains(&second.path().display().to_string()));
    let value: serde_json::Value = serde_json::from_str(machine.trim()).expect("machine JSON");
    assert_eq!(value["mounts"][0]["id"], "project");
    assert_eq!(value["mounts"][0]["prefix"], "project");
    assert_eq!(
        value["opaque_subjects"][0]["path"],
        "project/src/control.bin"
    );
}

#[test]
fn closed_namespace() {
    for (name, error) in [
        (
            "negative-duplicate-mount-id.toml",
            RepositoryError::MountIdDuplicate,
        ),
        (
            "negative-overlapping-prefixes.toml",
            RepositoryError::MountPrefixOverlap,
        ),
        (
            "negative-portable-prefix-collision.toml",
            RepositoryError::MountPrefixOverlap,
        ),
        (
            "negative-unknown-selector-mount.toml",
            RepositoryError::SelectorMount,
        ),
    ] {
        assert_eq!(
            RepositoryConfig::parse(
                &fs::read(Path::new(FIXTURE).join("configs").join(name))
                    .expect("negative configuration fixture"),
            ),
            Err(error),
            "{name}"
        );
    }
    let root = TempDir::new().expect("logical collision root");
    write(root.path(), "src/control.bin", b"collision");
    write(root.path(), "buoy/unresolved.toml", b"unresolved");
    let collision = RepositoryConfig::parse(
        &fs::read(Path::new(FIXTURE).join("configs/negative-logical-collision.toml"))
            .expect("logical collision config"),
    )
    .expect("logical collision configuration parses");
    assert_eq!(
        snapshot_mounted_repository(&mapping(&[("project", root.path())]), &collision,),
        Err(RepositoryError::LogicalPathCollision)
    );
}

#[test]
fn joined_snapshot() {
    let (project, authority) = two_roots();
    let config = config("two-mounts.toml");
    let snapshot = snapshot_mounted_repository(
        &mapping(&[("project", project.path()), ("authority", authority.path())]),
        &config,
    )
    .expect("joined logical snapshot");
    assert_eq!(
        snapshot
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "authority/openspec/specs/demo/spec.md",
            "project/src/control.bin",
        ]
    );
}

fn budget_config(total_bytes: u64, selected_files: u64) -> RepositoryConfig {
    let source = format!(
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

[budgets]
path_depth = 64
directory_entries = 100
selected_files = {selected_files}
file_bytes = 100
total_bytes = {total_bytes}
graph_objects = 200000
graph_references = 2000000
diagnostics = 100000
source_nodes = 200000
source_edges = 400000
source_depth = 64
source_visited = 200000
"#,
    );
    RepositoryConfig::parse(source.as_bytes()).expect("budget configuration")
}

#[test]
fn aggregate_budgets() {
    let project = TempDir::new().expect("project root");
    let authority = TempDir::new().expect("authority root");
    write(project.path(), "src/one.bin", b"123");
    write(authority.path(), "src/two.bin", b"4567");
    let mappings = mapping(&[("project", project.path()), ("authority", authority.path())]);
    assert!(snapshot_mounted_repository(&mappings, &budget_config(8, 2)).is_ok());
    assert_eq!(
        snapshot_mounted_repository(&mappings, &budget_config(6, 2)),
        Err(RepositoryError::TotalBytesBudget)
    );
    assert_eq!(
        snapshot_mounted_repository(&mappings, &budget_config(8, 1)),
        Err(RepositoryError::SelectedFilesBudget)
    );
}

#[test]
fn cross_mount_graph() {
    let (project, authority) = two_roots();
    let snapshot = snapshot_mounted_repository(
        &mapping(&[("project", project.path()), ("authority", authority.path())]),
        &config("two-mounts.toml"),
    )
    .expect("cross-mount snapshot");
    assert_eq!(snapshot.files.len(), 2);
    assert!(snapshot.files[0].path.as_str().starts_with("authority/"));
    assert!(snapshot.files[1].path.as_str().starts_with("project/"));
    let checked =
        check_snapshot(&snapshot, &config("two-mounts.toml")).expect("joined source graph");
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    assert!(checked.source.graph().nodes().iter().any(|node| {
        node.item().kind().as_str() == "artifact/file/v1"
            && node.item().path().as_str() == "project/src/control.bin"
    }));
    assert!(checked.source.graph().nodes().iter().any(|node| {
        node.item().kind().as_str() == "openspec/requirement/v1"
            && node
                .item()
                .path()
                .as_str()
                .starts_with("authority/openspec/specs/")
    }));
}

#[test]
fn mount_order_identity() {
    let canonical = config("two-mounts.toml");
    let reordered = config("two-mounts-reordered.toml");
    assert_eq!(
        canonical.semantic_digest().expect("canonical digest"),
        reordered.semantic_digest().expect("reordered digest")
    );
    let (project, authority) = two_roots();
    let roots = mapping(&[("project", project.path()), ("authority", authority.path())]);
    let canonical_snapshot =
        snapshot_mounted_repository(&roots, &canonical).expect("canonical snapshot");
    let reordered_snapshot =
        snapshot_mounted_repository(&roots, &reordered).expect("reordered snapshot");
    assert_eq!(canonical_snapshot, reordered_snapshot);
    let canonical_check = check_snapshot(&canonical_snapshot, &canonical).expect("canonical graph");
    let reordered_check = check_snapshot(&reordered_snapshot, &reordered).expect("reordered graph");
    assert_eq!(canonical_check.graph_digest, reordered_check.graph_digest);
    assert_eq!(canonical_check.diagnostics, reordered_check.diagnostics);
}

#[test]
fn cli_root_and_mount_arguments_are_schema_exclusive() {
    let project = TempDir::new().expect("project root");
    let authority = TempDir::new().expect("authority root");
    let extra = TempDir::new().expect("extra root");
    write(project.path(), "src/control.bin", b"project");
    write(project.path(), "src/lib.rs", b"pub fn control() {}\n");
    write(
        authority.path(),
        "openspec/specs/demo/spec.md",
        b"# Demo\n\n## Requirements\n\n### Requirement: OSA-900 Exact\nThe project MUST preserve bytes.\n",
    );
    let one = Path::new(FIXTURE).join("configs/one-mount.toml");
    let two = Path::new(FIXTURE).join("configs/two-mounts.toml");
    let v2 = Path::new(FIXTURE).join("configs/v2-compatibility.toml");
    let project_mapping = format!("project={}", project.path().display());
    let authority_mapping = format!("authority={}", authority.path().display());
    let extra_mapping = format!("extra={}", extra.path().display());
    let run = |arguments: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args(arguments)
            .output()
            .expect("Buoy CLI")
            .status
            .code()
    };
    assert_eq!(
        run(&[
            "static",
            "--mount",
            &project_mapping,
            "--config",
            one.to_str().expect("UTF-8 config"),
        ]),
        Some(0)
    );
    assert_eq!(
        run(&[
            "static",
            "--root",
            project.path().to_str().expect("UTF-8 root"),
            "--config",
            one.to_str().expect("UTF-8 config"),
        ]),
        Some(70)
    );
    assert_eq!(
        run(&[
            "static",
            "--root",
            project.path().to_str().expect("UTF-8 root"),
            "--mount",
            &project_mapping,
            "--config",
            one.to_str().expect("UTF-8 config"),
        ]),
        Some(70)
    );
    assert_eq!(
        run(&[
            "static",
            "--root",
            project.path().to_str().expect("UTF-8 root"),
            "--config",
            v2.to_str().expect("UTF-8 config"),
        ]),
        Some(0)
    );
    assert_eq!(
        run(&[
            "static",
            "--mount",
            &project_mapping,
            "--config",
            v2.to_str().expect("UTF-8 config"),
        ]),
        Some(70)
    );
    for mappings in [
        vec![project_mapping.as_str()],
        vec![
            project_mapping.as_str(),
            authority_mapping.as_str(),
            extra_mapping.as_str(),
        ],
        vec![project_mapping.as_str(), project_mapping.as_str()],
    ] {
        let mut arguments = vec!["static"];
        for mapping in mappings {
            arguments.extend(["--mount", mapping]);
        }
        arguments.extend(["--config", two.to_str().expect("UTF-8 config")]);
        assert_eq!(run(&arguments), Some(70));
    }
}

#[test]
fn barnacle_fixture_matches_the_read_only_machine_golden() {
    let root = Path::new(FIXTURE).join("barnacle");
    let project = root.join("project");
    let authority = root.join("authority");
    for path in [
        project.join("src/lib.rs"),
        project.join("flake.nix"),
        project.join("README.md"),
        project.join("buoy/objects/README.md"),
        authority.join("openspec/specs/demo/spec.md"),
    ] {
        assert!(path.is_file(), "{}", path.display());
    }
    assert_eq!(
        fs::read_dir(project.join("buoy/objects/v1/blake3-256"))
            .expect("Barnacle object store")
            .count(),
        1
    );
    assert_eq!(
        fs::read_dir(project.join("buoy/attachments/v1/blake3-256"))
            .expect("Barnacle attachment store")
            .count(),
        1
    );
    let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "static",
            "--mount",
            &format!("project={}", project.display()),
            "--mount",
            &format!("authority={}", authority.display()),
            "--config",
            root.join("buoy.toml")
                .to_str()
                .expect("UTF-8 Barnacle config"),
            "--format",
            "machine",
        ])
        .output()
        .expect("Barnacle static command");
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        fs::read(root.join("../goldens/barnacle-static.json"))
            .expect("reviewed Barnacle machine golden")
    );
}

#[test]
fn logical_mount_core_and_error_controls_are_complete() {
    assert_eq!(LogicalMountId::new("Bad"), Err(LogicalMountError::Id));
    let project_id = LogicalMountId::new("project").expect("project mount ID");
    let project = LogicalMount::new(
        project_id.clone(),
        RepositoryPath::new("project").expect("project prefix"),
    );
    assert!(project.canonical_bytes().is_ok());
    let relative = RepositoryPath::new("src/lib.rs").expect("relative path");
    let projected = project_logical_path(&project, &relative).expect("projected path");
    assert_eq!(projected.as_str(), "project/src/lib.rs");
    assert_eq!(
        mount_relative_path(&project, &projected),
        Some(relative.clone())
    );
    assert_eq!(
        mount_relative_path(
            &project,
            &RepositoryPath::new("authority/spec.md").expect("foreign path")
        ),
        None
    );
    assert_eq!(
        canonical_logical_mounts(vec![
            project.clone(),
            LogicalMount::new(
                project_id,
                RepositoryPath::new("other").expect("other prefix"),
            ),
        ]),
        Err(LogicalMountError::DuplicateId)
    );
    assert_eq!(
        canonical_logical_mounts(vec![
            project.clone(),
            LogicalMount::new(
                LogicalMountId::new("nested").expect("nested mount ID"),
                RepositoryPath::new("project/specs").expect("nested prefix"),
            ),
        ]),
        Err(LogicalMountError::PrefixOverlap)
    );
    #[cfg(feature = "coverage-controls")]
    {
        let invalid_mount = LogicalMount::new(
            LogicalMountId::new("invalid").expect("invalid control ID"),
            RepositoryPath::reviewed_control("/"),
        );
        assert_eq!(
            project_logical_path(&invalid_mount, &relative),
            Err(LogicalMountError::Projection)
        );
    }
    for (error, code) in [
        (LogicalMountError::Id, "LSM-MOUNT-ID"),
        (LogicalMountError::DuplicateId, "LSM-MOUNT-ID-DUPLICATE"),
        (LogicalMountError::PrefixOverlap, "LSM-MOUNT-PREFIX-OVERLAP"),
        (LogicalMountError::Projection, "LSM-LOGICAL-PATH"),
    ] {
        assert_eq!(error.code(), code);
    }
    for (error, code) in [
        (RepositoryError::MountId, "LSM-MOUNT-ID"),
        (RepositoryError::MountIdDuplicate, "LSM-MOUNT-ID-DUPLICATE"),
        (RepositoryError::MountPrefix, "LSM-MOUNT-PREFIX"),
        (
            RepositoryError::MountPrefixOverlap,
            "LSM-MOUNT-PREFIX-OVERLAP",
        ),
        (RepositoryError::SelectorMount, "LSM-SELECTOR-MOUNT"),
        (RepositoryError::MountMapping, "LSM-MOUNT-MAPPING"),
        (
            RepositoryError::MountMappingDuplicate,
            "LSM-MOUNT-MAPPING-DUPLICATE",
        ),
        (RepositoryError::MountMissing, "LSM-MOUNT-MISSING"),
        (RepositoryError::MountExtra, "LSM-MOUNT-EXTRA"),
        (RepositoryError::RootMountMixed, "LSM-ROOT-MOUNT-MIXED"),
        (RepositoryError::MountRoot, "REH-MOUNT-ROOT"),
        (
            RepositoryError::LogicalPathCollision,
            "LSM-LOGICAL-PATH-COLLISION",
        ),
        (RepositoryError::ArtifactAuthority, "RAS-ARTIFACT-AUTHORITY"),
        (
            RepositoryError::ArtifactConfiguration,
            "RAS-ARTIFACT-CONFIGURATION",
        ),
        (RepositoryError::OracleMount, "REH-ORACLE-MOUNT"),
    ] {
        assert_eq!(error.code(), code);
    }
    assert_eq!(
        MountMappings::parse(&[String::from("malformed")]),
        Err(RepositoryError::MountMapping)
    );
    let v2 = config("v2-compatibility.toml");
    assert_eq!(
        MountMappings::new(Vec::new())
            .expect("empty mappings")
            .validate(&v2),
        Err(RepositoryError::RootMountMixed)
    );
}

#[test]
fn physical_mapping_set_is_exact() {
    let project = TempDir::new().expect("project root");
    let authority = TempDir::new().expect("authority root");
    let extra = TempDir::new().expect("extra root");
    let config = config("two-mounts.toml");
    assert_eq!(
        mapping(&[("project", project.path())]).validate(&config),
        Err(RepositoryError::MountMissing)
    );
    assert_eq!(
        mapping(&[
            ("project", project.path()),
            ("authority", authority.path()),
            ("extra", extra.path()),
        ])
        .validate(&config),
        Err(RepositoryError::MountExtra)
    );
    assert_eq!(
        MountMappings::parse(&[
            format!("project={}", project.path().display()),
            format!("project={}", authority.path().display()),
        ]),
        Err(RepositoryError::MountMappingDuplicate)
    );
}
