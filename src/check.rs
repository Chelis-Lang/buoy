//! Shared authoritative repository checking and deterministic presentation values.

use crate::adapter_sdk::{
    AdapterConfiguration, AdapterSdkError, MaterializedSource, SavedSnapshotInput, SourceProduct,
    SourceRootClass, built_in_binding_configuration, built_in_configuration,
    built_in_mounted_binding_configuration, built_in_registry,
};
use crate::adapters::{AuthorityExpectation, SourceSpan};
use crate::repository::{RepositoryConfig, RepositorySnapshot};
use buoy_core::{
    ActiveAlgorithm, ArtifactBindingAttachment, BindingAttachment, BindingValidationContext,
    CanonicalValue, Diagnostic, GraphValidationContext, GraphValidationLane, GraphValidationLanes,
    IdRegistry, ObjectKind, Oracle, OracleKind, ParsedItemIdentity, RepositoryPath, SchemaId,
    ScopeId, SemanticDigest, SourceAuthority, SourceBudgets, SourceEdge, SourceGraph,
    SourceGraphError, SourceItemKind, SourceMap, SourceMapEntry, SourceNode, SourceNodeRef,
    SourceRef, SourceRole, SourceTraversal, SpecGraph, StableId, StoredBinding, ToolIdentity,
    TypedPayload, Verdict, implementation_binding_diagnostics, project_logical_path,
    semantic_digest_with, semantic_graph_digest_with, source_root_closure,
    validate_artifact_attachment, validate_graph_with_context,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Frozen specification kind that a compact binding requirement must have.
const OPENSPEC_REQUIREMENT_KIND: &str = "openspec/requirement/v1";

/// Stable shell/core diagnostic projection used by every frontend.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ShellDiagnostic {
    /// Stable code.
    pub code: String,
    /// Optional canonical repository path.
    pub path: Option<String>,
    /// Inclusive original-byte start.
    pub start: u64,
    /// Exclusive original-byte end.
    pub end: u64,
    /// Optional primary stable ID.
    pub id: Option<String>,
    /// Schema-owned deterministic context.
    pub context: String,
}

/// Advisory migration and visible-debt counters.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct MigrationReport {
    /// Selected text files containing normative-looking unatomized prose.
    pub normative_looking_files: Vec<String>,
    /// Ratified/live atoms without any attached carrier.
    pub uncovered_atoms: Vec<String>,
    /// Declared governed-surface records awaiting graph materialization.
    pub surface_declarations: usize,
    /// Surface declarations whose member dispositions are not materialized.
    pub undecided_surface_members: usize,
    /// Declared coverage policies.
    pub policy_declarations: usize,
    /// Declared debt records.
    pub debt_declarations: usize,
    /// Declared waivers.
    pub waiver_declarations: usize,
    /// Declared intentional derivations.
    pub derivation_declarations: usize,
    /// Declared formal properties.
    pub formal_property_declarations: usize,
    /// Equal-byte file groups not represented by a derivation.
    pub possible_untracked_copies: usize,
    /// Active debt count.
    pub active_debt: usize,
    /// Active waiver count.
    pub active_waivers: usize,
    /// Carrier count by explicit role tag.
    pub carrier_counts: BTreeMap<String, usize>,
}

/// Shared saved-snapshot check result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckResult {
    /// Semantic graph.
    pub graph: SpecGraph,
    /// Canonical semantic graph digest.
    pub graph_digest: SemanticDigest,
    /// Retained repository-level source context for the same snapshot.
    pub source: RepositorySourceAggregate,
    /// Canonically ordered diagnostics.
    pub diagnostics: Vec<ShellDiagnostic>,
    /// Advisory migration facts that never synthesize assurance.
    pub migration: MigrationReport,
}

/// Complete repository-level source context built from every adapter product.
///
/// No per-file adapter claims repository closure, so this aggregate is the one
/// place where a reference from one selected adapter reaches a target that
/// another selected adapter materialized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositorySourceAggregate {
    graph: SourceGraph,
    closure: SourceTraversal,
    spans: BTreeMap<SourceNodeRef, SourceSpan>,
    standing_roots: Vec<SourceNodeRef>,
    authorities: Vec<SourceAuthority>,
    attachments: Vec<BindingAttachment>,
    artifact_attachments: Vec<ArtifactBindingAttachment>,
    attachment_diagnostics: Vec<ShellDiagnostic>,
    store: Vec<StoredBinding>,
}

impl RepositorySourceAggregate {
    /// Closed, deduplicated, acyclic aggregate source graph.
    #[must_use]
    pub const fn graph(&self) -> &SourceGraph {
        &self.graph
    }

    /// Bounded canonical closure over every declared current root.
    #[must_use]
    pub const fn closure(&self) -> &SourceTraversal {
        &self.closure
    }

    /// Exact parser-backed span of one retained node.
    #[must_use]
    pub fn span(&self, node: SourceNodeRef) -> Option<&SourceSpan> {
        self.spans.get(&node)
    }

    /// Roots a standing specification selector established, in canonical order.
    ///
    /// Only these roots may later establish current `OpenSpec` document roots.
    #[must_use]
    pub fn standing_roots(&self) -> &[SourceNodeRef] {
        &self.standing_roots
    }

    /// Materialized requirement-to-atom authorities in canonical order.
    #[must_use]
    pub fn authorities(&self) -> &[SourceAuthority] {
        &self.authorities
    }

    /// Attached compact locators in canonical order.
    #[must_use]
    pub fn attachments(&self) -> &[BindingAttachment] {
        &self.attachments
    }

    /// Parsed artifact sidecars in canonical order.
    #[must_use]
    pub fn artifact_attachments(&self) -> &[ArtifactBindingAttachment] {
        &self.artifact_attachments
    }

    /// Materialized object-store records in canonical order.
    #[must_use]
    pub fn store(&self) -> &[StoredBinding] {
        &self.store
    }

    /// Every retained parsed item, in canonical node order.
    #[must_use]
    pub fn parsed_items(&self) -> Vec<ParsedItemIdentity> {
        self.graph
            .nodes()
            .iter()
            .map(|node| node.item().clone())
            .collect()
    }

    /// Returns each fresh resolved binding in canonical object order.
    ///
    /// Stale, ambiguous, missing, or mismatched attachments remain diagnostics.
    /// They do not enter the accepted semantic graph.
    ///
    /// # Panics
    ///
    /// Panics if the fixed `OpenSpec` requirement kind becomes invalid.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn fresh_bindings(&self) -> Vec<buoy_core::ImplementationBinding> {
        let requirement_kind = SourceItemKind::new(OPENSPEC_REQUIREMENT_KIND)
            .expect("the pinned requirement item kind is valid");
        let context = BindingValidationContext::new(
            &self.graph,
            &self.closure,
            requirement_kind,
            &self.authorities,
        );
        let mut fresh = BTreeMap::new();
        for attachment in &self.attachments {
            let matching = self
                .store
                .iter()
                .filter(|member| member.locator() == attachment.record())
                .collect::<Vec<_>>();
            let [member] = matching.as_slice() else {
                continue;
            };
            if implementation_binding_diagnostics(
                std::slice::from_ref(attachment),
                std::slice::from_ref(*member),
                &context,
            )
            .is_empty()
            {
                fresh.insert(member.record().meta().digest(), member.record().clone());
            }
        }
        fresh.into_values().collect()
    }

    /// Reports every failed compact-binding fact against this exact context.
    ///
    /// A binding without diagnostics is fresh navigation only. It supplies no
    /// behavioral coverage, execution verdict, proof, or approval authority.
    ///
    /// Each reported fact carries the exact span of the attachment subject the
    /// adapter retained, so a stale requirement points at the attribute that
    /// claims it rather than at the whole file.
    ///
    /// # Panics
    ///
    /// Panics if the pinned `OpenSpec` requirement item kind stops satisfying
    /// the frozen source-item-kind grammar.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn binding_diagnostics(&self) -> Vec<ShellDiagnostic> {
        let requirement_kind = SourceItemKind::new(OPENSPEC_REQUIREMENT_KIND)
            .expect("the pinned requirement item kind is valid");
        let context = BindingValidationContext::new(
            &self.graph,
            &self.closure,
            requirement_kind,
            &self.authorities,
        );
        let mut diagnostics = self.attachment_diagnostics.clone();
        diagnostics.extend(
            implementation_binding_diagnostics(&self.attachments, &self.store, &context)
                .into_iter()
                .map(|diagnostic| self.project_binding_diagnostic(diagnostic)),
        );
        diagnostics.extend(self.duplicate_record_diagnostics());
        diagnostics.sort_unstable();
        diagnostics.dedup();
        diagnostics
    }

    /// Attaches the exact retained subject span to one core binding fact.
    fn project_binding_diagnostic(&self, diagnostic: Diagnostic) -> ShellDiagnostic {
        let span = diagnostic
            .fields
            .iter()
            .find(|(key, _)| key == "subject")
            .and_then(|(_, value)| SemanticDigest::parse_canonical(value).ok())
            .and_then(|digest| self.spans.get(&SourceNodeRef::new(digest)));
        let mut projected = project_core_diagnostic(diagnostic);
        if let Some(span) = span {
            projected.path = Some(span.path.as_str().to_owned());
            projected.start = span.start;
            projected.end = span.end;
            return projected;
        }
        projected
    }

    /// Reports one locator that several unequal stored records claim.
    ///
    /// An unattached duplicate settles no record either, so the store reports
    /// it without waiting for a compact attribute to address it.
    fn duplicate_record_diagnostics(&self) -> Vec<ShellDiagnostic> {
        let mut claimed = BTreeSet::new();
        let mut diagnostics = Vec::new();
        for member in &self.store {
            if !claimed.insert(member.locator()) {
                diagnostics.push(ShellDiagnostic {
                    code: String::from("CAB-OBJECT-DUPLICATE"),
                    path: None,
                    start: 0,
                    end: 0,
                    id: Some(member.record().meta().id().as_str().to_string()),
                    context: member.locator().canonical_text(),
                });
            }
        }
        diagnostics
    }
}

/// Every retained aggregate collection and its remaining budget.
///
/// Each collection is counted before a new row is retained, so an oversized
/// repository fails closed during aggregation rather than after materializing
/// an unbounded set.
#[derive(Default)]
struct SourceAccumulator {
    nodes: BTreeMap<SemanticDigest, SourceNode>,
    spans: BTreeMap<SourceNodeRef, SourceSpan>,
    edges: BTreeSet<SourceEdge>,
    roots: BTreeSet<SourceNodeRef>,
    standing_roots: BTreeSet<SourceNodeRef>,
    authorities: BTreeSet<SourceAuthority>,
    attachments: BTreeSet<BindingAttachment>,
    artifact_attachments: BTreeSet<ArtifactBindingAttachment>,
    store: BTreeMap<(SemanticDigest, SemanticDigest), StoredBinding>,
    node_count: u32,
    reference_count: u32,
}

impl SourceAccumulator {
    fn admit_node(&mut self, limit: u32) -> Result<(), CheckError> {
        admit_row(&mut self.node_count, limit, SourceGraphError::NodeBudget)
    }

    fn admit_reference(&mut self, limit: u32) -> Result<(), CheckError> {
        admit_row(
            &mut self.reference_count,
            limit,
            SourceGraphError::EdgeBudget,
        )
    }

    fn absorb(
        &mut self,
        product: &SourceProduct,
        budgets: SourceBudgets,
    ) -> Result<(), CheckError> {
        let nodes = budgets.nodes().get();
        let references = budgets.edges().get();
        for retained in product.nodes() {
            let node = retained.node();
            if !self.nodes.contains_key(&node.digest()) {
                self.admit_node(nodes)?;
            }
            if self
                .nodes
                .insert(node.digest(), node.clone())
                .is_some_and(|prior| &prior != node)
            {
                return Err(CheckError::SourceGraph(
                    SourceGraphError::NodeDigestCollision,
                ));
            }
            // Two products may retain one node from separate files, so the
            // canonically least span wins and no product order changes it.
            self.spans
                .entry(node.reference())
                .and_modify(|span| {
                    if retained.span() < span {
                        *span = retained.span().clone();
                    }
                })
                .or_insert_with(|| retained.span().clone());
        }
        for edge in product.edges() {
            if self.edges.insert(*edge) {
                self.admit_reference(references)?;
            }
        }
        for root in product.roots() {
            if self.roots.insert(root.node()) {
                self.admit_reference(references)?;
            }
            if root.class() == SourceRootClass::StandingSpecification {
                self.standing_roots.insert(root.node());
            }
        }
        for authority in product.authorities() {
            if self.authorities.insert(authority.clone()) {
                self.admit_reference(references)?;
            }
        }
        for attachment in product.attachments() {
            if self.attachments.insert(*attachment) {
                self.admit_reference(references)?;
            }
        }
        for attachment in product.artifact_attachments() {
            if self.artifact_attachments.insert(attachment.clone()) {
                self.admit_reference(references)?;
            }
        }
        for member in product.bindings() {
            let key = (member.locator().digest(), member.record().meta().digest());
            if !self.store.contains_key(&key) {
                self.admit_reference(references)?;
            }
            self.store.insert(key, member.clone());
        }
        Ok(())
    }
}

/// Adds one aggregated row and fails closed at the configured limit.
fn admit_row(count: &mut u32, limit: u32, error: SourceGraphError) -> Result<(), CheckError> {
    if *count >= limit {
        return Err(CheckError::SourceGraph(error));
    }
    *count += 1;
    Ok(())
}

/// Aggregates every selected adapter source product before reference resolution.
///
/// Nodes, edges, roots, authorities, attachments, and records enter one
/// canonical set in product order, so equal rows aggregate idempotently and no
/// product permutation changes the aggregate or its digest.
///
/// Every retained collection is counted against its configured limit as it
/// grows, so an oversized repository fails closed during aggregation rather
/// than after materializing an unbounded set.
///
/// # Errors
///
/// Returns a source-graph error for a digest collision, a mixed digest member,
/// a dangling reference, a cycle, or an exceeded node, edge, depth, or visited
/// budget.
pub fn aggregate_source_products(
    active: ActiveAlgorithm,
    products: &[SourceProduct],
    budgets: SourceBudgets,
) -> Result<RepositorySourceAggregate, CheckError> {
    let mut aggregate = SourceAccumulator::default();
    for product in products {
        aggregate.absorb(product, budgets)?;
    }
    let graph = SourceGraph::new_with(
        active,
        aggregate.nodes.into_values().collect(),
        aggregate.edges.into_iter().collect(),
        aggregate.roots.into_iter().collect(),
        budgets,
    )
    .map_err(CheckError::SourceGraph)?;
    let closure = source_root_closure(&graph).map_err(CheckError::SourceGraph)?;
    let artifact_attachments = aggregate
        .artifact_attachments
        .into_iter()
        .collect::<Vec<_>>();
    let store = aggregate.store.into_values().collect::<Vec<_>>();
    let (resolved_attachments, attachment_diagnostics) =
        resolve_artifact_attachments(&graph, &artifact_attachments, &store);
    let mut attachments = aggregate.attachments.into_iter().collect::<Vec<_>>();
    attachments.extend(resolved_attachments);
    attachments.sort_unstable();
    attachments.dedup();
    Ok(RepositorySourceAggregate {
        graph,
        closure,
        spans: aggregate.spans,
        standing_roots: aggregate.standing_roots.into_iter().collect(),
        authorities: aggregate.authorities.into_iter().collect(),
        attachments,
        artifact_attachments,
        attachment_diagnostics,
        store,
    })
}

fn resolve_artifact_attachments(
    graph: &SourceGraph,
    attachments: &[ArtifactBindingAttachment],
    store: &[StoredBinding],
) -> (Vec<BindingAttachment>, Vec<ShellDiagnostic>) {
    let mut by_relation = BTreeMap::<String, Vec<&ArtifactBindingAttachment>>::new();
    for attachment in attachments {
        by_relation
            .entry(attachment.relation().as_str().to_owned())
            .or_default()
            .push(attachment);
    }
    let mut resolved = Vec::new();
    let mut diagnostics = Vec::new();
    for members in by_relation.into_values() {
        let attachment = members[0];
        if members.len() != 1 {
            diagnostics.push(artifact_attachment_diagnostic(
                attachment,
                "CAB-ATTACHMENT-DUPLICATE",
                attachment.relation().as_str(),
            ));
            continue;
        }
        let Some(node) = graph.node(attachment.subject()) else {
            diagnostics.push(artifact_attachment_diagnostic(
                attachment,
                "CAB-SUBJECT-STALE",
                &attachment.subject().canonical_text(),
            ));
            continue;
        };
        if node.item().path() != attachment.artifact() {
            diagnostics.push(artifact_attachment_diagnostic(
                attachment,
                "CAB-ATTACHMENT-PATH",
                node.item().path().as_str(),
            ));
            continue;
        }
        if node.item().kind().as_str() != "artifact/file/v1" {
            diagnostics.push(artifact_attachment_diagnostic(
                attachment,
                "CAB-SUBJECT-KIND",
                node.item().kind().as_str(),
            ));
            continue;
        }
        let matching = store
            .iter()
            .filter(|member| member.locator() == attachment.record())
            .collect::<Vec<_>>();
        let [member] = matching.as_slice() else {
            diagnostics.push(artifact_attachment_diagnostic(
                attachment,
                if matching.is_empty() {
                    "CAB-RECORD-ABSENT"
                } else {
                    "CAB-RECORD-AMBIGUOUS"
                },
                &attachment.record().canonical_text(),
            ));
            continue;
        };
        if let Err(error) = validate_artifact_attachment(attachment, member.record(), node) {
            diagnostics.push(artifact_attachment_diagnostic(
                attachment,
                error.code(),
                &member.record().record().canonical_text(),
            ));
            continue;
        }
        resolved.push(BindingAttachment::new(
            attachment.subject(),
            attachment.record(),
        ));
    }
    diagnostics.sort_unstable();
    (resolved, diagnostics)
}

fn artifact_attachment_diagnostic(
    attachment: &ArtifactBindingAttachment,
    code: &str,
    context: &str,
) -> ShellDiagnostic {
    ShellDiagnostic {
        code: code.to_owned(),
        path: Some(attachment.artifact().as_str().to_owned()),
        start: 0,
        end: 0,
        id: Some(attachment.relation().as_str().to_owned()),
        context: context.to_owned(),
    }
}

/// Returns the one source role that a materialized object kind carries.
///
/// A kind outside this total mapping has no source role in the frozen protocol,
/// so it contributes no source-map entry rather than an invented role.
const fn source_role_for(kind: ObjectKind) -> Option<SourceRole> {
    match kind {
        ObjectKind::Atom | ObjectKind::Tombstone => Some(SourceRole::Authority),
        ObjectKind::EvidenceCarrier => Some(SourceRole::Carrier),
        ObjectKind::ImplementationLink => Some(SourceRole::Implementation),
        ObjectKind::DecisionSurface => Some(SourceRole::Surface),
        ObjectKind::CoveragePolicy => Some(SourceRole::Policy),
        ObjectKind::Derivation => Some(SourceRole::Derivation),
        ObjectKind::FiniteModel => Some(SourceRole::Model),
        ObjectKind::FormalProperty => Some(SourceRole::Property),
        ObjectKind::Assumption => Some(SourceRole::Assumption),
        ObjectKind::ProofRecord => Some(SourceRole::Proof),
        ObjectKind::DebtRecord => Some(SourceRole::Debt),
        ObjectKind::Waiver => Some(SourceRole::Waiver),
        ObjectKind::Oracle
        | ObjectKind::ParsedItem
        | ObjectKind::Reaffirmation
        | ObjectKind::ChangeManifest
        | ObjectKind::ChangeDisposition
        | ObjectKind::ExternalTraceLink
        | ObjectKind::ImplementationBinding => None,
    }
}

/// Builds the exact object-to-source map that the selected adapters reported.
///
/// One object may be declared across several selected files, so rows join under
/// their exact object key. Two rows that disagree on the object digest join
/// under the canonically least digest, which keeps the map deterministic under
/// every file order.
#[allow(clippy::expect_used)]
fn repository_source_map(sources: &[MaterializedSource]) -> SourceMap {
    let mut grouped =
        BTreeMap::<(ObjectKind, StableId), (SemanticDigest, BTreeSet<SourceRef>)>::new();
    for source in sources {
        let object = source.object();
        if source_role_for(object.kind()).is_none() {
            continue;
        }
        let span = source.span();
        let reference = SourceRef::new(span.path.clone(), span.start, span.end)
            .expect("adapter spans are validated inclusive-start and exclusive-end");
        let entry = grouped
            .entry((object.kind(), object.id().clone()))
            .or_insert_with(|| (object.digest(), BTreeSet::new()));
        entry.0 = entry.0.min(object.digest());
        entry.1.insert(reference);
    }
    let entries = grouped
        .into_iter()
        .map(|((kind, id), (digest, references))| {
            let role = source_role_for(kind).expect("unmapped kinds are filtered above");
            SourceMapEntry::new(
                buoy_core::ObjectRef::new(kind, id, digest),
                references.into_iter().collect(),
                role,
            )
            .expect("one grouped object contributes a nonempty compatible source set")
        })
        .collect();
    SourceMap::new(entries).expect("object keys are grouped, so no entry repeats one key")
}

/// Collects every parsed item the selected adapters retained, without repeats.
fn repository_parsed_items(
    sources: &[MaterializedSource],
    aggregate: &RepositorySourceAggregate,
) -> Vec<ParsedItemIdentity> {
    let mut items = BTreeMap::<SemanticDigest, ParsedItemIdentity>::new();
    for source in sources {
        items.insert(source.item().digest(), source.item().clone());
    }
    for item in aggregate.parsed_items() {
        items.insert(item.digest(), item);
    }
    items.into_values().collect()
}

/// Returns one original-byte extent per canonical snapshot path.
fn repository_source_extents(snapshot: &RepositorySnapshot) -> Vec<(RepositoryPath, u64)> {
    snapshot
        .files
        .iter()
        .map(|file| (file.path.clone(), file.bytes.len() as u64))
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .collect()
}

/// Declares the lanes this repository snapshot has an authority source for.
///
/// The snapshot authorizes exactly two lanes. The selected adapters report
/// every object-to-source mapping and every parsed item they retained, and the
/// snapshot itself reports each file extent, so source currency and parsed-item
/// staleness are checkable facts.
///
/// The registry, approval, and host-schema lanes stay undeclared. A repository
/// snapshot contains no stable-ID allocation registry, no approval-permission
/// policy, and no host-schema registration, so running those lanes against an
/// empty snapshot would report absent authority as failed authority. Declaring
/// them requires a real authority source, not a self-authorized empty set.
const fn repository_validation_lanes() -> GraphValidationLanes {
    GraphValidationLanes::none()
        .with(GraphValidationLane::Sources)
        .with(GraphValidationLane::ParsedItems)
}

fn record_declaration(migration: &mut MigrationReport, kind: ObjectKind) {
    match kind {
        ObjectKind::DecisionSurface => {
            migration.surface_declarations += 1;
            migration.undecided_surface_members += 1;
        }
        ObjectKind::CoveragePolicy => migration.policy_declarations += 1,
        ObjectKind::DebtRecord => migration.debt_declarations += 1,
        ObjectKind::Waiver => migration.waiver_declarations += 1,
        ObjectKind::Derivation => migration.derivation_declarations += 1,
        ObjectKind::FormalProperty => migration.formal_property_declarations += 1,
        _ => {}
    }
}

/// Exercises the closed no-op declaration-kind branch under an explicit control feature.
#[cfg(feature = "coverage-controls")]
#[must_use]
pub fn migration_kind_control(kind: ObjectKind) -> MigrationReport {
    let mut migration = MigrationReport::default();
    record_declaration(&mut migration, kind);
    migration
}

/// Constructs and validates one explicit saved snapshot through shared adapters and core semantics.
///
/// # Errors
///
/// Returns a check error for unsupported adapters, invalid configuration, or
/// canonical encoding failure.
#[allow(
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::too_many_lines
)]
pub fn check_snapshot(
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
) -> Result<CheckResult, CheckError> {
    check_snapshot_internal(snapshot, config, false)
}

/// Runs the snapshot check with one deterministic encoding failure control.
///
/// # Errors
///
/// Returns the injected encoding error after adapter materialization.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
pub fn check_snapshot_encoding_control(
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
) -> Result<CheckResult, CheckError> {
    check_snapshot_internal(snapshot, config, true)
}

#[allow(clippy::expect_used, clippy::too_many_lines)]
fn check_snapshot_internal(
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
    inject_encoding_diagnostic: bool,
) -> Result<CheckResult, CheckError> {
    let mut graph = SpecGraph::default();
    for oracle in config.oracles() {
        let configuration = oracle_digest(
            config.active_algorithm(),
            oracle,
            config.oracle_concurrency(),
        );
        let runner = ToolIdentity::new(
            ScopeId::new("configured-oracle").expect("fixed tool ID is valid"),
            "configured/v1",
            configuration,
            configuration,
        )
        .expect("validated configured-oracle tool identity");
        let selection = TypedPayload::new(
            SchemaId::new("oracle-selection/configured/v1").expect("fixed schema ID is valid"),
            Vec::new(),
        )
        .expect("pinned configured-oracle selection schema");
        let normalization = TypedPayload::new(
            SchemaId::new(&oracle.result_protocol).expect("validated oracle schema ID"),
            oracle.result_protocol.as_bytes().to_vec(),
        )
        .expect("validated configured-oracle result protocol");
        let scope = TypedPayload::new(
            SchemaId::new("oracle-scope/repository-root/v1").expect("fixed schema ID is valid"),
            oracle.working_root.as_bytes().to_vec(),
        )
        .expect("validated configured-oracle working root");
        let declaration = Oracle::new(
            StableId::new(&oracle.id)
                .expect("repository configuration validates complete oracle declarations"),
            if oracle.derivation.is_some() {
                OracleKind::DerivationConsistency
            } else {
                OracleKind::Host
            },
            runner,
            selection,
            normalization,
            configuration,
            scope,
            Vec::new(),
        )
        .expect("repository configuration validates complete oracle declarations");
        graph.oracles.push(declaration);
    }

    let budgets = config.budgets();
    let mut graph_objects = u64::try_from(graph.object_meta().len())
        .expect("configured oracle count is bounded below u64::MAX");
    // Configured oracle declarations carry no direct ObjectMeta references.
    // Adapter products are the first possible source of graph references.
    let mut graph_references = 0_u64;
    if graph_objects > budgets.graph_objects {
        return Err(CheckError::GraphObjectsBudget);
    }

    let registry = built_in_registry().expect("pinned built-in adapter registry is valid");
    let mut diagnostics = Vec::new();
    let mut source_products = Vec::new();
    let mut materialized_sources = Vec::new();
    let mut migration = MigrationReport::default();
    let mut selector_authorities = BTreeMap::<String, usize>::new();
    let mut byte_groups = BTreeMap::<&[u8], usize>::new();
    for file in &snapshot.files {
        *byte_groups.entry(&file.bytes).or_default() += 1;
        let per_file_expectation = match file.authority_expectation {
            AuthorityExpectation::Forbidden => AuthorityExpectation::Forbidden,
            AuthorityExpectation::Optional | AuthorityExpectation::Required => {
                AuthorityExpectation::Optional
            }
        };
        let configuration = if let Some(bytes) = config.adapter_configuration(&file.adapter) {
            #[allow(clippy::expect_used)]
            AdapterConfiguration::new(
                &file.adapter,
                "buoy.adapter.configuration/v1",
                bytes.to_vec(),
            )
            .expect("parsed repository adapter configuration must satisfy SDK bounds")
        } else if file.adapter == "binding/v1" {
            // A compact locator names no algorithm, so the object store computes
            // record identities under the one member this repository selected.
            if let Some(mount) = config.mount_for_logical_path(&file.path) {
                built_in_mounted_binding_configuration(
                    config.active_algorithm(),
                    per_file_expectation,
                    mount.prefix(),
                )
                .expect("a parsed logical mount forms binding configuration")
            } else {
                built_in_binding_configuration(config.active_algorithm(), per_file_expectation)
                    .expect(
                        "a selected digest member and parsed expectation form binding configuration",
                    )
            }
        } else {
            built_in_configuration(&file.adapter, per_file_expectation)
                .map_err(project_adapter_error)?
        };
        let input = SavedSnapshotInput::new_with(
            config.active_algorithm(),
            &file.path,
            &file.bytes,
            &configuration,
        )
        .expect("snapshot paths and built-in configurations canonically encode");
        let output = registry
            .materialize(&input)
            .expect("built-in adapters totalize every bounded saved input");
        let (materialized, sources, adapter_diagnostics, advisory_declarations, source_product) =
            output.into_check_parts();
        source_products.push(source_product);
        materialized_sources.extend(sources);
        if inject_encoding_diagnostic
            | adapter_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "DET-ENC-TEXT-NFC")
        {
            return Err(CheckError::Encoding);
        }
        let materialized_meta = materialized.object_meta();
        add_budgeted_count(
            &mut graph_objects,
            materialized_meta.len(),
            budgets.graph_objects,
            CheckError::GraphObjectsBudget,
        )?;
        for meta in &materialized_meta {
            add_budgeted_count(
                &mut graph_references,
                meta.references().len(),
                budgets.graph_references,
                CheckError::GraphReferencesBudget,
            )?;
        }
        ensure_diagnostic_capacity(
            diagnostics.len(),
            adapter_diagnostics.len(),
            budgets.diagnostics,
        )?;
        let authority_count = materialized.atoms.len();
        *selector_authorities
            .entry(file.selector_root.clone())
            .or_insert(0) += authority_count;
        for carrier in &materialized.evidence_carriers {
            let role = format!("{:?}", carrier.role());
            *migration.carrier_counts.entry(role).or_insert(0) += 1;
        }
        for kind in advisory_declarations {
            record_declaration(&mut migration, kind);
        }
        diagnostics.extend(
            adapter_diagnostics
                .into_iter()
                .map(|diagnostic| ShellDiagnostic {
                    code: diagnostic.code.to_string(),
                    path: Some(diagnostic.span.path.as_str().to_string()),
                    start: diagnostic.span.start,
                    end: diagnostic.span.end,
                    id: None,
                    context: diagnostic.context,
                }),
        );
        if file.adapter == "markdown/v1" && authority_count == 0 && normative_looking(&file.bytes) {
            migration
                .normative_looking_files
                .push(file.path.as_str().to_string());
        }
        extend_graph(&mut graph, materialized);
    }

    for selector in config.selectors() {
        let selector_root = selector.mount().map_or_else(
            || selector.root().to_owned(),
            |mount_id| {
                let mount = config
                    .mount(mount_id)
                    .expect("repository configuration retains every selector mount");
                project_logical_path(mount, selector.root_path())
                    .expect("parsed mount projection remains canonical")
                    .as_str()
                    .to_owned()
            },
        );
        if selector.expectation() == AuthorityExpectation::Required
            && selector_authorities
                .get(&selector_root)
                .copied()
                .unwrap_or(0)
                == 0
        {
            ensure_diagnostic_capacity(diagnostics.len(), 1, budgets.diagnostics)?;
            diagnostics.push(ShellDiagnostic {
                code: String::from("SPG-ADAPTER-EXPECTED-AUTHORITY-EMPTY"),
                path: Some(selector_root),
                start: 0,
                end: 0,
                id: None,
                context: String::from("selector-wide"),
            });
        }
    }

    // Every selected product is materialized, so repository-level reference
    // resolution runs once against the complete retained source context rather
    // than against per-file products that no adapter can close.
    let source = aggregate_source_products(
        config.active_algorithm(),
        &source_products,
        config.source_budgets(),
    )?;
    let fresh_bindings = source.fresh_bindings();
    add_budgeted_count(
        &mut graph_objects,
        fresh_bindings.len(),
        budgets.graph_objects,
        CheckError::GraphObjectsBudget,
    )?;
    graph.implementation_bindings = fresh_bindings;
    let source_map = repository_source_map(&materialized_sources);
    let parsed_items = repository_parsed_items(&materialized_sources, &source);
    let artifacts = snapshot
        .artifact_index_with(config.active_algorithm())
        .expect("validated snapshot paths and one selected digest member encode artifacts");
    let allocations = IdRegistry::default();
    let context = GraphValidationContext::new(
        &allocations,
        &source_map,
        &artifacts,
        &parsed_items,
        Vec::new(),
        repository_source_extents(snapshot),
        Vec::new(),
    )
    .expect("one snapshot path occurs once and no permission or schema row is declared")
    .with_lanes(repository_validation_lanes());
    let core_diagnostics = validate_graph_with_context(&graph, &context);
    ensure_diagnostic_capacity(
        diagnostics.len(),
        core_diagnostics.len(),
        budgets.diagnostics,
    )?;
    diagnostics.extend(core_diagnostics.into_iter().map(project_core_diagnostic));
    let binding_diagnostics = source.binding_diagnostics();
    ensure_diagnostic_capacity(
        diagnostics.len(),
        binding_diagnostics.len(),
        budgets.diagnostics,
    )?;
    diagnostics.extend(binding_diagnostics);
    let covered = graph
        .evidence_carriers
        .iter()
        .flat_map(|carrier| carrier.atoms().iter().map(buoy_core::AtomRevisionRef::id))
        .collect::<BTreeSet<_>>();
    migration.uncovered_atoms = graph
        .atoms
        .iter()
        .filter(|atom| !covered.contains(atom.meta().id()))
        .map(|atom| atom.meta().id().as_str().to_string())
        .collect();
    migration.uncovered_atoms.sort_unstable();
    migration.active_debt = migration.debt_declarations;
    migration.active_waivers = migration.waiver_declarations;
    migration.possible_untracked_copies = byte_groups
        .values()
        .filter(|count| **count > 1)
        .count()
        .saturating_sub(migration.derivation_declarations);
    migration.normative_looking_files.sort_unstable();
    diagnostics.sort_unstable();
    diagnostics.dedup();
    let graph_digest = semantic_graph_digest_with(config.active_algorithm(), &graph)
        .map_err(|_| CheckError::Encoding)?;
    Ok(CheckResult {
        graph,
        graph_digest,
        source,
        diagnostics,
        migration,
    })
}

/// Lint frontend semantics over a saved snapshot.
///
/// # Errors
///
/// Returns the same shared check errors as [`check_snapshot`].
pub fn lint_saved_snapshot(
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
) -> Result<Vec<ShellDiagnostic>, CheckError> {
    check_snapshot(snapshot, config).map(|result| result.diagnostics)
}

/// LSP frontend semantics over a saved snapshot; unsaved buffers are deliberately absent.
///
/// # Errors
///
/// Returns the same shared check errors as [`check_snapshot`].
pub fn lsp_saved_snapshot(
    snapshot: &RepositorySnapshot,
    config: &RepositoryConfig,
) -> Result<Vec<ShellDiagnostic>, CheckError> {
    check_snapshot(snapshot, config).map(|result| result.diagnostics)
}

#[allow(clippy::expect_used)]
fn oracle_digest(
    active: buoy_core::ActiveAlgorithm,
    oracle: &crate::repository::OracleCommand,
    concurrency: u64,
) -> SemanticDigest {
    semantic_digest_with(
        active,
        "oracle",
        vec![
            (1, oracle.canonical_value()),
            (2, CanonicalValue::U64(concurrency)),
        ],
    )
    .expect("validated bounded oracle fields canonically encode")
}

fn project_adapter_error(error: AdapterSdkError) -> CheckError {
    match error {
        AdapterSdkError::UnsupportedAdapter => CheckError::UnsupportedAdapter,
        _ => CheckError::Encoding,
    }
}

/// Projects one SDK error through the production check boundary.
#[cfg(feature = "coverage-controls")]
#[doc(hidden)]
#[must_use]
pub fn project_adapter_error_control(error: AdapterSdkError) -> CheckError {
    project_adapter_error(error)
}

fn add_budgeted_count(
    current: &mut u64,
    additional: usize,
    maximum: u64,
    error: CheckError,
) -> Result<(), CheckError> {
    // Supported targets have at most 64-bit `usize`; saturation preserves the
    // budget failure on any wider target without an unexecutable conversion arm.
    let additional = u64::try_from(additional).unwrap_or(u64::MAX);
    *current = current.saturating_add(additional);
    if *current > maximum {
        return Err(error);
    }
    Ok(())
}

fn ensure_diagnostic_capacity(
    current: usize,
    additional: usize,
    maximum: u64,
) -> Result<(), CheckError> {
    let total = current.saturating_add(additional);
    // Supported targets have at most 64-bit `usize`; an oversized wider-target
    // value saturates into the same diagnostics-budget rejection.
    let total = u64::try_from(total).unwrap_or(u64::MAX);
    if total > maximum {
        return Err(CheckError::DiagnosticsBudget);
    }
    Ok(())
}

fn extend_graph(target: &mut SpecGraph, mut source: SpecGraph) {
    target.atoms.append(&mut source.atoms);
    target.tombstones.append(&mut source.tombstones);
    target
        .decision_surfaces
        .append(&mut source.decision_surfaces);
    target
        .evidence_carriers
        .append(&mut source.evidence_carriers);
    target
        .implementation_links
        .append(&mut source.implementation_links);
    target.derivations.append(&mut source.derivations);
    target
        .coverage_policies
        .append(&mut source.coverage_policies);
    target.debt_records.append(&mut source.debt_records);
    target.waivers.append(&mut source.waivers);
    target.oracles.append(&mut source.oracles);
    target.finite_models.append(&mut source.finite_models);
    target
        .formal_properties
        .append(&mut source.formal_properties);
    target.assumptions.append(&mut source.assumptions);
    target.proof_records.append(&mut source.proof_records);
    target.reaffirmations.append(&mut source.reaffirmations);
    target
        .external_trace_links
        .append(&mut source.external_trace_links);
}

fn normative_looking(bytes: &[u8]) -> bool {
    core::str::from_utf8(bytes).is_ok_and(|source| {
        [" MUST ", " MUST NOT ", " SHALL ", " SHOULD ", " MAY "]
            .iter()
            .any(|term| source.contains(term))
    })
}

/// Projects a synthetic core diagnostic through the production adapter.
#[cfg(feature = "coverage-controls")]
#[must_use]
pub fn project_diagnostic_control(diagnostic: Diagnostic) -> ShellDiagnostic {
    project_core_diagnostic(diagnostic)
}

fn project_core_diagnostic(diagnostic: Diagnostic) -> ShellDiagnostic {
    ShellDiagnostic {
        code: diagnostic.code,
        path: diagnostic.path.map(|path| path.as_str().to_string()),
        start: diagnostic.start,
        end: diagnostic.end,
        id: diagnostic.id,
        context: diagnostic
            .fields
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(","),
    }
}

/// Stable check construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckError {
    /// Canonical encoding failed.
    Encoding,
    /// Configuration closed fields were invalid.
    Configuration,
    /// Snapshot named an unsupported adapter.
    UnsupportedAdapter,
    /// Materialized semantic objects exceed the configured graph budget.
    GraphObjectsBudget,
    /// Direct semantic references exceed the configured graph budget.
    GraphReferencesBudget,
    /// Stable diagnostic retention exceeds the configured budget.
    DiagnosticsBudget,
    /// Repository-level source aggregation failed closed.
    SourceGraph(SourceGraphError),
}

impl CheckError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Encoding => "DET-ENC-SNAPSHOT",
            Self::Configuration => "REPO-CONFIG-SCHEMA",
            Self::UnsupportedAdapter => "SPG-ADAPTER-UNSUPPORTED",
            Self::GraphObjectsBudget => "REH-BUDGET-GRAPH-OBJECTS",
            Self::GraphReferencesBudget => "REH-BUDGET-GRAPH-REFERENCES",
            Self::DiagnosticsBudget => "REH-BUDGET-DIAGNOSTICS",
            Self::SourceGraph(error) => error.code(),
        }
    }
}

/// One canonical logical mount fact in a machine report.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct LogicalMountFact {
    /// Stable logical mount ID.
    pub id: String,
    /// Canonical logical path prefix.
    pub prefix: String,
}

/// One current `OpenSpec` requirement subject in a static machine report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RequirementSubjectFact {
    /// Stable requirement ID.
    pub id: String,
    /// Exact current requirement source-node digest.
    pub subject: String,
    /// Exact current semantic atom revision.
    pub atom_revision: String,
    /// Complete logical standing-spec path.
    pub path: String,
    /// Canonically ordered authority scopes.
    pub scopes: Vec<String>,
}

impl Ord for RequirementSubjectFact {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (
            &self.id,
            &self.scopes,
            &self.path,
            &self.subject,
            &self.atom_revision,
        )
            .cmp(&(
                &other.id,
                &other.scopes,
                &other.path,
                &other.subject,
                &other.atom_revision,
            ))
    }
}

impl PartialOrd for RequirementSubjectFact {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Fail-closed error for one inconsistent retained requirement authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequirementSubjectError {
    /// The authority points to a source node that is not retained.
    MissingNode,
    /// No live atom has the authority's stable ID.
    MissingAtom,
    /// More than one live atom has the authority's stable ID.
    DuplicateAtom,
    /// The live atom revision differs from the source authority revision.
    RevisionMismatch,
}

impl RequirementSubjectError {
    /// Stable diagnostic code for the inconsistent retained fact.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::MissingNode => "CAB-REQUIREMENT-SUBJECT-NODE",
            Self::MissingAtom => "CAB-REQUIREMENT-SUBJECT-ATOM",
            Self::DuplicateAtom => "CAB-REQUIREMENT-SUBJECT-AMBIGUOUS",
            Self::RevisionMismatch => "CAB-REQUIREMENT-SUBJECT-REVISION",
        }
    }
}

/// Derives the complete ordered requirement-subject inventory from one accepted check.
///
/// # Errors
///
/// Returns a fail-closed error if a retained source authority, source node, or live atom disagrees.
pub fn requirement_subject_facts(
    result: &CheckResult,
) -> Result<Vec<RequirementSubjectFact>, RequirementSubjectError> {
    let mut atoms_by_id = BTreeMap::<&str, Vec<&buoy_core::Atom>>::new();
    for atom in &result.graph.atoms {
        atoms_by_id
            .entry(atom.meta().id().as_str())
            .or_default()
            .push(atom);
    }
    let mut rows = Vec::with_capacity(result.source.authorities().len());
    for authority in result.source.authorities() {
        let node = result
            .source
            .graph()
            .node(authority.node())
            .ok_or(RequirementSubjectError::MissingNode)?;
        if node.item().kind().as_str() != OPENSPEC_REQUIREMENT_KIND {
            continue;
        }
        let matching_atoms = atoms_by_id
            .get(authority.atom().id().as_str())
            .map_or(&[][..], Vec::as_slice);
        let [atom] = matching_atoms else {
            return Err(if matching_atoms.is_empty() {
                RequirementSubjectError::MissingAtom
            } else {
                RequirementSubjectError::DuplicateAtom
            });
        };
        if atom.revision() != authority.atom().revision() {
            return Err(RequirementSubjectError::RevisionMismatch);
        }
        let scopes = atom
            .scopes()
            .iter()
            .map(|scope| scope.as_str().to_owned())
            .collect::<Vec<_>>();
        rows.push(RequirementSubjectFact {
            id: authority.atom().id().as_str().to_owned(),
            subject: authority.node().canonical_text(),
            atom_revision: authority.atom().revision().canonical_text(),
            path: node.item().path().as_str().to_owned(),
            scopes,
        });
    }
    rows.sort_unstable();
    Ok(rows)
}

/// One current opaque artifact subject in a machine report.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct OpaqueSubjectFact {
    /// Complete logical artifact path.
    pub path: String,
    /// Exact current source-node digest.
    pub subject: String,
}

/// One parsed artifact sidecar and its current freshness result.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ArtifactAttachmentFact {
    /// Stable relation ID.
    pub relation: String,
    /// Complete logical artifact path.
    pub artifact: String,
    /// Exact selected artifact source node.
    pub subject: String,
    /// Exact immutable binding record.
    pub record: String,
    /// Whether every current correspondence and freshness check passed.
    pub fresh: bool,
}

/// Returns canonical report facts for the mounted static snapshot.
#[must_use]
pub fn mounted_report_facts(
    config: &RepositoryConfig,
    result: &CheckResult,
) -> (
    Vec<LogicalMountFact>,
    Vec<OpaqueSubjectFact>,
    Vec<ArtifactAttachmentFact>,
) {
    let mounts = config
        .mounts()
        .iter()
        .map(|mount| LogicalMountFact {
            id: mount.id().as_str().to_owned(),
            prefix: mount.prefix().as_str().to_owned(),
        })
        .collect();
    let opaque_subjects = result
        .source
        .graph()
        .nodes()
        .iter()
        .filter(|node| node.item().kind().as_str() == "artifact/file/v1")
        .map(|node| OpaqueSubjectFact {
            path: node.item().path().as_str().to_owned(),
            subject: node.reference().canonical_text(),
        })
        .collect();
    let fresh = result
        .source
        .fresh_bindings()
        .into_iter()
        .map(|binding| binding.record())
        .collect::<BTreeSet<_>>();
    let artifact_attachments = result
        .source
        .artifact_attachments()
        .iter()
        .map(|attachment| ArtifactAttachmentFact {
            relation: attachment.relation().as_str().to_owned(),
            artifact: attachment.artifact().as_str().to_owned(),
            subject: attachment.subject().canonical_text(),
            record: attachment.record().canonical_text(),
            fresh: fresh.contains(&attachment.record()),
        })
        .collect();
    (mounts, opaque_subjects, artifact_attachments)
}

/// Machine report shape shared by CLI commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MachineReport {
    /// Command class.
    pub command: String,
    /// Primary static graph digest.
    pub graph: String,
    /// Canonically ordered diagnostics.
    pub diagnostics: Vec<ShellDiagnostic>,
    /// Advisory migration facts.
    pub migration: MigrationReport,
    /// Normalized oracle verdicts when applicable.
    pub verdicts: BTreeMap<String, String>,
    /// Canonical changed and transitively impacted object keys.
    pub impacts: Vec<String>,
    /// Declared logical mounts. Version 2 reports omit this field.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<LogicalMountFact>,
    /// Current `OpenSpec` requirement subjects. Reports without them omit this field.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requirement_subjects: Vec<RequirementSubjectFact>,
    /// Current opaque artifact subjects. Reports without them omit this field.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub opaque_subjects: Vec<OpaqueSubjectFact>,
    /// Parsed artifact sidecars and freshness facts. Empty reports omit this field.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artifact_attachments: Vec<ArtifactAttachmentFact>,
}

/// Renders a deterministic one-line JSON machine report.
///
/// # Errors
///
/// Returns a JSON serialization error if the closed report cannot be encoded.
pub fn render_machine(report: &MachineReport) -> Result<String, serde_json::Error> {
    serde_json::to_string(report)
}

/// Renders deterministic fixed-width-independent human text.
#[must_use]
pub fn render_human(report: &MachineReport) -> String {
    let mut lines = vec![
        format!("command: {}", report.command),
        format!("graph: {}", report.graph),
        format!("diagnostics: {}", report.diagnostics.len()),
    ];
    for diagnostic in &report.diagnostics {
        lines.push(format!(
            "{} {}:{}-{} {}",
            diagnostic.code,
            diagnostic.path.as_deref().unwrap_or("-"),
            diagnostic.start,
            diagnostic.end,
            diagnostic.id.as_deref().unwrap_or("-")
        ));
    }
    for (oracle, verdict) in &report.verdicts {
        lines.push(format!("oracle {oracle}: {verdict}"));
    }
    for impact in &report.impacts {
        lines.push(format!("impact: {impact}"));
    }
    for mount in &report.mounts {
        lines.push(format!("mount {}: {}", mount.id, mount.prefix));
    }
    for subject in &report.requirement_subjects {
        lines.push(format!(
            "requirement subject {}: {} atom={} path={} scopes={}",
            subject.id,
            subject.subject,
            subject.atom_revision,
            subject.path,
            subject.scopes.join(",")
        ));
    }
    for subject in &report.opaque_subjects {
        lines.push(format!(
            "opaque subject {}: {}",
            subject.path, subject.subject
        ));
    }
    for attachment in &report.artifact_attachments {
        lines.push(format!(
            "artifact attachment {}: {} fresh={}",
            attachment.relation, attachment.artifact, attachment.fresh
        ));
    }
    lines.join("\n") + "\n"
}

/// Converts a normalized verdict to its closed machine spelling.
#[must_use]
pub const fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::NotRun => "not-run",
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Error => "error",
        Verdict::Timeout => "timeout",
    }
}

#[cfg(all(test, feature = "blake3-256"))]
#[allow(clippy::expect_used, clippy::panic)]
mod requirement_subject_tests {
    use super::*;
    use crate::repository::{MountMappings, RepositoryConfig, snapshot_mounted_repository};
    use buoy_core::{Atom, AtomRevisionRef, LifecycleState, LogicalMountId, ScopeId};
    use std::fs;
    use std::path::Path;

    fn valid_result() -> CheckResult {
        let fixture = Path::new("fixtures/requirement-subjects/v1");
        let configuration =
            fs::read(fixture.join("buoy.toml")).expect("requirement-subject configuration");
        let config =
            RepositoryConfig::parse(&configuration).expect("requirement-subject configuration");
        let mappings = MountMappings::new(vec![(
            LogicalMountId::new("repository").expect("repository mount"),
            fixture.join("repository"),
        )])
        .expect("requirement-subject mappings");
        let snapshot =
            snapshot_mounted_repository(&mappings, &config).expect("requirement-subject snapshot");
        check_snapshot(&snapshot, &config).expect("requirement-subject check")
    }

    fn replacement_revision(id: StableId) -> AtomRevisionRef {
        AtomRevisionRef::new(id, SemanticDigest::xxh3_128([0x7f; 16]))
    }

    fn mutate_invalid_aggregate(result: &mut CheckResult, case: &str) {
        let authority = result.source.authorities[0].clone();
        match case {
            "wrong-kind" => {
                let artifact = result
                    .source
                    .graph()
                    .nodes()
                    .iter()
                    .find(|node| node.item().kind().as_str() == "artifact/file/v1")
                    .expect("artifact node")
                    .reference();
                result.source.authorities[0] =
                    SourceAuthority::new(artifact, authority.atom().clone());
            }
            "absent-node" => {
                let missing = SourceNodeRef::new(SemanticDigest::xxh3_128([0x4f; 16]));
                result.source.authorities[0] =
                    SourceAuthority::new(missing, authority.atom().clone());
            }
            "absent-atom" => {
                let absent =
                    replacement_revision(StableId::new("RQS-ABSENT").expect("absent atom ID"));
                result.source.authorities[0] = SourceAuthority::new(authority.node(), absent);
            }
            "duplicate-atom" => {
                result.graph.atoms.push(result.graph.atoms[0].clone());
            }
            "revision-mismatch" => {
                let mismatched = replacement_revision(authority.atom().id().clone());
                result.source.authorities[0] = SourceAuthority::new(authority.node(), mismatched);
            }
            _ => panic!("unknown invalid aggregate fixture: {case}"),
        }
    }

    #[test]
    fn inconsistent_requirement_subject_aggregates_fail_closed() {
        let manifest =
            fs::read_to_string("fixtures/requirement-subjects/v1/invalid-aggregates.tsv")
                .expect("invalid aggregate manifest");
        for line in manifest.lines().skip(1) {
            let (case, expected) = line.split_once('\t').expect("manifest row");
            let mut result = valid_result();
            mutate_invalid_aggregate(&mut result, case);
            if expected == "Skipped" {
                let rows = requirement_subject_facts(&result).expect("non-requirement authority");
                assert_eq!(rows.len(), result.source.authorities().len() - 1);
            } else {
                let error = requirement_subject_facts(&result).expect_err("invalid aggregate");
                assert_eq!(format!("{error:?}"), expected, "case {case}");
                assert!(!error.code().is_empty());
            }
        }
    }

    #[test]
    fn requirement_subject_scopes_preserve_the_atom_canonical_order() {
        let mut result = valid_result();
        let authority = result.source.authorities[0].clone();
        let atom_index = result
            .graph
            .atoms
            .iter()
            .position(|atom| atom.meta().id() == authority.atom().id())
            .expect("authority atom");
        let atom = Atom::new(
            authority.atom().id().clone(),
            LifecycleState::Proposed,
            result.graph.atoms[atom_index].atom_kind(),
            vec![
                ScopeId::new("openspec.demo").expect("long scope"),
                ScopeId::new("openspec.z").expect("short scope"),
            ],
            "The system MUST preserve canonical scope order.",
            None,
        )
        .expect("multi-scope atom");
        let expected_scopes = atom
            .scopes()
            .iter()
            .map(|scope| scope.as_str().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(expected_scopes, ["openspec.z", "openspec.demo"]);
        let revision = AtomRevisionRef::new(authority.atom().id().clone(), atom.revision());
        result.graph.atoms[atom_index] = atom;
        result.source.authorities[0] = SourceAuthority::new(authority.node(), revision);

        let rows = requirement_subject_facts(&result).expect("requirement subjects");
        assert_eq!(rows[0].scopes, expected_scopes);
    }

    #[test]
    #[should_panic(expected = "unknown invalid aggregate fixture")]
    fn unknown_invalid_aggregate_fixture_fails_closed() {
        mutate_invalid_aggregate(&mut valid_result(), "unknown");
    }
}

#[cfg(all(test, feature = "coverage-controls"))]
#[allow(clippy::expect_used, clippy::too_many_lines)]
mod coverage_tests {
    use super::*;
    use crate::adapter_sdk::DeclaredSourceRoot;
    use crate::adapter_sdk::MaterializedSourceNode;
    use crate::adapters::AuthorityExpectation;
    use crate::repository::{SnapshotFile, snapshot_repository};
    use buoy_core::{
        ArtifactBindingAttachment, AtomRevisionRef, ImplementationBinding,
        ImplementationBindingRef, NavigationKind, ParsedItemIdentity, SchemaId, SourceEdgeKind,
        SourceItemKind, SourceNode,
    };
    use std::fmt::Write as _;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn bounded_config(graph_objects: u64, diagnostics: u64) -> RepositoryConfig {
        RepositoryConfig::parse(
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
path_depth = 64
directory_entries = 100000
selected_files = 10000
file_bytes = 16777216
total_bytes = 536870912
graph_objects = {graph_objects}
graph_references = 100
diagnostics = {diagnostics}
"#
            )
            .as_bytes(),
        )
        .expect("unit coverage configuration")
    }

    fn markdown_file(name: &str) -> SnapshotFile {
        let path = format!("spec/{name}.md");
        SnapshotFile {
            path: RepositoryPath::new(&path).expect("snapshot path"),
            bytes: include_bytes!("../fixtures/adapters/markdown/positive.md").to_vec(),
            adapter: String::from("markdown/v1"),
            authority_expectation: AuthorityExpectation::Optional,
            selector_root: String::from("spec"),
        }
    }

    #[cfg(feature = "blake3-256")]
    fn binding_repository() -> TempDir {
        let source = Path::new("fixtures/content-addressed-bindings/v1/positive");
        let target = TempDir::new().expect("binding check root");
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

    #[cfg(feature = "blake3-256")]
    fn binding_config(
        root: &Path,
        graph_objects: u64,
        graph_references: u64,
        diagnostics: u64,
        oracle: bool,
    ) -> RepositoryConfig {
        let mut source =
            fs::read_to_string(root.join("buoy.toml")).expect("binding check configuration");
        if oracle {
            source = source.replace(
                "[adapter_configurations]",
                "[[oracles]]\nid = \"ORC-BINDING-BUDGET\"\nprogram = \"builtin:pass\"\ntimeout_ms = 10\n\n[adapter_configurations]",
            );
        }
        write!(
            source,
            "\n[budgets]\npath_depth = 64\ndirectory_entries = 100000\nselected_files = 10000\nfile_bytes = 16777216\ntotal_bytes = 536870912\ngraph_objects = {graph_objects}\ngraph_references = {graph_references}\ndiagnostics = {diagnostics}\nsource_nodes = 200000\nsource_edges = 400000\nsource_depth = 64\nsource_visited = 200000\n"
        )
        .expect("binding budget configuration");
        RepositoryConfig::parse(source.as_bytes()).expect("bounded binding configuration")
    }

    fn artifact_node(path: &str, kind: &str, semantic: &[u8]) -> SourceNode {
        let path = RepositoryPath::new(path).expect("artifact path");
        let item = ParsedItemIdentity::new(
            SchemaId::new("artifact.opaque-bytes/v1").expect("artifact schema"),
            path,
            SourceItemKind::new(kind).expect("artifact kind"),
            semantic.to_vec(),
            None,
        )
        .expect("artifact item");
        SourceNode::new(item, Vec::new()).expect("artifact node")
    }

    fn artifact_binding_control(
        node: &SourceNode,
        artifact_path: &str,
        relation: &str,
    ) -> (ImplementationBinding, ArtifactBindingAttachment) {
        let active = ActiveAlgorithm::build_default().expect("active digest member");
        let requirement = SourceNodeRef::new(
            semantic_digest_with(active, "coverage-requirement", Vec::new())
                .expect("requirement digest"),
        );
        let relation = StableId::new(relation).expect("relation");
        let record = ImplementationBinding::new_with(
            active,
            relation.clone(),
            NavigationKind::Implements,
            node.reference(),
            requirement,
            AtomRevisionRef::new(
                StableId::new("COVERAGE-REQUIREMENT").expect("requirement ID"),
                requirement.digest(),
            ),
        )
        .expect("binding record");
        let attachment = ArtifactBindingAttachment::new_with(
            active,
            relation,
            RepositoryPath::new(artifact_path).expect("attachment artifact path"),
            node.reference(),
            record.record(),
        )
        .expect("artifact attachment");
        (record, attachment)
    }

    fn retained_control(start: u64, semantic: &[u8]) -> MaterializedSourceNode {
        let path = RepositoryPath::new("src/control.rs").expect("control source path");
        let item = ParsedItemIdentity::new(
            SchemaId::new("rust/v1").expect("control schema"),
            path.clone(),
            SourceItemKind::new("rust/item/v1").expect("control source kind"),
            semantic.to_vec(),
            None,
        )
        .expect("control parsed item");
        let node = SourceNode::new(item, Vec::new()).expect("control source node");
        MaterializedSourceNode::new(
            node,
            SourceSpan {
                path,
                start,
                end: start + 1,
            },
            "rust.attribute/v1",
        )
        .expect("retained source control")
    }

    #[test]
    fn source_aggregate_controls_cover_retention_and_binding_failures() {
        let budgets = bounded_config(100, 100).source_budgets();
        let later = retained_control(10, b"fn control() {}");
        let earlier = retained_control(0, b"fn control() {}");
        let different = retained_control(5, b"fn different() {}");
        let later_product = SourceProduct::new(
            vec![later.clone()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("later-span product");
        let earlier_product = SourceProduct::new(
            vec![earlier.clone()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("earlier-span product");
        let mut spans = SourceAccumulator::default();
        spans.absorb(&later_product, budgets).expect("later span");
        spans
            .absorb(&earlier_product, budgets)
            .expect("earlier span");
        spans
            .absorb(&later_product, budgets)
            .expect("noncanonical later span remains ignored");
        assert_eq!(
            spans
                .spans
                .get(&earlier.node().reference())
                .expect("canonical retained span")
                .start,
            0
        );
        let mut collision = SourceAccumulator::default();
        collision
            .nodes
            .insert(earlier.node().digest(), different.node().clone());
        assert_eq!(
            collision.absorb(&earlier_product, budgets),
            Err(CheckError::SourceGraph(
                SourceGraphError::NodeDigestCollision
            ))
        );

        let source = earlier.node().reference();
        let edge = SourceEdge::new(source, SourceEdgeKind::Authority, None, source)
            .expect("reference control");
        let authority = SourceAuthority::new(
            source,
            AtomRevisionRef::new(
                StableId::new("CONTROL-ATOM").expect("control atom ID"),
                source.digest(),
            ),
        );
        let attachment = BindingAttachment::new(
            source,
            ImplementationBindingRef::new(SemanticDigest::xxh3_128([7; 16])),
        );
        for product in [
            SourceProduct::new(
                vec![earlier.clone()],
                vec![edge],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
            .expect("edge product"),
            SourceProduct::new(
                vec![earlier.clone()],
                Vec::new(),
                vec![DeclaredSourceRoot::new(
                    source,
                    SourceRootClass::Navigational,
                )],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
            .expect("root product"),
            SourceProduct::new(
                vec![earlier.clone()],
                Vec::new(),
                Vec::new(),
                vec![authority.clone()],
                Vec::new(),
                Vec::new(),
            )
            .expect("authority product"),
            SourceProduct::new(
                vec![earlier.clone()],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec![attachment],
                Vec::new(),
            )
            .expect("attachment product"),
        ] {
            let mut exhausted = SourceAccumulator {
                reference_count: budgets.edges().get(),
                ..SourceAccumulator::default()
            };
            assert_eq!(
                exhausted.absorb(&product, budgets),
                Err(CheckError::SourceGraph(SourceGraphError::EdgeBudget))
            );
            let mut deduplicated = SourceAccumulator::default();
            deduplicated
                .absorb(&product, budgets)
                .expect("first retained reference");
            deduplicated
                .absorb(&product, budgets)
                .expect("duplicate retained reference");
        }

        let active = ActiveAlgorithm::build_default().expect("active digest member");
        let requirement = SourceNodeRef::new(SemanticDigest::xxh3_128([9; 16]));
        let record = ImplementationBinding::new_with(
            active,
            StableId::new("CONTROL-BINDING").expect("control binding ID"),
            NavigationKind::Implements,
            source,
            requirement,
            AtomRevisionRef::new(
                StableId::new("CONTROL-REQ").expect("control requirement ID"),
                requirement.digest(),
            ),
        )
        .expect("stale binding control");
        let stale_attachment = BindingAttachment::new(source, record.record());
        let store_product = SourceProduct::new(
            vec![earlier.clone()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![StoredBinding::new(record.record(), record.clone())],
        )
        .expect("stored-binding product");
        let mut exhausted = SourceAccumulator {
            reference_count: budgets.edges().get(),
            ..SourceAccumulator::default()
        };
        assert_eq!(
            exhausted.absorb(&store_product, budgets),
            Err(CheckError::SourceGraph(SourceGraphError::EdgeBudget))
        );
        let mut deduplicated = SourceAccumulator::default();
        deduplicated
            .absorb(&store_product, budgets)
            .expect("first stored binding");
        deduplicated
            .absorb(&store_product, budgets)
            .expect("duplicate stored binding");
        let missing_store = SourceProduct::new(
            vec![earlier.clone()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![stale_attachment],
            Vec::new(),
        )
        .expect("missing-store product");
        let missing = aggregate_source_products(active, &[missing_store], budgets)
            .expect("missing-store aggregate");
        assert!(missing.fresh_bindings().is_empty());

        let stale = SourceProduct::new(
            vec![earlier],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![stale_attachment],
            vec![StoredBinding::new(record.record(), record)],
        )
        .expect("stale-binding product");
        let stale =
            aggregate_source_products(active, &[stale], budgets).expect("stale-binding aggregate");
        assert!(stale.fresh_bindings().is_empty());
        assert!(
            stale
                .binding_diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.path.as_deref() == Some("src/control.rs"))
        );
        let unprojected = stale.project_binding_diagnostic(Diagnostic {
            path: None,
            start: 0,
            end: 0,
            code: String::from("COVERAGE-CONTROL"),
            kind: None,
            id: None,
            fields: Vec::new(),
        });
        assert_eq!(unprojected.path, None);
    }

    #[test]
    fn artifact_attachment_controls_cover_aggregate_budgets_resolution_and_human_report() {
        let config = bounded_config(100, 100);
        let budgets = config.source_budgets();
        let node = artifact_node("artifact.bin", "artifact/file/v1", b"artifact");
        let (record, attachment) =
            artifact_binding_control(&node, "artifact.bin", "COVERAGE-ARTIFACT");
        let attachment_product = SourceProduct::from_artifact_attachment(Some(attachment.clone()));
        let mut exhausted = SourceAccumulator {
            reference_count: budgets.edges().get(),
            ..SourceAccumulator::default()
        };
        assert_eq!(
            exhausted.absorb(&attachment_product, budgets),
            Err(CheckError::SourceGraph(SourceGraphError::EdgeBudget))
        );
        let mut deduplicated = SourceAccumulator::default();
        deduplicated
            .absorb(&attachment_product, budgets)
            .expect("first artifact attachment");
        deduplicated
            .absorb(&attachment_product, budgets)
            .expect("duplicate artifact attachment");

        let active = config.active_algorithm();
        let graph = SourceGraph::new_with(
            active,
            vec![node.clone()],
            Vec::new(),
            vec![node.reference()],
            budgets,
        )
        .expect("artifact graph");
        let stored = StoredBinding::new(record.record(), record.clone());
        let (resolved, diagnostics) = resolve_artifact_attachments(
            &graph,
            core::slice::from_ref(&attachment),
            core::slice::from_ref(&stored),
        );
        assert_eq!(resolved.len(), 1);
        assert!(diagnostics.is_empty());

        let (_, duplicate) = resolve_artifact_attachments(
            &graph,
            &[attachment.clone(), attachment.clone()],
            core::slice::from_ref(&stored),
        );
        assert_eq!(duplicate[0].code, "CAB-ATTACHMENT-DUPLICATE");
        let empty = SourceGraph::new_with(active, Vec::new(), Vec::new(), Vec::new(), budgets)
            .expect("empty graph");
        let (_, stale) = resolve_artifact_attachments(
            &empty,
            core::slice::from_ref(&attachment),
            core::slice::from_ref(&stored),
        );
        assert_eq!(stale[0].code, "CAB-SUBJECT-STALE");

        let (_, wrong_path) =
            artifact_binding_control(&node, "different.bin", "COVERAGE-ARTIFACT-PATH");
        let (_, path_diagnostics) = resolve_artifact_attachments(
            &graph,
            core::slice::from_ref(&wrong_path),
            &[StoredBinding::new(
                wrong_path.record(),
                artifact_binding_control(&node, "different.bin", "COVERAGE-ARTIFACT-PATH").0,
            )],
        );
        assert_eq!(path_diagnostics[0].code, "CAB-ATTACHMENT-PATH");

        let wrong_kind_node = artifact_node("artifact.bin", "rust/item/v1", b"artifact");
        let (wrong_kind_record, wrong_kind) =
            artifact_binding_control(&wrong_kind_node, "artifact.bin", "COVERAGE-ARTIFACT-KIND");
        let wrong_kind_graph = SourceGraph::new_with(
            active,
            vec![wrong_kind_node.clone()],
            Vec::new(),
            vec![wrong_kind_node.reference()],
            budgets,
        )
        .expect("wrong-kind graph");
        let (_, kind_diagnostics) = resolve_artifact_attachments(
            &wrong_kind_graph,
            core::slice::from_ref(&wrong_kind),
            &[StoredBinding::new(
                wrong_kind_record.record(),
                wrong_kind_record,
            )],
        );
        assert_eq!(kind_diagnostics[0].code, "CAB-SUBJECT-KIND");

        let mismatched_relation = ArtifactBindingAttachment::new_with(
            active,
            StableId::new("COVERAGE-OTHER-RELATION").expect("other relation"),
            RepositoryPath::new("artifact.bin").expect("artifact path"),
            node.reference(),
            record.record(),
        )
        .expect("relation-mismatched attachment");
        let (_, correspondence) = resolve_artifact_attachments(
            &graph,
            core::slice::from_ref(&mismatched_relation),
            core::slice::from_ref(&stored),
        );
        assert_eq!(correspondence[0].code, "CAB-ATTACHMENT-RECORD");

        let (_, absent) =
            resolve_artifact_attachments(&graph, core::slice::from_ref(&attachment), &[]);
        assert_eq!(absent[0].code, "CAB-RECORD-ABSENT");
        let (_, ambiguous) = resolve_artifact_attachments(
            &graph,
            core::slice::from_ref(&attachment),
            &[stored.clone(), stored],
        );
        assert_eq!(ambiguous[0].code, "CAB-RECORD-AMBIGUOUS");

        let report = MachineReport {
            command: String::from("coverage"),
            graph: semantic_digest_with(active, "coverage-graph", Vec::new())
                .expect("graph digest")
                .canonical_text(),
            diagnostics: Vec::new(),
            migration: MigrationReport::default(),
            verdicts: BTreeMap::new(),
            impacts: Vec::new(),
            mounts: vec![LogicalMountFact {
                id: String::from("project"),
                prefix: String::from("project"),
            }],
            requirement_subjects: Vec::new(),
            opaque_subjects: vec![OpaqueSubjectFact {
                path: String::from("project/artifact.bin"),
                subject: node.digest().canonical_text(),
            }],
            artifact_attachments: vec![ArtifactAttachmentFact {
                relation: String::from("COVERAGE-ARTIFACT"),
                artifact: String::from("project/artifact.bin"),
                subject: node.digest().canonical_text(),
                record: record.record().canonical_text(),
                fresh: true,
            }],
        };
        let human = render_human(&report);
        assert!(human.contains("mount project: project"));
        assert!(human.contains("opaque subject project/artifact.bin"));
        assert!(human.contains("artifact attachment COVERAGE-ARTIFACT"));
    }

    #[cfg(feature = "blake3-256")]
    #[test]
    fn binding_check_controls_cover_post_aggregation_budgets() {
        let objects = binding_repository();
        let object_config = binding_config(objects.path(), 2, 100, 100, true);
        let object_snapshot = snapshot_repository(objects.path(), &object_config)
            .expect("object-budget binding snapshot");
        assert_eq!(
            check_snapshot(&object_snapshot, &object_config),
            Err(CheckError::GraphObjectsBudget)
        );

        let diagnostics = binding_repository();
        let rust_path = diagnostics.path().join("src/lib.rs");
        let stale = "blake3-256:0000000000000000000000000000000000000000000000000000000000000000";
        fs::write(
            &rust_path,
            format!(
                "#[buoy::implements(\"{stale}\")]\npub fn first() {{}}\n#[buoy::implements(\"{stale}\")]\npub fn second() {{}}\n"
            ),
        )
        .expect("stale binding controls");
        let diagnostic_config = binding_config(diagnostics.path(), 100, 100, 1, false);
        let diagnostic_snapshot = snapshot_repository(diagnostics.path(), &diagnostic_config)
            .expect("diagnostic-budget binding snapshot");
        assert_eq!(
            check_snapshot(&diagnostic_snapshot, &diagnostic_config),
            Err(CheckError::DiagnosticsBudget)
        );
    }

    #[test]
    fn unit_variant_covers_check_budget_and_encoding_propagation() {
        let duplicate = RepositorySnapshot {
            files: vec![markdown_file("first"), markdown_file("second")],
        };
        assert_eq!(
            check_snapshot(&duplicate, &bounded_config(1, 100)),
            Err(CheckError::GraphObjectsBudget)
        );
        assert_eq!(
            check_snapshot(&duplicate, &bounded_config(100, 100)),
            Err(CheckError::Encoding)
        );
        assert_eq!(
            check_snapshot_internal(
                &RepositorySnapshot {
                    files: vec![markdown_file("encoding-diagnostic")]
                },
                &bounded_config(100, 100),
                true,
            ),
            Err(CheckError::Encoding)
        );

        let selectors = RepositoryConfig::parse(
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
        .expect("selector diagnostic configuration");
        assert_eq!(
            check_snapshot(&RepositorySnapshot { files: Vec::new() }, &selectors),
            Err(CheckError::DiagnosticsBudget)
        );

        let mut count = 5;
        assert_eq!(
            add_budgeted_count(&mut count, 1, 5, CheckError::GraphObjectsBudget),
            Err(CheckError::GraphObjectsBudget)
        );
        assert_eq!(
            ensure_diagnostic_capacity(usize::MAX, 1, 5),
            Err(CheckError::DiagnosticsBudget)
        );
        assert_eq!(
            project_adapter_error(AdapterSdkError::Identity),
            CheckError::Encoding
        );
    }
}
