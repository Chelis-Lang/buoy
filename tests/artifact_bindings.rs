//! Requirement controls for opaque artifact nodes and sidecar bindings.

#![allow(clippy::expect_used, clippy::too_many_lines)]

use buoy::adapter_sdk::SourceProduct;
use buoy::adapters::{
    binding_attachment_path, binding_object_path, canonical_binding_attachment,
    canonical_binding_record, extract_artifact,
};
use buoy::check::{CheckResult, aggregate_source_products, check_snapshot};
use buoy::core::{
    ActiveAlgorithm, ArtifactAttachmentError, ArtifactBindingAttachment, AtomRevisionRef,
    CanonicalValue, DigestAlgorithm, ImplementationBinding, ImplementationBindingRef,
    LogicalMountId, NavigationKind, ParsedItemIdentity, RepositoryPath, SchemaId, SemanticDigest,
    SourceItemKind, SourceNode, SourceNodeRef, StableId, validate_artifact_attachment,
};
use buoy::repository::{
    MountMappings, RepositoryConfig, RepositoryError, snapshot_mounted_repository,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const FIXTURE: &str = "fixtures/logical-mount-artifact-bindings/v1";
const ARTIFACT_PATH: &str = "project/src/control.bin";

fn configuration(name: &str) -> Result<RepositoryConfig, RepositoryError> {
    RepositoryConfig::parse(
        &fs::read(Path::new(FIXTURE).join("configs").join(name)).expect("configuration fixture"),
    )
}

fn repository(bytes: &[u8]) -> (TempDir, RepositoryConfig, MountMappings) {
    let root = TempDir::new().expect("artifact mount root");
    fs::create_dir_all(root.path().join("src")).expect("artifact source directory");
    fs::write(root.path().join("src/control.bin"), bytes).expect("artifact bytes");
    let config = configuration("one-mount.toml").expect("artifact configuration");
    let mappings = MountMappings::new(vec![(
        LogicalMountId::new("project").expect("artifact mount ID"),
        root.path().to_path_buf(),
    )])
    .expect("artifact mount mapping");
    (root, config, mappings)
}

fn artifact_node(bytes: &[u8]) -> buoy::core::SourceNode {
    let (_root, config, mappings) = repository(bytes);
    let snapshot = snapshot_mounted_repository(&mappings, &config).expect("artifact snapshot");
    let checked = check_snapshot(&snapshot, &config).expect("artifact check");
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    checked
        .source
        .graph()
        .nodes()
        .first()
        .expect("one artifact node")
        .clone()
}

#[test]
fn opaque_source_node() {
    for bytes in [b"text\n".to_vec(), vec![0xff, 0x00, 0xfe], Vec::new()] {
        let node = artifact_node(&bytes);
        assert_eq!(node.item().path().as_str(), ARTIFACT_PATH);
        assert_eq!(node.item().kind().as_str(), "artifact/file/v1");
        assert_eq!(node.children(), &[]);
    }
    assert_ne!(
        artifact_node(b"before").digest(),
        artifact_node(b"after").digest()
    );
}

#[test]
fn fixed_opaque_interpretation() {
    let bytes = [0xff, 0x00, b'\n'];
    let node = artifact_node(&bytes);
    let expected = CanonicalValue::Record(vec![
        (
            1,
            CanonicalValue::Text(String::from("application/octet-stream")),
        ),
        (2, CanonicalValue::Text(String::from("saved-bytes/v1"))),
        (3, CanonicalValue::Bytes(bytes.to_vec())),
    ])
    .encode()
    .expect("fixed artifact payload");
    assert_eq!(node.item().language_schema_id(), "artifact.opaque-bytes/v1");
    assert_eq!(node.item().semantic_bytes(), expected);
}

#[test]
fn complete_byte_mapping() {
    let bytes = [0xff, 1, 2, 3, 4];
    let (_root, config, mappings) = repository(&bytes);
    let snapshot = snapshot_mounted_repository(&mappings, &config).expect("artifact snapshot");
    let checked = check_snapshot(&snapshot, &config).expect("artifact check");
    let node = checked.source.graph().nodes()[0].reference();
    let span = checked.source.span(node).expect("complete artifact span");
    assert_eq!(span.path.as_str(), ARTIFACT_PATH);
    assert_eq!((span.start, span.end), (0, bytes.len() as u64));
    assert!(checked.source.authorities().is_empty());
    assert!(checked.source.attachments().is_empty());
    assert!(checked.graph.object_meta().is_empty());
}

#[test]
fn forbidden_authority() {
    assert!(configuration("one-mount.toml").is_ok());
    assert_eq!(
        configuration("negative-artifact-authority.toml"),
        Err(RepositoryError::ArtifactAuthority)
    );
    assert_eq!(
        configuration("negative-artifact-configuration.toml"),
        Err(RepositoryError::ArtifactConfiguration)
    );
}

struct BindingRepository {
    project: TempDir,
    authority: TempDir,
    config: RepositoryConfig,
    mappings: MountMappings,
}

impl BindingRepository {
    fn new(bytes: &[u8]) -> Self {
        let project = TempDir::new().expect("binding project root");
        let authority = TempDir::new().expect("binding authority root");
        fs::create_dir_all(project.path().join("buoy")).expect("binding store root");
        fs::write(project.path().join("artifact.bin"), bytes).expect("binding artifact");
        let specification = authority.path().join("openspec/specs/demo/spec.md");
        fs::create_dir_all(specification.parent().expect("specification parent"))
            .expect("specification directory");
        fs::write(
            specification,
            b"# Demo\n\n## Requirements\n\n### Requirement: OSA-900 Preserve bytes\nThe project MUST preserve exact bytes.\n\n#### Scenario: Exact bytes\n\n- **GIVEN** one artifact\n- **WHEN** validation runs\n- **THEN** the bytes remain exact\n",
        )
        .expect("standing specification");
        let config = configuration("artifact-binding.toml").expect("binding configuration");
        let mappings = MountMappings::new(vec![
            (
                LogicalMountId::new("project").expect("project mount ID"),
                project.path().to_path_buf(),
            ),
            (
                LogicalMountId::new("authority").expect("authority mount ID"),
                authority.path().to_path_buf(),
            ),
        ])
        .expect("binding mount mappings");
        Self {
            project,
            authority,
            config,
            mappings,
        }
    }

    fn check(&self) -> CheckResult {
        let snapshot = snapshot_mounted_repository(&self.mappings, &self.config)
            .expect("mounted binding snapshot");
        check_snapshot(&snapshot, &self.config).expect("mounted binding check")
    }

    fn write_binding(
        &self,
        checked: &CheckResult,
        relation: &StableId,
    ) -> (
        ImplementationBinding,
        ArtifactBindingAttachment,
        String,
        String,
    ) {
        let artifact = checked
            .source
            .graph()
            .nodes()
            .iter()
            .find(|node| node.item().kind().as_str() == "artifact/file/v1")
            .expect("current artifact node")
            .reference();
        let requirement = checked
            .source
            .graph()
            .nodes()
            .iter()
            .find(|node| node.item().kind().as_str() == "openspec/requirement/v1")
            .expect("current requirement node")
            .reference();
        let authority = checked
            .source
            .authorities()
            .iter()
            .find(|authority| authority.node() == requirement)
            .expect("current requirement authority");
        let record = ImplementationBinding::new_with(
            self.config.active_algorithm(),
            relation.clone(),
            NavigationKind::Implements,
            artifact,
            requirement,
            authority.atom().clone(),
        )
        .expect("artifact implementation record");
        let attachment = ArtifactBindingAttachment::new_with(
            self.config.active_algorithm(),
            relation.clone(),
            checked
                .source
                .graph()
                .node(artifact)
                .expect("artifact node")
                .item()
                .path()
                .clone(),
            artifact,
            record.record(),
        )
        .expect("artifact attachment");
        let object_path = binding_object_path(record.record());
        let sidecar_path = binding_attachment_path(self.config.active_algorithm(), relation)
            .expect("attachment path");
        write_mount_file(
            self.project.path(),
            &object_path,
            canonical_binding_record(&record).as_bytes(),
        );
        write_mount_file(
            self.project.path(),
            &sidecar_path,
            canonical_binding_attachment(&attachment).as_bytes(),
        );
        (record, attachment, object_path, sidecar_path)
    }
}

fn write_mount_file(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("mounted file parent"))
        .expect("mounted file directory");
    fs::write(path, bytes).expect("mounted file bytes");
}

fn binding_relation() -> StableId {
    StableId::new("ARTIFACT-BINDING-CONTROL").expect("binding relation ID")
}

#[test]
fn artifact_attachment_core_controls_are_complete() {
    let active = ActiveAlgorithm::new(DigestAlgorithm::Blake3_256)
        .expect("BLAKE3 attachment control member");
    let relation = StableId::new("ARTIFACT-CORE-CONTROL").expect("core relation");
    let path = RepositoryPath::new("project/artifact.bin").expect("core artifact path");
    let node = extract_artifact(active, &path, b"artifact").node;
    let requirement = SourceNodeRef::new(SemanticDigest::blake3_256([8; 32]));
    let atom = AtomRevisionRef::new(
        StableId::new("ARTIFACT-CORE-ATOM").expect("core atom ID"),
        SemanticDigest::blake3_256([9; 32]),
    );
    let record = ImplementationBinding::new_with(
        active,
        relation.clone(),
        NavigationKind::Implements,
        node.reference(),
        requirement,
        atom.clone(),
    )
    .expect("core implementation record");
    let attachment = ArtifactBindingAttachment::new_with(
        active,
        relation.clone(),
        path.clone(),
        node.reference(),
        record.record(),
    )
    .expect("core artifact attachment");
    assert!(attachment.canonical_bytes().is_ok());
    assert_eq!(attachment.digest().algorithm(), DigestAlgorithm::Blake3_256);
    assert_eq!(
        validate_artifact_attachment(&attachment, &record, &node),
        Ok(())
    );

    let wrong_record = ImplementationBinding::new_with(
        active,
        StableId::new("ARTIFACT-CORE-OTHER").expect("other relation"),
        NavigationKind::Implements,
        node.reference(),
        requirement,
        atom.clone(),
    )
    .expect("wrong relation record");
    assert_eq!(
        validate_artifact_attachment(&attachment, &wrong_record, &node),
        Err(ArtifactAttachmentError::Record)
    );
    let other_subject = SourceNodeRef::new(SemanticDigest::blake3_256([7; 32]));
    let wrong_subject = ArtifactBindingAttachment::new_with(
        active,
        relation.clone(),
        path.clone(),
        other_subject,
        record.record(),
    )
    .expect("wrong subject attachment");
    assert_eq!(
        validate_artifact_attachment(&wrong_subject, &record, &node),
        Err(ArtifactAttachmentError::Subject)
    );
    let wrong_path = ArtifactBindingAttachment::new_with(
        active,
        relation.clone(),
        RepositoryPath::new("project/other.bin").expect("wrong artifact path"),
        node.reference(),
        record.record(),
    )
    .expect("wrong path attachment");
    assert_eq!(
        validate_artifact_attachment(&wrong_path, &record, &node),
        Err(ArtifactAttachmentError::Path)
    );

    let rust_item = ParsedItemIdentity::new_with(
        active,
        SchemaId::new("rust/v1").expect("Rust schema"),
        path.clone(),
        SourceItemKind::new("rust/item/v1").expect("Rust item kind"),
        b"item".to_vec(),
        None,
    )
    .expect("Rust parsed item");
    let rust_node = SourceNode::new_with(active, rust_item, Vec::new()).expect("Rust node");
    let rust_record = ImplementationBinding::new_with(
        active,
        relation.clone(),
        NavigationKind::Implements,
        rust_node.reference(),
        requirement,
        atom,
    )
    .expect("Rust control record");
    let wrong_kind = ArtifactBindingAttachment::new_with(
        active,
        relation.clone(),
        path,
        rust_node.reference(),
        rust_record.record(),
    )
    .expect("wrong kind attachment");
    assert_eq!(
        validate_artifact_attachment(&wrong_kind, &rust_record, &rust_node),
        Err(ArtifactAttachmentError::Kind)
    );
    assert_eq!(
        ArtifactBindingAttachment::new_with(
            active,
            relation,
            RepositoryPath::new("project/member.bin").expect("member control path"),
            SourceNodeRef::new(SemanticDigest::sha_256([1; 32])),
            ImplementationBindingRef::new(SemanticDigest::blake3_256([2; 32])),
        ),
        Err(ArtifactAttachmentError::DigestMember)
    );
    for (error, code) in [
        (ArtifactAttachmentError::DigestMember, "CAB-DIGEST-MEMBER"),
        (ArtifactAttachmentError::Encoding, "CAB-ATTACHMENT-ENCODING"),
        (ArtifactAttachmentError::Record, "CAB-ATTACHMENT-RECORD"),
        (ArtifactAttachmentError::Subject, "CAB-SUBJECT-ATTACHMENT"),
        (ArtifactAttachmentError::Path, "CAB-ATTACHMENT-PATH"),
        (ArtifactAttachmentError::Kind, "CAB-SUBJECT-KIND"),
    ] {
        assert_eq!(error.code(), code);
    }
}

#[test]
fn sidecar_attachment() {
    let repository = BindingRepository::new(b"artifact-v1");
    let before = repository.check();
    assert!(before.diagnostics.is_empty(), "{:?}", before.diagnostics);
    let relation = binding_relation();
    let (record, attachment, _object_path, sidecar_path) =
        repository.write_binding(&before, &relation);
    let checked = repository.check();
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    assert_eq!(
        checked.source.artifact_attachments(),
        std::slice::from_ref(&attachment)
    );
    assert_eq!(checked.source.attachments().len(), 1);
    assert_eq!(checked.source.store().len(), 1);
    assert_eq!(
        checked.source.fresh_bindings().as_slice(),
        std::slice::from_ref(&record)
    );
    assert_eq!(checked.graph.implementation_bindings, [record]);

    let wrong_relation = StableId::new("ARTIFACT-BINDING-WRONG").expect("wrong-path relation ID");
    let wrong_path = binding_attachment_path(repository.config.active_algorithm(), &wrong_relation)
        .expect("wrong attachment path");
    fs::rename(
        repository.project.path().join(&sidecar_path),
        repository.project.path().join(&wrong_path),
    )
    .expect("move sidecar to wrong relation path");
    let wrong = repository.check();
    assert!(
        wrong
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CAB-ATTACHMENT-PATH"),
        "{:?}",
        wrong.diagnostics
    );
}

#[test]
fn sidecar_negative_controls() {
    let repository = BindingRepository::new(b"artifact-v1");
    let relation = binding_relation();
    let before = repository.check();
    let (_record, attachment, _object_path, sidecar_path) =
        repository.write_binding(&before, &relation);
    let other_relation = StableId::new("ARTIFACT-BINDING-OTHER").expect("other relation ID");
    let (other_record, _other_attachment, _other_object, other_sidecar) =
        repository.write_binding(&before, &other_relation);
    fs::remove_file(repository.project.path().join(other_sidecar))
        .expect("remove other current sidecar");
    let wrong_record = ArtifactBindingAttachment::new_with(
        repository.config.active_algorithm(),
        relation.clone(),
        attachment.artifact().clone(),
        attachment.subject(),
        other_record.record(),
    )
    .expect("wrong-record sidecar value");
    fs::write(
        repository.project.path().join(&sidecar_path),
        canonical_binding_attachment(&wrong_record),
    )
    .expect("wrong-record sidecar");
    let wrong = repository.check();
    assert!(
        wrong
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CAB-ATTACHMENT-RECORD"),
        "{:?}",
        wrong.diagnostics
    );

    let foreign = canonical_binding_attachment(&attachment).replacen(
        "record = \"blake3-256:",
        "record = \"sha-256:",
        1,
    );
    fs::write(repository.project.path().join(&sidecar_path), foreign)
        .expect("foreign-member sidecar");
    let foreign = repository.check();
    assert!(
        foreign
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CAB-DIGEST-MEMBER"),
        "{:?}",
        foreign.diagnostics
    );

    let canonical = canonical_binding_attachment(&attachment);
    for (source, code) in [
        (
            canonical.replace(
                "schema = \"buoy/binding-attachment/v1\"",
                "schema = \"wrong/v1\"",
            ),
            "CAB-ATTACHMENT-SCHEMA",
        ),
        (format!("{canonical}\n"), "CAB-ATTACHMENT-CANONICAL"),
    ] {
        fs::write(repository.project.path().join(&sidecar_path), source)
            .expect("malformed sidecar control");
        let malformed = repository.check();
        assert!(
            malformed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "{code}: {:?}",
            malformed.diagnostics
        );
    }

    let active = repository.config.active_algorithm();
    let duplicate_relation =
        StableId::new("ARTIFACT-BINDING-DUPLICATE").expect("duplicate relation ID");
    let first = ArtifactBindingAttachment::new_with(
        active,
        duplicate_relation.clone(),
        RepositoryPath::new("project/one.bin").expect("first artifact path"),
        SourceNodeRef::new(SemanticDigest::blake3_256([1; 32])),
        ImplementationBindingRef::new(SemanticDigest::blake3_256([2; 32])),
    )
    .expect("first duplicate sidecar");
    let second = ArtifactBindingAttachment::new_with(
        active,
        duplicate_relation,
        RepositoryPath::new("mirror/two.bin").expect("second artifact path"),
        SourceNodeRef::new(SemanticDigest::blake3_256([3; 32])),
        ImplementationBindingRef::new(SemanticDigest::blake3_256([4; 32])),
    )
    .expect("second duplicate sidecar");
    let aggregate = aggregate_source_products(
        active,
        &[
            SourceProduct::from_artifact_attachment(Some(first)),
            SourceProduct::from_artifact_attachment(Some(second)),
        ],
        repository.config.source_budgets(),
    )
    .expect("duplicate sidecar aggregate");
    assert_eq!(
        aggregate
            .binding_diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec!["CAB-ATTACHMENT-DUPLICATE"]
    );
}

#[test]
fn current_sidecar() {
    let repository = BindingRepository::new(b"artifact-v1");
    let relation = binding_relation();
    let before = repository.check();
    let (old_record, _old_attachment, old_path, sidecar_path) =
        repository.write_binding(&before, &relation);
    fs::write(
        repository.project.path().join("artifact.bin"),
        b"artifact-v2",
    )
    .expect("updated artifact bytes");
    let stale = repository.check();
    assert_eq!(
        stale
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code.starts_with("CAB-"))
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec!["CAB-SUBJECT-STALE"]
    );
    let (new_record, _new_attachment, new_path, new_sidecar_path) =
        repository.write_binding(&stale, &relation);
    assert_ne!(old_record.record(), new_record.record());
    assert_ne!(old_path, new_path);
    assert_eq!(sidecar_path, new_sidecar_path);
    let current = repository.check();
    assert!(current.diagnostics.is_empty(), "{:?}", current.diagnostics);
    assert_eq!(current.source.store().len(), 2);
    assert_eq!(current.source.fresh_bindings(), [new_record]);
    assert!(repository.project.path().join(old_path).is_file());
}

#[test]
fn bind_artifact_transaction() {
    let repository = BindingRepository::new(b"transaction-artifact");
    let before_bytes =
        fs::read(repository.project.path().join("artifact.bin")).expect("artifact before bind");
    let checked = repository.check();
    let requirement = checked
        .source
        .graph()
        .nodes()
        .iter()
        .find(|node| node.item().kind().as_str() == "openspec/requirement/v1")
        .expect("binding requirement")
        .reference()
        .canonical_text();
    let config = Path::new(FIXTURE).join("configs/artifact-binding.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "bind",
            "--mount",
            &format!("project={}", repository.project.path().display()),
            "--mount",
            &format!("authority={}", repository.authority.path().display()),
            "--config",
            config.to_str().expect("UTF-8 fixture path"),
            "--requirement",
            &requirement,
            "--artifact",
            "project/artifact.bin",
            "--relation",
            "ARTIFACT-BINDING-CLI",
            "--format",
            "machine",
        ])
        .output()
        .expect("artifact bind command");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(repository.project.path().join("artifact.bin")).expect("artifact after bind"),
        before_bytes
    );
    let after = repository.check();
    assert!(after.diagnostics.is_empty(), "{:?}", after.diagnostics);
    assert_eq!(after.source.fresh_bindings().len(), 1);
    assert_eq!(after.source.artifact_attachments().len(), 1);
    assert_eq!(after.source.store().len(), 1);

    let static_output = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "static",
            "--mount",
            &format!("project={}", repository.project.path().display()),
            "--mount",
            &format!("authority={}", repository.authority.path().display()),
            "--config",
            config.to_str().expect("UTF-8 fixture path"),
            "--format",
            "machine",
        ])
        .output()
        .expect("mounted static report");
    assert!(static_output.status.success());
    let static_text = String::from_utf8(static_output.stdout).expect("static report UTF-8");
    assert!(!static_text.contains(&repository.project.path().display().to_string()));
    assert!(!static_text.contains(&repository.authority.path().display().to_string()));
    let static_report: serde_json::Value =
        serde_json::from_str(static_text.trim()).expect("static report JSON");
    assert_eq!(static_report["artifact_attachments"][0]["fresh"], true);
    assert_eq!(
        static_report["artifact_attachments"][0]["artifact"],
        "project/artifact.bin"
    );

    let exclusive = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "bind",
            "--mount",
            &format!("project={}", repository.project.path().display()),
            "--mount",
            &format!("authority={}", repository.authority.path().display()),
            "--config",
            config.to_str().expect("UTF-8 fixture path"),
            "--requirement",
            &requirement,
            "--artifact",
            "project/artifact.bin",
            "--implementation",
            &requirement,
            "--relation",
            "ARTIFACT-BINDING-INVALID",
        ])
        .output()
        .expect("exclusive selector control");
    assert_eq!(exclusive.status.code(), Some(64));

    for (selected_requirement, artifact) in [
        (
            String::from("not-a-digest"),
            String::from("project/artifact.bin"),
        ),
        (
            format!("sha-256:{}", "0".repeat(64)),
            String::from("project/artifact.bin"),
        ),
        (requirement.clone(), String::from("project/missing.bin")),
        (
            requirement.clone(),
            String::from("authority/openspec/specs/demo/spec.md"),
        ),
        (requirement.clone(), String::from("/absolute")),
    ] {
        let failed = Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args([
                "bind",
                "--mount",
                &format!("project={}", repository.project.path().display()),
                "--mount",
                &format!("authority={}", repository.authority.path().display()),
                "--config",
                config.to_str().expect("UTF-8 fixture path"),
                "--requirement",
                &selected_requirement,
                "--artifact",
                &artifact,
                "--relation",
                "ARTIFACT-BINDING-INVALID",
            ])
            .output()
            .expect("artifact selector failure control");
        assert_eq!(
            failed.status.code(),
            Some(70),
            "requirement={selected_requirement} artifact={artifact}"
        );
    }

    let binding_selector = r#"
[[selectors]]
mount = "project"
root = "buoy"
adapter = "binding/v1"
include = ["**/*.toml"]
exclude = []
authority_expectation = "forbidden"
"#;
    let without_binding = repository.project.path().join("without-binding.toml");
    fs::write(
        &without_binding,
        fs::read_to_string(&config)
            .expect("binding configuration source")
            .replace(binding_selector, ""),
    )
    .expect("configuration without binding selector");
    let unselected_write = Command::new(env!("CARGO_BIN_EXE_buoy"))
        .args([
            "bind",
            "--mount",
            &format!("project={}", repository.project.path().display()),
            "--mount",
            &format!("authority={}", repository.authority.path().display()),
            "--config",
            without_binding.to_str().expect("UTF-8 control config"),
            "--requirement",
            &requirement,
            "--artifact",
            "project/artifact.bin",
            "--relation",
            "ARTIFACT-BINDING-UNSELECTED",
        ])
        .output()
        .expect("unselected write-mount control");
    assert_eq!(unselected_write.status.code(), Some(70));
    assert_eq!(
        fs::read(repository.project.path().join("artifact.bin"))
            .expect("artifact after selector failures"),
        before_bytes
    );
}

#[test]
fn barnacle_artifact_binding_fixture_is_current() {
    let root = Path::new(FIXTURE).join("barnacle");
    let project = root.join("project");
    let authority = root.join("authority");
    let config =
        RepositoryConfig::parse(&fs::read(root.join("buoy.toml")).expect("Barnacle configuration"))
            .expect("Barnacle configuration parse");
    let mappings = MountMappings::new(vec![
        (
            LogicalMountId::new("project").expect("project mount ID"),
            project.clone(),
        ),
        (
            LogicalMountId::new("authority").expect("authority mount ID"),
            authority.clone(),
        ),
    ])
    .expect("Barnacle mappings");
    let initial_snapshot =
        snapshot_mounted_repository(&mappings, &config).expect("Barnacle snapshot");
    let initial = check_snapshot(&initial_snapshot, &config).expect("Barnacle static check");
    let requirement = initial
        .source
        .graph()
        .nodes()
        .iter()
        .find(|node| node.item().kind().as_str() == "openspec/requirement/v1")
        .expect("Barnacle requirement")
        .reference()
        .canonical_text();
    let artifact_before = fs::read(project.join("flake.nix")).expect("Barnacle artifact before");
    if std::env::var("BUOY_WRITE_BINDING_GOLDENS").as_deref() == Ok("1") {
        let output = Command::new(env!("CARGO_BIN_EXE_buoy"))
            .args([
                "bind",
                "--mount",
                &format!("project={}", project.display()),
                "--mount",
                &format!("authority={}", authority.display()),
                "--config",
                root.join("buoy.toml")
                    .to_str()
                    .expect("UTF-8 Barnacle config"),
                "--requirement",
                &requirement,
                "--artifact",
                "project/flake.nix",
                "--relation",
                "BARNACLE-FLAKE-BINDING",
                "--format",
                "machine",
            ])
            .output()
            .expect("Barnacle binding generator");
        assert!(
            output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(
        fs::read(project.join("flake.nix")).expect("Barnacle artifact after"),
        artifact_before
    );
    let snapshot =
        snapshot_mounted_repository(&mappings, &config).expect("bound Barnacle snapshot");
    let checked = check_snapshot(&snapshot, &config).expect("bound Barnacle check");
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    assert_eq!(checked.source.artifact_attachments().len(), 1);
    assert_eq!(checked.source.store().len(), 1);
    assert_eq!(checked.source.fresh_bindings().len(), 1);
}

#[test]
fn artifact_freshness() {
    let repository = BindingRepository::new(b"stable-artifact");
    let relation = binding_relation();
    let before = repository.check();
    let (_record, _attachment, object_path, sidecar_path) =
        repository.write_binding(&before, &relation);
    assert!(repository.check().diagnostics.is_empty());

    let moved = BindingRepository::new(b"stable-artifact");
    write_mount_file(
        moved.project.path(),
        &object_path,
        &fs::read(repository.project.path().join(&object_path)).expect("source object"),
    );
    write_mount_file(
        moved.project.path(),
        &sidecar_path,
        &fs::read(repository.project.path().join(&sidecar_path)).expect("source sidecar"),
    );
    let moved_check = moved.check();
    assert!(
        moved_check.diagnostics.is_empty(),
        "{:?}",
        moved_check.diagnostics
    );
    assert_eq!(moved_check.source.fresh_bindings().len(), 1);

    fs::write(
        moved.project.path().join("artifact.bin"),
        b"drifted-artifact",
    )
    .expect("drifted artifact");
    let drifted = moved.check();
    assert!(
        drifted
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "CAB-SUBJECT-STALE"),
        "{:?}",
        drifted.diagnostics
    );
}
