//! Deterministic static and provider-neutral change command adapters.
//!
//! Both adapters preserve the core exit classes of the authoritative Buoy
//! commands: `0` success, `2` static findings, `3` change findings, `4`
//! execution findings, `64` usage, and `70` invalid input.

use crate::adapter::ChelisAdapter;
use crate::records::ChelisDialect;
use crate::snapshot::{ChelisConfiguration, ChelisSnapshot};
use buoy::adapter_sdk::{
    AdapterConfiguration, AdapterRegistry, AdapterSdkError, SavedSnapshotInput,
};
use buoy::adapters::{extract_binding_in_mount, parse_binding_record};
use buoy::core::{
    ActiveAlgorithm, ArtifactBindingKind, ArtifactEntry, ArtifactIndex, CanonicalValue,
    DigestAssuranceClass, EnforcementMode, ObjectKind, RepositoryPath, SemanticDigest, SpecGraph,
    derivation_fresh_with_artifacts, semantic_digest, semantic_digest_with, validate_graph,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Core success exit class.
pub const EXIT_OK: u8 = 0;
/// Core static-finding exit class.
pub const EXIT_STATIC: u8 = 2;
/// Core change-finding exit class.
pub const EXIT_CHANGE: u8 = 3;
/// Core execution-finding exit class.
pub const EXIT_EXECUTION: u8 = 4;
/// Core usage exit class.
pub const EXIT_USAGE: u8 = 64;
/// Core invalid-input exit class.
pub const EXIT_INPUT: u8 = 70;

/// Stable diagnostic for a stale derivation source binding.
pub const DERIVATION_SOURCE_STALE: &str = "CHELIS-PROV-DERIVATION-SOURCE-STALE";
/// Stable diagnostic for a stale derivation target binding.
pub const DERIVATION_TARGET_STALE: &str = "CHELIS-PROV-DERIVATION-TARGET-STALE";
/// Stable diagnostic for a stale derivation tool identity.
pub const DERIVATION_TOOL_STALE: &str = "CHELIS-PROV-DERIVATION-TOOL-STALE";
/// Stable diagnostic for a stale derivation configuration.
pub const DERIVATION_CONFIGURATION_STALE: &str = "CHELIS-PROV-DERIVATION-CONFIGURATION-STALE";
/// Stable diagnostic for a derivation whose bound artifact is absent.
pub const DERIVATION_STALE: &str = "CHELIS-PROV-DERIVATION-STALE";
/// Stable diagnostic for an undispositioned transitive impact.
pub const IMPACT_DISPOSITION_MISSING: &str = "CHELIS-PROV-IMPACT-DISPOSITION-MISSING";
/// Stable diagnostic for a doubly dispositioned impact.
pub const IMPACT_DISPOSITION_DUPLICATE: &str = "CHELIS-PROV-IMPACT-DISPOSITION-DUPLICATE";
/// Stable diagnostic for a reaffirmation offered as technical freshness.
pub const REAFFIRMATION_REVIEW_ONLY: &str = "CHELIS-PROV-REAFFIRMATION-REVIEW-ONLY";
/// Stable diagnostic for policy weakening without the complete protocol.
pub const POLICY_WEAKENING: &str = "CHELIS-PROV-POLICY-WEAKENING";
/// Stable diagnostic for a bound item digest that differs from the current
/// parsed declaration node bytes.
pub const ITEM_STALE: &str = "CHELIS-PROV-ITEM-STALE";
/// Stable diagnostic for recorded oracle runner digests that differ from the
/// declared runner configuration artifacts.
pub const ORACLE_RUNNER_STALE: &str = "CHELIS-PROV-ORACLE-RUNNER-STALE";
/// Stable diagnostic for a compact binding whose content address does not
/// resolve to one valid immutable store object.
pub const BINDING_ADDRESS: &str = "CHELIS-PROV-BINDING-ADDRESS";
/// Stable diagnostic for a resolved binding whose recorded subject differs
/// from the parsed bytes of its attached declaration.
pub const BINDING_SUBJECT_STALE: &str = "CHELIS-PROV-BINDING-SUBJECT-STALE";
/// Stable diagnostic for a resolved binding whose requirement differs from
/// the current exact authority record block.
pub const BINDING_REQUIREMENT_STALE: &str = "CHELIS-PROV-BINDING-REQUIREMENT-STALE";
/// Stable diagnostic for one relation ID present in both inline and compact forms.
pub const BINDING_LINK_DUPLICATE: &str = "CHELIS-PROV-BINDING-LINK-DUPLICATE";

/// Returns the active member when this build can address content.
///
/// A content address is only meaningful under a collision-resistant member,
/// and every digest in one stored binding must share the member the rest of
/// the graph uses. A change-detection build therefore resolves no compact
/// binding at all rather than resolving one weakly.
///
/// # Errors
///
/// Returns the stable reason this build cannot address content.
pub fn binding_algorithm() -> Result<ActiveAlgorithm, String> {
    let active = ActiveAlgorithm::build_default()
        .map_err(|error| format!("algorithm-unavailable:{}", error.code()))?;
    if active.algorithm().assurance_class() != DigestAssuranceClass::CollisionResistant {
        return Err(format!(
            "algorithm-not-collision-resistant:{}",
            active.algorithm().cargo_feature()
        ));
    }
    Ok(active)
}

/// One canonical report diagnostic row.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ReportDiagnostic {
    /// Stable machine code.
    pub code: String,
    /// Canonical repository path, or `-` for a graph-level finding.
    pub path: String,
    /// Inclusive byte start.
    pub start: u64,
    /// Exclusive byte end.
    pub end: u64,
    /// Schema-owned deterministic context.
    pub context: String,
}

/// Complete deterministic static outcome.
#[derive(Clone, Debug)]
pub struct StaticOutcome {
    /// Merged canonical graph.
    pub graph: SpecGraph,
    /// Canonical machine-report bytes.
    pub report: String,
    /// Sorted canonical diagnostics.
    pub diagnostics: Vec<ReportDiagnostic>,
    /// Core exit class.
    pub exit: u8,
    /// Canonical report digest naming the static graph identity.
    pub identity: SemanticDigest,
}

/// Deterministic command failure outside the report protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandError {
    /// Adapter SDK rejection.
    Adapter(AdapterSdkError),
    /// Canonical encoding failure.
    Encoding,
}

#[derive(Serialize)]
struct StaticReport<'a> {
    schema: &'static str,
    objects: BTreeMap<&'static str, Vec<ReportObject>>,
    diagnostics: &'a [ReportDiagnostic],
    verdict: &'static str,
}

/// One identity-bearing object row in the canonical static report.
///
/// Atoms additionally carry their revision digest so a host repository can
/// bind carriers, links, and surface rows from the report alone.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReportObject {
    /// Stable object ID.
    pub id: String,
    /// Canonical object identity digest.
    pub digest: String,
    /// Atom revision digest; absent for every other object class.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

fn registry() -> Result<AdapterRegistry, AdapterSdkError> {
    let mut registry = AdapterRegistry::new();
    registry.register(ChelisAdapter::new(ChelisDialect::Surf)?)?;
    registry.register(ChelisAdapter::new(ChelisDialect::Deep)?)?;
    Ok(registry)
}

fn merge(target: &mut SpecGraph, source: SpecGraph) {
    target.atoms.extend(source.atoms);
    target.tombstones.extend(source.tombstones);
    target.decision_surfaces.extend(source.decision_surfaces);
    target.evidence_carriers.extend(source.evidence_carriers);
    target
        .implementation_links
        .extend(source.implementation_links);
    target
        .implementation_bindings
        .extend(source.implementation_bindings);
    target.derivations.extend(source.derivations);
    target.coverage_policies.extend(source.coverage_policies);
    target.debt_records.extend(source.debt_records);
    target.waivers.extend(source.waivers);
    target.oracles.extend(source.oracles);
    target.finite_models.extend(source.finite_models);
    target.formal_properties.extend(source.formal_properties);
    target.assumptions.extend(source.assumptions);
    target.proof_records.extend(source.proof_records);
    target.reaffirmations.extend(source.reaffirmations);
    target
        .external_trace_links
        .extend(source.external_trace_links);
}

/// Computes the frozen raw-artifact digest for saved bytes.
///
/// # Errors
///
/// Returns an encoding failure when no digest member is available.
pub fn artifact_digest(bytes: &[u8]) -> Result<SemanticDigest, CommandError> {
    semantic_digest(
        "chelis-raw-artifact",
        vec![(1, CanonicalValue::Bytes(bytes.to_vec()))],
    )
    .map_err(|_| CommandError::Encoding)
}

/// Computes the frozen raw-artifact digest of every saved snapshot path.
///
/// # Errors
///
/// Returns an encoding failure when no digest member is available.
pub fn snapshot_digests(
    snapshot: &ChelisSnapshot,
) -> Result<BTreeMap<String, SemanticDigest>, CommandError> {
    let mut file_digests = BTreeMap::new();
    for (path, _, bytes) in snapshot.files() {
        file_digests.insert(path.as_str().to_owned(), artifact_digest(bytes)?);
    }
    for (path, bytes) in snapshot.artifacts() {
        file_digests.insert(path.to_owned(), artifact_digest(bytes)?);
    }
    Ok(file_digests)
}

/// Computes one raw-artifact digest under an explicit active member.
///
/// Compact bindings are content addressed, so their digests must come from
/// the member the address itself names rather than the build default.
///
/// # Errors
///
/// Returns an encoding failure when the payload violates canonical rules.
pub fn binding_digest(
    active: ActiveAlgorithm,
    bytes: &[u8],
) -> Result<SemanticDigest, CommandError> {
    semantic_digest_with(
        active,
        "chelis-raw-artifact",
        vec![(1, CanonicalValue::Bytes(bytes.to_vec()))],
    )
    .map_err(|_| CommandError::Encoding)
}

/// Returns the store-relative object path of one binding address.
///
/// The layout is the frozen upstream one, so a Chelis store and a Rust store
/// hold byte-identical objects at identical relative paths.
#[must_use]
pub fn binding_object_repository_path(store: &str, address: SemanticDigest) -> String {
    format!(
        "{store}/{}",
        buoy::adapters::binding_object_path(buoy::core::ImplementationBindingRef::new(address))
    )
}

/// One resolved compact binding with the location of its in-source marker.
pub struct ResolvedBinding {
    /// Materialized immutable record.
    pub record: buoy::core::ImplementationBinding,
    /// Canonical path of the source file carrying the marker.
    pub path: String,
    /// Store-relative path of the resolved object.
    pub object_path: String,
    /// Exact bytes of the attached declaration node.
    pub declaration: String,
    /// Inclusive byte start of the attached declaration node.
    pub declaration_start: u64,
    /// Exclusive byte end of the attached declaration node.
    pub declaration_end: u64,
}

/// Resolves every compact binding marker in one saved snapshot.
///
/// Resolution is total: each marker either materializes exactly one immutable
/// record or produces exactly one deterministic diagnostic. Rejected bytes
/// never reach the graph, so no unverified object can supply navigation.
#[allow(clippy::too_many_lines)]
fn binding_resolution(
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
) -> (Vec<ResolvedBinding>, Vec<ReportDiagnostic>) {
    let mut resolved = Vec::new();
    let mut diagnostics = Vec::new();
    for (path, dialect, bytes) in snapshot.files() {
        if dialect != ChelisDialect::Surf {
            continue;
        }
        let Ok(extraction) = crate::records::extract(dialect, bytes) else {
            continue;
        };
        for record in &extraction.records {
            if record.kind != crate::records::BINDING_RECORD_KIND {
                continue;
            }
            let mut reject = |context: String| {
                diagnostics.push(ReportDiagnostic {
                    code: BINDING_ADDRESS.to_owned(),
                    path: path.as_str().to_owned(),
                    start: record.start,
                    end: record.end,
                    context,
                });
            };
            let Some(address) = record
                .fields
                .get(crate::records::BINDING_ADDRESS_FIELD)
                .and_then(|value| SemanticDigest::parse_canonical(value).ok())
            else {
                reject(String::from("invalid-address"));
                continue;
            };
            let active = match binding_algorithm() {
                Ok(active) => active,
                Err(reason) => {
                    reject(reason);
                    continue;
                }
            };
            if address.algorithm() != active.algorithm() {
                reject(format!(
                    "address-member-mismatch;bound={};current={}",
                    address.algorithm().cargo_feature(),
                    active.algorithm().cargo_feature()
                ));
                continue;
            }
            let Some(store_prefix) = configuration.binding_store.as_ref() else {
                reject(String::from("store-unconfigured"));
                continue;
            };
            let object_path = binding_object_repository_path(store_prefix.as_str(), address);
            let Some(object_bytes) = snapshot.artifact(&object_path) else {
                reject(format!("missing-object:{object_path}"));
                continue;
            };
            let Ok(canonical_object) = RepositoryPath::new(&object_path) else {
                reject(format!("invalid-object-path:{object_path}"));
                continue;
            };
            let output = extract_binding_in_mount(
                active,
                Some(store_prefix),
                &canonical_object,
                object_bytes,
            );
            let Some(stored) = output.stored else {
                let code = output
                    .diagnostics
                    .first()
                    .map_or("CAB-OBJECT-UNKNOWN", |diagnostic| diagnostic.code);
                // The store refuses a substituted object because its path no
                // longer matches its content, so the address the bytes now
                // hash to is exactly the fact a reviewer needs. Bytes that
                // parse as no record at all can name no such address.
                let current = parse_binding_record(active, object_bytes).map_or_else(
                    || String::from("unavailable"),
                    |record| record.record().canonical_text(),
                );
                reject(format!(
                    "object:{code}:{object_path};bound={};current={current}",
                    address.canonical_text()
                ));
                continue;
            };
            resolved.push(ResolvedBinding {
                record: stored.record().clone(),
                path: path.as_str().to_owned(),
                object_path,
                declaration: record.declaration_text.clone(),
                declaration_start: record.declaration_start,
                declaration_end: record.declaration_end,
            });
        }
    }
    (resolved, diagnostics)
}

/// Compares each resolved binding subject with its attached declaration.
fn binding_subject_diagnostics(
    resolved: &[ResolvedBinding],
) -> Result<Vec<ReportDiagnostic>, CommandError> {
    let mut diagnostics = Vec::new();
    for binding in resolved {
        let bound = binding.record.subject().digest();
        let Ok(active) = binding_algorithm() else {
            continue;
        };
        let current = binding_digest(active, binding.declaration.as_bytes())?;
        if current != bound {
            diagnostics.push(ReportDiagnostic {
                code: BINDING_SUBJECT_STALE.to_owned(),
                path: binding.path.clone(),
                start: binding.declaration_start,
                end: binding.declaration_end,
                context: format!(
                    "binding:{};bound={};current={}",
                    binding.record.meta().id().as_str(),
                    bound.canonical_text(),
                    current.canonical_text()
                ),
            });
        }
    }
    Ok(diagnostics)
}

fn binding_requirement_diagnostics(
    snapshot: &ChelisSnapshot,
    resolved: &[ResolvedBinding],
) -> Result<Vec<ReportDiagnostic>, CommandError> {
    let mut diagnostics = Vec::new();
    for binding in resolved {
        let bound = binding.record.requirement().digest();
        let authority = crate::bind::authority_block(snapshot, binding.record.atom().id().as_str());
        let current = match &authority {
            Some(authority) => {
                let active = binding_algorithm().map_err(|_| CommandError::Encoding)?;
                binding_digest(active, authority.bytes.as_bytes())?.canonical_text()
            }
            None => String::from("absent"),
        };
        if current == bound.canonical_text() {
            continue;
        }
        let (path, start, end) = authority.map_or_else(
            || {
                (
                    binding.path.clone(),
                    binding.declaration_start,
                    binding.declaration_end,
                )
            },
            |authority| (authority.path, authority.start, authority.end),
        );
        diagnostics.push(ReportDiagnostic {
            code: BINDING_REQUIREMENT_STALE.to_owned(),
            path,
            start,
            end,
            context: format!(
                "binding:{};bound={};current={current}",
                binding.record.meta().id().as_str(),
                bound.canonical_text()
            ),
        });
    }
    Ok(diagnostics)
}

fn binding_link_duplicate_diagnostics(graph: &SpecGraph) -> Vec<ReportDiagnostic> {
    let inline = graph
        .implementation_links
        .iter()
        .map(|link| link.meta().id().as_str())
        .collect::<BTreeSet<_>>();
    graph
        .implementation_bindings
        .iter()
        .filter(|binding| inline.contains(binding.meta().id().as_str()))
        .map(|binding| ReportDiagnostic {
            code: BINDING_LINK_DUPLICATE.to_owned(),
            path: String::from("-"),
            start: 0,
            end: 0,
            context: format!("relation:{}", binding.meta().id().as_str()),
        })
        .collect()
}

fn oracle_runner_diagnostics(
    snapshot: &ChelisSnapshot,
    file_digests: &BTreeMap<String, SemanticDigest>,
) -> Vec<ReportDiagnostic> {
    let mut diagnostics = Vec::new();
    for (path, dialect, bytes) in snapshot.files() {
        let Ok(extraction) = crate::records::extract(dialect, bytes) else {
            continue;
        };
        for record in &extraction.records {
            if record.kind != "oracle" {
                continue;
            }
            let Some(paths_field) = record.fields.get("runner-config-paths") else {
                continue;
            };
            let declared = paths_field
                .split(';')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .collect::<Vec<_>>();
            let id = record.fields.get("id").cloned().unwrap_or_default();
            if declared.is_empty() || declared.len() > 2 {
                diagnostics.push(ReportDiagnostic {
                    code: crate::records::MALFORMED_RECORD.to_owned(),
                    path: path.as_str().to_owned(),
                    start: record.start,
                    end: record.end,
                    context: format!("invalid-runner-config-paths:{id}"),
                });
                continue;
            }
            for (index, artifact_path) in declared.iter().enumerate() {
                let field = if index == 0 {
                    "runner-digest"
                } else {
                    "runner-configuration-digest"
                };
                let bound = record.fields.get(field).cloned().unwrap_or_default();
                let current = file_digests
                    .get(*artifact_path)
                    .map(SemanticDigest::canonical_text);
                if current.as_deref() != Some(bound.as_str()) {
                    diagnostics.push(ReportDiagnostic {
                        code: ORACLE_RUNNER_STALE.to_owned(),
                        path: path.as_str().to_owned(),
                        start: record.start,
                        end: record.end,
                        context: format!(
                            "oracle:{id}:{field}:{artifact_path};bound={bound};current={}",
                            current.unwrap_or_else(|| String::from("absent"))
                        ),
                    });
                }
            }
        }
    }
    diagnostics
}

fn atom_mismatch_parts(
    atoms: &BTreeMap<String, SemanticDigest>,
    references: &[buoy::core::AtomRevisionRef],
    parts: &mut Vec<String>,
) {
    for reference in references {
        let bound = reference.revision().canonical_text();
        match atoms.get(reference.id().as_str()) {
            Some(revision) if revision.canonical_text() == bound => {}
            Some(revision) => parts.push(format!(
                "atom={};bound={bound};current={}",
                reference.id().as_str(),
                revision.canonical_text()
            )),
            None => parts.push(format!(
                "atom={};bound={bound};current=absent",
                reference.id().as_str()
            )),
        }
    }
}

#[allow(clippy::too_many_lines)]
fn enrich_staleness(
    graph: &SpecGraph,
    file_digests: &BTreeMap<String, SemanticDigest>,
    diagnostics: &mut [ReportDiagnostic],
) {
    let atoms = graph
        .atoms
        .iter()
        .map(|atom| (atom.meta().id().as_str().to_owned(), atom.revision()))
        .collect::<BTreeMap<_, _>>();
    let oracles = graph
        .oracles
        .iter()
        .map(|oracle| {
            (
                oracle.meta().id().as_str().to_owned(),
                oracle.meta().digest(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for diagnostic in diagnostics.iter_mut() {
        let enrichable = matches!(
            diagnostic.code.as_str(),
            "SPG-REF-DANGLING-OR-STALE" | "EVD-FRESH-CARRIER" | "SPG-DERIVATION-STALE"
        );
        if !enrichable || diagnostic.context.contains(";bound=") {
            continue;
        }
        let id = diagnostic.context.clone();
        let mut parts = Vec::new();
        for carrier in &graph.evidence_carriers {
            if carrier.meta().id().as_str() != id {
                continue;
            }
            atom_mismatch_parts(&atoms, carrier.atoms(), &mut parts);
            let bound = carrier.oracle().digest().canonical_text();
            match oracles.get(carrier.oracle().id().as_str()) {
                Some(digest) if digest.canonical_text() == bound => {}
                Some(digest) => parts.push(format!(
                    "oracle={};bound={bound};current={}",
                    carrier.oracle().id().as_str(),
                    digest.canonical_text()
                )),
                None => parts.push(format!(
                    "oracle={};bound={bound};current=absent",
                    carrier.oracle().id().as_str()
                )),
            }
        }
        for link in &graph.implementation_links {
            if link.meta().id().as_str() == id {
                atom_mismatch_parts(&atoms, link.atoms(), &mut parts);
            }
        }
        for binding in &graph.implementation_bindings {
            if binding.meta().id().as_str() == id {
                atom_mismatch_parts(&atoms, core::slice::from_ref(binding.atom()), &mut parts);
            }
        }
        for property in &graph.formal_properties {
            if property.meta().id().as_str() == id {
                atom_mismatch_parts(&atoms, property.atoms(), &mut parts);
            }
        }
        for surface in &graph.decision_surfaces {
            if surface.meta().id().as_str() != id {
                continue;
            }
            for row in surface.dispositions() {
                if let Some(reference) = row.authority() {
                    atom_mismatch_parts(&atoms, core::slice::from_ref(reference), &mut parts);
                }
            }
        }
        for derivation in &graph.derivations {
            if derivation.meta().id().as_str() != id {
                continue;
            }
            for binding in derivation.sources().iter().chain(derivation.targets()) {
                let Some(artifact) = binding.repository_artifact() else {
                    continue;
                };
                let bound = artifact.digest().canonical_text();
                match file_digests.get(artifact.path().as_str()) {
                    Some(current) if current.canonical_text() == bound => {}
                    Some(current) => parts.push(format!(
                        "artifact={};bound={bound};current={}",
                        artifact.path().as_str(),
                        current.canonical_text()
                    )),
                    None => parts.push(format!(
                        "artifact={};bound={bound};current=absent",
                        artifact.path().as_str()
                    )),
                }
            }
        }
        if !parts.is_empty() {
            parts.sort();
            parts.dedup();
            diagnostic.context = format!("{id};{}", parts.join(";"));
        }
    }
}

fn derivation_diagnostics(
    graph: &SpecGraph,
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
) -> Result<Vec<ReportDiagnostic>, CommandError> {
    let file_digests = snapshot_digests(snapshot)?;
    let tools = configured_tools(configuration);
    let mut diagnostics = Vec::new();
    for derivation in &graph.derivations {
        let id = derivation.meta().id().as_str().to_owned();
        for (bindings, code) in [
            (derivation.sources(), DERIVATION_SOURCE_STALE),
            (derivation.targets(), DERIVATION_TARGET_STALE),
        ] {
            for binding in bindings {
                if binding.kind() != ArtifactBindingKind::RepositoryArtifact {
                    continue;
                }
                let Some(artifact) = binding.repository_artifact() else {
                    continue;
                };
                let path = artifact.path().as_str().to_owned();
                match file_digests.get(&path) {
                    None => diagnostics.push(ReportDiagnostic {
                        code: DERIVATION_STALE.to_owned(),
                        path,
                        start: 0,
                        end: 0,
                        context: format!(
                            "derivation:{id}:missing-artifact;bound={};current=absent",
                            artifact.digest().canonical_text()
                        ),
                    }),
                    Some(current) if *current != artifact.digest() => {
                        diagnostics.push(ReportDiagnostic {
                            code: code.to_owned(),
                            path,
                            start: 0,
                            end: 0,
                            context: format!(
                                "derivation:{id};bound={};current={}",
                                artifact.digest().canonical_text(),
                                current.canonical_text()
                            ),
                        });
                    }
                    Some(_) => {}
                }
            }
        }
        let tool = derivation.tool();
        if let Some(expected) = tools.get(tool.id()) {
            if expected.0 != tool.version() || expected.1 != tool.executable() {
                diagnostics.push(ReportDiagnostic {
                    code: DERIVATION_TOOL_STALE.to_owned(),
                    path: String::from("-"),
                    start: 0,
                    end: 0,
                    context: format!(
                        "derivation:{id}:tool:{};bound={}@{};current={}@{}",
                        tool.id(),
                        tool.version(),
                        tool.executable().canonical_text(),
                        expected.0,
                        expected.1.canonical_text()
                    ),
                });
            } else if expected.2 != tool.configuration() {
                diagnostics.push(ReportDiagnostic {
                    code: DERIVATION_CONFIGURATION_STALE.to_owned(),
                    path: String::from("-"),
                    start: 0,
                    end: 0,
                    context: format!(
                        "derivation:{id}:tool:{};bound={};current={}",
                        tool.id(),
                        tool.configuration().canonical_text(),
                        expected.2.canonical_text()
                    ),
                });
            }
        }
    }
    Ok(diagnostics)
}

fn current_artifact_index(
    graph: &SpecGraph,
    snapshot: &ChelisSnapshot,
) -> Result<ArtifactIndex, CommandError> {
    let mut bytes_by_path = BTreeMap::new();
    for (path, _, bytes) in snapshot.files() {
        bytes_by_path.insert(path.as_str().to_owned(), bytes);
    }
    for (path, bytes) in snapshot.artifacts() {
        bytes_by_path.insert(path.to_owned(), bytes);
    }
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for derivation in &graph.derivations {
        for binding in derivation.sources().iter().chain(derivation.targets()) {
            let Some(artifact) = binding.repository_artifact() else {
                continue;
            };
            let Some(bytes) = bytes_by_path.get(artifact.path().as_str()) else {
                continue;
            };
            let key = (
                artifact.path().as_str().to_owned(),
                artifact.media_type().to_owned(),
                artifact.representation().to_owned(),
            );
            if !seen.insert(key) {
                continue;
            }
            let entry = ArtifactEntry::new(
                artifact.path().clone(),
                artifact.raw_domain(),
                artifact.media_type(),
                artifact.representation(),
                artifact_digest(bytes)?,
            )
            .map_err(|_| CommandError::Encoding)?;
            entries.push(entry);
        }
    }
    ArtifactIndex::new(entries).map_err(|_| CommandError::Encoding)
}

fn configured_tools(
    configuration: &ChelisConfiguration,
) -> BTreeMap<String, (String, SemanticDigest, SemanticDigest)> {
    #[derive(Deserialize)]
    struct RawTools {
        #[serde(default)]
        tool: Vec<RawTool>,
    }
    #[derive(Deserialize)]
    struct RawTool {
        id: String,
        version: String,
        digest: String,
        #[serde(rename = "configuration-digest")]
        configuration_digest: String,
    }
    let Ok(text) = core::str::from_utf8(&configuration.bytes) else {
        return BTreeMap::new();
    };
    let Ok(raw) = toml::from_str::<RawTools>(text) else {
        return BTreeMap::new();
    };
    raw.tool
        .into_iter()
        .filter_map(|tool| {
            let digest = SemanticDigest::parse_canonical(&tool.digest).ok()?;
            let configuration_digest =
                SemanticDigest::parse_canonical(&tool.configuration_digest).ok()?;
            Some((tool.id, (tool.version, digest, configuration_digest)))
        })
        .collect()
}

/// Runs the deterministic static check over one explicit snapshot.
///
/// # Errors
///
/// Returns an adapter or encoding failure outside the report protocol.
#[allow(clippy::too_many_lines)]
pub fn static_check(
    snapshot: &ChelisSnapshot,
    configuration: &ChelisConfiguration,
) -> Result<StaticOutcome, CommandError> {
    let registry = registry().map_err(CommandError::Adapter)?;
    let mut graph = SpecGraph::default();
    let mut diagnostics = Vec::new();
    let mut current_items: BTreeMap<(ObjectKind, String), (SemanticDigest, String, u64, u64)> =
        BTreeMap::new();
    for (path, dialect, bytes) in snapshot.files() {
        let adapter_configuration = AdapterConfiguration::new(
            dialect.adapter_id(),
            crate::adapter::CONFIGURATION_SCHEMA_ID,
            configuration.bytes.clone(),
        )
        .map_err(CommandError::Adapter)?;
        let input = SavedSnapshotInput::new(path, bytes, &adapter_configuration)
            .map_err(CommandError::Adapter)?;
        let output = registry
            .materialize(&input)
            .map_err(CommandError::Adapter)?;
        let product = output.product();
        for source in product.sources() {
            let object = source.object();
            current_items.insert(
                (object.kind(), object.id().as_str().to_owned()),
                (
                    artifact_digest(source.item().semantic_bytes())?,
                    source.span().path.as_str().to_owned(),
                    source.span().start,
                    source.span().end,
                ),
            );
        }
        for diagnostic in product.diagnostics() {
            diagnostics.push(ReportDiagnostic {
                code: diagnostic.code.to_owned(),
                path: diagnostic.span.path.as_str().to_owned(),
                start: diagnostic.span.start,
                end: diagnostic.span.end,
                context: diagnostic.context.clone(),
            });
        }
        merge(&mut graph, product.graph().clone());
    }
    let (resolved_bindings, mut binding_diagnostics) = binding_resolution(snapshot, configuration);
    binding_diagnostics.extend(binding_subject_diagnostics(&resolved_bindings)?);
    binding_diagnostics.extend(binding_requirement_diagnostics(
        snapshot,
        &resolved_bindings,
    )?);
    for binding in &resolved_bindings {
        graph.implementation_bindings.push(binding.record.clone());
    }
    binding_diagnostics.extend(binding_link_duplicate_diagnostics(&graph));
    diagnostics.extend(binding_diagnostics);
    let artifact_index = current_artifact_index(&graph, snapshot)?;
    for diagnostic in validate_graph(&graph) {
        if diagnostic.code == "SPG-DERIVATION-STALE" {
            continue;
        }
        diagnostics.push(ReportDiagnostic {
            code: diagnostic.code,
            path: diagnostic
                .path
                .map_or_else(|| String::from("-"), |path| path.as_str().to_owned()),
            start: diagnostic.start,
            end: diagnostic.end,
            context: diagnostic.id.unwrap_or_default(),
        });
    }
    for derivation in &graph.derivations {
        if !derivation_fresh_with_artifacts(&graph, derivation, &artifact_index) {
            diagnostics.push(ReportDiagnostic {
                code: String::from("SPG-DERIVATION-STALE"),
                path: String::from("-"),
                start: 0,
                end: 0,
                context: derivation.meta().id().as_str().to_owned(),
            });
        }
    }
    diagnostics.extend(derivation_diagnostics(&graph, snapshot, configuration)?);
    let file_digests = snapshot_digests(snapshot)?;
    diagnostics.extend(oracle_runner_diagnostics(snapshot, &file_digests));
    for carrier in &graph.evidence_carriers {
        let id = carrier.meta().id().as_str().to_owned();
        if let Some((current, path, start, end)) =
            current_items.get(&(ObjectKind::EvidenceCarrier, id.clone()))
        {
            if *current != carrier.parsed_item_digest() {
                diagnostics.push(ReportDiagnostic {
                    code: ITEM_STALE.to_owned(),
                    path: path.clone(),
                    start: *start,
                    end: *end,
                    context: format!(
                        "carrier:{id};bound={};current={}",
                        carrier.parsed_item_digest().canonical_text(),
                        current.canonical_text()
                    ),
                });
            }
        }
    }
    for link in &graph.implementation_links {
        let id = link.meta().id().as_str().to_owned();
        if let Some((current, path, start, end)) =
            current_items.get(&(ObjectKind::ImplementationLink, id.clone()))
        {
            if *current != link.parsed_item_digest() {
                diagnostics.push(ReportDiagnostic {
                    code: ITEM_STALE.to_owned(),
                    path: path.clone(),
                    start: *start,
                    end: *end,
                    context: format!(
                        "link:{id};bound={};current={}",
                        link.parsed_item_digest().canonical_text(),
                        current.canonical_text()
                    ),
                });
            }
        }
    }
    enrich_staleness(&graph, &file_digests, &mut diagnostics);
    diagnostics.sort();
    diagnostics.dedup();
    let mut objects = BTreeMap::new();
    let collect = |rows: Vec<ReportObject>| {
        let mut rows = rows;
        rows.sort_by(|left, right| left.id.cmp(&right.id));
        rows
    };
    let row = |meta: &buoy::core::ObjectMeta| ReportObject {
        id: meta.id().as_str().to_owned(),
        digest: meta.digest().canonical_text(),
        revision: None,
    };
    objects.insert(
        "atoms",
        collect(
            graph
                .atoms
                .iter()
                .map(|value| ReportObject {
                    id: value.meta().id().as_str().to_owned(),
                    digest: value.meta().digest().canonical_text(),
                    revision: Some(value.revision().canonical_text()),
                })
                .collect(),
        ),
    );
    objects.insert(
        "carriers",
        collect(
            graph
                .evidence_carriers
                .iter()
                .map(|value| row(value.meta()))
                .collect(),
        ),
    );
    objects.insert(
        "links",
        collect(
            graph
                .implementation_links
                .iter()
                .map(|value| row(value.meta()))
                .collect(),
        ),
    );
    objects.insert(
        "surfaces",
        collect(
            graph
                .decision_surfaces
                .iter()
                .map(|value| row(value.meta()))
                .collect(),
        ),
    );
    objects.insert(
        "policies",
        collect(
            graph
                .coverage_policies
                .iter()
                .map(|value| row(value.meta()))
                .collect(),
        ),
    );
    objects.insert(
        "derivations",
        collect(
            graph
                .derivations
                .iter()
                .map(|value| row(value.meta()))
                .collect(),
        ),
    );
    objects.insert(
        "oracles",
        collect(
            graph
                .oracles
                .iter()
                .map(|value| row(value.meta()))
                .collect(),
        ),
    );
    objects.insert(
        "bindings",
        collect(
            graph
                .implementation_bindings
                .iter()
                .map(|value| row(value.meta()))
                .collect(),
        ),
    );
    let verdict = if diagnostics.is_empty() {
        "accept"
    } else {
        "reject"
    };
    let report = serde_json::to_string(&StaticReport {
        schema: "chelis-static-report/v3",
        objects,
        diagnostics: &diagnostics,
        verdict,
    })
    .map_err(|_| CommandError::Encoding)?;
    let identity = semantic_digest(
        "chelis-static-report",
        vec![(1, CanonicalValue::Bytes(report.clone().into_bytes()))],
    )
    .map_err(|_| CommandError::Encoding)?;
    let exit = if diagnostics.is_empty() {
        EXIT_OK
    } else {
        EXIT_STATIC
    };
    Ok(StaticOutcome {
        graph,
        report,
        diagnostics,
        exit,
        identity,
    })
}

/// Renders the deterministic human projection of one static outcome.
///
/// The projection derives only from canonical report values and stays
/// byte-identical for identical canonical inputs.
#[must_use]
#[allow(clippy::format_push_string)]
pub fn render_human_static(outcome: &StaticOutcome) -> String {
    let mut text = String::from("chelis static report\n");
    text.push_str(&format!(
        "verdict: {}\n",
        if outcome.diagnostics.is_empty() {
            "accept"
        } else {
            "reject"
        }
    ));
    text.push_str(&format!(
        "objects: atoms={} bindings={} carriers={} links={} surfaces={} policies={} derivations={} oracles={}\n",
        outcome.graph.atoms.len(),
        outcome.graph.implementation_bindings.len(),
        outcome.graph.evidence_carriers.len(),
        outcome.graph.implementation_links.len(),
        outcome.graph.decision_surfaces.len(),
        outcome.graph.coverage_policies.len(),
        outcome.graph.derivations.len(),
        outcome.graph.oracles.len(),
    ));
    for diagnostic in &outcome.diagnostics {
        text.push_str(&format!(
            "finding: {} {}:{}..{} {}\n",
            diagnostic.code, diagnostic.path, diagnostic.start, diagnostic.end, diagnostic.context
        ));
    }
    text
}

/// One neutral change disposition row.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ManifestDisposition {
    /// Impacted object kind name.
    #[serde(rename = "object-kind")]
    pub object_kind: String,
    /// Impacted stable object ID.
    #[serde(rename = "object-id")]
    pub object_id: String,
    /// Exact disposition kind.
    pub kind: String,
    /// Waiver ID required for `waived` dispositions.
    #[serde(default, rename = "waiver-id")]
    pub waiver_id: Option<String>,
    /// Reaffirmation ID required for `reaffirmed` dispositions.
    #[serde(default, rename = "reaffirmation-id")]
    pub reaffirmation_id: Option<String>,
}

/// The repository-owned weakening protocol block.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WeakeningProtocol {
    /// Repository-owned migration record path.
    #[serde(default, rename = "migration-record")]
    pub migration_record: Option<String>,
    /// Repository-owned approval record path.
    #[serde(default, rename = "approval-record")]
    pub approval_record: Option<String>,
    /// Visible debt record ID.
    #[serde(default, rename = "debt-id")]
    pub debt_id: Option<String>,
    /// Applicable process waiver ID.
    #[serde(default, rename = "waiver-id")]
    pub waiver_id: Option<String>,
}

/// One committed provider-neutral change manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChangeManifest {
    /// Manifest schema identity.
    pub schema: String,
    /// Exact dispositions.
    #[serde(default)]
    pub disposition: Vec<ManifestDisposition>,
    /// Optional complete weakening protocol.
    #[serde(default, rename = "weakening-protocol")]
    pub weakening_protocol: Option<WeakeningProtocol>,
}

impl ChangeManifest {
    /// Parses one committed neutral manifest.
    ///
    /// # Errors
    ///
    /// Rejects malformed or provider-bearing manifests.
    pub fn parse(bytes: &[u8]) -> Result<Self, CommandError> {
        let text = core::str::from_utf8(bytes).map_err(|_| CommandError::Encoding)?;
        let manifest = toml::from_str::<Self>(text).map_err(|_| CommandError::Encoding)?;
        if manifest.schema != "chelis-change-manifest/v1" {
            return Err(CommandError::Encoding);
        }
        Ok(manifest)
    }
}

/// Complete deterministic change outcome.
#[derive(Clone, Debug)]
pub struct ChangeOutcome {
    /// Canonical machine-report bytes.
    pub report: String,
    /// Sorted canonical diagnostics.
    pub diagnostics: Vec<ReportDiagnostic>,
    /// Accepted policy weakenings with a complete protocol.
    pub accepted_weakenings: Vec<String>,
    /// Core exit class.
    pub exit: u8,
}

#[derive(Serialize)]
struct ChangeReport<'a> {
    schema: &'static str,
    impacted: Vec<String>,
    diagnostics: &'a [ReportDiagnostic],
    accepted_weakenings: &'a [String],
    verdict: &'static str,
}

fn kind_name(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Atom => "atom",
        ObjectKind::EvidenceCarrier => "evidence-carrier",
        ObjectKind::DecisionSurface => "decision-surface",
        ObjectKind::ImplementationLink => "implementation-link",
        ObjectKind::ImplementationBinding => "implementation-binding",
        ObjectKind::CoveragePolicy => "coverage-policy",
        ObjectKind::Derivation => "derivation",
        ObjectKind::DebtRecord => "debt-record",
        ObjectKind::Waiver => "waiver",
        ObjectKind::Oracle => "oracle",
        ObjectKind::FormalProperty => "formal-property",
        ObjectKind::Reaffirmation => "reaffirmation",
        _ => "other",
    }
}

fn graph_metas(graph: &SpecGraph) -> Vec<&buoy::core::ObjectMeta> {
    let mut metas = Vec::new();
    metas.extend(graph.atoms.iter().map(buoy::core::Atom::meta));
    metas.extend(
        graph
            .evidence_carriers
            .iter()
            .map(buoy::core::EvidenceCarrier::meta),
    );
    metas.extend(
        graph
            .implementation_links
            .iter()
            .map(buoy::core::ImplementationLink::meta),
    );
    metas.extend(
        graph
            .implementation_bindings
            .iter()
            .map(buoy::core::ImplementationBinding::meta),
    );
    metas.extend(
        graph
            .decision_surfaces
            .iter()
            .map(buoy::core::DecisionSurface::meta),
    );
    metas.extend(
        graph
            .coverage_policies
            .iter()
            .map(buoy::core::CoveragePolicy::meta),
    );
    metas.extend(graph.derivations.iter().map(buoy::core::Derivation::meta));
    metas.extend(graph.debt_records.iter().map(buoy::core::DebtRecord::meta));
    metas.extend(graph.waivers.iter().map(buoy::core::Waiver::meta));
    metas.extend(
        graph
            .formal_properties
            .iter()
            .map(buoy::core::FormalProperty::meta),
    );
    metas.extend(
        graph
            .reaffirmations
            .iter()
            .map(buoy::core::Reaffirmation::meta),
    );
    metas
}

fn impacted_set(base: &SpecGraph, head: &SpecGraph) -> BTreeSet<(String, String)> {
    let base_revisions = base
        .atoms
        .iter()
        .map(|atom| (atom.meta().id().as_str().to_owned(), atom.revision()))
        .collect::<BTreeMap<_, _>>();
    let head_revisions = head
        .atoms
        .iter()
        .map(|atom| (atom.meta().id().as_str().to_owned(), atom.revision()))
        .collect::<BTreeMap<_, _>>();
    let mut impacted = BTreeSet::new();
    let mut pending = VecDeque::new();
    for (id, base_revision) in &base_revisions {
        let changed = head_revisions
            .get(id)
            .is_none_or(|head_revision| head_revision != base_revision);
        if changed && impacted.insert((String::from("atom"), id.clone())) {
            pending.push_back(id.clone());
        }
    }
    let mut atom_dependents = BTreeMap::<String, Vec<(String, String)>>::new();
    for carrier in &head.evidence_carriers {
        for atom in carrier.atoms() {
            atom_dependents
                .entry(atom.id().as_str().to_owned())
                .or_default()
                .push((
                    String::from("evidence-carrier"),
                    carrier.meta().id().as_str().to_owned(),
                ));
        }
    }
    for link in &head.implementation_links {
        for atom in link.atoms() {
            atom_dependents
                .entry(atom.id().as_str().to_owned())
                .or_default()
                .push((
                    String::from("implementation-link"),
                    link.meta().id().as_str().to_owned(),
                ));
        }
    }
    // A compact binding carries the same atom dependency an inline link
    // carries, so converting one must not drop the relation out of the impact
    // closure and silently retire its review obligation.
    for binding in &head.implementation_bindings {
        atom_dependents
            .entry(binding.atom().id().as_str().to_owned())
            .or_default()
            .push((
                String::from("implementation-binding"),
                binding.meta().id().as_str().to_owned(),
            ));
    }
    for property in &head.formal_properties {
        for atom in property.atoms() {
            atom_dependents
                .entry(atom.id().as_str().to_owned())
                .or_default()
                .push((
                    String::from("formal-property"),
                    property.meta().id().as_str().to_owned(),
                ));
        }
    }
    let mut id_dependents = BTreeMap::<String, Vec<(String, String)>>::new();
    for meta in graph_metas(head) {
        for reference in meta.references() {
            id_dependents
                .entry(reference.id().as_str().to_owned())
                .or_default()
                .push((
                    kind_name(meta.kind()).to_owned(),
                    meta.id().as_str().to_owned(),
                ));
        }
    }
    while let Some(current) = pending.pop_front() {
        let dependents = atom_dependents
            .get(&current)
            .into_iter()
            .flatten()
            .chain(id_dependents.get(&current).into_iter().flatten());
        for dependent in dependents {
            if impacted.insert(dependent.clone()) {
                pending.push_back(dependent.1.clone());
            }
        }
    }
    impacted
}

fn carrier_is_fresh(head: &SpecGraph, carrier_id: &str) -> bool {
    let head_revisions = head
        .atoms
        .iter()
        .map(|atom| (atom.meta().id().as_str().to_owned(), atom.revision()))
        .collect::<BTreeMap<_, _>>();
    head.evidence_carriers
        .iter()
        .filter(|carrier| carrier.meta().id().as_str() == carrier_id)
        .all(|carrier| {
            carrier.atoms().iter().all(|atom| {
                head_revisions
                    .get(atom.id().as_str())
                    .is_some_and(|revision| *revision == atom.revision())
            })
        })
}

fn ratchet_diagnostics(
    base: &SpecGraph,
    head: &SpecGraph,
    manifest: &ChangeManifest,
    accepted: &mut Vec<String>,
) -> Vec<ReportDiagnostic> {
    let mut diagnostics = Vec::new();
    for base_policy in &base.coverage_policies {
        let ratchet = base_policy.revision_value().ratchet_id().to_owned();
        let Some(head_policy) = head
            .coverage_policies
            .iter()
            .find(|policy| policy.revision_value().ratchet_id() == ratchet)
        else {
            diagnostics.push(weakening(&ratchet, "policy-removed"));
            continue;
        };
        let base_revision = base_policy.revision_value();
        let head_revision = head_policy.revision_value();
        let base_roles = base_revision
            .required_roles()
            .iter()
            .collect::<BTreeSet<_>>();
        let head_roles = head_revision
            .required_roles()
            .iter()
            .collect::<BTreeSet<_>>();
        let mut weakened = Vec::new();
        if !base_roles.is_subset(&head_roles) {
            weakened.push("required-role-removed");
        }
        if base_revision.scope() != head_revision.scope() {
            weakened.push("scope-changed");
        }
        if base_revision.enforcement() == EnforcementMode::Blocking
            && head_revision.enforcement() == EnforcementMode::Advisory
        {
            weakened.push("blocking-to-advisory");
        }
        if weakened.is_empty() {
            continue;
        }
        let protocol = manifest.weakening_protocol.clone().unwrap_or_default();
        let missing = [
            ("migration", protocol.migration_record.is_none()),
            ("approval", protocol.approval_record.is_none()),
            ("debt", protocol.debt_id.is_none()),
            ("waiver", protocol.waiver_id.is_none()),
        ]
        .into_iter()
        .filter_map(|(step, absent)| absent.then_some(step))
        .collect::<Vec<_>>();
        if missing.is_empty() {
            accepted.push(format!("{ratchet}:{}", weakened.join(",")));
        } else {
            for step in missing {
                diagnostics.push(weakening(
                    &ratchet,
                    &format!("{}:protocol-step-missing:{step}", weakened.join(",")),
                ));
            }
        }
    }
    diagnostics
}

fn weakening(ratchet: &str, context: &str) -> ReportDiagnostic {
    ReportDiagnostic {
        code: POLICY_WEAKENING.to_owned(),
        path: String::from("-"),
        start: 0,
        end: 0,
        context: format!("ratchet:{ratchet}:{context}"),
    }
}

/// Runs the provider-neutral base-to-head change check.
///
/// # Errors
///
/// Returns an encoding failure outside the report protocol.
pub fn change_check(
    base: &StaticOutcome,
    head: &StaticOutcome,
    manifest: &ChangeManifest,
) -> Result<ChangeOutcome, CommandError> {
    let impacted = impacted_set(&base.graph, &head.graph);
    let mut diagnostics = Vec::new();
    let mut accepted = Vec::new();
    for (kind, id) in &impacted {
        let matching = manifest
            .disposition
            .iter()
            .filter(|row| row.object_kind == *kind && row.object_id == *id)
            .collect::<Vec<_>>();
        match matching.as_slice() {
            [] => diagnostics.push(ReportDiagnostic {
                code: IMPACT_DISPOSITION_MISSING.to_owned(),
                path: String::from("-"),
                start: 0,
                end: 0,
                context: format!("{kind}:{id}"),
            }),
            [row] => {
                if row.kind == "reaffirmed"
                    && *kind == "evidence-carrier"
                    && !carrier_is_fresh(&head.graph, id)
                {
                    diagnostics.push(ReportDiagnostic {
                        code: REAFFIRMATION_REVIEW_ONLY.to_owned(),
                        path: String::from("-"),
                        start: 0,
                        end: 0,
                        context: format!("{kind}:{id}:stale-carrier"),
                    });
                }
            }
            _ => diagnostics.push(ReportDiagnostic {
                code: IMPACT_DISPOSITION_DUPLICATE.to_owned(),
                path: String::from("-"),
                start: 0,
                end: 0,
                context: format!("{kind}:{id}"),
            }),
        }
    }
    diagnostics.extend(ratchet_diagnostics(
        &base.graph,
        &head.graph,
        manifest,
        &mut accepted,
    ));
    diagnostics.sort();
    diagnostics.dedup();
    accepted.sort();
    let verdict = if diagnostics.is_empty() {
        "accept"
    } else {
        "reject"
    };
    let report = serde_json::to_string(&ChangeReport {
        schema: "chelis-impact-report/v1",
        impacted: impacted
            .iter()
            .map(|(kind, id)| format!("{kind}:{id}"))
            .collect(),
        diagnostics: &diagnostics,
        accepted_weakenings: &accepted,
        verdict,
    })
    .map_err(|_| CommandError::Encoding)?;
    let exit = if diagnostics.is_empty() {
        EXIT_OK
    } else {
        EXIT_CHANGE
    };
    Ok(ChangeOutcome {
        report,
        diagnostics,
        accepted_weakenings: accepted,
        exit,
    })
}
