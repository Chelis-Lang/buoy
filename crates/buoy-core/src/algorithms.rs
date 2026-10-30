//! Deterministic integrity, freshness, policy, aggregation, and impact algorithms.

use crate::identifier::{SchemaId, SourceItemKind, StableId};
use crate::model::{
    ApprovalBinding, ArtifactBinding, ArtifactIndex, AssuranceClass, AtomRevisionRef,
    BindingAttachment, ChangeDisposition, ChangeDispositionKind, ChangeManifest, ConstructionError,
    CoveragePolicy, DebtLifecycle, DecisionSurface, Derivation, EnforcementMode, EvidenceCarrier,
    EvidenceRole, IdAllocationClass, IdRegistry, ImplementationBinding, ImplementationBindingRef,
    LifecycleState, ObjectKind, ObjectMeta, ObjectRef, ObligationKind, ObligationRef, OracleKind,
    ParsedItemIdentity, PolicyLifecycle, PolicyRatchetImpact, ProofRecord, SourceAuthority,
    SourceGraph, SourceGraphError, SourceMap, SourceNodeRef, SpecGraph, StoredBinding,
    SurfaceDispositionKind, ToolIdentity, TypedPayload, TypedSelector, Verdict, WaiverLifecycle,
    object_reference_value,
};
use crate::value::{
    ActiveAlgorithm, CanonicalValue, DigestAssuranceClass, EncodingError, RepositoryPath,
    SemanticDigest, canonicalize_set, semantic_digest_trusted_with,
};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::{vec, vec::Vec};

/// Lifecycle event accepted by the closed transition relation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleEvent {
    /// Ratify a proposed atom.
    Ratify,
    /// Withdraw a proposed or ratified atom.
    Withdraw,
    /// Supersede a ratified atom.
    Supersede,
}

/// Applies the closed atom lifecycle transition relation.
///
/// # Errors
///
/// Returns [`LifecycleError::IllegalTransition`] when the state/event pair is
/// absent from the closed v2 relation.
pub const fn lifecycle_transition(
    state: LifecycleState,
    event: LifecycleEvent,
) -> Result<LifecycleState, LifecycleError> {
    match (state, event) {
        (LifecycleState::Proposed, LifecycleEvent::Ratify) => Ok(LifecycleState::Ratified),
        (LifecycleState::Proposed | LifecycleState::Ratified, LifecycleEvent::Withdraw) => {
            Ok(LifecycleState::Withdrawn)
        }
        (LifecycleState::Ratified, LifecycleEvent::Supersede) => Ok(LifecycleState::Superseded),
        _ => Err(LifecycleError::IllegalTransition),
    }
}

/// Closed lifecycle failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleError {
    /// The state/event pair is absent from the v2 relation.
    IllegalTransition,
}

/// One stable deterministic diagnostic.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Diagnostic {
    /// Optional canonical repository path.
    pub path: Option<RepositoryPath>,
    /// Inclusive original-byte start.
    pub start: u64,
    /// Exclusive original-byte end.
    pub end: u64,
    /// Stable machine code.
    pub code: String,
    /// Optional primary object kind.
    pub kind: Option<ObjectKind>,
    /// Optional primary stable ID.
    pub id: Option<String>,
    /// Canonical structured detail fields.
    pub fields: Vec<(String, String)>,
}

impl Diagnostic {
    fn global(code: &str) -> Self {
        Self {
            path: None,
            start: 0,
            end: 0,
            code: code.to_string(),
            kind: None,
            id: None,
            fields: Vec::new(),
        }
    }

    fn object(code: &str, kind: ObjectKind, id: &StableId) -> Self {
        Self {
            path: None,
            start: 0,
            end: 0,
            code: code.to_string(),
            kind: Some(kind),
            id: Some(id.as_str().to_string()),
            fields: Vec::new(),
        }
    }
}

/// Closed graph-level review action checked against one approval binding.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApprovalAction {
    /// Ratification of one live atom revision.
    Ratify,
    /// Declaration or revision approval for one semantic object.
    Declare,
    /// Irreversible terminalization approval.
    Terminalize,
    /// Formal prose-to-property correspondence review.
    Correspondence,
    /// Active process-only authorization.
    Authorize,
    /// Base-to-head rereview approval.
    Review,
}

/// One explicit authority, object-kind, and review-action permission.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ApprovalPermission {
    authority: String,
    kind: ObjectKind,
    action: ApprovalAction,
}

impl ApprovalPermission {
    /// Constructs one graph-level approval permission.
    ///
    /// # Errors
    ///
    /// Rejects an authority outside the frozen lowercase scope-ID grammar.
    pub fn new(
        authority: impl Into<String>,
        kind: ObjectKind,
        action: ApprovalAction,
    ) -> Result<Self, GraphValidationContextError> {
        let authority = authority.into();
        if !approval_authority_id(&authority) {
            return Err(GraphValidationContextError::ApprovalAuthorityId);
        }
        Ok(Self {
            authority,
            kind,
            action,
        })
    }

    /// Configured approval authority ID.
    #[must_use]
    pub fn authority(&self) -> &str {
        &self.authority
    }

    /// Authorized object kind.
    #[must_use]
    pub const fn kind(&self) -> ObjectKind {
        self.kind
    }

    /// Authorized graph review action.
    #[must_use]
    pub const fn action(&self) -> ApprovalAction {
        self.action
    }
}

fn approval_authority_id(authority: &str) -> bool {
    CanonicalValue::Text(authority.to_string()).encode().is_ok()
        && authority.split(['.', '-']).all(|component| {
            !component.is_empty()
                && component.as_bytes()[0].is_ascii_lowercase()
                && component
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

/// One explicitly declared graph-validation lane.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum GraphValidationLane {
    /// Stable-ID registry allocation and class.
    Registry = 1,
    /// Approval authority, permission, and review-record currency.
    Approvals = 2,
    /// Source-map object currency and original-byte extents.
    Sources = 4,
    /// Parsed-item enclosure closure and carrier/link item staleness.
    ParsedItems = 8,
    /// Host-schema registration of every typed payload schema.
    HostSchemas = 16,
}

/// The set of graph-validation lanes a caller declares authority for.
///
/// An absent authority snapshot is not evidence of failure: an empty registry
/// is not an unallocated ID, an empty permission set is not an unauthorized
/// approval, and an empty host-schema set is not an unknown schema. A caller
/// therefore declares each lane it has authority for, and every undeclared lane
/// reports nothing rather than diagnosing its own empty input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphValidationLanes(u8);

impl GraphValidationLanes {
    /// Declares every lane.
    #[must_use]
    pub const fn all() -> Self {
        Self::none()
            .with(GraphValidationLane::Registry)
            .with(GraphValidationLane::Approvals)
            .with(GraphValidationLane::Sources)
            .with(GraphValidationLane::ParsedItems)
            .with(GraphValidationLane::HostSchemas)
    }

    /// Declares no lane, leaving only graph-local validation.
    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    /// Adds one declared lane.
    #[must_use]
    pub const fn with(self, lane: GraphValidationLane) -> Self {
        Self(self.0 | lane as u8)
    }

    /// Whether one lane is declared.
    #[must_use]
    pub const fn contains(self, lane: GraphValidationLane) -> bool {
        self.0 & lane as u8 != 0
    }
}

impl Default for GraphValidationLanes {
    fn default() -> Self {
        Self::all()
    }
}

/// Complete explicit inputs for graph-level registry, source, and schema validation.
pub struct GraphValidationContext<'a> {
    registry: &'a IdRegistry,
    source_map: &'a SourceMap,
    artifact_index: &'a ArtifactIndex,
    parsed_items: &'a [ParsedItemIdentity],
    approval_permissions: BTreeSet<ApprovalPermission>,
    source_extents: BTreeMap<RepositoryPath, u64>,
    host_schemas: BTreeSet<String>,
    lanes: GraphValidationLanes,
}

impl<'a> GraphValidationContext<'a> {
    /// Constructs an explicit graph-validation context without ambient repository state.
    ///
    /// # Errors
    ///
    /// Rejects duplicate source paths, duplicate host-schema IDs, or malformed
    /// host-schema IDs before graph validation begins.
    pub fn new(
        registry: &'a IdRegistry,
        source_map: &'a SourceMap,
        artifact_index: &'a ArtifactIndex,
        parsed_items: &'a [ParsedItemIdentity],
        approval_permissions: Vec<ApprovalPermission>,
        source_extents: Vec<(RepositoryPath, u64)>,
        host_schemas: Vec<String>,
    ) -> Result<Self, GraphValidationContextError> {
        let permission_count = approval_permissions.len();
        let approval_permissions = approval_permissions.into_iter().collect::<BTreeSet<_>>();
        if approval_permissions.len() != permission_count {
            return Err(GraphValidationContextError::DuplicateApprovalPermission);
        }
        let mut extents = BTreeMap::new();
        for (path, length) in source_extents {
            if extents.insert(path, length).is_some() {
                return Err(GraphValidationContextError::DuplicateSourcePath);
            }
        }
        let mut schemas = BTreeSet::new();
        for schema in host_schemas {
            if SchemaId::new(&schema).is_err() {
                return Err(GraphValidationContextError::HostSchemaId);
            }
            if !schemas.insert(schema) {
                return Err(GraphValidationContextError::DuplicateHostSchema);
            }
        }
        Ok(Self {
            registry,
            source_map,
            artifact_index,
            parsed_items,
            approval_permissions,
            source_extents: extents,
            host_schemas: schemas,
            lanes: GraphValidationLanes::all(),
        })
    }

    /// Restricts validation to the lanes the caller has authority for.
    #[must_use]
    pub const fn with_lanes(mut self, lanes: GraphValidationLanes) -> Self {
        self.lanes = lanes;
        self
    }

    /// Declared validation lanes.
    #[must_use]
    pub const fn lanes(&self) -> GraphValidationLanes {
        self.lanes
    }
}

/// Deterministic graph-validation-context construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphValidationContextError {
    /// Two authority/kind/action permission rows are identical.
    DuplicateApprovalPermission,
    /// One approval-authority ID is outside the frozen lowercase grammar.
    ApprovalAuthorityId,
    /// Two source extents use the same canonical repository path.
    DuplicateSourcePath,
    /// Two host-schema declarations use the same canonical ID.
    DuplicateHostSchema,
    /// A host-schema declaration is outside the frozen schema-ID grammar.
    HostSchemaId,
}

impl GraphValidationContextError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DuplicateApprovalPermission => "SCF-APPROVAL-ACTION-DUPLICATE",
            Self::ApprovalAuthorityId => "SCF-APPROVAL-AUTHORITY",
            Self::DuplicateSourcePath => "SCF-SOURCE-MAP-DUPLICATE",
            Self::DuplicateHostSchema => "SPG-REF-HOST-SCHEMA-DUPLICATE",
            Self::HostSchemaId => "SPG-REF-HOST-SCHEMA-ID",
        }
    }
}

/// Surface set-comparison result that never relies on row counts alone.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SurfaceCompleteness {
    /// Whether expected and actual keys agree uniquely and all rows are valid.
    pub complete: bool,
    /// Expected keys absent from dispositions.
    pub missing: Vec<TypedPayload>,
    /// Actual keys that occur more than once.
    pub duplicate: Vec<TypedPayload>,
    /// Actual keys outside the domain.
    pub unknown: Vec<TypedPayload>,
    /// Expected members without required live authority.
    pub unauthorized: Vec<TypedPayload>,
}

/// Evaluates complete finite surface membership and authority.
#[must_use]
pub fn surface_completeness(graph: &SpecGraph, surface: &DecisionSurface) -> SurfaceCompleteness {
    let expected = surface
        .expected_members()
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    let mut unauthorized = BTreeSet::new();
    for row in surface.dispositions() {
        let key = row.key().clone();
        actual.insert(key.clone());
        let requires_authority = matches!(
            row.disposition(),
            SurfaceDispositionKind::Normative
                | SurfaceDispositionKind::Prohibited
                | SurfaceDispositionKind::Deferred
        );
        let authority_valid = row
            .authority()
            .is_some_and(|authority| atom_revision_is_live(graph, authority));
        if requires_authority && !authority_valid {
            unauthorized.insert(key);
        }
    }
    let missing = expected.difference(&actual).cloned().collect::<Vec<_>>();
    let unknown = actual.difference(&expected).cloned().collect::<Vec<_>>();
    let unauthorized = unauthorized.into_iter().collect::<Vec<_>>();
    SurfaceCompleteness {
        complete: (
            missing.is_empty(),
            unknown.is_empty(),
            unauthorized.is_empty(),
        ) == (true, true, true),
        missing,
        // `DecisionSurface::new` rejects duplicate member keys.
        duplicate: Vec::new(),
        unknown,
        unauthorized,
    }
}

/// Validates irreversible atom and tombstone history across a graph pair.
#[must_use]
pub fn validate_atom_history(base: &SpecGraph, head: &SpecGraph) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for prior in &base.tombstones {
        let retained = head.tombstones.iter().any(|current| {
            current.meta().id() == prior.meta().id()
                && current.state() == prior.state()
                && current.final_revision() == prior.final_revision()
        });
        let reused = head
            .atoms
            .iter()
            .any(|atom| atom.meta().id() == prior.meta().id());
        if !retained || reused {
            diagnostics.push(Diagnostic::object(
                "SPG-AUTH-TERMINAL-HISTORY",
                ObjectKind::Tombstone,
                prior.meta().id(),
            ));
        }
    }
    for prior in &base.atoms {
        if let Some(current) = head
            .atoms
            .iter()
            .find(|atom| atom.meta().id() == prior.meta().id())
        {
            let legal = matches!(
                (prior.state(), current.state()),
                (
                    LifecycleState::Proposed,
                    LifecycleState::Proposed | LifecycleState::Ratified
                ) | (LifecycleState::Ratified, LifecycleState::Ratified)
            );
            if !legal {
                diagnostics.push(Diagnostic::object(
                    "SPG-AUTH-ILLEGAL-TRANSITION",
                    ObjectKind::Atom,
                    prior.meta().id(),
                ));
            }
        } else {
            let valid_terminal = head.tombstones.iter().any(|tombstone| {
                tombstone.meta().id() == prior.meta().id()
                    && tombstone.final_revision() == prior.revision()
            });
            if !valid_terminal {
                diagnostics.push(Diagnostic::object(
                    "SPG-AUTH-MISSING-TOMBSTONE",
                    ObjectKind::Atom,
                    prior.meta().id(),
                ));
            }
        }
    }
    diagnostics.sort_unstable();
    diagnostics.dedup();
    diagnostics
}

/// Validates append-only registry and irreversible atom/policy/debt/waiver history.
#[must_use]
pub fn validate_graph_history(
    base: &SpecGraph,
    head: &SpecGraph,
    base_registry: &IdRegistry,
    head_registry: &IdRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = validate_atom_history(base, head);
    if let Err(error) = base_registry.validate_successor(head_registry) {
        diagnostics.push(Diagnostic::global(error.code()));
    }
    for prior in &base.coverage_policies {
        let current = head
            .coverage_policies
            .iter()
            .find(|object| object.meta().id() == prior.meta().id());
        let retained = if prior.lifecycle() == PolicyLifecycle::Active {
            current.is_some()
        } else {
            current == Some(prior)
        };
        if !retained {
            diagnostics.push(Diagnostic::object(
                "SCF-LIFECYCLE-TERMINAL-REMOVED",
                ObjectKind::CoveragePolicy,
                prior.meta().id(),
            ));
        }
    }
    for prior in &base.debt_records {
        let current = head
            .debt_records
            .iter()
            .find(|object| object.meta().id() == prior.meta().id());
        let retained = if prior.lifecycle() == DebtLifecycle::Active {
            current.is_some()
        } else {
            current == Some(prior)
        };
        if !retained {
            diagnostics.push(Diagnostic::object(
                "SCF-LIFECYCLE-TERMINAL-REMOVED",
                ObjectKind::DebtRecord,
                prior.meta().id(),
            ));
        }
    }
    for prior in &base.waivers {
        let current = head
            .waivers
            .iter()
            .find(|object| object.meta().id() == prior.meta().id());
        let retained = if prior.lifecycle() == WaiverLifecycle::Active {
            current.is_some()
        } else {
            current == Some(prior)
        };
        if !retained {
            diagnostics.push(Diagnostic::object(
                "SCF-LIFECYCLE-TERMINAL-REMOVED",
                ObjectKind::Waiver,
                prior.meta().id(),
            ));
        }
    }
    diagnostics.sort_unstable();
    diagnostics.dedup();
    diagnostics
}

/// Validates graph uniqueness, lifecycle separation, closure, surfaces, and current bindings.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn validate_graph(graph: &SpecGraph) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut keys = BTreeSet::new();
    for meta in graph.object_meta() {
        let key = (meta.kind(), meta.id().clone());
        if !keys.insert(key) {
            diagnostics.push(Diagnostic::object(
                "DET-DUP-OBJECT-KEY",
                meta.kind(),
                meta.id(),
            ));
        }
    }

    let tombstone_ids = graph
        .tombstones
        .iter()
        .map(|item| item.meta().id())
        .collect::<BTreeSet<_>>();
    for atom in &graph.atoms {
        if tombstone_ids.contains(atom.meta().id()) {
            diagnostics.push(Diagnostic::object(
                "SPG-AUTH-RETIRED-ID-REUSE",
                ObjectKind::Atom,
                atom.meta().id(),
            ));
        }
        // Atom and tombstone lifecycle/approval fields are private and their
        // constructors already enforce these local shapes. Graph validation
        // therefore checks only cross-object lifecycle invariants here.
    }

    for meta in graph.object_meta() {
        for reference in meta.references() {
            if !object_ref_is_current(graph, reference.kind(), reference.id(), reference.digest()) {
                diagnostics.push(Diagnostic::object(
                    "SPG-REF-DANGLING-OR-STALE",
                    meta.kind(),
                    meta.id(),
                ));
            }
        }
    }
    validate_atom_reference_closure(graph, &mut diagnostics);
    validate_carrier_roles(graph, &mut diagnostics);
    for surface in &graph.decision_surfaces {
        if !surface_completeness(graph, surface).complete {
            diagnostics.push(Diagnostic::object(
                "SPG-SURFACE-INCOMPLETE",
                ObjectKind::DecisionSurface,
                surface.meta().id(),
            ));
        }
    }
    for carrier in &graph.evidence_carriers {
        if !carrier_fresh(graph, carrier) {
            diagnostics.push(Diagnostic::object(
                "EVD-FRESH-CARRIER",
                ObjectKind::EvidenceCarrier,
                carrier.meta().id(),
            ));
        }
    }
    for derivation in &graph.derivations {
        if !derivation_fresh(graph, derivation) {
            diagnostics.push(Diagnostic::object(
                "SPG-DERIVATION-STALE",
                ObjectKind::Derivation,
                derivation.meta().id(),
            ));
        }
    }
    for proof in &graph.proof_records {
        if !proof_fresh(graph, proof) {
            diagnostics.push(Diagnostic::object(
                "FPR-BIND-STALE-OR-INCOMPLETE",
                ObjectKind::ProofRecord,
                proof.meta().id(),
            ));
        }
    }
    diagnostics.sort_unstable();
    diagnostics.dedup();
    diagnostics
}

fn validate_atom_references(
    graph: &SpecGraph,
    meta: &ObjectMeta,
    references: &[AtomRevisionRef],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if references
        .iter()
        .any(|reference| !atom_revision_is_live(graph, reference))
    {
        diagnostics.push(Diagnostic::object(
            "SPG-REF-DANGLING-OR-STALE",
            meta.kind(),
            meta.id(),
        ));
    }
}

fn validate_atom_reference_closure(graph: &SpecGraph, diagnostics: &mut Vec<Diagnostic>) {
    for object in &graph.tombstones {
        validate_atom_references(graph, object.meta(), object.replacements(), diagnostics);
    }
    for object in &graph.decision_surfaces {
        if object
            .dispositions()
            .iter()
            .filter_map(|row| row.authority())
            .any(|reference| !atom_revision_is_live(graph, reference))
        {
            diagnostics.push(Diagnostic::object(
                "SPG-REF-DANGLING-OR-STALE",
                object.meta().kind(),
                object.meta().id(),
            ));
        }
    }
    for object in &graph.evidence_carriers {
        validate_atom_references(graph, object.meta(), object.atoms(), diagnostics);
    }
    for object in &graph.implementation_links {
        validate_atom_references(graph, object.meta(), object.atoms(), diagnostics);
    }
    for object in &graph.implementation_bindings {
        validate_atom_references(
            graph,
            object.meta(),
            core::slice::from_ref(object.atom()),
            diagnostics,
        );
    }
    for object in &graph.external_trace_links {
        validate_atom_references(graph, object.meta(), object.atoms(), diagnostics);
    }
    for object in &graph.finite_models {
        validate_atom_references(
            graph,
            object.meta(),
            object.checked_invariants(),
            diagnostics,
        );
    }
    for object in &graph.formal_properties {
        validate_atom_references(graph, object.meta(), object.atoms(), diagnostics);
    }
    for object in &graph.proof_records {
        validate_atom_references(graph, object.meta(), object.atoms(), diagnostics);
    }
}

fn validate_carrier_roles(graph: &SpecGraph, diagnostics: &mut Vec<Diagnostic>) {
    for carrier in &graph.evidence_carriers {
        let expected_oracle = (carrier.oracle().id(), carrier.oracle().digest());
        let oracle = graph
            .oracles
            .iter()
            .find(|oracle| (oracle.meta().id(), oracle.meta().digest()) == expected_oracle);
        let compatible = oracle.is_some_and(|oracle| {
            matches!(
                (carrier.role(), oracle.oracle_kind()),
                (
                    EvidenceRole::Positive | EvidenceRole::Negative,
                    OracleKind::Test
                ) | (EvidenceRole::Property, OracleKind::Property)
                    | (EvidenceRole::ExhaustiveModel, OracleKind::ExhaustiveModel)
                    | (EvidenceRole::FormalProperty, OracleKind::Formal)
                    | (EvidenceRole::Structural, OracleKind::Structural)
            )
        });
        if oracle.is_some() && !compatible {
            diagnostics.push(Diagnostic::object(
                "SPG-REF-ROLE-INCOMPATIBLE",
                ObjectKind::EvidenceCarrier,
                carrier.meta().id(),
            ));
        }
    }
}

/// Validates one semantic graph against explicit registry, source, artifact,
/// parsed-item, and host-schema snapshots.
///
/// Only the lanes the context declares run, so a caller that holds no authority
/// for one lane never receives that lane's diagnostics from an empty snapshot.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn validate_graph_with_context(
    graph: &SpecGraph,
    context: &GraphValidationContext<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = validate_graph(graph);
    if context.lanes.contains(GraphValidationLane::Registry) {
        validate_graph_registry(graph, context.registry, &mut diagnostics);
    }
    if context.lanes.contains(GraphValidationLane::Approvals) {
        validate_graph_approvals(graph, context, &mut diagnostics);
    }
    if context.lanes.contains(GraphValidationLane::Sources) {
        validate_graph_sources(graph, context, &mut diagnostics);
    }
    if context.lanes.contains(GraphValidationLane::ParsedItems) {
        validate_parsed_item_closure(graph, context, &mut diagnostics);
    }
    if context.lanes.contains(GraphValidationLane::HostSchemas) {
        validate_host_schemas(graph, context, &mut diagnostics);
    }
    diagnostics.sort_unstable();
    diagnostics.dedup();
    diagnostics
}

fn expected_registry_class(graph: &SpecGraph, meta: &ObjectMeta) -> IdAllocationClass {
    let terminal = match meta.kind() {
        ObjectKind::Tombstone => true,
        ObjectKind::CoveragePolicy => graph.coverage_policies.iter().any(|object| {
            object.meta().id() == meta.id() && object.lifecycle() != PolicyLifecycle::Active
        }),
        ObjectKind::DebtRecord => graph.debt_records.iter().any(|object| {
            object.meta().id() == meta.id() && object.lifecycle() != DebtLifecycle::Active
        }),
        ObjectKind::Waiver => graph.waivers.iter().any(|object| {
            object.meta().id() == meta.id() && object.lifecycle() != WaiverLifecycle::Active
        }),
        ObjectKind::Atom
        | ObjectKind::DecisionSurface
        | ObjectKind::EvidenceCarrier
        | ObjectKind::ImplementationLink
        | ObjectKind::Derivation
        | ObjectKind::Oracle
        | ObjectKind::FiniteModel
        | ObjectKind::FormalProperty
        | ObjectKind::Assumption
        | ObjectKind::ProofRecord
        | ObjectKind::ParsedItem
        | ObjectKind::Reaffirmation
        | ObjectKind::ChangeManifest
        | ObjectKind::ChangeDisposition
        | ObjectKind::ExternalTraceLink
        | ObjectKind::ImplementationBinding => false,
    };
    if terminal {
        IdAllocationClass::Terminal
    } else {
        IdAllocationClass::Live
    }
}

fn validate_graph_registry(
    graph: &SpecGraph,
    registry: &IdRegistry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for meta in graph.object_meta() {
        let allocation = registry
            .allocations()
            .iter()
            .find(|allocation| allocation.stable_id() == meta.id());
        let Some(allocation) = allocation else {
            diagnostics.push(Diagnostic::object(
                "SCF-REGISTRY-UNALLOCATED",
                meta.kind(),
                meta.id(),
            ));
            continue;
        };
        if allocation.class() != expected_registry_class(graph, meta) {
            diagnostics.push(Diagnostic::object(
                "SCF-REGISTRY-CLASS",
                meta.kind(),
                meta.id(),
            ));
        }
    }
}

fn approval_artifact_is_current(
    approval: &ApprovalBinding,
    artifact_index: &ArtifactIndex,
) -> bool {
    let expected = (approval.record(), approval.digest());
    artifact_index
        .entries()
        .iter()
        .any(|entry| (entry.path(), entry.raw_digest()) == expected)
}

fn validate_approval(
    meta: &ObjectMeta,
    approval: &ApprovalBinding,
    action: ApprovalAction,
    context: &GraphValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let namespace = context
        .registry
        .allocations()
        .iter()
        .find(|allocation| allocation.stable_id() == meta.id())
        .and_then(|allocation| {
            context
                .registry
                .namespaces()
                .iter()
                .find(|namespace| namespace.namespace_id() == allocation.namespace_id())
        });
    if !namespace.is_some_and(|namespace| {
        namespace
            .approval_authorities()
            .iter()
            .any(|authority| authority.as_str() == approval.authority())
    }) {
        diagnostics.push(Diagnostic::object(
            "SCF-APPROVAL-AUTHORITY",
            meta.kind(),
            meta.id(),
        ));
    }
    if !context.approval_permissions.contains(&ApprovalPermission {
        authority: approval.authority().to_string(),
        kind: meta.kind(),
        action,
    }) {
        diagnostics.push(Diagnostic::object(
            "SCF-APPROVAL-ACTION",
            meta.kind(),
            meta.id(),
        ));
    }
    if !approval_artifact_is_current(approval, context.artifact_index) {
        diagnostics.push(Diagnostic::object(
            "SCF-APPROVAL-STALE",
            meta.kind(),
            meta.id(),
        ));
    }
}

fn validate_graph_approvals(
    graph: &SpecGraph,
    context: &GraphValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for object in &graph.atoms {
        if let Some(approval) = object.approval() {
            validate_approval(
                object.meta(),
                approval,
                ApprovalAction::Ratify,
                context,
                diagnostics,
            );
        }
    }
    for object in &graph.tombstones {
        validate_approval(
            object.meta(),
            object.approval(),
            ApprovalAction::Terminalize,
            context,
            diagnostics,
        );
    }
    for object in &graph.coverage_policies {
        validate_approval(
            object.meta(),
            object.active_approval(),
            ApprovalAction::Declare,
            context,
            diagnostics,
        );
        if let Some(approval) = object.terminal_approval() {
            validate_approval(
                object.meta(),
                approval,
                ApprovalAction::Terminalize,
                context,
                diagnostics,
            );
        }
    }
    for object in &graph.debt_records {
        if let Some(approval) = object.terminal_approval() {
            validate_approval(
                object.meta(),
                approval,
                ApprovalAction::Terminalize,
                context,
                diagnostics,
            );
        }
    }
    for object in &graph.waivers {
        validate_approval(
            object.meta(),
            object.active_approval(),
            ApprovalAction::Authorize,
            context,
            diagnostics,
        );
        if let Some(approval) = object.terminal_approval() {
            validate_approval(
                object.meta(),
                approval,
                ApprovalAction::Terminalize,
                context,
                diagnostics,
            );
        }
    }
    for object in &graph.assumptions {
        validate_approval(
            object.meta(),
            object.approval(),
            ApprovalAction::Declare,
            context,
            diagnostics,
        );
    }
    for object in &graph.formal_properties {
        validate_approval(
            object.meta(),
            object.correspondence_approval(),
            ApprovalAction::Correspondence,
            context,
            diagnostics,
        );
    }
    for object in &graph.reaffirmations {
        validate_approval(
            object.meta(),
            object.review_approval(),
            ApprovalAction::Review,
            context,
            diagnostics,
        );
    }
}

fn validate_graph_sources(
    graph: &SpecGraph,
    context: &GraphValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for entry in context.source_map.entries() {
        let object = entry.object();
        if !object_ref_is_current(graph, object.kind(), object.id(), object.digest()) {
            diagnostics.push(Diagnostic::object(
                "SCF-SOURCE-MAP-DANGLING",
                object.kind(),
                object.id(),
            ));
        }
        for source in entry.sources() {
            if context
                .source_extents
                .get(source.path())
                .is_none_or(|length| source.end() > *length)
            {
                let mut diagnostic =
                    Diagnostic::object("SCF-SOURCE-MAP-SPAN", object.kind(), object.id());
                diagnostic.path = Some(source.path().clone());
                diagnostic.start = source.start();
                diagnostic.end = source.end();
                diagnostics.push(diagnostic);
            }
        }
    }
}

fn validate_parsed_item_closure(
    graph: &SpecGraph,
    context: &GraphValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let parsed_digests = context
        .parsed_items
        .iter()
        .map(ParsedItemIdentity::digest)
        .collect::<BTreeSet<_>>();
    for item in context.parsed_items {
        if item
            .enclosing_item()
            .is_some_and(|digest| !parsed_digests.contains(&digest))
        {
            diagnostics.push(Diagnostic::global("SCF-SOURCE-MAP-ENCLOSING"));
        }
    }
    for carrier in &graph.evidence_carriers {
        if !parsed_digests.contains(&carrier.parsed_item_digest()) {
            diagnostics.push(Diagnostic::object(
                "SCF-SOURCE-MAP-ITEM-STALE",
                ObjectKind::EvidenceCarrier,
                carrier.meta().id(),
            ));
        }
    }
    for link in &graph.implementation_links {
        if !parsed_digests.contains(&link.parsed_item_digest()) {
            diagnostics.push(Diagnostic::object(
                "SCF-SOURCE-MAP-ITEM-STALE",
                ObjectKind::ImplementationLink,
                link.meta().id(),
            ));
        }
    }
}

fn validate_schema(
    meta: &ObjectMeta,
    schema: &str,
    context: &GraphValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !context.host_schemas.contains(schema) {
        diagnostics.push(Diagnostic::object(
            "SPG-REF-HOST-SCHEMA",
            meta.kind(),
            meta.id(),
        ));
    }
}

fn validate_payload(
    meta: &ObjectMeta,
    payload: &TypedPayload,
    context: &GraphValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_schema(meta, payload.schema(), context, diagnostics);
}

fn validate_host_schemas(
    graph: &SpecGraph,
    context: &GraphValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for surface in &graph.decision_surfaces {
        let meta = surface.meta();
        validate_payload(
            meta,
            surface.domain().member_key_schema(),
            context,
            diagnostics,
        );
        validate_payload(
            meta,
            surface.domain().disposition_vocabulary(),
            context,
            diagnostics,
        );
        for payload in surface.domain().member_keys() {
            validate_payload(meta, payload, context, diagnostics);
        }
        for row in surface.dispositions() {
            validate_payload(meta, row.key(), context, diagnostics);
            if let Some(payload) = row.payload() {
                validate_payload(meta, payload, context, diagnostics);
            }
        }
    }
    for oracle in &graph.oracles {
        validate_payload(oracle.meta(), oracle.selection(), context, diagnostics);
        validate_payload(oracle.meta(), oracle.normalization(), context, diagnostics);
        validate_payload(oracle.meta(), oracle.scope(), context, diagnostics);
    }
    for carrier in &graph.evidence_carriers {
        validate_payload(carrier.meta(), carrier.scope(), context, diagnostics);
    }
    for derivation in &graph.derivations {
        validate_payload(
            derivation.meta(),
            derivation.relation_scope(),
            context,
            diagnostics,
        );
    }
    for assumption in &graph.assumptions {
        validate_payload(assumption.meta(), assumption.scope(), context, diagnostics);
    }
    for model in &graph.finite_models {
        validate_payload(model.meta(), model.state_space(), context, diagnostics);
        validate_payload(model.meta(), model.event_alphabet(), context, diagnostics);
        validate_payload(
            model.meta(),
            model.transition_relation(),
            context,
            diagnostics,
        );
        for state in model.initial_states() {
            validate_payload(model.meta(), state, context, diagnostics);
        }
        validate_payload(model.meta(), model.bound(), context, diagnostics);
    }
    for property in &graph.formal_properties {
        validate_payload(property.meta(), property.property(), context, diagnostics);
        validate_payload(
            property.meta(),
            property.implementation_abstraction(),
            context,
            diagnostics,
        );
    }
    for policy in &graph.coverage_policies {
        let revision = policy.revision_value();
        validate_payload(policy.meta(), revision.scope(), context, diagnostics);
        if let Some(requirements) = revision.derivation_requirements() {
            validate_payload(policy.meta(), requirements, context, diagnostics);
        }
        for selector in revision.authority_selectors() {
            validate_schema(policy.meta(), selector.adapter_id(), context, diagnostics);
        }
    }
    for debt in &graph.debt_records {
        validate_payload(
            debt.meta(),
            debt.discharge_condition(),
            context,
            diagnostics,
        );
    }
    for waiver in &graph.waivers {
        validate_payload(
            waiver.meta(),
            waiver.discharge_condition(),
            context,
            diagnostics,
        );
    }
    for proof in &graph.proof_records {
        validate_payload(proof.meta(), proof.scope(), context, diagnostics);
    }
    validate_context_host_schemas(context, diagnostics);
}

fn validate_context_host_schemas(
    context: &GraphValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for namespace in context.registry.namespaces() {
        if !context
            .host_schemas
            .contains(namespace.allocation_policy().schema())
        {
            diagnostics.push(Diagnostic::global("SPG-REF-HOST-SCHEMA"));
        }
    }
    for item in context.parsed_items {
        if !context.host_schemas.contains(item.language_schema_id()) {
            diagnostics.push(Diagnostic::global("SPG-REF-HOST-SCHEMA"));
        }
    }
}

fn atom_revision_is_live(graph: &SpecGraph, reference: &AtomRevisionRef) -> bool {
    let expected = (reference.id(), reference.revision());
    graph
        .atoms
        .iter()
        .any(|atom| (atom.meta().id(), atom.revision()) == expected)
}

fn object_ref_is_current(
    graph: &SpecGraph,
    kind: ObjectKind,
    id: &StableId,
    digest: SemanticDigest,
) -> bool {
    let expected = (kind, id, digest);
    graph
        .object_meta()
        .iter()
        .any(|meta| (meta.kind(), meta.id(), meta.digest()) == expected)
}

/// Returns whether every exact atom and oracle binding of a carrier is current.
#[must_use]
pub fn carrier_fresh(graph: &SpecGraph, carrier: &EvidenceCarrier) -> bool {
    carrier
        .atoms()
        .iter()
        .all(|reference| atom_revision_is_live(graph, reference))
        && object_ref_is_current(
            graph,
            carrier.oracle().kind(),
            carrier.oracle().id(),
            carrier.oracle().digest(),
        )
}

/// Returns whether every derivation binding and oracle reference is current.
///
/// Repository-artifact bindings fail closed because this convenience form has
/// no explicit artifact index. Use [`derivation_fresh_with_artifacts`] when a
/// selected snapshot contains repository-artifact bindings.
#[must_use]
pub fn derivation_fresh(graph: &SpecGraph, derivation: &Derivation) -> bool {
    derivation_fresh_with_artifacts(graph, derivation, &ArtifactIndex::default())
}

/// Evaluates derivation freshness against one explicit current artifact index.
#[must_use]
pub fn derivation_fresh_with_artifacts(
    graph: &SpecGraph,
    derivation: &Derivation,
    artifacts: &ArtifactIndex,
) -> bool {
    derivation
        .sources()
        .iter()
        .chain(derivation.targets())
        .all(|binding| artifact_binding_current(graph, artifacts, binding))
        && object_ref_is_current(
            graph,
            derivation.oracle().kind(),
            derivation.oracle().id(),
            derivation.oracle().digest(),
        )
}

fn artifact_binding_current(
    graph: &SpecGraph,
    artifacts: &ArtifactIndex,
    binding: &ArtifactBinding,
) -> bool {
    match binding {
        ArtifactBinding::Object(reference) => {
            object_ref_is_current(graph, reference.kind(), reference.id(), reference.digest())
        }
        ArtifactBinding::RepositoryArtifact(binding) => artifacts.contains_binding(binding),
    }
}

/// Returns whether a proof is complete, current, approved, and accepted.
///
/// Repository-artifact bindings fail closed because this convenience form has
/// no explicit artifact index. Use [`proof_fresh_with_artifacts`] for selected
/// snapshot artifacts.
#[must_use]
pub fn proof_fresh(graph: &SpecGraph, proof: &ProofRecord) -> bool {
    proof_fresh_with_artifacts(graph, proof, &ArtifactIndex::default())
}

/// Evaluates proof freshness against one explicit current artifact index.
#[must_use]
pub fn proof_fresh_with_artifacts(
    graph: &SpecGraph,
    proof: &ProofRecord,
    artifacts: &ArtifactIndex,
) -> bool {
    let property_current = object_ref_is_current(
        graph,
        proof.property().kind(),
        proof.property().id(),
        proof.property().digest(),
    );
    proof
        .atoms()
        .iter()
        .all(|reference| atom_revision_is_live(graph, reference))
        && property_current
        && artifact_binding_current(graph, artifacts, proof.implementation())
        && proof.assumptions().iter().all(|reference| {
            object_ref_is_current(graph, reference.kind(), reference.id(), reference.digest())
        })
        && proof
            .external_models()
            .iter()
            .chain(proof.generated())
            .chain(proof.proof_sources())
            .all(|binding| artifact_binding_current(graph, artifacts, binding))
        && proof.verdict() == Verdict::Pass
}

/// Current non-object proof bindings that cannot be inferred from a graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofExpectation {
    proof_id: StableId,
    verifier_configuration: SemanticDigest,
    scope: TypedPayload,
    exclusions: Vec<String>,
    correspondence_derivation_current: bool,
    negative_control_satisfied: bool,
}

impl ProofExpectation {
    /// Constructs one exact external proof expectation.
    #[must_use]
    #[allow(clippy::struct_excessive_bools)]
    pub fn new(
        proof_id: StableId,
        verifier_configuration: SemanticDigest,
        scope: TypedPayload,
        exclusions: Vec<String>,
        correspondence_derivation_current: bool,
        negative_control_satisfied: bool,
    ) -> Self {
        let mut exclusions = exclusions;
        exclusions.sort_unstable();
        exclusions.dedup();
        Self {
            proof_id,
            verifier_configuration,
            scope,
            exclusions,
            correspondence_derivation_current,
            negative_control_satisfied,
        }
    }
}

/// Explicit current tool/artifact/expectation snapshot for proof invalidation.
pub struct ProofInvalidationContext<'a> {
    artifacts: &'a ArtifactIndex,
    current_tools: BTreeSet<ToolIdentity>,
    expectations: BTreeMap<StableId, ProofExpectation>,
}

impl<'a> ProofInvalidationContext<'a> {
    /// Constructs one closed proof-invalidation snapshot.
    ///
    /// # Errors
    ///
    /// Rejects duplicate current tools or duplicate proof expectations.
    pub fn new(
        artifacts: &'a ArtifactIndex,
        current_tools: Vec<ToolIdentity>,
        expectations: Vec<ProofExpectation>,
    ) -> Result<Self, ProofInvalidationContextError> {
        let tool_count = current_tools.len();
        let current_tools = current_tools.into_iter().collect::<BTreeSet<_>>();
        if current_tools.len() != tool_count {
            return Err(ProofInvalidationContextError::DuplicateTool);
        }
        let expectation_count = expectations.len();
        let expectations = expectations
            .into_iter()
            .map(|expectation| (expectation.proof_id.clone(), expectation))
            .collect::<BTreeMap<_, _>>();
        if expectations.len() != expectation_count {
            return Err(ProofInvalidationContextError::DuplicateExpectation);
        }
        Ok(Self {
            artifacts,
            current_tools,
            expectations,
        })
    }
}

/// Closed proof-invalidation-context construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofInvalidationContextError {
    /// Two current tool rows are identical.
    DuplicateTool,
    /// Two expectation rows address the same proof ID.
    DuplicateExpectation,
}

/// Returns every independently stale proof dimension in canonical code order.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn proof_invalidation_reasons(
    graph: &SpecGraph,
    proof: &ProofRecord,
    context: &ProofInvalidationContext<'_>,
) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    if !proof
        .atoms()
        .iter()
        .all(|reference| atom_revision_is_live(graph, reference))
    {
        reasons.insert(String::from("FPR-BIND-ATOM"));
    }
    let property_current = object_ref_is_current(
        graph,
        proof.property().kind(),
        proof.property().id(),
        proof.property().digest(),
    );
    if !property_current {
        reasons.insert(String::from("FPR-BIND-PROPERTY"));
    }
    if !artifact_binding_current(graph, context.artifacts, proof.implementation()) {
        reasons.insert(String::from("FPR-BIND-IMPLEMENTATION"));
    }
    if !proof.assumptions().iter().all(|reference| {
        object_ref_is_current(graph, reference.kind(), reference.id(), reference.digest())
    }) {
        reasons.insert(String::from("FPR-BIND-ASSUMPTION"));
    }
    if !proof
        .translators()
        .iter()
        .all(|tool| context.current_tools.contains(tool))
    {
        reasons.insert(String::from("FPR-BIND-TRANSLATOR"));
    }
    if !context.current_tools.contains(proof.verifier()) {
        reasons.insert(String::from("FPR-BIND-VERIFIER"));
    }
    if !proof
        .external_models()
        .iter()
        .all(|binding| artifact_binding_current(graph, context.artifacts, binding))
    {
        reasons.insert(String::from("FPR-BIND-EXTERNAL-MODEL"));
    }
    if !proof
        .generated()
        .iter()
        .all(|binding| artifact_binding_current(graph, context.artifacts, binding))
    {
        reasons.insert(String::from("FPR-BIND-GENERATED"));
    }
    if !proof
        .proof_sources()
        .iter()
        .all(|binding| artifact_binding_current(graph, context.artifacts, binding))
    {
        reasons.insert(String::from("FPR-BIND-PROOF-SOURCE"));
    }
    if proof.verdict() != Verdict::Pass {
        reasons.insert(String::from("FPR-VERDICT-NONPASS"));
    }
    let Some(expectation) = context.expectations.get(proof.meta().id()) else {
        reasons.insert(String::from("FPR-BIND-EXPECTATION"));
        return reasons.into_iter().collect();
    };
    if proof.verifier_configuration() != expectation.verifier_configuration {
        reasons.insert(String::from("FPR-BIND-VERIFIER-CONFIGURATION"));
    }
    if proof.scope() != &expectation.scope {
        reasons.insert(String::from("FPR-BIND-SCOPE"));
    }
    if proof.exclusions() != expectation.exclusions {
        reasons.insert(String::from("FPR-BIND-EXCLUSIONS"));
    }
    if !expectation.correspondence_derivation_current {
        reasons.insert(String::from("FPR-BIND-CORRESPONDENCE"));
    }
    if !expectation.negative_control_satisfied {
        reasons.insert(String::from("FPR-VERDICT-NEGATIVE-CONTROL"));
    }
    reasons.into_iter().collect()
}

/// Evaluates all sixteen frozen proof invalidation dimensions.
#[must_use]
pub fn proof_fresh_with_context(
    graph: &SpecGraph,
    proof: &ProofRecord,
    context: &ProofInvalidationContext<'_>,
) -> bool {
    proof_invalidation_reasons(graph, proof, context).is_empty()
}

/// Required-role coverage result for one policy.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RoleCoverage {
    /// Required roles with at least one fresh matching carrier.
    pub present: Vec<EvidenceRole>,
    /// Required roles without a fresh matching carrier.
    pub missing: Vec<EvidenceRole>,
}

/// Evaluates explicit required roles without name inference.
#[must_use]
pub fn required_role_coverage(graph: &SpecGraph, policy: &CoveragePolicy) -> RoleCoverage {
    let subject_ids = policy
        .subjects()
        .iter()
        .filter(|reference| reference.kind() == ObjectKind::Atom)
        .map(crate::model::ObjectRef::id)
        .collect::<BTreeSet<_>>();
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for role in policy.roles() {
        let found = graph.evidence_carriers.iter().any(|carrier| {
            carrier.role() == *role
                && carrier_fresh(graph, carrier)
                && carrier
                    .atoms()
                    .iter()
                    .any(|reference| subject_ids.contains(reference.id()))
        });
        if found {
            present.push(*role);
        } else {
            missing.push(*role);
        }
    }
    present.sort_unstable();
    present.dedup();
    missing.sort_unstable();
    missing.dedup();
    RoleCoverage { present, missing }
}

/// Base-to-head policy ratchet classification.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PolicyRatchet {
    /// Subject IDs removed from materialized scope.
    pub removed_subjects: Vec<(ObjectKind, StableId)>,
    /// Required roles removed.
    pub removed_roles: Vec<EvidenceRole>,
    /// Whether blocking enforcement became advisory.
    pub mode_weakened: bool,
    /// Whether lineage identity changed.
    pub lineage_changed: bool,
}

impl PolicyRatchet {
    /// Returns true if any weakening protocol is required.
    #[must_use]
    pub fn weakened(&self) -> bool {
        !self.removed_subjects.is_empty()
            || !self.removed_roles.is_empty()
            || self.mode_weakened
            || self.lineage_changed
    }
}

/// Compares two active policy revisions by finite materialization.
#[must_use]
pub fn compare_policy(base: &CoveragePolicy, head: &CoveragePolicy) -> PolicyRatchet {
    let head_subjects = head
        .subjects()
        .iter()
        .map(|reference| (reference.kind(), reference.id()))
        .collect::<BTreeSet<_>>();
    let mut removed_subjects = base
        .subjects()
        .iter()
        .filter(|reference| !head_subjects.contains(&(reference.kind(), reference.id())))
        .map(|reference| (reference.kind(), reference.id().clone()))
        .collect::<Vec<_>>();
    removed_subjects.sort_unstable();
    let head_roles = head.roles().iter().copied().collect::<BTreeSet<_>>();
    let mut removed_roles = base
        .roles()
        .iter()
        .filter(|role| !head_roles.contains(role))
        .copied()
        .collect::<Vec<_>>();
    removed_roles.sort_unstable();
    PolicyRatchet {
        removed_subjects,
        removed_roles,
        mode_weakened: base.mode() == EnforcementMode::Blocking
            && head.mode() == EnforcementMode::Advisory,
        lineage_changed: base.ratchet() != head.ratchet(),
    }
}

/// Closed graph-derived policy change classification.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PolicyChangeClassification {
    /// Exact obligations and policy dimensions are unchanged.
    Equivalent,
    /// Obligations or constraints were added without weakening another dimension.
    Strengthening,
    /// Irreversible retirement or covering supersession retains reviewed history.
    EquivalentReviewed,
    /// At least one prior obligation or constraint was weakened.
    Weakening,
}

/// One complete graph-derived policy comparison and its exact obligations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedPolicyRatchet {
    policy_id: StableId,
    base_policy: Option<ObjectRef>,
    head_policy: Option<ObjectRef>,
    base_obligations: Vec<ObligationRef>,
    head_obligations: Vec<ObligationRef>,
    removed_obligations: Vec<ObligationRef>,
    added_obligations: Vec<ObligationRef>,
    classification: PolicyChangeClassification,
    diagnostics: Vec<String>,
    impact: Option<PolicyRatchetImpact>,
}

impl DerivedPolicyRatchet {
    /// Stable policy identity compared across the graph pair.
    #[must_use]
    pub const fn policy_id(&self) -> &StableId {
        &self.policy_id
    }

    /// Exact base-side policy binding when present.
    #[must_use]
    pub const fn base_policy(&self) -> Option<&ObjectRef> {
        self.base_policy.as_ref()
    }

    /// Exact head-side policy binding when present.
    #[must_use]
    pub const fn head_policy(&self) -> Option<&ObjectRef> {
        self.head_policy.as_ref()
    }

    /// Canonical base-side materialized obligations.
    #[must_use]
    pub fn base_obligations(&self) -> &[ObligationRef] {
        &self.base_obligations
    }

    /// Canonical head-side materialized obligations.
    #[must_use]
    pub fn head_obligations(&self) -> &[ObligationRef] {
        &self.head_obligations
    }

    /// Base obligations absent from the head graph.
    #[must_use]
    pub fn removed_obligations(&self) -> &[ObligationRef] {
        &self.removed_obligations
    }

    /// Head obligations absent from the base graph.
    #[must_use]
    pub fn added_obligations(&self) -> &[ObligationRef] {
        &self.added_obligations
    }

    /// Derived change class; manifests cannot override this value.
    #[must_use]
    pub const fn classification(&self) -> PolicyChangeClassification {
        self.classification
    }

    /// Canonically ordered weakening diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    /// Frozen canonical ratchet impact for a changed policy present on both sides.
    #[must_use]
    pub const fn impact(&self) -> Option<&PolicyRatchetImpact> {
        self.impact.as_ref()
    }
}

/// Deterministic graph-derived policy analysis failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyAnalysisError {
    /// Two policy objects use the same stable ID on one graph side.
    DuplicatePolicy,
    /// A materialized subject does not resolve to an exact current graph object.
    DanglingSubject,
    /// A derived exact reference or obligation failed frozen construction.
    Construction(ConstructionError),
}

impl PolicyAnalysisError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DuplicatePolicy => "CHG-RATCHET-POLICY-DUPLICATE",
            Self::DanglingSubject => "CHG-RATCHET-SCOPE-DANGLING",
            Self::Construction(_) => "CHG-RATCHET-MATERIALIZATION",
        }
    }
}

impl From<ConstructionError> for PolicyAnalysisError {
    fn from(value: ConstructionError) -> Self {
        Self::Construction(value)
    }
}

type ObligationIdentity = (ObligationKind, ObjectKind, StableId, Option<EvidenceRole>);

fn obligation_identity(obligation: &ObligationRef) -> ObligationIdentity {
    (
        obligation.kind(),
        obligation.subject().kind(),
        obligation.subject().id().clone(),
        obligation.required_role(),
    )
}

fn current_object_reference(
    graph: &SpecGraph,
    kind: ObjectKind,
    id: &StableId,
) -> Result<ObjectRef, PolicyAnalysisError> {
    let meta = graph
        .object_meta()
        .into_iter()
        .find(|meta| meta.kind() == kind && meta.id() == id)
        .ok_or(PolicyAnalysisError::DanglingSubject)?;
    Ok(ObjectRef::new(kind, id.clone(), meta.digest()))
}

/// Materializes every exact obligation implied by one policy in one graph.
///
/// # Errors
///
/// Rejects dangling materialized subjects or impossible derived obligation shapes.
pub fn materialize_policy_obligations(
    graph: &SpecGraph,
    policy: &CoveragePolicy,
) -> Result<Vec<ObligationRef>, PolicyAnalysisError> {
    let mut obligations = BTreeSet::new();
    let policy_subject = exact_policy_reference(policy);
    if !policy.revision_value().authority_selectors().is_empty() {
        obligations.insert(ObligationRef::for_policy(
            policy,
            ObligationKind::Authority,
            policy_subject.clone(),
            None,
        ));
    }
    if policy.revision_value().derivation_requirements().is_some() {
        obligations.insert(ObligationRef::for_policy(
            policy,
            ObligationKind::DerivationConsistency,
            policy_subject,
            None,
        ));
    }
    for retained_subject in policy.subjects() {
        let subject =
            current_object_reference(graph, retained_subject.kind(), retained_subject.id())?;
        obligations.insert(ObligationRef::for_policy(
            policy,
            ObligationKind::Freshness,
            subject.clone(),
            None,
        ));
        if subject.kind() == ObjectKind::DecisionSurface {
            obligations.insert(ObligationRef::for_policy(
                policy,
                ObligationKind::SurfaceCompleteness,
                subject.clone(),
                None,
            ));
            if !policy.revision_value().authority_selectors().is_empty() {
                obligations.insert(ObligationRef::for_policy(
                    policy,
                    ObligationKind::SurfaceAuthority,
                    subject.clone(),
                    None,
                ));
            }
        }
        for role in policy.roles() {
            obligations.insert(ObligationRef::for_policy(
                policy,
                ObligationKind::RequiredRole,
                subject.clone(),
                Some(*role),
            ));
        }
    }
    Ok(obligations.into_iter().collect())
}

fn policy_map(
    graph: &SpecGraph,
) -> Result<BTreeMap<StableId, &CoveragePolicy>, PolicyAnalysisError> {
    let mut policies = BTreeMap::new();
    for policy in &graph.coverage_policies {
        if policies
            .insert(policy.meta().id().clone(), policy)
            .is_some()
        {
            return Err(PolicyAnalysisError::DuplicatePolicy);
        }
    }
    Ok(policies)
}

fn exact_policy_reference(policy: &CoveragePolicy) -> ObjectRef {
    ObjectRef::new(
        ObjectKind::CoveragePolicy,
        policy.meta().id().clone(),
        policy.meta().digest(),
    )
}

fn obligation_map(obligations: &[ObligationRef]) -> BTreeMap<ObligationIdentity, ObligationRef> {
    obligations
        .iter()
        .cloned()
        .map(|obligation| (obligation_identity(&obligation), obligation))
        .collect()
}

fn subject_keys(policy: &CoveragePolicy) -> BTreeSet<(ObjectKind, StableId)> {
    policy
        .subjects()
        .iter()
        .map(|subject| (subject.kind(), subject.id().clone()))
        .collect()
}

fn surface_members(
    graph: &SpecGraph,
    policy: &CoveragePolicy,
) -> BTreeSet<(StableId, TypedPayload)> {
    let subjects = subject_keys(policy);
    graph
        .decision_surfaces
        .iter()
        .filter(|surface| {
            subjects.contains(&(ObjectKind::DecisionSurface, surface.meta().id().clone()))
        })
        .flat_map(|surface| {
            surface
                .domain()
                .member_keys()
                .iter()
                .cloned()
                .map(|member| (surface.meta().id().clone(), member))
        })
        .collect()
}

fn accepted_waiver_classes(graph: &SpecGraph, policy_id: &StableId) -> BTreeSet<TypedPayload> {
    graph
        .waivers
        .iter()
        .filter(|waiver| {
            let applies = waiver
                .obligations()
                .iter()
                .any(|obligation| obligation.policy_id() == policy_id);
            (waiver.lifecycle(), applies) == (WaiverLifecycle::Active, true)
        })
        .map(|waiver| waiver.discharge_condition().clone())
        .collect()
}

fn replacement_covers(
    graph: &SpecGraph,
    policy: &CoveragePolicy,
    required: &BTreeSet<ObligationIdentity>,
) -> Result<bool, PolicyAnalysisError> {
    let mut replacement_obligations = BTreeSet::new();
    for replacement in policy.replacements() {
        let expected = (
            replacement.id(),
            replacement.digest(),
            PolicyLifecycle::Active,
        );
        let Some(replacement_policy) = graph.coverage_policies.iter().find(|candidate| {
            (
                candidate.meta().id(),
                candidate.meta().digest(),
                candidate.lifecycle(),
            ) == expected
        }) else {
            return Ok(false);
        };
        replacement_obligations.extend(
            materialize_policy_obligations(graph, replacement_policy)?
                .iter()
                .map(obligation_identity),
        );
    }
    // Superseded policies are constructor-bound to a nonempty replacement set.
    Ok(required.is_subset(&replacement_obligations))
}

/// Derives complete obligation sets and policy ratchet classes from a graph pair.
///
/// # Errors
///
/// Rejects duplicate policy IDs, dangling scope materializations, and impossible
/// exact derived references or obligations.
#[allow(clippy::too_many_lines)]
pub fn derive_policy_ratchets(
    base: &SpecGraph,
    head: &SpecGraph,
) -> Result<Vec<DerivedPolicyRatchet>, PolicyAnalysisError> {
    let base_policies = policy_map(base)?;
    let head_policies = policy_map(head)?;
    let policy_ids = base_policies
        .keys()
        .chain(head_policies.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut derived = Vec::new();
    for policy_id in policy_ids {
        let base_policy = base_policies.get(&policy_id).copied();
        let head_policy = head_policies.get(&policy_id).copied();
        let base_obligations = base_policy.map_or(Ok(Vec::new()), |policy| {
            materialize_policy_obligations(base, policy)
        })?;
        let head_obligations = head_policy.map_or(Ok(Vec::new()), |policy| {
            materialize_policy_obligations(head, policy)
        })?;
        let base_by_identity = obligation_map(&base_obligations);
        let head_by_identity = obligation_map(&head_obligations);
        let base_exact = base_obligations.iter().cloned().collect::<BTreeSet<_>>();
        let head_exact = head_obligations.iter().cloned().collect::<BTreeSet<_>>();
        let mut removed_obligations = base_exact
            .difference(&head_exact)
            .cloned()
            .collect::<Vec<_>>();
        let added_obligations = head_exact
            .difference(&base_exact)
            .cloned()
            .collect::<Vec<_>>();
        let logical_obligation_removed = base_by_identity
            .keys()
            .any(|identity| !head_by_identity.contains_key(identity));
        let mut diagnostics = BTreeSet::new();
        let mut strengthening = base_policy.is_none();
        let mut weakening = logical_obligation_removed || head_policy.is_none();
        let mut reviewed_terminal = false;

        if let (Some(base_policy), Some(head_policy)) = (base_policy, head_policy) {
            let base_subjects = subject_keys(base_policy);
            let head_subjects = subject_keys(head_policy);
            if !base_subjects.is_subset(&head_subjects) {
                diagnostics.insert(String::from("CHG-RATCHET-SCOPE"));
                weakening = true;
            }
            strengthening |= !head_subjects.is_subset(&base_subjects);

            let base_roles = base_policy.roles().iter().copied().collect::<BTreeSet<_>>();
            let head_roles = head_policy.roles().iter().copied().collect::<BTreeSet<_>>();
            if !base_roles.is_subset(&head_roles) {
                diagnostics.insert(String::from("CHG-RATCHET-ROLE"));
                weakening = true;
            }
            strengthening |= !head_roles.is_subset(&base_roles);

            if base_policy.mode() == EnforcementMode::Blocking
                && head_policy.mode() == EnforcementMode::Advisory
            {
                diagnostics.insert(String::from("CHG-RATCHET-MODE"));
                weakening = true;
            }
            strengthening |= base_policy.mode() == EnforcementMode::Advisory
                && head_policy.mode() == EnforcementMode::Blocking;

            let base_authorities = base_policy
                .revision_value()
                .authority_selectors()
                .iter()
                .cloned()
                .collect::<BTreeSet<TypedSelector>>();
            let head_authorities = head_policy
                .revision_value()
                .authority_selectors()
                .iter()
                .cloned()
                .collect::<BTreeSet<TypedSelector>>();
            if !base_authorities.is_subset(&head_authorities) {
                diagnostics.insert(String::from("CHG-RATCHET-AUTHORITY"));
                weakening = true;
            }
            strengthening |= !head_authorities.is_subset(&base_authorities);

            let base_derivation = base_policy.revision_value().derivation_requirements();
            let head_derivation = head_policy.revision_value().derivation_requirements();
            if (
                base_derivation.is_some(),
                base_derivation == head_derivation,
            ) == (true, false)
            {
                diagnostics.insert(String::from("CHG-RATCHET-DERIVATION"));
                weakening = true;
            }
            strengthening |= (
                head_derivation.is_some(),
                base_derivation == head_derivation,
            ) == (true, false);

            let base_members = surface_members(base, base_policy);
            let head_members = surface_members(head, head_policy);
            if !base_members.is_subset(&head_members) {
                diagnostics.insert(String::from("CHG-RATCHET-SURFACE"));
                weakening = true;
            }
            strengthening |= !head_members.is_subset(&base_members);

            let base_waiver_classes = accepted_waiver_classes(base, &policy_id);
            let head_waiver_classes = accepted_waiver_classes(head, &policy_id);
            if !head_waiver_classes.is_subset(&base_waiver_classes) {
                diagnostics.insert(String::from("CHG-RATCHET-WAIVER-CLASS"));
                weakening = true;
            }
            strengthening |= !base_waiver_classes.is_subset(&head_waiver_classes);

            if base_policy.ratchet() == head_policy.ratchet() {
                strengthening |= base_policy.revision() != head_policy.revision();
            } else {
                diagnostics.insert(String::from("CHG-RATCHET-LINEAGE"));
                weakening = true;
            }

            if (base_policy.lifecycle(), head_policy.lifecycle())
                == (PolicyLifecycle::Active, PolicyLifecycle::Superseded)
            {
                reviewed_terminal = true;
                let required = base_by_identity.keys().cloned().collect::<BTreeSet<_>>();
                if !replacement_covers(head, head_policy, &required)? {
                    diagnostics.insert(String::from("CHG-RATCHET-SUPERSESSION"));
                    weakening = true;
                }
            } else if (base_policy.lifecycle(), head_policy.lifecycle())
                == (PolicyLifecycle::Active, PolicyLifecycle::Retired)
            {
                reviewed_terminal = true;
            }
        } else if head_policy.is_none() {
            diagnostics.insert(String::from("CHG-RATCHET-RETIREMENT"));
        }

        if weakening && removed_obligations.is_empty() {
            removed_obligations.extend(base_obligations.iter().cloned());
        }
        removed_obligations.sort_unstable();
        removed_obligations.dedup();
        let classification = if weakening {
            PolicyChangeClassification::Weakening
        } else if reviewed_terminal {
            PolicyChangeClassification::EquivalentReviewed
        } else if strengthening {
            PolicyChangeClassification::Strengthening
        } else {
            PolicyChangeClassification::Equivalent
        };
        let base_reference = base_policy.map(exact_policy_reference);
        let head_reference = head_policy.map(exact_policy_reference);
        let impact = if classification == PolicyChangeClassification::Weakening {
            match (base_policy, base_reference.clone(), head_reference.clone()) {
                (Some(base_value), Some(base_reference), Some(head_reference)) => {
                    Some(PolicyRatchetImpact::from_validated(
                        base_value.revision_value().ratchet_scope_id().clone(),
                        base_reference,
                        head_reference,
                        removed_obligations.clone(),
                    ))
                }
                _ => None,
            }
        } else {
            None
        };
        derived.push(DerivedPolicyRatchet {
            policy_id,
            base_policy: base_reference,
            head_policy: head_reference,
            base_obligations,
            head_obligations,
            removed_obligations,
            added_obligations,
            classification,
            diagnostics: diagnostics.into_iter().collect(),
            impact,
        });
    }
    Ok(derived)
}

/// Inputs to conservative evidence aggregation.
///
/// The booleans are intentionally orthogonal report facts; replacing them with
/// one optimistic state enum would permit assurance laundering.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct AggregationInput {
    /// Current syntax attachment and bindings.
    pub linked: bool,
    /// Whether every required ordinary carrier is fresh.
    pub required_fresh: bool,
    /// Whether every required ordinary carrier was selected.
    pub required_selected: bool,
    /// Required ordinary verdicts.
    pub verdicts: Vec<Verdict>,
    /// Whether an exact finite model reached a fixed point.
    pub exhaustive: bool,
    /// Whether an exact accepted proof record is fresh.
    pub formal: bool,
    /// Whether an exact structural oracle passed.
    pub structural: bool,
    /// Whether an exact repository review binding exists.
    pub reviewed: bool,
    /// Visible active debt; never used to create assurance.
    pub debt: bool,
    /// Visible active waiver; never used to create assurance.
    pub waiver: bool,
    /// Visible reaffirmation; only `reviewed` controls review assurance.
    pub reaffirmation: bool,
}

/// Emits only independently established assurance classes.
#[must_use]
pub fn aggregate_assurance(input: &AggregationInput) -> Vec<AssuranceClass> {
    let mut classes = Vec::new();
    if input.linked {
        classes.push(AssuranceClass::Linked);
    }
    if input.linked
        && input.required_fresh
        && input.required_selected
        && !input.verdicts.is_empty()
        && input
            .verdicts
            .iter()
            .all(|verdict| *verdict == Verdict::Pass)
    {
        classes.push(AssuranceClass::Tested);
    }
    if input.exhaustive {
        classes.push(AssuranceClass::ExhaustivelyChecked);
    }
    if input.formal {
        classes.push(AssuranceClass::FormallyProved);
    }
    if input.structural {
        classes.push(AssuranceClass::StructurallyEnforced);
    }
    if input.reviewed {
        classes.push(AssuranceClass::ReviewOnly);
    }
    classes
}

/// One frozen schema-declared dependency edge used by impact indexing.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SchemaImpactEdge {
    id: &'static str,
    source: &'static str,
    target: &'static str,
}

impl SchemaImpactEdge {
    /// Stable edge identity formed from the source domain and field name.
    #[must_use]
    pub const fn id(self) -> &'static str {
        self.id
    }

    /// Schema domain containing the dependency field.
    #[must_use]
    pub const fn source(self) -> &'static str {
        self.source
    }

    /// Schema domain referenced by the dependency field.
    #[must_use]
    pub const fn target(self) -> &'static str {
        self.target
    }
}

/// Closed schema-v3 dependency-edge catalog in schema inventory order.
pub static SCHEMA_IMPACT_EDGES: &[SchemaImpactEdge] = &[
    SchemaImpactEdge {
        id: "approval-binding-record-artifact",
        source: "approval-binding",
        target: "raw-artifact",
    },
    SchemaImpactEdge {
        id: "approval-authority-declaration-approval",
        source: "approval-authority",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "id-allocation-allocation-record",
        source: "id-allocation",
        target: "raw-artifact",
    },
    SchemaImpactEdge {
        id: "repository-config-policies",
        source: "repository-config",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "repository-config-tools",
        source: "repository-config",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "repository-config-approval-authorities",
        source: "repository-config",
        target: "approval-authority",
    },
    SchemaImpactEdge {
        id: "repository-config-id-namespaces",
        source: "repository-config",
        target: "id-namespace",
    },
    SchemaImpactEdge {
        id: "repository-snapshot-repository-config",
        source: "repository-snapshot",
        target: "repository-config",
    },
    SchemaImpactEdge {
        id: "repository-snapshot-schema-tool-lock",
        source: "repository-snapshot",
        target: "schema-tool-lock",
    },
    SchemaImpactEdge {
        id: "repository-snapshot-files",
        source: "repository-snapshot",
        target: "snapshot-file",
    },
    SchemaImpactEdge {
        id: "snapshot-file-raw-artifact",
        source: "snapshot-file",
        target: "raw-artifact",
    },
    SchemaImpactEdge {
        id: "schema-tool-lock-unicode-tool",
        source: "schema-tool-lock",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "schema-tool-lock-commonmark-tool",
        source: "schema-tool-lock",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "schema-tool-lock-toml-tool",
        source: "schema-tool-lock",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "schema-tool-lock-rust-tool",
        source: "schema-tool-lock",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "schema-tool-lock-additional-tools",
        source: "schema-tool-lock",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "atom-object-revision",
        source: "atom-object",
        target: "atom-revision",
    },
    SchemaImpactEdge {
        id: "atom-object-approval",
        source: "atom-object",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "tombstone-final-revision",
        source: "tombstone",
        target: "atom-revision",
    },
    SchemaImpactEdge {
        id: "tombstone-approval",
        source: "tombstone",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "tombstone-replacement-atoms",
        source: "tombstone",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "decision-surface-domain",
        source: "decision-surface",
        target: "decision-surface-domain",
    },
    SchemaImpactEdge {
        id: "decision-surface-dispositions",
        source: "decision-surface",
        target: "surface-disposition",
    },
    SchemaImpactEdge {
        id: "decision-surface-policies",
        source: "decision-surface",
        target: "coverage-policy-object",
    },
    SchemaImpactEdge {
        id: "surface-disposition-authority-atom",
        source: "surface-disposition",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "evidence-carrier-atoms",
        source: "evidence-carrier",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "evidence-carrier-parsed-item",
        source: "evidence-carrier",
        target: "parsed-item-identity",
    },
    SchemaImpactEdge {
        id: "evidence-carrier-oracle",
        source: "evidence-carrier",
        target: "oracle",
    },
    SchemaImpactEdge {
        id: "evidence-carrier-assumptions",
        source: "evidence-carrier",
        target: "assumption",
    },
    SchemaImpactEdge {
        id: "implementation-link-atoms",
        source: "implementation-link",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "implementation-link-parsed-item",
        source: "implementation-link",
        target: "parsed-item-identity",
    },
    SchemaImpactEdge {
        id: "external-trace-link-atoms",
        source: "external-trace-link",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "buoy/implementation-binding/v1-atom",
        source: "buoy/implementation-binding/v1",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "parsed-item-identity-enclosing-item",
        source: "parsed-item-identity",
        target: "parsed-item-identity",
    },
    SchemaImpactEdge {
        id: "derivation-source-bindings",
        source: "derivation",
        target: "artifact-binding",
    },
    SchemaImpactEdge {
        id: "derivation-target-bindings",
        source: "derivation",
        target: "artifact-binding",
    },
    SchemaImpactEdge {
        id: "derivation-consistency-oracle",
        source: "derivation",
        target: "oracle",
    },
    SchemaImpactEdge {
        id: "derivation-tool",
        source: "derivation",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "artifact-binding-object",
        source: "artifact-binding",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "artifact-binding-repository-artifact",
        source: "artifact-binding",
        target: "artifact-index",
    },
    SchemaImpactEdge {
        id: "artifact-entry-raw-artifact",
        source: "artifact-entry",
        target: "raw-artifact",
    },
    SchemaImpactEdge {
        id: "artifact-index-entries",
        source: "artifact-index",
        target: "artifact-entry",
    },
    SchemaImpactEdge {
        id: "coverage-policy-object-revision",
        source: "coverage-policy-object",
        target: "coverage-policy-revision",
    },
    SchemaImpactEdge {
        id: "coverage-policy-object-active-approval",
        source: "coverage-policy-object",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "coverage-policy-object-replacements",
        source: "coverage-policy-object",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "coverage-policy-object-terminal-approval",
        source: "coverage-policy-object",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "debt-record-obligations",
        source: "debt-record",
        target: "obligation-reference",
    },
    SchemaImpactEdge {
        id: "debt-record-objects",
        source: "debt-record",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "debt-record-discharge-evidence",
        source: "debt-record",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "debt-record-replacements",
        source: "debt-record",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "debt-record-terminal-approval",
        source: "debt-record",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "waiver-obligations",
        source: "waiver",
        target: "obligation-reference",
    },
    SchemaImpactEdge {
        id: "waiver-objects",
        source: "waiver",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "waiver-active-approval",
        source: "waiver",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "waiver-discharge-evidence",
        source: "waiver",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "waiver-replacements",
        source: "waiver",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "waiver-terminal-approval",
        source: "waiver",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "reaffirmation-affected-object",
        source: "reaffirmation",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "reaffirmation-review-approval",
        source: "reaffirmation",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "obligation-reference-policy",
        source: "obligation-reference",
        target: "coverage-policy-object",
    },
    SchemaImpactEdge {
        id: "obligation-reference-subject",
        source: "obligation-reference",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "oracle-runner-tool",
        source: "oracle",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "normalized-verdict-oracle",
        source: "normalized-verdict",
        target: "oracle",
    },
    SchemaImpactEdge {
        id: "normalized-verdict-subjects",
        source: "normalized-verdict",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "verdict-set-verdicts",
        source: "verdict-set",
        target: "normalized-verdict",
    },
    SchemaImpactEdge {
        id: "assumption-approval",
        source: "assumption",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "finite-model-checked-invariants",
        source: "finite-model",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "finite-model-assumptions",
        source: "finite-model",
        target: "assumption",
    },
    SchemaImpactEdge {
        id: "formal-property-atoms",
        source: "formal-property",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "formal-property-assumptions",
        source: "formal-property",
        target: "assumption",
    },
    SchemaImpactEdge {
        id: "formal-property-correspondence-approval",
        source: "formal-property",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "proof-record-atoms",
        source: "proof-record",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "proof-record-formal-property",
        source: "proof-record",
        target: "formal-property",
    },
    SchemaImpactEdge {
        id: "proof-record-implementation-representation",
        source: "proof-record",
        target: "artifact-binding",
    },
    SchemaImpactEdge {
        id: "proof-record-assumptions",
        source: "proof-record",
        target: "assumption",
    },
    SchemaImpactEdge {
        id: "proof-record-translators",
        source: "proof-record",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "proof-record-verifier",
        source: "proof-record",
        target: "tool-identity",
    },
    SchemaImpactEdge {
        id: "proof-record-external-models",
        source: "proof-record",
        target: "artifact-binding",
    },
    SchemaImpactEdge {
        id: "proof-record-generated-artifacts",
        source: "proof-record",
        target: "artifact-binding",
    },
    SchemaImpactEdge {
        id: "proof-record-proof-sources",
        source: "proof-record",
        target: "artifact-binding",
    },
    SchemaImpactEdge {
        id: "source-map-entry-object",
        source: "source-map-entry",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "source-map-entry-sources",
        source: "source-map-entry",
        target: "source-reference",
    },
    SchemaImpactEdge {
        id: "source-map-entries",
        source: "source-map",
        target: "source-map-entry",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-atoms",
        source: "spec-graph-semantic",
        target: "atom-object",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-tombstones",
        source: "spec-graph-semantic",
        target: "tombstone",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-decision-surfaces",
        source: "spec-graph-semantic",
        target: "decision-surface",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-evidence-carriers",
        source: "spec-graph-semantic",
        target: "evidence-carrier",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-implementation-links",
        source: "spec-graph-semantic",
        target: "implementation-link",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-derivations",
        source: "spec-graph-semantic",
        target: "derivation",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-coverage-policies",
        source: "spec-graph-semantic",
        target: "coverage-policy-object",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-debt-records",
        source: "spec-graph-semantic",
        target: "debt-record",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-waivers",
        source: "spec-graph-semantic",
        target: "waiver",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-oracles",
        source: "spec-graph-semantic",
        target: "oracle",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-finite-models",
        source: "spec-graph-semantic",
        target: "finite-model",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-formal-properties",
        source: "spec-graph-semantic",
        target: "formal-property",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-assumptions",
        source: "spec-graph-semantic",
        target: "assumption",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-proof-records",
        source: "spec-graph-semantic",
        target: "proof-record",
    },
    SchemaImpactEdge {
        id: "spec-graph-semantic-reaffirmations",
        source: "spec-graph-semantic",
        target: "reaffirmation",
    },
    SchemaImpactEdge {
        id: "spec-graph-complete-semantic-graph",
        source: "spec-graph-complete",
        target: "spec-graph-semantic",
    },
    SchemaImpactEdge {
        id: "spec-graph-complete-source-map",
        source: "spec-graph-complete",
        target: "source-map",
    },
    SchemaImpactEdge {
        id: "spec-graph-complete-repository-config",
        source: "spec-graph-complete",
        target: "repository-config",
    },
    SchemaImpactEdge {
        id: "spec-graph-complete-schema-tool-lock",
        source: "spec-graph-complete",
        target: "schema-tool-lock",
    },
    SchemaImpactEdge {
        id: "change-manifest-base-graph",
        source: "change-manifest",
        target: "spec-graph-complete",
    },
    SchemaImpactEdge {
        id: "change-manifest-head-graph",
        source: "change-manifest",
        target: "spec-graph-complete",
    },
    SchemaImpactEdge {
        id: "change-manifest-dispositions",
        source: "change-manifest",
        target: "change-disposition",
    },
    SchemaImpactEdge {
        id: "change-manifest-approval",
        source: "change-manifest",
        target: "approval-binding",
    },
    SchemaImpactEdge {
        id: "change-disposition-affected-object",
        source: "change-disposition",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "change-disposition-reaffirmation",
        source: "change-disposition",
        target: "reaffirmation",
    },
    SchemaImpactEdge {
        id: "change-disposition-waiver",
        source: "change-disposition",
        target: "waiver",
    },
    SchemaImpactEdge {
        id: "change-disposition-replacements",
        source: "change-disposition",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "policy-ratchet-impact-base-policy",
        source: "policy-ratchet-impact",
        target: "coverage-policy-object",
    },
    SchemaImpactEdge {
        id: "policy-ratchet-impact-head-policy",
        source: "policy-ratchet-impact",
        target: "coverage-policy-object",
    },
    SchemaImpactEdge {
        id: "policy-ratchet-impact-weakened-obligations",
        source: "policy-ratchet-impact",
        target: "obligation-reference",
    },
    SchemaImpactEdge {
        id: "impact-set-direct-changes",
        source: "impact-set",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "impact-set-transitive-impacts",
        source: "impact-set",
        target: "object-reference",
    },
    SchemaImpactEdge {
        id: "impact-set-policy-ratchets",
        source: "impact-set",
        target: "policy-ratchet-impact",
    },
];

fn schema_impact_edge(id: &str) -> Option<SchemaImpactEdge> {
    SCHEMA_IMPACT_EDGES
        .iter()
        .copied()
        .find(|edge| edge.id == id)
}

fn schema_impact_kind_known(kind: &str) -> bool {
    SCHEMA_IMPACT_EDGES
        .iter()
        .any(|edge| edge.source == kind || edge.target == kind)
}

fn schema_object_domain(kind: &str) -> bool {
    matches!(
        kind,
        "atom-object"
            | "tombstone"
            | "decision-surface"
            | "evidence-carrier"
            | "implementation-link"
            | "derivation"
            | "coverage-policy-object"
            | "debt-record"
            | "waiver"
            | "oracle"
            | "finite-model"
            | "formal-property"
            | "assumption"
            | "proof-record"
            | "parsed-item-identity"
            | "reaffirmation"
            | "change-manifest"
            | "change-disposition"
    )
}

fn schema_impact_target_matches(declared: &str, actual: &str) -> bool {
    declared == actual || (declared == "object-reference" && schema_object_domain(actual))
}

/// Canonically ordered identity of one schema impact node.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SchemaImpactKey {
    kind: String,
    id: String,
}

impl SchemaImpactKey {
    /// Constructs one known schema-domain key.
    ///
    /// # Errors
    ///
    /// Rejects unknown schema domains and empty or noncanonical identifiers.
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Result<Self, SchemaImpactError> {
        let kind = kind.into();
        let id = id.into();
        if !schema_impact_kind_known(&kind) {
            return Err(SchemaImpactError::UnknownKind);
        }
        if id.is_empty() || CanonicalValue::Text(id.clone()).encode().is_err() {
            return Err(SchemaImpactError::InvalidKey);
        }
        Ok(Self { kind, id })
    }

    /// Frozen schema domain name.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Stable identity within the schema domain.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// One declared dependency from an indexed schema value to another value.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SchemaImpactDependency {
    edge_id: String,
    target: SchemaImpactKey,
}

impl SchemaImpactDependency {
    /// Constructs one dependency using the frozen edge catalog.
    ///
    /// # Errors
    ///
    /// Rejects unknown edges or a target whose domain disagrees with the edge.
    pub fn new(
        edge_id: impl Into<String>,
        target: SchemaImpactKey,
    ) -> Result<Self, SchemaImpactError> {
        let edge_id = edge_id.into();
        let Some(edge) = schema_impact_edge(&edge_id) else {
            return Err(SchemaImpactError::UnknownEdge);
        };
        if !schema_impact_target_matches(edge.target, &target.kind) {
            return Err(SchemaImpactError::TargetKind);
        }
        Ok(Self { edge_id, target })
    }

    /// Frozen edge identity.
    #[must_use]
    pub fn edge_id(&self) -> &str {
        &self.edge_id
    }

    /// Referenced impact node.
    #[must_use]
    pub const fn target(&self) -> &SchemaImpactKey {
        &self.target
    }
}

/// One immutable schema-domain value and all declared outgoing dependencies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaImpactNode {
    key: SchemaImpactKey,
    digest: SemanticDigest,
    dependencies: Vec<SchemaImpactDependency>,
}

impl SchemaImpactNode {
    /// Constructs one canonical impact-index node.
    ///
    /// # Errors
    ///
    /// Rejects duplicate dependencies or dependencies declared for another source domain.
    pub fn new(
        key: SchemaImpactKey,
        digest: SemanticDigest,
        mut dependencies: Vec<SchemaImpactDependency>,
    ) -> Result<Self, SchemaImpactError> {
        if dependencies.iter().any(|dependency| {
            schema_impact_edge(&dependency.edge_id).is_none_or(|edge| edge.source != key.kind)
        }) {
            return Err(SchemaImpactError::SourceKind);
        }
        dependencies.sort_unstable();
        if dependencies.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SchemaImpactError::DuplicateDependency);
        }
        Ok(Self {
            key,
            digest,
            dependencies,
        })
    }

    /// Node identity.
    #[must_use]
    pub const fn key(&self) -> &SchemaImpactKey {
        &self.key
    }

    /// Canonical semantic digest for this graph side.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Canonically ordered declared dependencies.
    #[must_use]
    pub fn dependencies(&self) -> &[SchemaImpactDependency] {
        &self.dependencies
    }
}

/// Closed immutable schema-impact index for one graph side.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SchemaImpactIndex {
    nodes: BTreeMap<SchemaImpactKey, SchemaImpactNode>,
}

impl SchemaImpactIndex {
    /// Constructs one index and proves local edge closure.
    ///
    /// # Errors
    ///
    /// Rejects duplicate node keys or dependencies whose target is absent.
    pub fn new(nodes: Vec<SchemaImpactNode>) -> Result<Self, SchemaImpactError> {
        let mut indexed = BTreeMap::new();
        for node in nodes {
            if indexed.insert(node.key.clone(), node).is_some() {
                return Err(SchemaImpactError::DuplicateNode);
            }
        }
        if indexed.values().any(|node| {
            node.dependencies
                .iter()
                .any(|dependency| !indexed.contains_key(&dependency.target))
        }) {
            return Err(SchemaImpactError::DanglingDependency);
        }
        Ok(Self { nodes: indexed })
    }

    /// Returns one indexed node by exact key.
    #[must_use]
    pub fn get(&self, key: &SchemaImpactKey) -> Option<&SchemaImpactNode> {
        self.nodes.get(key)
    }

    /// Number of indexed values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether no values are indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Deterministic schema-impact index construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaImpactError {
    /// Node schema domain is absent from the frozen edge catalog.
    UnknownKind,
    /// Node identity is empty or not canonical NFC text.
    InvalidKey,
    /// Dependency edge is absent from the frozen edge catalog.
    UnknownEdge,
    /// Dependency target domain disagrees with its declared edge.
    TargetKind,
    /// Dependency source domain disagrees with its containing node.
    SourceKind,
    /// The same edge and target appear more than once on a node.
    DuplicateDependency,
    /// Two nodes have the same schema domain and stable identity.
    DuplicateNode,
    /// A dependency target is absent from the same immutable index.
    DanglingDependency,
}

impl SchemaImpactError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnknownKind => "CHG-IMPACT-KIND-UNKNOWN",
            Self::InvalidKey => "CHG-IMPACT-KEY-INVALID",
            Self::UnknownEdge => "CHG-IMPACT-EDGE-UNKNOWN",
            Self::TargetKind => "CHG-IMPACT-EDGE-TARGET",
            Self::SourceKind => "CHG-IMPACT-EDGE-SOURCE",
            Self::DuplicateDependency => "CHG-IMPACT-EDGE-DUPLICATE",
            Self::DuplicateNode => "CHG-IMPACT-NODE-DUPLICATE",
            Self::DanglingDependency => "CHG-IMPACT-EDGE-DANGLING",
        }
    }
}

/// One direct or transitively reached schema value with exact graph-pair bindings.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SchemaImpact {
    key: SchemaImpactKey,
    base: Option<SemanticDigest>,
    head: Option<SemanticDigest>,
    direct: bool,
}

impl SchemaImpact {
    /// Impacted schema-domain key.
    #[must_use]
    pub const fn key(&self) -> &SchemaImpactKey {
        &self.key
    }

    /// Base-side digest when present.
    #[must_use]
    pub const fn base(&self) -> Option<SemanticDigest> {
        self.base
    }

    /// Head-side digest when present.
    #[must_use]
    pub const fn head(&self) -> Option<SemanticDigest> {
        self.head
    }

    /// Whether canonical kind/ID/digest comparison found this value directly changed.
    #[must_use]
    pub const fn direct(&self) -> bool {
        self.direct
    }
}

/// Computes direct changes and reverse-dependency closure to a deterministic fixed point.
#[must_use = "impact results must be checked"]
pub fn schema_change_impacts(
    base: &SchemaImpactIndex,
    head: &SchemaImpactIndex,
) -> Vec<SchemaImpact> {
    let all_keys = base
        .nodes
        .keys()
        .chain(head.nodes.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let direct = all_keys
        .into_iter()
        .filter(|key| {
            base.nodes.get(key).map(SchemaImpactNode::digest)
                != head.nodes.get(key).map(SchemaImpactNode::digest)
        })
        .collect::<BTreeSet<_>>();
    let mut reverse = BTreeMap::<SchemaImpactKey, BTreeSet<SchemaImpactKey>>::new();
    for index in [base, head] {
        for node in index.nodes.values() {
            for dependency in &node.dependencies {
                reverse
                    .entry(dependency.target.clone())
                    .or_default()
                    .insert(node.key.clone());
            }
        }
    }
    let mut impacted = direct.clone();
    loop {
        let before = impacted.len();
        let frontier = impacted.iter().cloned().collect::<Vec<_>>();
        for key in frontier {
            if let Some(dependents) = reverse.get(&key) {
                impacted.extend(dependents.iter().cloned());
            }
        }
        if impacted.len() == before {
            break;
        }
    }
    impacted
        .into_iter()
        .map(|key| SchemaImpact {
            base: base.nodes.get(&key).map(SchemaImpactNode::digest),
            head: head.nodes.get(&key).map(SchemaImpactNode::digest),
            direct: direct.contains(&key),
            key,
        })
        .collect()
}

/// Canonical child references keyed by exact parent node digest.
type SourceAdjacency = BTreeMap<SemanticDigest, Vec<SourceNodeRef>>;

/// Collects outgoing references in canonical relation, position, and target order.
fn source_adjacency(graph: &SourceGraph) -> SourceAdjacency {
    let mut adjacency = SourceAdjacency::new();
    for node in graph.nodes() {
        adjacency.entry(node.digest()).or_default();
    }
    for edge in graph.edges() {
        adjacency
            .entry(edge.source().digest())
            .or_default()
            .push(edge.target());
    }
    adjacency
}

/// Outgoing references of one node in canonical edge order.
fn source_children(adjacency: &SourceAdjacency, node: SourceNodeRef) -> &[SourceNodeRef] {
    adjacency.get(&node.digest()).map_or(&[][..], Vec::as_slice)
}

/// One bounded canonical source-graph traversal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceTraversal {
    order: Vec<SourceNodeRef>,
    visited: u32,
    depth: u32,
}

impl SourceTraversal {
    /// Visited nodes in canonical preorder.
    #[must_use]
    pub fn order(&self) -> &[SourceNodeRef] {
        &self.order
    }

    /// Count of visited nodes.
    #[must_use]
    pub const fn visited(&self) -> u32 {
        self.visited
    }

    /// Longest traversed path length in edges.
    #[must_use]
    pub const fn depth(&self) -> u32 {
        self.depth
    }

    /// Whether this traversal reached one exact node.
    #[must_use]
    pub fn contains(&self, reference: SourceNodeRef) -> bool {
        self.order.contains(&reference)
    }
}

/// Traverses one acyclic source graph from a start set inside every budget.
///
/// The walk stops at the exact node that crosses the visited budget and at the
/// exact relaxation that crosses the depth budget, so no exhausted budget
/// accumulates a complete closure first. Every rejection stays total and
/// canonical: one exceeded budget returns no partial traversal, and the
/// reported budget never depends on canonical edge order, because a crossed
/// visited budget is exactly a reachable set wider than that budget and a
/// crossed depth budget is exactly a longest reachable path above that budget.
///
/// `SourceGraph` construction settles acyclicity, so the reached subgraph is
/// one directed acyclic graph and settlement order relaxes each node once its
/// reached parents are final.
fn bounded_source_walk(
    graph: &SourceGraph,
    starts: &[SourceNodeRef],
) -> Result<SourceTraversal, SourceGraphError> {
    let adjacency = source_adjacency(graph);
    let visited_budget = graph.budgets().visited().get();
    let depth_budget = graph.budgets().depth().get();

    let mut reached = BTreeSet::new();
    let mut order = Vec::new();
    let mut visited = 0_u32;
    let mut pending = starts.iter().rev().copied().collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        if !reached.insert(node) {
            continue;
        }
        if visited == visited_budget {
            return Err(SourceGraphError::VisitedBudget);
        }
        visited += 1;
        order.push(node);
        pending.extend(source_children(&adjacency, node).iter().rev().copied());
    }

    let mut longest = order
        .iter()
        .map(|node| (*node, 0_u32))
        .collect::<BTreeMap<_, _>>();
    let mut remaining = BTreeMap::<SourceNodeRef, u32>::new();
    for node in &order {
        remaining.entry(*node).or_default();
        for child in source_children(&adjacency, *node) {
            *remaining.entry(*child).or_default() += 1;
        }
    }
    let mut ready = order
        .iter()
        .filter(|node| remaining.get(node).copied().unwrap_or_default() == 0)
        .copied()
        .collect::<Vec<_>>();
    let mut depth = 0_u32;
    while let Some(node) = ready.pop() {
        let current = longest.get(&node).copied().unwrap_or_default();
        for child in source_children(&adjacency, node) {
            let relaxed = current.saturating_add(1);
            if relaxed > depth_budget {
                return Err(SourceGraphError::DepthBudget);
            }
            longest.entry(*child).and_modify(|value| {
                *value = (*value).max(relaxed);
                depth = depth.max(*value);
            });
            remaining.entry(*child).and_modify(|count| {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    ready.push(*child);
                }
            });
        }
    }
    Ok(SourceTraversal {
        order,
        visited,
        depth,
    })
}

/// Computes the bounded canonical closure over every declared current root.
///
/// # Errors
///
/// Rejects a closure above the visited or depth budget.
pub fn source_root_closure(graph: &SourceGraph) -> Result<SourceTraversal, SourceGraphError> {
    bounded_source_walk(graph, graph.roots())
}

/// Computes one bounded canonical traversal from one exact source node.
///
/// # Errors
///
/// Rejects an unknown start node and an exceeded visited or depth budget.
pub fn source_traversal(
    graph: &SourceGraph,
    from: SourceNodeRef,
) -> Result<SourceTraversal, SourceGraphError> {
    if graph.node(from).is_none() {
        return Err(SourceGraphError::UnknownNode);
    }
    bounded_source_walk(graph, &[from])
}

/// Requires one referenced node to be current under the declared root closure.
///
/// A retained historical node supports diagnostics, but it never satisfies
/// current source-reference closure.
///
/// # Errors
///
/// Rejects every reference outside the current root closure.
pub fn require_current_source_reference(
    closure: &SourceTraversal,
    reference: SourceNodeRef,
) -> Result<(), SourceGraphError> {
    if closure.contains(reference) {
        Ok(())
    } else {
        Err(SourceGraphError::StaleReference)
    }
}

/// Complete explicit inputs for compact implementation-binding validation.
///
/// The adapter that owns the specification host parses its own requirement kind
/// and supplies it here, so this core compares parsed kinds and never carries
/// host syntax. Construction indexes the materialized authorities by
/// requirement node, so every reported authority fact counts distinct atom
/// revisions rather than caller-supplied rows.
pub struct BindingValidationContext<'a> {
    graph: &'a SourceGraph,
    closure: &'a SourceTraversal,
    requirement_kind: SourceItemKind,
    authorities: BTreeMap<SourceNodeRef, BTreeSet<AtomRevisionRef>>,
}

impl<'a> BindingValidationContext<'a> {
    /// Collects one source graph, its current root closure, the expected
    /// requirement kind, and every materialized requirement-to-atom authority.
    #[must_use]
    pub fn new(
        graph: &'a SourceGraph,
        closure: &'a SourceTraversal,
        requirement_kind: SourceItemKind,
        authorities: &[SourceAuthority],
    ) -> Self {
        let mut indexed = BTreeMap::<SourceNodeRef, BTreeSet<AtomRevisionRef>>::new();
        for authority in authorities {
            indexed
                .entry(authority.node())
                .or_default()
                .insert(authority.atom().clone());
        }
        Self {
            graph,
            closure,
            requirement_kind,
            authorities: indexed,
        }
    }

    /// Distinct atom revisions that materialize one exact requirement node.
    fn requirement_authority(
        &self,
        requirement: SourceNodeRef,
    ) -> Option<&BTreeSet<AtomRevisionRef>> {
        self.authorities.get(&requirement)
    }
}

/// Reports every failed fact for one attached compact implementation binding.
///
/// Each fact has one stable code, so a caller never replaces several failures
/// with one optimistic result. An unresolved locator stops the record-dependent
/// facts, because no preimage remains to compare. The returned diagnostics are
/// canonically ordered and deduplicated, and every fact names its exact
/// attachment, so no attachment, store, or authority permutation changes the
/// reported set and no two distinct attachments collapse into one fact.
///
/// A binding without diagnostics is fresh navigation only. It supplies no
/// behavioral coverage, execution verdict, proof, or approval authority.
///
/// The context does not contain the selected repository digest member. The
/// repository boundary must reject locators from other members before this
/// function runs. This function checks record fields against each locator.
#[must_use]
pub fn implementation_binding_diagnostics(
    attachments: &[BindingAttachment],
    store: &[StoredBinding],
    context: &BindingValidationContext<'_>,
) -> Vec<Diagnostic> {
    let index = StoreIndex::new(store);
    let mut diagnostics = Vec::new();
    for attachment in attachments {
        binding_attachment_diagnostics(*attachment, &index, context, &mut diagnostics);
    }
    diagnostics.sort_unstable();
    diagnostics.dedup();
    diagnostics
}

/// One locator-keyed view of the object store.
///
/// Record equality is symmetric and transitive, so agreement with the first
/// stored member is exactly pairwise agreement across every stored member. The
/// index therefore settles each locator once, and attachment resolution stops
/// rescanning the whole store for every attachment.
struct StoreIndex<'a> {
    addressed: BTreeMap<ImplementationBindingRef, (&'a ImplementationBinding, bool)>,
}

impl<'a> StoreIndex<'a> {
    fn new(store: &'a [StoredBinding]) -> Self {
        let mut addressed =
            BTreeMap::<ImplementationBindingRef, (&'a ImplementationBinding, bool)>::new();
        for member in store {
            addressed
                .entry(member.locator())
                .and_modify(|claimed| claimed.1 &= claimed.0 == member.record())
                .or_insert((member.record(), true));
        }
        Self { addressed }
    }

    /// Selects the one stored record that an exact locator addresses.
    fn resolve(
        &self,
        locator: ImplementationBindingRef,
    ) -> Result<&'a ImplementationBinding, &'static str> {
        match self.addressed.get(&locator) {
            None => Err("CAB-RECORD-ABSENT"),
            Some((_, false)) => Err("CAB-RECORD-AMBIGUOUS"),
            Some((record, true)) => Ok(record),
        }
    }
}

/// Reports every failed fact for one exact attached locator.
fn binding_attachment_diagnostics(
    attachment: BindingAttachment,
    store: &StoreIndex<'_>,
    context: &BindingValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let record = match store.resolve(attachment.record()) {
        Ok(record) => record,
        Err(code) => {
            diagnostics.push(binding_diagnostic(code, attachment, None));
            binding_digest_diagnostics(attachment, None, diagnostics);
            return;
        }
    };
    let relation = Some(record.meta().id());
    binding_digest_diagnostics(attachment, Some(record), diagnostics);
    if record.record() != attachment.record() {
        diagnostics.push(binding_diagnostic(
            "CAB-RECORD-DIGEST",
            attachment,
            relation,
        ));
    }
    let subject_is_current =
        require_current_source_reference(context.closure, record.subject()).is_ok();
    if record.subject() != attachment.subject() && subject_is_current {
        diagnostics.push(binding_diagnostic(
            "CAB-SUBJECT-ATTACHMENT",
            attachment,
            relation,
        ));
    }
    binding_source_diagnostics(record, attachment, context, diagnostics);
    binding_authority_diagnostics(record, attachment, context, diagnostics);
}

/// Binds one binding fact to its exact attachment and, once resolved, its relation.
fn binding_diagnostic(
    code: &str,
    attachment: BindingAttachment,
    relation: Option<&StableId>,
) -> Diagnostic {
    let mut diagnostic = relation.map_or_else(
        || Diagnostic::global(code),
        |id| Diagnostic::object(code, ObjectKind::ImplementationBinding, id),
    );
    diagnostic
        .fields
        .push((String::from("record"), attachment.record().canonical_text()));
    diagnostic.fields.push((
        String::from("subject"),
        attachment.subject().canonical_text(),
    ));
    diagnostic
}

/// Checks the assurance class and registry member of every addressed digest.
///
/// One compact locator carries no algorithm agility, so it can resolve a record
/// only when the locator, the record digest, and every reference the record
/// binds address one registered member with the collision-resistant class.
fn binding_digest_diagnostics(
    attachment: BindingAttachment,
    record: Option<&ImplementationBinding>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let locator = attachment.record().digest();
    let mut members = vec![locator];
    if let Some(record) = record {
        members.extend([
            record.meta().digest(),
            record.subject().digest(),
            record.requirement().digest(),
            record.atom().revision(),
        ]);
    }
    let relation = record.map(|record| record.meta().id());
    if members.iter().any(|digest| {
        digest.algorithm().assurance_class() != DigestAssuranceClass::CollisionResistant
    }) {
        diagnostics.push(binding_diagnostic("CAB-DIGEST-CLASS", attachment, relation));
    }
    if members
        .iter()
        .any(|digest| digest.algorithm() != locator.algorithm())
    {
        diagnostics.push(binding_diagnostic(
            "CAB-DIGEST-MEMBER",
            attachment,
            relation,
        ));
    }
}

/// Checks the target kind and current reachability of both bound source nodes.
///
/// Each side reports its own staleness, so an absent node and a retained
/// historical node are the same freshness fact for that exact side and never
/// hide the other side. An absent requirement node carries no parsed kind, so
/// it reports staleness alone rather than an invented kind failure.
fn binding_source_diagnostics(
    record: &ImplementationBinding,
    attachment: BindingAttachment,
    context: &BindingValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let relation = Some(record.meta().id());
    if require_current_source_reference(context.closure, record.subject()).is_err() {
        diagnostics.push(binding_diagnostic(
            "CAB-SUBJECT-STALE",
            attachment,
            relation,
        ));
    }
    if context
        .graph
        .node(record.requirement())
        .is_some_and(|requirement| *requirement.item().kind() != context.requirement_kind)
    {
        diagnostics.push(binding_diagnostic(
            "CAB-REQUIREMENT-KIND",
            attachment,
            relation,
        ));
    }
    if require_current_source_reference(context.closure, record.requirement()).is_err() {
        diagnostics.push(binding_diagnostic(
            "CAB-REQUIREMENT-STALE",
            attachment,
            relation,
        ));
    }
}

/// Checks that exactly one materialized authority governs the bound requirement.
///
/// A requirement without an authority and a requirement whose one authority
/// names another atom revision are both correspondence failures, and a
/// requirement with several distinct authorities settles no correspondence at
/// all.
fn binding_authority_diagnostics(
    record: &ImplementationBinding,
    attachment: BindingAttachment,
    context: &BindingValidationContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if require_current_source_reference(context.closure, record.requirement()).is_err()
        || context
            .graph
            .node(record.requirement())
            .is_some_and(|requirement| *requirement.item().kind() != context.requirement_kind)
    {
        return;
    }
    let relation = Some(record.meta().id());
    let atoms = context.requirement_authority(record.requirement());
    if atoms.map_or(0, BTreeSet::len) > 1 {
        diagnostics.push(binding_diagnostic(
            "CAB-AUTHORITY-AMBIGUOUS",
            attachment,
            relation,
        ));
    } else if !atoms.is_some_and(|atoms| atoms.contains(record.atom())) {
        diagnostics.push(binding_diagnostic(
            "CAB-AUTHORITY-MISMATCH",
            attachment,
            relation,
        ));
    }
}

/// One direct or transitively impacted object key and graph-pair binding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Impact {
    /// Object kind.
    pub kind: ObjectKind,
    /// Stable ID.
    pub id: StableId,
    /// Base digest when present.
    pub base: Option<SemanticDigest>,
    /// Head digest when present.
    pub head: Option<SemanticDigest>,
    /// True for key/digest graph differences, false for dependency closure.
    pub direct: bool,
}

/// Computes deterministic direct changes and transitive reference closure.
#[must_use]
pub fn change_impacts(base: &SpecGraph, head: &SpecGraph) -> Vec<Impact> {
    let base_objects = object_map(base);
    let head_objects = object_map(head);
    let all_keys = base_objects
        .keys()
        .chain(head_objects.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut impacted = BTreeSet::new();
    let mut direct_keys = BTreeSet::new();
    for key in all_keys {
        let base_digest = base_objects.get(&key).copied();
        let head_digest = head_objects.get(&key).copied();
        if base_digest != head_digest {
            direct_keys.insert(key.clone());
            impacted.insert(key);
        }
    }

    let base_dependencies = additional_object_dependencies(base);
    let head_dependencies = additional_object_dependencies(head);
    loop {
        let before = impacted.len();
        for (graph, additional) in [(base, &base_dependencies), (head, &head_dependencies)] {
            for meta in graph.object_meta() {
                let key = (meta.kind(), meta.id().clone());
                if object_dependencies(meta, additional)
                    .iter()
                    .any(|dependency| impacted.contains(dependency))
                {
                    impacted.insert(key);
                }
            }
        }
        if impacted.len() == before {
            break;
        }
    }

    impacted
        .into_iter()
        .map(|(kind, id)| Impact {
            kind,
            base: base_objects.get(&(kind, id.clone())).copied(),
            head: head_objects.get(&(kind, id.clone())).copied(),
            direct: direct_keys.contains(&(kind, id.clone())),
            id,
        })
        .collect()
}

fn object_map(graph: &SpecGraph) -> BTreeMap<(ObjectKind, StableId), SemanticDigest> {
    graph
        .object_meta()
        .into_iter()
        .map(|meta| ((meta.kind(), meta.id().clone()), meta.digest()))
        .collect()
}

type ObjectDependencyMap = BTreeMap<(ObjectKind, StableId), BTreeSet<(ObjectKind, StableId)>>;

fn atom_dependencies(atoms: &[AtomRevisionRef]) -> BTreeSet<(ObjectKind, StableId)> {
    atoms
        .iter()
        .map(|reference| (ObjectKind::Atom, reference.id().clone()))
        .collect()
}

fn additional_object_dependencies(graph: &SpecGraph) -> ObjectDependencyMap {
    let mut dependencies = ObjectDependencyMap::new();
    for object in &graph.tombstones {
        dependencies.insert(
            (ObjectKind::Tombstone, object.meta().id().clone()),
            atom_dependencies(object.replacements()),
        );
    }
    for object in &graph.decision_surfaces {
        dependencies.insert(
            (ObjectKind::DecisionSurface, object.meta().id().clone()),
            object
                .dispositions()
                .iter()
                .filter_map(|row| row.authority())
                .map(|authority| (ObjectKind::Atom, authority.id().clone()))
                .collect(),
        );
    }
    for object in &graph.evidence_carriers {
        dependencies.insert(
            (ObjectKind::EvidenceCarrier, object.meta().id().clone()),
            atom_dependencies(object.atoms()),
        );
    }
    for object in &graph.implementation_links {
        dependencies.insert(
            (ObjectKind::ImplementationLink, object.meta().id().clone()),
            atom_dependencies(object.atoms()),
        );
    }
    for object in &graph.external_trace_links {
        dependencies.insert(
            (ObjectKind::ExternalTraceLink, object.meta().id().clone()),
            atom_dependencies(object.atoms()),
        );
    }
    for object in &graph.implementation_bindings {
        dependencies.insert(
            (
                ObjectKind::ImplementationBinding,
                object.meta().id().clone(),
            ),
            atom_dependencies(core::slice::from_ref(object.atom())),
        );
    }
    for object in &graph.finite_models {
        dependencies.insert(
            (ObjectKind::FiniteModel, object.meta().id().clone()),
            atom_dependencies(object.checked_invariants()),
        );
    }
    for object in &graph.formal_properties {
        dependencies.insert(
            (ObjectKind::FormalProperty, object.meta().id().clone()),
            atom_dependencies(object.atoms()),
        );
    }
    for object in &graph.proof_records {
        dependencies.insert(
            (ObjectKind::ProofRecord, object.meta().id().clone()),
            atom_dependencies(object.atoms()),
        );
    }
    for object in &graph.debt_records {
        dependencies.insert(
            (ObjectKind::DebtRecord, object.meta().id().clone()),
            object
                .obligations()
                .iter()
                .map(|obligation| (ObjectKind::CoveragePolicy, obligation.policy_id().clone()))
                .collect(),
        );
    }
    for object in &graph.waivers {
        dependencies.insert(
            (ObjectKind::Waiver, object.meta().id().clone()),
            object
                .obligations()
                .iter()
                .map(|obligation| (ObjectKind::CoveragePolicy, obligation.policy_id().clone()))
                .collect(),
        );
    }
    dependencies
}

fn object_dependencies(
    meta: &ObjectMeta,
    additional: &ObjectDependencyMap,
) -> BTreeSet<(ObjectKind, StableId)> {
    meta.references()
        .iter()
        .map(|reference| (reference.kind(), reference.id().clone()))
        .chain(
            additional
                .get(&(meta.kind(), meta.id().clone()))
                .into_iter()
                .flatten()
                .cloned(),
        )
        .collect()
}

/// Validates exact one-to-one manifest closure for every impact.
#[must_use]
pub fn disposition_closure(impacts: &[Impact], manifest: &ChangeManifest) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for impact in impacts {
        let matches = manifest
            .dispositions()
            .iter()
            .filter(|disposition| {
                disposition.impacted_kind() == impact.kind
                    && *disposition.impacted_id() == impact.id
            })
            .collect::<Vec<_>>();
        let Some(disposition) = matches.first() else {
            diagnostics.push(Diagnostic::object(
                "CHG-DISPOSITION-MISSING",
                impact.kind,
                &impact.id,
            ));
            continue;
        };
        if disposition.base() != impact.base || disposition.head() != impact.head {
            diagnostics.push(Diagnostic::object(
                "CHG-DISPOSITION-STALE-OR-INAPPLICABLE",
                impact.kind,
                &impact.id,
            ));
        }
    }
    for disposition in manifest.dispositions() {
        if !impacts.iter().any(|impact| {
            impact.kind == disposition.impacted_kind() && impact.id == *disposition.impacted_id()
        }) {
            diagnostics.push(Diagnostic::object(
                "CHG-DISPOSITION-INAPPLICABLE",
                disposition.impacted_kind(),
                disposition.impacted_id(),
            ));
        }
    }
    diagnostics.sort_unstable();
    diagnostics.dedup();
    diagnostics
}

/// Explicit immutable inputs for complete disposition and weakening closure.
pub struct DispositionProtocolContext<'a> {
    base: &'a SpecGraph,
    head: &'a SpecGraph,
    artifact_index: &'a ArtifactIndex,
    ratchets: &'a [DerivedPolicyRatchet],
}

impl<'a> DispositionProtocolContext<'a> {
    /// Constructs one repository-independent protocol-validation context.
    #[must_use]
    pub const fn new(
        base: &'a SpecGraph,
        head: &'a SpecGraph,
        artifact_index: &'a ArtifactIndex,
        ratchets: &'a [DerivedPolicyRatchet],
    ) -> Self {
        Self {
            base,
            head,
            artifact_index,
            ratchets,
        }
    }

    /// Base semantic graph.
    #[must_use]
    pub const fn base(&self) -> &'a SpecGraph {
        self.base
    }

    /// Head semantic graph.
    #[must_use]
    pub const fn head(&self) -> &'a SpecGraph {
        self.head
    }

    /// Derived policy ratchets used by weakening closure.
    #[must_use]
    pub const fn ratchets(&self) -> &'a [DerivedPolicyRatchet] {
        self.ratchets
    }
}

fn disposition_diagnostic(code: &'static str, disposition: &ChangeDisposition) -> Diagnostic {
    Diagnostic::object(code, disposition.impacted_kind(), disposition.impacted_id())
}

fn exact_object_exists(graph: &SpecGraph, reference: &ObjectRef) -> bool {
    let expected = (reference.kind(), reference.id(), reference.digest());
    graph
        .object_meta()
        .into_iter()
        .any(|meta| (meta.kind(), meta.id(), meta.digest()) == expected)
}

fn related_ratchets<'a>(
    disposition: &ChangeDisposition,
    ratchets: &'a [DerivedPolicyRatchet],
) -> Vec<&'a DerivedPolicyRatchet> {
    ratchets
        .iter()
        .filter(|ratchet| {
            let affected = (disposition.impacted_kind(), disposition.impacted_id());
            let direct_policy = affected == (ObjectKind::CoveragePolicy, ratchet.policy_id());
            let removed_subject = ratchet.removed_obligations().iter().any(|obligation| {
                (obligation.subject().kind(), obligation.subject().id()) == affected
            });
            matches!(
                (ratchet.classification(), direct_policy, removed_subject),
                (PolicyChangeClassification::Weakening, true, _)
                    | (PolicyChangeClassification::Weakening, _, true)
            )
        })
        .collect()
}

fn policy_authority<'a>(
    context: &'a DispositionProtocolContext<'_>,
    disposition: &ChangeDisposition,
    ratchets: &[&DerivedPolicyRatchet],
) -> Option<&'a str> {
    let policy_id = if disposition.impacted_kind() == ObjectKind::CoveragePolicy {
        Some(disposition.impacted_id())
    } else if let Some(ratchet) = ratchets.first() {
        Some(ratchet.policy_id())
    } else {
        context
            .base
            .coverage_policies
            .iter()
            .chain(&context.head.coverage_policies)
            .find(|policy| {
                policy.subjects().iter().any(|subject| {
                    (subject.kind(), subject.id())
                        == (disposition.impacted_kind(), disposition.impacted_id())
                })
            })
            .map(|policy| policy.meta().id())
    }?;
    context
        .base
        .coverage_policies
        .iter()
        .chain(&context.head.coverage_policies)
        .find(|policy| policy.meta().id() == policy_id)
        .map(|policy| policy.active_approval().authority())
}

fn approval_record_current(approval: &ApprovalBinding, artifacts: &ArtifactIndex) -> bool {
    artifacts
        .entries()
        .iter()
        .any(|entry| entry.path() == approval.record() && entry.raw_digest() == approval.digest())
}

fn reaffirmation_record<'a>(
    disposition: &ChangeDisposition,
    graph: &'a SpecGraph,
) -> Option<&'a crate::model::Reaffirmation> {
    disposition.reaffirmation_digest().and_then(|digest| {
        let expected = (
            digest,
            disposition.impacted_kind(),
            disposition.impacted_id(),
            disposition.base(),
            disposition.head(),
        );
        graph.reaffirmations.iter().find(|record| {
            (
                record.meta().digest(),
                record.affected_object_kind(),
                record.affected_object_id(),
                Some(record.base_object_digest()),
                Some(record.head_object_digest()),
            ) == expected
        })
    })
}

fn active_waiver<'a>(
    disposition: &ChangeDisposition,
    graph: &'a SpecGraph,
) -> Option<&'a crate::model::Waiver> {
    disposition.waiver().and_then(|reference| {
        let expected = (WaiverLifecycle::Active, reference.id(), reference.digest());
        graph.waivers.iter().find(|waiver| {
            (
                waiver.lifecycle(),
                waiver.meta().id(),
                waiver.meta().digest(),
            ) == expected
        })
    })
}

fn waiver_applies(
    waiver: &crate::model::Waiver,
    disposition: &ChangeDisposition,
    ratchets: &[&DerivedPolicyRatchet],
) -> bool {
    let affected_key = (disposition.impacted_kind(), disposition.impacted_id());
    let affected = waiver
        .objects()
        .iter()
        .chain(waiver.obligations().iter().map(ObligationRef::subject))
        .any(|object| (object.kind(), object.id()) == affected_key);
    let weakened = ratchets.iter().all(|ratchet| {
        ratchet
            .removed_obligations()
            .iter()
            .all(|required| waiver.obligations().contains(required))
    });
    (affected, weakened) == (true, true)
}

fn active_debt_covers(graph: &SpecGraph, required: &[&DerivedPolicyRatchet]) -> bool {
    required.iter().all(|ratchet| {
        ratchet.removed_obligations().iter().all(|obligation| {
            graph
                .debt_records
                .iter()
                .filter(|debt| debt.lifecycle() == DebtLifecycle::Active)
                .any(|debt| debt.obligations().contains(obligation))
        })
    })
}

fn related_active_debt(graph: &SpecGraph, disposition: &ChangeDisposition) -> bool {
    let affected = (disposition.impacted_kind(), disposition.impacted_id());
    graph
        .debt_records
        .iter()
        .filter(|debt| debt.lifecycle() == DebtLifecycle::Active)
        .flat_map(|debt| {
            debt.objects()
                .iter()
                .chain(debt.obligations().iter().map(ObligationRef::subject))
        })
        .any(|object| (object.kind(), object.id()) == affected)
}

fn related_active_waiver(graph: &SpecGraph, disposition: &ChangeDisposition) -> bool {
    let affected = (disposition.impacted_kind(), disposition.impacted_id());
    graph
        .waivers
        .iter()
        .filter(|waiver| waiver.lifecycle() == WaiverLifecycle::Active)
        .flat_map(|waiver| {
            waiver
                .objects()
                .iter()
                .chain(waiver.obligations().iter().map(ObligationRef::subject))
        })
        .any(|object| (object.kind(), object.id()) == affected)
}

fn weakening_replacements(
    context: &DispositionProtocolContext<'_>,
    disposition: &ChangeDisposition,
    ratchets: &[&DerivedPolicyRatchet],
) -> BTreeSet<ObjectRef> {
    let mut replacements = ratchets
        .iter()
        .filter_map(|ratchet| {
            context
                .head
                .coverage_policies
                .iter()
                .find(|policy| policy.meta().id() == ratchet.policy_id())
        })
        .flat_map(|policy| policy.replacements().iter().cloned())
        .collect::<BTreeSet<_>>();
    let affected = (disposition.impacted_kind(), disposition.impacted_id());
    let direct_replacements = context
        .head
        .coverage_policies
        .iter()
        .map(|object| {
            (
                ObjectKind::CoveragePolicy,
                object.meta().id(),
                object.replacements(),
            )
        })
        .chain(context.head.debt_records.iter().map(|object| {
            (
                ObjectKind::DebtRecord,
                object.meta().id(),
                object.replacements(),
            )
        }))
        .chain(context.head.waivers.iter().map(|object| {
            (
                ObjectKind::Waiver,
                object.meta().id(),
                object.replacements(),
            )
        }))
        .filter(|(kind, id, _)| (*kind, *id) == affected)
        .flat_map(|(_, _, values)| values.iter().cloned());
    replacements.extend(direct_replacements);
    replacements
}

/// Validates complete exact disposition records and weakening protocol closure.
///
/// The result remains diagnostic-only: debt and waiver authorization never
/// changes technical impact or obligation satisfaction.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn disposition_protocol_closure(
    impacts: &[Impact],
    manifest: &ChangeManifest,
    context: &DispositionProtocolContext<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = disposition_closure(impacts, manifest)
        .into_iter()
        .filter(|diagnostic| {
            !matches!(
                diagnostic.code.as_str(),
                "CHG-DISPOSITION-STALE-OR-INAPPLICABLE" | "CHG-DISPOSITION-INAPPLICABLE"
            )
        })
        .collect::<Vec<_>>();
    for disposition in manifest.dispositions() {
        let affected = (disposition.impacted_kind(), disposition.impacted_id());
        let Some(impact) = impacts
            .iter()
            .find(|impact| (impact.kind, &impact.id) == affected)
        else {
            diagnostics.push(disposition_diagnostic(
                "CHG-DISPOSITION-APPLICABILITY",
                disposition,
            ));
            continue;
        };
        if disposition.base() != impact.base {
            diagnostics.push(disposition_diagnostic(
                "CHG-DISPOSITION-BASE-DIGEST",
                disposition,
            ));
        }
        if disposition.head() != impact.head {
            diagnostics.push(disposition_diagnostic(
                "CHG-DISPOSITION-HEAD-DIGEST",
                disposition,
            ));
        }
        let ratchets = related_ratchets(disposition, context.ratchets);
        let expected_authority = policy_authority(context, disposition, &ratchets);
        let weakening_kind_valid = ratchets.is_empty()
            || match (disposition.disposition(), disposition.head()) {
                (ChangeDispositionKind::Revised, Some(_)) => ratchets
                    .iter()
                    .all(|ratchet| ratchet.head_policy().is_some()),
                (ChangeDispositionKind::Retired, None) => ratchets
                    .iter()
                    .any(|ratchet| ratchet.head_policy().is_none()),
                (ChangeDispositionKind::Waived, _) => true,
                _ => false,
            };
        if !weakening_kind_valid {
            diagnostics.push(disposition_diagnostic(
                "CHG-DISPOSITION-APPLICABILITY",
                disposition,
            ));
        }

        let mut reaffirmation_missing = false;
        let approval = match disposition.disposition() {
            ChangeDispositionKind::Reaffirmed => {
                let record = reaffirmation_record(disposition, context.head);
                reaffirmation_missing = record.is_none();
                if reaffirmation_missing {
                    diagnostics.push(disposition_diagnostic(
                        "CHG-DISPOSITION-RECORD",
                        disposition,
                    ));
                }
                record.map(crate::model::Reaffirmation::review_approval)
            }
            _ => manifest.approval(),
        };
        if let Some(approval) = approval {
            if !approval_record_current(approval, context.artifact_index) {
                diagnostics.push(disposition_diagnostic(
                    "CHG-DISPOSITION-RECORD",
                    disposition,
                ));
            } else if expected_authority.is_some_and(|authority| authority != approval.authority())
            {
                diagnostics.push(disposition_diagnostic(
                    "CHG-DISPOSITION-APPROVAL",
                    disposition,
                ));
            }
        } else if (reaffirmation_missing, ratchets.is_empty()) == (false, false) {
            diagnostics.push(disposition_diagnostic(
                "CHG-DISPOSITION-APPROVAL",
                disposition,
            ));
        }

        let expected_replacements = weakening_replacements(context, disposition, &ratchets);
        let actual_replacements = disposition
            .replacements()
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let replacements_current = actual_replacements
            .iter()
            .all(|reference| exact_object_exists(context.head, reference));
        let expected_mismatch = (
            !expected_replacements.is_empty(),
            expected_replacements == actual_replacements,
        ) == (true, false);
        let replacements_forbidden = (
            matches!(
                disposition.disposition(),
                ChangeDispositionKind::Reaffirmed | ChangeDispositionKind::Waived
            ),
            actual_replacements.is_empty(),
        ) == (true, false);
        if (
            expected_mismatch,
            replacements_current,
            replacements_forbidden,
        ) != (false, true, false)
        {
            diagnostics.push(disposition_diagnostic(
                "CHG-DISPOSITION-REPLACEMENT",
                disposition,
            ));
        }

        if ratchets.is_empty() {
            if disposition.disposition() == ChangeDispositionKind::Reaffirmed
                && related_active_debt(context.head, disposition)
            {
                diagnostics.push(disposition_diagnostic("CHG-DISPOSITION-DEBT", disposition));
            }
            if disposition.disposition() == ChangeDispositionKind::Reaffirmed
                && related_active_waiver(context.head, disposition)
            {
                diagnostics.push(disposition_diagnostic(
                    "CHG-DISPOSITION-WAIVER",
                    disposition,
                ));
            }
        } else {
            if !active_debt_covers(context.head, &ratchets) {
                diagnostics.push(disposition_diagnostic("CHG-DISPOSITION-DEBT", disposition));
            }
            let blocking = ratchets.iter().any(|ratchet| {
                context.base.coverage_policies.iter().any(|policy| {
                    (policy.meta().id(), policy.mode())
                        == (ratchet.policy_id(), EnforcementMode::Blocking)
                })
            });
            if blocking {
                let waiver_valid = context.head.waivers.iter().any(|waiver| {
                    let authority_valid = expected_authority.is_none_or(|authority| {
                        (
                            waiver.active_approval().authority(),
                            approval_record_current(
                                waiver.active_approval(),
                                context.artifact_index,
                            ),
                        ) == (authority, true)
                    });
                    (
                        waiver.lifecycle(),
                        waiver_applies(waiver, disposition, &ratchets),
                        authority_valid,
                    ) == (WaiverLifecycle::Active, true, true)
                });
                if !waiver_valid {
                    diagnostics.push(disposition_diagnostic(
                        "CHG-DISPOSITION-WAIVER",
                        disposition,
                    ));
                }
            }
        }

        if disposition.disposition() == ChangeDispositionKind::Waived {
            let waiver = active_waiver(disposition, context.head);
            if waiver.is_none_or(|waiver| {
                let authority_valid = expected_authority
                    .is_none_or(|authority| waiver.active_approval().authority() == authority);
                (
                    waiver_applies(waiver, disposition, &ratchets),
                    approval_record_current(waiver.active_approval(), context.artifact_index),
                    authority_valid,
                ) != (true, true, true)
            }) {
                diagnostics.push(disposition_diagnostic(
                    "CHG-DISPOSITION-WAIVER",
                    disposition,
                ));
            }
        }
    }
    for ratchet in context
        .ratchets
        .iter()
        .filter(|ratchet| ratchet.classification() == PolicyChangeClassification::Weakening)
    {
        let represented = manifest.dispositions().iter().any(|disposition| {
            related_ratchets(disposition, core::slice::from_ref(ratchet))
                .iter()
                .any(|related| related.policy_id() == ratchet.policy_id())
        });
        if !represented {
            diagnostics.push(Diagnostic::object(
                "CHG-DISPOSITION-MISSING",
                ObjectKind::CoveragePolicy,
                ratchet.policy_id(),
            ));
        }
    }
    diagnostics.sort_unstable();
    diagnostics.dedup();
    diagnostics
}

fn canonical_object_set<T, F>(values: &[T], meta: F) -> Result<CanonicalValue, EncodingError>
where
    F: Fn(&T) -> &ObjectMeta,
{
    let values = values
        .iter()
        .map(|value| meta(value).canonical_value().clone())
        .collect();
    canonicalize_set(values, Clone::clone).map(CanonicalValue::Set)
}

/// Computes the exact version 3 semantic graph digest with the first build member.
///
/// # Errors
///
/// Returns an encoding error if an object set violates canonical constraints.
///
/// # Panics
///
/// Panics if the build contains no digest adapter.
#[allow(clippy::expect_used)]
pub fn semantic_graph_digest(graph: &SpecGraph) -> Result<SemanticDigest, EncodingError> {
    let active = ActiveAlgorithm::build_default()
        .expect("the build default must have an enabled digest adapter");
    semantic_graph_digest_with(active, graph)
}

/// Computes the exact version 3 semantic graph digest with one active member.
///
/// # Errors
///
/// Returns an encoding error if an object set violates canonical constraints.
pub fn semantic_graph_digest_with(
    active: ActiveAlgorithm,
    graph: &SpecGraph,
) -> Result<SemanticDigest, EncodingError> {
    Ok(semantic_digest_trusted_with(
        active,
        "spec-graph-semantic",
        alloc::vec![
            (1, canonical_object_set(&graph.atoms, |value| value.meta())?),
            (
                2,
                canonical_object_set(&graph.tombstones, |value| value.meta())?,
            ),
            (
                3,
                canonical_object_set(&graph.decision_surfaces, |value| value.meta())?,
            ),
            (
                4,
                canonical_object_set(&graph.evidence_carriers, |value| value.meta())?,
            ),
            (
                5,
                canonical_object_set(&graph.implementation_links, |value| value.meta())?,
            ),
            (
                6,
                canonical_object_set(&graph.derivations, |value| value.meta())?,
            ),
            (
                7,
                canonical_object_set(&graph.coverage_policies, |value| value.meta())?,
            ),
            (
                8,
                canonical_object_set(&graph.debt_records, |value| value.meta())?,
            ),
            (
                9,
                canonical_object_set(&graph.waivers, |value| value.meta())?
            ),
            (
                10,
                canonical_object_set(&graph.oracles, |value| value.meta())?
            ),
            (
                11,
                canonical_object_set(&graph.finite_models, |value| value.meta())?,
            ),
            (
                12,
                canonical_object_set(&graph.formal_properties, |value| value.meta())?,
            ),
            (
                13,
                canonical_object_set(&graph.assumptions, |value| value.meta())?,
            ),
            (
                14,
                canonical_object_set(&graph.proof_records, |value| value.meta())?,
            ),
            (
                15,
                canonical_object_set(&graph.reaffirmations, |value| value.meta())?,
            ),
            (
                16,
                canonical_object_set(&graph.external_trace_links, |value| value.meta())?,
            ),
        ],
    ))
}

/// Computes the version 3 direct, transitive, and policy-ratchet impact-set identity.
///
/// # Errors
///
/// Rejects duplicate entries, overlap between direct and transitive sets, or
/// noncanonical nested values.
///
/// # Panics
///
/// Panics if the build contains no digest adapter.
#[allow(clippy::expect_used)]
pub fn impact_set_digest(
    direct_changes: Vec<ObjectRef>,
    transitive_impacts: Vec<ObjectRef>,
    policy_ratchets: Vec<PolicyRatchetImpact>,
) -> Result<SemanticDigest, EncodingError> {
    let active = ActiveAlgorithm::build_default()
        .expect("the build default must have an enabled digest adapter");
    impact_set_digest_with(active, direct_changes, transitive_impacts, policy_ratchets)
}

/// Computes the version 3 impact-set identity with one active registry member.
///
/// # Errors
///
/// Rejects duplicate entries, overlap between direct and transitive sets, or
/// noncanonical nested values.
pub fn impact_set_digest_with(
    active: ActiveAlgorithm,
    direct_changes: Vec<ObjectRef>,
    transitive_impacts: Vec<ObjectRef>,
    policy_ratchets: Vec<PolicyRatchetImpact>,
) -> Result<SemanticDigest, EncodingError> {
    let direct_changes = canonicalize_set(direct_changes, object_reference_value)?;
    let transitive_impacts = canonicalize_set(transitive_impacts, object_reference_value)?;
    if direct_changes.iter().any(|direct| {
        transitive_impacts
            .iter()
            .any(|transitive| direct == transitive)
    }) {
        return Err(EncodingError::DuplicateSetValue);
    }
    let policy_ratchets = canonicalize_set(policy_ratchets, PolicyRatchetImpact::canonical_value)?;
    Ok(semantic_digest_trusted_with(
        active,
        "impact-set",
        alloc::vec![
            (
                1,
                CanonicalValue::Set(direct_changes.iter().map(object_reference_value).collect()),
            ),
            (
                2,
                CanonicalValue::Set(
                    transitive_impacts
                        .iter()
                        .map(object_reference_value)
                        .collect(),
                ),
            ),
            (
                3,
                CanonicalValue::Set(
                    policy_ratchets
                        .iter()
                        .map(PolicyRatchetImpact::canonical_value)
                        .collect(),
                ),
            ),
        ],
    ))
}

/// Binds the semantic graph to exact source-map, repository-config, and schema-tool inputs.
///
/// # Errors
///
/// Returns an error only if the complete field table cannot be encoded.
///
/// # Panics
///
/// Panics if the build contains no digest adapter.
#[allow(clippy::expect_used)]
pub fn complete_graph_digest(
    semantic_graph: SemanticDigest,
    source_map: SemanticDigest,
    repository_config: SemanticDigest,
    schema_tool_lock: SemanticDigest,
) -> Result<SemanticDigest, EncodingError> {
    let active = ActiveAlgorithm::build_default()
        .expect("the build default must have an enabled digest adapter");
    complete_graph_digest_with(
        active,
        semantic_graph,
        source_map,
        repository_config,
        schema_tool_lock,
    )
}

/// Binds complete graph inputs with one active registry member.
///
/// # Errors
///
/// Returns an error only if the complete field table cannot be encoded.
pub fn complete_graph_digest_with(
    active: ActiveAlgorithm,
    semantic_graph: SemanticDigest,
    source_map: SemanticDigest,
    repository_config: SemanticDigest,
    schema_tool_lock: SemanticDigest,
) -> Result<SemanticDigest, EncodingError> {
    Ok(semantic_digest_trusted_with(
        active,
        "spec-graph-complete",
        alloc::vec![
            (1, CanonicalValue::Digest(semantic_graph)),
            (2, CanonicalValue::Digest(source_map)),
            (3, CanonicalValue::Digest(repository_config)),
            (4, CanonicalValue::Digest(schema_tool_lock)),
        ],
    ))
}

/// Deterministic static report value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticReport {
    /// Complete or semantic graph identity selected by configuration.
    pub graph: SemanticDigest,
    /// Canonically ordered diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Deterministic execution report value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionReport {
    /// Unchanged static graph identity.
    pub graph: SemanticDigest,
    /// Normalized verdicts in canonical order.
    pub verdicts: Vec<(String, Verdict)>,
    /// Orthogonal assurance classes actually established.
    pub assurance: Vec<AssuranceClass>,
    /// Canonically ordered diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Constructs an order-independent execution report without raw logs.
#[must_use]
pub fn execution_report(
    graph: SemanticDigest,
    mut verdicts: Vec<(String, Verdict)>,
    assurance: Vec<AssuranceClass>,
    mut diagnostics: Vec<Diagnostic>,
) -> ExecutionReport {
    verdicts.sort_unstable();
    diagnostics.sort_unstable();
    diagnostics.dedup();
    ExecutionReport {
        graph,
        verdicts,
        assurance,
        diagnostics,
    }
}

/// Deterministic provider-neutral impact report value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImpactReport {
    /// Base complete graph.
    pub base: SemanticDigest,
    /// Head complete graph.
    pub head: SemanticDigest,
    /// Canonical direct and transitive impacts.
    pub impacts: Vec<Impact>,
    /// Closure diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Returns true only for irreversible legal governance lifecycle progression.
#[must_use]
pub const fn governance_transition_valid(
    base_policy: Option<PolicyLifecycle>,
    head_policy: Option<PolicyLifecycle>,
    base_debt: Option<DebtLifecycle>,
    head_debt: Option<DebtLifecycle>,
    base_waiver: Option<WaiverLifecycle>,
    head_waiver: Option<WaiverLifecycle>,
) -> bool {
    policy_transition_valid(base_policy, head_policy)
        && debt_transition_valid(base_debt, head_debt)
        && waiver_transition_valid(base_waiver, head_waiver)
}

const fn policy_transition_valid(
    base: Option<PolicyLifecycle>,
    head: Option<PolicyLifecycle>,
) -> bool {
    matches!(
        (base, head),
        (None, Some(PolicyLifecycle::Active))
            | (
                Some(PolicyLifecycle::Active),
                Some(
                    PolicyLifecycle::Active
                        | PolicyLifecycle::Superseded
                        | PolicyLifecycle::Retired
                )
            )
            | (
                Some(PolicyLifecycle::Superseded),
                Some(PolicyLifecycle::Superseded)
            )
            | (
                Some(PolicyLifecycle::Retired),
                Some(PolicyLifecycle::Retired)
            )
    )
}

const fn debt_transition_valid(base: Option<DebtLifecycle>, head: Option<DebtLifecycle>) -> bool {
    matches!(
        (base, head),
        (None, Some(DebtLifecycle::Active))
            | (
                Some(DebtLifecycle::Active),
                Some(DebtLifecycle::Active | DebtLifecycle::Discharged | DebtLifecycle::Superseded)
            )
            | (
                Some(DebtLifecycle::Discharged),
                Some(DebtLifecycle::Discharged)
            )
            | (
                Some(DebtLifecycle::Superseded),
                Some(DebtLifecycle::Superseded)
            )
    )
}

const fn waiver_transition_valid(
    base: Option<WaiverLifecycle>,
    head: Option<WaiverLifecycle>,
) -> bool {
    matches!(
        (base, head),
        (None, Some(WaiverLifecycle::Active))
            | (
                Some(WaiverLifecycle::Active),
                Some(
                    WaiverLifecycle::Active
                        | WaiverLifecycle::Discharged
                        | WaiverLifecycle::Revoked
                        | WaiverLifecycle::Superseded
                )
            )
            | (
                Some(WaiverLifecycle::Discharged),
                Some(WaiverLifecycle::Discharged)
            )
            | (
                Some(WaiverLifecycle::Revoked),
                Some(WaiverLifecycle::Revoked)
            )
            | (
                Some(WaiverLifecycle::Superseded),
                Some(WaiverLifecycle::Superseded)
            )
    )
}
