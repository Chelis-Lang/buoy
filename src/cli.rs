#![doc = "Authoritative offline repository provenance command."]

use crate::adapters::{
    binding_attachment_path, binding_object_path, canonical_binding_attachment,
    canonical_binding_record,
};
#[cfg(test)]
use crate::check::MigrationReport;
use crate::check::{
    CheckResult, MachineReport, ShellDiagnostic, check_snapshot, mounted_report_facts,
    render_human, render_machine, requirement_subject_facts, verdict_name,
};
use crate::core::{
    ApprovalBinding, ArtifactBindingAttachment, ChangeDisposition, ChangeDispositionKind,
    ChangeManifest, DerivedPolicyRatchet, DispositionProtocolContext, ImplementationBinding,
    ImplementationBindingRef, LogicalMountId, NavigationKind, ObjectKind, ObjectRef,
    PolicyAnalysisError, RepositoryPath, ScopeId, SemanticDigest, SourceItemKind, SourceNodeRef,
    StableId, StoredBinding, TraceContext, TraceError, TraceHistory, TraceRow, TraceSelector,
    TraceState, change_impacts, derive_policy_ratchets, disposition_protocol_closure,
    mount_relative_path, source_traversal, trace_query,
};
#[cfg(test)]
use crate::core::{
    ArtifactBindingTransactionStage, CanonicalValue, artifact_binding_transaction_state,
};
use crate::oracle::{CancellationToken, run_oracles_at_roots_controlled, run_oracles_controlled};
use crate::repository::{
    MountMappings, RepositoryConfig, RepositorySnapshot, load_config, snapshot_mounted_repository,
    snapshot_repository,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "coverage-controls")]
use std::cell::Cell;
use std::collections::BTreeMap;
#[cfg(feature = "coverage-controls")]
use std::collections::BTreeSet;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

#[cfg(feature = "coverage-controls")]
thread_local! {
    static CLI_COVERAGE_FAULTS: Cell<u64> = const { Cell::new(0) };
}

#[cfg(feature = "coverage-controls")]
fn cli_coverage_fault(fault: u64) -> bool {
    CLI_COVERAGE_FAULTS.with(|faults| faults.get() & fault != 0)
}

#[cfg(not(feature = "coverage-controls"))]
const fn cli_coverage_fault(_fault: u64) -> bool {
    false
}

const TRACE_KIND_FAULT: u64 = 1 << 0;
const TRACE_CONTEXT_FAULT: u64 = 1 << 1;
const TRACE_QUERY_FAULT: u64 = 1 << 2;
const TRACE_RENDER_FAULT: u64 = 1 << 3;
const PUBLICATION_SNAPSHOT_FAULT: u64 = 1 << 4;
const PUBLICATION_CHECK_FAULT: u64 = 1 << 5;
const PUBLICATION_KIND_FAULT: u64 = 1 << 6;
const PUBLICATION_CONTEXT_FAULT: u64 = 1 << 7;
const PUBLICATION_SELECTOR_FAULT: u64 = 1 << 8;
const PUBLICATION_BASIS_FAULT: u64 = 1 << 9;
const PUBLICATION_REOPEN_FAULT: u64 = 1 << 10;
const PUBLICATION_REVERIFY_FAULT: u64 = 1 << 11;
const DERIVED_DIGEST_FAULT: u64 = 1 << 12;
const DERIVED_PARENT_FAULT: u64 = 1 << 13;
const EXTERNAL_CANDIDATE_FAULT: u64 = 1 << 14;
const EXTERNAL_DIGEST_FAULT: u64 = 1 << 15;
const EXTERNAL_PARENT_FAULT: u64 = 1 << 16;
const EXTERNAL_LOCK_FAULT: u64 = 1 << 17;
const EXTERNAL_REOPEN_FAULT: u64 = 1 << 18;
const EXTERNAL_CHANGED_FAULT: u64 = 1 << 19;
const EXTERNAL_WRITE_FAULT: u64 = 1 << 20;
const EXTERNAL_NORMALIZATION_FAULT: u64 = 1 << 21;
const EXTERNAL_DIAGNOSTIC_FAULT: u64 = 1 << 22;
const EVIDENCE_DIGEST_FAULT: u64 = 1 << 23;
const SOURCE_EDIT_LOGICAL_FAULT: u64 = 1 << 24;
const SOURCE_EDIT_RETAINED_FAULT: u64 = 1 << 25;
const SOURCE_EDIT_READ_FAULT: u64 = 1 << 26;
const SOURCE_EDIT_ABSENT_FAULT: u64 = 1 << 27;
const SOURCE_EDIT_OPEN_FAULT: u64 = 1 << 28;
const SOURCE_EDIT_STALE_FAULT: u64 = 1 << 29;
const PACKAGE_IDENTITY_FAULT: u64 = 1 << 30;
const TRACE_CHECK_FAULT: u64 = 1 << 31;
const TRACE_DIAGNOSTIC_FAULT: u64 = 1 << 32;
const PUBLICATION_REPORT_FAULT: u64 = 1 << 33;
const EVIDENCE_MARKDOWN_FAULT: u64 = 1 << 34;
const EVIDENCE_PAGE_CANDIDATE_FAULT: u64 = 1 << 35;
const EVIDENCE_PAGE_DIGEST_FAULT: u64 = 1 << 36;
const PUBLICATION_FORCE_MOUNTED_FAULT: u64 = 1 << 37;
const PUBLICATION_SOURCE_UTF8_FAULT: u64 = 1 << 38;
const PUBLICATION_NORMALIZE_FAULT: u64 = 1 << 39;
const PUBLICATION_OUTPUT_LOGICAL_FAULT: u64 = 1 << 40;
const PUBLICATION_RENDER_PATH_FAULT: u64 = 1 << 41;
const PUBLICATION_LATE_ARTIFACT_FAULT: u64 = 1 << 42;
const PUBLICATION_QUERY_FAULT: u64 = 1 << 43;
const EXTERNAL_PARENT_ABSENT_FAULT: u64 = 1 << 44;
const EXTERNAL_CONFLICT_FAULT: u64 = 1 << 45;
const DERIVED_WRITE_FAULT: u64 = 1 << 46;
const DERIVED_LOCK_FAULT: u64 = 1 << 47;
const EVIDENCE_LINK_TARGET_FAULT: u64 = 1 << 48;

const EXIT_OK: u8 = 0;
const EXIT_STATIC: u8 = 2;
const EXIT_CHANGE: u8 = 3;
const EXIT_EXECUTION: u8 = 4;
const EXIT_USAGE: u8 = 64;
const EXIT_INPUT: u8 = 70;
const USAGE: &str = "usage: buoy static (--root ROOT | --mount ID=PATH...) --config FILE [--format machine|human]\n       buoy change --base ROOT --head ROOT --config FILE --manifest FILE [--format machine|human]\n       buoy execute (--root ROOT | --mount ID=PATH...) --config FILE [--format machine|human]\n       buoy bind (--root ROOT | --mount ID=PATH...) --config FILE --requirement DIGEST (--implementation DIGEST | --artifact LOGICAL_PATH) --relation ID [--format machine|human]\n       buoy trace (--root ROOT | --mount ID=PATH...) --config FILE (--requirement-id ID | --relation-id ID | --implementation-path PATH | --record DIGEST) [--include-history] [--format machine|human|markdown]\n       buoy publication {check|generate} --plan FILE --artifact ID --config FILE (--root ROOT | --mount ID=PATH...) [--format machine|human]\n       buoy publication evidence --plan FILE --config FILE (--root ROOT | --mount ID=PATH...) [--artifact ID] [--format machine|human]";

/// Flags that carry no value. Every other flag requires one explicit value.
const BOOLEAN_FLAGS: [&str; 1] = ["include-history"];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct CliOptions {
    values: BTreeMap<String, String>,
    mounts: Vec<String>,
}

impl CliOptions {
    fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }

    fn mounts(&self) -> &[String] {
        &self.mounts
    }
}

/// Executes one Buoy CLI invocation and returns its stable exit class.
#[must_use]
pub fn run(arguments: &[String]) -> u8 {
    let Some(command) = arguments.first().map(String::as_str) else {
        return usage();
    };
    if matches!(command, "help" | "--help" | "-h") {
        println!("{USAGE}");
        return EXIT_OK;
    }
    if command == "publication" {
        return publication_command(&arguments[1..]);
    }
    if command == "package-identity" {
        // Task 10.6: a Nix realization of `nix/consumer-package-v1.nix`
        // embeds these exact values through its own build environment; a
        // plain `cargo build` outside that derivation falls back to
        // development-identity placeholders, so the command remains usable
        // for local iteration without a Nix build.
        let source_revision = option_env!("BUOY_PACKAGE_SOURCE_REVISION").unwrap_or("source-tree");
        let toolchain = option_env!("BUOY_PACKAGE_TOOLCHAIN").unwrap_or("rust-toolchain");
        let features = if cli_coverage_fault(PACKAGE_IDENTITY_FAULT) {
            Vec::new()
        } else {
            compiled_digest_features()
        };
        let Ok(rendered) = render_package_identity(source_revision, toolchain, features) else {
            return EXIT_INPUT;
        };
        println!("{}", String::from_utf8_lossy(&rendered));
        return EXIT_OK;
    }
    let Ok(options) = parse_options(&arguments[1..]) else {
        return usage();
    };
    if command != "trace" && options.get("include-history").is_some() {
        return usage();
    }
    match command {
        "static" => static_command(&options),
        "change" => change_command(&options),
        "execute" => execute_command(&options),
        "bind" => bind_command(&options),
        "trace" => trace_command(&options),
        _ => usage(),
    }
}

fn render_package_identity(
    source_revision: &str,
    toolchain: &str,
    features: Vec<&str>,
) -> Result<Vec<u8>, ()> {
    crate::publication::PackageIdentity::new(
        env!("CARGO_PKG_VERSION"),
        source_revision,
        toolchain,
        features,
    )
    .render()
    .map_err(drop)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct BindingTreeReport {
    schema: String,
    relation: String,
    binding: String,
    requirement_id: String,
    atom_revision: String,
    implementation_path: String,
    requirement_path: String,
    freshness: BindingFreshness,
    nodes: Vec<BindingTreeNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)]
struct BindingFreshness {
    record: bool,
    implementation: bool,
    requirement: bool,
    authority: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct BindingTreeNode {
    position: usize,
    digest: String,
    kind: String,
    path: String,
    semantic: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BindingEdit {
    mount: Option<LogicalMountId>,
    source_path: RepositoryPath,
    start: usize,
    end: usize,
    replacement: String,
    object_path: RepositoryPath,
    object_bytes: String,
    report: BindingTreeReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ArtifactBindingEdit {
    mount: Option<LogicalMountId>,
    artifact_path: RepositoryPath,
    artifact_bytes: Vec<u8>,
    object_path: RepositoryPath,
    object_bytes: String,
    attachment_path: RepositoryPath,
    attachment_bytes: String,
    report: BindingTreeReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BindError {
    Input,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AcceptedBindingCheck(CheckResult);

impl AcceptedBindingCheck {
    fn parse(result: CheckResult) -> Result<Self, BindError> {
        if result.diagnostics.is_empty() {
            Ok(Self(result))
        } else {
            Err(BindError::Input)
        }
    }
}

impl core::ops::Deref for AcceptedBindingCheck {
    type Target = CheckResult;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(clippy::expect_used)]
fn bind_command(options: &CliOptions) -> u8 {
    bind_command_internal(options, None)
}

#[allow(clippy::expect_used)]
fn bind_command_internal(options: &CliOptions, fault: Option<&str>) -> u8 {
    let Some(config_path) = options.get("config").map(PathBuf::from) else {
        return usage();
    };
    let (Some(requirement), Some(relation)) = (options.get("requirement"), options.get("relation"))
    else {
        return usage();
    };
    let subject = match (options.get("implementation"), options.get("artifact")) {
        (Some(implementation), None) => Ok((implementation.as_str(), None)),
        (None, Some(artifact)) => Ok((artifact.as_str(), Some(artifact.as_str()))),
        _ => Err(()),
    };
    let Ok((subject, artifact)) = subject else {
        return usage();
    };
    let format = options.get("format").map_or("machine", String::as_str);
    if !matches!(format, "machine" | "human") {
        return usage();
    }
    let Ok(config) = load_config(&config_path) else {
        return EXIT_INPUT;
    };
    let Ok((snapshot, roots)) = invocation_snapshot(options, &config) else {
        return EXIT_INPUT;
    };
    if let Some(artifact) = artifact {
        let edit = match prepare_artifact_binding_edit(
            &snapshot,
            &config,
            requirement,
            artifact,
            relation,
        ) {
            Ok(edit) => edit,
            Err(BindError::Input) => return EXIT_INPUT,
        };
        let root = if fault == Some("write-root") {
            None
        } else {
            binding_write_root(&roots, edit.mount.as_ref())
        };
        let Some(root) = root else {
            return EXIT_INPUT;
        };
        return apply_and_render_artifact_binding(
            &mut RealBindingIo::default(),
            &root,
            &edit,
            format,
        );
    }
    let edit = match prepare_binding_edit(&snapshot, &config, requirement, subject, relation) {
        Ok(edit) => edit,
        Err(BindError::Input) => return EXIT_INPUT,
    };
    let root = if fault == Some("write-root") {
        None
    } else {
        binding_write_root(&roots, edit.mount.as_ref())
    };
    let Some(root) = root else {
        return EXIT_INPUT;
    };
    apply_and_render_binding(&mut RealBindingIo::default(), &root, &edit, format)
}

#[allow(clippy::expect_used)]
fn apply_and_render_binding(
    io: &mut dyn BindingIo,
    root: &Path,
    edit: &BindingEdit,
    format: &str,
) -> u8 {
    if apply_binding_edit_with(io, root, edit).is_err() {
        return EXIT_INPUT;
    }
    let rendered = if format == "human" {
        render_binding_tree_human(&edit.report)
    } else {
        format!(
            "{}\n",
            serde_json::to_string(&edit.report)
                .expect("closed binding tree report serialization is infallible")
        )
    };
    print!("{rendered}");
    EXIT_OK
}

#[allow(clippy::expect_used)]
fn apply_and_render_artifact_binding(
    io: &mut dyn BindingIo,
    root: &Path,
    edit: &ArtifactBindingEdit,
    format: &str,
) -> u8 {
    if apply_artifact_binding_edit_with(io, root, edit).is_err() {
        return EXIT_INPUT;
    }
    let rendered = if format == "human" {
        render_binding_tree_human(&edit.report)
    } else {
        format!(
            "{}\n",
            serde_json::to_string(&edit.report)
                .expect("closed artifact binding report serialization is infallible")
        )
    };
    print!("{rendered}");
    EXIT_OK
}

#[allow(clippy::expect_used)]
fn prepare_binding_edit(
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    requirement_text: &str,
    implementation_text: &str,
    relation_text: &str,
) -> Result<BindingEdit, BindError> {
    let requirement = SourceNodeRef::parse(requirement_text).map_err(|_| BindError::Input)?;
    let implementation = SourceNodeRef::parse(implementation_text).map_err(|_| BindError::Input)?;
    let relation = StableId::new(relation_text).map_err(|_| BindError::Input)?;
    if [requirement.digest(), implementation.digest()]
        .iter()
        .any(|digest| digest.algorithm() != config.active_algorithm().algorithm())
    {
        return Err(BindError::Input);
    }
    let checked = AcceptedBindingCheck::parse(
        check_snapshot(snapshot, config).map_err(|_| BindError::Input)?,
    )?;
    let graph = checked.source.graph();
    let requirement_node = graph.node(requirement).ok_or(BindError::Input)?;
    let implementation_node = graph.node(implementation).ok_or(BindError::Input)?;
    // The closed built-in registry roots each Rust item and connects every
    // OpenSpec child to its selected document root. A node in this checked
    // graph is therefore current. Historical and stale selectors are absent.
    if requirement_node.item().kind().as_str() != "openspec/requirement/v1"
        || implementation_node.item().kind().as_str() != "rust/item/v1"
    {
        return Err(BindError::Input);
    }
    let authorities = checked
        .source
        .authorities()
        .iter()
        .filter(|authority| authority.node() == requirement)
        .collect::<Vec<_>>();
    let authority = authorities
        .into_iter()
        .next()
        .expect("one validated OpenSpec requirement has one configured source authority");
    let record = ImplementationBinding::new_with(
        config.active_algorithm(),
        relation,
        NavigationKind::Implements,
        implementation,
        requirement,
        authority.atom().clone(),
    )
    .expect("validated binding inputs produce one record under the selected digest member");
    let source_span = checked
        .source
        .span(implementation)
        .expect("a retained implementation node has one canonical source span");
    let source_file = snapshot
        .files
        .iter()
        .find(|file| file.path == source_span.path)
        .expect("a retained source span names its saved snapshot file");
    let source = core::str::from_utf8(&source_file.bytes)
        .expect("a retained Rust item comes from UTF-8 parser input");
    let (start, end) = binding_attribute_span(source, source_span.start, source_span.end)
        .expect("a retained Rust item has one validated source span");
    let object_path = RepositoryPath::new(&binding_object_path(record.record()))
        .expect("a canonical binding digest produces a repository path");
    let object_bytes = canonical_binding_record(&record);
    if snapshot.files.iter().any(|file| file.path == object_path) {
        return Err(BindError::Input);
    }
    let (mount, source_path) = binding_subject_location(config, &source_span.path);
    let report = binding_tree_report(&checked, &record).ok_or(BindError::Input)?;
    Ok(BindingEdit {
        mount,
        source_path,
        start,
        end,
        replacement: format!(
            "#[buoy::implements(\"{}\")]",
            record.record().canonical_text()
        ),
        object_path,
        object_bytes,
        report,
    })
}

#[allow(clippy::expect_used)]
fn prepare_artifact_binding_edit(
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    requirement_text: &str,
    artifact_text: &str,
    relation_text: &str,
) -> Result<ArtifactBindingEdit, BindError> {
    let requirement = SourceNodeRef::parse(requirement_text).map_err(|_| BindError::Input)?;
    let artifact_path = RepositoryPath::new(artifact_text).map_err(|_| BindError::Input)?;
    let relation = StableId::new(relation_text).map_err(|_| BindError::Input)?;
    if requirement.digest().algorithm() != config.active_algorithm().algorithm() {
        return Err(BindError::Input);
    }
    let checked = AcceptedBindingCheck::parse(
        check_snapshot(snapshot, config).map_err(|_| BindError::Input)?,
    )?;
    let requirement_node = checked
        .source
        .graph()
        .node(requirement)
        .ok_or(BindError::Input)?;
    if requirement_node.item().kind().as_str() != "openspec/requirement/v1" {
        return Err(BindError::Input);
    }
    let matching_artifacts = checked
        .source
        .graph()
        .nodes()
        .iter()
        .filter(|node| {
            node.item().path() == &artifact_path
                && node.item().kind().as_str() == "artifact/file/v1"
        })
        .collect::<Vec<_>>();
    let [artifact_node] = matching_artifacts.as_slice() else {
        return Err(BindError::Input);
    };
    let authority = unique_requirement_authority(&checked, requirement);
    let record = ImplementationBinding::new_with(
        config.active_algorithm(),
        relation.clone(),
        NavigationKind::Implements,
        artifact_node.reference(),
        requirement,
        authority.atom().clone(),
    )
    .expect("validated artifact binding inputs always construct");
    let attachment = ArtifactBindingAttachment::new_with(
        config.active_algorithm(),
        relation.clone(),
        artifact_path.clone(),
        artifact_node.reference(),
        record.record(),
    )
    .expect("validated artifact attachment inputs always construct");
    let source_file = snapshot
        .files
        .iter()
        .find(|file| file.path == artifact_path)
        .expect("a selected artifact node retains its saved snapshot file");
    let (mount, physical_artifact_path) = if config.is_mounted() {
        let declaration = config
            .mount_for_logical_path(&artifact_path)
            .expect("a selected logical artifact belongs to one declared mount");
        let relative = mount_relative_path(declaration, &artifact_path)
            .expect("a selected logical artifact contains its mount prefix");
        (Some(declaration.id().clone()), relative)
    } else {
        (None, artifact_path)
    };
    let object_path = RepositoryPath::new(&binding_object_path(record.record()))
        .expect("a canonical record digest always produces a repository path");
    let attachment_path = RepositoryPath::new(
        &binding_attachment_path(config.active_algorithm(), &relation)
            .expect("a validated relation always produces an attachment path"),
    )
    .expect("a canonical attachment key always produces a repository path");
    if !config.selectors().iter().any(|selector| {
        selector.mount() == mount.as_ref()
            && selector.adapter() == "binding/v1"
            && selector.selects_path(&object_path)
            && selector.selects_path(&attachment_path)
    }) {
        return Err(BindError::Input);
    }
    let report = binding_tree_report(&checked, &record).ok_or(BindError::Input)?;
    Ok(ArtifactBindingEdit {
        mount,
        artifact_path: physical_artifact_path,
        artifact_bytes: source_file.bytes.clone(),
        object_path,
        object_bytes: canonical_binding_record(&record),
        attachment_path,
        attachment_bytes: canonical_binding_attachment(&attachment),
        report,
    })
}

#[allow(clippy::expect_used)]
fn binding_subject_location(
    config: &RepositoryConfig,
    source_path: &RepositoryPath,
) -> (Option<LogicalMountId>, RepositoryPath) {
    if config.is_mounted() {
        let declaration = config
            .mount_for_logical_path(source_path)
            .expect("a retained logical source belongs to one declared mount");
        let relative = mount_relative_path(declaration, source_path)
            .expect("a retained logical source contains its mount prefix");
        (Some(declaration.id().clone()), relative)
    } else {
        (None, source_path.clone())
    }
}

#[allow(clippy::expect_used)]
fn unique_requirement_authority(
    checked: &CheckResult,
    requirement: SourceNodeRef,
) -> &crate::core::SourceAuthority {
    let authorities = checked
        .source
        .authorities()
        .iter()
        .filter(|authority| authority.node() == requirement)
        .collect::<Vec<_>>();
    authorities
        .first()
        .copied()
        .expect("a retained OpenSpec requirement has one exact atom authority")
}

#[allow(clippy::expect_used)]
fn binding_attribute_span(source: &str, start: u64, end: u64) -> Result<(usize, usize), BindError> {
    let start = usize::try_from(start).expect("a validated span start fits its input length");
    let end = usize::try_from(end).expect("a validated span end fits its input length");
    if start >= end || end > source.len() {
        return Err(BindError::Input);
    }
    let candidate = &source[start..end];
    if candidate.starts_with("#[buoy::implements(") && candidate.ends_with(']') {
        Ok((start, end))
    } else {
        Ok((start, start))
    }
}

/// Shared projection facts for one fresh implementation relation.
///
/// One builder supplies these facts to the frozen successful-bind report and to
/// one fresh trace row, so the two views cannot disagree about one relation.
#[derive(Clone, Debug, Eq, PartialEq)]
struct FreshLinkFacts {
    relation: String,
    binding: String,
    requirement_id: String,
    atom_revision: String,
    implementation_path: String,
    requirement_path: String,
    nodes: Vec<BindingTreeNode>,
}

fn fresh_link_facts(
    checked: &CheckResult,
    record: &ImplementationBinding,
) -> Option<FreshLinkFacts> {
    let graph = checked.source.graph();
    let traversal = source_traversal(graph, record.requirement()).ok()?;
    let mut nodes = Vec::with_capacity(traversal.order().len() + 1);
    for (position, reference) in core::iter::once(record.subject())
        .chain(traversal.order().iter().copied())
        .enumerate()
    {
        let node = graph.node(reference)?;
        nodes.push(BindingTreeNode {
            position,
            digest: reference.canonical_text(),
            kind: node.item().kind().as_str().to_owned(),
            path: node.item().path().as_str().to_owned(),
            semantic: String::from_utf8_lossy(node.item().semantic_bytes()).into_owned(),
        });
    }
    let implementation_path = graph
        .node(record.subject())?
        .item()
        .path()
        .as_str()
        .to_owned();
    let requirement_path = graph
        .node(record.requirement())?
        .item()
        .path()
        .as_str()
        .to_owned();
    Some(FreshLinkFacts {
        relation: record.meta().id().as_str().to_owned(),
        binding: record.record().canonical_text(),
        requirement_id: record.atom().id().as_str().to_owned(),
        atom_revision: record.atom().revision().canonical_text(),
        implementation_path,
        requirement_path,
        nodes,
    })
}

/// Renders the frozen successful-bind report from the shared fresh facts.
///
/// A successful bind reports one fresh relation, so every freshness fact is
/// true by construction. The schema stays byte-compatible.
fn binding_tree_report(
    checked: &CheckResult,
    record: &ImplementationBinding,
) -> Option<BindingTreeReport> {
    let facts = fresh_link_facts(checked, record)?;
    Some(BindingTreeReport {
        schema: String::from("buoy/binding-tree-report/v1"),
        relation: facts.relation,
        binding: facts.binding,
        requirement_id: facts.requirement_id,
        atom_revision: facts.atom_revision,
        implementation_path: facts.implementation_path,
        requirement_path: facts.requirement_path,
        freshness: BindingFreshness {
            record: true,
            implementation: true,
            requirement: true,
            authority: true,
        },
        nodes: facts.nodes,
    })
}

fn render_binding_tree_human(report: &BindingTreeReport) -> String {
    let mut output = format!(
        "relation: {}\nbinding: {}\nrequirement: {}\nimplementation path: {}\nrequirement path: {}\nfreshness: record={} implementation={} requirement={} authority={}\n",
        report.relation,
        report.binding,
        report.requirement_id,
        report.implementation_path,
        report.requirement_path,
        report.freshness.record,
        report.freshness.implementation,
        report.freshness.requirement,
        report.freshness.authority,
    );
    for node in &report.nodes {
        let _ = writeln!(
            output,
            "node {}: {} {} {}",
            node.position, node.kind, node.path, node.digest
        );
    }
    output
}

trait BindingIo {
    fn exists(&self, path: &Path) -> bool;
    fn read(&mut self, path: &Path) -> std::io::Result<Vec<u8>>;
    fn create_dir_all(&mut self, path: &Path) -> std::io::Result<()>;
    fn create_new(&mut self, path: &Path) -> std::io::Result<()>;
    fn write_all(&mut self, path: &Path, bytes: &[u8]) -> std::io::Result<()>;
    fn sync_all(&mut self, path: &Path) -> std::io::Result<()>;
    fn write(&mut self, path: &Path, bytes: &[u8]) -> std::io::Result<()>;
    fn rename(&mut self, from: &Path, to: &Path) -> std::io::Result<()>;
    fn remove_file(&mut self, path: &Path) -> std::io::Result<()>;
    fn remove_dir(&mut self, path: &Path) -> std::io::Result<()>;
}

#[derive(Default)]
struct RealBindingIo {
    open_files: BTreeMap<PathBuf, fs::File>,
}

impl BindingIo for RealBindingIo {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn read(&mut self, path: &Path) -> std::io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn create_dir_all(&mut self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }

    fn create_new(&mut self, path: &Path) -> std::io::Result<()> {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        self.open_files.insert(path.to_path_buf(), file);
        Ok(())
    }

    fn write_all(&mut self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        self.open_files
            .get_mut(path)
            .ok_or(std::io::ErrorKind::NotFound)?
            .write_all(bytes)
    }

    fn sync_all(&mut self, path: &Path) -> std::io::Result<()> {
        self.open_files
            .get(path)
            .ok_or(std::io::ErrorKind::NotFound)?
            .sync_all()
    }

    fn write(&mut self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        fs::write(path, bytes)
    }

    fn rename(&mut self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.open_files.remove(from);
        fs::rename(from, to)
    }

    fn remove_file(&mut self, path: &Path) -> std::io::Result<()> {
        self.open_files.remove(path);
        fs::remove_file(path)
    }

    fn remove_dir(&mut self, path: &Path) -> std::io::Result<()> {
        fs::remove_dir(path)
    }
}

#[cfg(feature = "coverage-controls")]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CoverageIoStage {
    ReadArtifact,
    ReadObject,
    ReadAttachment,
    CreateDirectory,
    CreateObject,
    WriteObject,
    SyncObject,
    RenameObject,
    CreateAttachment,
    WriteAttachment,
    WriteSource,
    SyncAttachment,
    RenameAttachment,
    RemoveFile,
    RemoveDirectory,
}

#[cfg(feature = "coverage-controls")]
struct CoverageBindingIo {
    files: BTreeMap<PathBuf, Vec<u8>>,
    directories: BTreeSet<PathBuf>,
    failures: BTreeMap<CoverageIoStage, u8>,
}

#[cfg(feature = "coverage-controls")]
impl CoverageBindingIo {
    fn new(root: &Path, edit: &ArtifactBindingEdit) -> Self {
        let artifact = root.join(edit.artifact_path.as_str());
        let attachment = root.join(edit.attachment_path.as_str());
        let files = BTreeMap::from([
            (artifact, edit.artifact_bytes.clone()),
            (attachment.clone(), b"prior-sidecar".to_vec()),
        ]);
        let mut directories = BTreeSet::from([root.to_path_buf()]);
        let mut current = attachment.parent();
        while let Some(directory) = current {
            directories.insert(directory.to_path_buf());
            current = if directory == root {
                None
            } else {
                directory.parent()
            };
        }
        Self {
            files,
            directories,
            failures: BTreeMap::new(),
        }
    }

    fn inject(&mut self, stage: CoverageIoStage, count: u8) {
        self.failures.insert(stage, count);
    }

    fn fail(&mut self, stage: CoverageIoStage) -> std::io::Result<()> {
        let remaining = self.failures.entry(stage).or_default();
        if *remaining == 0 {
            return Ok(());
        }
        *remaining -= 1;
        Err(std::io::Error::other("coverage binding I/O fault"))
    }

    fn path_stage(
        path: &Path,
        object: CoverageIoStage,
        attachment: CoverageIoStage,
    ) -> CoverageIoStage {
        if path.to_string_lossy().contains("/objects/") {
            object
        } else {
            attachment
        }
    }
}

#[cfg(feature = "coverage-controls")]
impl BindingIo for CoverageBindingIo {
    fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path) || self.directories.contains(path)
    }

    fn read(&mut self, path: &Path) -> std::io::Result<Vec<u8>> {
        let stage = if path.ends_with("artifact.bin") {
            CoverageIoStage::ReadArtifact
        } else if path.to_string_lossy().contains("/objects/") {
            CoverageIoStage::ReadObject
        } else {
            CoverageIoStage::ReadAttachment
        };
        self.fail(stage)?;
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
    }

    fn create_dir_all(&mut self, path: &Path) -> std::io::Result<()> {
        self.fail(CoverageIoStage::CreateDirectory)?;
        self.directories.insert(path.to_path_buf());
        Ok(())
    }

    fn create_new(&mut self, path: &Path) -> std::io::Result<()> {
        let stage = Self::path_stage(
            path,
            CoverageIoStage::CreateObject,
            CoverageIoStage::CreateAttachment,
        );
        self.fail(stage)?;
        if self.files.contains_key(path) {
            return Err(std::io::ErrorKind::AlreadyExists.into());
        }
        self.files.insert(path.to_path_buf(), Vec::new());
        Ok(())
    }

    fn write_all(&mut self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let stage = Self::path_stage(
            path,
            CoverageIoStage::WriteObject,
            CoverageIoStage::WriteAttachment,
        );
        self.fail(stage)?;
        self.files.insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn sync_all(&mut self, path: &Path) -> std::io::Result<()> {
        let stage = Self::path_stage(
            path,
            CoverageIoStage::SyncObject,
            CoverageIoStage::SyncAttachment,
        );
        self.fail(stage)
    }

    fn write(&mut self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        self.fail(CoverageIoStage::WriteSource)?;
        self.files.insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn rename(&mut self, from: &Path, to: &Path) -> std::io::Result<()> {
        let stage = Self::path_stage(
            from,
            CoverageIoStage::RenameObject,
            CoverageIoStage::RenameAttachment,
        );
        self.fail(stage)?;
        let bytes = self
            .files
            .remove(from)
            .ok_or(std::io::ErrorKind::NotFound)?;
        self.files.insert(to.to_path_buf(), bytes);
        Ok(())
    }

    fn remove_file(&mut self, path: &Path) -> std::io::Result<()> {
        self.fail(CoverageIoStage::RemoveFile)?;
        self.files.remove(path);
        Ok(())
    }

    fn remove_dir(&mut self, path: &Path) -> std::io::Result<()> {
        self.fail(CoverageIoStage::RemoveDirectory)?;
        self.directories.remove(path);
        Ok(())
    }
}

#[allow(clippy::expect_used)]
fn apply_binding_edit_with(
    io: &mut dyn BindingIo,
    root: &Path,
    edit: &BindingEdit,
) -> Result<(), ()> {
    let source_path = root.join(edit.source_path.as_str());
    let object_path = root.join(edit.object_path.as_str());
    if io.exists(&object_path) {
        return Err(());
    }
    let source = io.read(&source_path).map_err(|_| ())?;
    let mut updated = source.clone();
    updated.splice(edit.start..edit.end, edit.replacement.bytes());
    let parent = object_path
        .parent()
        .expect("a canonical repository path has a parent below its repository root");
    io.create_dir_all(parent).map_err(|_| ())?;
    let temporary = object_path.with_extension("toml.tmp");
    io.create_new(&temporary).map_err(|_| ())?;
    io.write_all(&temporary, edit.object_bytes.as_bytes())
        .map_err(|_| ())?;
    io.sync_all(&temporary).map_err(|_| ())?;
    if io.write(&source_path, &updated).is_err() {
        let _ = io.remove_file(&temporary);
        return Err(());
    }
    if io.rename(&temporary, &object_path).is_err() {
        let _ = io.write(&source_path, &source);
        let _ = io.remove_file(&temporary);
        return Err(());
    }
    Ok(())
}

#[allow(clippy::expect_used)]
fn apply_artifact_binding_edit_with(
    io: &mut dyn BindingIo,
    root: &Path,
    edit: &ArtifactBindingEdit,
) -> Result<(), ()> {
    let artifact_path = root.join(edit.artifact_path.as_str());
    let object_path = root.join(edit.object_path.as_str());
    let attachment_path = root.join(edit.attachment_path.as_str());
    let object_temporary = object_path.with_extension("toml.tmp");
    let attachment_temporary = attachment_path.with_extension("toml.tmp");

    if io.read(&artifact_path).map_err(|_| ())? != edit.artifact_bytes {
        return Err(());
    }
    let object_exists = io.exists(&object_path);
    if object_exists && io.read(&object_path).map_err(|_| ())? != edit.object_bytes.as_bytes() {
        return Err(());
    }
    if io.exists(&attachment_path) {
        let _prior_attachment = io.read(&attachment_path).map_err(|_| ())?;
    }

    let mut created_directories = Vec::new();
    let mut object_created = false;
    let transaction: Result<(), ()> = (|| {
        ensure_parent_directories(
            io,
            root,
            object_path
                .parent()
                .expect("a canonical object path has one parent"),
            &mut created_directories,
        )?;
        ensure_parent_directories(
            io,
            root,
            attachment_path
                .parent()
                .expect("a canonical attachment path has one parent"),
            &mut created_directories,
        )?;
        remove_stale_temporary(io, &object_temporary)?;
        remove_stale_temporary(io, &attachment_temporary)?;

        if !object_exists {
            io.create_new(&object_temporary).map_err(|_| ())?;
            io.write_all(&object_temporary, edit.object_bytes.as_bytes())
                .map_err(|_| ())?;
            io.sync_all(&object_temporary).map_err(|_| ())?;
            io.rename(&object_temporary, &object_path).map_err(|_| ())?;
            object_created = true;
        }

        io.create_new(&attachment_temporary).map_err(|_| ())?;
        io.write_all(&attachment_temporary, edit.attachment_bytes.as_bytes())
            .map_err(|_| ())?;
        io.sync_all(&attachment_temporary).map_err(|_| ())?;
        io.rename(&attachment_temporary, &attachment_path)
            .map_err(|_| ())?;
        Ok(())
    })();

    if transaction.is_err() {
        let cleanup = cleanup_artifact_transaction(
            io,
            &object_temporary,
            &attachment_temporary,
            object_created.then_some(object_path.as_path()),
            &created_directories,
        );
        return cleanup.and(Err(()));
    }
    Ok(())
}

fn ensure_parent_directories(
    io: &mut dyn BindingIo,
    root: &Path,
    parent: &Path,
    created: &mut Vec<PathBuf>,
) -> Result<(), ()> {
    let relative = parent.strip_prefix(root).map_err(|_| ())?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        if !io.exists(&current) {
            io.create_dir_all(&current).map_err(|_| ())?;
            created.push(current.clone());
        }
    }
    Ok(())
}

fn remove_stale_temporary(io: &mut dyn BindingIo, path: &Path) -> Result<(), ()> {
    if io.exists(path) {
        retry_remove_file(io, path)?;
    }
    Ok(())
}

fn cleanup_artifact_transaction(
    io: &mut dyn BindingIo,
    object_temporary: &Path,
    attachment_temporary: &Path,
    object: Option<&Path>,
    created_directories: &[PathBuf],
) -> Result<(), ()> {
    for path in [Some(object_temporary), Some(attachment_temporary), object]
        .into_iter()
        .flatten()
    {
        if io.exists(path) {
            retry_remove_file(io, path)?;
        }
    }
    for directory in created_directories.iter().rev() {
        if io.exists(directory) {
            retry_remove_directory(io, directory)?;
        }
    }
    Ok(())
}

fn retry_remove_file(io: &mut dyn BindingIo, path: &Path) -> Result<(), ()> {
    for _ in 0..3 {
        if io.remove_file(path).is_ok() || !io.exists(path) {
            return Ok(());
        }
    }
    Err(())
}

fn retry_remove_directory(io: &mut dyn BindingIo, path: &Path) -> Result<(), ()> {
    for _ in 0..3 {
        if io.remove_dir(path).is_ok() || !io.exists(path) {
            return Ok(());
        }
    }
    Err(())
}

fn static_command(options: &CliOptions) -> u8 {
    let Some(config_path) = options.get("config").map(PathBuf::from) else {
        return usage();
    };
    let Ok(config) = load_config(&config_path) else {
        return EXIT_INPUT;
    };
    let Ok((snapshot, _)) = invocation_snapshot(options, &config) else {
        return EXIT_INPUT;
    };
    let Ok(result) = check_snapshot(&snapshot, &config) else {
        return EXIT_INPUT;
    };
    let requirement_subjects = static_requirement_subjects(&result);
    static_report_command(
        &config,
        result,
        options.get("format").map(String::as_str),
        requirement_subjects,
    )
}

fn static_requirement_subjects(
    result: &CheckResult,
) -> Result<Vec<crate::check::RequirementSubjectFact>, crate::check::RequirementSubjectError> {
    if result.diagnostics.is_empty() {
        requirement_subject_facts(result)
    } else {
        Ok(Vec::new())
    }
}

fn static_report_command(
    config: &RepositoryConfig,
    result: CheckResult,
    format: Option<&str>,
    requirement_subjects: Result<
        Vec<crate::check::RequirementSubjectFact>,
        crate::check::RequirementSubjectError,
    >,
) -> u8 {
    let requirement_subjects = match requirement_subjects {
        Ok(subjects) => subjects,
        Err(error) => {
            eprintln!("{}", error.code());
            return EXIT_INPUT;
        }
    };
    let (mounts, opaque_subjects, artifact_attachments) = mounted_report_facts(config, &result);
    let report = MachineReport {
        command: String::from("static"),
        graph: result.graph_digest.canonical_text(),
        diagnostics: result.diagnostics,
        migration: result.migration,
        verdicts: BTreeMap::new(),
        impacts: Vec::new(),
        mounts,
        requirement_subjects,
        opaque_subjects,
        artifact_attachments,
    };
    if emit(&report, format).is_err() {
        return EXIT_INPUT;
    }
    if report.diagnostics.is_empty() {
        EXIT_OK
    } else {
        EXIT_STATIC
    }
}

// ---------------------------------------------------------------------------
// Read-only trace projections
// ---------------------------------------------------------------------------

/// Leading hexadecimal characters one human digest shows by default.
const TRACE_DIGEST_PREFIX: usize = 8;
/// Versioned machine schema for one trace projection.
const TRACE_REPORT_SCHEMA: &str = "buoy/trace-report/v1";
/// Versioned schema for the Markdown trace-page renderer (RTP-007).
const TRACE_MARKDOWN_SCHEMA: &str = "buoy/readable-trace-markdown/v1";
/// Shared assurance boundary text for every rendered trace view.
const TRACE_BOUNDARY: &str = "This projection supplies navigation and impact only. It establishes no coverage, execution, proof, approval, correctness, or prose-to-code correspondence.";

/// Rendered view one trace invocation selects.
///
/// Only `trace` accepts Markdown. Every other command keeps its machine and
/// human set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TraceFormat {
    Machine,
    Human,
    Markdown,
}

/// Boundary failure class for one typed selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectorFault {
    /// Absent or conflicting selectors are usage failures.
    Usage,
    /// A malformed typed value is an input failure with one stable code.
    Input(&'static str),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TraceSelectorFact {
    kind: String,
    value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct TraceSpanFact {
    start: u64,
    end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools)]
struct TraceFreshnessFact {
    record: bool,
    attachment: bool,
    implementation: bool,
    requirement: bool,
    authority: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct TraceDiagnosticFact {
    code: String,
    id: Option<String>,
    path: Option<String>,
    start: u64,
    end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TraceTraversalNode {
    position: usize,
    digest: String,
    kind: String,
    path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TraceReportRow {
    state: String,
    relation: String,
    navigation: String,
    record: String,
    requirement_id: String,
    atom_revision: String,
    requirement_node: String,
    implementation_node: String,
    requirement_path: Option<String>,
    implementation_path: Option<String>,
    requirement_kind: Option<String>,
    implementation_kind: Option<String>,
    requirement_name: Option<String>,
    implementation_name: Option<String>,
    requirement_span: Option<TraceSpanFact>,
    implementation_span: Option<TraceSpanFact>,
    freshness: TraceFreshnessFact,
    traversal: Vec<TraceTraversalNode>,
    diagnostics: Vec<TraceDiagnosticFact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TraceReport {
    schema: String,
    graph: String,
    selector: TraceSelectorFact,
    include_history: bool,
    rows: Vec<TraceReportRow>,
    diagnostics: Vec<TraceDiagnosticFact>,
}

fn parse_trace_format(value: Option<&String>) -> Result<TraceFormat, ()> {
    match value.map_or("machine", String::as_str) {
        "machine" => Ok(TraceFormat::Machine),
        "human" => Ok(TraceFormat::Human),
        "markdown" => Ok(TraceFormat::Markdown),
        _ => Err(()),
    }
}

/// Parses exactly one typed selector at the untrusted-input boundary.
fn parse_trace_selector(options: &CliOptions) -> Result<TraceSelector, SelectorFault> {
    let mut present = Vec::new();
    for key in [
        "requirement-id",
        "relation-id",
        "implementation-path",
        "record",
    ] {
        if let Some(value) = options.get(key) {
            present.push((key, value));
        }
    }
    let [(key, value)] = present.as_slice() else {
        return Err(SelectorFault::Usage);
    };
    match *key {
        "requirement-id" => StableId::new(value)
            .map(TraceSelector::RequirementId)
            .map_err(|error| SelectorFault::Input(error.code())),
        "relation-id" => StableId::new(value)
            .map(TraceSelector::RelationId)
            .map_err(|error| SelectorFault::Input(error.code())),
        "implementation-path" => RepositoryPath::new(value)
            .map(TraceSelector::ImplementationPath)
            .map_err(|error| SelectorFault::Input(error.code())),
        _ => ImplementationBindingRef::parse(value)
            .map(TraceSelector::Record)
            .map_err(|error| SelectorFault::Input(error.code())),
    }
}

fn trace_selector_fact(selector: &TraceSelector) -> TraceSelectorFact {
    let (kind, value) = match selector {
        TraceSelector::RequirementId(id) => ("requirement-id", id.as_str().to_owned()),
        TraceSelector::RelationId(id) => ("relation-id", id.as_str().to_owned()),
        TraceSelector::ImplementationPath(path) => {
            ("implementation-path", path.as_str().to_owned())
        }
        TraceSelector::Record(reference) => ("record", reference.canonical_text()),
    };
    TraceSelectorFact {
        kind: String::from(kind),
        value,
    }
}

const fn navigation_name(kind: NavigationKind) -> &'static str {
    match kind {
        NavigationKind::Implements => "implements",
        NavigationKind::Owns => "owns",
        NavigationKind::Consumes => "consumes",
        NavigationKind::Configures => "configures",
    }
}

/// Parser-owned display name for one retained node.
///
/// The name is the first line of the exact parser-canonical semantic bytes, so
/// no separate label can drift from the identity those bytes bind.
fn node_display_name(node: &crate::core::SourceNode) -> String {
    let semantic = String::from_utf8_lossy(node.item().semantic_bytes()).into_owned();
    match node.item().kind().as_str() {
        "openspec/requirement/v1" => semantic
            .lines()
            .find_map(|line| line.strip_prefix("### Requirement: "))
            .and_then(|heading| heading.split_once(char::is_whitespace))
            .map_or_else(
                || first_display_line(&semantic),
                |(_, title)| title.to_owned(),
            ),
        "openspec/scenario/v1" => match semantic
            .lines()
            .find_map(|line| line.strip_prefix("#### Scenario: "))
        {
            Some(title) => title.to_owned(),
            None => first_display_line(&semantic),
        },
        "openspec/step/v1" => semantic.lines().skip(1).collect::<Vec<_>>().join(" "),
        "rust/item/v1" => match rust_item_name(&semantic) {
            Some(name) => name,
            None => first_display_line(&semantic),
        },
        "artifact/file/v1" => node
            .item()
            .path()
            .as_str()
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_owned(),
        _ => first_display_line(&semantic),
    }
}

fn first_display_line(semantic: &str) -> String {
    semantic
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .chars()
        .take(120)
        .collect()
}

fn rust_item_name(semantic: &str) -> Option<String> {
    let item = syn::parse_file(semantic).ok()?.items.into_iter().next()?;
    let name = match item {
        syn::Item::Const(item) => item.ident.to_string(),
        syn::Item::Enum(item) => item.ident.to_string(),
        syn::Item::Fn(item) => item.sig.ident.to_string(),
        syn::Item::Mod(item) => item.ident.to_string(),
        syn::Item::Static(item) => item.ident.to_string(),
        syn::Item::Struct(item) => item.ident.to_string(),
        syn::Item::Trait(item) => item.ident.to_string(),
        syn::Item::TraitAlias(item) => item.ident.to_string(),
        syn::Item::Type(item) => item.ident.to_string(),
        syn::Item::Union(item) => item.ident.to_string(),
        _ => return None,
    };
    Some(name)
}

/// Retained logical path, parser item kind, display name, and exact span.
fn node_facts(
    checked: &CheckResult,
    node: SourceNodeRef,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<TraceSpanFact>,
) {
    let Some(retained) = checked.source.graph().node(node) else {
        return (None, None, None, None);
    };
    let span = checked.source.span(node).map(|span| TraceSpanFact {
        start: span.start,
        end: span.end,
    });
    (
        Some(retained.item().path().as_str().to_owned()),
        Some(retained.item().kind().as_str().to_owned()),
        Some(node_display_name(retained)),
        span,
    )
}

/// Ordered requirement traversal for one relation, when both nodes are retained.
fn link_nodes(
    checked: &CheckResult,
    subject: SourceNodeRef,
    requirement: SourceNodeRef,
) -> Vec<BindingTreeNode> {
    let graph = checked.source.graph();
    let mut references = Vec::new();
    if graph.node(subject).is_some() {
        references.push(subject);
    }
    if let Ok(traversal) = source_traversal(graph, requirement) {
        references.extend(traversal.order().iter().copied());
    }
    references
        .into_iter()
        .enumerate()
        .filter_map(|(position, reference)| {
            graph.node(reference).map(|node| BindingTreeNode {
                position,
                digest: reference.canonical_text(),
                kind: node.item().kind().as_str().to_owned(),
                path: node.item().path().as_str().to_owned(),
                semantic: String::from_utf8_lossy(node.item().semantic_bytes()).into_owned(),
            })
        })
        .collect()
}

fn stored_record(
    checked: &CheckResult,
    locator: ImplementationBindingRef,
) -> Option<&ImplementationBinding> {
    checked
        .source
        .store()
        .iter()
        .find(|member| member.locator() == locator)
        .map(StoredBinding::record)
}

/// Omits only the diagnostics whose primary stable ID equals `link_id`, and
/// returns every diagnostic unchanged when `link_id` is absent.
///
/// This is the CLI-boundary conversion wrapper around
/// [`crate::publication::omit_declared_link_diagnostics`], the tested pure
/// filter: TPT-003 requires normalization to suppress only diagnostics
/// caused by the exact declared link currently being checked or generated,
/// never any other relation, requirement, implementation, authority, or
/// unrelated-link diagnostic.
fn filter_link_diagnostics(
    diagnostics: &[TraceDiagnosticFact],
    link_id: Option<&str>,
) -> Vec<TraceDiagnosticFact> {
    let Some(link_id) = link_id else {
        return diagnostics.to_vec();
    };
    let shell: Vec<crate::check::ShellDiagnostic> = diagnostics
        .iter()
        .map(|diagnostic| crate::check::ShellDiagnostic {
            code: diagnostic.code.clone(),
            path: diagnostic.path.clone(),
            start: diagnostic.start,
            end: diagnostic.end,
            id: diagnostic.id.clone(),
            // `TraceDiagnosticFact` (the publication bridge's own diagnostic
            // shape) carries no separate context field; only `id` matters to
            // `omit_declared_link_diagnostics`'s filter, and this round trip
            // never re-emits `context` back into `TraceDiagnosticFact`.
            context: String::new(),
        })
        .collect();
    crate::publication::omit_declared_link_diagnostics(&shell, link_id)
        .into_iter()
        .map(|diagnostic| TraceDiagnosticFact {
            code: diagnostic.code,
            id: diagnostic.id,
            path: diagnostic.path,
            start: diagnostic.start,
            end: diagnostic.end,
        })
        .collect()
}

/// Exposes the retained typed trace report to the publication module.
///
/// This is the one internal bridge the publication conformance pipeline uses.
/// It never re-derives facts from serialized text: every field comes from the
/// same row values `buoy trace` already rendered. `omitted_link_id`, when
/// present, is applied identically to each row's own diagnostics as
/// [`filter_link_diagnostics`] applies to the top-level report diagnostics,
/// so a stale declared link's own diagnostic can never block the exact
/// operation that exists to repair it, at either granularity.
fn publication_conformance_rows(
    report: &TraceReport,
    omitted_link_id: Option<&str>,
) -> Vec<crate::publication::TraceRow> {
    report
        .rows
        .iter()
        .map(|row| crate::publication::TraceRow {
            relation: Some(row.relation.clone()),
            record: Some(row.record.clone()),
            requirement_id: Some(row.requirement_id.clone()),
            atom_revision: Some(row.atom_revision.clone()),
            requirement_node: Some(row.requirement_node.clone()),
            implementation_node: Some(row.implementation_node.clone()),
            requirement_path: row.requirement_path.clone(),
            implementation_path: row.implementation_path.clone(),
            state: Some(row.state.clone()),
            freshness: Some(crate::publication::RowFreshness {
                record: row.freshness.record,
                attachment: row.freshness.attachment,
                implementation: row.freshness.implementation,
                requirement: row.freshness.requirement,
                authority: row.freshness.authority,
            }),
            diagnostics: filter_link_diagnostics(&row.diagnostics, omitted_link_id)
                .into_iter()
                .map(|diagnostic| diagnostic.code)
                .collect(),
        })
        .collect()
}

fn diagnostic_path(path: Option<&RepositoryPath>) -> Option<String> {
    path.map(|path| path.as_str().to_owned())
}

fn trace_diagnostic_facts(row: &TraceRow) -> Vec<TraceDiagnosticFact> {
    let mut facts = row
        .diagnostics()
        .iter()
        .map(|diagnostic| TraceDiagnosticFact {
            code: diagnostic.code.clone(),
            id: diagnostic.id.clone(),
            path: diagnostic_path(diagnostic.path.as_ref()),
            start: diagnostic.start,
            end: diagnostic.end,
        })
        .collect::<Vec<_>>();
    facts.sort();
    facts.dedup();
    facts
}

/// Builds one machine row from the core projection and retained display data.
///
/// A fresh row reads the shared fresh-link facts, so it cannot disagree with
/// the successful-bind report about the same relation.
fn trace_report_row(checked: &CheckResult, row: &TraceRow) -> TraceReportRow {
    let fresh = if row.state() == TraceState::Current {
        stored_record(checked, row.record()).and_then(|record| fresh_link_facts(checked, record))
    } else {
        None
    };
    let (requirement_path, requirement_kind, requirement_name, requirement_span) =
        node_facts(checked, row.requirement());
    let (implementation_path, implementation_kind, implementation_name, implementation_span) =
        node_facts(checked, row.implementation());
    let traversal = fresh.as_ref().map_or_else(
        || link_nodes(checked, row.implementation(), row.requirement()),
        |facts| facts.nodes.clone(),
    );
    let traversal = traversal
        .into_iter()
        .map(|node| TraceTraversalNode {
            position: node.position,
            digest: node.digest,
            kind: node.kind,
            path: node.path,
        })
        .collect();
    TraceReportRow {
        state: String::from(row.state().name()),
        relation: fresh.as_ref().map_or_else(
            || row.relation().as_str().to_owned(),
            |facts| facts.relation.clone(),
        ),
        navigation: String::from(navigation_name(row.navigation())),
        record: fresh.as_ref().map_or_else(
            || row.record().canonical_text(),
            |facts| facts.binding.clone(),
        ),
        requirement_id: fresh.as_ref().map_or_else(
            || row.atom().id().as_str().to_owned(),
            |facts| facts.requirement_id.clone(),
        ),
        atom_revision: fresh.as_ref().map_or_else(
            || row.atom().revision().canonical_text(),
            |facts| facts.atom_revision.clone(),
        ),
        requirement_node: row.requirement().canonical_text(),
        implementation_node: row.implementation().canonical_text(),
        requirement_path,
        implementation_path,
        requirement_kind,
        implementation_kind,
        requirement_name,
        implementation_name,
        requirement_span,
        implementation_span,
        freshness: TraceFreshnessFact {
            record: row.freshness().record(),
            attachment: row.freshness().attachment(),
            implementation: row.freshness().implementation(),
            requirement: row.freshness().requirement(),
            authority: row.freshness().authority(),
        },
        traversal,
        diagnostics: trace_diagnostic_facts(row),
    }
}

fn trace_report(checked: &CheckResult, projection: &crate::core::TraceProjection) -> TraceReport {
    let rows = projection
        .rows()
        .iter()
        .map(|row| trace_report_row(checked, row))
        .collect::<Vec<_>>();
    let mut diagnostics = rows
        .iter()
        .flat_map(|row| row.diagnostics.iter().cloned())
        .collect::<Vec<_>>();
    diagnostics.sort();
    diagnostics.dedup();
    TraceReport {
        schema: String::from(TRACE_REPORT_SCHEMA),
        graph: projection.graph().canonical_text(),
        selector: trace_selector_fact(projection.selector()),
        include_history: projection.history() == TraceHistory::IncludeHistory,
        rows,
        diagnostics,
    }
}

fn blocked_trace_report(
    checked: &CheckResult,
    selector: &TraceSelector,
    history: TraceHistory,
    error: TraceError,
) -> TraceReport {
    TraceReport {
        schema: String::from(TRACE_REPORT_SCHEMA),
        graph: checked.source.graph().digest().canonical_text(),
        selector: trace_selector_fact(selector),
        include_history: history == TraceHistory::IncludeHistory,
        rows: Vec::new(),
        diagnostics: vec![TraceDiagnosticFact {
            code: String::from(error.code()),
            id: None,
            path: None,
            start: 0,
            end: 0,
        }],
    }
}

/// Hexadecimal payload of one canonical algorithm-qualified digest.
fn digest_hex(canonical: &str) -> &str {
    canonical.split_once(':').map_or(canonical, |(_, hex)| hex)
}

fn abbreviated_digest(canonical: &str) -> String {
    digest_hex(canonical)
        .chars()
        .take(TRACE_DIGEST_PREFIX)
        .collect()
}

/// Whether every displayed record keeps its abbreviated prefix.
///
/// Two distinct records that share one displayed prefix force complete digests
/// for the whole view. One record displayed twice is not a collision.
fn records_stay_abbreviated(report: &TraceReport) -> bool {
    let distinct = report
        .rows
        .iter()
        .map(|row| row.record.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let prefixes = distinct
        .iter()
        .map(|record| abbreviated_digest(record))
        .collect::<std::collections::BTreeSet<_>>();
    prefixes.len() == distinct.len()
}

fn render_trace_human(report: &TraceReport) -> String {
    let abbreviate = records_stay_abbreviated(report);
    let show_record = |record: &str| {
        if abbreviate {
            abbreviated_digest(record)
        } else {
            record.to_owned()
        }
    };
    let mut output = format!(
        "trace: {} {}\ngraph: {}\nrows: {}\n",
        report.selector.kind,
        report.selector.value,
        abbreviated_digest(&report.graph),
        report.rows.len(),
    );
    for row in &report.rows {
        let _ = write!(
            output,
            "\nrelation: {} ({})\nstate: {}\n",
            row.relation, row.navigation, row.state
        );
        let _ = writeln!(
            output,
            "requirement: {} {}",
            row.requirement_id,
            located(row.requirement_path.as_deref(), row.requirement_span),
        );
        if let Some(name) = &row.requirement_name {
            let _ = writeln!(output, "  {name}");
        }
        let _ = writeln!(
            output,
            "implementation: {} {}",
            located(row.implementation_path.as_deref(), row.implementation_span),
            row.implementation_kind.as_deref().unwrap_or("unretained"),
        );
        if let Some(name) = &row.implementation_name {
            let _ = writeln!(output, "  {name}");
        }
        let _ = writeln!(output, "record: {}", show_record(&row.record));
        let _ = writeln!(output, "atom: {}", abbreviated_digest(&row.atom_revision));
        let _ = writeln!(
            output,
            "freshness: record={} attachment={} implementation={} requirement={} authority={}",
            row.freshness.record,
            row.freshness.attachment,
            row.freshness.implementation,
            row.freshness.requirement,
            row.freshness.authority,
        );
        for diagnostic in &row.diagnostics {
            let _ = writeln!(output, "blocker: {}", diagnostic.code);
        }
    }
    let _ = write!(output, "\n{TRACE_BOUNDARY}\n");
    output
}

/// Readable location text for one retained node.
fn located(path: Option<&str>, span: Option<TraceSpanFact>) -> String {
    match (path, span) {
        (Some(path), Some(span)) => format!("{path}:{}-{}", span.start, span.end),
        (Some(path), None) => path.to_owned(),
        (None, _) => String::from("not retained in this snapshot"),
    }
}

fn render_trace_markdown(report: &TraceReport) -> String {
    let mut output = String::from("# Implementation trace\n\n");
    output.push_str(
        "Derived data. Buoy remains the current-state authority. Regenerate this page from a current query.\n\n",
    );
    let _ = writeln!(output, "- Renderer schema: `{TRACE_MARKDOWN_SCHEMA}`");
    let _ = writeln!(output, "- Trace-report schema: `{TRACE_REPORT_SCHEMA}`");
    let _ = writeln!(
        output,
        "- Selector: `{} {}`",
        report.selector.kind, report.selector.value
    );
    let _ = writeln!(output, "- Source graph: `{}`", report.graph);
    let _ = writeln!(output, "- History included: {}\n", report.include_history);

    output.push_str("## Requirement\n\n");
    let mut requirements = report
        .rows
        .iter()
        .map(|row| {
            format!(
                "- `{}` at {}\n",
                row.requirement_id,
                located(row.requirement_path.as_deref(), row.requirement_span)
            )
        })
        .collect::<Vec<_>>();
    requirements.sort();
    requirements.dedup();
    output.extend(requirements);

    output.push_str("\n## Implementation\n\n");
    let mut implementations = report
        .rows
        .iter()
        .map(|row| {
            format!(
                "- {} (`{}`)\n",
                located(row.implementation_path.as_deref(), row.implementation_span),
                row.implementation_kind.as_deref().unwrap_or("unretained"),
            )
        })
        .collect::<Vec<_>>();
    implementations.sort();
    implementations.dedup();
    output.extend(implementations);

    output.push_str("\n## Relations\n\n");
    output.push_str(
        "| State | Relation | Navigation | Record | Requirement | Atom revision | Requirement path | Implementation path | Freshness |\n",
    );
    output.push_str("|---|---|---|---|---|---|---|---|---|\n");
    for row in &report.rows {
        let _ = writeln!(
            output,
            // RTP-007 requires each row to retain ALL freshness facets.
            // `attachment` is a real, independent facet of
            // `buoy/trace-report/v1`'s freshness object -- a binding whose
            // compact locator no longer resolves is attachment-stale even
            // when its record, implementation, requirement, and authority
            // are all current -- so omitting it silently dropped a fact the
            // machine report already carried.
            "| {} | `{}` | {} | `{}` | `{}` | `{}` | {} | {} | record={} attachment={} implementation={} requirement={} authority={} |",
            row.state,
            row.relation,
            row.navigation,
            row.record,
            row.requirement_id,
            row.atom_revision,
            row.requirement_path.as_deref().unwrap_or("unretained"),
            row.implementation_path.as_deref().unwrap_or("unretained"),
            row.freshness.record,
            row.freshness.attachment,
            row.freshness.implementation,
            row.freshness.requirement,
            row.freshness.authority,
        );
    }
    if !report.diagnostics.is_empty() {
        output.push_str("\n## Blockers\n\n");
        for diagnostic in &report.diagnostics {
            let _ = writeln!(output, "- `{}`", diagnostic.code);
        }
    }
    let _ = write!(output, "\n## Boundary\n\n{TRACE_BOUNDARY}\n");
    output
}

const FORBIDDEN_TRACE_FIELDS: [&str; 7] = [
    "root",
    "mount_root",
    "absolute_path",
    "timestamp",
    "locale",
    "terminal_width",
    "provider",
];

fn validate_trace_machine_value(value: &serde_json::Value) -> Result<(), &'static str> {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                if FORBIDDEN_TRACE_FIELDS.contains(&key.as_str()) {
                    return Err("TRC-FORBIDDEN-REPORT-FIELD");
                }
                if (key == "path" || key.ends_with("_path"))
                    && value
                        .as_str()
                        .is_some_and(|path| RepositoryPath::new(path).is_err())
                {
                    return Err("TRC-FORBIDDEN-REPORT-FIELD");
                }
                validate_trace_machine_value(value)?;
            }
            Ok(())
        }
        serde_json::Value::Array(values) => {
            for value in values {
                validate_trace_machine_value(value)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[allow(clippy::expect_used)]
fn render_trace(report: &TraceReport, format: TraceFormat) -> Result<String, &'static str> {
    if cli_coverage_fault(TRACE_RENDER_FAULT) {
        return Err("TRC-COVERAGE-RENDER");
    }
    let value =
        serde_json::to_value(report).expect("closed trace report serialization is infallible");
    validate_trace_machine_value(&value)?;
    Ok(match format {
        TraceFormat::Machine => {
            serde_json::to_string(&value).expect("closed trace report serialization is infallible")
                + "\n"
        }
        TraceFormat::Human => render_trace_human(report),
        TraceFormat::Markdown => render_trace_markdown(report),
    })
}

/// Answers one read-only trace query and renders the selected view.
///
/// The command reads saved bytes only. It invokes no oracle, fetches no target,
/// and writes no repository file.
#[allow(clippy::too_many_lines)]
fn trace_command(options: &CliOptions) -> u8 {
    const TRACE_OPTIONS: [&str; 8] = [
        "config",
        "root",
        "format",
        "requirement-id",
        "relation-id",
        "implementation-path",
        "record",
        "include-history",
    ];
    if options
        .values
        .keys()
        .any(|key| !TRACE_OPTIONS.contains(&key.as_str()))
    {
        return usage();
    }
    let Some(config_path) = options.get("config").map(PathBuf::from) else {
        return usage();
    };
    let Ok(format) = parse_trace_format(options.get("format")) else {
        return usage();
    };
    let selector = match parse_trace_selector(options) {
        Ok(selector) => selector,
        Err(SelectorFault::Usage) => return usage(),
        Err(SelectorFault::Input(code)) => {
            eprintln!("{code}");
            return EXIT_INPUT;
        }
    };
    let history = if options.get("include-history").is_some() {
        TraceHistory::IncludeHistory
    } else {
        TraceHistory::CurrentOnly
    };
    let Ok(config) = load_config(&config_path) else {
        return EXIT_INPUT;
    };
    let Ok((snapshot, _)) = invocation_snapshot(options, &config) else {
        return EXIT_INPUT;
    };
    let checked = if cli_coverage_fault(TRACE_CHECK_FAULT) {
        None
    } else {
        check_snapshot(&snapshot, &config).ok()
    };
    let Some(checked) = checked else {
        return EXIT_INPUT;
    };
    let requirement_kind = if cli_coverage_fault(TRACE_KIND_FAULT) {
        Err(())
    } else {
        SourceItemKind::new("openspec/requirement/v1").map_err(drop)
    };
    let Ok(requirement_kind) = requirement_kind else {
        return EXIT_INPUT;
    };
    let context = if cli_coverage_fault(TRACE_CONTEXT_FAULT) {
        Err(TraceError::DuplicateRecord)
    } else {
        TraceContext::new(
            checked.source.graph(),
            checked.source.closure(),
            requirement_kind,
            checked.source.authorities(),
            checked.source.attachments(),
            checked.source.store(),
        )
    };
    let context = match context {
        Ok(context) => context,
        Err(error) => {
            let report = blocked_trace_report(&checked, &selector, history, error);
            let Ok(rendered) = render_trace(&report, format) else {
                return EXIT_STATIC;
            };
            print!("{rendered}");
            return EXIT_STATIC;
        }
    };
    let projection = match if cli_coverage_fault(TRACE_QUERY_FAULT) {
        Err(TraceError::DuplicateRecord)
    } else {
        trace_query(&context, &selector, history)
    } {
        Ok(projection) => projection,
        Err(TraceError::SelectorNotFound) => {
            eprintln!("{}", TraceError::SelectorNotFound.code());
            return EXIT_INPUT;
        }
        Err(error) => {
            let report = blocked_trace_report(&checked, &selector, history, error);
            let Ok(rendered) = render_trace(&report, format) else {
                return EXIT_STATIC;
            };
            print!("{rendered}");
            return EXIT_STATIC;
        }
    };
    let mut report = trace_report(&checked, &projection);
    if cli_coverage_fault(TRACE_DIAGNOSTIC_FAULT) {
        report.diagnostics.push(TraceDiagnosticFact {
            code: String::from("TRC-COVERAGE-DIAGNOSTIC"),
            id: None,
            path: None,
            start: 0,
            end: 0,
        });
    }
    let rendered = match render_trace(&report, format) {
        Ok(rendered) => rendered,
        Err(code) => {
            eprintln!("{code}");
            return EXIT_STATIC;
        }
    };
    print!("{rendered}");
    // The projection reaches the caller before any static blocker, and a
    // historical row never sets this fact.
    if projection.has_stale_attachment() || !report.diagnostics.is_empty() {
        EXIT_STATIC
    } else {
        EXIT_OK
    }
}

fn change_command(options: &CliOptions) -> u8 {
    let (Some(base), Some(head), Some(config_path), Some(manifest_path)) = (
        options.get("base").map(PathBuf::from),
        options.get("head").map(PathBuf::from),
        options.get("config").map(PathBuf::from),
        options.get("manifest").map(PathBuf::from),
    ) else {
        return usage();
    };
    let (Ok((base_result, _)), Ok((head_result, head_artifacts))) = (
        load_and_check_with_artifacts(&base, &config_path),
        load_and_check_with_artifacts(&head, &config_path),
    ) else {
        return EXIT_INPUT;
    };
    let parsed_manifest = parse_manifest(&manifest_path);
    let Ok(parsed_manifest) = parsed_manifest else {
        return EXIT_INPUT;
    };
    let manifest = parsed_manifest.manifest;
    let mut diagnostics = head_result.diagnostics;
    if manifest.base_graph() != base_result.graph_digest
        || manifest.head_graph() != head_result.graph_digest
    {
        diagnostics.push(shell_diagnostic("CHG-IMPACT-GRAPH-BINDING", None));
    }
    let impacts = change_impacts(&base_result.graph, &head_result.graph);
    let ratchets = resolve_policy_ratchets(
        derive_policy_ratchets(&base_result.graph, &head_result.graph),
        &mut diagnostics,
    );
    let protocol_context = DispositionProtocolContext::new(
        &base_result.graph,
        &head_result.graph,
        &head_artifacts,
        &ratchets,
    );
    diagnostics.extend(
        disposition_protocol_closure(&impacts, &manifest, &protocol_context)
            .into_iter()
            .map(|diagnostic| ShellDiagnostic {
                code: diagnostic.code,
                path: None,
                start: 0,
                end: 0,
                id: diagnostic.id,
                context: String::new(),
            }),
    );
    append_ratchet_diagnostics(&mut diagnostics, &ratchets);
    diagnostics.sort_unstable();
    diagnostics.dedup();
    let impact_keys = impacts
        .iter()
        .map(|impact| format!("{}:{}", impact.kind as u16, impact.id.as_str()))
        .collect();
    let report = MachineReport {
        command: String::from("change"),
        graph: head_result.graph_digest.canonical_text(),
        diagnostics,
        migration: head_result.migration,
        verdicts: BTreeMap::new(),
        impacts: impact_keys,
        mounts: Vec::new(),
        requirement_subjects: Vec::new(),
        opaque_subjects: Vec::new(),
        artifact_attachments: Vec::new(),
    };
    if emit(&report, options.get("format").map(String::as_str)).is_err() {
        return EXIT_INPUT;
    }
    if report
        .diagnostics
        .iter()
        .any(|diagnostic| change_diagnostic_blocks(&diagnostic.code))
    {
        EXIT_CHANGE
    } else {
        EXIT_OK
    }
}

fn resolve_policy_ratchets(
    result: Result<Vec<DerivedPolicyRatchet>, PolicyAnalysisError>,
    diagnostics: &mut Vec<ShellDiagnostic>,
) -> Vec<DerivedPolicyRatchet> {
    match result {
        Ok(ratchets) => ratchets,
        Err(error) => {
            diagnostics.push(shell_diagnostic(error.code(), None));
            Vec::new()
        }
    }
}

fn append_ratchet_diagnostics(
    diagnostics: &mut Vec<ShellDiagnostic>,
    ratchets: &[DerivedPolicyRatchet],
) {
    for ratchet in ratchets {
        diagnostics.extend(
            ratchet
                .diagnostics()
                .iter()
                .map(|code| shell_diagnostic(code, Some(ratchet.policy_id().as_str().to_string()))),
        );
    }
}

/// Exercises deterministic shell projection for pre-derived ratchets.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
#[must_use]
pub fn ratchet_diagnostics_control(ratchets: &[DerivedPolicyRatchet]) -> Vec<ShellDiagnostic> {
    let mut diagnostics = Vec::new();
    append_ratchet_diagnostics(&mut diagnostics, ratchets);
    diagnostics
}

fn change_diagnostic_blocks(code: &str) -> bool {
    !matches!(
        code,
        "CHG-RATCHET-AUTHORITY"
            | "CHG-RATCHET-DERIVATION"
            | "CHG-RATCHET-LINEAGE"
            | "CHG-RATCHET-MODE"
            | "CHG-RATCHET-RETIREMENT"
            | "CHG-RATCHET-ROLE"
            | "CHG-RATCHET-SCOPE"
            | "CHG-RATCHET-SUPERSESSION"
            | "CHG-RATCHET-SURFACE"
            | "CHG-RATCHET-WAIVER-CLASS"
    )
}

#[allow(clippy::expect_used)]
fn execute_command(options: &CliOptions) -> u8 {
    execute_command_internal(options, None)
}

fn execute_command_internal(options: &CliOptions, fault: Option<&str>) -> u8 {
    let Some(config_path) = options.get("config").map(PathBuf::from) else {
        return usage();
    };
    let Ok(config) = load_config(&config_path) else {
        return EXIT_INPUT;
    };
    let Ok((snapshot, roots)) = invocation_snapshot(options, &config) else {
        return EXIT_INPUT;
    };
    let Ok(result) = check_snapshot(&snapshot, &config) else {
        return EXIT_INPUT;
    };
    let receipts = match roots {
        InvocationRoots::Single(root) => run_oracles_controlled(
            config.oracles(),
            &root,
            config.oracle_concurrency(),
            &CancellationToken::new(),
        ),
        InvocationRoots::Mounted(mappings) => {
            let working_roots = if fault == Some("working-roots") {
                None
            } else {
                config
                    .oracles()
                    .iter()
                    .map(|oracle| {
                        LogicalMountId::new(&oracle.working_root)
                            .ok()
                            .and_then(|id| mappings.root(&id))
                            .map(Path::to_path_buf)
                    })
                    .collect::<Option<Vec<_>>>()
            };
            let Some(working_roots) = working_roots else {
                return EXIT_INPUT;
            };
            run_oracles_at_roots_controlled(
                config.oracles(),
                &working_roots,
                config.oracle_concurrency(),
                &CancellationToken::new(),
            )
        }
    };
    let receipts = if fault == Some("receipts") {
        Err(crate::oracle::OracleSetError::Worker)
    } else {
        receipts
    };
    let Ok(receipts) = receipts else {
        return EXIT_INPUT;
    };
    let mut verdicts = BTreeMap::new();
    let mut execution_failed = false;
    for receipt in receipts {
        execution_failed |= receipt.verdict != crate::core::Verdict::Pass;
        verdicts.insert(receipt.id, verdict_name(receipt.verdict).to_string());
    }
    let (mounts, opaque_subjects, artifact_attachments) = mounted_report_facts(&config, &result);
    let report = MachineReport {
        command: String::from("execute"),
        graph: result.graph_digest.canonical_text(),
        diagnostics: result.diagnostics,
        migration: result.migration,
        verdicts,
        impacts: Vec::new(),
        mounts,
        requirement_subjects: Vec::new(),
        opaque_subjects,
        artifact_attachments,
    };
    if emit(&report, options.get("format").map(String::as_str)).is_err() {
        return EXIT_INPUT;
    }
    if !report.diagnostics.is_empty() {
        EXIT_STATIC
    } else if execution_failed {
        EXIT_EXECUTION
    } else {
        EXIT_OK
    }
}

enum InvocationRoots {
    Single(PathBuf),
    Mounted(MountMappings),
}

fn binding_write_root(roots: &InvocationRoots, mount: Option<&LogicalMountId>) -> Option<PathBuf> {
    match roots {
        InvocationRoots::Single(root) => Some(root.clone()),
        InvocationRoots::Mounted(mappings) => mount
            .and_then(|mount| mappings.root(mount))
            .map(Path::to_path_buf),
    }
}

/// Converts one declared output's mount identity and mount-relative rendered
/// path into the single canonical LOGICAL path that the retained snapshot's
/// own `selected` source paths already use: the mount-prefixed projected
/// path in mounted mode, and the bare mount-relative path in single-root
/// mode (which declares no mount namespace to project into).
///
/// This is the ONE function every source-set membership check, output
/// classification check, path-collision check, and mounted-snapshot lookup
/// MUST call -- never a second, independently written prefixing. A physical
/// descent (`publication_open_parent`) uses the separate, un-prefixed
/// mount-relative string directly against the physical root that mount
/// already resolves to, so the mount prefix is applied here exactly once,
/// only for the logical comparison namespace, and never for the physical
/// descent path.
///
/// # Errors
///
/// Returns `Err(())` for an undeclared or malformed mount identity in
/// mounted mode, or an invalid mount-relative path.
fn output_logical_path(
    mount_field: &str,
    mount_relative: &str,
    config: &RepositoryConfig,
) -> Result<String, ()> {
    if config.is_mounted() {
        let mount_id = LogicalMountId::new(mount_field).map_err(drop)?;
        let mount = config.mount(&mount_id).ok_or(())?;
        let relative = RepositoryPath::new(mount_relative).map_err(drop)?;
        crate::core::project_logical_path(mount, &relative)
            .map(|logical| logical.as_str().to_owned())
            .map_err(drop)
    } else {
        Ok(mount_relative.to_owned())
    }
}

fn invocation_snapshot(
    options: &CliOptions,
    config: &RepositoryConfig,
) -> Result<(RepositorySnapshot, InvocationRoots), ()> {
    if config.is_mounted() {
        if options.get("root").is_some() || options.mounts().is_empty() {
            return Err(());
        }
        let mappings = MountMappings::parse(options.mounts()).map_err(drop)?;
        let snapshot = snapshot_mounted_repository(&mappings, config).map_err(drop)?;
        Ok((snapshot, InvocationRoots::Mounted(mappings)))
    } else {
        if !options.mounts().is_empty() {
            return Err(());
        }
        let root = options.get("root").map(PathBuf::from).ok_or(())?;
        let snapshot = snapshot_repository(&root, config).map_err(drop)?;
        Ok((snapshot, InvocationRoots::Single(root)))
    }
}

#[cfg(test)]
fn load_and_check(root: &Path, config_path: &Path) -> Result<crate::check::CheckResult, ()> {
    load_and_check_with_artifacts(root, config_path).map(|(result, _)| result)
}

#[allow(clippy::expect_used)]
fn load_and_check_with_artifacts(
    root: &Path,
    config_path: &Path,
) -> Result<(crate::check::CheckResult, crate::core::ArtifactIndex), ()> {
    let config = load_config(config_path).map_err(drop)?;
    let snapshot = snapshot_repository(root, &config).map_err(drop)?;
    let artifacts = snapshot
        .artifact_index_with(config.active_algorithm())
        .expect("repository snapshots materialize unique artifact paths");
    let result = check_snapshot(&snapshot, &config).map_err(drop)?;
    Ok((result, artifacts))
}

/// Returns every digest-algorithm feature actually compiled into this
/// binary, in whatever order `cfg(feature)` checks encounter them; callers
/// that need canonical order (such as [`crate::publication::PackageIdentity`])
/// sort it themselves.
fn compiled_digest_features() -> Vec<&'static str> {
    let mut features = Vec::new();
    if cfg!(feature = "xxh3-128") {
        features.push("xxh3-128");
    }
    if cfg!(feature = "blake3-256") {
        features.push("blake3-256");
    }
    if cfg!(feature = "sha-256") {
        features.push("sha-256");
    }
    features
}

/// Bridges one typed publication selector into the same [`TraceSelector`]
/// `buoy trace` parses, so both commands resolve one selector identically.
fn publication_core_selector(selector: &crate::publication::Selector) -> Result<TraceSelector, ()> {
    use crate::publication::Selector;
    match selector {
        Selector::RequirementId(value) => StableId::new(value)
            .map(TraceSelector::RequirementId)
            .map_err(drop),
        Selector::RelationId(value) => StableId::new(value)
            .map(TraceSelector::RelationId)
            .map_err(drop),
        Selector::ImplementationPath(value) => RepositoryPath::new(value)
            .map(TraceSelector::ImplementationPath)
            .map_err(drop),
        Selector::Record(value) => ImplementationBindingRef::parse(value)
            .map(TraceSelector::Record)
            .map_err(drop),
    }
}

/// The outcome of opening one declared output's parent directory.
enum OpenParentOutcome {
    /// The parent directory exists (or was created, when `create` was
    /// requested); the descriptor and file name are ready for a read or
    /// write.
    Ready(rustix::fd::OwnedFd, String),
    /// A read-only descent (`create = false`) found the parent directory
    /// itself absent. The declared output is therefore absent, with no
    /// incidental directory created to observe that fact.
    Absent,
}

fn absent_parent(create: bool) -> Result<OpenParentOutcome, crate::publication_fs::WriteError> {
    if create {
        Err(crate::publication_fs::WriteError::Absent)
    } else {
        Ok(OpenParentOutcome::Absent)
    }
}

/// Opens one declared output's parent directory through the no-follow
/// descriptor chain, from the resolved physical root through every declared
/// path component.
///
/// `create` must be `true` only for `generate`, which is the only operation
/// allowed to create a missing directory. `check` and `evidence` always
/// pass `create = false`, so neither ever creates a directory as a side
/// effect of parent descent: the repository stays byte- and
/// directory-identical before and after either operation runs.
/// Every failure below returns the REAL [`crate::publication_fs::WriteError`]
/// rather than a bare `()`. This is what lets the caller emit the exact
/// stable code the fault deserves -- above all `TPT-OUTPUT-LINK` for a
/// symlinked or otherwise link-like ancestor, which is a redirection attempt
/// a caller must never see collapsed into a generic failure.
fn publication_open_parent(
    output: &crate::publication::PublicationOutput,
    rendered_path: &str,
    roots: &InvocationRoots,
    create: bool,
) -> Result<OpenParentOutcome, crate::publication_fs::WriteError> {
    let mount = LogicalMountId::new(&output.mount).ok();
    let physical_root =
        binding_write_root(roots, mount.as_ref()).ok_or(crate::publication_fs::WriteError::Io)?;
    let (components, filename) = crate::publication_fs::split_relative_path(rendered_path)?;
    let root_fd = crate::publication_fs::unix::open_root(&physical_root)?;
    match crate::publication_fs::unix::descend_parents(&root_fd, &components, create) {
        Ok(parent_fd) => Ok(OpenParentOutcome::Ready(parent_fd, filename.to_owned())),
        Err(crate::publication_fs::WriteError::Absent) => absent_parent(create),
        Err(error) => Err(error),
    }
}

/// The one unambiguous value every row of a selected trace agrees on, using
/// `picker` to extract each row's own candidate value.
///
/// Returns `None` for an empty row set or a row set whose rows disagree, so
/// a template that needs this substitution fails closed rather than pick an
/// arbitrary row's value.
#[derive(Clone, Copy)]
enum RowIdentity {
    AtomRevision,
    Record,
}

fn row_identity(row: &TraceReportRow, identity: RowIdentity) -> &str {
    match identity {
        RowIdentity::AtomRevision => row.atom_revision.as_str(),
        RowIdentity::Record => row.record.as_str(),
    }
}

fn unambiguous_row_value(rows: &[TraceReportRow], identity: RowIdentity) -> Option<&str> {
    let first = row_identity(rows.first()?, identity);
    for row in &rows[1..] {
        if row_identity(row, identity) != first {
            return None;
        }
    }
    Some(first)
}

/// Derives this operation's typed path identities: the computed publication
/// basis, and -- independently -- the selected trace's own unambiguous atom
/// revision and binding-record digest, so `{atom_revision_path}` and
/// `{record_digest_path}` substitutions render distinct declared values
/// instead of collapsing to the basis.
fn publication_path_identities<'a>(
    basis: &'a str,
    report: &'a TraceReport,
) -> crate::publication::PathIdentities<'a> {
    crate::publication::PathIdentities {
        basis,
        atom_revision: unambiguous_row_value(&report.rows, RowIdentity::AtomRevision),
        record_digest: unambiguous_row_value(&report.rows, RowIdentity::Record),
    }
}

/// Renders one declared page output's candidate bytes and logical path.
///
/// The rendered candidate is run through RTP-007 projection conformance
/// before it is returned. That check is not redundant with the renderer's own
/// tests: an immutable publication target is keyed by a basis that binds the
/// renderer SCHEMA, so a renderer that silently stopped emitting a required
/// label or freshness facet under the unchanged schema would publish a
/// permanently addressed page whose identity claims a contract its bytes no
/// longer satisfy. Validating the actual candidate bytes here means no such
/// page can ever be written or reported at parity.
fn publication_page_candidate(
    output: &crate::publication::PublicationOutput,
    basis: &str,
    report: &TraceReport,
) -> Result<(String, Vec<u8>), crate::publication::ProjectionError> {
    let identities = publication_path_identities(basis, report);
    let rendered_path = crate::publication::render_path(&output.path, &identities)
        .map_err(|_| crate::publication::ProjectionError::MissingSchemaIdentity)?;
    let candidate = render_trace_markdown(report);
    crate::publication::validate_markdown_projection(&candidate)?;
    Ok((rendered_path, candidate.into_bytes()))
}

/// Compares one declared artifact's existing bytes with its candidate,
/// writing nothing. A page artifact compares real reopened bytes against the
/// real rendered candidate. Every other declared kind currently has no
/// candidate renderer this pipeline can call, so it fails closed with
/// [`Err`] rather than fabricate a pass.
#[allow(clippy::too_many_arguments)]
fn publication_artifact_parity(
    output: &crate::publication::PublicationOutput,
    basis: &str,
    report: &TraceReport,
    plan: &crate::publication::TracePublicationPlan,
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    algorithm: crate::core::DigestAlgorithm,
    roots: &InvocationRoots,
) -> Result<bool, ()> {
    match output.kind {
        crate::publication::ArtifactKind::Page => {
            let (rendered_path, candidate) =
                publication_page_candidate(output, basis, report).map_err(drop)?;
            // `evidence` never writes, so it must never create a missing
            // output directory as a side effect of parent descent either.
            match publication_open_parent(output, &rendered_path, roots, false).map_err(drop)? {
                OpenParentOutcome::Ready(parent_fd, filename) => {
                    let existing = crate::publication_fs::unix::read_regular(&parent_fd, &filename)
                        .map_err(drop)?;
                    Ok(existing.as_deref() == Some(candidate.as_slice()))
                }
                OpenParentOutcome::Absent => Ok(false),
            }
        }
        crate::publication::ArtifactKind::ExternalLink => {
            let (_, existing, candidate) = publication_external_link_candidate(
                output, plan, snapshot, config, algorithm, basis, report,
            )?;
            Ok(existing == candidate)
        }
        crate::publication::ArtifactKind::SourceEdit => {
            // A source-edit output has no Buoy-rendered candidate (Buoy
            // never authors a human source edit), so its real conformance
            // fact is a genuine byte comparison between the retained
            // snapshot this operation captured and the file's current
            // on-disk bytes -- not an unconditional pass. This closes the
            // exact gap TPT-004 requires: `evidence` must never treat a
            // source edit as conforming without comparing a single byte.
            let logical = output_logical_path(&output.mount, &output.path, config)?;
            let retained = snapshot
                .files
                .iter()
                .find(|file| file.path.as_str() == logical)
                .map(|file| file.bytes.clone())
                .ok_or(())?;
            match publication_open_parent(output, &output.path, roots, false).map_err(drop)? {
                OpenParentOutcome::Ready(parent_fd, filename) => {
                    let current = crate::publication_fs::unix::read_regular(&parent_fd, &filename)
                        .map_err(drop)?;
                    Ok(current.as_deref() == Some(retained.as_slice()))
                }
                OpenParentOutcome::Absent => Ok(false),
            }
        }
        crate::publication::ArtifactKind::Evidence => Err(()),
    }
}

/// Runs the declared page operation: `check` compares saved bytes with the
/// rendered candidate and writes nothing; `generate` writes the candidate
/// through the locked, no-follow, atomic publication writer.
/// Prints one publication operation report to standard output. A render
/// failure (a claimed cross-repository scope, or an empty required field)
/// is a defensive impossibility for reports this pipeline itself builds;
/// it degrades to no output rather than panic, since printing is the last
/// pipeline step and the caller's exit code remains the operation contract.
fn print_publication_report(
    report: &crate::publication::PublicationReport,
    format: PublicationFormat,
) {
    match format {
        PublicationFormat::Machine => {
            let rendered = if cli_coverage_fault(PUBLICATION_REPORT_FAULT) {
                crate::publication::render_report(&crate::publication::PublicationReport::new(
                    "", "",
                ))
            } else {
                crate::publication::render_report(report)
            };
            if let Ok(rendered) = rendered {
                print!("{}", String::from_utf8_lossy(&rendered));
            }
        }
        PublicationFormat::Human => {
            let rendered = if cli_coverage_fault(PUBLICATION_REPORT_FAULT) {
                crate::publication::render_report_human(
                    &crate::publication::PublicationReport::new("", ""),
                )
            } else {
                crate::publication::render_report_human(report)
            };
            if let Ok(rendered) = rendered {
                print!("{rendered}");
            }
        }
    }
}

/// The two output encodings one publication operation can emit.
///
/// `Machine` is the automation contract
/// (`buoy/trace-publication-report/v1`). `Human` is the deterministic
/// projection of that same typed value, which additionally carries the
/// fixed-label parity summary a reader needs in order to see WHICH bytes
/// differ.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublicationFormat {
    Machine,
    Human,
}

fn write_outcome_label(outcome: crate::publication_fs::WriteOutcome) -> &'static str {
    match outcome {
        crate::publication_fs::WriteOutcome::Created => "created",
        crate::publication_fs::WriteOutcome::NoOp => "no-op",
        crate::publication_fs::WriteOutcome::Replaced => "replaced",
        crate::publication_fs::WriteOutcome::Conflict => "conflict",
    }
}

impl PublicationFormat {
    /// Parses the `--format` option shared with every other Buoy command.
    fn parse(value: Option<&String>) -> Result<Self, ()> {
        match value.map_or("machine", String::as_str) {
            "machine" => Ok(Self::Machine),
            "human" => Ok(Self::Human),
            _ => Err(()),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn publication_page_operation(
    operation: &str,
    output: &crate::publication::PublicationOutput,
    plan_id: &str,
    basis: &str,
    graph: &str,
    report: &TraceReport,
    algorithm: crate::core::DigestAlgorithm,
    roots: &InvocationRoots,
    format: PublicationFormat,
) -> u8 {
    let base_report = crate::publication::PublicationReport::new(operation, &output.id)
        .with_plan_id(plan_id)
        .with_basis(basis)
        .with_graph(graph);
    if let Err(error) = crate::publication::require_collision_resistant_member(algorithm) {
        if matches!(output.lifecycle, crate::publication::Lifecycle::Immutable) {
            print_publication_report(
                &base_report.with_diagnostic(
                    error.stable_code(),
                    "an immutable publication target requires a collision-resistant digest member",
                ),
                format,
            );
            return EXIT_INPUT;
        }
    }
    let (rendered_path, candidate) = match publication_page_candidate(output, basis, report) {
        Ok(candidate) => candidate,
        Err(error) => {
            // RTP-007: the rendered page candidate failed projection
            // conformance. An immutable target is keyed by a basis that binds
            // the renderer schema, so publishing a page whose bytes no longer
            // satisfy that schema's contract would permanently address a
            // false claim. Report the exact `RTP-*` code and write nothing.
            print_publication_report(
                &base_report.with_outcome("blocked").with_diagnostic(
                    error.stable_code(),
                    "the rendered page candidate failed publication projection conformance",
                ),
                format,
            );
            return EXIT_STATIC;
        }
    };
    publication_derived_write_operation(
        operation,
        output,
        base_report,
        &rendered_path,
        &candidate,
        algorithm,
        roots,
        format,
    )
}

/// Runs the declared `evidence`-kind artifact operation: `check` compares
/// saved Markdown with the rendered candidate and writes nothing;
/// `generate` writes the candidate through the identical locked, no-follow,
/// atomic publication writer a page uses. The candidate is the same
/// complete evidence value the whole-plan `evidence` operation computes,
/// rendered to deterministic Markdown rather than machine JSON.
#[allow(clippy::too_many_arguments)]
fn publication_evidence_operation(
    operation: &str,
    output: &crate::publication::PublicationOutput,
    plan: &crate::publication::TracePublicationPlan,
    basis: &str,
    graph: &str,
    report: &TraceReport,
    rows: &[crate::publication::TraceRow],
    canonical: &[(String, Vec<u8>)],
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    algorithm: crate::core::DigestAlgorithm,
    roots: &InvocationRoots,
    format: PublicationFormat,
) -> u8 {
    let base_report = crate::publication::PublicationReport::new(operation, &output.id)
        .with_plan_id(plan.publication_id())
        .with_basis(basis)
        .with_graph(graph);
    if matches!(output.lifecycle, crate::publication::Lifecycle::Immutable)
        && crate::publication::require_collision_resistant_member(algorithm).is_err()
    {
        // Round-5 finding H6: the digest-class rejection on the EVIDENCE
        // artifact path returned a bare exit code. An immutable artifact
        // keyed by a change-detection member is the same contract violation
        // here as on the page path, and reports the same stable code.
        print_publication_report(
            &base_report.with_outcome("blocked").with_diagnostic(
                "TPT-DIGEST-CLASS",
                "an immutable artifact requires a collision-resistant digest member",
            ),
            format,
        );
        return EXIT_INPUT;
    }
    let (rendered_path, candidate) = match publication_evidence_candidate(
        output, plan, basis, report, rows, canonical, snapshot, config, algorithm, roots,
    ) {
        Ok(candidate) => candidate,
        Err(exit) => return exit,
    };
    publication_derived_write_operation(
        operation,
        output,
        base_report,
        &rendered_path,
        &candidate,
        algorithm,
        roots,
        format,
    )
}

/// Shared `check`/`generate` write pipeline for a derived artifact whose
/// candidate bytes and rendered logical path are already computed: compares
/// saved bytes with the candidate for `check`, and performs one locked,
/// no-follow, atomic publication write for `generate`.
// This is one linear, order-sensitive pipeline: open the parent, read the
// prior state, compute parity, take the output lock, write, then release the
// lock. Splitting it would mean threading the parent descriptor, the prior
// bytes, the lock guard, and the partially built report through additional
// signatures, which obscures rather than clarifies the one ordering that
// makes it correct.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn publication_derived_write_operation(
    operation: &str,
    output: &crate::publication::PublicationOutput,
    base_report: crate::publication::PublicationReport,
    rendered_path: &str,
    candidate: &[u8],
    algorithm: crate::core::DigestAlgorithm,
    roots: &InvocationRoots,
    format: PublicationFormat,
) -> u8 {
    let candidate_digest = if cli_coverage_fault(DERIVED_DIGEST_FAULT) {
        Err(crate::publication::PublicationError::DigestMemberUnavailable)
    } else {
        crate::publication::page_content_digest(algorithm, candidate)
    };
    let Ok(candidate_digest) = candidate_digest else {
        // Round-5 finding H6: an unavailable digest member returned a bare
        // exit code from the shared derived-write path, so every artifact
        // kind lost its machine document at once.
        print_publication_report(
            &base_report.with_outcome("blocked").with_diagnostic(
                "TPT-DIGEST-MEMBER-UNAVAILABLE",
                "the configured digest member has no adapter in this build",
            ),
            format,
        );
        return EXIT_INPUT;
    };
    // Only `generate` may create a missing output directory; `check` must
    // never do so as a side effect of parent descent.
    let create = operation == "generate";
    let opened = match publication_open_parent(output, rendered_path, roots, create) {
        Ok(opened) => opened,
        Err(error) => {
            // REH-012: a link-like ancestor, a root escape, or any other
            // descent fault must emit its own stable code. `TPT-OUTPUT-LINK`
            // in particular tells the caller a parent component could
            // redirect this write outside the declared publication root.
            print_publication_report(
                &base_report
                    .with_outcome("blocked")
                    .with_diagnostic(error.stable_code(), error.message()),
                format,
            );
            return EXIT_STATIC;
        }
    };
    let (parent_fd, filename, existing) = match opened {
        OpenParentOutcome::Ready(parent_fd, filename) => {
            let existing = crate::publication_fs::unix::read_regular(&parent_fd, &filename);
            (Some(parent_fd), filename, existing)
        }
        OpenParentOutcome::Absent => (None, String::new(), Ok(None)),
    };
    let (prior_state, prior_digest) = match &existing {
        Ok(None) => ("absent".to_owned(), None),
        Ok(Some(bytes)) => {
            let digest = crate::publication::page_content_digest(algorithm, bytes).ok();
            let state = if *bytes == candidate {
                "equal"
            } else {
                "unequal"
            };
            (state.to_owned(), digest)
        }
        Err(error) => {
            // A non-regular existing target (`TPT-OUTPUT-NOT-REGULAR`) or a
            // symlinked final component (`TPT-OUTPUT-LINK`) is a real,
            // separately actionable safety outcome observed while reading
            // the prior state, not a generic input fault.
            print_publication_report(
                &base_report
                    .with_outcome("blocked")
                    .with_diagnostic(error.stable_code(), error.message()),
                format,
            );
            return EXIT_STATIC;
        }
    };
    let base_report = base_report
        .with_prior(&prior_state, prior_digest.as_deref())
        .with_candidate_digest(&candidate_digest);
    // TPT-009: attach the fixed-label, deterministic parity summary for
    // every non-equal comparison. This is the ONE production wiring of
    // `render_parity_diff`.
    //
    // Round-5 finding H1: this call site used to treat a parity-rendering
    // failure as a blocked outcome and return early. Combined with a
    // renderer that quoted document content, that let ARBITRARY EXISTING
    // PAGE BYTES decide the machine exit class -- a page that merely
    // contained a date or a logical path was reported as
    // `TPT-FORBIDDEN-PUBLICATION-FIELD` instead of the immutable conflict
    // the caller actually needed to see.
    //
    // The parity detail is now renderer-owned metadata only and attaching it
    // is infallible, so the comparison below determines the outcome and the
    // parity summary can never override, mask, or reorder it.
    let base_report = if prior_state == "equal" {
        base_report
    } else {
        let existing = match &existing {
            Ok(bytes) => bytes.as_deref(),
            Err(_) => None,
        };
        base_report.with_parity(existing, candidate)
    };
    if operation == "check" {
        let outcome = if prior_state == "equal" {
            "no-op"
        } else {
            "blocked"
        };
        print_publication_report(&base_report.clone().with_outcome(outcome), format);
        return if prior_state == "equal" {
            EXIT_OK
        } else {
            EXIT_STATIC
        };
    }
    // `generate` always passed `create = true`, so `parent_fd` is always
    // `Some` here; only `check` (already returned above) can observe
    // `None`.
    let parent_fd = if cli_coverage_fault(DERIVED_PARENT_FAULT) {
        None
    } else {
        parent_fd
    };
    let Some(parent_fd) = parent_fd else {
        return EXIT_INPUT;
    };
    let lock_name = format!(".{filename}.publication-lock");
    let lock_guard = if cli_coverage_fault(DERIVED_LOCK_FAULT) {
        Err(crate::publication_fs::WriteError::Busy)
    } else {
        crate::publication_fs::unix::acquire_output_lock(&parent_fd, &lock_name, 20)
    };
    let lock_guard = match lock_guard {
        Ok(guard) => guard,
        Err(error) => {
            // REH-013/TPT-009: a busy or unusable output lock is a real,
            // distinguishable outcome with its own stable code, not a bare
            // exit code. A caller that sees `TPT-OUTPUT-BUSY` knows to
            // retry; one that sees `TPT-OUTPUT-IO` knows not to.
            print_publication_report(
                &base_report
                    .with_outcome("blocked")
                    .with_diagnostic(error.stable_code(), error.message()),
                format,
            );
            return EXIT_STATIC;
        }
    };
    let written = if cli_coverage_fault(DERIVED_WRITE_FAULT) {
        Err(crate::publication_fs::WriteError::Io)
    } else {
        crate::publication_fs::unix::write_output(
            &parent_fd,
            &filename,
            candidate,
            &output.lifecycle,
        )
    };
    let result = match written {
        Ok(
            outcome @ (crate::publication_fs::WriteOutcome::Created
            | crate::publication_fs::WriteOutcome::NoOp
            | crate::publication_fs::WriteOutcome::Replaced),
        ) => {
            let label = write_outcome_label(outcome);
            print_publication_report(&base_report.with_outcome(label), format);
            EXIT_OK
        }
        Ok(crate::publication_fs::WriteOutcome::Conflict) => {
            print_publication_report(&base_report.with_outcome("conflict"), format);
            EXIT_STATIC
        }
        Err(error) => {
            // REH-012/TPT-009: every writer fault -- a symlinked ancestor,
            // a root escape, a non-regular existing target, or an I/O
            // failure -- must emit a versioned report naming its own stable
            // code. A symlinked ancestor in particular (`TPT-OUTPUT-LINK`)
            // is a redirection attempt a caller must be able to distinguish
            // from an ordinary write failure.
            print_publication_report(
                &base_report
                    .with_outcome("blocked")
                    .with_diagnostic(error.stable_code(), error.message()),
                format,
            );
            EXIT_STATIC
        }
    };
    // The operation-owned lock file must never remain after the operation
    // completes: a leftover lock file under a selected source root could
    // otherwise itself enter a later exact source-set selection. Removal
    // happens inside the guard, which unlinks the entry only while still
    // holding the `flock` and only when the name still resolves to the very
    // inode this operation locked -- the one ordering that cannot delete a
    // lock another operation has already legitimately acquired.
    lock_guard.release_and_unlink(&parent_fd, &lock_name);
    result
}

/// Counts diagnostics whose code does not begin with `CAB-`.
///
/// A compact content-addressed implementation binding pins the exact digest
/// of its bound document, so *any* byte-level edit to that document --
/// including a declared, authorized publication-slot edit -- necessarily
/// invalidates it. That is a real, independent, already-documented
/// consequence of content-addressed binding pinning (the maintainer
/// re-runs `buoy bind` to refresh it after any such edit), not a graph or
/// external-link closure defect a publication-slot candidate introduces,
/// so it is excluded from the candidate closure comparison below.
fn non_binding_diagnostic_count(diagnostics: &[ShellDiagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| !diagnostic.code.starts_with("CAB-"))
        .count()
}

/// Renders one declared external-link output's candidate bytes: the
/// resolved current atom, the basis-keyed page target, the rendered slot,
/// and the candidate document with that slot applied.
///
/// This is real production logic, not a stub: it resolves the current atom
/// through [`crate::external_link_generation::resolve_current_atom`] against
/// the real retained snapshot bytes and the repository's real configured
/// `openspec/v1` adapter configuration, joins the real basis-keyed page path
/// with the declared target base, and verifies the candidate preserves
/// every normative span through
/// [`crate::external_link_generation::verify_candidate_preserves_authority`]
/// before returning it.
#[allow(clippy::too_many_arguments)]
fn publication_external_link_candidate(
    output: &crate::publication::PublicationOutput,
    plan: &crate::publication::TracePublicationPlan,
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    algorithm: crate::core::DigestAlgorithm,
    basis: &str,
    report: &TraceReport,
) -> Result<(RepositoryPath, Vec<u8>, Vec<u8>), ()> {
    let crate::publication::Selector::RequirementId(requirement_id) = plan.selector() else {
        return Err(());
    };
    // `repository_path` is the MOUNT-RELATIVE declared path: the physical
    // descent boundary (`publication_open_parent`) resolves the physical
    // root for `output.mount` and then descends this exact string relative
    // to it, so it must never itself carry a mount prefix. The retained
    // snapshot's own files, however, are keyed by the LOGICAL (mount-prefixed
    // in mounted mode) path, so the lookup below uses the separate logical
    // projection -- never the mount-relative string -- to find this output's
    // saved source bytes. Applying the mount prefix to `repository_path`
    // itself here would double-prefix the later physical descent.
    let repository_path = RepositoryPath::new(&output.path).map_err(drop)?;
    let logical_path = output_logical_path(&output.mount, &output.path, config)?;
    let source_bytes = snapshot
        .files
        .iter()
        .find(|file| file.path.as_str() == logical_path)
        .map(|file| file.bytes.clone())
        .ok_or(())?;
    let active = crate::core::ActiveAlgorithm::new(algorithm).map_err(drop)?;
    let config_bytes = config.adapter_configuration("openspec/v1").ok_or(())?;
    let openspec_configuration =
        crate::adapters::OpenSpecConfiguration::parse(config_bytes).map_err(drop)?;
    let (atom_id, atom_revision) = crate::external_link_generation::resolve_current_atom(
        active,
        &repository_path,
        &source_bytes,
        &openspec_configuration,
        requirement_id.as_str(),
    )
    .map_err(drop)?;

    let page_id = output.page.as_deref().ok_or(())?;
    let page_output = plan
        .outputs()
        .iter()
        .find(|candidate| candidate.id == page_id)
        .ok_or(())?;
    let identities = publication_path_identities(basis, report);
    let rendered_page_path =
        crate::publication::render_path(&page_output.path, &identities).map_err(drop)?;
    // TPT-001: the target base is owned by the declared PAGE output, not
    // the external-link output that merely references it.
    let target_base = page_output.target_base.as_deref().ok_or(())?;
    let target =
        crate::publication::join_target_base(target_base, &rendered_page_path).map_err(drop)?;
    let (basis_algorithm, basis_payload) = basis.split_once(':').ok_or(())?;
    let basis_path_encoding = format!("{basis_algorithm}/{basis_payload}");
    crate::publication::validate_immutable_target(&target, &basis_path_encoding).map_err(drop)?;

    let relation = output.relation.as_deref().unwrap_or("documents");
    let rendered_slot = crate::external_link_generation::render_slot(
        &output.id,
        &atom_id,
        &atom_revision,
        relation,
        &target,
    );
    let source_text = core::str::from_utf8(&source_bytes).map_err(drop)?;
    let candidate_text =
        crate::external_link_generation::apply_slot(source_text, &output.id, &rendered_slot)
            .map_err(drop)?;
    crate::external_link_generation::verify_candidate_preserves_authority(
        active,
        &repository_path,
        &source_bytes,
        candidate_text.as_bytes(),
        &openspec_configuration,
    )
    .map_err(drop)?;
    // Task 8.15: candidate slot normalization must reproduce the exact
    // pre-link basis input the basis was computed from. This is the
    // complementary, stronger invariant to the authority-node comparison
    // above: it proves every byte outside the declared slot's own span --
    // not only every normative span -- stayed identical, so the basis this
    // exact candidate was rendered under remains the correct one a later
    // operation recomputes from the now-written candidate bytes.
    let normalized_source =
        crate::external_link_generation::normalize_for_basis(source_text, &output.id)
            .map_err(drop)?;
    let normalized_candidate =
        crate::external_link_generation::normalize_for_basis(&candidate_text, &output.id)
            .map_err(drop)?;
    let normalized_candidate = if cli_coverage_fault(EXTERNAL_NORMALIZATION_FAULT) {
        format!("{normalized_candidate}coverage-fault")
    } else {
        normalized_candidate
    };
    if normalized_source != normalized_candidate {
        return Err(());
    }

    // Task 8.16: run the complete graph and semantic atom-revision closure
    // check on the candidate -- not only the OpenSpec-adapter-level
    // authority comparison above -- before this candidate is ever eligible
    // to be written. A candidate snapshot substitutes only this declared
    // link's own file's bytes for its rendered candidate and re-runs the
    // exact same `check_snapshot` the read-only `buoy check`/`buoy trace`
    // commands use; any new non-binding diagnostic (a dangling reference, a
    // malformed fence, or any other candidate graph defect) fails closed
    // here, before the write path is ever reached. See
    // `non_binding_diagnostic_count` for why `CAB-` diagnostics are
    // excluded from this comparison.
    let baseline_diagnostic_count = check_snapshot(snapshot, config)
        .map(|checked| non_binding_diagnostic_count(&checked.diagnostics))
        .map_err(drop)?;
    let mut candidate_snapshot = snapshot.clone();
    for file in &mut candidate_snapshot.files {
        if file.path.as_str() == logical_path {
            file.bytes = candidate_text.clone().into_bytes();
        }
    }
    let candidate_diagnostic_count = check_snapshot(&candidate_snapshot, config)
        .map(|checked| non_binding_diagnostic_count(&checked.diagnostics))
        .map_err(drop)?;
    let candidate_diagnostic_count = if cli_coverage_fault(EXTERNAL_DIAGNOSTIC_FAULT) {
        baseline_diagnostic_count.saturating_add(1)
    } else {
        candidate_diagnostic_count
    };
    if candidate_diagnostic_count > baseline_diagnostic_count {
        return Err(());
    }
    Ok((repository_path, source_bytes, candidate_text.into_bytes()))
}

/// Runs the declared external-link operation: `check` compares the saved
/// document with the rendered candidate and writes nothing; `generate`
/// reopens the selected source for a final guard and writes the candidate
/// through the same locked, no-follow, atomic publication writer a page
/// uses, since an external-link edit is one declared source-edit path.
// Round-5 finding H6 added a distinct stable diagnostic to each previously
// bare exit on this path (descent fault, lock contention, source reopen
// failure, source changed). Splitting the function to satisfy the line
// budget would separate those emissions from the single lock-cleanup
// closure that must own every one of them.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn publication_external_link_operation(
    operation: &str,
    output: &crate::publication::PublicationOutput,
    plan: &crate::publication::TracePublicationPlan,
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    algorithm: crate::core::DigestAlgorithm,
    graph: &str,
    basis: &str,
    roots: &InvocationRoots,
    report: &TraceReport,
    format: PublicationFormat,
) -> u8 {
    let base_report = crate::publication::PublicationReport::new(operation, &output.id)
        .with_plan_id(plan.publication_id())
        .with_basis(basis)
        .with_graph(graph);
    let candidate_result = if cli_coverage_fault(EXTERNAL_CANDIDATE_FAULT) {
        Err(())
    } else {
        publication_external_link_candidate(
            output, plan, snapshot, config, algorithm, basis, report,
        )
    };
    let Ok((repository_path, existing, candidate)) = candidate_result else {
        // TPT-006/TPT-007: an external-link candidate that fails to render
        // (an unresolved page reference, a relation/target violation, a
        // rejected cycle-prone dependency, or a candidate that fails the
        // authority-preservation or closure check) must emit a versioned
        // report with a stable code, not a bare exit code.
        let failure_report = base_report.with_diagnostic(
            "TPT-EXTERNAL-LINK-CANDIDATE-REJECTED",
            "the declared external-link candidate failed to render or verify",
        );
        print_publication_report(&failure_report, format);
        return EXIT_INPUT;
    };
    let digests = if cli_coverage_fault(EXTERNAL_DIGEST_FAULT) {
        (
            Err(crate::publication::PublicationError::DigestMemberUnavailable),
            Err(crate::publication::PublicationError::DigestMemberUnavailable),
        )
    } else {
        (
            crate::publication::page_content_digest(algorithm, &candidate),
            crate::publication::page_content_digest(algorithm, &existing),
        )
    };
    let (Ok(candidate_digest), Ok(prior_digest)) = digests else {
        return EXIT_INPUT;
    };
    let prior_state = if existing == candidate {
        "equal"
    } else {
        "unequal"
    };
    let base_report = base_report
        .with_prior(prior_state, Some(&prior_digest))
        .with_candidate_digest(&candidate_digest);
    if operation == "check" {
        let outcome = if prior_state == "equal" {
            "no-op"
        } else {
            "blocked"
        };
        print_publication_report(&base_report.clone().with_outcome(outcome), format);
        return if prior_state == "equal" {
            EXIT_OK
        } else {
            EXIT_STATIC
        };
    }
    // Only reachable for `generate` (`check` already returned above), so
    // this is the one call site allowed to create a missing directory.
    let parent = if cli_coverage_fault(EXTERNAL_PARENT_ABSENT_FAULT) {
        Ok(OpenParentOutcome::Absent)
    } else if cli_coverage_fault(EXTERNAL_PARENT_FAULT) {
        Err(crate::publication_fs::WriteError::Io)
    } else {
        publication_open_parent(output, repository_path.as_str(), roots, true)
    };
    let Ok(OpenParentOutcome::Ready(parent_fd, filename)) = parent else {
        // Round-5 finding H6: a descent fault on the external-link write
        // path emitted a bare exit code, so a caller learned only "input
        // fault" for a link-like ancestor or a root escape.
        let (code, message) = match parent {
            Err(error) => (error.stable_code(), error.message()),
            _ => (
                "TPT-OUTPUT-PARENT-UNAVAILABLE",
                "the declared output parent could not be opened for writing",
            ),
        };
        print_publication_report(
            &base_report
                .with_outcome("blocked")
                .with_diagnostic(code, message),
            format,
        );
        return EXIT_INPUT;
    };
    let lock_name = format!(".{filename}.publication-lock");
    let lock_guard = if cli_coverage_fault(EXTERNAL_LOCK_FAULT) {
        Err(crate::publication_fs::WriteError::Busy)
    } else {
        crate::publication_fs::unix::acquire_output_lock(&parent_fd, &lock_name, 20)
    };
    let Ok(lock_guard) = lock_guard else {
        print_publication_report(
            &base_report.with_outcome("blocked").with_diagnostic(
                "TPT-OUTPUT-BUSY",
                "another publication operation holds this output's lock",
            ),
            format,
        );
        return EXIT_STATIC;
    };
    // Every exit from this point on -- including an early abort from the
    // final reverification guard -- must still remove the operation-owned
    // lock file before returning, so the closure captures every such path
    // and cleanup runs exactly once, after the closure returns.
    let result = (|| -> u8 {
        // Reopen and reverify the selected source immediately before
        // replacement: a change between snapshot capture and this final
        // guard aborts the write without ever reaching the writer.
        //
        // Round-5 finding H6: both aborts below returned a bare exit `2`.
        // These are the two states TPT explicitly separates -- the source
        // became unreadable, versus the source changed under the operation
        // -- and a caller cannot distinguish "retry" from "your worktree
        // moved" without being told which happened.
        let reopened = if cli_coverage_fault(EXTERNAL_REOPEN_FAULT) {
            Ok(None)
        } else {
            crate::publication_fs::unix::read_regular(&parent_fd, &filename)
        };
        let Ok(Some(reopened)) = reopened else {
            print_publication_report(
                &base_report.clone().with_outcome("blocked").with_diagnostic(
                    "TPT-SOURCE-REOPEN-FAILED",
                    "the declared source edit could not be reopened before replacement",
                ),
                format,
            );
            return EXIT_STATIC;
        };
        let reopened = if cli_coverage_fault(EXTERNAL_CHANGED_FAULT) {
            b"coverage-changed-source".to_vec()
        } else {
            reopened
        };
        if reopened != existing {
            print_publication_report(
                &base_report.clone().with_outcome("blocked").with_diagnostic(
                    "TPT-SOURCE-CHANGED",
                    "the declared source edit changed between snapshot capture and replacement",
                ),
                format,
            );
            return EXIT_STATIC;
        }
        let written = if cli_coverage_fault(EXTERNAL_CONFLICT_FAULT) {
            Ok(crate::publication_fs::WriteOutcome::Conflict)
        } else if cli_coverage_fault(EXTERNAL_WRITE_FAULT) {
            Err(crate::publication_fs::WriteError::Io)
        } else {
            crate::publication_fs::unix::write_output(
                &parent_fd,
                &filename,
                &candidate,
                &crate::publication::Lifecycle::Replaceable,
            )
        };
        match written {
            Ok(
                outcome @ (crate::publication_fs::WriteOutcome::Created
                | crate::publication_fs::WriteOutcome::NoOp
                | crate::publication_fs::WriteOutcome::Replaced),
            ) => {
                let label = write_outcome_label(outcome);
                print_publication_report(&base_report.with_outcome(label), format);
                EXIT_OK
            }
            Ok(crate::publication_fs::WriteOutcome::Conflict) => {
                print_publication_report(&base_report.with_outcome("conflict"), format);
                EXIT_STATIC
            }
            Err(_) => EXIT_INPUT,
        }
    })();
    // The operation-owned lock file must never remain after the operation
    // completes: dropping `lock_guard` first releases the advisory `flock`, then
    // this removes the directory entry so a source-edit output's lock file
    // can never itself enter a later exact source-set selection.
    drop(lock_guard);
    crate::publication_fs::unix::cleanup_temporary(&parent_fd, &lock_name);
    result
}

fn external_link_target(candidate: &[u8], link_id: &str) -> Option<String> {
    let text = core::str::from_utf8(candidate).ok()?;
    let (start, end) = crate::external_link_generation::find_slot(text, link_id).ok()??;
    for line in text[start..end].lines() {
        if let Some(target) = line.strip_prefix("target = \"") {
            return Some(target.trim_end_matches('"').to_owned());
        }
    }
    None
}

/// Builds the complete `buoy/trace-publication-evidence/v1` value from every
/// declared non-evidence artifact's real parity and content.
///
/// TPT-008: every non-evidence artifact must pass parity (the exact fact
/// the whole-plan `evidence` operation already required) before any
/// completion payload -- whether rendered to stdout or generated as an
/// explicit `evidence`-kind artifact -- can be built at all.
///
/// # Errors
///
/// Returns the exit code that should be returned immediately when parity
/// fails or any evidence input cannot be computed.
#[allow(clippy::too_many_arguments)]
fn build_publication_evidence(
    plan: &crate::publication::TracePublicationPlan,
    basis: &str,
    report: &TraceReport,
    rows: &[crate::publication::TraceRow],
    canonical: &[(String, Vec<u8>)],
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    algorithm: crate::core::DigestAlgorithm,
    roots: &InvocationRoots,
) -> Result<EvidenceOutcome, u8> {
    // TPT-011: `evidence` is the one operation whose entire product IS its
    // machine document. Returning a bare exit `2` on drift -- with empty
    // output -- leaves a caller nothing machine-readable at all, which is
    // precisely the gap this closes. Every drifted or unverifiable artifact
    // is instead recorded as a stable blocking diagnostic on a real
    // `buoy/trace-publication-evidence/v1` document, which the caller still
    // emits before exiting `2`.
    let mut blockers: Vec<(String, String)> = Vec::new();
    for output in plan.outputs() {
        if matches!(output.kind, crate::publication::ArtifactKind::Evidence) {
            continue;
        }
        match publication_artifact_parity(
            output, basis, report, plan, snapshot, config, algorithm, roots,
        ) {
            Ok(true) => {}
            Ok(false) => blockers.push((
                String::from("TPT-EVIDENCE-ARTIFACT-DRIFT"),
                format!(
                    "declared artifact `{}` is not at parity with its current candidate",
                    output.id
                ),
            )),
            Err(()) => blockers.push((
                String::from("TPT-EVIDENCE-ARTIFACT-UNVERIFIABLE"),
                format!(
                    "declared artifact `{}` could not be compared with a candidate",
                    output.id
                ),
            )),
        }
    }
    let mut flattened = Vec::new();
    for (path, bytes) in canonical {
        flattened.extend_from_slice(path.as_bytes());
        flattened.push(0);
        flattened.extend_from_slice(bytes);
        flattened.push(0);
    }
    let source_digest = if cli_coverage_fault(EVIDENCE_DIGEST_FAULT) {
        Err(crate::publication::PublicationError::DigestMemberUnavailable)
    } else {
        crate::publication::publication_basis_digest(algorithm, &flattened)
    };
    let Ok(source_digest) = source_digest else {
        return Err(EXIT_INPUT);
    };
    let mut evidence = crate::publication::PublicationEvidence::new(plan.publication_id())
        .with_source_digest(&source_digest)
        .with_selector(plan.selector())
        .with_basis(basis)
        .with_graph(&report.graph)
        .with_rows(rows);
    if let Some(page_output) = plan
        .outputs()
        .iter()
        .find(|output| matches!(output.kind, crate::publication::ArtifactKind::Page))
    {
        let page_candidate = if cli_coverage_fault(EVIDENCE_PAGE_CANDIDATE_FAULT) {
            Err(crate::publication::ProjectionError::MissingSchemaIdentity)
        } else {
            publication_page_candidate(page_output, basis, report)
        };
        if let Ok((_, candidate)) = page_candidate {
            let digest = if cli_coverage_fault(EVIDENCE_PAGE_DIGEST_FAULT) {
                Err(crate::publication::PublicationError::DigestMemberUnavailable)
            } else {
                crate::publication::page_content_digest(algorithm, &candidate)
            };
            if let Ok(digest) = digest {
                evidence = evidence.with_page_digest(&digest);
            }
        }
    }
    if let Some(link_output) = plan
        .outputs()
        .iter()
        .find(|output| matches!(output.kind, crate::publication::ArtifactKind::ExternalLink))
    {
        if let Ok((_, _, candidate)) = publication_external_link_candidate(
            link_output,
            plan,
            snapshot,
            config,
            algorithm,
            basis,
            report,
        ) {
            let target = if cli_coverage_fault(EVIDENCE_LINK_TARGET_FAULT) {
                None
            } else {
                external_link_target(&candidate, &link_output.id)
            };
            if let Some(target) = target {
                evidence = evidence.with_link_target(&target);
            }
        }
    }
    // Attach every blocker AFTER every identity the operation did resolve,
    // so a blocked document still shows exactly how far the operation got
    // rather than collapsing to a bare failure.
    let exit = if blockers.is_empty() {
        EXIT_OK
    } else {
        EXIT_STATIC
    };
    for (code, message) in blockers {
        evidence = evidence.with_diagnostic(&code, &message);
    }
    Ok(EvidenceOutcome { evidence, exit })
}

/// One rendered evidence document together with the exit class the operation
/// that produced it must return.
///
/// These travel together so a blocked `evidence` run can still emit its
/// complete machine document before exiting `2`, rather than exiting with no
/// output at all.
struct EvidenceOutcome {
    evidence: crate::publication::PublicationEvidence,
    exit: u8,
}

/// Renders one declared evidence output's candidate Markdown bytes and
/// logical path.
///
/// This is the explicit `evidence`-kind artifact path: a `check` or
/// `generate` operation naming an `evidence`-kind `--artifact` renders the
/// same complete evidence value the whole-plan `evidence` operation
/// computes, but as deterministic Markdown through the identical safe
/// writer a page uses, rather than machine JSON to standard output.
#[allow(clippy::too_many_arguments)]
fn publication_evidence_candidate(
    output: &crate::publication::PublicationOutput,
    plan: &crate::publication::TracePublicationPlan,
    basis: &str,
    report: &TraceReport,
    rows: &[crate::publication::TraceRow],
    canonical: &[(String, Vec<u8>)],
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    algorithm: crate::core::DigestAlgorithm,
    roots: &InvocationRoots,
) -> Result<(String, Vec<u8>), u8> {
    let outcome = build_publication_evidence(
        plan, basis, report, rows, canonical, snapshot, config, algorithm, roots,
    )?;
    if outcome.exit != EXIT_OK {
        // An `evidence`-kind artifact whose underlying plan is blocked must
        // never have a candidate written. Emit the real blocked evidence
        // document -- carrying each stable blocker code -- so this failure
        // is machine-readable too, then stop before any write.
        print!(
            "{}",
            String::from_utf8_lossy(&crate::publication::render_evidence(&outcome.evidence))
        );
        return Err(outcome.exit);
    }
    let markdown = if cli_coverage_fault(EVIDENCE_MARKDOWN_FAULT) {
        String::new()
    } else {
        outcome.evidence.render_markdown()
    };
    if markdown.is_empty() {
        return Err(EXIT_INPUT);
    }
    let identities = publication_path_identities(basis, report);
    let rendered_path =
        crate::publication::render_path(&output.path, &identities).map_err(|_| EXIT_INPUT)?;
    Ok((rendered_path, markdown.into_bytes()))
}

#[allow(clippy::too_many_arguments)]
fn publication_source_changed(
    operation: &str,
    output_id: &str,
    plan_id: &str,
    basis: &str,
    graph: &str,
    code: &str,
    message: &str,
    format: PublicationFormat,
) -> u8 {
    print_publication_report(
        &crate::publication::PublicationReport::new(operation, output_id)
            .with_plan_id(plan_id)
            .with_basis(basis)
            .with_graph(graph)
            .with_outcome("blocked")
            .with_diagnostic(code, message),
        format,
    );
    EXIT_STATIC
}

/// Runs one publication `check`, `generate`, or `evidence` operation through
/// the real typed production pipeline: strict plan parsing, a retained
/// no-follow snapshot, source-set and output-classification guards, real
/// row conformance over the same `buoy trace` query the read-only command
/// uses, a publication basis computed under the repository's configured
/// digest-registry member, and — for `generate` — a locked, no-follow,
/// atomic write through [`crate::publication_fs`].
///
/// `check` and the whole-plan `evidence` operation never create, replace,
/// or remove a repository file. `generate` writes only the one declared
/// artifact its `--artifact` selector names -- including an explicit
/// `evidence`-kind artifact, which writes deterministic Markdown through
/// the same safe writer a page uses.
#[allow(clippy::too_many_lines)]
fn publication_command(arguments: &[String]) -> u8 {
    let Some(operation) = arguments.first().map(String::as_str) else {
        return usage();
    };
    if !matches!(operation, "check" | "generate" | "evidence") {
        return usage();
    }
    let Ok(options) = parse_options(&arguments[1..]) else {
        return usage();
    };
    let Some(plan_path) = options.get("plan") else {
        return usage();
    };
    // Every publication operation emits either the machine automation
    // contract or its deterministic human projection, exactly like `buoy
    // trace`. An unknown `--format` is a usage fault, never a silent
    // fallback to machine output.
    let Ok(format) = PublicationFormat::parse(options.get("format")) else {
        return usage();
    };
    if operation != "evidence"
        && options.get("artifact").is_none()
        && !cli_coverage_fault(PUBLICATION_LATE_ARTIFACT_FAULT)
    {
        return usage();
    }
    let Ok(bytes) = fs::read(plan_path) else {
        print_publication_report(
            &crate::publication::PublicationReport::new(operation, "plan").with_diagnostic(
                "TPT-PLAN-UNREADABLE",
                "the declared publication plan could not be read",
            ),
            format,
        );
        return EXIT_INPUT;
    };
    let plan = match crate::publication::parse_plan(&bytes) {
        Ok(plan) => plan,
        Err(error) => {
            // TPT: a malformed plan must emit a versioned report with one
            // stable `TPT-*` code and structured fields -- not a bare exit
            // code. `artifact` and every repository-derived field are
            // genuinely unavailable this early (plan parsing itself
            // failed), so this report names only the operation, a fixed
            // `"plan"` label for the object under report, and the one
            // stable diagnostic; it fabricates nothing.
            let report = crate::publication::PublicationReport::new(operation, "plan")
                .with_diagnostic(error.stable_code(), "malformed publication plan");
            print_publication_report(&report, format);
            return error.exit_class();
        }
    };
    if let Some(artifact) = options.get("artifact") {
        if !plan.outputs().iter().any(|output| &output.id == artifact)
            && !cli_coverage_fault(PUBLICATION_LATE_ARTIFACT_FAULT)
        {
            // TPT-002: an `--artifact` selector naming an output the plan
            // never declares is a real, nameable input fault. Report it with
            // its own stable code rather than a bare exit code, so a caller
            // can tell an undeclared artifact from an unreadable plan file.
            print_publication_report(
                &crate::publication::PublicationReport::new(operation, "plan")
                    .with_plan_id(plan.publication_id())
                    .with_diagnostic(
                        "TPT-ARTIFACT-UNDECLARED",
                        "the requested artifact is not declared by this plan",
                    ),
                format,
            );
            return EXIT_INPUT;
        }
    }

    // Real repository facts require explicit repository configuration and
    // mount inputs, exactly like every other Buoy command that reads
    // selected bytes; a publication operation is never a report-only stub.
    let Some(config_path) = options.get("config") else {
        return usage();
    };
    // Derive the complete retained publication snapshot from parsed
    // repository configuration and explicit logical-mount inputs. Ambient
    // repository-root environment variables never select source bytes; only
    // explicit `--root`/`--mount` do.
    let plan_fault = |code: &'static str, message: &'static str, exit: u8| {
        print_publication_report(
            &crate::publication::PublicationReport::new(
                operation,
                options.get("artifact").map_or("plan", String::as_str),
            )
            .with_plan_id(plan.publication_id())
            .with_diagnostic(code, message),
            format,
        );
        exit
    };
    let Ok(config) = load_config(&PathBuf::from(config_path)) else {
        return plan_fault(
            "TPT-CONFIG-INVALID",
            "the declared repository configuration could not be loaded",
            EXIT_INPUT,
        );
    };
    let algorithm = config.active_algorithm().algorithm();

    // TPT-008: reject a non-collision-resistant digest member for any plan
    // that declares an immutable output BEFORE any repository read, not only
    // immediately before that output's write.
    //
    // Ordering matters here and is deliberate. Placing this check after
    // repository derivation made it UNREACHABLE for the realistic case: a
    // repository configured for a change-detection member typically fails
    // its own static check first (its committed records are keyed under a
    // different member), so the operation exited on that earlier fault and
    // `TPT-DIGEST-CLASS` never had a reachable production emitter at all.
    // The plan and the configuration are both fully known at this point, so
    // this fault can and must be reported before any expensive or
    // failure-prone repository work runs. A plan with no immutable output
    // needs no collision resistance, so the check stays scoped to plans that
    // do.
    if plan
        .outputs()
        .iter()
        .any(|output| matches!(output.lifecycle, crate::publication::Lifecycle::Immutable))
    {
        if let Err(error) = crate::publication::require_collision_resistant_member(algorithm) {
            return plan_fault(
                error.stable_code(),
                "an immutable publication target requires a collision-resistant digest member",
                EXIT_INPUT,
            );
        }
    }

    let snapshot_result = if cli_coverage_fault(PUBLICATION_SNAPSHOT_FAULT) {
        Err(())
    } else {
        invocation_snapshot(&options, &config)
    };
    let Ok((snapshot, roots)) = snapshot_result else {
        return plan_fault(
            "TPT-SNAPSHOT-FAILED",
            "the retained publication snapshot could not be derived from the declared roots",
            EXIT_INPUT,
        );
    };

    // Every declared output's mount identity must be validated before any
    // source-set or path check runs. In mounted mode, an undeclared mount
    // identity is rejected outright (never silently ignored, and never
    // treated as an alias for another declared mount). A single-root
    // repository has exactly one physical location and no mount namespace to
    // validate an identity against; a plan that still names more than one
    // distinct mount label there is ambiguous about which one is real, and
    // that ambiguity is exactly what collapses authority and output
    // ownership onto the same physical root without the plan ever declaring
    // it. Requiring one consistent label keeps single-root plans honest
    // about there being only one namespace.
    let mount_fault = |output_id: &str, code: &'static str, message: &'static str| {
        print_publication_report(
            &crate::publication::PublicationReport::new(operation, output_id)
                .with_plan_id(plan.publication_id())
                .with_diagnostic(code, message),
            format,
        );
        EXIT_INPUT
    };
    if config.is_mounted() || cli_coverage_fault(PUBLICATION_FORCE_MOUNTED_FAULT) {
        for output in plan.outputs() {
            let Ok(mount_id) = LogicalMountId::new(&output.mount) else {
                return mount_fault(
                    &output.id,
                    "TPT-OUTPUT-MOUNT-INVALID",
                    "the declared output mount identity is malformed",
                );
            };
            if config.mount(&mount_id).is_none() {
                return mount_fault(
                    &output.id,
                    "TPT-OUTPUT-MOUNT-UNDECLARED",
                    "the declared output mount is not declared by this repository configuration",
                );
            }
        }
    } else {
        let mut mount_labels = plan
            .outputs()
            .iter()
            .map(|output| (output.id.as_str(), output.mount.as_str()));
        if let Some((_, first)) = mount_labels.next() {
            if let Some((id, _)) = mount_labels.find(|(_, mount)| *mount != first) {
                return mount_fault(
                    id,
                    "TPT-OUTPUT-MOUNT-AMBIGUOUS",
                    "a single-root repository has one namespace; outputs name more than one mount",
                );
            }
        }
    }

    let selected: Vec<String> = snapshot
        .files
        .iter()
        .map(|file| file.path.as_str().to_owned())
        .collect();
    if crate::publication::enforce_source_set_policy(plan.source_set(), &selected).is_err() {
        // TPT-004: a declared exact source-set policy that no longer equals
        // the real resolved selection (scope drift) must emit a versioned
        // report with a stable code, not a bare exit code.
        let artifact_label = options.get("artifact").map_or("plan", String::as_str);
        let drift_report = crate::publication::PublicationReport::new(operation, artifact_label)
            .with_plan_id(plan.publication_id())
            .with_diagnostic(
                "TPT-SOURCE-SET-SCOPE-DRIFT",
                "the declared exact source-set policy no longer equals the resolved selection",
            );
        print_publication_report(&drift_report, format);
        return EXIT_STATIC;
    }

    // Answer the identical read-only trace query `buoy trace` answers, over
    // the same retained snapshot, so publication conformance never re-derives
    // facts from serialized text.
    let checked = if cli_coverage_fault(PUBLICATION_CHECK_FAULT) {
        None
    } else {
        check_snapshot(&snapshot, &config).ok()
    };
    let Some(checked) = checked else {
        return plan_fault(
            "TPT-REPOSITORY-CHECK-FAILED",
            "the repository static check could not complete over the retained snapshot",
            EXIT_INPUT,
        );
    };
    let requirement_kind = if cli_coverage_fault(PUBLICATION_KIND_FAULT) {
        Err(())
    } else {
        SourceItemKind::new("openspec/requirement/v1").map_err(drop)
    };
    let Ok(requirement_kind) = requirement_kind else {
        return plan_fault(
            "TPT-REQUIREMENT-KIND-INVALID",
            "the publication requirement source-item kind could not be constructed",
            EXIT_INPUT,
        );
    };
    let context = if cli_coverage_fault(PUBLICATION_CONTEXT_FAULT) {
        Err(TraceError::DuplicateRecord)
    } else {
        TraceContext::new(
            checked.source.graph(),
            checked.source.closure(),
            requirement_kind,
            checked.source.authorities(),
            checked.source.attachments(),
            checked.source.store(),
        )
    };
    let Ok(context) = context else {
        return plan_fault(
            "TPT-TRACE-CONTEXT-FAILED",
            "the read-only trace context could not be built over the retained snapshot",
            EXIT_STATIC,
        );
    };
    let core_selector = if cli_coverage_fault(PUBLICATION_SELECTOR_FAULT) {
        Err(())
    } else {
        publication_core_selector(plan.selector())
    };
    let Ok(core_selector) = core_selector else {
        return plan_fault(
            "TPT-SELECTOR-INVALID",
            "the declared selector value is not a well-formed trace selector",
            EXIT_INPUT,
        );
    };
    let artifact_label = options.get("artifact").map_or("plan", String::as_str);
    let projection = match if cli_coverage_fault(PUBLICATION_QUERY_FAULT) {
        Err(TraceError::DuplicateRecord)
    } else {
        trace_query(&context, &core_selector, TraceHistory::CurrentOnly)
    } {
        Ok(projection) => projection,
        Err(TraceError::SelectorNotFound) => {
            // TPT-002: a selector that resolves to nothing is the single
            // most common real publication input fault. A caller must be
            // able to distinguish it by code from an unreadable plan or an
            // undeclared artifact.
            print_publication_report(
                &crate::publication::PublicationReport::new(operation, artifact_label)
                    .with_plan_id(plan.publication_id())
                    .with_diagnostic(
                        "TPT-SELECTOR-NOT-FOUND",
                        "the declared selector resolves to no current trace",
                    ),
                format,
            );
            return EXIT_INPUT;
        }
        Err(_) => {
            print_publication_report(
                &crate::publication::PublicationReport::new(operation, artifact_label)
                    .with_plan_id(plan.publication_id())
                    .with_diagnostic(
                        "TPT-SELECTOR-UNRESOLVED",
                        "the declared selector could not be resolved against the current graph",
                    ),
                format,
            );
            return EXIT_STATIC;
        }
    };
    let report = trace_report(&checked, &projection);
    // TPT-003: pre-link conformance may omit only the exact declared
    // link's own diagnostics, and only when the current operation is
    // `check` or `generate` targeting that exact link artifact. `evidence`
    // has no single-artifact selector for THIS purpose: even though
    // `evidence` accepts an optional `--artifact` naming one
    // evidence-kind output to check or generate as Markdown, that selector
    // must never suppress a link's own diagnostic too. `evidence` requires
    // every non-evidence artifact -- including every declared external
    // link -- at full parity with no exception, so omission is disabled
    // outright for `evidence`, regardless of what `--artifact` names. This
    // is what lets a stale declared link actually receive a repair
    // candidate under `check`/`generate`: without it, the very diagnostic
    // `generate` exists to fix would itself block `generate` from ever
    // reaching the repair.
    let omitted_link_id: Option<String> = if operation == "evidence" {
        None
    } else {
        options.get("artifact").and_then(|artifact_id| {
            plan.outputs()
                .iter()
                .find(|output| {
                    &output.id == artifact_id
                        && matches!(output.kind, crate::publication::ArtifactKind::ExternalLink)
                })
                .map(|output| output.id.clone())
        })
    };
    let blocking_diagnostics =
        filter_link_diagnostics(&report.diagnostics, omitted_link_id.as_deref());
    if !blocking_diagnostics.is_empty() {
        // TPT-003/TPT-009: every unresolved blocking diagnostic the
        // underlying `buoy trace` query already carries (a relation,
        // requirement, implementation, authority, content-addressed
        // binding, or other graph fault) must emit a versioned report
        // naming each real stable code -- not a bare exit code that
        // discards facts this pipeline already computed.
        let artifact_label = options.get("artifact").map_or("plan", String::as_str);
        let mut blocking_report =
            crate::publication::PublicationReport::new(operation, artifact_label)
                .with_plan_id(plan.publication_id())
                .with_graph(&report.graph);
        for diagnostic in &blocking_diagnostics {
            blocking_report = blocking_report.with_diagnostic(
                &diagnostic.code,
                "an unresolved trace diagnostic blocks this publication operation",
            );
        }
        print_publication_report(&blocking_report, format);
        return EXIT_STATIC;
    }
    let rows = publication_conformance_rows(&report, omitted_link_id.as_deref());
    if let Err(conformance_error) = crate::publication::row_conformance(&rows, plan.expected_rows())
    {
        // TPT: a drift check (a stale, fragmented, miscounted, or
        // self-diagnosed row) must emit a versioned report with its one
        // stable `TPT-*` code and exit `2` -- not a bare exit code.
        let artifact_label = options.get("artifact").map_or("plan", String::as_str);
        let report = crate::publication::PublicationReport::new(operation, artifact_label)
            .with_plan_id(plan.publication_id())
            .with_graph(&report.graph)
            .with_diagnostic(conformance_error.stable_code(), "row conformance drift");
        print_publication_report(&report, format);
        return EXIT_STATIC;
    }

    // The digest-member assurance class was already enforced immediately
    // after configuration load, before any repository read, so that
    // `TPT-DIGEST-CLASS` has a reachable production emitter even for a
    // repository whose own static check would otherwise fail first.

    // Canonical retained-snapshot bytes, in stable path order, for the
    // publication basis and the completion-evidence source digest. Every
    // declared publication slot is normalized to its fixed, link-ID-keyed
    // marker before this snapshot enters the basis: an absent, canonical,
    // or stale-but-well-formed slot must all produce the same basis input,
    // so a later `check` against freshly reopened (slot-populated) bytes
    // recomputes the identical basis a prior `generate` used.
    let mut canonical: Vec<(String, Vec<u8>)> = snapshot
        .files
        .iter()
        .map(|file| (file.path.as_str().to_owned(), file.bytes.clone()))
        .collect();
    for output in plan.outputs() {
        if !matches!(output.kind, crate::publication::ArtifactKind::ExternalLink) {
            continue;
        }
        // The retained snapshot's own paths are the LOGICAL (mount-prefixed
        // in mounted mode) form; the declared output path is mount-relative.
        // Project it into the same logical namespace before matching, or
        // this normalization silently no-ops for every mounted external
        // link (the `continue` below would always fire).
        let logical = if cli_coverage_fault(PUBLICATION_OUTPUT_LOGICAL_FAULT) {
            Err(())
        } else {
            output_logical_path(&output.mount, &output.path, &config)
        };
        let Ok(logical) = logical else {
            return plan_fault(
                "TPT-OUTPUT-MOUNT-UNDECLARED",
                "a declared external-link mount could not be projected into the logical namespace",
                EXIT_INPUT,
            );
        };
        let Some(entry) = canonical.iter_mut().find(|(path, _)| path == &logical) else {
            continue;
        };
        let source_bytes: &[u8] = if cli_coverage_fault(PUBLICATION_SOURCE_UTF8_FAULT) {
            &[0xff]
        } else {
            &entry.1
        };
        let Ok(source_text) = core::str::from_utf8(source_bytes) else {
            return plan_fault(
                "TPT-SOURCE-NOT-UTF8",
                "a declared external-link source document is not valid UTF-8",
                EXIT_INPUT,
            );
        };
        let normalized = if cli_coverage_fault(PUBLICATION_NORMALIZE_FAULT) {
            Err(crate::external_link_generation::SlotError::MalformedFence)
        } else {
            crate::external_link_generation::normalize_for_basis(source_text, &output.id)
        };
        let Ok(normalized) = normalized else {
            return plan_fault(
                "TPT-EXTERNAL-LINK-NORMALIZATION-FAILED",
                "a declared external-link slot could not be normalized for the publication basis",
                EXIT_STATIC,
            );
        };
        entry.1 = normalized.into_bytes();
    }
    canonical.sort_by(|left, right| left.0.cmp(&right.0));

    let basis_inputs = crate::publication::BasisInputs {
        plan_schema: crate::publication::PLAN_SCHEMA,
        renderer_schema: TRACE_MARKDOWN_SCHEMA,
        active_member: algorithm.canonical_name(),
        selector: plan.selector(),
        trace_rows: &rows,
        normalized_snapshot: &canonical,
    };
    let basis = if cli_coverage_fault(PUBLICATION_BASIS_FAULT) {
        Err(crate::publication::PublicationError::DigestMemberUnavailable)
    } else {
        crate::publication::compute_publication_basis(&basis_inputs)
    };
    let Ok(basis) = basis else {
        return plan_fault(
            "TPT-BASIS-COMPUTATION-FAILED",
            "the publication basis could not be computed from the retained inputs",
            EXIT_INPUT,
        );
    };

    // TPT: source-set membership and path collisions must be checked on
    // every declared output's REAL, RENDERED path -- not its unrendered
    // template. An unrendered `page` or `evidence` template always looks
    // "derived" (its literal `{...}` braces can never alias a real selected
    // source path), so checking the template is vacuous for any output that
    // uses a substitution; only the fully rendered candidate path can
    // actually alias a selected source or another output's target.
    let identities = publication_path_identities(&basis, &report);
    let mut rendered_paths: Vec<(String, String)> = Vec::with_capacity(plan.outputs().len());
    for output in plan.outputs() {
        let rendered = match output.kind {
            crate::publication::ArtifactKind::Page | crate::publication::ArtifactKind::Evidence => {
                let rendered = if cli_coverage_fault(PUBLICATION_RENDER_PATH_FAULT) {
                    Err(crate::publication::PublicationError::InvalidField(
                        String::from("coverage-render-path"),
                    ))
                } else {
                    crate::publication::render_path(&output.path, &identities)
                };
                match rendered {
                    Ok(rendered) => rendered,
                    Err(error) => {
                        // TPT-001: a path template whose registered
                        // substitution has no unambiguous value for this
                        // selected trace fails with its own stable code
                        // (`TPT-UNKNOWN-SUBSTITUTION`), never a bare exit.
                        print_publication_report(
                            &crate::publication::PublicationReport::new(operation, &output.id)
                                .with_plan_id(plan.publication_id())
                                .with_basis(&basis)
                                .with_diagnostic(
                                    error.stable_code(),
                                    "the declared output path template could not be rendered",
                                ),
                            format,
                        );
                        return EXIT_INPUT;
                    }
                }
            }
            crate::publication::ArtifactKind::SourceEdit
            | crate::publication::ArtifactKind::ExternalLink => output.path.clone(),
        };
        // An external-link output edits an existing `OpenSpec` source file
        // in place, so it is a declared source edit (selecting exactly one
        // regular selected source), exactly like a `SourceEdit` output. A
        // page or evidence artifact is always derived and must stay outside
        // every selected source.
        let kind = match output.kind {
            crate::publication::ArtifactKind::SourceEdit
            | crate::publication::ArtifactKind::ExternalLink => "source-edit",
            _ => "derived",
        };
        // `selected` (the retained snapshot's own paths) is already in the
        // LOGICAL (mount-prefixed in mounted mode) namespace, so every
        // membership and collision check below MUST compare the declared
        // output's LOGICAL path too, with its mount identity validated
        // against the loaded configuration -- never the bare mount-relative
        // rendered path alone. Comparing bare strings would miss a plan
        // output that collides with a selected source only once its mount
        // context is taken into account (a `mount: "project"`,
        // `path: "src/lib.rs"` output colliding with a separately selected
        // `project/src/lib.rs` source), and would accept an undeclared or
        // malformed mount identity silently.
        let logical = if cli_coverage_fault(PUBLICATION_OUTPUT_LOGICAL_FAULT) {
            Err(())
        } else {
            output_logical_path(&output.mount, &rendered, &config)
        };
        let Ok(logical) = logical else {
            return plan_fault(
                "TPT-OUTPUT-MOUNT-UNDECLARED",
                "a declared output mount could not be projected into the logical namespace",
                EXIT_INPUT,
            );
        };
        if let Err(error) = crate::publication::classify_output(&logical, kind, &selected) {
            // TPT: a declared output that classifies as a selected source
            // (or any other classification fault) must emit a versioned
            // report with a stable code, not a bare exit code.
            let classification_report =
                crate::publication::PublicationReport::new(operation, &output.id)
                    .with_plan_id(plan.publication_id())
                    .with_diagnostic(error.stable_code(), "output path classification failed");
            print_publication_report(&classification_report, format);
            return EXIT_STATIC;
        }
        rendered_paths.push((output.id.clone(), logical));
    }
    let collision_candidates: Vec<&str> = rendered_paths
        .iter()
        .map(|(_, rendered)| rendered.as_str())
        .collect();
    if crate::publication::detect_path_collisions(&collision_candidates).is_err() {
        // TPT-OUTPUT-COLLISION: two declared outputs that render to the
        // same logical path must emit a versioned report with a stable
        // code, not a bare exit code.
        let artifact_label = options.get("artifact").map_or("plan", String::as_str);
        let collision_report =
            crate::publication::PublicationReport::new(operation, artifact_label)
                .with_plan_id(plan.publication_id())
                .with_diagnostic(
                    "TPT-OUTPUT-COLLISION",
                    "two declared outputs render to the same logical path",
                );
        print_publication_report(&collision_report, format);
        return EXIT_STATIC;
    }

    if operation == "evidence" {
        let outcome = match build_publication_evidence(
            &plan, &basis, &report, &rows, &canonical, &snapshot, &config, algorithm, &roots,
        ) {
            Ok(outcome) => outcome,
            Err(exit) => return exit,
        };
        // The document is emitted on BOTH paths: a blocked evidence run
        // still produces a complete, versioned
        // `buoy/trace-publication-evidence/v1` value carrying each stable
        // blocker code, then returns exit `2`.
        print!(
            "{}",
            String::from_utf8_lossy(&crate::publication::render_evidence(&outcome.evidence))
        );
        return outcome.exit;
    }

    let Some(artifact_id) = options.get("artifact") else {
        return usage();
    };
    let Some(output) = plan
        .outputs()
        .iter()
        .find(|output| &output.id == artifact_id)
    else {
        // Round-5 finding H6: `--artifact` naming an output the plan never
        // declares returned a bare exit code, so the one thing the caller
        // needed to know -- that the SELECTOR was wrong, not the repository
        // -- was exactly what the process did not say.
        print_publication_report(
            &crate::publication::PublicationReport::new(operation, artifact_id)
                .with_plan_id(plan.publication_id())
                .with_outcome("blocked")
                .with_diagnostic(
                    "TPT-ARTIFACT-UNDECLARED",
                    "the selected artifact is not declared by this plan",
                ),
            format,
        );
        return EXIT_INPUT;
    };

    // TPT-004: before any derived-artifact operation on a declared page,
    // every external link that references it must already be an equal,
    // saved source edit in this exact retained snapshot. An absent or
    // stale declared link edit returns exit 2 before page rendering, for
    // both `check` and `generate` — this is the real production gate for
    // the maintainer-controlled two-step order design.md section 13
    // requires: generate and save the link candidate first, in the
    // authority worktree; only a later, new snapshot that already
    // contains those exact saved bytes can satisfy page operations.
    if matches!(output.kind, crate::publication::ArtifactKind::Page) {
        for link in plan.outputs().iter().filter(|candidate| {
            matches!(
                candidate.kind,
                crate::publication::ArtifactKind::ExternalLink
            ) && candidate.page.as_deref() == Some(output.id.as_str())
        }) {
            let page_precedes_link_report =
                crate::publication::PublicationReport::new(operation, &output.id)
                    .with_plan_id(plan.publication_id())
                    .with_basis(&basis)
                    .with_graph(&report.graph);
            let Ok((_, existing, candidate)) = publication_external_link_candidate(
                link, &plan, &snapshot, &config, algorithm, &basis, &report,
            ) else {
                // TPT-004: the declared link this page references failed to
                // render or verify at all, so the page operation cannot
                // proceed. Emit a versioned report with a stable code
                // rather than a bare exit code.
                print_publication_report(
                    &page_precedes_link_report.with_diagnostic(
                        "TPT-EXTERNAL-LINK-CANDIDATE-REJECTED",
                        "the page's declared external link failed to render or verify",
                    ),
                    format,
                );
                return EXIT_STATIC;
            };
            if existing != candidate {
                // TPT-004: a page operation requires every external link it
                // references to already be an equal, saved source edit.
                print_publication_report(&page_precedes_link_report.with_diagnostic(
                    "TPT-PAGE-PRECEDES-LINK",
                    "a page operation requires its declared external link to already be a current saved source edit",
                ), format);
                return EXIT_STATIC;
            }
        }
    }

    // REH-013/TPT-004: immediately before any write, reopen every selected
    // source path and require it to remain byte-identical to the retained
    // snapshot this operation captured. A change, removal, or new arrival
    // after snapshot capture aborts the write before it ever reaches the
    // output; the candidate keeps its retained-snapshot identity but never
    // reaches the file system. `check` and `evidence` never write, so this
    // guard is unconditional for `generate`.
    //
    // A declared `source-edit` artifact has no Buoy-rendered candidate at
    // all (Buoy never authors a human source edit), so its ONLY real
    // conformance fact is exactly this reopened-bytes comparison. Without
    // running it here too, `check --artifact <source-edit-id>` would
    // return success unconditionally, without ever comparing a single
    // byte -- exactly the parity gap TPT-004 requires closed. `generate`
    // already needs the identical comparison for every kind, so this
    // extension only ADDS the same real guarantee to `check` for the one
    // kind that previously had none.
    if operation == "generate"
        || matches!(output.kind, crate::publication::ArtifactKind::SourceEdit)
    {
        let reopened = if cli_coverage_fault(PUBLICATION_REOPEN_FAULT) {
            Err(())
        } else {
            invocation_snapshot(&options, &config)
        };
        let Ok((reopened, _)) = reopened else {
            return publication_source_changed(
                operation,
                &output.id,
                plan.publication_id(),
                &basis,
                &report.graph,
                "TPT-SOURCE-REOPEN-FAILED",
                "the selected source set could not be reopened for pre-write reverification",
                format,
            );
        };
        let retained_pairs: Vec<(String, Vec<u8>)> = snapshot
            .files
            .iter()
            .map(|file| (file.path.as_str().to_owned(), file.bytes.clone()))
            .collect();
        let reopened_pairs: Vec<(String, Vec<u8>)> = reopened
            .files
            .iter()
            .map(|file| (file.path.as_str().to_owned(), file.bytes.clone()))
            .collect();
        let mut reopened_pairs = reopened_pairs;
        if cli_coverage_fault(PUBLICATION_REVERIFY_FAULT) {
            reopened_pairs.push((String::from("coverage/new-source"), Vec::new()));
        }
        if crate::publication::reverify_selected_paths(&retained_pairs, &reopened_pairs).is_err() {
            // REH-013/TPT-004: a selected source that changed, was removed,
            // was replaced, or newly appeared after the retained snapshot was
            // captured aborts the write. That is a real, stable, actionable
            // outcome, not a bare exit code.
            return publication_source_changed(
                operation,
                &output.id,
                plan.publication_id(),
                &basis,
                &report.graph,
                "TPT-SOURCE-CHANGED",
                "a selected source path changed after the retained snapshot was captured",
                format,
            );
        }
    }

    match output.kind {
        crate::publication::ArtifactKind::Page => publication_page_operation(
            operation,
            output,
            plan.publication_id(),
            &basis,
            &report.graph,
            &report,
            algorithm,
            &roots,
            format,
        ),
        crate::publication::ArtifactKind::ExternalLink => publication_external_link_operation(
            operation,
            output,
            &plan,
            &snapshot,
            &config,
            algorithm,
            &report.graph,
            &basis,
            &roots,
            &report,
            format,
        ),
        crate::publication::ArtifactKind::Evidence => publication_evidence_operation(
            operation,
            output,
            &plan,
            &basis,
            &report.graph,
            &report,
            &rows,
            &canonical,
            &snapshot,
            &config,
            algorithm,
            &roots,
            format,
        ),
        // A source-edit artifact is human-authored: Buoy verifies it (its
        // classification, and -- for `check` and `generate` alike -- the
        // real reopened-bytes reverification against the retained snapshot,
        // already ran above) but never writes it itself. TPT-004/TPT-011:
        // every publication operation, including a source-edit `check`,
        // must emit a real versioned `buoy/trace-publication-report/v1`
        // naming its exact outcome -- never a bare exit code with silent
        // success.
        crate::publication::ArtifactKind::SourceEdit if operation == "check" => {
            // A source-edit `check` must never CLAIM equality: it must
            // establish it by comparing real bytes against an independently
            // obtained candidate. The candidate here is the declared path's
            // current on-disk content, read through the descriptor-relative
            // no-follow reader -- a genuinely different acquisition path
            // from the repository snapshot traversal that produced the
            // retained bytes. Reporting `prior_state: "equal"`
            // unconditionally, as this arm previously did, asserted the very
            // fact the operation exists to verify.
            let source_edit_report =
                crate::publication::PublicationReport::new(operation, &output.id)
                    .with_plan_id(plan.publication_id())
                    .with_basis(&basis)
                    .with_graph(&report.graph);
            let logical = if cli_coverage_fault(SOURCE_EDIT_LOGICAL_FAULT) {
                Err(())
            } else {
                output_logical_path(&output.mount, &output.path, &config)
            };
            let Ok(logical) = logical else {
                print_publication_report(
                    &source_edit_report.with_diagnostic(
                        "TPT-OUTPUT-MOUNT-UNDECLARED",
                        "the declared source-edit mount could not be resolved",
                    ),
                    format,
                );
                return EXIT_INPUT;
            };
            let retained = if cli_coverage_fault(SOURCE_EDIT_RETAINED_FAULT) {
                None
            } else {
                snapshot
                    .files
                    .iter()
                    .find(|file| file.path.as_str() == logical)
                    .map(|file| file.bytes.clone())
            };
            let Some(retained) = retained else {
                print_publication_report(
                    &source_edit_report.with_diagnostic(
                        "TPT-SOURCE-EDIT-NOT-SELECTED",
                        "the declared source edit is not present in the retained snapshot",
                    ),
                    format,
                );
                return EXIT_STATIC;
            };
            let opened = if cli_coverage_fault(SOURCE_EDIT_ABSENT_FAULT) {
                Ok(OpenParentOutcome::Absent)
            } else if cli_coverage_fault(SOURCE_EDIT_OPEN_FAULT) {
                Err(crate::publication_fs::WriteError::Io)
            } else {
                publication_open_parent(output, &output.path, &roots, false)
            };
            let candidate = match opened {
                Ok(OpenParentOutcome::Ready(parent_fd, filename)) => {
                    let read = if cli_coverage_fault(SOURCE_EDIT_READ_FAULT) {
                        Err(crate::publication_fs::WriteError::Io)
                    } else {
                        crate::publication_fs::unix::read_regular(&parent_fd, &filename)
                    };
                    match read {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            print_publication_report(
                                &source_edit_report
                                    .with_outcome("blocked")
                                    .with_diagnostic(error.stable_code(), error.message()),
                                format,
                            );
                            return EXIT_STATIC;
                        }
                    }
                }
                Ok(OpenParentOutcome::Absent) => None,
                Err(error) => {
                    print_publication_report(
                        &source_edit_report
                            .with_outcome("blocked")
                            .with_diagnostic(error.stable_code(), error.message()),
                        format,
                    );
                    return EXIT_STATIC;
                }
            };
            let candidate = if cli_coverage_fault(SOURCE_EDIT_STALE_FAULT) {
                Some(b"stale coverage candidate".to_vec())
            } else {
                candidate
            };
            let retained_digest =
                crate::publication::page_content_digest(algorithm, &retained).ok();
            let candidate_digest = candidate
                .as_deref()
                .and_then(|bytes| crate::publication::page_content_digest(algorithm, bytes).ok());
            let source_edit_report = match &candidate_digest {
                Some(digest) => source_edit_report.with_candidate_digest(digest),
                None => source_edit_report,
            };
            match candidate {
                Some(bytes) if bytes == retained => {
                    print_publication_report(
                        &source_edit_report
                            .with_prior("equal", retained_digest.as_deref())
                            .with_outcome("no-op"),
                        format,
                    );
                    EXIT_OK
                }
                Some(_) => {
                    print_publication_report(
                        &source_edit_report
                            .with_prior("unequal", retained_digest.as_deref())
                            .with_outcome("blocked")
                            .with_diagnostic(
                                "TPT-SOURCE-EDIT-STALE",
                                "the declared source edit differs from its retained snapshot bytes",
                            ),
                        format,
                    );
                    EXIT_STATIC
                }
                None => {
                    print_publication_report(
                        &source_edit_report
                            .with_prior("absent", None)
                            .with_outcome("blocked")
                            .with_diagnostic(
                                "TPT-SOURCE-EDIT-ABSENT",
                                "the declared source edit is absent at its declared output path",
                            ),
                        format,
                    );
                    EXIT_STATIC
                }
            }
        }
        crate::publication::ArtifactKind::SourceEdit => {
            let source_edit_report =
                crate::publication::PublicationReport::new(operation, &output.id)
                    .with_plan_id(plan.publication_id())
                    .with_basis(&basis)
                    .with_graph(&report.graph)
                    .with_diagnostic(
                        "TPT-SOURCE-EDIT-NOT-WRITABLE",
                        "a source-edit artifact is human-authored; Buoy never generates it",
                    );
            print_publication_report(&source_edit_report, format);
            EXIT_INPUT
        }
    }
}

fn parse_options(arguments: &[String]) -> Result<CliOptions, ()> {
    let mut options = CliOptions::default();
    let mut index = 0_usize;
    while index < arguments.len() {
        let Some(key) = arguments[index].strip_prefix("--") else {
            return Err(());
        };
        if BOOLEAN_FLAGS.contains(&key) {
            if options
                .values
                .insert(key.to_string(), String::from("true"))
                .is_some()
            {
                return Err(());
            }
            index += 1;
            continue;
        }
        let Some(value) = arguments.get(index + 1) else {
            return Err(());
        };
        if key == "mount" {
            options.mounts.push(value.clone());
        } else if options
            .values
            .insert(key.to_string(), value.clone())
            .is_some()
        {
            return Err(());
        }
        index += 2;
    }
    Ok(options)
}

#[allow(clippy::expect_used)]
fn emit(report: &MachineReport, format: Option<&str>) -> Result<(), ()> {
    let rendered = match format.unwrap_or("machine") {
        "machine" => {
            render_machine(report).expect("closed machine report serialization is infallible")
                + "\n"
        }
        "human" => render_human(report),
        _ => return Err(()),
    };
    print!("{rendered}");
    Ok(())
}

fn usage() -> u8 {
    eprintln!("{USAGE}");
    EXIT_USAGE
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestInput {
    schema: String,
    id: String,
    base_graph: String,
    head_graph: String,
    dispositions: Vec<DispositionInput>,
    approval: Option<ApprovalInput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DispositionInput {
    id: String,
    kind: String,
    object: String,
    base: Option<String>,
    head: Option<String>,
    disposition: String,
    reaffirmation: Option<String>,
    waiver: Option<ReferenceInput>,
    #[serde(default)]
    replacements: Vec<ReferenceInput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceInput {
    id: String,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalInput {
    authority: String,
    record: String,
    digest: String,
}

struct ParsedManifest {
    manifest: ChangeManifest,
}

fn parse_manifest(path: &Path) -> Result<ParsedManifest, ()> {
    let bytes = fs::read(path).map_err(|_| ())?;
    parse_manifest_bytes(&bytes)
}

fn parse_manifest_bytes(bytes: &[u8]) -> Result<ParsedManifest, ()> {
    let source = core::str::from_utf8(bytes).map_err(|_| ())?;
    let input = toml::from_str::<ManifestInput>(source).map_err(|_| ())?;
    if input.schema != "spec-provenance-change-manifest/v1" {
        return Err(());
    }
    let base_graph = SemanticDigest::parse_canonical(&input.base_graph).map_err(drop)?;
    let head_graph = SemanticDigest::parse_canonical(&input.head_graph).map_err(drop)?;
    let mut dispositions = Vec::new();
    for value in input.dispositions {
        let kind = parse_kind(&value.kind).ok_or(())?;
        let disposition = parse_disposition(&value.disposition).ok_or(())?;
        let base = value
            .base
            .as_deref()
            .map(SemanticDigest::parse_canonical)
            .transpose()
            .map_err(drop)?;
        let head = value
            .head
            .as_deref()
            .map(SemanticDigest::parse_canonical)
            .transpose()
            .map_err(drop)?;
        let reaffirmation = value
            .reaffirmation
            .as_deref()
            .map(SemanticDigest::parse_canonical)
            .transpose()
            .map_err(drop)?;
        let waiver = value
            .waiver
            .as_ref()
            .map(|reference| parse_object_reference(ObjectKind::Waiver, reference))
            .transpose()?;
        let replacements = value
            .replacements
            .into_iter()
            .map(|reference| parse_object_reference(kind, &reference))
            .collect::<Result<Vec<_>, _>>()?;
        let id = StableId::new(&value.id).map_err(drop)?;
        let Ok(object) = StableId::new(&value.object) else {
            return Err(());
        };
        let Ok(disposition) = ChangeDisposition::new(
            id,
            kind,
            object,
            base,
            head,
            disposition,
            reaffirmation,
            waiver,
            replacements,
        ) else {
            return Err(());
        };
        dispositions.push(disposition);
    }
    let approval = input
        .approval
        .map(|approval| parse_approval(&approval))
        .transpose()?;
    Ok(ParsedManifest {
        manifest: ChangeManifest::new(
            StableId::new(&input.id).map_err(drop)?,
            base_graph,
            head_graph,
            dispositions,
            approval,
        )
        .map_err(drop)?,
    })
}

/// Runs the pure strict neutral change-manifest parser for bounded fuzzing.
#[doc(hidden)]
#[must_use]
pub fn validate_change_manifest_bytes(bytes: &[u8]) -> bool {
    parse_manifest_bytes(bytes).is_ok()
}

fn parse_object_reference(kind: ObjectKind, input: &ReferenceInput) -> Result<ObjectRef, ()> {
    let digest = SemanticDigest::parse_canonical(&input.digest).map_err(drop)?;
    Ok(ObjectRef::new(
        kind,
        StableId::new(&input.id).map_err(drop)?,
        digest,
    ))
}

fn parse_approval(input: &ApprovalInput) -> Result<ApprovalBinding, ()> {
    let record = RepositoryPath::new(&input.record).map_err(drop)?;
    let digest = SemanticDigest::parse_canonical(&input.digest).map_err(drop)?;
    ApprovalBinding::new(
        ScopeId::new(&input.authority).map_err(drop)?,
        record,
        digest,
    )
    .map_err(drop)
}

#[cfg(feature = "coverage-controls")]
#[allow(clippy::expect_used)]
fn coverage_binding_edit(report: &BindingTreeReport) -> BindingEdit {
    BindingEdit {
        mount: None,
        source_path: RepositoryPath::new("source.txt").expect("coverage source path"),
        start: 0,
        end: 6,
        replacement: String::from("changed"),
        object_path: RepositoryPath::new(
            "buoy/objects/xxh3-128/22222222222222222222222222222222.toml",
        )
        .expect("coverage binding object path"),
        object_bytes: String::from("binding-object"),
        report: report.clone(),
    }
}

#[cfg(feature = "coverage-controls")]
#[allow(clippy::expect_used)]
fn coverage_artifact_edit() -> ArtifactBindingEdit {
    ArtifactBindingEdit {
        mount: None,
        artifact_path: RepositoryPath::new("artifact.bin").expect("coverage artifact path"),
        artifact_bytes: b"artifact".to_vec(),
        object_path: RepositoryPath::new(
            "buoy/objects/xxh3-128/00000000000000000000000000000000.toml",
        )
        .expect("coverage object path"),
        object_bytes: String::from("object"),
        attachment_path: RepositoryPath::new(
            "buoy/attachments/xxh3-128/11111111111111111111111111111111.toml",
        )
        .expect("coverage attachment path"),
        attachment_bytes: String::from("attachment"),
        report: BindingTreeReport {
            schema: String::from("buoy-binding-tree/v1"),
            relation: String::new(),
            binding: String::new(),
            requirement_id: String::new(),
            atom_revision: String::new(),
            implementation_path: String::new(),
            requirement_path: String::new(),
            freshness: BindingFreshness {
                record: false,
                implementation: false,
                requirement: false,
                authority: false,
            },
            nodes: Vec::new(),
        },
    }
}

/// Exercises artifact preparation boundaries under the coverage profile.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
#[must_use]
#[allow(clippy::expect_used)]
pub fn artifact_binding_prepare_coverage_control(
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    invalid_config: &RepositoryConfig,
) -> bool {
    let checked = check_snapshot(snapshot, config).expect("coverage snapshot");
    let requirement = checked
        .source
        .graph()
        .nodes()
        .iter()
        .find(|node| node.item().kind().as_str() == "openspec/requirement/v1")
        .expect("coverage requirement")
        .reference();
    let artifact = checked
        .source
        .graph()
        .nodes()
        .iter()
        .find(|node| node.item().kind().as_str() == "artifact/file/v1")
        .expect("coverage artifact");
    let artifact_path = artifact.item().path().as_str();
    let valid = prepare_artifact_binding_edit(
        snapshot,
        config,
        &requirement.canonical_text(),
        artifact_path,
        "COVERAGE-PREPARE-ARTIFACT",
    )
    .is_ok();
    let invalid_relation = prepare_artifact_binding_edit(
        snapshot,
        config,
        &requirement.canonical_text(),
        artifact_path,
        "invalid relation",
    )
    .is_err();
    let unknown = SourceNodeRef::new(
        config
            .active_algorithm()
            .compute(
                config.active_algorithm().algorithm(),
                b"unknown requirement",
            )
            .expect("coverage unknown requirement"),
    );
    let unknown_requirement = prepare_artifact_binding_edit(
        snapshot,
        config,
        &unknown.canonical_text(),
        artifact_path,
        "COVERAGE-UNKNOWN-REQUIREMENT",
    )
    .is_err();
    let wrong_kind = prepare_artifact_binding_edit(
        snapshot,
        config,
        &artifact.reference().canonical_text(),
        artifact_path,
        "COVERAGE-WRONG-KIND",
    )
    .is_err();
    let invalid_snapshot = prepare_artifact_binding_edit(
        snapshot,
        invalid_config,
        &requirement.canonical_text(),
        artifact_path,
        "COVERAGE-INVALID-SNAPSHOT",
    )
    .is_err();
    valid & invalid_relation & unknown_requirement & wrong_kind & invalid_snapshot
}

/// Exercises production filesystem cleanup helpers under the coverage profile.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
#[must_use]
#[allow(clippy::expect_used, clippy::too_many_lines)]
pub fn artifact_binding_real_io_coverage_control() -> bool {
    let root = std::env::temp_dir().join(format!(
        "buoy-artifact-binding-coverage-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let parent = root.join("cleanup");
    fs::create_dir_all(&parent).expect("coverage cleanup directory");
    let stale = parent.join("stale.tmp");
    fs::write(&stale, b"stale").expect("coverage stale file");
    let mut io = RealBindingIo::default();
    let real_missing = parent.join("unopened");
    let real_missing_write = io.write_all(&real_missing, b"missing").is_err();
    let real_missing_sync = io.sync_all(&real_missing).is_err();
    let real_existing = parent.join("existing");
    fs::write(&real_existing, b"existing").expect("coverage existing file");
    let real_existing_create = io.create_new(&real_existing).is_err();
    fs::remove_file(&real_existing).expect("remove coverage existing file");
    let stale_removed = remove_stale_temporary(&mut io, &stale).is_ok();
    let object_temporary = parent.join("object.tmp");
    let attachment_temporary = parent.join("attachment.tmp");
    let object = parent.join("object.toml");
    for path in [&object_temporary, &attachment_temporary, &object] {
        fs::write(path, b"cleanup").expect("coverage transaction file");
    }
    let cleaned = cleanup_artifact_transaction(
        &mut io,
        &object_temporary,
        &attachment_temporary,
        Some(&object),
        core::slice::from_ref(&parent),
    )
    .is_ok();
    let absent_file = retry_remove_file(&mut io, &root.join("absent-file")).is_ok();
    let absent_directory = retry_remove_directory(&mut io, &root.join("absent-directory")).is_ok();
    let absent_cleanup = cleanup_artifact_transaction(
        &mut io,
        &root.join("absent-object"),
        &root.join("absent-attachment"),
        None,
        &[root.join("absent-directory")],
    )
    .is_ok();
    let _ = fs::remove_dir_all(&root);

    let logical_root = PathBuf::from("coverage");
    let edit = coverage_artifact_edit();
    let object = logical_root.join(edit.object_path.as_str());
    let attachment = logical_root.join(edit.attachment_path.as_str());
    let mut fault_controls = true;
    for stage in [
        CoverageIoStage::ReadArtifact,
        CoverageIoStage::ReadObject,
        CoverageIoStage::ReadAttachment,
        CoverageIoStage::CreateDirectory,
        CoverageIoStage::CreateObject,
        CoverageIoStage::WriteObject,
        CoverageIoStage::SyncObject,
        CoverageIoStage::RenameObject,
        CoverageIoStage::CreateAttachment,
        CoverageIoStage::WriteAttachment,
        CoverageIoStage::SyncAttachment,
        CoverageIoStage::RenameAttachment,
        CoverageIoStage::RemoveFile,
        CoverageIoStage::RemoveDirectory,
    ] {
        let mut control = CoverageBindingIo::new(&logical_root, &edit);
        if stage == CoverageIoStage::ReadObject {
            control
                .files
                .insert(object.clone(), edit.object_bytes.as_bytes().to_vec());
        }
        if matches!(
            stage,
            CoverageIoStage::RemoveFile | CoverageIoStage::RemoveDirectory
        ) {
            control.inject(CoverageIoStage::WriteAttachment, 1);
            control.inject(stage, 3);
        } else {
            control.inject(stage, 1);
        }
        fault_controls &=
            apply_artifact_binding_edit_with(&mut control, &logical_root, &edit).is_err();
    }

    let mut success = CoverageBindingIo::new(&logical_root, &edit);
    let fresh_success =
        apply_artifact_binding_edit_with(&mut success, &logical_root, &edit).is_ok();
    let mut without_attachment = CoverageBindingIo::new(&logical_root, &edit);
    without_attachment.files.remove(&attachment);
    let absent_attachment_success =
        apply_artifact_binding_edit_with(&mut without_attachment, &logical_root, &edit).is_ok();
    let mut existing_object = CoverageBindingIo::new(&logical_root, &edit);
    existing_object
        .files
        .insert(object.clone(), edit.object_bytes.as_bytes().to_vec());
    let existing_object_success =
        apply_artifact_binding_edit_with(&mut existing_object, &logical_root, &edit).is_ok();
    let mut stale_object = CoverageBindingIo::new(&logical_root, &edit);
    stale_object.files.insert(object, b"stale".to_vec());
    let stale_object_rejected =
        apply_artifact_binding_edit_with(&mut stale_object, &logical_root, &edit).is_err();
    let mut changed_artifact = CoverageBindingIo::new(&logical_root, &edit);
    changed_artifact.files.insert(
        logical_root.join(edit.artifact_path.as_str()),
        b"changed".to_vec(),
    );
    let changed_artifact_rejected =
        apply_artifact_binding_edit_with(&mut changed_artifact, &logical_root, &edit).is_err();
    let binding_edit = coverage_binding_edit(&edit.report);
    let binding_source = logical_root.join(binding_edit.source_path.as_str());
    let binding_object = logical_root.join(binding_edit.object_path.as_str());
    let mut binding_controls = true;
    for stage in [
        CoverageIoStage::ReadAttachment,
        CoverageIoStage::CreateDirectory,
        CoverageIoStage::CreateObject,
        CoverageIoStage::WriteObject,
        CoverageIoStage::SyncObject,
        CoverageIoStage::WriteSource,
        CoverageIoStage::RenameObject,
    ] {
        let mut control = CoverageBindingIo::new(&logical_root, &edit);
        control
            .files
            .insert(binding_source.clone(), b"source".to_vec());
        control.inject(stage, 1);
        binding_controls &=
            apply_binding_edit_with(&mut control, &logical_root, &binding_edit).is_err();
    }
    let mut binding_success = CoverageBindingIo::new(&logical_root, &edit);
    binding_success
        .files
        .insert(binding_source.clone(), b"source".to_vec());
    let binding_succeeds =
        apply_binding_edit_with(&mut binding_success, &logical_root, &binding_edit).is_ok();
    let mut binding_exists = CoverageBindingIo::new(&logical_root, &edit);
    binding_exists
        .files
        .insert(binding_source.clone(), b"source".to_vec());
    binding_exists.files.insert(binding_object, Vec::new());
    let binding_existing_rejected =
        apply_binding_edit_with(&mut binding_exists, &logical_root, &binding_edit).is_err();

    let mut artifact_render_failure = CoverageBindingIo::new(&logical_root, &edit);
    artifact_render_failure.inject(CoverageIoStage::ReadArtifact, 1);
    let artifact_apply_error = apply_and_render_artifact_binding(
        &mut artifact_render_failure,
        &logical_root,
        &edit,
        "machine",
    ) == EXIT_INPUT;
    let mut artifact_render_invalid = CoverageBindingIo::new(&logical_root, &edit);
    let artifact_render_error = apply_and_render_artifact_binding(
        &mut artifact_render_invalid,
        &logical_root,
        &edit,
        "invalid",
    ) == EXIT_OK;
    let mut binding_render_failure = CoverageBindingIo::new(&logical_root, &edit);
    binding_render_failure
        .files
        .insert(binding_source.clone(), b"source".to_vec());
    binding_render_failure.inject(CoverageIoStage::ReadAttachment, 1);
    let binding_apply_error = apply_and_render_binding(
        &mut binding_render_failure,
        &logical_root,
        &binding_edit,
        "machine",
    ) == EXIT_INPUT;
    let mut binding_render_invalid = CoverageBindingIo::new(&logical_root, &edit);
    binding_render_invalid
        .files
        .insert(binding_source.clone(), b"source".to_vec());
    let binding_render_error = apply_and_render_binding(
        &mut binding_render_invalid,
        &logical_root,
        &binding_edit,
        "invalid",
    ) == EXIT_OK;

    let mut direct = CoverageBindingIo::new(&logical_root, &edit);
    let missing = logical_root.join("missing");
    let missing_read = direct.read(&missing).is_err();
    direct.files.insert(missing.clone(), Vec::new());
    let duplicate_create = direct.create_new(&missing).is_err();
    let missing_rename = direct
        .rename(&logical_root.join("absent"), &missing)
        .is_err();

    real_missing_write
        & real_missing_sync
        & real_existing_create
        & stale_removed
        & cleaned
        & absent_file
        & absent_directory
        & absent_cleanup
        & fault_controls
        & fresh_success
        & absent_attachment_success
        & existing_object_success
        & stale_object_rejected
        & changed_artifact_rejected
        & missing_read
        & duplicate_create
        & missing_rename
        & binding_controls
        & binding_succeeds
        & binding_existing_rejected
        & artifact_apply_error
        & artifact_render_error
        & binding_apply_error
        & binding_render_error
}

#[cfg(test)]
fn option_digest(value: Option<SemanticDigest>) -> CanonicalValue {
    value.map_or(CanonicalValue::None, |digest| {
        CanonicalValue::Some(Box::new(CanonicalValue::Digest(digest)))
    })
}

fn parse_kind(value: &str) -> Option<ObjectKind> {
    match value {
        "atom" => Some(ObjectKind::Atom),
        "tombstone" => Some(ObjectKind::Tombstone),
        "decision-surface" => Some(ObjectKind::DecisionSurface),
        "evidence-carrier" => Some(ObjectKind::EvidenceCarrier),
        "implementation-link" => Some(ObjectKind::ImplementationLink),
        "derivation" => Some(ObjectKind::Derivation),
        "coverage-policy" => Some(ObjectKind::CoveragePolicy),
        "debt-record" => Some(ObjectKind::DebtRecord),
        "waiver" => Some(ObjectKind::Waiver),
        "oracle" => Some(ObjectKind::Oracle),
        "finite-model" => Some(ObjectKind::FiniteModel),
        "formal-property" => Some(ObjectKind::FormalProperty),
        "assumption" => Some(ObjectKind::Assumption),
        "proof-record" => Some(ObjectKind::ProofRecord),
        _ => None,
    }
}

fn parse_disposition(value: &str) -> Option<ChangeDispositionKind> {
    match value {
        "reaffirmed" => Some(ChangeDispositionKind::Reaffirmed),
        "revised" => Some(ChangeDispositionKind::Revised),
        "retired" => Some(ChangeDispositionKind::Retired),
        "waived" => Some(ChangeDispositionKind::Waived),
        _ => None,
    }
}

fn shell_diagnostic(code: &str, id: Option<String>) -> ShellDiagnostic {
    ShellDiagnostic {
        code: code.to_string(),
        path: None,
        start: 0,
        end: 0,
        id,
        context: String::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::needless_borrow, clippy::too_many_lines)]
    use super::*;
    use crate::core::{
        CoveragePolicy, CoveragePolicyRevision, EnforcementMode, EvidenceRole, PolicyLifecycle,
        SchemaId, ScopeId, SpecGraph, TypedPayload,
    };
    use std::collections::BTreeSet;
    use std::fs;
    use tempfile::TempDir;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[cfg(feature = "coverage-controls")]
    fn with_cli_faults(faults: u64, operation: impl FnOnce() -> u8) -> u8 {
        CLI_COVERAGE_FAULTS.with(|active| {
            let previous = active.replace(faults);
            let result = operation();
            active.set(previous);
            result
        })
    }

    fn config(oracles: &str, expectation: &str) -> String {
        format!(
            r#"schema = "spec-provenance-config/v2"
digest_algorithms = ["xxh3-128"]

[[selectors]]
root = "spec"
adapter = "markdown/v1"
include = ["**/*.md"]
exclude = []
authority_expectation = "{expectation}"
{oracles}
"#,
        )
    }

    fn make_repository(source: &str, oracles: &str, expectation: &str) -> (TempDir, PathBuf) {
        let temporary = TempDir::new().expect("CLI control root");
        fs::create_dir_all(temporary.path().join("spec")).expect("CLI spec directory");
        fs::write(temporary.path().join("spec/control.md"), source).expect("CLI source");
        let config_path = temporary.path().join("config.toml");
        fs::write(&config_path, config(oracles, expectation)).expect("CLI config");
        (temporary, config_path)
    }

    #[cfg(feature = "blake3-256")]
    fn copy_binding_repository() -> TempDir {
        let source = Path::new("fixtures/content-addressed-bindings/v1/positive");
        let target = TempDir::new().expect("binding CLI control root");
        for path in [
            "buoy.toml",
            "openspec/specs/demo/spec.md",
            "src/lib.rs",
            "buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml",
        ] {
            let destination = target.path().join(path);
            fs::create_dir_all(destination.parent().expect("binding fixture parent"))
                .expect("binding fixture directory");
            fs::copy(source.join(path), destination).expect("copied binding fixture");
        }
        target
    }

    fn atom(statement: &str) -> String {
        format!(
            "```spec-provenance-atom-v1\nid = \"CLI-CONTROL\"\nstate = \"proposed\"\nkind = \"behavioral\"\nscope = [\"cli\"]\nstatement = \"{statement}\"\n```\n"
        )
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum BindingIoStage {
        Success,
        Exists,
        Read,
        CreateDirectory,
        CreateFile,
        WriteFile,
        SyncFile,
        WriteSource,
        Rename,
    }

    struct BindingIoControl {
        stage: BindingIoStage,
    }

    impl BindingIoControl {
        fn result(&self, stage: BindingIoStage) -> std::io::Result<()> {
            if self.stage == stage {
                Err(std::io::Error::other("binding I/O control"))
            } else {
                Ok(())
            }
        }
    }

    impl BindingIo for BindingIoControl {
        fn exists(&self, _path: &Path) -> bool {
            self.stage == BindingIoStage::Exists
        }

        fn read(&mut self, _path: &Path) -> std::io::Result<Vec<u8>> {
            self.result(BindingIoStage::Read)?;
            Ok(b"source".to_vec())
        }

        fn create_dir_all(&mut self, _path: &Path) -> std::io::Result<()> {
            self.result(BindingIoStage::CreateDirectory)
        }

        fn create_new(&mut self, _path: &Path) -> std::io::Result<()> {
            self.result(BindingIoStage::CreateFile)
        }

        fn write_all(&mut self, _path: &Path, _bytes: &[u8]) -> std::io::Result<()> {
            self.result(BindingIoStage::WriteFile)
        }

        fn sync_all(&mut self, _path: &Path) -> std::io::Result<()> {
            self.result(BindingIoStage::SyncFile)
        }

        fn write(&mut self, _path: &Path, _bytes: &[u8]) -> std::io::Result<()> {
            self.result(BindingIoStage::WriteSource)
        }

        fn rename(&mut self, _from: &Path, _to: &Path) -> std::io::Result<()> {
            self.result(BindingIoStage::Rename)
        }

        fn remove_file(&mut self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }

        fn remove_dir(&mut self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum ArtifactIoStage {
        ReadArtifact,
        ReadObject,
        ReadAttachment,
        CreateDirectory,
        CreateObject,
        WriteObject,
        SyncObject,
        RenameObject,
        CreateAttachment,
        WriteAttachment,
        SyncAttachment,
        RenameAttachment,
        RemoveFile,
        RemoveDirectory,
    }

    impl ArtifactIoStage {
        const fn model_stage(self) -> ArtifactBindingTransactionStage {
            match self {
                Self::ReadArtifact | Self::ReadObject | Self::ReadAttachment => {
                    ArtifactBindingTransactionStage::Validate
                }
                Self::CreateDirectory => ArtifactBindingTransactionStage::CreateObjectDirectory,
                Self::CreateObject => ArtifactBindingTransactionStage::CreateObjectTemporary,
                Self::WriteObject => ArtifactBindingTransactionStage::WriteObject,
                Self::SyncObject => ArtifactBindingTransactionStage::SyncObject,
                Self::RenameObject => ArtifactBindingTransactionStage::ReplaceObject,
                Self::CreateAttachment => {
                    ArtifactBindingTransactionStage::CreateAttachmentTemporary
                }
                Self::WriteAttachment => ArtifactBindingTransactionStage::WriteAttachment,
                Self::SyncAttachment => ArtifactBindingTransactionStage::SyncAttachment,
                Self::RenameAttachment => ArtifactBindingTransactionStage::ReplaceAttachment,
                Self::RemoveFile => ArtifactBindingTransactionStage::RemoveUncommittedObject,
                Self::RemoveDirectory => ArtifactBindingTransactionStage::RetryCleanup,
            }
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ArtifactIoState {
        files: BTreeMap<PathBuf, Vec<u8>>,
        directories: BTreeSet<PathBuf>,
    }

    struct ArtifactIoControl {
        state: ArtifactIoState,
        failures: BTreeMap<ArtifactIoStage, u8>,
    }

    impl ArtifactIoControl {
        fn new(edit: &ArtifactBindingEdit) -> Self {
            let root = PathBuf::from("control");
            let artifact = root.join(edit.artifact_path.as_str());
            let attachment = root.join(edit.attachment_path.as_str());
            let files = BTreeMap::from([
                (artifact, edit.artifact_bytes.clone()),
                (attachment.clone(), b"prior-sidecar".to_vec()),
            ]);
            let mut directories = BTreeSet::from([root.clone()]);
            let mut current = attachment.parent();
            while let Some(directory) = current {
                directories.insert(directory.to_path_buf());
                current = if directory == root {
                    None
                } else {
                    directory.parent()
                };
            }
            Self {
                state: ArtifactIoState { files, directories },
                failures: BTreeMap::new(),
            }
        }

        fn fail(&mut self, stage: ArtifactIoStage) -> std::io::Result<()> {
            let remaining = self.failures.entry(stage).or_default();
            if *remaining == 0 {
                return Ok(());
            }
            *remaining -= 1;
            Err(std::io::Error::other("artifact binding I/O control"))
        }

        fn inject(&mut self, stage: ArtifactIoStage, count: u8) {
            self.failures.insert(stage, count);
        }

        fn path_stage(
            path: &Path,
            object: ArtifactIoStage,
            attachment: ArtifactIoStage,
        ) -> ArtifactIoStage {
            if path.to_string_lossy().contains("/objects/") {
                object
            } else {
                attachment
            }
        }
    }

    impl BindingIo for ArtifactIoControl {
        fn exists(&self, path: &Path) -> bool {
            self.state.files.contains_key(path) || self.state.directories.contains(path)
        }

        fn read(&mut self, path: &Path) -> std::io::Result<Vec<u8>> {
            let stage = if path.ends_with("artifact.bin") {
                ArtifactIoStage::ReadArtifact
            } else if path.to_string_lossy().contains("/objects/") {
                ArtifactIoStage::ReadObject
            } else {
                ArtifactIoStage::ReadAttachment
            };
            self.fail(stage)?;
            self.state
                .files
                .get(path)
                .cloned()
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
        }

        fn create_dir_all(&mut self, path: &Path) -> std::io::Result<()> {
            self.fail(ArtifactIoStage::CreateDirectory)?;
            self.state.directories.insert(path.to_path_buf());
            Ok(())
        }

        fn create_new(&mut self, path: &Path) -> std::io::Result<()> {
            let stage = Self::path_stage(
                path,
                ArtifactIoStage::CreateObject,
                ArtifactIoStage::CreateAttachment,
            );
            self.fail(stage)?;
            if self.state.files.contains_key(path) {
                return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
            }
            self.state.files.insert(path.to_path_buf(), Vec::new());
            Ok(())
        }

        fn write_all(&mut self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
            let stage = Self::path_stage(
                path,
                ArtifactIoStage::WriteObject,
                ArtifactIoStage::WriteAttachment,
            );
            self.fail(stage)?;
            self.state.files.insert(path.to_path_buf(), bytes.to_vec());
            Ok(())
        }

        fn sync_all(&mut self, path: &Path) -> std::io::Result<()> {
            let stage = Self::path_stage(
                path,
                ArtifactIoStage::SyncObject,
                ArtifactIoStage::SyncAttachment,
            );
            self.fail(stage)
        }

        fn write(&mut self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
            self.state.files.insert(path.to_path_buf(), bytes.to_vec());
            Ok(())
        }

        fn rename(&mut self, from: &Path, to: &Path) -> std::io::Result<()> {
            let stage = Self::path_stage(
                from,
                ArtifactIoStage::RenameObject,
                ArtifactIoStage::RenameAttachment,
            );
            self.fail(stage)?;
            let bytes = self
                .state
                .files
                .remove(from)
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            self.state.files.insert(to.to_path_buf(), bytes);
            Ok(())
        }

        fn remove_file(&mut self, path: &Path) -> std::io::Result<()> {
            self.fail(ArtifactIoStage::RemoveFile)?;
            self.state
                .files
                .remove(path)
                .map(|_| ())
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
        }

        fn remove_dir(&mut self, path: &Path) -> std::io::Result<()> {
            self.fail(ArtifactIoStage::RemoveDirectory)?;
            if self
                .state
                .files
                .keys()
                .any(|file| file.parent() == Some(path))
                || self
                    .state
                    .directories
                    .iter()
                    .any(|directory| directory.parent() == Some(path))
            {
                return Err(std::io::Error::from(std::io::ErrorKind::DirectoryNotEmpty));
            }
            self.state
                .directories
                .remove(path)
                .then_some(())
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
        }
    }

    fn binding_edit_control() -> BindingEdit {
        BindingEdit {
            mount: None,
            source_path: RepositoryPath::new("src/control.rs").expect("source control path"),
            start: 0,
            end: 0,
            replacement: String::from("bound"),
            object_path: RepositoryPath::new("buoy/objects/control.toml")
                .expect("object control path"),
            object_bytes: String::from("record"),
            report: BindingTreeReport {
                schema: String::from("control/v1"),
                relation: String::from("CONTROL"),
                binding: String::from("control"),
                requirement_id: String::from("CONTROL"),
                atom_revision: String::from("control"),
                implementation_path: String::from("src/control.rs"),
                requirement_path: String::from("spec/control.md"),
                freshness: BindingFreshness {
                    record: true,
                    implementation: true,
                    requirement: true,
                    authority: true,
                },
                nodes: Vec::new(),
            },
        }
    }

    fn artifact_binding_edit_control() -> ArtifactBindingEdit {
        ArtifactBindingEdit {
            mount: None,
            artifact_path: RepositoryPath::new("artifact.bin").expect("artifact control path"),
            artifact_bytes: b"artifact-control".to_vec(),
            object_path: RepositoryPath::new(
                "buoy/objects/v1/blake3-256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.toml",
            )
            .expect("object control path"),
            object_bytes: String::from("record-control"),
            attachment_path: RepositoryPath::new(
                "buoy/attachments/v1/blake3-256/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.toml",
            )
            .expect("attachment control path"),
            attachment_bytes: String::from("attachment-control"),
            report: binding_edit_control().report,
        }
    }

    fn report() -> MachineReport {
        MachineReport {
            command: String::from("control"),
            graph: SemanticDigest::xxh3_128([0; 16]).canonical_text(),
            diagnostics: Vec::new(),
            migration: MigrationReport::default(),
            verdicts: BTreeMap::new(),
            impacts: Vec::new(),
            mounts: Vec::new(),
            requirement_subjects: Vec::new(),
            opaque_subjects: Vec::new(),
            artifact_attachments: Vec::new(),
        }
    }

    fn coverage_policy(enforcement: EnforcementMode) -> CoveragePolicy {
        let revision = CoveragePolicyRevision::new(
            TypedPayload::new(
                SchemaId::new("policy-scope/v1").expect("policy schema"),
                Vec::new(),
            )
            .expect("policy scope"),
            vec![EvidenceRole::Positive],
            enforcement,
            ScopeId::new("ratchet-control").expect("ratchet ID"),
            Vec::new(),
            None,
        )
        .expect("policy revision");
        let approval = ApprovalBinding::new(
            ScopeId::new("maintainers").expect("authority ID"),
            RepositoryPath::new("reviews/control.md").expect("review path"),
            SemanticDigest::xxh3_128([7; 16]),
        )
        .expect("policy approval");
        CoveragePolicy::new(
            StableId::new("POL-CONTROL").expect("control policy ID"),
            revision,
            approval,
            PolicyLifecycle::Active,
            Vec::new(),
            None,
            Vec::new(),
        )
        .expect("coverage policy")
    }

    #[test]
    fn pure_cli_controls_cover_closed_parsers_and_presentations() {
        for help in ["help", "--help", "-h"] {
            assert_eq!(run(&arguments(&[help])), EXIT_OK);
        }
        assert_eq!(run(&[]), EXIT_USAGE);
        assert_eq!(run(&arguments(&["unknown"])), EXIT_USAGE);
        assert_eq!(run(&arguments(&["static", "root", "."])), EXIT_USAGE);
        assert_eq!(run(&arguments(&["static"])), EXIT_USAGE);
        assert_eq!(run(&arguments(&["static", "--root", "."])), EXIT_USAGE);
        assert_eq!(run(&arguments(&["change"])), EXIT_USAGE);
        assert_eq!(run(&arguments(&["execute"])), EXIT_USAGE);
        assert_eq!(run(&arguments(&["execute", "--root", "."])), EXIT_USAGE);
        assert!(parse_options(&arguments(&["root", "."])).is_err());
        assert!(parse_options(&arguments(&["--root"])).is_err());
        assert!(parse_options(&arguments(&["--root", ".", "--root", "."])).is_err());
        assert!(emit(&report(), Some("machine")).is_ok());
        assert!(emit(&report(), Some("human")).is_ok());
        assert!(emit(&report(), Some("unknown")).is_err());
        assert!(!change_diagnostic_blocks("CHG-RATCHET-SCOPE"));
        assert!(change_diagnostic_blocks("CHG-RATCHET-MATERIALIZATION"));
        assert!(change_diagnostic_blocks("CHG-DISPOSITION-DEBT"));

        for kind in [
            "atom",
            "tombstone",
            "decision-surface",
            "evidence-carrier",
            "implementation-link",
            "derivation",
            "coverage-policy",
            "debt-record",
            "waiver",
            "oracle",
            "finite-model",
            "formal-property",
            "assumption",
            "proof-record",
        ] {
            assert!(parse_kind(kind).is_some(), "{kind}");
        }
        assert!(parse_kind("unknown").is_none());
        for disposition in ["reaffirmed", "revised", "retired", "waived"] {
            assert!(parse_disposition(disposition).is_some(), "{disposition}");
        }
        assert!(parse_disposition("unknown").is_none());
        assert_eq!(option_digest(None), CanonicalValue::None);
        assert_eq!(
            option_digest(Some(SemanticDigest::xxh3_128([1; 16]))),
            CanonicalValue::Some(Box::new(CanonicalValue::Digest(SemanticDigest::xxh3_128(
                [1; 16]
            ),)))
        );
        assert_eq!(shell_diagnostic("CONTROL", None).code, "CONTROL");

        let mut diagnostics = Vec::new();
        assert!(
            resolve_policy_ratchets(Err(PolicyAnalysisError::DuplicatePolicy), &mut diagnostics,)
                .is_empty()
        );
        assert_eq!(diagnostics[0].code, "CHG-RATCHET-POLICY-DUPLICATE");

        #[cfg(feature = "coverage-controls")]
        {
            let base = SpecGraph {
                coverage_policies: vec![coverage_policy(EnforcementMode::Blocking)],
                ..SpecGraph::default()
            };
            let head = SpecGraph::default();
            let ratchets = derive_policy_ratchets(&base, &head).expect("derived policy ratchet");
            assert!(!ratchets.is_empty());
            let ratchet_diagnostics = ratchet_diagnostics_control(&ratchets);
            assert_eq!(ratchet_diagnostics.len(), 1);
            assert_eq!(ratchet_diagnostics[0].code, "CHG-RATCHET-RETIREMENT");
            assert_eq!(ratchet_diagnostics[0].id.as_deref(), Some("POL-CONTROL"));
        }
    }

    #[test]
    fn binding_edit_io_controls_cover_each_transaction_stage() {
        let edit = binding_edit_control();
        for stage in [
            BindingIoStage::Success,
            BindingIoStage::Exists,
            BindingIoStage::Read,
            BindingIoStage::CreateDirectory,
            BindingIoStage::CreateFile,
            BindingIoStage::WriteFile,
            BindingIoStage::SyncFile,
            BindingIoStage::WriteSource,
            BindingIoStage::Rename,
        ] {
            let result = apply_binding_edit_with(
                &mut BindingIoControl { stage },
                Path::new("control"),
                &edit,
            );
            assert_eq!(
                result.is_ok(),
                stage == BindingIoStage::Success,
                "{stage:?}"
            );
        }
        let removable = TempDir::new().expect("real binding I/O control");
        let removable_file = removable.path().join("remove-me");
        fs::write(&removable_file, b"control").expect("removable control file");
        RealBindingIo::default()
            .remove_file(&removable_file)
            .expect("real remove-file control");
        assert!(!removable_file.exists());
        assert_eq!(
            apply_and_render_binding(
                &mut BindingIoControl {
                    stage: BindingIoStage::Read,
                },
                Path::new("control"),
                &edit,
                "machine",
            ),
            EXIT_INPUT
        );
        assert_eq!(
            apply_and_render_binding(
                &mut BindingIoControl {
                    stage: BindingIoStage::Success,
                },
                Path::new("control"),
                &edit,
                "machine",
            ),
            EXIT_OK
        );
        assert_eq!(
            apply_and_render_binding(
                &mut BindingIoControl {
                    stage: BindingIoStage::Success,
                },
                Path::new("control"),
                &edit,
                "human",
            ),
            EXIT_OK
        );
        for (source, start, end, expected) in [
            ("attribute", 0, 9, Ok((0, 0))),
            ("#[buoy::implements(\"digest\")]", 0, 29, Ok((0, 29))),
            ("short", 5, 5, Err(BindError::Input)),
            ("short", 0, 6, Err(BindError::Input)),
        ] {
            assert_eq!(binding_attribute_span(source, start, end), expected);
        }
        let incomplete = "#[buoy::implements(\"digest\")";
        assert_eq!(
            binding_attribute_span(incomplete, 0, incomplete.len() as u64),
            Ok((0, 0))
        );
    }

    #[test]
    fn binding_io_controls_cover_cleanup_stale_paths_and_real_directory_removal() {
        let mut binding_io = BindingIoControl {
            stage: BindingIoStage::Success,
        };
        assert!(binding_io.remove_dir(Path::new("unused")).is_ok());
        assert_eq!(
            ensure_parent_directories(
                &mut binding_io,
                Path::new("control"),
                Path::new("outside"),
                &mut Vec::new(),
            ),
            Err(())
        );
        assert_eq!(
            binding_write_root(
                &InvocationRoots::Mounted(MountMappings::new(Vec::new()).expect("empty mappings")),
                Some(&LogicalMountId::new("missing").expect("mount ID")),
            ),
            None
        );
        let (invalid_static, invalid_static_config) = make_repository(
            include_str!("../fixtures/adapters/markdown/objects.md"),
            "",
            "optional",
        );
        let mut invalid_static_source =
            fs::read_to_string(&invalid_static_config).expect("static control configuration");
        invalid_static_source.push_str(
            "\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = 1\ngraph_references = 2000000\ndiagnostics = 200000\nsource_nodes = 200000\nsource_edges = 400000\nsource_depth = 64\nsource_visited = 200000\n",
        );
        fs::write(&invalid_static_config, invalid_static_source).expect("static control budgets");
        let invalid_static_options = CliOptions {
            values: BTreeMap::from([
                (
                    String::from("root"),
                    invalid_static.path().to_string_lossy().into_owned(),
                ),
                (
                    String::from("config"),
                    invalid_static_config.to_string_lossy().into_owned(),
                ),
            ]),
            mounts: Vec::new(),
        };
        assert_eq!(static_command(&invalid_static_options), EXIT_INPUT);

        let directory = tempfile::TempDir::new().expect("real I/O root");
        let empty = directory.path().join("empty");
        fs::create_dir(&empty).expect("empty directory");
        let removable = directory.path().join("removable");
        fs::write(&removable, b"remove").expect("removable file");
        let mut real = RealBindingIo::default();
        let unopened = directory.path().join("unopened");
        assert!(real.write_all(&unopened, b"missing").is_err());
        assert!(real.sync_all(&unopened).is_err());
        assert!(real.remove_file(&removable).is_ok());
        assert!(real.remove_dir(&empty).is_ok());
        let cleanup_parent = directory.path().join("cleanup");
        let cleanup_child = cleanup_parent.join("child");
        fs::create_dir_all(&cleanup_child).expect("cleanup directories");
        let cleanup_object_temporary = cleanup_child.join("object.tmp");
        let cleanup_attachment_temporary = cleanup_child.join("attachment.tmp");
        let cleanup_object = cleanup_child.join("object.toml");
        for path in [
            &cleanup_object_temporary,
            &cleanup_attachment_temporary,
            &cleanup_object,
        ] {
            fs::write(path, b"cleanup").expect("cleanup file");
        }
        assert!(
            cleanup_artifact_transaction(
                &mut real,
                &cleanup_object_temporary,
                &cleanup_attachment_temporary,
                Some(&cleanup_object),
                &[cleanup_parent.clone(), cleanup_child],
            )
            .is_ok()
        );
        assert!(!cleanup_parent.exists());

        let edit = artifact_binding_edit_control();
        let root = Path::new("control");
        let mut artifact_prefix = ArtifactIoControl::new(&edit);
        assert_eq!(
            ensure_parent_directories(
                &mut artifact_prefix,
                root,
                Path::new("outside"),
                &mut Vec::new(),
            ),
            Err(())
        );
        let object = root.join(edit.object_path.as_str());
        let object_temporary = object.with_extension("toml.tmp");
        let attachment = root.join(edit.attachment_path.as_str());
        let attachment_temporary = attachment.with_extension("toml.tmp");

        let mut second_directory = ArtifactIoControl::new(&edit);
        second_directory.state.directories.retain(|directory| {
            *directory == root
                || object
                    .parent()
                    .is_some_and(|parent| parent.starts_with(directory))
        });
        let mut current = object.parent();
        while let Some(directory) = current {
            second_directory
                .state
                .directories
                .insert(directory.to_path_buf());
            current = if directory == root {
                None
            } else {
                directory.parent()
            };
        }
        second_directory.inject(ArtifactIoStage::CreateDirectory, 1);
        assert_eq!(
            apply_artifact_binding_edit_with(&mut second_directory, root, &edit),
            Err(())
        );

        let mut transaction_stale_object = ArtifactIoControl::new(&edit);
        transaction_stale_object
            .state
            .files
            .insert(object_temporary.clone(), b"stale-object".to_vec());
        transaction_stale_object.inject(ArtifactIoStage::RemoveFile, 4);
        assert_eq!(
            apply_artifact_binding_edit_with(&mut transaction_stale_object, root, &edit),
            Err(())
        );
        let mut transaction_stale_attachment = ArtifactIoControl::new(&edit);
        transaction_stale_attachment
            .state
            .files
            .insert(attachment_temporary.clone(), b"stale-attachment".to_vec());
        transaction_stale_attachment.inject(ArtifactIoStage::RemoveFile, 4);
        assert_eq!(
            apply_artifact_binding_edit_with(&mut transaction_stale_attachment, root, &edit),
            Err(())
        );

        let mut without_prior_attachment = ArtifactIoControl::new(&edit);
        without_prior_attachment.state.files.remove(&attachment);
        assert!(
            apply_artifact_binding_edit_with(&mut without_prior_attachment, root, &edit).is_ok()
        );

        let mut stale_object = ArtifactIoControl::new(&edit);
        stale_object
            .state
            .files
            .insert(object_temporary.clone(), b"stale-object".to_vec());
        stale_object.inject(ArtifactIoStage::RemoveFile, 4);
        assert_eq!(
            remove_stale_temporary(&mut stale_object, &object_temporary),
            Err(())
        );
        let mut stale_attachment = ArtifactIoControl::new(&edit);
        stale_attachment
            .state
            .files
            .insert(attachment_temporary.clone(), b"stale-attachment".to_vec());
        stale_attachment.inject(ArtifactIoStage::RemoveFile, 4);
        assert_eq!(
            remove_stale_temporary(&mut stale_attachment, &attachment_temporary),
            Err(())
        );

        let mut stale = ArtifactIoControl::new(&edit);
        stale
            .state
            .files
            .insert(object_temporary.clone(), b"stale-object".to_vec());
        stale
            .state
            .files
            .insert(attachment_temporary.clone(), b"stale-attachment".to_vec());
        assert!(apply_artifact_binding_edit_with(&mut stale, root, &edit).is_ok());

        let mut existing = ArtifactIoControl::new(&edit);
        assert!(existing.create_new(&attachment).is_err());
        assert!(existing.read(Path::new("control/missing-read")).is_err());
        assert!(
            existing
                .rename(
                    Path::new("control/missing-rename"),
                    Path::new("control/rename-target"),
                )
                .is_err()
        );
        assert!(
            existing
                .write(Path::new("control/direct"), b"direct")
                .is_ok()
        );
        let occupied_parent = existing
            .state
            .files
            .keys()
            .next()
            .and_then(|path| path.parent())
            .expect("one controlled file has a parent")
            .to_path_buf();
        assert!(existing.remove_dir(&occupied_parent).is_err());
        assert!(existing.remove_dir(Path::new("control/buoy")).is_err());
        assert!(existing.remove_dir(Path::new("control/missing")).is_err());

        let cleanup_directory = PathBuf::from("control/cleanup-directory");
        existing.state.directories.insert(cleanup_directory.clone());
        assert!(
            cleanup_artifact_transaction(
                &mut existing,
                Path::new("control/absent-object.tmp"),
                Path::new("control/absent-attachment.tmp"),
                None,
                core::slice::from_ref(&cleanup_directory),
            )
            .is_ok()
        );
        assert!(!existing.state.directories.contains(&cleanup_directory));

        let cleanup_file = PathBuf::from("control/cleanup-persistent.tmp");
        existing
            .state
            .files
            .insert(cleanup_file.clone(), Vec::new());
        existing.inject(ArtifactIoStage::RemoveFile, 4);
        assert_eq!(
            cleanup_artifact_transaction(
                &mut existing,
                &cleanup_file,
                Path::new("control/no-attachment"),
                None,
                &[],
            ),
            Err(())
        );
        let cleanup_persistent_directory = PathBuf::from("control/cleanup-persistent-directory");
        existing
            .state
            .directories
            .insert(cleanup_persistent_directory.clone());
        existing.inject(ArtifactIoStage::RemoveDirectory, 4);
        assert_eq!(
            cleanup_artifact_transaction(
                &mut existing,
                Path::new("control/no-object"),
                Path::new("control/no-attachment"),
                None,
                core::slice::from_ref(&cleanup_persistent_directory),
            ),
            Err(())
        );

        let persistent_file = PathBuf::from("control/persistent.tmp");
        existing
            .state
            .files
            .insert(persistent_file.clone(), Vec::new());
        existing.inject(ArtifactIoStage::RemoveFile, 4);
        assert_eq!(retry_remove_file(&mut existing, &persistent_file), Err(()));
        let persistent_directory = PathBuf::from("control/persistent-directory");
        existing
            .state
            .directories
            .insert(persistent_directory.clone());
        existing.inject(ArtifactIoStage::RemoveDirectory, 4);
        assert_eq!(
            retry_remove_directory(&mut existing, &persistent_directory),
            Err(())
        );

        let mut mismatched = ArtifactIoControl::new(&edit);
        mismatched
            .state
            .files
            .insert(object, b"wrong-object".to_vec());
        assert_eq!(
            apply_artifact_binding_edit_with(&mut mismatched, root, &edit),
            Err(())
        );
        let mut failed = ArtifactIoControl::new(&edit);
        failed.inject(ArtifactIoStage::ReadArtifact, 1);
        assert_eq!(
            apply_and_render_artifact_binding(&mut failed, root, &edit, "human"),
            EXIT_INPUT
        );
        let mut human = ArtifactIoControl::new(&edit);
        assert_eq!(
            apply_and_render_artifact_binding(&mut human, root, &edit, "human"),
            EXIT_OK
        );
        let mut machine = ArtifactIoControl::new(&edit);
        assert_eq!(
            apply_and_render_artifact_binding(&mut machine, root, &edit, "machine"),
            EXIT_OK
        );
    }

    #[test]
    fn mounted_artifact_and_oracle_controls_cover_logical_cli_paths() {
        let fixture = Path::new("fixtures/logical-mount-artifact-bindings/v1/barnacle");
        let config = RepositoryConfig::parse(
            &fs::read(fixture.join("buoy.toml")).expect("Barnacle configuration"),
        )
        .expect("Barnacle configuration parse");
        let mappings = MountMappings::new(vec![
            (
                LogicalMountId::new("project").expect("project mount"),
                fixture.join("project"),
            ),
            (
                LogicalMountId::new("authority").expect("authority mount"),
                fixture.join("authority"),
            ),
        ])
        .expect("Barnacle mappings");
        let snapshot = snapshot_mounted_repository(&mappings, &config).expect("Barnacle snapshot");
        let checked = check_snapshot(&snapshot, &config).expect("Barnacle check");
        let projected_artifact =
            RepositoryPath::new("project/flake.nix").expect("projected artifact path");
        let (project_mount, relative_artifact) =
            binding_subject_location(&config, &projected_artifact);
        assert_eq!(
            project_mount.as_ref().map(LogicalMountId::as_str),
            Some("project")
        );
        assert_eq!(relative_artifact.as_str(), "flake.nix");
        let requirement = checked
            .source
            .graph()
            .nodes()
            .iter()
            .find(|node| node.item().kind().as_str() == "openspec/requirement/v1")
            .expect("Barnacle requirement")
            .reference();
        let artifact = checked
            .source
            .graph()
            .nodes()
            .iter()
            .find(|node| node.item().kind().as_str() == "artifact/file/v1")
            .expect("Barnacle artifact");
        assert!(
            prepare_artifact_binding_edit(
                &snapshot,
                &config,
                &requirement.canonical_text(),
                artifact.item().path().as_str(),
                "COVERAGE-MOUNTED-ARTIFACT",
            )
            .is_ok()
        );
        assert_eq!(
            prepare_artifact_binding_edit(
                &snapshot,
                &config,
                &artifact.reference().canonical_text(),
                artifact.item().path().as_str(),
                "COVERAGE-WRONG-REQUIREMENT",
            ),
            Err(BindError::Input)
        );
        assert_eq!(
            prepare_artifact_binding_edit(
                &snapshot,
                &config,
                &requirement.canonical_text(),
                artifact.item().path().as_str(),
                "invalid relation",
            ),
            Err(BindError::Input)
        );
        assert_eq!(
            prepare_artifact_binding_edit(
                &snapshot,
                &config,
                &SourceNodeRef::new(SemanticDigest::blake3_256([0; 32])).canonical_text(),
                artifact.item().path().as_str(),
                "COVERAGE-MISSING-REQUIREMENT",
            ),
            Err(BindError::Input)
        );
        let mut bounded_source =
            fs::read_to_string(fixture.join("buoy.toml")).expect("Barnacle configuration");
        bounded_source.push_str(
            "\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = 1\ngraph_references = 2000000\ndiagnostics = 200000\nsource_nodes = 200000\nsource_edges = 400000\nsource_depth = 64\nsource_visited = 200000\n",
        );
        let bounded_config = RepositoryConfig::parse(bounded_source.as_bytes())
            .expect("bounded Barnacle configuration");
        assert_eq!(
            prepare_artifact_binding_edit(
                &snapshot,
                &bounded_config,
                &requirement.canonical_text(),
                artifact.item().path().as_str(),
                "COVERAGE-INVALID-SNAPSHOT",
            ),
            Err(BindError::Input)
        );

        assert!(invocation_snapshot(&CliOptions::default(), &config).is_err());
        let mounted = InvocationRoots::Mounted(mappings);
        assert_eq!(binding_write_root(&mounted, None), None);

        let oracle_root = tempfile::TempDir::new().expect("mounted oracle root");
        fs::create_dir(oracle_root.path().join("src")).expect("oracle source root");
        fs::write(oracle_root.path().join("src/control.bin"), b"control")
            .expect("oracle source file");
        let oracle_config = Path::new(
            "fixtures/logical-mount-artifact-bindings/v1/configs/oracle-project-mount.toml",
        );
        let options = CliOptions {
            values: BTreeMap::from([
                (
                    String::from("config"),
                    oracle_config.to_string_lossy().into_owned(),
                ),
                (String::from("format"), String::from("machine")),
            ]),
            mounts: vec![format!("project={}", oracle_root.path().display())],
        };
        assert_eq!(execute_command(&options), EXIT_OK);
        assert_eq!(
            execute_command_internal(&options, Some("working-roots")),
            EXIT_INPUT
        );
        assert_eq!(
            execute_command_internal(&options, Some("receipts")),
            EXIT_INPUT
        );

        let v2_root = tempfile::TempDir::new().expect("version 2 artifact root");
        fs::create_dir_all(v2_root.path().join("openspec/specs/demo"))
            .expect("version 2 OpenSpec root");
        fs::copy(
            fixture.join("authority/openspec/specs/demo/spec.md"),
            v2_root.path().join("openspec/specs/demo/spec.md"),
        )
        .expect("version 2 requirement");
        fs::create_dir(v2_root.path().join("artifacts")).expect("version 2 artifact root");
        fs::create_dir_all(v2_root.path().join("buoy/objects")).expect("version 2 object root");
        fs::create_dir(v2_root.path().join("buoy/attachments")).expect("version 2 attachment root");
        fs::copy(
            fixture.join("project/flake.nix"),
            v2_root.path().join("artifacts/artifact.bin"),
        )
        .expect("version 2 artifact");
        let v2_config_source = br#"schema = "spec-provenance-config/v2"
digest_algorithms = ["blake3-256"]

[adapter_configurations]
"openspec/v1" = '''
schema = "buoy.openspec-authority/v1"

[[capabilities]]
name = "demo"
format = "openspec/standing-spec/v1"
scope = "openspec.demo"
atom_kind = "behavioral"
lifecycle = "proposed"
reserved_ids = ["OSA-900"]
'''

[[selectors]]
root = "openspec/specs"
adapter = "openspec/v1"
include = ["**/spec.md"]
exclude = []
authority_expectation = "required"

[[selectors]]
root = "artifacts"
adapter = "artifact/v1"
include = ["**/*.bin"]
exclude = []
authority_expectation = "forbidden"

[[selectors]]
root = "buoy/objects"
adapter = "binding/v1"
include = ["**/*.toml"]
exclude = []
authority_expectation = "forbidden"

[[selectors]]
root = "buoy/attachments"
adapter = "binding/v1"
include = ["**/*.toml"]
exclude = []
authority_expectation = "forbidden"

[[selectors]]
root = "buoy"
adapter = "binding/v1"
include = ["**/*.toml"]
exclude = []
authority_expectation = "forbidden"
"#;
        let v2_config =
            RepositoryConfig::parse(v2_config_source).expect("version 2 artifact configuration");
        fs::write(v2_root.path().join("buoy.toml"), v2_config_source)
            .expect("version 2 CLI configuration");
        assert!(invocation_snapshot(&CliOptions::default(), &v2_config).is_err());
        assert!(
            load_and_check_with_artifacts(
                v2_root.path(),
                &v2_root.path().join("missing-config.toml"),
            )
            .is_err()
        );
        let v2_snapshot =
            snapshot_repository(v2_root.path(), &v2_config).expect("version 2 snapshot");
        let v2_checked = check_snapshot(&v2_snapshot, &v2_config).expect("version 2 check");
        let v2_artifact_path =
            RepositoryPath::new("artifacts/artifact.bin").expect("version 2 artifact path");
        assert_eq!(
            binding_subject_location(&v2_config, &v2_artifact_path),
            (None, v2_artifact_path)
        );
        let v2_requirement = v2_checked
            .source
            .graph()
            .nodes()
            .iter()
            .find(|node| node.item().kind().as_str() == "openspec/requirement/v1")
            .expect("version 2 requirement node")
            .reference();
        assert!(
            prepare_artifact_binding_edit(
                &v2_snapshot,
                &v2_config,
                &v2_requirement.canonical_text(),
                "artifacts/artifact.bin",
                "COVERAGE-V2-ARTIFACT",
            )
            .is_ok()
        );
        let v2_options = CliOptions {
            values: BTreeMap::from([
                (
                    String::from("root"),
                    v2_root.path().to_string_lossy().into_owned(),
                ),
                (
                    String::from("config"),
                    v2_root
                        .path()
                        .join("buoy.toml")
                        .to_string_lossy()
                        .into_owned(),
                ),
                (String::from("requirement"), v2_requirement.canonical_text()),
                (
                    String::from("artifact"),
                    String::from("artifacts/artifact.bin"),
                ),
                (
                    String::from("relation"),
                    String::from("COVERAGE-V2-ARTIFACT"),
                ),
            ]),
            mounts: Vec::new(),
        };
        assert_eq!(
            bind_command_internal(&v2_options, Some("write-root")),
            EXIT_INPUT
        );
    }

    #[test]
    fn artifact_binding_transaction_controls_restore_every_failure_stage() {
        let edit = artifact_binding_edit_control();
        let unrelated =
            Path::new("control").join("buoy/attachments/v1/blake3-256/unrelated-relation.toml");
        for stage in [
            ArtifactIoStage::ReadArtifact,
            ArtifactIoStage::ReadAttachment,
            ArtifactIoStage::CreateDirectory,
            ArtifactIoStage::CreateObject,
            ArtifactIoStage::WriteObject,
            ArtifactIoStage::SyncObject,
            ArtifactIoStage::RenameObject,
            ArtifactIoStage::CreateAttachment,
            ArtifactIoStage::WriteAttachment,
            ArtifactIoStage::SyncAttachment,
            ArtifactIoStage::RenameAttachment,
        ] {
            let mut io = ArtifactIoControl::new(&edit);
            io.state
                .files
                .insert(unrelated.clone(), b"unrelated-sidecar".to_vec());
            let prior = io.state.clone();
            io.inject(stage, 1);
            assert!(
                apply_artifact_binding_edit_with(&mut io, Path::new("control"), &edit).is_err(),
                "{stage:?}"
            );
            assert_eq!(io.state, prior, "{stage:?}");
            let model = artifact_binding_transaction_state(
                false,
                Some(b"prior-sidecar"),
                edit.attachment_bytes.as_bytes(),
                Some(stage.model_stage()),
            );
            assert!(!model.object_present);
            assert_eq!(
                model.attachment.as_deref(),
                Some(b"prior-sidecar".as_slice())
            );
            assert!(!model.temporary_present);
            assert!(!model.committed);
        }

        for cleanup in [
            ArtifactIoStage::RemoveFile,
            ArtifactIoStage::RemoveDirectory,
        ] {
            let mut io = ArtifactIoControl::new(&edit);
            io.state
                .files
                .insert(unrelated.clone(), b"unrelated-sidecar".to_vec());
            let prior = io.state.clone();
            io.inject(ArtifactIoStage::RenameAttachment, 1);
            io.inject(cleanup, 2);
            assert!(
                apply_artifact_binding_edit_with(&mut io, Path::new("control"), &edit).is_err()
            );
            assert_eq!(io.state, prior, "cleanup retry {cleanup:?}");
            let model = artifact_binding_transaction_state(
                false,
                Some(b"prior-sidecar"),
                edit.attachment_bytes.as_bytes(),
                Some(cleanup.model_stage()),
            );
            assert!(!model.object_present);
            assert!(!model.temporary_present);
            assert!(!model.committed);
            assert_eq!(
                model.attachment.as_deref(),
                Some(b"prior-sidecar".as_slice())
            );
        }

        let mut absent_cleanup = ArtifactIoControl::new(&edit);
        assert!(retry_remove_file(&mut absent_cleanup, Path::new("control/absent")).is_ok());
        assert!(
            retry_remove_directory(&mut absent_cleanup, Path::new("control/absent-directory"))
                .is_ok()
        );
        let directory_parent = PathBuf::from("control/directory-parent");
        let directory_child = directory_parent.join("child");
        absent_cleanup
            .state
            .directories
            .insert(directory_parent.clone());
        absent_cleanup.state.directories.insert(directory_child);
        assert!(absent_cleanup.remove_dir(&directory_parent).is_err());
        assert!(
            cleanup_artifact_transaction(
                &mut absent_cleanup,
                Path::new("control/no-object"),
                Path::new("control/no-attachment"),
                None,
                &[PathBuf::from("control/no-directory")],
            )
            .is_ok()
        );

        assert_eq!(
            ArtifactIoStage::ReadObject.model_stage(),
            ArtifactBindingTransactionStage::Validate
        );
        let mut retry = ArtifactIoControl::new(&edit);
        retry
            .state
            .files
            .insert(unrelated.clone(), b"unrelated-sidecar".to_vec());
        let prior = retry.state.clone();
        retry.inject(ArtifactIoStage::RenameAttachment, 1);
        assert!(apply_artifact_binding_edit_with(&mut retry, Path::new("control"), &edit).is_err());
        assert_eq!(retry.state, prior);
        assert!(apply_artifact_binding_edit_with(&mut retry, Path::new("control"), &edit).is_ok());
        let artifact = Path::new("control").join(edit.artifact_path.as_str());
        let object = Path::new("control").join(edit.object_path.as_str());
        let attachment = Path::new("control").join(edit.attachment_path.as_str());
        assert_eq!(retry.state.files.get(&artifact), Some(&edit.artifact_bytes));
        assert_eq!(
            retry.state.files.get(&object).map(Vec::as_slice),
            Some(edit.object_bytes.as_bytes())
        );
        assert_eq!(
            retry.state.files.get(&attachment).map(Vec::as_slice),
            Some(edit.attachment_bytes.as_bytes())
        );
        assert!(
            retry
                .state
                .files
                .keys()
                .all(|path| path.extension().and_then(|value| value.to_str()) != Some("tmp"))
        );

        let committed = retry.state.clone();
        retry.inject(ArtifactIoStage::ReadObject, 1);
        assert!(apply_artifact_binding_edit_with(&mut retry, Path::new("control"), &edit).is_err());
        assert_eq!(retry.state, committed);
        assert!(apply_artifact_binding_edit_with(&mut retry, Path::new("control"), &edit).is_ok());
        let retry_model = artifact_binding_transaction_state(
            true,
            Some(edit.attachment_bytes.as_bytes()),
            edit.attachment_bytes.as_bytes(),
            None,
        );
        assert!(retry_model.object_present);
        assert!(retry_model.committed);
        assert_eq!(retry_model.completed_steps, 11);
        assert_eq!(
            retry.state.files.get(&unrelated).map(Vec::as_slice),
            Some(b"unrelated-sidecar".as_slice())
        );

        let mut drifted = ArtifactIoControl::new(&edit);
        drifted.state.files.insert(artifact, b"drifted".to_vec());
        let drifted_prior = drifted.state.clone();
        assert!(
            apply_artifact_binding_edit_with(&mut drifted, Path::new("control"), &edit).is_err()
        );
        assert_eq!(drifted.state, drifted_prior);
    }

    #[cfg(all(feature = "coverage-controls", feature = "blake3-256"))]
    #[test]
    fn binding_cli_controls_cover_direct_success_and_input_failures() {
        const REQUIREMENT: &str =
            "blake3-256:a07ec0fdd5f3250af9d18ba6025f9788ccb2f98df6c4538d16e0c695ccff34b7";
        const IMPLEMENTATION: &str =
            "blake3-256:686f70dca776da0eb0c9159ffb1eb1eb04adeb83ecb8e81f3435d1dedbd71915";
        const STALE: &str =
            "blake3-256:0000000000000000000000000000000000000000000000000000000000000000";
        const FOREIGN: &str = "xxh3-128:00000000000000000000000000000000";
        for format in ["machine", "human"] {
            let repository = copy_binding_repository();
            let root = repository.path().to_string_lossy().into_owned();
            let config = repository
                .path()
                .join("buoy.toml")
                .to_string_lossy()
                .into_owned();
            assert_eq!(
                run(&arguments(&[
                    "bind",
                    "--root",
                    &root,
                    "--config",
                    &config,
                    "--requirement",
                    REQUIREMENT,
                    "--implementation",
                    IMPLEMENTATION,
                    "--relation",
                    "BGLC-RENDER-CATALOG-V2",
                    "--format",
                    format,
                ])),
                EXIT_OK
            );
        }

        let write_root_control = copy_binding_repository();
        let write_root_options = CliOptions {
            values: BTreeMap::from([
                (
                    String::from("root"),
                    write_root_control.path().to_string_lossy().into_owned(),
                ),
                (
                    String::from("config"),
                    write_root_control
                        .path()
                        .join("buoy.toml")
                        .to_string_lossy()
                        .into_owned(),
                ),
                (String::from("requirement"), String::from(REQUIREMENT)),
                (String::from("implementation"), String::from(IMPLEMENTATION)),
                (
                    String::from("relation"),
                    String::from("BGLC-RENDER-CATALOG-V2"),
                ),
            ]),
            mounts: Vec::new(),
        };
        assert_eq!(
            bind_command_internal(&write_root_options, Some("write-root")),
            EXIT_INPUT
        );

        let repository = copy_binding_repository();
        let root = repository.path().to_string_lossy().into_owned();
        let config = repository
            .path()
            .join("buoy.toml")
            .to_string_lossy()
            .into_owned();
        for (requirement, implementation, relation, format, exit) in [
            (
                "invalid",
                IMPLEMENTATION,
                "BGLC-RENDER-CATALOG-V2",
                "machine",
                EXIT_INPUT,
            ),
            (
                REQUIREMENT,
                "invalid",
                "BGLC-RENDER-CATALOG-V2",
                "machine",
                EXIT_INPUT,
            ),
            (
                REQUIREMENT,
                IMPLEMENTATION,
                "invalid relation",
                "machine",
                EXIT_INPUT,
            ),
            (
                FOREIGN,
                IMPLEMENTATION,
                "BGLC-RENDER-CATALOG-V2",
                "machine",
                EXIT_INPUT,
            ),
            (
                STALE,
                IMPLEMENTATION,
                "BGLC-RENDER-CATALOG-V2",
                "machine",
                EXIT_INPUT,
            ),
            (
                REQUIREMENT,
                STALE,
                "BGLC-RENDER-CATALOG-V2",
                "machine",
                EXIT_INPUT,
            ),
            (
                IMPLEMENTATION,
                IMPLEMENTATION,
                "BGLC-RENDER-CATALOG-V2",
                "machine",
                EXIT_INPUT,
            ),
            (
                REQUIREMENT,
                REQUIREMENT,
                "BGLC-RENDER-CATALOG-V2",
                "machine",
                EXIT_INPUT,
            ),
            (
                REQUIREMENT,
                IMPLEMENTATION,
                "BGLC-RENDER-CATALOG",
                "machine",
                EXIT_INPUT,
            ),
            (
                REQUIREMENT,
                IMPLEMENTATION,
                "BGLC-RENDER-CATALOG-V2",
                "invalid",
                EXIT_USAGE,
            ),
        ] {
            assert_eq!(
                run(&arguments(&[
                    "bind",
                    "--root",
                    &root,
                    "--config",
                    &config,
                    "--requirement",
                    requirement,
                    "--implementation",
                    implementation,
                    "--relation",
                    relation,
                    "--format",
                    format,
                ])),
                exit
            );
        }
        assert_eq!(run(&arguments(&["bind"])), EXIT_USAGE);
        assert_eq!(run(&arguments(&["bind", "--root", "."])), EXIT_USAGE);
        assert_eq!(
            run(&arguments(&[
                "bind",
                "--root",
                ".",
                "--config",
                "missing-config",
            ])),
            EXIT_USAGE
        );
        assert_eq!(
            run(&arguments(&[
                "bind",
                "--root",
                &root,
                "--config",
                "missing-config",
                "--requirement",
                REQUIREMENT,
                "--implementation",
                IMPLEMENTATION,
                "--relation",
                "BGLC-RENDER-CATALOG-V2",
            ])),
            EXIT_INPUT
        );

        let missing_root = copy_binding_repository();
        fs::remove_dir_all(missing_root.path().join("src")).expect("removed selected root");
        let missing_root_text = missing_root.path().to_string_lossy().into_owned();
        let missing_root_config = missing_root
            .path()
            .join("buoy.toml")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            run(&arguments(&[
                "bind",
                "--root",
                &missing_root_text,
                "--config",
                &missing_root_config,
                "--requirement",
                REQUIREMENT,
                "--implementation",
                IMPLEMENTATION,
                "--relation",
                "BGLC-RENDER-CATALOG-V2",
            ])),
            EXIT_INPUT
        );

        let source_budget = copy_binding_repository();
        let source_budget_config = source_budget.path().join("buoy.toml");
        let mut budget_source =
            fs::read_to_string(&source_budget_config).expect("binding source-budget configuration");
        budget_source.push_str(
            "\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = 200000\ngraph_references = 2000000\ndiagnostics = 200000\nsource_nodes = 1\nsource_edges = 400000\nsource_depth = 64\nsource_visited = 200000\n",
        );
        fs::write(&source_budget_config, budget_source).expect("source-budget configuration");
        assert_eq!(
            run(&arguments(&[
                "bind",
                "--root",
                &source_budget.path().to_string_lossy(),
                "--config",
                &source_budget_config.to_string_lossy(),
                "--requirement",
                REQUIREMENT,
                "--implementation",
                IMPLEMENTATION,
                "--relation",
                "BGLC-RENDER-CATALOG-V2",
            ])),
            EXIT_INPUT
        );

        let no_authority = copy_binding_repository();
        let no_authority_config = no_authority.path().join("buoy.toml");
        let config_source = fs::read_to_string(&no_authority_config)
            .expect("binding configuration without authority");
        fs::write(
            &no_authority_config,
            config_source.replace("reserved_ids = [\"BGLC-003\"]", "reserved_ids = []"),
        )
        .expect("configuration without authority");
        assert_eq!(
            run(&arguments(&[
                "bind",
                "--root",
                &no_authority.path().to_string_lossy(),
                "--config",
                &no_authority_config.to_string_lossy(),
                "--requirement",
                REQUIREMENT,
                "--implementation",
                IMPLEMENTATION,
                "--relation",
                "BGLC-RENDER-CATALOG-V2",
            ])),
            EXIT_INPUT
        );
    }

    #[test]
    fn effectful_cli_controls_cover_each_command_and_manifest_failure() {
        let valid_atom = atom("CLI behavior MUST remain stable.");
        let (repository, config_path) = make_repository(&valid_atom, "", "optional");
        let root = repository.path().to_string_lossy().into_owned();
        let config_text = config_path.to_string_lossy().into_owned();
        for format in ["machine", "human"] {
            assert_eq!(
                run(&arguments(&[
                    "static",
                    "--root",
                    &root,
                    "--config",
                    &config_text,
                    "--format",
                    format,
                ])),
                EXIT_OK
            );
        }
        assert_eq!(
            run(&arguments(&[
                "static",
                "--root",
                &root,
                "--config",
                &config_text,
                "--format",
                "unknown",
            ])),
            EXIT_INPUT
        );
        assert_eq!(
            run(&arguments(&[
                "static",
                "--root",
                &root,
                "--config",
                "missing-config",
            ])),
            EXIT_INPUT
        );

        let (diagnostic_repository, diagnostic_config) =
            make_repository("Text MUST be atomized.", "", "required");
        let diagnostic_root = diagnostic_repository.path().to_string_lossy().into_owned();
        let diagnostic_config = diagnostic_config.to_string_lossy().into_owned();
        assert_eq!(
            run(&arguments(&[
                "static",
                "--root",
                &diagnostic_root,
                "--config",
                &diagnostic_config,
            ])),
            EXIT_STATIC
        );

        for oracle in ["builtin:pass", "builtin:fail"] {
            let oracle_config = format!(
                "\n[[oracles]]\nid = \"ORC-CONTROL\"\nprogram = \"{oracle}\"\ntimeout_ms = 10\n"
            );
            let (execution_repository, execution_config) =
                make_repository(&valid_atom, &oracle_config, "optional");
            let execution_root = execution_repository.path().to_string_lossy().into_owned();
            let execution_config = execution_config.to_string_lossy().into_owned();
            assert_eq!(
                run(&arguments(&[
                    "execute",
                    "--root",
                    &execution_root,
                    "--config",
                    &execution_config,
                ])),
                if oracle == "builtin:pass" {
                    EXIT_OK
                } else {
                    EXIT_EXECUTION
                }
            );
            if oracle == "builtin:pass" {
                assert_eq!(
                    run(&arguments(&[
                        "execute",
                        "--root",
                        &execution_root,
                        "--config",
                        &execution_config,
                        "--format",
                        "unknown",
                    ])),
                    EXIT_INPUT
                );
            }
        }
        let (duplicate_repository, duplicate_config) = make_repository(&valid_atom, "", "optional");
        fs::write(
            duplicate_repository.path().join("spec/duplicate.md"),
            valid_atom.as_bytes(),
        )
        .expect("duplicate execute source");
        assert_eq!(
            run(&arguments(&[
                "execute",
                "--root",
                &duplicate_repository.path().to_string_lossy(),
                "--config",
                &duplicate_config.to_string_lossy(),
            ])),
            EXIT_INPUT
        );
        let noncanonical_rust =
            String::from_utf8_lossy(include_bytes!("../fixtures/adapters/rust/positive.rs"))
                .replace("CAR-ADAPTER-001", "CAR-e\u{301}");
        let (rust_repository, rust_config) = make_repository("", "", "optional");
        fs::write(
            rust_repository.path().join("spec/control.rs"),
            noncanonical_rust.as_bytes(),
        )
        .expect("noncanonical Rust control");
        fs::write(
            &rust_config,
            config("", "optional")
                .replace("adapter = \"markdown/v1\"", "adapter = \"rust/v1\"")
                .replace("include = [\"**/*.md\"]", "include = [\"**/*.rs\"]"),
        )
        .expect("Rust control config");
        let rust_root_text = rust_repository.path().to_string_lossy().into_owned();
        let rust_config_text = rust_config.to_string_lossy().into_owned();
        assert_eq!(
            run(&arguments(&[
                "execute",
                "--root",
                &rust_root_text,
                "--config",
                &rust_config_text,
            ])),
            EXIT_STATIC
        );

        assert_eq!(
            run(&arguments(&[
                "execute",
                "--root",
                "missing-root",
                "--config",
                &config_text,
            ])),
            EXIT_INPUT
        );
        assert_eq!(
            run(&arguments(&[
                "execute",
                "--root",
                &root,
                "--config",
                "missing-config",
            ])),
            EXIT_INPUT
        );
        assert_eq!(
            run(&arguments(&[
                "execute",
                "--root",
                &diagnostic_root,
                "--config",
                &diagnostic_config,
            ])),
            EXIT_STATIC
        );

        let checked = load_and_check(repository.path(), &config_path).expect("CLI graph");
        let manifest_path = repository.path().join("manifest.toml");
        let manifest = format!(
            "schema = \"spec-provenance-change-manifest/v1\"\nid = \"CHANGE-CONTROL\"\nbase_graph = \"{}\"\nhead_graph = \"{}\"\ndispositions = []\n",
            checked.graph_digest, checked.graph_digest
        );
        fs::write(&manifest_path, &manifest).expect("CLI manifest");
        let manifest_text = manifest_path.to_string_lossy().into_owned();
        assert_eq!(
            run(&arguments(&[
                "change",
                "--base",
                &root,
                "--head",
                &root,
                "--config",
                &config_text,
                "--manifest",
                &manifest_text,
            ])),
            EXIT_OK
        );

        let (head_repository, _) = make_repository(
            &atom("Changed CLI behavior MUST remain stable."),
            "",
            "optional",
        );
        let head_checked =
            load_and_check(head_repository.path(), &config_path).expect("changed CLI graph");
        let impact_manifest_path = repository.path().join("impact.toml");
        fs::write(
            &impact_manifest_path,
            format!(
                "schema = \"spec-provenance-change-manifest/v1\"\nid = \"CHANGE-IMPACT\"\nbase_graph = \"{}\"\nhead_graph = \"{}\"\ndispositions = []\n",
                checked.graph_digest, head_checked.graph_digest
            ),
        )
        .expect("impact manifest");
        let head_root = head_repository.path().to_string_lossy().into_owned();
        let impact_manifest_text = impact_manifest_path.to_string_lossy().into_owned();
        assert_eq!(
            run(&arguments(&[
                "change",
                "--base",
                &root,
                "--head",
                &head_root,
                "--config",
                &config_text,
                "--manifest",
                &impact_manifest_text,
            ])),
            EXIT_CHANGE
        );

        let mismatch_path = repository.path().join("mismatch.toml");
        fs::write(
            &mismatch_path,
            manifest.replacen(
                &checked.graph_digest.canonical_text(),
                &SemanticDigest::xxh3_128([9; 16]).canonical_text(),
                1,
            ),
        )
        .expect("mismatch manifest");
        let mismatch_text = mismatch_path.to_string_lossy().into_owned();
        assert_eq!(
            run(&arguments(&[
                "change",
                "--base",
                &root,
                "--head",
                &root,
                "--config",
                &config_text,
                "--manifest",
                &mismatch_text,
            ])),
            EXIT_CHANGE
        );
        let head_mismatch_path = repository.path().join("head-mismatch.toml");
        fs::write(
            &head_mismatch_path,
            manifest.replace(
                &format!("head_graph = \"{}\"", checked.graph_digest),
                &format!("head_graph = \"{}\"", SemanticDigest::xxh3_128([8; 16])),
            ),
        )
        .expect("head mismatch manifest");
        let head_mismatch_text = head_mismatch_path.to_string_lossy().into_owned();
        assert_eq!(
            run(&arguments(&[
                "change",
                "--base",
                &root,
                "--head",
                &root,
                "--config",
                &config_text,
                "--manifest",
                &head_mismatch_text,
            ])),
            EXIT_CHANGE
        );

        let supplied_ratchet_path = repository.path().join("supplied-ratchet.toml");
        fs::write(
            &supplied_ratchet_path,
            format!("{manifest}\npolicy_ratchets = []\n"),
        )
        .expect("manifest-supplied ratchet control");
        let supplied_ratchet_text = supplied_ratchet_path.to_string_lossy().into_owned();
        assert_eq!(
            run(&arguments(&[
                "change",
                "--base",
                &root,
                "--head",
                &root,
                "--config",
                &config_text,
                "--manifest",
                &supplied_ratchet_text,
            ])),
            EXIT_INPUT
        );

        assert_eq!(
            run(&arguments(&[
                "change",
                "--base",
                "missing-root",
                "--head",
                &root,
                "--config",
                &config_text,
                "--manifest",
                &manifest_text,
            ])),
            EXIT_INPUT
        );
        assert_eq!(
            run(&arguments(&[
                "change",
                "--base",
                &root,
                "--head",
                &root,
                "--config",
                &config_text,
                "--manifest",
                "missing-manifest",
            ])),
            EXIT_INPUT
        );
        assert_eq!(
            run(&arguments(&[
                "change",
                "--base",
                &root,
                "--head",
                &root,
                "--config",
                &config_text,
                "--manifest",
                &manifest_text,
                "--format",
                "unknown",
            ])),
            EXIT_INPUT
        );

        let controls = TempDir::new().expect("manifest controls");
        let disposition_prefix = manifest.replace("dispositions = []\n", "");
        let write = |name: &str, bytes: &[u8]| {
            let path = controls.path().join(name);
            fs::write(&path, bytes).expect("manifest control");
            path
        };
        let full_manifest = format!(
            "schema = \"spec-provenance-change-manifest/v1\"\nid = \"CHANGE-FULL\"\nbase_graph = \"{}\"\nhead_graph = \"{}\"\n\n[approval]\nauthority = \"maintainers\"\nrecord = \"reviews/change.md\"\ndigest = \"{}\"\n\n[[dispositions]]\nid = \"DISPOSITION-FULL\"\nkind = \"atom\"\nobject = \"ATOM-FULL\"\nbase = \"{}\"\nhead = \"{}\"\ndisposition = \"revised\"\n\n[[dispositions.replacements]]\nid = \"ATOM-REPLACEMENT\"\ndigest = \"{}\"\n",
            SemanticDigest::xxh3_128([1; 16]),
            SemanticDigest::xxh3_128([2; 16]),
            SemanticDigest::xxh3_128([3; 16]),
            SemanticDigest::xxh3_128([4; 16]),
            SemanticDigest::xxh3_128([5; 16]),
            SemanticDigest::xxh3_128([6; 16]),
        );
        let parsed_full = parse_manifest(&write("full", full_manifest.as_bytes()))
            .expect("full protocol manifest");
        assert!(parsed_full.manifest.approval().is_some());
        assert_eq!(
            parsed_full.manifest.dispositions()[0].replacements().len(),
            1
        );
        assert!(parse_manifest(&controls.path().join("missing")).is_err());
        assert!(parse_manifest(&write("utf8", &[0xff])).is_err());
        for (name, source) in [
            ("toml", String::from("not = [toml")),
            (
                "schema",
                manifest.replace("spec-provenance-change-manifest/v1", "wrong"),
            ),
            (
                "base",
                manifest.replace(&checked.graph_digest.canonical_text(), "wrong"),
            ),
            (
                "head",
                manifest.replace(
                    &format!("head_graph = \"{}\"", checked.graph_digest),
                    "head_graph = \"wrong\"",
                ),
            ),
            (
                "kind",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D\"\nkind = \"unknown\"\nobject = \"O\"\ndisposition = \"reaffirmed\"\n"
                ),
            ),
            (
                "disposition",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D\"\nkind = \"atom\"\nobject = \"O\"\ndisposition = \"unknown\"\n"
                ),
            ),
            (
                "object-id",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D\"\nkind = \"atom\"\nobject = \"invalid id\"\ndisposition = \"reaffirmed\"\n"
                ),
            ),
            (
                "disposition-construction",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D\"\nkind = \"atom\"\nobject = \"O\"\ndisposition = \"reaffirmed\"\n"
                ),
            ),
            (
                "base-digest",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D\"\nkind = \"atom\"\nobject = \"O\"\nbase = \"wrong\"\ndisposition = \"reaffirmed\"\n"
                ),
            ),
            (
                "head-digest",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D\"\nkind = \"atom\"\nobject = \"O\"\nhead = \"wrong\"\ndisposition = \"reaffirmed\"\n"
                ),
            ),
            (
                "reaffirmation-digest",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D\"\nkind = \"atom\"\nobject = \"O\"\ndisposition = \"reaffirmed\"\nreaffirmation = \"wrong\"\n"
                ),
            ),
            (
                "waiver-digest",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D\"\nkind = \"atom\"\nobject = \"O\"\ndisposition = \"waived\"\n[dispositions.waiver]\nid = \"WAIVER\"\ndigest = \"wrong\"\n"
                ),
            ),
            (
                "replacement-digest",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D\"\nkind = \"atom\"\nobject = \"O\"\ndisposition = \"revised\"\n[[dispositions.replacements]]\nid = \"REPLACEMENT\"\ndigest = \"wrong\"\n"
                ),
            ),
            (
                "approval-digest",
                format!(
                    "{manifest}\n[approval]\nauthority = \"maintainers\"\nrecord = \"reviews/change.md\"\ndigest = \"wrong\"\n"
                ),
            ),
            (
                "disposition-encoding",
                format!(
                    "{disposition_prefix}\n[[dispositions]]\nid = \"D-e{}\"\nkind = \"atom\"\nobject = \"O\"\ndisposition = \"reaffirmed\"\n",
                    '\u{301}'
                ),
            ),
            (
                "manifest-encoding",
                manifest.replace("CHANGE-CONTROL", &format!("CHANGE-e{}", '\u{301}')),
            ),
        ] {
            let source = source
                .replace("id = \"D\"\n", "id = \"DISPOSITION-D\"\n")
                .replace("object = \"O\"\n", "object = \"OBJECT-D\"\n");
            assert!(
                parse_manifest(&write(&name, source.as_bytes())).is_err(),
                "{name}"
            );
        }
        let changed_digest = SemanticDigest::xxh3_128([9; 16]);
        let valid_disposition_prefix = disposition_prefix.replace(
            &format!("head_graph = \"{}\"", checked.graph_digest),
            &format!("head_graph = \"{changed_digest}\""),
        );
        let disposition_manifest = format!(
            "{valid_disposition_prefix}\n[[dispositions]]\nid = \"DISPOSITION-1\"\nkind = \"atom\"\nobject = \"OBJECT-1\"\nbase = \"{}\"\nhead = \"{changed_digest}\"\ndisposition = \"revised\"\n",
            checked.graph_digest,
        );
        assert!(
            parse_manifest(&write("valid-disposition", disposition_manifest.as_bytes())).is_ok()
        );
        let invalid_manifest = disposition_manifest.replace(
            &format!("head_graph = \"{changed_digest}\""),
            &format!("head_graph = \"{}\"", checked.graph_digest),
        );
        assert!(
            parse_manifest(&write("manifest-construction", invalid_manifest.as_bytes())).is_err()
        );
        let valid_digest = SemanticDigest::xxh3_128([7; 16]).canonical_text();
        assert!(
            parse_object_reference(
                ObjectKind::Atom,
                &ReferenceInput {
                    id: String::from("invalid id"),
                    digest: valid_digest.clone(),
                },
            )
            .is_err()
        );
        assert!(
            parse_object_reference(
                ObjectKind::Atom,
                &ReferenceInput {
                    id: String::from("OBJECT-1"),
                    digest: valid_digest.clone(),
                },
            )
            .is_ok()
        );
        assert!(
            parse_object_reference(
                ObjectKind::Atom,
                &ReferenceInput {
                    id: String::from("OBJECT-1"),
                    digest: String::from("invalid"),
                },
            )
            .is_err()
        );
        assert!(
            parse_approval(&ApprovalInput {
                authority: String::from("invalid authority"),
                record: String::from("reviews/change.md"),
                digest: valid_digest.clone(),
            })
            .is_err()
        );
        assert!(
            parse_approval(&ApprovalInput {
                authority: String::from("maintainers"),
                record: String::from("/absolute"),
                digest: valid_digest,
            })
            .is_err()
        );

        let budget_config = repository.path().join("budget.toml");
        fs::write(
            &budget_config,
            format!(
                "{}\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = 1\ngraph_references = 100\ndiagnostics = 100\n",
                config(
                    "\n[[oracles]]\nid = \"ORC-BUDGET\"\nprogram = \"builtin:pass\"\ntimeout_ms = 10\n",
                    "optional",
                )
            ),
        )
        .expect("budget control config");
        assert!(load_and_check_with_artifacts(repository.path(), &budget_config).is_err());
    }

    fn write_publication_plan(root: &Path, source_set: &[&str]) -> PathBuf {
        let plan_path = root.join("publication-plan.json");
        let source_set_json = source_set
            .iter()
            .map(|path| format!("\"{path}\""))
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            &plan_path,
            format!(
                r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"cli-control","selector":{{"requirement_id":"CLI-CONTROL"}},"expected_rows":1,"source_set":[{source_set_json}],"outputs":[{{"id":"trace-page","mount":"repository","path":"docs/{{publication_basis_path}}.md","kind":"page","lifecycle":"replaceable"}}]}}"#
            ),
        )
        .expect("CLI publication plan");
        plan_path
    }

    #[test]
    fn publication_usage_faults_return_usage_exit() {
        assert_eq!(run(&arguments(&["publication"])), EXIT_USAGE);
        assert_eq!(run(&arguments(&["publication", "unknown"])), EXIT_USAGE);
        assert_eq!(
            run(&arguments(&["publication", "check", "not-an-option"])),
            EXIT_USAGE
        );
        assert_eq!(
            run(&arguments(&["publication", "check", "--plan", "x"])),
            EXIT_USAGE
        );
        assert_eq!(
            run(&arguments(&[
                "publication",
                "check",
                "--artifact",
                "trace-page",
            ])),
            EXIT_USAGE
        );

        let (repository, config_path) =
            make_repository(&atom("CLI-CONTROL boundary"), "", "optional");
        let plan_path = write_publication_plan(repository.path(), &["spec/control.md"]);
        assert_eq!(
            run(&arguments(&[
                "publication",
                "check",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "trace-page",
                "--format",
                "invalid",
            ])),
            EXIT_USAGE
        );
        assert_eq!(
            run(&arguments(&[
                "publication",
                "check",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "trace-page",
            ])),
            EXIT_USAGE
        );
        assert_eq!(
            run(&arguments(&[
                "publication",
                "check",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "trace-page",
                "--config",
                "missing-config.toml",
                "--root",
                repository.path().to_str().expect("root path"),
            ])),
            EXIT_INPUT
        );
        assert_eq!(
            run(&arguments(&[
                "publication",
                "check",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "trace-page",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                "missing-root",
            ])),
            EXIT_INPUT
        );

        assert_eq!(
            run(&arguments(&["trace", "--requirement-id", "CLI-CONTROL"])),
            EXIT_USAGE
        );
        assert_eq!(
            run(&arguments(&[
                "trace",
                "--config",
                "missing-config.toml",
                "--requirement-id",
                "CLI-CONTROL",
                "--root",
                repository.path().to_str().expect("root path"),
            ])),
            EXIT_INPUT
        );
        assert_eq!(
            run(&arguments(&[
                "trace",
                "--config",
                config_path.to_str().expect("config path"),
                "--requirement-id",
                "CLI-CONTROL",
                "--root",
                "missing-root",
            ])),
            EXIT_INPUT
        );
    }

    #[test]
    fn publication_input_fault_returns_input_exit() {
        assert_eq!(
            run(&arguments(&[
                "publication",
                "check",
                "--plan",
                "/nonexistent/plan.json",
                "--artifact",
                "trace-page",
            ])),
            EXIT_INPUT
        );
    }

    #[cfg(feature = "blake3-256")]
    fn write_binding_publication_plan(
        root: &Path,
        publication_id: &str,
        outputs_json: &str,
    ) -> PathBuf {
        let plan_path = root.join("binding-publication-plan.json");
        fs::write(
            &plan_path,
            format!(
                r#"{{"schema":"buoy/trace-publication-plan/v1","publication_id":"{publication_id}","selector":{{"requirement_id":"BGLC-003"}},"expected_rows":1,"source_set":["openspec/specs/demo/spec.md","src/lib.rs","buoy/objects/v1/blake3-256/0e857e33e387325fbf457e426bc42286d9d99926b19f3efa3c203795cfeeb6c4.toml"],"outputs":[{outputs_json}]}}"#
            ),
        )
        .expect("binding publication plan");
        plan_path
    }

    #[cfg(all(feature = "coverage-controls", feature = "blake3-256"))]
    #[test]
    fn trace_and_publication_fault_controls_cover_closed_error_reports() {
        assert_eq!(
            with_cli_faults(PACKAGE_IDENTITY_FAULT, || {
                run(&arguments(&["package-identity"]))
            }),
            EXIT_INPUT
        );
        let repository = copy_binding_repository();
        let config_path = repository.path().join("buoy.toml");
        let trace_arguments = arguments(&[
            "trace",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
            "--requirement-id",
            "BGLC-003",
        ]);
        for (faults, expected) in [
            (TRACE_CHECK_FAULT, EXIT_INPUT),
            (TRACE_KIND_FAULT, EXIT_INPUT),
            (TRACE_CONTEXT_FAULT, EXIT_STATIC),
            (TRACE_QUERY_FAULT, EXIT_STATIC),
            (TRACE_RENDER_FAULT, EXIT_STATIC),
            (TRACE_DIAGNOSTIC_FAULT, EXIT_STATIC),
            (TRACE_CONTEXT_FAULT | TRACE_RENDER_FAULT, EXIT_STATIC),
            (TRACE_QUERY_FAULT | TRACE_RENDER_FAULT, EXIT_STATIC),
        ] {
            assert_eq!(with_cli_faults(faults, || run(&trace_arguments)), expected);
        }

        let evidence_plan =
            write_binding_publication_plan(repository.path(), "cli-evidence-fault", "");
        let evidence_arguments = arguments(&[
            "publication",
            "evidence",
            "--plan",
            evidence_plan.to_str().expect("evidence plan"),
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
        ]);
        assert_eq!(
            with_cli_faults(EVIDENCE_DIGEST_FAULT, || run(&evidence_arguments)),
            EXIT_INPUT
        );

        for (fault, expected) in [
            (PUBLICATION_SNAPSHOT_FAULT, EXIT_INPUT),
            (PUBLICATION_CHECK_FAULT, EXIT_INPUT),
            (PUBLICATION_KIND_FAULT, EXIT_INPUT),
            (PUBLICATION_CONTEXT_FAULT, EXIT_STATIC),
            (PUBLICATION_SELECTOR_FAULT, EXIT_INPUT),
            (PUBLICATION_QUERY_FAULT, EXIT_STATIC),
            (PUBLICATION_BASIS_FAULT, EXIT_INPUT),
            (PUBLICATION_FORCE_MOUNTED_FAULT, EXIT_INPUT),
            (PUBLICATION_OUTPUT_LOGICAL_FAULT, EXIT_INPUT),
            (PUBLICATION_RENDER_PATH_FAULT, EXIT_INPUT),
            (PUBLICATION_REOPEN_FAULT, EXIT_STATIC),
            (PUBLICATION_REVERIFY_FAULT, EXIT_STATIC),
        ] {
            let repository = copy_binding_repository();
            let config_path = repository.path().join("buoy.toml");
            let plan_path = write_binding_publication_plan(
                repository.path(),
                "cli-fault-control",
                r#"{"id":"trace-page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"}"#,
            );
            let operation =
                if matches!(fault, PUBLICATION_REOPEN_FAULT | PUBLICATION_REVERIFY_FAULT) {
                    "generate"
                } else {
                    "check"
                };
            let publication_arguments = arguments(&[
                "publication",
                operation,
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "trace-page",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("root path"),
            ]);
            assert_eq!(
                with_cli_faults(fault, || run(&publication_arguments)),
                expected,
                "fault {fault}"
            );
        }

        let repository = copy_binding_repository();
        let config_path = repository.path().join("buoy.toml");
        let plan_path = write_binding_publication_plan(
            repository.path(),
            "cli-late-artifact",
            r#"{"id":"trace-page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"}"#,
        );
        for artifact in [None, Some("not-declared")] {
            let mut late_arguments = vec![
                String::from("publication"),
                String::from("check"),
                String::from("--plan"),
                plan_path.to_string_lossy().into_owned(),
            ];
            if let Some(artifact) = artifact {
                late_arguments.push(String::from("--artifact"));
                late_arguments.push(String::from(artifact));
            }
            late_arguments.extend([
                String::from("--config"),
                config_path.to_string_lossy().into_owned(),
                String::from("--root"),
                repository.path().to_string_lossy().into_owned(),
            ]);
            let expected = if artifact.is_some() {
                EXIT_INPUT
            } else {
                EXIT_USAGE
            };
            assert_eq!(
                with_cli_faults(PUBLICATION_LATE_ARTIFACT_FAULT, || run(&late_arguments)),
                expected
            );
        }

        for outputs in [
            r#"{"id":"trace-page","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"page","lifecycle":"replaceable"}"#,
            r#"{"id":"trace-page","mount":"repository","path":"docs/same.md","kind":"page","lifecycle":"replaceable"},{"id":"evidence","mount":"repository","path":"./docs/same.md","kind":"evidence","lifecycle":"replaceable"}"#,
        ] {
            let repository = copy_binding_repository();
            let config_path = repository.path().join("buoy.toml");
            let plan_path =
                write_binding_publication_plan(repository.path(), "cli-output-fault", outputs);
            let arguments = arguments(&[
                "publication",
                "check",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "trace-page",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("root path"),
            ]);
            assert_eq!(run(&arguments), EXIT_STATIC);
        }

        for (outputs, faults) in [
            (
                r#"{"id":"trace-page","mount":"Bad mount","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"}"#,
                PUBLICATION_FORCE_MOUNTED_FAULT,
            ),
            (
                r#"{"id":"trace-page","mount":"one","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"},{"id":"evidence","mount":"two","path":"evidence/{publication_basis_path}.md","kind":"evidence","lifecycle":"replaceable"}"#,
                0,
            ),
        ] {
            let repository = copy_binding_repository();
            let config_path = repository.path().join("buoy.toml");
            let plan_path =
                write_binding_publication_plan(repository.path(), "cli-mount-fault", outputs);
            let arguments = arguments(&[
                "publication",
                "check",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "trace-page",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("root path"),
            ]);
            assert_eq!(with_cli_faults(faults, || run(&arguments)), EXIT_INPUT);
        }
    }

    #[cfg(all(feature = "coverage-controls", feature = "blake3-256", unix))]
    #[test]
    fn external_link_operation_fault_controls_cover_every_write_boundary() {
        let outputs = r#"{"id":"trace-page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable","target_base":"https://example.com/trace/"},{"id":"external-link","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"external-link","lifecycle":"replaceable","relation":"documents","page":"trace-page"}"#;
        for (fault, expected) in [
            (EXTERNAL_CANDIDATE_FAULT, EXIT_INPUT),
            (EXTERNAL_DIGEST_FAULT, EXIT_INPUT),
            (EXTERNAL_PARENT_FAULT, EXIT_INPUT),
            (EXTERNAL_PARENT_ABSENT_FAULT, EXIT_INPUT),
            (EXTERNAL_LOCK_FAULT, EXIT_STATIC),
            (EXTERNAL_REOPEN_FAULT, EXIT_STATIC),
            (EXTERNAL_CHANGED_FAULT, EXIT_STATIC),
            (EXTERNAL_WRITE_FAULT, EXIT_INPUT),
            (EXTERNAL_CONFLICT_FAULT, EXIT_STATIC),
            (EXTERNAL_NORMALIZATION_FAULT, EXIT_INPUT),
            (EXTERNAL_DIAGNOSTIC_FAULT, EXIT_INPUT),
            (PUBLICATION_SOURCE_UTF8_FAULT, EXIT_INPUT),
            (PUBLICATION_NORMALIZE_FAULT, EXIT_STATIC),
            (PUBLICATION_OUTPUT_LOGICAL_FAULT, EXIT_INPUT),
        ] {
            let repository = copy_binding_repository();
            let config_path = repository.path().join("buoy.toml");
            let plan_path =
                write_binding_publication_plan(repository.path(), "cli-external-fault", outputs);
            let publication_arguments = arguments(&[
                "publication",
                "generate",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "external-link",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("root path"),
            ]);
            assert_eq!(
                with_cli_faults(fault, || run(&publication_arguments)),
                expected,
                "fault {fault}"
            );
        }

        let missing_repository = copy_binding_repository();
        let missing_config = missing_repository.path().join("buoy.toml");
        let missing_outputs = outputs.replace("openspec/specs/demo/spec.md", "missing.md");
        let missing_plan = write_binding_publication_plan(
            missing_repository.path(),
            "cli-missing-external",
            &missing_outputs,
        );
        assert_eq!(
            run(&arguments(&[
                "publication",
                "check",
                "--plan",
                missing_plan.to_str().expect("missing plan"),
                "--artifact",
                "external-link",
                "--config",
                missing_config.to_str().expect("missing config"),
                "--root",
                missing_repository.path().to_str().expect("missing root"),
            ])),
            EXIT_STATIC
        );

        let repository = copy_binding_repository();
        let config_path = repository.path().join("buoy.toml");
        let plan_path =
            write_binding_publication_plan(repository.path(), "cli-selector-control", outputs);
        let page_arguments = arguments(&[
            "publication",
            "check",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--artifact",
            "trace-page",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
        ]);
        assert_eq!(run(&page_arguments), EXIT_STATIC);
        assert_eq!(
            with_cli_faults(EXTERNAL_NORMALIZATION_FAULT, || run(&page_arguments)),
            EXIT_STATIC
        );

        let evidence_arguments = arguments(&[
            "publication",
            "evidence",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
        ]);
        assert_eq!(run(&evidence_arguments), EXIT_STATIC);
        assert_eq!(
            with_cli_faults(EXTERNAL_NORMALIZATION_FAULT, || run(&evidence_arguments)),
            EXIT_STATIC
        );
        assert_eq!(
            with_cli_faults(EVIDENCE_LINK_TARGET_FAULT, || run(&evidence_arguments)),
            EXIT_STATIC
        );

        let requirement_plan = fs::read_to_string(&plan_path).expect("requirement plan");
        let relation_plan = crate::publication::parse_plan(
            requirement_plan
                .replace(
                    "\"requirement_id\":\"BGLC-003\"",
                    "\"relation_id\":\"RELATION-1\"",
                )
                .as_bytes(),
        )
        .expect("relation plan");
        let config = load_config(&config_path).expect("configuration");
        let snapshot = snapshot_repository(repository.path(), &config).expect("snapshot");
        let checked = check_snapshot(&snapshot, &config).expect("check");
        let requirement_kind =
            SourceItemKind::new("openspec/requirement/v1").expect("requirement kind");
        let context = TraceContext::new(
            checked.source.graph(),
            checked.source.closure(),
            requirement_kind,
            checked.source.authorities(),
            checked.source.attachments(),
            checked.source.store(),
        )
        .expect("trace context");
        let selector = TraceSelector::RequirementId(StableId::new("BGLC-003").expect("selector"));
        let projection =
            trace_query(&context, &selector, TraceHistory::CurrentOnly).expect("trace projection");
        let report = trace_report(&checked, &projection);
        let link = relation_plan
            .outputs()
            .iter()
            .find(|output| output.id == "external-link")
            .expect("external-link output");
        let evidence_output = crate::publication::PublicationOutput {
            id: String::from("evidence"),
            mount: String::from("repository"),
            path: String::from("evidence.md"),
            kind: crate::publication::ArtifactKind::Evidence,
            lifecycle: crate::publication::Lifecycle::Replaceable,
            relation: None,
            page: None,
            target_base: None,
        };
        let physical_roots = InvocationRoots::Single(repository.path().to_path_buf());
        let mut immutable_evidence = evidence_output.clone();
        immutable_evidence.lifecycle = crate::publication::Lifecycle::Immutable;
        assert_eq!(
            publication_evidence_operation(
                "check",
                &immutable_evidence,
                &relation_plan,
                "xxh3-128:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                &report.graph,
                &report,
                &[],
                &[],
                &snapshot,
                &config,
                crate::core::DigestAlgorithm::Xxh3_128,
                &physical_roots,
                PublicationFormat::Machine,
            ),
            EXIT_INPUT
        );
        assert_eq!(
            with_cli_faults(EVIDENCE_DIGEST_FAULT, || {
                publication_evidence_operation(
                    "check",
                    &immutable_evidence,
                    &relation_plan,
                    "blake3-256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    &report.graph,
                    &report,
                    &[],
                    &[],
                    &snapshot,
                    &config,
                    crate::core::DigestAlgorithm::Blake3_256,
                    &physical_roots,
                    PublicationFormat::Machine,
                )
            }),
            EXIT_INPUT
        );
        assert!(
            publication_artifact_parity(
                &evidence_output,
                "blake3-256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                &report,
                &relation_plan,
                &snapshot,
                &config,
                config.active_algorithm().algorithm(),
                &physical_roots,
            )
            .is_err()
        );
        let source_edit = crate::publication::PublicationOutput {
            id: String::from("source-edit"),
            mount: String::from("repository"),
            path: String::from("openspec/specs/demo/spec.md"),
            kind: crate::publication::ArtifactKind::SourceEdit,
            lifecycle: crate::publication::Lifecycle::Replaceable,
            relation: None,
            page: None,
            target_base: None,
        };
        let absent = TempDir::new().expect("absent parity root");
        assert_eq!(
            publication_artifact_parity(
                &source_edit,
                "blake3-256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                &report,
                &relation_plan,
                &snapshot,
                &config,
                config.active_algorithm().algorithm(),
                &InvocationRoots::Single(absent.path().to_path_buf()),
            ),
            Ok(false)
        );
        assert!(
            publication_external_link_candidate(
                link,
                &relation_plan,
                &snapshot,
                &config,
                config.active_algorithm().algorithm(),
                "blake3-256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                &report,
            )
            .is_err()
        );
    }

    #[cfg(feature = "blake3-256")]
    #[test]
    fn publication_check_succeeds_with_a_real_current_row() {
        // A plain markdown atom with no committed binding record can never
        // satisfy a real `trace_query`, so this positive check uses the
        // same frozen, self-consistent binding fixture the read-only trace
        // command already proves resolves to one current row with no
        // diagnostics.
        let repository = copy_binding_repository();
        let config_path = repository.path().join("buoy.toml");
        let plan_path = write_binding_publication_plan(
            repository.path(),
            "cli-control",
            r#"{"id":"trace-page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"}"#,
        );
        let status = run(&arguments(&[
            "publication",
            "check",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--artifact",
            "trace-page",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
        ]));
        // No page exists yet, so a real check must report a static blocker
        // rather than fabricate success.
        assert_eq!(status, EXIT_STATIC);

        let generated = run(&arguments(&[
            "publication",
            "generate",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--artifact",
            "trace-page",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
        ]));
        assert_eq!(generated, EXIT_OK);

        let rechecked = run(&arguments(&[
            "publication",
            "check",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--artifact",
            "trace-page",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
        ]));
        assert_eq!(rechecked, EXIT_OK);
    }

    #[test]
    fn publication_check_detects_recursive_selector_growth() {
        let (repository, config_path) =
            make_repository(&atom("cli publication control"), "", "optional");
        fs::write(repository.path().join("spec/extra.md"), "# Extra\n")
            .expect("extra selected file");
        let plan_path = write_publication_plan(repository.path(), &["spec/control.md"]);
        let status = run(&arguments(&[
            "publication",
            "check",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--artifact",
            "trace-page",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
        ]));
        assert_eq!(status, EXIT_STATIC);
    }

    #[test]
    fn publication_check_rejects_undeclared_artifact() {
        let (repository, config_path) =
            make_repository(&atom("cli publication control"), "", "optional");
        let plan_path = write_publication_plan(repository.path(), &["spec/control.md"]);
        let status = run(&arguments(&[
            "publication",
            "check",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--artifact",
            "not-declared",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
        ]));
        assert_eq!(status, EXIT_INPUT);
    }

    #[cfg(feature = "blake3-256")]
    #[test]
    fn publication_evidence_succeeds_without_artifact_selector() {
        let repository = copy_binding_repository();
        let config_path = repository.path().join("buoy.toml");
        let plan_path =
            write_binding_publication_plan(repository.path(), "cli-evidence-control", "");
        let status = run(&arguments(&[
            "publication",
            "evidence",
            "--plan",
            plan_path.to_str().expect("plan path"),
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
        ]));
        assert_eq!(status, EXIT_OK);
    }

    #[cfg(all(feature = "coverage-controls", feature = "blake3-256", unix))]
    #[test]
    fn source_edit_checks_cover_equal_stale_absent_and_io_outcomes() {
        for (fault, expected) in [
            (0, EXIT_OK),
            (SOURCE_EDIT_LOGICAL_FAULT, EXIT_INPUT),
            (SOURCE_EDIT_RETAINED_FAULT, EXIT_STATIC),
            (SOURCE_EDIT_READ_FAULT, EXIT_STATIC),
            (SOURCE_EDIT_ABSENT_FAULT, EXIT_STATIC),
            (SOURCE_EDIT_OPEN_FAULT, EXIT_STATIC),
            (SOURCE_EDIT_STALE_FAULT, EXIT_STATIC),
        ] {
            let repository = copy_binding_repository();
            let config_path = repository.path().join("buoy.toml");
            let plan_path = write_binding_publication_plan(
                repository.path(),
                "cli-source-edit",
                r#"{"id":"source-edit","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"source-edit","lifecycle":"replaceable"}"#,
            );
            let check = arguments(&[
                "publication",
                "check",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "source-edit",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("root path"),
            ]);
            assert_eq!(
                with_cli_faults(fault, || run(&check)),
                expected,
                "fault {fault}"
            );
        }

        let repository = copy_binding_repository();
        let config_path = repository.path().join("buoy.toml");
        let plan_path = write_binding_publication_plan(
            repository.path(),
            "cli-source-edit-generate",
            r#"{"id":"source-edit","mount":"repository","path":"openspec/specs/demo/spec.md","kind":"source-edit","lifecycle":"replaceable"}"#,
        );
        assert_eq!(
            run(&arguments(&[
                "publication",
                "generate",
                "--plan",
                plan_path.to_str().expect("plan path"),
                "--artifact",
                "source-edit",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("root path"),
            ])),
            EXIT_INPUT
        );
    }

    #[cfg(all(feature = "coverage-controls", feature = "blake3-256", unix))]
    #[test]
    fn explicit_evidence_artifacts_cover_blocked_and_current_candidates() {
        let repository = copy_binding_repository();
        let config_path = repository.path().join("buoy.toml");
        let evidence_plan = write_binding_publication_plan(
            repository.path(),
            "cli-evidence-artifact",
            r#"{"id":"evidence","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"evidence","lifecycle":"replaceable"}"#,
        );
        let invoke = |operation: &str| {
            run(&arguments(&[
                "publication",
                operation,
                "--plan",
                evidence_plan.to_str().expect("evidence plan"),
                "--artifact",
                "evidence",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("root path"),
            ]))
        };
        assert_eq!(invoke("check"), EXIT_STATIC);
        assert_eq!(invoke("generate"), EXIT_OK);
        assert_eq!(invoke("check"), EXIT_OK);
        assert_eq!(
            with_cli_faults(EVIDENCE_MARKDOWN_FAULT, || invoke("check")),
            EXIT_INPUT
        );

        let blocked = copy_binding_repository();
        let blocked_config = blocked.path().join("buoy.toml");
        let blocked_plan = write_binding_publication_plan(
            blocked.path(),
            "cli-evidence-blocked",
            r#"{"id":"trace-page","mount":"repository","path":"docs/{publication_basis_path}.md","kind":"page","lifecycle":"replaceable"},{"id":"evidence","mount":"repository","path":"evidence/{publication_basis_path}.md","kind":"evidence","lifecycle":"replaceable"}"#,
        );
        let blocked_arguments = arguments(&[
            "publication",
            "check",
            "--plan",
            blocked_plan.to_str().expect("blocked plan"),
            "--artifact",
            "evidence",
            "--config",
            blocked_config.to_str().expect("blocked config"),
            "--root",
            blocked.path().to_str().expect("blocked root"),
        ]);
        assert_eq!(run(&blocked_arguments), EXIT_STATIC);
        assert_eq!(
            with_cli_faults(EVIDENCE_PAGE_CANDIDATE_FAULT, || run(&blocked_arguments)),
            EXIT_STATIC
        );
        assert_eq!(
            with_cli_faults(EVIDENCE_PAGE_DIGEST_FAULT, || run(&blocked_arguments)),
            EXIT_STATIC
        );
    }

    #[cfg(all(feature = "coverage-controls", feature = "blake3-256"))]
    #[test]
    fn trace_command_stays_read_only_and_keeps_existing_exit_classes() {
        let repository = copy_binding_repository();
        let config_path = repository.path().join("buoy.toml");
        let spec_path = repository.path().join("openspec/specs/demo/spec.md");
        let before = fs::read(&spec_path).expect("before bytes");
        let object_dir = repository.path().join("buoy/objects/v1/blake3-256");
        let object_count_before = fs::read_dir(&object_dir)
            .expect("object directory listing")
            .count();

        let status = run(&arguments(&[
            "trace",
            "--config",
            config_path.to_str().expect("config path"),
            "--root",
            repository.path().to_str().expect("root path"),
            "--requirement-id",
            "BGLC-003",
        ]));
        assert_eq!(status, EXIT_OK);

        let after = fs::read(&spec_path).expect("after bytes");
        assert_eq!(before, after, "trace must never rewrite selected source");
        let object_count_after = fs::read_dir(&object_dir)
            .expect("object directory listing")
            .count();
        assert_eq!(
            object_count_before, object_count_after,
            "trace must write no new repository file"
        );

        assert_eq!(
            run(&arguments(&[
                "trace",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("root path"),
                "--requirement-id",
                "MISSING-001",
            ])),
            EXIT_INPUT
        );
        let implementation = fs::read_to_string(repository.path().join("src/lib.rs"))
            .expect("implementation source");
        fs::write(
            repository.path().join("src/lib.rs"),
            implementation.replace("libraries.join(\"\\n\")", "String::from(\"changed\")"),
        )
        .expect("stale implementation bytes");
        assert_eq!(
            run(&arguments(&[
                "trace",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("root path"),
                "--requirement-id",
                "BGLC-003",
                "--include-history",
            ])),
            EXIT_STATIC
        );
    }

    #[test]
    fn trace_machine_validation_rejects_forbidden_fields_and_paths() {
        for value in [
            serde_json::json!({"root": "/tmp/repository"}),
            serde_json::json!({"implementation_path": "/tmp/source.rs"}),
            serde_json::json!({"rows": [{"provider": "github"}]}),
        ] {
            assert_eq!(
                validate_trace_machine_value(&value),
                Err("TRC-FORBIDDEN-REPORT-FIELD")
            );
        }
        assert!(
            validate_trace_machine_value(&serde_json::json!({"implementation_path": "src/lib.rs"}))
                .is_ok()
        );
    }

    #[test]
    fn human_trace_expands_colliding_record_prefixes_and_keeps_blockers() {
        let row =
            |record: &str, state: &str, diagnostics: Vec<TraceDiagnosticFact>| TraceReportRow {
                state: String::from(state),
                relation: String::from("TRACE-RELATION"),
                navigation: String::from("implements"),
                record: String::from(record),
                requirement_id: String::from("TRACE-001"),
                atom_revision: String::from(
                    "blake3-256:2222222222222222222222222222222222222222222222222222222222222222",
                ),
                requirement_node: String::from(
                    "blake3-256:3333333333333333333333333333333333333333333333333333333333333333",
                ),
                implementation_node: String::from(
                    "blake3-256:4444444444444444444444444444444444444444444444444444444444444444",
                ),
                requirement_path: Some(String::from("openspec/specs/trace/spec.md")),
                implementation_path: Some(String::from("src/lib.rs")),
                requirement_kind: Some(String::from("openspec/requirement/v1")),
                implementation_kind: Some(String::from("rust/item/v1")),
                requirement_name: Some(String::from("Trace requirement")),
                implementation_name: Some(String::from("trace_item")),
                requirement_span: Some(TraceSpanFact { start: 0, end: 10 }),
                implementation_span: Some(TraceSpanFact { start: 10, end: 20 }),
                freshness: TraceFreshnessFact {
                    record: true,
                    attachment: true,
                    implementation: state != "stale",
                    requirement: true,
                    authority: true,
                },
                traversal: Vec::new(),
                diagnostics,
            };
        let first = "blake3-256:0e857e33aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let second = "blake3-256:0e857e33bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let blocker = TraceDiagnosticFact {
            code: String::from("CAB-SUBJECT-STALE"),
            id: Some(String::from("TRACE-RELATION")),
            path: Some(String::from("src/lib.rs")),
            start: 10,
            end: 20,
        };
        let report = TraceReport {
            schema: String::from(TRACE_REPORT_SCHEMA),
            graph: String::from(
                "blake3-256:1111111111111111111111111111111111111111111111111111111111111111",
            ),
            selector: TraceSelectorFact {
                kind: String::from("relation-id"),
                value: String::from("TRACE-RELATION"),
            },
            include_history: true,
            rows: vec![
                row(first, "current", Vec::new()),
                row(second, "stale", vec![blocker.clone()]),
            ],
            diagnostics: vec![blocker],
        };
        assert!(!records_stay_abbreviated(&report));
        let rendered = render_trace_human(&report);
        assert!(rendered.contains(first));
        assert!(rendered.contains(second));
        assert!(rendered.contains("CAB-SUBJECT-STALE"));

        let bridged = publication_conformance_rows(&report, None);
        assert_eq!(bridged.len(), 2);
        assert_eq!(bridged[0].state.as_deref(), Some("current"));
        assert_eq!(bridged[1].state.as_deref(), Some("stale"));
        assert_eq!(
            crate::publication::row_conformance(&bridged, 2),
            Err(crate::publication::ConformanceError::Stale)
        );
        let single = vec![bridged[0].clone()];
        assert!(crate::publication::row_conformance(&single, 1).is_ok());
    }

    #[cfg(all(feature = "coverage-controls", feature = "blake3-256"))]
    #[test]
    fn trace_and_static_pure_controls_cover_closed_report_variants() {
        use crate::core::{ParsedItemIdentity, SourceNode};

        assert!(render_package_identity("revision", "toolchain", Vec::new()).is_err());
        for (kind, name) in [
            (NavigationKind::Implements, "implements"),
            (NavigationKind::Owns, "owns"),
            (NavigationKind::Consumes, "consumes"),
            (NavigationKind::Configures, "configures"),
        ] {
            assert_eq!(navigation_name(kind), name);
        }
        assert_eq!(
            first_display_line("\n  display line  \nnext"),
            "display line"
        );
        assert_eq!(first_display_line(""), "");
        assert_eq!(first_display_line(&"x".repeat(140)).len(), 120);
        for source in [
            "const ITEM: u8 = 0;",
            "enum Item {}",
            "fn item() {}",
            "mod item {}",
            "static ITEM: u8 = 0;",
            "struct Item;",
            "trait Item {}",
            "trait Item = Other;",
            "type Item = u8;",
            "union Item { value: u8 }",
        ] {
            assert!(rust_item_name(source).is_some(), "{source}");
        }
        assert!(rust_item_name("impl Item {}").is_none());
        assert!(rust_item_name("not valid Rust").is_none());

        let node = |kind: &str, semantic: &str, path: &str| {
            SourceNode::new(
                ParsedItemIdentity::new(
                    SchemaId::new("coverage/v1").expect("coverage schema"),
                    RepositoryPath::new(path).expect("coverage path"),
                    SourceItemKind::new(kind).expect("coverage item kind"),
                    semantic.as_bytes().to_vec(),
                    None,
                )
                .expect("coverage item"),
                Vec::new(),
            )
            .expect("coverage node")
        };
        for (kind, semantic, path, expected) in [
            (
                "openspec/requirement/v1",
                "### Requirement: DEMO-001 Display title\nStatement.",
                "openspec/specs/demo/spec.md",
                "Display title",
            ),
            (
                "openspec/requirement/v1",
                "fallback requirement",
                "openspec/specs/demo/spec.md",
                "fallback requirement",
            ),
            (
                "openspec/scenario/v1",
                "#### Scenario: Display scenario\n",
                "openspec/specs/demo/spec.md",
                "Display scenario",
            ),
            (
                "openspec/scenario/v1",
                "fallback scenario",
                "openspec/specs/demo/spec.md",
                "fallback scenario",
            ),
            (
                "openspec/step/v1",
                "heading\nfirst step\nsecond step",
                "openspec/specs/demo/spec.md",
                "first step second step",
            ),
            (
                "rust/item/v1",
                "fn display_item() {}",
                "src/lib.rs",
                "display_item",
            ),
            (
                "rust/item/v1",
                "invalid Rust display",
                "src/invalid.rs",
                "invalid Rust display",
            ),
            (
                "artifact/file/v1",
                "artifact",
                "data/artifact.bin",
                "artifact.bin",
            ),
            (
                "coverage/unknown/v1",
                "\nunknown item",
                "data/unknown",
                "unknown item",
            ),
        ] {
            assert_eq!(node_display_name(&node(kind, semantic, path)), expected);
        }

        assert_eq!(located(Some("src/lib.rs"), None), "src/lib.rs");
        let diagnostic_repository_path = RepositoryPath::new("src/lib.rs").expect("path");
        assert_eq!(diagnostic_path(None), None);
        assert_eq!(
            diagnostic_path(Some(&diagnostic_repository_path)),
            Some(String::from("src/lib.rs"))
        );
        assert_eq!(
            located(None, Some(TraceSpanFact { start: 1, end: 2 })),
            "not retained in this snapshot"
        );
        assert_eq!(
            located(Some("src/lib.rs"), Some(TraceSpanFact { start: 1, end: 2 })),
            "src/lib.rs:1-2"
        );
        for (key, value, expected_kind) in [
            ("requirement-id", "REQ-001", "requirement-id"),
            ("relation-id", "RELATION-1", "relation-id"),
            ("implementation-path", "src/lib.rs", "implementation-path"),
            (
                "record",
                "blake3-256:1111111111111111111111111111111111111111111111111111111111111111",
                "record",
            ),
        ] {
            let mut options = CliOptions::default();
            options
                .values
                .insert(String::from(key), String::from(value));
            let selector = parse_trace_selector(&options).expect("typed trace selector");
            assert_eq!(trace_selector_fact(&selector).kind, expected_kind);
        }
        for selector in [
            crate::publication::Selector::RequirementId(String::from("REQ-001")),
            crate::publication::Selector::RelationId(String::from("RELATION-1")),
            crate::publication::Selector::ImplementationPath(String::from("src/lib.rs")),
            crate::publication::Selector::Record(String::from(
                "blake3-256:1111111111111111111111111111111111111111111111111111111111111111",
            )),
        ] {
            assert!(publication_core_selector(&selector).is_ok());
        }
        for selector in [
            crate::publication::Selector::RequirementId(String::from("invalid id")),
            crate::publication::Selector::RelationId(String::from("invalid id")),
            crate::publication::Selector::ImplementationPath(String::from("/absolute")),
            crate::publication::Selector::Record(String::from("invalid")),
        ] {
            assert!(publication_core_selector(&selector).is_err());
        }

        assert_eq!(
            run(&arguments(&["static", "--include-history"])),
            EXIT_USAGE
        );
        assert_eq!(run(&arguments(&["package-identity"])), EXIT_OK);

        let repository = copy_binding_repository();
        let config_path = repository.path().join("buoy.toml");
        assert_eq!(
            run(&arguments(&[
                "trace",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("repository root"),
                "--requirement-id",
                "BGLC-003",
                "--unknown",
                "value",
            ])),
            EXIT_USAGE
        );
        assert_eq!(
            run(&arguments(&[
                "trace",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("repository root"),
                "--requirement-id",
                "BGLC-003",
                "--format",
                "invalid",
            ])),
            EXIT_USAGE
        );
        assert_eq!(
            run(&arguments(&[
                "trace",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("repository root"),
                "--requirement-id",
                "invalid id",
            ])),
            EXIT_INPUT
        );
        assert_eq!(
            run(&arguments(&[
                "trace",
                "--config",
                config_path.to_str().expect("config path"),
                "--root",
                repository.path().to_str().expect("repository root"),
                "--requirement-id",
                "BGLC-003",
                "--relation-id",
                "RELATION-1",
            ])),
            EXIT_USAGE
        );
        let config = load_config(&config_path).expect("trace config");
        let snapshot = snapshot_repository(repository.path(), &config).expect("trace snapshot");
        let checked = check_snapshot(&snapshot, &config).expect("trace check");
        let selector =
            TraceSelector::RequirementId(StableId::new("BGLC-003").expect("requirement selector"));
        let blocked = blocked_trace_report(
            &checked,
            &selector,
            TraceHistory::IncludeHistory,
            TraceError::DuplicateRecord,
        );
        assert_eq!(blocked.diagnostics[0].code, "TRC-RECORD-DUPLICATE");
        assert!(render_trace(&blocked, TraceFormat::Machine).is_ok());
        assert!(render_trace(&blocked, TraceFormat::Human).is_ok());
        assert!(render_trace(&blocked, TraceFormat::Markdown).is_ok());
        assert!(render_trace_markdown(&blocked).contains("## Blockers"));
        assert!(records_stay_abbreviated(&blocked));
        assert_eq!(
            unambiguous_row_value(&blocked.rows, RowIdentity::Record),
            None
        );
        let row = TraceReportRow {
            state: String::from("current"),
            relation: String::from("RELATION-1"),
            navigation: String::from("implements"),
            record: String::from(
                "blake3-256:1111111111111111111111111111111111111111111111111111111111111111",
            ),
            requirement_id: String::from("REQ-001"),
            atom_revision: String::from(
                "blake3-256:2222222222222222222222222222222222222222222222222222222222222222",
            ),
            requirement_node: String::from(
                "blake3-256:3333333333333333333333333333333333333333333333333333333333333333",
            ),
            implementation_node: String::from(
                "blake3-256:4444444444444444444444444444444444444444444444444444444444444444",
            ),
            requirement_path: Some(String::from("openspec/specs/demo/spec.md")),
            implementation_path: Some(String::from("src/lib.rs")),
            requirement_kind: Some(String::from("openspec/requirement/v1")),
            implementation_kind: Some(String::from("rust/item/v1")),
            requirement_name: None,
            implementation_name: None,
            requirement_span: None,
            implementation_span: None,
            freshness: TraceFreshnessFact {
                record: true,
                attachment: true,
                implementation: true,
                requirement: true,
                authority: true,
            },
            traversal: Vec::new(),
            diagnostics: Vec::new(),
        };
        assert_eq!(
            unambiguous_row_value(&[row.clone(), row.clone()], RowIdentity::Record),
            Some(row.record.as_str())
        );
        let mut different = row.clone();
        different.record = String::from(
            "blake3-256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert_eq!(
            unambiguous_row_value(&[row.clone(), different], RowIdentity::Record),
            None
        );
        let identity_report = TraceReport {
            rows: vec![row.clone(), row.clone()],
            ..blocked.clone()
        };
        let identities = publication_path_identities("blake3-256:basis", &identity_report);
        assert_eq!(identities.atom_revision, Some(row.atom_revision.as_str()));
        assert_eq!(identities.record_digest, Some(row.record.as_str()));

        let slot = crate::external_link_generation::render_slot(
            "LINK-1",
            "REQ-001",
            "blake3-256:atom",
            "documents",
            "https://example.com/trace",
        );
        assert_eq!(
            external_link_target(slot.as_bytes(), "LINK-1"),
            Some(String::from("https://example.com/trace"))
        );
        assert_eq!(external_link_target(b"not utf8: \xff", "LINK-1"), None);
        assert_eq!(external_link_target(b"no slot", "LINK-1"), None);
        assert_eq!(
            publication_source_changed(
                "generate",
                "page",
                "plan",
                "blake3-256:basis",
                "blake3-256:graph",
                "TPT-SOURCE-CHANGED",
                "coverage control",
                PublicationFormat::Machine,
            ),
            EXIT_STATIC
        );

        let unnamed_report = TraceReport {
            schema: String::from(TRACE_REPORT_SCHEMA),
            graph: blocked.graph.clone(),
            selector: blocked.selector.clone(),
            include_history: false,
            rows: vec![row.clone()],
            diagnostics: Vec::new(),
        };
        assert!(render_trace_human(&unnamed_report).contains("src/lib.rs"));

        let invalid_report = TraceReport {
            schema: String::from(TRACE_REPORT_SCHEMA),
            graph: blocked.graph.clone(),
            selector: blocked.selector.clone(),
            include_history: false,
            rows: vec![TraceReportRow {
                implementation_path: Some(String::from("/absolute")),
                ..row
            }],
            diagnostics: Vec::new(),
        };
        assert_eq!(
            render_trace(&invalid_report, TraceFormat::Machine),
            Err("TRC-FORBIDDEN-REPORT-FIELD")
        );
        assert_eq!(
            node_facts(
                &checked,
                SourceNodeRef::new(SemanticDigest::blake3_256([0; 32]))
            ),
            (None, None, None, None)
        );

        let result = checked.clone();
        assert_eq!(
            static_report_command(
                &config,
                result,
                Some("machine"),
                Err(crate::check::RequirementSubjectError::MissingNode),
            ),
            EXIT_INPUT
        );

        let mut unaccepted = checked.clone();
        unaccepted.diagnostics.push(ShellDiagnostic {
            code: String::from("CONTROL-STATIC-DIAGNOSTIC"),
            path: None,
            start: 0,
            end: 0,
            id: None,
            context: String::from("coverage control"),
        });
        assert!(
            static_requirement_subjects(&unaccepted)
                .expect("unaccepted static inventory")
                .is_empty()
        );
        assert_eq!(
            AcceptedBindingCheck::parse(unaccepted.clone()),
            Err(BindError::Input)
        );
        assert_eq!(
            static_report_command(&config, unaccepted, Some("machine"), Ok(Vec::new()),),
            EXIT_STATIC
        );
        assert_eq!(parse_trace_format(Some(&String::from("invalid"))), Err(()));
    }

    #[cfg(all(feature = "coverage-controls", feature = "blake3-256", unix))]
    #[test]
    fn publication_write_pipeline_controls_cover_real_local_outcomes() {
        use crate::core::DigestAlgorithm;
        use crate::publication::{ArtifactKind, Lifecycle, PublicationOutput, PublicationReport};
        use std::os::unix::fs::symlink;

        assert_eq!(
            PublicationFormat::parse(None),
            Ok(PublicationFormat::Machine)
        );
        assert_eq!(
            PublicationFormat::parse(Some(&String::from("human"))),
            Ok(PublicationFormat::Human)
        );
        assert_eq!(
            PublicationFormat::parse(Some(&String::from("invalid"))),
            Err(())
        );
        print_publication_report(
            &PublicationReport::new("check", "control"),
            PublicationFormat::Machine,
        );
        print_publication_report(
            &PublicationReport::new("check", "control"),
            PublicationFormat::Human,
        );
        with_cli_faults(PUBLICATION_REPORT_FAULT, || {
            print_publication_report(
                &PublicationReport::new("check", "control"),
                PublicationFormat::Machine,
            );
            print_publication_report(
                &PublicationReport::new("check", "control"),
                PublicationFormat::Human,
            );
            EXIT_OK
        });

        let root = TempDir::new().expect("publication write root");
        let roots = InvocationRoots::Single(root.path().to_path_buf());
        assert!(matches!(
            absent_parent(false),
            Ok(OpenParentOutcome::Absent)
        ));
        assert_eq!(
            absent_parent(true).err(),
            Some(crate::publication_fs::WriteError::Absent)
        );
        let output = |lifecycle| PublicationOutput {
            id: String::from("page"),
            mount: String::from("repository"),
            path: String::from("out/{publication_basis_path}.md"),
            kind: ArtifactKind::Page,
            lifecycle,
            relation: None,
            page: None,
            target_base: None,
        };
        let graph = "blake3-256:1111111111111111111111111111111111111111111111111111111111111111";
        let basis = "blake3-256:2222222222222222222222222222222222222222222222222222222222222222";
        let report = TraceReport {
            schema: String::from(TRACE_REPORT_SCHEMA),
            graph: String::from(graph),
            selector: TraceSelectorFact {
                kind: String::from("requirement-id"),
                value: String::from("REQ-001"),
            },
            include_history: false,
            rows: Vec::new(),
            diagnostics: Vec::new(),
        };
        assert_eq!(
            publication_page_operation(
                "generate",
                &output(Lifecycle::Immutable),
                "plan",
                basis,
                graph,
                &report,
                DigestAlgorithm::Xxh3_128,
                &roots,
                PublicationFormat::Machine,
            ),
            EXIT_INPUT
        );
        assert_eq!(
            publication_page_operation(
                "check",
                &output(Lifecycle::Replaceable),
                "plan",
                basis,
                graph,
                &report,
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Machine,
            ),
            EXIT_STATIC
        );
        let mut unavailable_identity = output(Lifecycle::Replaceable);
        unavailable_identity.path = String::from("out/{atom_revision_path}.md");
        assert_eq!(
            publication_page_operation(
                "check",
                &unavailable_identity,
                "plan",
                basis,
                graph,
                &report,
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Machine,
            ),
            EXIT_STATIC
        );

        let replaceable = output(Lifecycle::Replaceable);
        for (outcome, label) in [
            (crate::publication_fs::WriteOutcome::Created, "created"),
            (crate::publication_fs::WriteOutcome::NoOp, "no-op"),
            (crate::publication_fs::WriteOutcome::Replaced, "replaced"),
            (crate::publication_fs::WriteOutcome::Conflict, "conflict"),
        ] {
            assert_eq!(write_outcome_label(outcome), label);
        }
        let base_report = || {
            PublicationReport::new("check", "page")
                .with_plan_id("plan")
                .with_basis(basis)
                .with_graph(graph)
        };
        assert_eq!(
            with_cli_faults(DERIVED_DIGEST_FAULT, || {
                publication_derived_write_operation(
                    "check",
                    &replaceable,
                    base_report(),
                    "out/fault.md",
                    b"candidate\n",
                    DigestAlgorithm::Blake3_256,
                    &roots,
                    PublicationFormat::Machine,
                )
            }),
            EXIT_INPUT
        );
        assert_eq!(
            with_cli_faults(DERIVED_PARENT_FAULT, || {
                publication_derived_write_operation(
                    "generate",
                    &replaceable,
                    base_report(),
                    "out/fault.md",
                    b"candidate\n",
                    DigestAlgorithm::Blake3_256,
                    &roots,
                    PublicationFormat::Machine,
                )
            }),
            EXIT_INPUT
        );
        assert_eq!(
            with_cli_faults(DERIVED_LOCK_FAULT, || {
                publication_derived_write_operation(
                    "generate",
                    &replaceable,
                    base_report(),
                    "out/lock-fault.md",
                    b"candidate\n",
                    DigestAlgorithm::Blake3_256,
                    &roots,
                    PublicationFormat::Machine,
                )
            }),
            EXIT_STATIC
        );
        assert_eq!(
            with_cli_faults(DERIVED_WRITE_FAULT, || {
                publication_derived_write_operation(
                    "generate",
                    &replaceable,
                    base_report(),
                    "out/write-fault.md",
                    b"candidate\n",
                    DigestAlgorithm::Blake3_256,
                    &roots,
                    PublicationFormat::Machine,
                )
            }),
            EXIT_STATIC
        );
        assert_eq!(
            publication_derived_write_operation(
                "check",
                &replaceable,
                base_report(),
                "out/page.md",
                b"candidate\n",
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Machine,
            ),
            EXIT_STATIC
        );
        assert_eq!(
            publication_derived_write_operation(
                "generate",
                &replaceable,
                base_report(),
                "out/page.md",
                b"candidate\n",
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Human,
            ),
            EXIT_OK
        );
        assert_eq!(
            publication_derived_write_operation(
                "check",
                &replaceable,
                base_report(),
                "out/page.md",
                b"candidate\n",
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Machine,
            ),
            EXIT_OK
        );
        fs::write(root.path().join("out/page.md"), b"different\n").expect("different page");
        assert_eq!(
            publication_derived_write_operation(
                "check",
                &replaceable,
                base_report(),
                "out/page.md",
                b"candidate\n",
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Human,
            ),
            EXIT_STATIC
        );
        assert_eq!(
            publication_derived_write_operation(
                "generate",
                &replaceable,
                base_report(),
                "out/page.md",
                b"candidate\n",
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Machine,
            ),
            EXIT_OK
        );

        let immutable = output(Lifecycle::Immutable);
        assert_eq!(
            publication_derived_write_operation(
                "generate",
                &immutable,
                base_report(),
                "out/immutable.md",
                b"first\n",
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Machine,
            ),
            EXIT_OK
        );
        assert_eq!(
            publication_derived_write_operation(
                "generate",
                &immutable,
                base_report(),
                "out/immutable.md",
                b"second\n",
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Machine,
            ),
            EXIT_STATIC
        );

        let linked_root = TempDir::new().expect("linked publication root");
        symlink(linked_root.path(), root.path().join("linked")).expect("linked output parent");
        assert_eq!(
            publication_derived_write_operation(
                "generate",
                &replaceable,
                base_report(),
                "linked/page.md",
                b"candidate\n",
                DigestAlgorithm::Blake3_256,
                &roots,
                PublicationFormat::Machine,
            ),
            EXIT_STATIC
        );
    }

    /// RTP-007 (Markdown-only additions) must never change `buoy/trace-report/v1`
    /// or frozen `buoy/binding-tree-report/v1` machine bytes. This freezes the
    /// exact JSON key set both machine schemas carry.
    #[test]
    fn trace_report_and_binding_tree_report_json_keys_stay_frozen() {
        let row = TraceReportRow {
            state: String::from("current"),
            relation: String::from("TRACE-RELATION"),
            navigation: String::from("implements"),
            record: String::from("blake3-256:aaaa"),
            requirement_id: String::from("TRACE-001"),
            atom_revision: String::from("blake3-256:bbbb"),
            requirement_node: String::from("blake3-256:cccc"),
            implementation_node: String::from("blake3-256:dddd"),
            requirement_path: Some(String::from("openspec/specs/demo/spec.md")),
            implementation_path: Some(String::from("src/lib.rs")),
            requirement_kind: Some(String::from("openspec/requirement/v1")),
            implementation_kind: Some(String::from("rust/item/v1")),
            requirement_name: Some(String::from("Trace requirement")),
            implementation_name: Some(String::from("trace_item")),
            requirement_span: Some(TraceSpanFact { start: 0, end: 10 }),
            implementation_span: Some(TraceSpanFact { start: 10, end: 20 }),
            freshness: TraceFreshnessFact {
                record: true,
                attachment: true,
                implementation: true,
                requirement: true,
                authority: true,
            },
            traversal: Vec::new(),
            diagnostics: Vec::new(),
        };
        let report = TraceReport {
            schema: String::from(TRACE_REPORT_SCHEMA),
            graph: String::from("blake3-256:eeee"),
            selector: TraceSelectorFact {
                kind: String::from("relation-id"),
                value: String::from("TRACE-RELATION"),
            },
            include_history: false,
            rows: vec![row],
            diagnostics: Vec::new(),
        };
        let value = serde_json::to_value(&report).expect("trace report must serialize");
        let top_level = value
            .as_object()
            .expect("trace report is a JSON object")
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            top_level,
            [
                "schema",
                "graph",
                "selector",
                "include_history",
                "rows",
                "diagnostics"
            ]
            .into_iter()
            .map(String::from)
            .collect::<std::collections::BTreeSet<_>>(),
            "buoy/trace-report/v1 top-level keys must stay frozen"
        );
        let row_keys = value["rows"][0]
            .as_object()
            .expect("row is a JSON object")
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            row_keys,
            [
                "state",
                "relation",
                "navigation",
                "record",
                "requirement_id",
                "atom_revision",
                "requirement_node",
                "implementation_node",
                "requirement_path",
                "implementation_path",
                "requirement_kind",
                "implementation_kind",
                "requirement_name",
                "implementation_name",
                "requirement_span",
                "implementation_span",
                "freshness",
                "traversal",
                "diagnostics",
            ]
            .into_iter()
            .map(String::from)
            .collect::<std::collections::BTreeSet<_>>(),
            "buoy/trace-report/v1 row keys must stay frozen"
        );

        let binding_report = BindingTreeReport {
            schema: String::from("buoy/binding-tree-report/v1"),
            relation: String::from("TRACE-RELATION"),
            binding: String::from("blake3-256:ffff"),
            requirement_id: String::from("TRACE-001"),
            atom_revision: String::from("blake3-256:bbbb"),
            implementation_path: String::from("src/lib.rs"),
            requirement_path: String::from("openspec/specs/demo/spec.md"),
            freshness: BindingFreshness {
                record: true,
                implementation: true,
                requirement: true,
                authority: true,
            },
            nodes: Vec::new(),
        };
        let binding_value =
            serde_json::to_value(&binding_report).expect("binding tree report must serialize");
        let binding_keys = binding_value
            .as_object()
            .expect("binding tree report is a JSON object")
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            binding_keys,
            [
                "schema",
                "relation",
                "binding",
                "requirement_id",
                "atom_revision",
                "implementation_path",
                "requirement_path",
                "freshness",
                "nodes",
            ]
            .into_iter()
            .map(String::from)
            .collect::<std::collections::BTreeSet<_>>(),
            "buoy/binding-tree-report/v1 keys must stay frozen"
        );
    }

    #[cfg(all(feature = "coverage-controls", feature = "blake3-256"))]
    #[test]
    fn successful_bind_report_rejects_a_historical_subject() {
        let repository = copy_binding_repository();
        let config =
            load_config(&repository.path().join("buoy.toml")).expect("binding configuration");
        let snapshot = snapshot_repository(repository.path(), &config).expect("binding snapshot");
        let checked = check_snapshot(&snapshot, &config).expect("binding check");
        let current = checked.source.store()[0].record();
        let absent = SourceNodeRef::new(
            crate::core::semantic_digest_with(
                config.active_algorithm(),
                "historical-bind-report-subject",
                Vec::new(),
            )
            .expect("historical subject digest"),
        );
        let historical = ImplementationBinding::new_with(
            config.active_algorithm(),
            current.meta().id().clone(),
            current.navigation(),
            absent,
            current.requirement(),
            current.atom().clone(),
        )
        .expect("historical binding");
        assert!(fresh_link_facts(&checked, &historical).is_none());
        assert!(binding_tree_report(&checked, &historical).is_none());
    }
}
