//! Schema-v2 domain values.

use crate::identifier::{
    ExternalTraceTarget, IdentifierError, NamespacePrefix, SchemaId, ScopeId, SourceItemKind,
    StableId,
};
use crate::value::{
    ActiveAlgorithm, CanonicalValue, DigestTextError, EncodingError, RepositoryPath,
    SemanticDigest, canonicalize_set, canonicalize_validated_set, semantic_digest_trusted,
    semantic_digest_trusted_with,
};
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Stable object-kind serialization identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[repr(u16)]
pub enum ObjectKind {
    /// Requirement atom.
    Atom = 1,
    /// Permanent terminal atom record.
    Tombstone = 2,
    /// Governed finite decision surface.
    DecisionSurface = 3,
    /// Syntax-attached evidence registration.
    EvidenceCarrier = 4,
    /// Navigational implementation link.
    ImplementationLink = 5,
    /// Bound generated or mirrored relation.
    Derivation = 6,
    /// Coverage and enforcement policy.
    CoveragePolicy = 7,
    /// Visible remediation debt.
    DebtRecord = 8,
    /// Process-only exception.
    Waiver = 9,
    /// Effectful checker declaration.
    Oracle = 10,
    /// Bounded finite reference model.
    FiniteModel = 11,
    /// Human-approved formalization.
    FormalProperty = 12,
    /// Explicit proof assumption.
    Assumption = 13,
    /// Exact formal result binding.
    ProofRecord = 14,
    /// Parser-canonical source item.
    ParsedItem = 15,
    /// Review-only change reaffirmation.
    Reaffirmation = 16,
    /// Provider-neutral change manifest.
    ChangeManifest = 17,
    /// One exact impact disposition.
    ChangeDisposition = 18,
    /// Repository-owned external navigation link.
    ExternalTraceLink = 19,
    /// Immutable content-addressed implementation binding.
    ImplementationBinding = 20,
}

/// Atom semantic category.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum AtomKind {
    /// Observable behavior.
    Behavioral = 1,
    /// Source or graph structure.
    Structural = 2,
    /// Constraint over valid states.
    Constraint = 3,
}

/// Combined live and terminal lifecycle state.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum LifecycleState {
    /// Authored but not ratified.
    Proposed = 1,
    /// Ratified live authority.
    Ratified = 2,
    /// Terminal replacement retirement.
    Superseded = 3,
    /// Terminal withdrawal.
    Withdrawn = 4,
}

/// Permanent stable-ID allocation reservation class.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum IdAllocationClass {
    /// The allocated ID currently denotes a live object.
    Live = 1,
    /// The allocated ID is permanently reserved by terminal history.
    Terminal = 2,
}

/// Explicit evidence role.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum EvidenceRole {
    /// Positive executable example.
    Positive = 1,
    /// Negative executable example.
    Negative = 2,
    /// Quantified sampled property.
    Property = 3,
    /// Exhaustive finite model.
    ExhaustiveModel = 4,
    /// Formal property or verifier carrier.
    FormalProperty = 5,
    /// Structural checker.
    Structural = 6,
}

/// Policy enforcement behavior.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum EnforcementMode {
    /// Report without changing the coverage exit class.
    Advisory = 1,
    /// Unsatisfied unwaived obligations block.
    Blocking = 2,
}

/// Closed authority expectation carried by one typed selector.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum AuthorityExpectation {
    /// Any selected authority is invalid.
    Forbidden = 1,
    /// Zero or more selected authorities are valid.
    Optional = 2,
    /// The selector must produce authority.
    Required = 3,
}

/// Irreversible policy lifecycle.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum PolicyLifecycle {
    /// Current obligation authority.
    Active = 1,
    /// Replaced terminal history.
    Superseded = 2,
    /// Retired terminal history.
    Retired = 3,
}

/// Irreversible debt lifecycle.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum DebtLifecycle {
    /// Current remediation debt.
    Active = 1,
    /// Evidence-backed terminal discharge.
    Discharged = 2,
    /// Terminal replacement by other debt.
    Superseded = 3,
}

/// Irreversible waiver lifecycle.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum WaiverLifecycle {
    /// Current process authorization.
    Active = 1,
    /// Satisfied terminal discharge.
    Discharged = 2,
    /// Terminal cancellation.
    Revoked = 3,
    /// Terminal replacement.
    Superseded = 4,
}

/// Core decision-surface disposition.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum SurfaceDispositionKind {
    /// Positive normative decision.
    Normative = 1,
    /// Normative prohibition.
    Prohibited = 2,
    /// Explicit debt-bound deferral.
    Deferred = 3,
    /// Explicit non-applicability decision.
    NotApplicable = 4,
}

/// Navigational implementation-link relation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum NavigationKind {
    /// The parsed item implements the bound atom revisions.
    Implements = 1,
    /// The parsed item owns the bound atom revisions.
    Owns = 2,
    /// The parsed item consumes the bound atom revisions.
    Consumes = 3,
    /// The parsed item configures the bound atom revisions.
    Configures = 4,
}

/// Closed semantic relation for a repository-owned external link.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum ExternalTraceRelation {
    /// The target tracks work for the atom set.
    Tracks = 1,
    /// The target contains related discussion.
    Discusses = 2,
    /// The target documents the atom set.
    Documents = 3,
    /// The target reviews the atom set.
    Reviews = 4,
}

impl ExternalTraceRelation {
    /// Parses one closed relation name.
    ///
    /// # Errors
    ///
    /// Returns a local invariant error for an unknown name.
    pub fn parse(input: &str) -> Result<Self, ConstructionError> {
        match input {
            "tracks" => Ok(Self::Tracks),
            "discusses" => Ok(Self::Discusses),
            "documents" => Ok(Self::Documents),
            "reviews" => Ok(Self::Reviews),
            _ => Err(ConstructionError::LocalInvariant),
        }
    }

    /// Returns the canonical relation name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tracks => "tracks",
            Self::Discusses => "discusses",
            Self::Documents => "documents",
            Self::Reviews => "reviews",
        }
    }
}

/// Closed artifact-binding variant identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum ArtifactBindingKind {
    /// Exact semantic graph object.
    Object = 1,
    /// Exact selected repository artifact.
    RepositoryArtifact = 2,
}

/// Declared derivation relation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum DerivationRelation {
    /// Source and target raw bytes are equal.
    ByteEqual = 1,
    /// Targets are generated from sources.
    GeneratedFrom = 2,
    /// Source and target structures correspond under the declared scope.
    StructurallyEquivalent = 3,
    /// A pinned host schema defines the consistency relation.
    HostConsistency = 4,
}

/// Effectful oracle category.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum OracleKind {
    /// Ordinary executable test.
    Test = 1,
    /// Sampled property runner.
    Property = 2,
    /// Exhaustive finite-model runner.
    ExhaustiveModel = 3,
    /// Structural checker.
    Structural = 4,
    /// Formal verifier.
    Formal = 5,
    /// Derivation consistency checker.
    DerivationConsistency = 6,
    /// Pinned host-specific oracle.
    Host = 7,
}

/// Closed repository snapshot entry classification.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum EntryKind {
    /// Selected regular file.
    RegularFile = 1,
    /// Symbolic link rejected by no-follow snapshotting.
    Symlink = 2,
    /// Directory descent observation.
    Directory = 3,
    /// Version-control submodule boundary.
    Submodule = 4,
    /// Nested repository boundary.
    NestedRepository = 5,
    /// Any other unsupported filesystem entry.
    Other = 6,
}

/// Semantic role of one source-map entry.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum SourceRole {
    /// Sole normative authority.
    Authority = 1,
    /// Syntax-attached evidence carrier.
    Carrier = 2,
    /// Navigational implementation item.
    Implementation = 3,
    /// Governed decision-surface declaration.
    Surface = 4,
    /// Coverage-policy declaration.
    Policy = 5,
    /// Derivation declaration.
    Derivation = 6,
    /// Finite-model declaration.
    Model = 7,
    /// Formal-property declaration.
    Property = 8,
    /// Assumption declaration.
    Assumption = 9,
    /// Proof-record declaration.
    Proof = 10,
    /// Debt-record declaration.
    Debt = 11,
    /// Waiver declaration.
    Waiver = 12,
}

/// Normalized five-state execution verdict.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum Verdict {
    /// No selected execution result.
    NotRun = 1,
    /// Successful semantic result.
    Pass = 2,
    /// Validly executed failing result.
    Fail = 3,
    /// Runner or result-normalization error.
    Error = 4,
    /// Explicit timeout.
    Timeout = 5,
}

/// Scoped assurance fact, without cross-class ranking.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum AssuranceClass {
    /// Current syntax-attached registration.
    Linked = 1,
    /// Passing ordinary execution in exact scope.
    Tested = 2,
    /// Complete exploration of one finite model.
    ExhaustivelyChecked = 3,
    /// Accepted proof under exact bindings.
    FormallyProved = 4,
    /// Passing structural checker in exact scope.
    StructurallyEnforced = 5,
    /// Repository-owned approval or reaffirmation.
    ReviewOnly = 6,
}

/// Neutral impact disposition.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum ChangeDispositionKind {
    /// Same role deliberately reread under new bindings.
    Reaffirmed = 1,
    /// Claim or binding revised.
    Revised = 2,
    /// Claim retired.
    Retired = 3,
    /// Process continuation authorized without satisfaction.
    Waived = 4,
}

/// Shared stable identity, semantic digest, and closed object references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectMeta {
    kind: ObjectKind,
    id: StableId,
    digest: SemanticDigest,
    references: Vec<ObjectRef>,
    canonical_value: CanonicalValue,
}

impl ObjectMeta {
    fn for_fields(
        kind: ObjectKind,
        id: StableId,
        domain: &str,
        fields: Vec<(u16, CanonicalValue)>,
        references: Vec<ObjectRef>,
    ) -> Self {
        let canonical_value = CanonicalValue::Record(fields.clone());
        let digest = semantic_digest_trusted(domain, fields);
        Self {
            kind,
            id,
            digest,
            references,
            canonical_value,
        }
    }

    /// Computes one object identity under an explicit configured member.
    fn for_fields_with(
        active: ActiveAlgorithm,
        kind: ObjectKind,
        id: StableId,
        domain: &str,
        fields: Vec<(u16, CanonicalValue)>,
        references: Vec<ObjectRef>,
    ) -> Self {
        let canonical_value = CanonicalValue::Record(fields.clone());
        let digest = semantic_digest_trusted_with(active, domain, fields);
        Self {
            kind,
            id,
            digest,
            references,
            canonical_value,
        }
    }

    /// Returns the object kind.
    #[must_use]
    pub const fn kind(&self) -> ObjectKind {
        self.kind
    }

    /// Returns the stable object ID.
    #[must_use]
    pub const fn id(&self) -> &StableId {
        &self.id
    }

    /// Returns the complete semantic object digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Returns all direct object references.
    #[must_use]
    pub fn references(&self) -> &[ObjectRef] {
        &self.references
    }

    pub(crate) const fn canonical_value(&self) -> &CanonicalValue {
        &self.canonical_value
    }
}

fn require_nfc(text: &str) -> Result<(), ConstructionError> {
    CanonicalValue::Text(text.to_string()).encode()?;
    Ok(())
}

fn require_nonempty_nfc(text: &str) -> Result<(), ConstructionError> {
    if text.is_empty() {
        return Err(ConstructionError::EmptyText);
    }
    require_nfc(text)
}

fn require_same_digest_algorithm(
    left: SemanticDigest,
    right: SemanticDigest,
) -> Result<(), ConstructionError> {
    if left.algorithm() == right.algorithm() {
        Ok(())
    } else {
        Err(ConstructionError::DigestAlgorithmMismatch)
    }
}

fn contains_rfc_2119_term(statement: &str) -> bool {
    statement
        .split(|character: char| !character.is_ascii_uppercase())
        .any(|word| matches!(word, "MUST" | "SHALL" | "SHOULD" | "MAY"))
}

fn role_scope_compatible(role: EvidenceRole, schema: &str) -> bool {
    schema != "spec-provenance.scope.quantified-property/v1" || role == EvidenceRole::Property
}

const fn source_role_kind_compatible(role: SourceRole, kind: ObjectKind) -> bool {
    matches!(
        (role, kind),
        (
            SourceRole::Authority,
            ObjectKind::Atom | ObjectKind::Tombstone
        ) | (SourceRole::Carrier, ObjectKind::EvidenceCarrier)
            | (SourceRole::Implementation, ObjectKind::ImplementationLink)
            | (SourceRole::Surface, ObjectKind::DecisionSurface)
            | (SourceRole::Policy, ObjectKind::CoveragePolicy)
            | (SourceRole::Derivation, ObjectKind::Derivation)
            | (SourceRole::Model, ObjectKind::FiniteModel)
            | (SourceRole::Property, ObjectKind::FormalProperty)
            | (SourceRole::Assumption, ObjectKind::Assumption)
            | (SourceRole::Proof, ObjectKind::ProofRecord)
            | (SourceRole::Debt, ObjectKind::DebtRecord)
            | (SourceRole::Waiver, ObjectKind::Waiver)
    )
}

/// Exact reference to one object kind, ID, and digest.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObjectRef {
    kind: ObjectKind,
    id: StableId,
    digest: SemanticDigest,
}

impl ObjectRef {
    /// Constructs an exact object reference from an already-validated ID.
    #[must_use]
    pub fn new(kind: ObjectKind, id: StableId, digest: SemanticDigest) -> Self {
        Self { kind, id, digest }
    }

    /// Referenced kind.
    #[must_use]
    pub const fn kind(&self) -> ObjectKind {
        self.kind
    }

    /// Referenced stable ID.
    #[must_use]
    pub const fn id(&self) -> &StableId {
        &self.id
    }

    /// Expected object digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

/// Exact atom ID and semantic revision binding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AtomRevisionRef {
    id: StableId,
    revision: SemanticDigest,
}

impl AtomRevisionRef {
    /// Constructs an exact atom revision reference from an already-validated ID.
    #[must_use]
    pub fn new(id: StableId, revision: SemanticDigest) -> Self {
        Self { id, revision }
    }

    /// Referenced atom ID.
    #[must_use]
    pub const fn id(&self) -> &StableId {
        &self.id
    }

    /// Expected atom revision.
    #[must_use]
    pub const fn revision(&self) -> SemanticDigest {
        self.revision
    }
}

/// Repository-owned review approval binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalBinding {
    authority: ScopeId,
    record: RepositoryPath,
    digest: SemanticDigest,
}

impl ApprovalBinding {
    /// Constructs a review-only approval binding.
    ///
    /// # Errors
    ///
    /// Rejects an authority ID outside the frozen lowercase scope-ID grammar
    /// or non-NFC text.
    pub fn new(
        authority: ScopeId,
        record: RepositoryPath,
        digest: SemanticDigest,
    ) -> Result<Self, ConstructionError> {
        Ok(Self {
            authority,
            record,
            digest,
        })
    }

    /// Configured authority ID.
    #[must_use]
    pub fn authority(&self) -> &str {
        self.authority.as_str()
    }

    /// Repository-owned review record path.
    #[must_use]
    pub const fn record(&self) -> &RepositoryPath {
        &self.record
    }

    /// Review record digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

/// Pinned host-schema payload bytes.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TypedPayload {
    schema: SchemaId,
    bytes: Vec<u8>,
}

impl TypedPayload {
    /// Constructs an opaque payload under an immutable schema ID.
    ///
    /// # Errors
    ///
    /// Rejects an empty, non-NFC, or noncanonical lowercase schema ID.
    pub fn new(schema: SchemaId, bytes: Vec<u8>) -> Result<Self, ConstructionError> {
        Ok(Self { schema, bytes })
    }

    /// Host schema ID.
    #[must_use]
    pub fn schema(&self) -> &str {
        self.schema.as_str()
    }

    /// Canonical payload bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Text(self.schema.as_str().to_string())),
            (2, CanonicalValue::Bytes(self.bytes.clone())),
        ])
    }
}

/// Closed portable selector retained by policy and authority values.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TypedSelector {
    root: RepositoryPath,
    adapter_id: SchemaId,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    authority_expectation: AuthorityExpectation,
}

impl TypedSelector {
    /// Constructs one selector with canonical duplicate-free pattern sets.
    ///
    /// # Errors
    ///
    /// Rejects an empty adapter ID, an empty include set, duplicate patterns,
    /// or noncanonical retained text.
    pub fn new(
        root: RepositoryPath,
        adapter_id: SchemaId,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
        authority_expectation: AuthorityExpectation,
    ) -> Result<Self, ConstructionError> {
        Self::new_inner(
            root,
            adapter_id,
            include_patterns,
            exclude_patterns,
            authority_expectation,
        )
    }

    fn new_inner(
        root: RepositoryPath,
        adapter_id: SchemaId,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
        authority_expectation: AuthorityExpectation,
    ) -> Result<Self, ConstructionError> {
        if include_patterns.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let include_patterns = canonicalize_nonempty_text_set(include_patterns)?;
        let exclude_patterns = canonicalize_nonempty_text_set(exclude_patterns)?;
        Ok(Self {
            root,
            adapter_id,
            include_patterns,
            exclude_patterns,
            authority_expectation,
        })
    }

    /// Canonical repository-relative selector root.
    #[must_use]
    pub const fn root(&self) -> &RepositoryPath {
        &self.root
    }

    /// Declared adapter/schema ID.
    #[must_use]
    pub fn adapter_id(&self) -> &str {
        self.adapter_id.as_str()
    }

    /// Nonempty canonical include-pattern set.
    #[must_use]
    pub fn include_patterns(&self) -> &[String] {
        &self.include_patterns
    }

    /// Canonical exclude-pattern set.
    #[must_use]
    pub fn exclude_patterns(&self) -> &[String] {
        &self.exclude_patterns
    }

    /// Closed authority expectation.
    #[must_use]
    pub const fn authority_expectation(&self) -> AuthorityExpectation {
        self.authority_expectation
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Text(self.root.as_str().to_string())),
            (
                2,
                CanonicalValue::Text(self.adapter_id.as_str().to_string())
            ),
            (
                3,
                CanonicalValue::Set(
                    self.include_patterns
                        .iter()
                        .cloned()
                        .map(CanonicalValue::Text)
                        .collect(),
                ),
            ),
            (
                4,
                CanonicalValue::Set(
                    self.exclude_patterns
                        .iter()
                        .cloned()
                        .map(CanonicalValue::Text)
                        .collect(),
                ),
            ),
            (
                5,
                CanonicalValue::Enum(self.authority_expectation as u16, None),
            ),
        ])
    }
}

/// Closed repository-owned approval authority declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalAuthority {
    authority_id: ScopeId,
    allowed_kind_actions: Vec<TypedPayload>,
    allowed_namespaces: Vec<ScopeId>,
    review_record_selector: TypedSelector,
    separation_of_duty: Option<TypedPayload>,
    declaration_approval: ApprovalBinding,
}

impl ApprovalAuthority {
    /// Constructs one complete approval-authority declaration.
    ///
    /// # Errors
    ///
    /// Rejects invalid IDs, empty required sets, duplicates, or noncanonical text.
    pub fn new(
        authority_id: ScopeId,
        allowed_kind_actions: Vec<TypedPayload>,
        allowed_namespaces: Vec<ScopeId>,
        review_record_selector: TypedSelector,
        separation_of_duty: Option<TypedPayload>,
        declaration_approval: ApprovalBinding,
    ) -> Result<Self, ConstructionError> {
        if (
            allowed_kind_actions.is_empty(),
            allowed_namespaces.is_empty(),
        ) != (false, false)
        {
            return Err(ConstructionError::EmptySet);
        }
        let allowed_kind_actions =
            canonicalize_set(allowed_kind_actions, TypedPayload::canonical_value)?;
        let allowed_namespaces = canonicalize_set(allowed_namespaces, |namespace| {
            CanonicalValue::Text(namespace.as_str().to_string())
        })?;
        Ok(Self {
            authority_id,
            allowed_kind_actions,
            allowed_namespaces,
            review_record_selector,
            separation_of_duty,
            declaration_approval,
        })
    }

    /// Stable authority ID.
    #[must_use]
    pub fn authority_id(&self) -> &str {
        self.authority_id.as_str()
    }

    /// Closed kind/action payloads.
    #[must_use]
    pub fn allowed_kind_actions(&self) -> &[TypedPayload] {
        &self.allowed_kind_actions
    }

    /// Authorized stable-ID namespaces.
    #[must_use]
    pub fn allowed_namespaces(&self) -> &[ScopeId] {
        &self.allowed_namespaces
    }

    /// Selector for repository-owned approval records.
    #[must_use]
    pub const fn review_record_selector(&self) -> &TypedSelector {
        &self.review_record_selector
    }

    /// Optional separation-of-duty policy.
    #[must_use]
    pub const fn separation_of_duty(&self) -> Option<&TypedPayload> {
        self.separation_of_duty.as_ref()
    }

    /// Approval of this authority declaration.
    #[must_use]
    pub const fn declaration_approval(&self) -> &ApprovalBinding {
        &self.declaration_approval
    }

    /// Complete frozen-v2 canonical value.
    #[must_use]
    pub fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (
                1,
                CanonicalValue::Text(self.authority_id.as_str().to_string())
            ),
            (
                2,
                CanonicalValue::Set(
                    self.allowed_kind_actions
                        .iter()
                        .map(TypedPayload::canonical_value)
                        .collect(),
                ),
            ),
            (
                3,
                CanonicalValue::Set(
                    self.allowed_namespaces
                        .iter()
                        .map(|value| CanonicalValue::Text(value.as_str().to_string()))
                        .collect(),
                ),
            ),
            (4, self.review_record_selector.canonical_value()),
            (
                5,
                self.separation_of_duty
                    .as_ref()
                    .map_or(CanonicalValue::None, |value| {
                        CanonicalValue::Some(Box::new(value.canonical_value()))
                    }),
            ),
            (6, approval_binding_value(&self.declaration_approval)),
        ])
    }
}

/// Pure frozen repository-configuration semantic value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticRepositoryConfig {
    digest: SemanticDigest,
    configuration_schema_id: SchemaId,
    selectors: Vec<TypedSelector>,
    id_namespaces: Vec<IdNamespace>,
    approval_authorities: Vec<ApprovalAuthority>,
    policy_references: Vec<ObjectRef>,
    semantic_tools: Vec<ToolIdentity>,
}

impl SemanticRepositoryConfig {
    /// Constructs one complete repository semantic configuration.
    ///
    /// # Errors
    ///
    /// Rejects an invalid schema, an empty selector set, duplicates, or
    /// non-policy references.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        configuration_schema_id: SchemaId,
        selectors: Vec<TypedSelector>,
        id_namespaces: Vec<IdNamespace>,
        approval_authorities: Vec<ApprovalAuthority>,
        policy_references: Vec<ObjectRef>,
        semantic_tools: Vec<ToolIdentity>,
    ) -> Result<Self, ConstructionError> {
        if selectors.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let selectors = canonicalize_set(selectors, TypedSelector::canonical_value)?;
        let id_namespaces = canonicalize_set(id_namespaces, IdNamespace::canonical_value)?;
        let approval_authorities =
            canonicalize_set(approval_authorities, ApprovalAuthority::canonical_value)?;
        let policy_references = canonicalize_set(policy_references, object_reference_value)?;
        if policy_references
            .iter()
            .any(|reference| reference.kind() != ObjectKind::CoveragePolicy)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        let semantic_tools = canonicalize_set(semantic_tools, ToolIdentity::canonical_value)?;
        let fields = alloc::vec![
            (
                1,
                CanonicalValue::Text(configuration_schema_id.as_str().to_string())
            ),
            (
                2,
                CanonicalValue::Set(
                    selectors
                        .iter()
                        .map(TypedSelector::canonical_value)
                        .collect(),
                ),
            ),
            (
                3,
                CanonicalValue::Set(
                    id_namespaces
                        .iter()
                        .map(IdNamespace::canonical_value)
                        .collect(),
                ),
            ),
            (
                4,
                CanonicalValue::Set(
                    approval_authorities
                        .iter()
                        .map(ApprovalAuthority::canonical_value)
                        .collect(),
                ),
            ),
            (
                5,
                CanonicalValue::Set(
                    policy_references
                        .iter()
                        .map(object_reference_value)
                        .collect(),
                ),
            ),
            (
                6,
                CanonicalValue::Set(
                    semantic_tools
                        .iter()
                        .map(ToolIdentity::canonical_value)
                        .collect(),
                ),
            ),
        ];
        let digest = semantic_digest_trusted("repository-config", fields);
        Ok(Self {
            digest,
            configuration_schema_id,
            selectors,
            id_namespaces,
            approval_authorities,
            policy_references,
            semantic_tools,
        })
    }

    /// Complete repository-config digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Frozen configuration schema ID.
    #[must_use]
    pub fn configuration_schema_id(&self) -> &str {
        self.configuration_schema_id.as_str()
    }

    /// Canonical nonempty selector set.
    #[must_use]
    pub fn selectors(&self) -> &[TypedSelector] {
        &self.selectors
    }

    /// Canonical namespace declarations.
    #[must_use]
    pub fn id_namespaces(&self) -> &[IdNamespace] {
        &self.id_namespaces
    }

    /// Canonical approval-authority declarations.
    #[must_use]
    pub fn approval_authorities(&self) -> &[ApprovalAuthority] {
        &self.approval_authorities
    }

    /// Referenced coverage policies.
    #[must_use]
    pub fn policy_references(&self) -> &[ObjectRef] {
        &self.policy_references
    }

    /// Additional semantic tool identities.
    #[must_use]
    pub fn semantic_tools(&self) -> &[ToolIdentity] {
        &self.semantic_tools
    }
}

/// Deterministic observation of one rejected repository entry.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RejectedEntryObservation {
    path: RepositoryPath,
    kind: EntryKind,
    diagnostic_code: String,
}

impl RejectedEntryObservation {
    /// Constructs one canonical rejected-entry observation.
    ///
    /// # Errors
    ///
    /// Rejects an empty or non-NFC stable diagnostic code.
    pub fn new(
        path: RepositoryPath,
        kind: EntryKind,
        diagnostic_code: impl Into<String>,
    ) -> Result<Self, ConstructionError> {
        let diagnostic_code = diagnostic_code.into();
        require_nonempty_nfc(&diagnostic_code)?;
        Ok(Self {
            path,
            kind,
            diagnostic_code,
        })
    }

    /// Canonical repository-relative path.
    #[must_use]
    pub const fn path(&self) -> &RepositoryPath {
        &self.path
    }

    /// Closed observed entry kind.
    #[must_use]
    pub const fn kind(&self) -> EntryKind {
        self.kind
    }

    /// Stable normalized diagnostic code.
    #[must_use]
    pub fn diagnostic_code(&self) -> &str {
        &self.diagnostic_code
    }

    /// Complete frozen-v2 canonical value.
    #[must_use]
    pub fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Text(self.path.as_str().to_string())),
            (2, CanonicalValue::Enum(self.kind as u16, None)),
            (3, CanonicalValue::Text(self.diagnostic_code.clone())),
        ])
    }
}

/// One exact environment allowlist/value binding after inherited clearing.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EnvironmentBinding {
    name: String,
    value: String,
}

impl EnvironmentBinding {
    /// Constructs one canonical environment binding.
    ///
    /// # Errors
    ///
    /// Rejects names outside the frozen uppercase environment grammar or
    /// non-NFC values.
    pub fn new(
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, ConstructionError> {
        let name = name.into();
        let value = value.into();
        let valid_name = !name.is_empty()
            && name.as_bytes()[0].is_ascii_uppercase()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
        if !valid_name {
            return Err(ConstructionError::LocalInvariant);
        }
        require_nfc(&value)?;
        Ok(Self { name, value })
    }

    /// Environment variable name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Exact configured UTF-8 value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Complete frozen-v2 canonical value.
    #[must_use]
    pub fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Text(self.name.clone())),
            (2, CanonicalValue::Text(self.value.clone())),
        ])
    }
}

/// Exact semantic tool identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ToolIdentity {
    id: ScopeId,
    version: String,
    executable: SemanticDigest,
    configuration: SemanticDigest,
}

impl ToolIdentity {
    /// Constructs a pinned tool identity.
    ///
    /// # Errors
    ///
    /// Rejects a tool ID outside the lowercase scope-ID grammar or an empty or
    /// non-NFC version.
    pub fn new(
        id: ScopeId,
        version: impl Into<String>,
        executable: SemanticDigest,
        configuration: SemanticDigest,
    ) -> Result<Self, ConstructionError> {
        let version = version.into();
        require_nonempty_nfc(&version)?;
        Ok(Self {
            id,
            version,
            executable,
            configuration,
        })
    }

    /// Stable tool ID.
    #[must_use]
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Exact tool version or revision.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Exact executable artifact digest.
    #[must_use]
    pub const fn executable(&self) -> SemanticDigest {
        self.executable
    }

    /// Exact tool-configuration digest.
    #[must_use]
    pub const fn configuration(&self) -> SemanticDigest {
        self.configuration
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Text(self.id.as_str().to_string())),
            (2, CanonicalValue::Text(self.version.clone())),
            (3, CanonicalValue::Digest(self.executable)),
            (4, CanonicalValue::Digest(self.configuration)),
        ])
    }
}

/// Exact semantic schema/tool lock identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaToolLock {
    digest: SemanticDigest,
    encoding_schema_id: SchemaId,
    unicode_data: ToolIdentity,
    commonmark_parser: ToolIdentity,
    toml_parser: ToolIdentity,
    rust_parser: Option<ToolIdentity>,
    additional_tools: Vec<ToolIdentity>,
}

impl SchemaToolLock {
    /// Constructs one complete schema/tool lock and computes its identity.
    ///
    /// # Errors
    ///
    /// Rejects an invalid schema ID, duplicate tools, or noncanonical values.
    pub fn new(
        encoding_schema_id: SchemaId,
        unicode_data: ToolIdentity,
        commonmark_parser: ToolIdentity,
        toml_parser: ToolIdentity,
        rust_parser: Option<ToolIdentity>,
        additional_tools: Vec<ToolIdentity>,
    ) -> Result<Self, ConstructionError> {
        let additional_tools = canonicalize_set(additional_tools, ToolIdentity::canonical_value)?;
        let fields = alloc::vec![
            (
                1,
                CanonicalValue::Text(encoding_schema_id.as_str().to_string())
            ),
            (2, unicode_data.canonical_value()),
            (3, commonmark_parser.canonical_value()),
            (4, toml_parser.canonical_value()),
            (
                5,
                rust_parser.as_ref().map_or(CanonicalValue::None, |tool| {
                    CanonicalValue::Some(Box::new(tool.canonical_value()))
                }),
            ),
            (
                6,
                CanonicalValue::Set(
                    additional_tools
                        .iter()
                        .map(ToolIdentity::canonical_value)
                        .collect(),
                ),
            ),
        ];
        let digest = semantic_digest_trusted("schema-tool-lock", fields);
        Ok(Self {
            digest,
            encoding_schema_id,
            unicode_data,
            commonmark_parser,
            toml_parser,
            rust_parser,
            additional_tools,
        })
    }

    /// Complete schema-tool-lock digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Canonical encoding schema ID.
    #[must_use]
    pub fn encoding_schema_id(&self) -> &str {
        self.encoding_schema_id.as_str()
    }

    /// Pinned Unicode-data identity.
    #[must_use]
    pub const fn unicode_data(&self) -> &ToolIdentity {
        &self.unicode_data
    }

    /// Pinned `CommonMark` parser identity.
    #[must_use]
    pub const fn commonmark_parser(&self) -> &ToolIdentity {
        &self.commonmark_parser
    }

    /// Pinned TOML parser identity.
    #[must_use]
    pub const fn toml_parser(&self) -> &ToolIdentity {
        &self.toml_parser
    }

    /// Optional pinned Rust parser identity.
    #[must_use]
    pub const fn rust_parser(&self) -> Option<&ToolIdentity> {
        self.rust_parser.as_ref()
    }

    /// Additional semantic tools in canonical order.
    #[must_use]
    pub fn additional_tools(&self) -> &[ToolIdentity] {
        &self.additional_tools
    }
}

/// One live normative atom.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Atom {
    meta: ObjectMeta,
    revision: SemanticDigest,
    state: LifecycleState,
    kind: AtomKind,
    scopes: Vec<ScopeId>,
    statement: String,
    approval: Option<ApprovalBinding>,
}

impl Atom {
    /// Constructs an atom and computes its schema-v2 revision and object digest.
    ///
    /// # Errors
    ///
    /// Rejects invalid scope IDs, empty or nonnormative statements,
    /// duplicate scopes, non-NFC text, terminal states, and invalid approval shape.
    pub fn new(
        id: StableId,
        state: LifecycleState,
        kind: AtomKind,
        scopes: Vec<ScopeId>,
        statement: impl Into<String>,
        approval: Option<ApprovalBinding>,
    ) -> Result<Self, ConstructionError> {
        Self::build(None, id, state, kind, scopes, statement.into(), approval)
    }

    /// Constructs an atom under one explicit active digest member.
    ///
    /// # Errors
    ///
    /// Returns every [`Atom::new`] construction failure.
    pub fn new_with(
        active: ActiveAlgorithm,
        id: StableId,
        state: LifecycleState,
        kind: AtomKind,
        scopes: Vec<ScopeId>,
        statement: impl Into<String>,
        approval: Option<ApprovalBinding>,
    ) -> Result<Self, ConstructionError> {
        Self::build(
            Some(active),
            id,
            state,
            kind,
            scopes,
            statement.into(),
            approval,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        active: Option<ActiveAlgorithm>,
        id: StableId,
        state: LifecycleState,
        kind: AtomKind,
        scopes: Vec<ScopeId>,
        statement: String,
        approval: Option<ApprovalBinding>,
    ) -> Result<Self, ConstructionError> {
        require_nonempty_nfc(&statement)?;
        if !contains_rfc_2119_term(&statement) {
            return Err(ConstructionError::NormativeStatement);
        }
        if scopes.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let scopes = canonicalize_set(scopes, |scope| {
            CanonicalValue::Text(scope.as_str().to_string())
        })?;
        let valid_shape = matches!(
            (state, approval.is_some()),
            (LifecycleState::Proposed, false) | (LifecycleState::Ratified, true)
        );
        if !valid_shape {
            return Err(ConstructionError::LifecycleShape);
        }
        let revision_fields = alloc::vec![
            (1, CanonicalValue::Enum(kind as u16, None)),
            (
                2,
                CanonicalValue::Set(
                    scopes
                        .iter()
                        .map(|scope| CanonicalValue::Text(scope.as_str().to_string()))
                        .collect(),
                ),
            ),
            (3, CanonicalValue::Text(statement.clone())),
        ];
        let revision = match active {
            Some(member) => semantic_digest_trusted_with(member, "atom-revision", revision_fields),
            None => semantic_digest_trusted("atom-revision", revision_fields),
        };
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (2, CanonicalValue::Digest(revision)),
            (3, CanonicalValue::Enum(state as u16, None)),
            (4, approval_value(approval.as_ref())),
        ];
        let meta = match active {
            Some(member) => ObjectMeta::for_fields_with(
                member,
                ObjectKind::Atom,
                id,
                "atom-object",
                fields,
                Vec::new(),
            ),
            None => ObjectMeta::for_fields(ObjectKind::Atom, id, "atom-object", fields, Vec::new()),
        };
        Ok(Self {
            meta,
            revision,
            state,
            kind,
            scopes,
            statement,
            approval,
        })
    }

    /// Shared object metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Semantic atom revision.
    #[must_use]
    pub const fn revision(&self) -> SemanticDigest {
        self.revision
    }

    /// Current live lifecycle state.
    #[must_use]
    pub const fn state(&self) -> LifecycleState {
        self.state
    }

    /// Semantic atom category.
    #[must_use]
    pub const fn atom_kind(&self) -> AtomKind {
        self.kind
    }

    /// Canonical scope IDs.
    #[must_use]
    pub fn scopes(&self) -> &[ScopeId] {
        &self.scopes
    }

    /// Canonical normative statement.
    #[must_use]
    pub fn statement(&self) -> &str {
        &self.statement
    }

    /// Optional ratification approval.
    #[must_use]
    pub const fn approval(&self) -> Option<&ApprovalBinding> {
        self.approval.as_ref()
    }
}

fn approval_binding_value(binding: &ApprovalBinding) -> CanonicalValue {
    CanonicalValue::Record(alloc::vec![
        (
            1,
            CanonicalValue::Text(binding.authority.as_str().to_string())
        ),
        (2, CanonicalValue::Text(binding.record.as_str().to_string())),
        (3, CanonicalValue::Digest(binding.digest)),
    ])
}

fn approval_value(approval: Option<&ApprovalBinding>) -> CanonicalValue {
    approval.map_or(CanonicalValue::None, |binding| {
        CanonicalValue::Some(Box::new(approval_binding_value(binding)))
    })
}

/// Permanent terminal atom identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tombstone {
    meta: ObjectMeta,
    state: LifecycleState,
    final_revision: SemanticDigest,
    replacements: Vec<AtomRevisionRef>,
    approval: ApprovalBinding,
}

impl Tombstone {
    /// Constructs a terminal atom record and computes its complete object digest.
    ///
    /// # Errors
    ///
    /// Rejects live states, missing supersession replacements,
    /// self-replacements, duplicates, or noncanonical text.
    pub fn new(
        id: StableId,
        state: LifecycleState,
        final_revision: SemanticDigest,
        replacements: Vec<AtomRevisionRef>,
        approval: ApprovalBinding,
    ) -> Result<Self, ConstructionError> {
        if !matches!(
            state,
            LifecycleState::Superseded | LifecycleState::Withdrawn
        ) || (state == LifecycleState::Superseded && replacements.is_empty())
        {
            return Err(ConstructionError::LifecycleShape);
        }
        if replacements
            .iter()
            .any(|replacement| *replacement.id() == id)
        {
            return Err(ConstructionError::LocalInvariant);
        }
        let replacements = canonicalize_set(replacements, atom_revision_reference_value)?;
        let id_value = text_value(id.as_str());
        let meta = ObjectMeta::for_fields(
            ObjectKind::Tombstone,
            id,
            "tombstone",
            alloc::vec![
                (1, id_value),
                (2, CanonicalValue::Enum(state as u16, None)),
                (3, CanonicalValue::Digest(final_revision)),
                (
                    4,
                    CanonicalValue::Set(
                        replacements
                            .iter()
                            .map(atom_revision_reference_value)
                            .collect(),
                    ),
                ),
                (5, approval_binding_value(&approval)),
            ],
            Vec::new(),
        );
        Ok(Self {
            meta,
            state,
            final_revision,
            replacements,
            approval,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Terminal state.
    #[must_use]
    pub const fn state(&self) -> LifecycleState {
        self.state
    }

    /// Final accepted atom revision.
    #[must_use]
    pub const fn final_revision(&self) -> SemanticDigest {
        self.final_revision
    }

    /// Replacement atom revisions in canonical set order.
    #[must_use]
    pub fn replacements(&self) -> &[AtomRevisionRef] {
        &self.replacements
    }

    /// Repository-owned terminal approval.
    #[must_use]
    pub const fn approval(&self) -> &ApprovalBinding {
        &self.approval
    }
}

/// One declared permanent stable-ID namespace.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IdNamespace {
    namespace_id: ScopeId,
    prefix: NamespacePrefix,
    allocation_policy: TypedPayload,
    approval_authorities: Vec<ScopeId>,
}

impl IdNamespace {
    /// Constructs a namespace declaration and rejects duplicate authority IDs.
    ///
    /// # Errors
    ///
    /// Rejects empty namespace IDs, empty prefixes, or duplicate approval authorities.
    pub fn new(
        namespace_id: ScopeId,
        prefix: NamespacePrefix,
        allocation_policy: TypedPayload,
        mut approval_authorities: Vec<ScopeId>,
    ) -> Result<Self, RegistryError> {
        approval_authorities = canonicalize_set(approval_authorities, |authority| {
            CanonicalValue::Text(authority.as_str().to_string())
        })
        .map_err(|_| RegistryError::Duplicate)?;
        Ok(Self {
            namespace_id,
            prefix,
            allocation_policy,
            approval_authorities,
        })
    }

    /// Stable lowercase namespace ID.
    #[must_use]
    pub fn namespace_id(&self) -> &ScopeId {
        &self.namespace_id
    }

    /// Uppercase stable-ID prefix.
    #[must_use]
    pub fn prefix(&self) -> &NamespacePrefix {
        &self.prefix
    }

    /// Pinned allocation-policy payload.
    #[must_use]
    pub const fn allocation_policy(&self) -> &TypedPayload {
        &self.allocation_policy
    }

    /// Deterministically ordered approval-authority IDs.
    #[must_use]
    pub fn approval_authorities(&self) -> &[ScopeId] {
        &self.approval_authorities
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (
                1,
                CanonicalValue::Text(self.namespace_id.as_str().to_string())
            ),
            (2, CanonicalValue::Text(self.prefix.as_str().to_string())),
            (3, typed_payload_value(&self.allocation_policy)),
            (
                4,
                CanonicalValue::Set(
                    self.approval_authorities
                        .iter()
                        .map(|value| CanonicalValue::Text(value.as_str().to_string()))
                        .collect(),
                ),
            ),
        ])
    }
}

/// One append-only stable-ID allocation and reservation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IdAllocation {
    namespace_id: ScopeId,
    stable_id: StableId,
    allocation_digest: SemanticDigest,
    ordinal: u64,
    class: IdAllocationClass,
}

impl IdAllocation {
    /// Constructs one allocation with a positive namespace-local ordinal.
    ///
    /// # Errors
    ///
    /// Rejects an empty namespace ID and ordinal zero.
    pub fn new(
        namespace_id: ScopeId,
        stable_id: StableId,
        ordinal: u64,
        class: IdAllocationClass,
        allocation_digest: SemanticDigest,
    ) -> Result<Self, RegistryError> {
        if ordinal == 0 {
            return Err(RegistryError::Ordinal);
        }
        Ok(Self {
            namespace_id,
            stable_id,
            allocation_digest,
            ordinal,
            class,
        })
    }

    /// Declared namespace ID.
    #[must_use]
    pub fn namespace_id(&self) -> &ScopeId {
        &self.namespace_id
    }

    /// Permanently reserved stable ID.
    #[must_use]
    pub const fn stable_id(&self) -> &StableId {
        &self.stable_id
    }

    /// Positive allocation ordinal within the namespace.
    #[must_use]
    pub const fn ordinal(&self) -> u64 {
        self.ordinal
    }

    /// Current live or terminal reservation class.
    #[must_use]
    pub const fn class(&self) -> IdAllocationClass {
        self.class
    }

    /// Repository-owned allocation-record digest.
    #[must_use]
    pub const fn allocation_digest(&self) -> SemanticDigest {
        self.allocation_digest
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (
                1,
                CanonicalValue::Text(self.namespace_id.as_str().to_string())
            ),
            (2, text_value(self.stable_id.as_str())),
            (3, CanonicalValue::Digest(self.allocation_digest)),
            (4, CanonicalValue::U64(self.ordinal)),
            (5, CanonicalValue::Enum(self.class as u16, None)),
        ])
    }
}

/// Complete append-only stable-ID allocation authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IdRegistry {
    namespaces: Vec<IdNamespace>,
    allocations: Vec<IdAllocation>,
}

impl IdRegistry {
    /// Constructs and canonically orders a complete registry snapshot.
    ///
    /// # Errors
    ///
    /// Rejects duplicate namespace IDs or prefixes, duplicate stable IDs or
    /// ordinals, unknown allocation namespaces, and IDs outside their declared prefix.
    pub fn new(
        mut namespaces: Vec<IdNamespace>,
        mut allocations: Vec<IdAllocation>,
    ) -> Result<Self, RegistryError> {
        let mut namespace_ids = BTreeMap::<&ScopeId, &IdNamespace>::new();
        let mut prefixes = BTreeSet::<&NamespacePrefix>::new();
        for namespace in &namespaces {
            if namespace_ids
                .insert(namespace.namespace_id(), namespace)
                .is_some()
            {
                return Err(RegistryError::Duplicate);
            }
            if !prefixes.insert(namespace.prefix()) {
                return Err(RegistryError::Duplicate);
            }
        }

        let mut stable_ids = BTreeSet::<&StableId>::new();
        let mut ordinals = BTreeSet::<(&ScopeId, u64)>::new();
        for allocation in &allocations {
            let Some(namespace) = namespace_ids.get(allocation.namespace_id()) else {
                return Err(RegistryError::Namespace);
            };
            if !stable_id_has_prefix(allocation.stable_id(), namespace.prefix()) {
                return Err(RegistryError::Prefix);
            }
            if !stable_ids.insert(allocation.stable_id()) {
                return Err(RegistryError::Duplicate);
            }
            if !ordinals.insert((allocation.namespace_id(), allocation.ordinal())) {
                return Err(RegistryError::Duplicate);
            }
        }

        // The complete field-level uniqueness checks above prove both sets unique.
        namespaces = canonicalize_validated_set(namespaces, IdNamespace::canonical_value);
        allocations = canonicalize_validated_set(allocations, IdAllocation::canonical_value);
        Ok(Self {
            namespaces,
            allocations,
        })
    }

    /// Deterministically ordered namespace declarations.
    #[must_use]
    pub fn namespaces(&self) -> &[IdNamespace] {
        &self.namespaces
    }

    /// Allocation-history order grouped by namespace and positive ordinal.
    #[must_use]
    pub fn allocations(&self) -> &[IdAllocation] {
        &self.allocations
    }

    /// Returns the current reservation class for one permanently allocated ID.
    #[must_use]
    pub fn reservation(&self, stable_id: &StableId) -> Option<IdAllocationClass> {
        self.allocations
            .iter()
            .find(|allocation| allocation.stable_id() == stable_id)
            .map(IdAllocation::class)
    }

    /// Returns a new registry with one explicit appended allocation.
    ///
    /// # Errors
    ///
    /// Applies all complete-registry namespace, prefix, and uniqueness checks.
    pub fn with_allocation(&self, allocation: IdAllocation) -> Result<Self, RegistryError> {
        let mut allocations = self.allocations.clone();
        allocations.push(allocation);
        Self::new(self.namespaces.clone(), allocations)
    }

    /// Validates that `successor` preserves all prior namespace and allocation history.
    ///
    /// A live reservation may become terminal. Terminal reservations can never
    /// become live, and no prior namespace or allocation binding may disappear or change.
    ///
    /// # Errors
    ///
    /// Returns a stable registry-history or terminal-ID reuse error.
    pub fn validate_successor(&self, successor: &Self) -> Result<(), RegistryError> {
        for prior in &self.namespaces {
            let current = successor
                .namespaces
                .iter()
                .find(|namespace| namespace.namespace_id() == prior.namespace_id());
            if current != Some(prior) {
                return Err(RegistryError::History);
            }
        }
        for prior in &self.allocations {
            let Some(current) = successor
                .allocations
                .iter()
                .find(|allocation| allocation.stable_id() == prior.stable_id())
            else {
                return Err(RegistryError::History);
            };
            if prior.class() == IdAllocationClass::Terminal
                && current.class() == IdAllocationClass::Live
            {
                return Err(RegistryError::Reuse);
            }
            if (
                current.namespace_id(),
                current.ordinal(),
                current.allocation_digest(),
            ) != (
                prior.namespace_id(),
                prior.ordinal(),
                prior.allocation_digest(),
            ) {
                return Err(RegistryError::History);
            }
        }
        Ok(())
    }

    /// Computes the complete semantic `id-registry` digest.
    ///
    /// # Errors
    ///
    /// Returns an encoding error if retained text or set values are noncanonical.
    pub fn digest(&self) -> Result<SemanticDigest, EncodingError> {
        Ok(semantic_digest_trusted(
            "id-registry",
            alloc::vec![
                (
                    1,
                    CanonicalValue::Set(
                        self.namespaces
                            .iter()
                            .map(IdNamespace::canonical_value)
                            .collect(),
                    ),
                ),
                (
                    2,
                    CanonicalValue::Set(
                        self.allocations
                            .iter()
                            .map(IdAllocation::canonical_value)
                            .collect(),
                    ),
                ),
            ],
        ))
    }
}

fn stable_id_has_prefix(stable_id: &StableId, prefix: &NamespacePrefix) -> bool {
    stable_id
        .as_str()
        .strip_prefix(prefix.as_str())
        .is_some_and(|suffix| suffix.starts_with('-'))
}

fn typed_payload_value(payload: &TypedPayload) -> CanonicalValue {
    payload.canonical_value()
}

pub(crate) fn object_reference_value(reference: &ObjectRef) -> CanonicalValue {
    CanonicalValue::Record(alloc::vec![
        (1, CanonicalValue::Enum(reference.kind() as u16, None)),
        (2, text_value(reference.id().as_str())),
        (3, CanonicalValue::Digest(reference.digest())),
    ])
}

fn atom_revision_reference_value(reference: &AtomRevisionRef) -> CanonicalValue {
    CanonicalValue::Record(alloc::vec![
        (1, text_value(reference.id().as_str())),
        (2, CanonicalValue::Digest(reference.revision())),
    ])
}

fn digest_value(digest: &SemanticDigest) -> CanonicalValue {
    CanonicalValue::Digest(*digest)
}

fn digest_option_value(digest: Option<SemanticDigest>) -> CanonicalValue {
    digest.map_or(CanonicalValue::None, |digest| {
        CanonicalValue::Some(Box::new(CanonicalValue::Digest(digest)))
    })
}

fn text_value(text: &str) -> CanonicalValue {
    CanonicalValue::Text(text.to_string())
}

fn typed_selector_value(selector: &TypedSelector) -> CanonicalValue {
    selector.canonical_value()
}

fn canonicalize_nonempty_text_set(values: Vec<String>) -> Result<Vec<String>, ConstructionError> {
    if values.iter().any(String::is_empty) {
        return Err(ConstructionError::EmptyText);
    }
    canonicalize_set(values, |text| text_value(text)).map_err(ConstructionError::from)
}

/// Deterministic ID-registry construction or history failure.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RegistryError {
    /// An allocation names no declared namespace.
    Namespace,
    /// A namespace prefix is empty or does not contain an allocated stable ID.
    Prefix,
    /// A namespace, prefix, stable ID, authority, or ordinal occurs more than once.
    Duplicate,
    /// An allocation ordinal is not positive.
    Ordinal,
    /// A prior namespace or allocation binding was removed or changed.
    History,
    /// A terminal reservation became live again.
    Reuse,
}

impl RegistryError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Namespace => "SCF-REGISTRY-NAMESPACE",
            Self::Prefix => "SCF-REGISTRY-PREFIX",
            Self::Duplicate => "SCF-REGISTRY-DUPLICATE",
            Self::Ordinal => "SCF-REGISTRY-ORDINAL",
            Self::History => "SCF-REGISTRY-HISTORY",
            Self::Reuse => "SCF-REGISTRY-REUSE",
        }
    }
}

/// One immutable generation of a complete finite decision-surface domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionSurfaceDomain {
    surface_id: StableId,
    member_key_schema: TypedPayload,
    disposition_vocabulary: TypedPayload,
    member_keys: Vec<TypedPayload>,
    generation: u64,
    digest: SemanticDigest,
}

impl DecisionSurfaceDomain {
    /// Constructs a nonempty positive-generation domain and computes its digest.
    ///
    /// # Errors
    ///
    /// Rejects generation zero, an empty member set, duplicate members, or
    /// noncanonical schema text.
    pub fn new(
        surface_id: StableId,
        member_key_schema: TypedPayload,
        disposition_vocabulary: TypedPayload,
        member_keys: Vec<TypedPayload>,
        generation: u64,
    ) -> Result<Self, ConstructionError> {
        if generation == 0 {
            return Err(ConstructionError::NonPositiveGeneration);
        }
        if member_keys.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let member_keys = canonicalize_set(member_keys, TypedPayload::canonical_value)?;
        let digest = semantic_digest_trusted(
            "decision-surface-domain",
            alloc::vec![
                (1, text_value(surface_id.as_str())),
                (2, member_key_schema.canonical_value()),
                (3, disposition_vocabulary.canonical_value()),
                (
                    4,
                    CanonicalValue::Set(
                        member_keys
                            .iter()
                            .map(TypedPayload::canonical_value)
                            .collect(),
                    ),
                ),
                (5, CanonicalValue::U64(generation)),
            ],
        );
        Ok(Self {
            surface_id,
            member_key_schema,
            disposition_vocabulary,
            member_keys,
            generation,
            digest,
        })
    }

    /// Stable decision-surface ID shared by every generation.
    #[must_use]
    pub const fn surface_id(&self) -> &StableId {
        &self.surface_id
    }

    /// Pinned member-key construction schema.
    #[must_use]
    pub const fn member_key_schema(&self) -> &TypedPayload {
        &self.member_key_schema
    }

    /// Pinned host disposition vocabulary.
    #[must_use]
    pub const fn disposition_vocabulary(&self) -> &TypedPayload {
        &self.disposition_vocabulary
    }

    /// Complete member-key set in canonical encoding order.
    #[must_use]
    pub fn member_keys(&self) -> &[TypedPayload] {
        &self.member_keys
    }

    /// Positive domain evolution generation.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Complete `decision-surface-domain` digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

/// One declared member disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceDisposition {
    key: TypedPayload,
    disposition: SurfaceDispositionKind,
    payload: Option<TypedPayload>,
    authority: Option<AtomRevisionRef>,
}

impl SurfaceDisposition {
    /// Constructs one explicit surface row.
    ///
    /// # Errors
    ///
    /// Rejects a normative, prohibited, or deferred row without both its host
    /// payload and controlling atom authority.
    pub fn new(
        key: TypedPayload,
        disposition: SurfaceDispositionKind,
        payload: Option<TypedPayload>,
        authority: Option<AtomRevisionRef>,
    ) -> Result<Self, ConstructionError> {
        if disposition != SurfaceDispositionKind::NotApplicable
            && (payload.is_none() || authority.is_none())
        {
            return Err(ConstructionError::LocalInvariant);
        }
        Ok(Self {
            key,
            disposition,
            payload,
            authority,
        })
    }

    /// Canonical member key.
    #[must_use]
    pub const fn key(&self) -> &TypedPayload {
        &self.key
    }

    /// Core disposition.
    #[must_use]
    pub const fn disposition(&self) -> SurfaceDispositionKind {
        self.disposition
    }

    /// Optional host payload.
    #[must_use]
    pub const fn payload(&self) -> Option<&TypedPayload> {
        self.payload.as_ref()
    }

    /// Optional controlling atom revision.
    #[must_use]
    pub const fn authority(&self) -> Option<&AtomRevisionRef> {
        self.authority.as_ref()
    }

    /// Computes the complete `surface-disposition` digest.
    ///
    /// # Errors
    ///
    /// Returns an encoding error for noncanonical retained text.
    pub fn digest(&self) -> Result<SemanticDigest, EncodingError> {
        Ok(semantic_digest_trusted(
            "surface-disposition",
            surface_disposition_fields(
                &self.key,
                self.disposition,
                self.payload.as_ref(),
                self.authority.as_ref(),
            ),
        ))
    }

    fn canonical_value(&self) -> CanonicalValue {
        surface_disposition_value(
            &self.key,
            self.disposition,
            self.payload.as_ref(),
            self.authority.as_ref(),
        )
    }
}

fn surface_disposition_fields(
    key: &TypedPayload,
    disposition: SurfaceDispositionKind,
    payload: Option<&TypedPayload>,
    authority: Option<&AtomRevisionRef>,
) -> Vec<(u16, CanonicalValue)> {
    alloc::vec![
        (1, key.canonical_value()),
        (2, CanonicalValue::Enum(disposition as u16, None)),
        (
            3,
            payload.map_or(CanonicalValue::None, |value| {
                CanonicalValue::Some(Box::new(value.canonical_value()))
            }),
        ),
        (
            4,
            authority.map_or(CanonicalValue::None, |reference| {
                CanonicalValue::Some(Box::new(atom_revision_reference_value(reference)))
            }),
        ),
    ]
}

fn surface_disposition_value(
    key: &TypedPayload,
    disposition: SurfaceDispositionKind,
    payload: Option<&TypedPayload>,
    authority: Option<&AtomRevisionRef>,
) -> CanonicalValue {
    CanonicalValue::Record(surface_disposition_fields(
        key,
        disposition,
        payload,
        authority,
    ))
}

/// Complete governed decision surface bound to one exact finite domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionSurface {
    meta: ObjectMeta,
    domain: DecisionSurfaceDomain,
    dispositions: Vec<SurfaceDisposition>,
    policies: Vec<ObjectRef>,
}

impl DecisionSurface {
    /// Constructs one surface and computes its complete semantic object digest.
    ///
    /// Membership completeness and reference closure remain graph-level checks.
    ///
    /// # Errors
    ///
    /// Rejects duplicate rows, duplicate member keys, wrong-kind policies, or
    /// noncanonical text.
    pub fn new(
        domain: DecisionSurfaceDomain,
        dispositions: Vec<SurfaceDisposition>,
        policies: Vec<ObjectRef>,
    ) -> Result<Self, ConstructionError> {
        let mut member_keys = BTreeSet::new();
        for disposition in &dispositions {
            if !member_keys.insert(disposition.key()) {
                return Err(ConstructionError::LocalInvariant);
            }
        }
        // Unique keys make complete disposition values unique as well.
        let dispositions =
            canonicalize_validated_set(dispositions, SurfaceDisposition::canonical_value);
        let policies = canonicalize_set(policies, object_reference_value)?;
        if policies
            .iter()
            .any(|policy| policy.kind() != ObjectKind::CoveragePolicy)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        let meta = ObjectMeta::for_fields(
            ObjectKind::DecisionSurface,
            domain.surface_id().clone(),
            "decision-surface",
            alloc::vec![
                (1, text_value(domain.surface_id().as_str())),
                (2, CanonicalValue::Digest(domain.digest())),
                (
                    3,
                    CanonicalValue::Set(
                        dispositions
                            .iter()
                            .map(SurfaceDisposition::canonical_value)
                            .collect(),
                    ),
                ),
                (
                    4,
                    CanonicalValue::Set(policies.iter().map(object_reference_value).collect(),),
                ),
            ],
            policies.clone(),
        );
        Ok(Self {
            meta,
            domain,
            dispositions,
            policies,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Complete bound domain value.
    #[must_use]
    pub const fn domain(&self) -> &DecisionSurfaceDomain {
        &self.domain
    }

    /// Bound finite-domain digest.
    #[must_use]
    pub const fn domain_digest(&self) -> SemanticDigest {
        self.domain.digest()
    }

    /// Complete expected member-key set.
    #[must_use]
    pub fn expected_members(&self) -> &[TypedPayload] {
        self.domain.member_keys()
    }

    /// Declared member dispositions in canonical set order.
    #[must_use]
    pub fn dispositions(&self) -> &[SurfaceDisposition] {
        &self.dispositions
    }

    /// Applicable coverage-policy references in canonical set order.
    #[must_use]
    pub fn policies(&self) -> &[ObjectRef] {
        &self.policies
    }
}

/// Parser-canonical semantic identity for one supported source item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedItemIdentity {
    language_schema_id: SchemaId,
    path: RepositoryPath,
    kind: SourceItemKind,
    semantic_bytes: Vec<u8>,
    enclosing_item: Option<SemanticDigest>,
    digest: SemanticDigest,
}

impl ParsedItemIdentity {
    /// Constructs one parser-canonical identity and computes its exact digest.
    ///
    /// # Errors
    ///
    /// Rejects an invalid or noncanonical language/schema ID.
    pub fn new(
        language_schema_id: SchemaId,
        path: RepositoryPath,
        kind: SourceItemKind,
        semantic_bytes: Vec<u8>,
        enclosing_item: Option<SemanticDigest>,
    ) -> Result<Self, ConstructionError> {
        let digest = semantic_digest_trusted(
            "parsed-item-identity",
            parsed_item_identity_fields(
                &language_schema_id,
                &path,
                &kind,
                &semantic_bytes,
                enclosing_item,
            ),
        );
        Ok(Self {
            language_schema_id,
            path,
            kind,
            semantic_bytes,
            enclosing_item,
            digest,
        })
    }

    /// Constructs one parser-canonical identity under an explicit active member.
    ///
    /// A repository selects exactly one digest member, so every content-addressed
    /// reference that resolves inside one snapshot computes under that member
    /// rather than under the first member this build happens to contain.
    ///
    /// # Errors
    ///
    /// Rejects an invalid or noncanonical language/schema ID.
    pub fn new_with(
        active: ActiveAlgorithm,
        language_schema_id: SchemaId,
        path: RepositoryPath,
        kind: SourceItemKind,
        semantic_bytes: Vec<u8>,
        enclosing_item: Option<SemanticDigest>,
    ) -> Result<Self, ConstructionError> {
        let digest = semantic_digest_trusted_with(
            active,
            "parsed-item-identity",
            parsed_item_identity_fields(
                &language_schema_id,
                &path,
                &kind,
                &semantic_bytes,
                enclosing_item,
            ),
        );
        Ok(Self {
            language_schema_id,
            path,
            kind,
            semantic_bytes,
            enclosing_item,
            digest,
        })
    }

    /// Pinned language and parser schema ID.
    #[must_use]
    pub fn language_schema_id(&self) -> &str {
        self.language_schema_id.as_str()
    }

    /// Canonical repository-relative source path.
    #[must_use]
    pub const fn path(&self) -> &RepositoryPath {
        &self.path
    }

    /// Versioned parser item kind.
    #[must_use]
    pub const fn kind(&self) -> &SourceItemKind {
        &self.kind
    }

    /// Parser-canonical semantic token bytes.
    #[must_use]
    pub fn semantic_bytes(&self) -> &[u8] {
        &self.semantic_bytes
    }

    /// Optional enclosing parsed-item identity digest.
    #[must_use]
    pub const fn enclosing_item(&self) -> Option<SemanticDigest> {
        self.enclosing_item
    }

    /// Complete `parsed-item-identity` digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

fn parsed_item_identity_fields(
    language_schema_id: &SchemaId,
    path: &RepositoryPath,
    kind: &SourceItemKind,
    semantic_bytes: &[u8],
    enclosing_item: Option<SemanticDigest>,
) -> Vec<(u16, CanonicalValue)> {
    alloc::vec![
        (
            1,
            CanonicalValue::Text(language_schema_id.as_str().to_string())
        ),
        (2, CanonicalValue::Text(path.as_str().to_string())),
        (3, CanonicalValue::Text(kind.as_str().to_string())),
        (4, CanonicalValue::Bytes(semantic_bytes.to_vec())),
        (
            5,
            enclosing_item.map_or(CanonicalValue::None, |value| {
                CanonicalValue::Some(Box::new(CanonicalValue::Digest(value)))
            }),
        ),
    ]
}

/// Public object-surface name for a parser-canonical item identity.
pub type ParsedItem = ParsedItemIdentity;

/// Original-byte source extent under one canonical snapshot path.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourceRef {
    path: RepositoryPath,
    start: u64,
    end: u64,
}

impl SourceRef {
    /// Constructs an inclusive-start, exclusive-end original-byte extent.
    ///
    /// # Errors
    ///
    /// Rejects an end offset preceding the start offset.
    pub fn new(path: RepositoryPath, start: u64, end: u64) -> Result<Self, ConstructionError> {
        if start > end {
            return Err(ConstructionError::SourceSpan);
        }
        Ok(Self { path, start, end })
    }

    /// Canonical repository-relative source path.
    #[must_use]
    pub const fn path(&self) -> &RepositoryPath {
        &self.path
    }

    /// Inclusive original-byte start.
    #[must_use]
    pub const fn start(&self) -> u64 {
        self.start
    }

    /// Exclusive original-byte end.
    #[must_use]
    pub const fn end(&self) -> u64 {
        self.end
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Text(self.path.as_str().to_string())),
            (2, CanonicalValue::U64(self.start)),
            (3, CanonicalValue::U64(self.end)),
        ])
    }
}

/// One exact semantic-object to source mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMapEntry {
    object: ObjectRef,
    sources: Vec<SourceRef>,
    role: SourceRole,
    digest: SemanticDigest,
}

impl SourceMapEntry {
    /// Constructs one nonempty source-map entry and computes its digest.
    ///
    /// # Errors
    ///
    /// Rejects an empty/duplicate source set or noncanonical retained text.
    pub fn new(
        object: ObjectRef,
        sources: Vec<SourceRef>,
        role: SourceRole,
    ) -> Result<Self, ConstructionError> {
        if sources.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        if !source_role_kind_compatible(role, object.kind()) {
            return Err(ConstructionError::ReferenceKind);
        }
        let sources = canonicalize_set(sources, SourceRef::canonical_value)?;
        let digest = semantic_digest_trusted(
            "source-map-entry",
            alloc::vec![
                (1, object_reference_value(&object)),
                (
                    2,
                    CanonicalValue::Set(sources.iter().map(SourceRef::canonical_value).collect(),),
                ),
                (3, CanonicalValue::Enum(role as u16, None)),
            ],
        );
        Ok(Self {
            object,
            sources,
            role,
            digest,
        })
    }

    /// Exact mapped object reference.
    #[must_use]
    pub const fn object(&self) -> &ObjectRef {
        &self.object
    }

    /// Nonempty original-byte source-reference set.
    #[must_use]
    pub fn sources(&self) -> &[SourceRef] {
        &self.sources
    }

    /// Semantic source role.
    #[must_use]
    pub const fn role(&self) -> SourceRole {
        self.role
    }

    /// Complete `source-map-entry` digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, object_reference_value(&self.object)),
            (
                2,
                CanonicalValue::Set(
                    self.sources
                        .iter()
                        .map(SourceRef::canonical_value)
                        .collect(),
                ),
            ),
            (3, CanonicalValue::Enum(self.role as u16, None)),
        ])
    }
}

/// Complete source map, separate from semantic object revisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMap {
    entries: Vec<SourceMapEntry>,
    digest: SemanticDigest,
}

impl SourceMap {
    /// Constructs a canonically ordered source map and computes its digest.
    ///
    /// # Errors
    ///
    /// Rejects duplicate complete entries or noncanonical retained text.
    pub fn new(entries: Vec<SourceMapEntry>) -> Result<Self, ConstructionError> {
        let entries = canonicalize_set(entries, SourceMapEntry::canonical_value)?;
        let mut object_keys = BTreeSet::new();
        if entries
            .iter()
            .any(|entry| !object_keys.insert((entry.object().kind(), entry.object().id().clone())))
        {
            return Err(ConstructionError::LocalInvariant);
        }
        let digest = semantic_digest_trusted(
            "source-map",
            alloc::vec![(
                1,
                CanonicalValue::Set(
                    entries
                        .iter()
                        .map(SourceMapEntry::canonical_value)
                        .collect(),
                ),
            )],
        );
        Ok(Self { entries, digest })
    }

    /// Canonically ordered source-map entries.
    #[must_use]
    pub fn entries(&self) -> &[SourceMapEntry] {
        &self.entries
    }

    /// Complete `source-map` digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

/// Closed typed relation vocabulary for content-addressed source references.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[repr(u16)]
pub enum SourceEdgeKind {
    /// Ordered structural containment from one parent node to one child node.
    Contains = 1,
    /// Navigation from one implementation subject to one specification node.
    Implements = 2,
    /// Correspondence from one specification node to its governing authority.
    Authority = 3,
}

impl SourceEdgeKind {
    /// Parses one frozen relation name at the untrusted-input boundary.
    ///
    /// # Errors
    ///
    /// Rejects every name outside the frozen source-edge vocabulary.
    pub fn parse(input: &str) -> Result<Self, SourceGraphError> {
        match input {
            "contains" => Ok(Self::Contains),
            "implements" => Ok(Self::Implements),
            "authority" => Ok(Self::Authority),
            _ => Err(SourceGraphError::EdgeKindGrammar),
        }
    }

    /// Frozen canonical relation name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::Implements => "implements",
            Self::Authority => "authority",
        }
    }
}

/// A parsed reference to one exact content-addressed source node.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct SourceNodeRef(SemanticDigest);

impl SourceNodeRef {
    /// Parses one algorithm-qualified node digest at the untrusted-input boundary.
    ///
    /// # Errors
    ///
    /// Rejects text outside the canonical algorithm-qualified digest form.
    pub fn parse(input: &str) -> Result<Self, SourceGraphError> {
        SemanticDigest::parse_canonical(input)
            .map(Self)
            .map_err(SourceGraphError::NodeReferenceText)
    }

    /// Wraps one node digest already computed by this core.
    #[must_use]
    pub const fn new(digest: SemanticDigest) -> Self {
        Self(digest)
    }

    /// Referenced node digest.
    #[must_use]
    pub const fn digest(self) -> SemanticDigest {
        self.0
    }

    /// Canonical algorithm-qualified reference text.
    #[must_use]
    pub fn canonical_text(self) -> String {
        self.0.canonical_text()
    }

    fn canonical_value(self) -> CanonicalValue {
        CanonicalValue::Digest(self.0)
    }
}

/// One ordered typed child reference under one parent source node.
///
/// A node preimage binds containment only, so `SourceNode` construction rejects
/// every navigation relation here.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourceChildRef {
    relation: SourceEdgeKind,
    position: u32,
    target: SourceNodeRef,
}

impl SourceChildRef {
    /// Constructs one ordered typed child reference.
    #[must_use]
    pub const fn new(relation: SourceEdgeKind, position: u32, target: SourceNodeRef) -> Self {
        Self {
            relation,
            position,
            target,
        }
    }

    /// Typed child relation.
    #[must_use]
    pub const fn relation(&self) -> SourceEdgeKind {
        self.relation
    }

    /// Nonnegative child position under this parent and relation.
    #[must_use]
    pub const fn position(&self) -> u32 {
        self.position
    }

    /// Exact child node reference.
    #[must_use]
    pub const fn target(&self) -> SourceNodeRef {
        self.target
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Enum(self.relation as u16, None)),
            (2, CanonicalValue::U64(u64::from(self.position))),
            (3, self.target.canonical_value()),
        ])
    }

    /// Frozen canonical bytes for inventory vectors and independent consumers.
    ///
    /// # Errors
    ///
    /// Returns the canonical encoder error for an unsupported value.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EncodingError> {
        self.canonical_value().encode()
    }
}

/// Rejects positioned navigation and noncanonical positions in sorted children.
///
/// A position orders containment only. Every other relation is an unordered
/// declared edge, so one navigation relation never reaches a node preimage.
fn validate_children(children: &[SourceChildRef]) -> Result<(), SourceGraphError> {
    if children
        .iter()
        .any(|child| !matches!(child.relation, SourceEdgeKind::Contains))
    {
        return Err(SourceGraphError::PositionedNavigation);
    }
    if children
        .windows(2)
        .any(|pair| pair[0].position == pair[1].position)
    {
        return Err(SourceGraphError::DuplicateChildPosition);
    }
    let mut expected = 0_u32;
    for child in children {
        if child.position != expected {
            return Err(SourceGraphError::ChildPositionGap);
        }
        expected = expected.saturating_add(1);
    }
    Ok(())
}

fn source_node_fields(
    item: &ParsedItemIdentity,
    children: &[SourceChildRef],
) -> Vec<(u16, CanonicalValue)> {
    alloc::vec![
        (1, CanonicalValue::Digest(item.digest())),
        (
            2,
            CanonicalValue::Sequence(
                children
                    .iter()
                    .map(SourceChildRef::canonical_value)
                    .collect()
            ),
        ),
    ]
}

/// One content-addressed parsed source node and its ordered child references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceNode {
    item: ParsedItemIdentity,
    children: Vec<SourceChildRef>,
    digest: SemanticDigest,
}

impl SourceNode {
    /// Constructs one source node in canonical child order and computes its digest.
    ///
    /// The node digest binds the complete parsed item identity, which already
    /// binds the language schema ID, source item kind, canonical repository
    /// path, semantic payload, and enclosing item, together with every child
    /// relation, position, and child digest. Source offsets and formatting
    /// trivia never reach this preimage.
    ///
    /// # Errors
    ///
    /// Rejects one positioned navigation child and duplicate or noncontiguous
    /// child positions.
    pub fn new(
        item: ParsedItemIdentity,
        children: Vec<SourceChildRef>,
    ) -> Result<Self, SourceGraphError> {
        let mut children = children;
        children.sort_unstable();
        validate_children(&children)?;
        let digest = semantic_digest_trusted("source-node", source_node_fields(&item, &children));
        Ok(Self {
            item,
            children,
            digest,
        })
    }

    /// Constructs one source node under an explicit active member.
    ///
    /// Local containment stays Merkle-backed: the child digests this node binds
    /// must already address the same member, which the caller establishes by
    /// building every child under that member.
    ///
    /// # Errors
    ///
    /// Rejects one positioned navigation child, duplicate or noncontiguous
    /// child positions, and a child or item digest outside the active member.
    pub fn new_with(
        active: ActiveAlgorithm,
        item: ParsedItemIdentity,
        children: Vec<SourceChildRef>,
    ) -> Result<Self, SourceGraphError> {
        let mut children = children;
        children.sort_unstable();
        validate_children(&children)?;
        if item.digest().algorithm() != active.algorithm()
            || children
                .iter()
                .any(|child| child.target.digest().algorithm() != active.algorithm())
        {
            return Err(SourceGraphError::NodeDigestMember);
        }
        let digest = semantic_digest_trusted_with(
            active,
            "source-node",
            source_node_fields(&item, &children),
        );
        Ok(Self {
            item,
            children,
            digest,
        })
    }

    /// Parser-canonical item identity.
    #[must_use]
    pub const fn item(&self) -> &ParsedItemIdentity {
        &self.item
    }

    /// Child references in canonical relation and position order.
    #[must_use]
    pub fn children(&self) -> &[SourceChildRef] {
        &self.children
    }

    /// Complete `source-node` digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Reference to this exact node.
    #[must_use]
    pub const fn reference(&self) -> SourceNodeRef {
        SourceNodeRef::new(self.digest)
    }

    /// Creates an unequal node with this digest for collision-path controls.
    #[cfg(feature = "coverage-controls")]
    #[doc(hidden)]
    #[must_use]
    pub fn digest_collision_control(&self, item: ParsedItemIdentity) -> Self {
        Self {
            item,
            children: self.children.clone(),
            digest: self.digest,
        }
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Digest(self.item.digest())),
            (
                2,
                CanonicalValue::Sequence(
                    self.children
                        .iter()
                        .map(SourceChildRef::canonical_value)
                        .collect(),
                ),
            ),
        ])
    }

    /// Frozen canonical bytes for inventory vectors and independent consumers.
    ///
    /// # Errors
    ///
    /// Returns the canonical encoder error for an unsupported value.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EncodingError> {
        self.canonical_value().encode()
    }
}

/// One typed directed edge between exact source-node digests.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourceEdge {
    source: SourceNodeRef,
    relation: SourceEdgeKind,
    position: Option<u32>,
    target: SourceNodeRef,
}

impl SourceEdge {
    /// Constructs one typed directed source edge.
    ///
    /// A position orders containment only, so every navigation relation carries
    /// no position and one relation pair has exactly one canonical edge value.
    ///
    /// # Errors
    ///
    /// Rejects a `contains` edge that omits its nonnegative child position and
    /// every navigation edge that carries one.
    pub const fn new(
        source: SourceNodeRef,
        relation: SourceEdgeKind,
        position: Option<u32>,
        target: SourceNodeRef,
    ) -> Result<Self, SourceGraphError> {
        if matches!(relation, SourceEdgeKind::Contains) {
            if position.is_none() {
                return Err(SourceGraphError::MissingChildPosition);
            }
        } else if position.is_some() {
            return Err(SourceGraphError::PositionedNavigation);
        }
        Ok(Self {
            source,
            relation,
            position,
            target,
        })
    }

    /// Exact parent node reference.
    #[must_use]
    pub const fn source(&self) -> SourceNodeRef {
        self.source
    }

    /// Typed relation.
    #[must_use]
    pub const fn relation(&self) -> SourceEdgeKind {
        self.relation
    }

    /// Child position when the relation is ordered.
    #[must_use]
    pub const fn position(&self) -> Option<u32> {
        self.position
    }

    /// Exact target node reference.
    #[must_use]
    pub const fn target(&self) -> SourceNodeRef {
        self.target
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, self.source.canonical_value()),
            (2, CanonicalValue::Enum(self.relation as u16, None)),
            (
                3,
                self.position.map_or(CanonicalValue::None, |position| {
                    CanonicalValue::Some(Box::new(CanonicalValue::U64(u64::from(position))))
                }),
            ),
            (4, self.target.canonical_value()),
        ])
    }

    /// Frozen canonical bytes for inventory vectors and independent consumers.
    ///
    /// # Errors
    ///
    /// Returns the canonical encoder error for an unsupported value.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EncodingError> {
        self.canonical_value().encode()
    }
}

macro_rules! source_budget {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
        pub struct $name(u32);

        impl $name {
            /// Parses one positive configured limit.
            ///
            /// # Errors
            ///
            /// Rejects a zero limit, which would admit no bounded traversal.
            pub const fn new(value: u32) -> Result<Self, SourceGraphError> {
                if value == 0 {
                    return Err(SourceGraphError::NonPositiveBudget);
                }
                Ok(Self(value))
            }

            /// Configured positive limit.
            #[must_use]
            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

source_budget!(
    SourceNodeBudget,
    "A parsed positive limit on total source-graph nodes."
);
source_budget!(
    SourceEdgeBudget,
    "A parsed positive limit on total source-graph edges."
);
source_budget!(
    SourceDepthBudget,
    "A parsed positive limit on traversed source-graph depth in edges."
);
source_budget!(
    SourceVisitedBudget,
    "A parsed positive limit on visited nodes in one source-graph traversal."
);

/// Complete positive resource limits for one bounded source graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceBudgets {
    nodes: SourceNodeBudget,
    edges: SourceEdgeBudget,
    depth: SourceDepthBudget,
    visited: SourceVisitedBudget,
}

impl SourceBudgets {
    /// Collects four parsed positive limits.
    #[must_use]
    pub const fn new(
        nodes: SourceNodeBudget,
        edges: SourceEdgeBudget,
        depth: SourceDepthBudget,
        visited: SourceVisitedBudget,
    ) -> Self {
        Self {
            nodes,
            edges,
            depth,
            visited,
        }
    }

    /// Total node limit.
    #[must_use]
    pub const fn nodes(&self) -> SourceNodeBudget {
        self.nodes
    }

    /// Total edge limit.
    #[must_use]
    pub const fn edges(&self) -> SourceEdgeBudget {
        self.edges
    }

    /// Traversed depth limit.
    #[must_use]
    pub const fn depth(&self) -> SourceDepthBudget {
        self.depth
    }

    /// Visited node limit.
    #[must_use]
    pub const fn visited(&self) -> SourceVisitedBudget {
        self.visited
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::U64(u64::from(self.nodes.get()))),
            (2, CanonicalValue::U64(u64::from(self.edges.get()))),
            (3, CanonicalValue::U64(u64::from(self.depth.get()))),
            (4, CanonicalValue::U64(u64::from(self.visited.get()))),
        ])
    }

    /// Frozen canonical bytes for inventory vectors and independent consumers.
    ///
    /// # Errors
    ///
    /// Returns the canonical encoder error for an unsupported value.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EncodingError> {
        self.canonical_value().encode()
    }
}

/// One closed, deduplicated, acyclic, size-bounded content-addressed source graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceGraph {
    nodes: Vec<SourceNode>,
    index: BTreeMap<SemanticDigest, usize>,
    edges: Vec<SourceEdge>,
    roots: Vec<SourceNodeRef>,
    budgets: SourceBudgets,
    digest: SemanticDigest,
}

impl SourceGraph {
    /// Aggregates nodes, materialized child edges, declared edges, and current roots.
    ///
    /// Every child reference of every node becomes one positioned `contains`
    /// edge, so the aggregate edge set is exactly the Merkle containment
    /// structure plus the declared navigation relations. One declared
    /// `contains` edge repeats one materialized child edge exactly or fails
    /// closed, so no declared edge adds containment that no node preimage
    /// binds.
    ///
    /// Aggregation is idempotent: equal nodes, equal edges, and equal roots
    /// collapse into one canonical graph. Two unequal nodes under one digest
    /// fail closed instead.
    ///
    /// Every check reads one canonical set in one fixed precedence, so no
    /// input permutation changes the reported diagnostic.
    ///
    /// # Errors
    ///
    /// Rejects unequal nodes under one digest, node or edge counts above their
    /// configured budgets, references absent from the same graph, duplicate or
    /// noncontiguous child positions, unbacked declared containment, and every
    /// directed cycle.
    pub fn new(
        nodes: Vec<SourceNode>,
        edges: Vec<SourceEdge>,
        roots: Vec<SourceNodeRef>,
        budgets: SourceBudgets,
    ) -> Result<Self, SourceGraphError> {
        Self::build(None, nodes, edges, roots, budgets)
    }

    /// Aggregates one source graph under an explicit configured member.
    ///
    /// Every node, edge endpoint, and root must already address the active
    /// member, so one aggregate never mixes two content-address spaces that no
    /// compact locator could distinguish.
    ///
    /// # Errors
    ///
    /// Returns every [`SourceGraph::new`] failure and rejects a node, edge
    /// endpoint, or root outside the active member.
    pub fn new_with(
        active: ActiveAlgorithm,
        nodes: Vec<SourceNode>,
        edges: Vec<SourceEdge>,
        roots: Vec<SourceNodeRef>,
        budgets: SourceBudgets,
    ) -> Result<Self, SourceGraphError> {
        Self::build(Some(active), nodes, edges, roots, budgets)
    }

    fn build(
        active: Option<ActiveAlgorithm>,
        nodes: Vec<SourceNode>,
        edges: Vec<SourceEdge>,
        roots: Vec<SourceNodeRef>,
        budgets: SourceBudgets,
    ) -> Result<Self, SourceGraphError> {
        if let Some(active) = active {
            let member = active.algorithm();
            if nodes.iter().any(|node| node.digest.algorithm() != member)
                || edges.iter().any(|edge| {
                    edge.source.digest().algorithm() != member
                        || edge.target.digest().algorithm() != member
                })
                || roots.iter().any(|root| root.digest().algorithm() != member)
            {
                return Err(SourceGraphError::NodeDigestMember);
            }
        }
        let mut ordered = BTreeMap::new();
        for node in nodes {
            if ordered
                .insert(node.digest, node.clone())
                .is_some_and(|existing| existing != node)
            {
                return Err(SourceGraphError::NodeDigestCollision);
            }
        }
        if ordered.len() > budgets.nodes.get() as usize {
            return Err(SourceGraphError::NodeBudget);
        }

        let aggregate = aggregate_source_edges(&ordered, edges, budgets)?;

        let current = roots.into_iter().collect::<BTreeSet<_>>();
        if current
            .iter()
            .any(|root| !ordered.contains_key(&root.digest()))
        {
            return Err(SourceGraphError::DanglingRoot);
        }
        validate_source_acyclic(&ordered, &aggregate)?;

        let fields = alloc::vec![
            (
                1,
                CanonicalValue::Set(ordered.values().map(SourceNode::canonical_value).collect()),
            ),
            (
                2,
                CanonicalValue::Set(aggregate.iter().map(SourceEdge::canonical_value).collect()),
            ),
            (
                3,
                CanonicalValue::Set(current.iter().map(|root| root.canonical_value()).collect()),
            ),
            (4, budgets.canonical_value()),
        ];
        let digest = match active {
            Some(active) => semantic_digest_trusted_with(active, "source-graph", fields),
            None => semantic_digest_trusted("source-graph", fields),
        };
        let nodes = ordered.into_values().collect::<Vec<_>>();
        let index = nodes
            .iter()
            .enumerate()
            .map(|(position, node)| (node.digest, position))
            .collect();
        Ok(Self {
            nodes,
            index,
            edges: aggregate.into_iter().collect(),
            roots: current.into_iter().collect(),
            budgets,
            digest,
        })
    }

    /// Nodes in canonical digest order.
    #[must_use]
    pub fn nodes(&self) -> &[SourceNode] {
        &self.nodes
    }

    /// Aggregate edges in canonical source, relation, position, and target order.
    #[must_use]
    pub fn edges(&self) -> &[SourceEdge] {
        &self.edges
    }

    /// Declared current roots in canonical digest order.
    #[must_use]
    pub fn roots(&self) -> &[SourceNodeRef] {
        &self.roots
    }

    /// Configured positive resource limits.
    #[must_use]
    pub const fn budgets(&self) -> SourceBudgets {
        self.budgets
    }

    /// Returns one node by exact reference.
    #[must_use]
    pub fn node(&self, reference: SourceNodeRef) -> Option<&SourceNode> {
        self.index
            .get(&reference.digest())
            .and_then(|position| self.nodes.get(*position))
    }

    /// Complete `source-graph` digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Frozen canonical bytes for inventory vectors and independent consumers.
    ///
    /// # Errors
    ///
    /// Returns the canonical encoder error for an unsupported value.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EncodingError> {
        CanonicalValue::Record(alloc::vec![
            (
                1,
                CanonicalValue::Set(self.nodes.iter().map(SourceNode::canonical_value).collect()),
            ),
            (
                2,
                CanonicalValue::Set(self.edges.iter().map(SourceEdge::canonical_value).collect()),
            ),
            (
                3,
                CanonicalValue::Set(
                    self.roots
                        .iter()
                        .map(|root| root.canonical_value())
                        .collect(),
                ),
            ),
            (4, self.budgets.canonical_value()),
        ])
        .encode()
    }
}

/// Unions materialized child edges with declared edges and validates the union.
///
/// Every declared `contains` edge repeats one materialized child edge exactly
/// or fails closed, so the aggregate containment relation stays exactly the
/// union of the node preimages. Each check reads the canonical union in one
/// fixed precedence, so no declaration order changes the diagnostic.
fn aggregate_source_edges(
    nodes: &BTreeMap<SemanticDigest, SourceNode>,
    declared: Vec<SourceEdge>,
    budgets: SourceBudgets,
) -> Result<BTreeSet<SourceEdge>, SourceGraphError> {
    let mut materialized = BTreeSet::new();
    for node in nodes.values() {
        for child in &node.children {
            materialized.insert(SourceEdge {
                source: node.reference(),
                relation: child.relation,
                position: Some(child.position),
                target: child.target,
            });
        }
    }
    let mut aggregate = materialized.clone();
    aggregate.extend(declared);

    if aggregate.len() > budgets.edges.get() as usize {
        return Err(SourceGraphError::EdgeBudget);
    }
    if aggregate.iter().any(|edge| {
        !nodes.contains_key(&edge.source.digest()) || !nodes.contains_key(&edge.target.digest())
    }) {
        return Err(SourceGraphError::DanglingEdge);
    }
    let positions = aggregate
        .iter()
        .filter_map(|edge| {
            edge.position
                .map(|position| (edge.source, edge.relation, position))
        })
        .collect::<Vec<_>>();
    validate_edge_positions(&positions)?;
    if aggregate
        .iter()
        .any(|edge| edge.relation == SourceEdgeKind::Contains && !materialized.contains(edge))
    {
        return Err(SourceGraphError::UnbackedContainment);
    }
    Ok(aggregate)
}

/// Rejects every directed cycle, including one that no current root reaches.
///
/// Kahn settlement removes zero-indegree nodes over the complete aggregate, so
/// an unreachable cycle never settles and acyclicity becomes one construction
/// invariant rather than one traversal outcome.
fn validate_source_acyclic(
    nodes: &BTreeMap<SemanticDigest, SourceNode>,
    edges: &BTreeSet<SourceEdge>,
) -> Result<(), SourceGraphError> {
    let mut children = BTreeMap::<SemanticDigest, Vec<SemanticDigest>>::new();
    let mut indegree = nodes
        .keys()
        .map(|digest| (*digest, 0_usize))
        .collect::<BTreeMap<_, _>>();
    for edge in edges {
        children
            .entry(edge.source.digest())
            .or_default()
            .push(edge.target.digest());
        indegree
            .entry(edge.target.digest())
            .and_modify(|count| *count += 1);
    }
    let mut ready = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(digest, _)| *digest)
        .collect::<Vec<_>>();
    let mut settled = 0_usize;
    while let Some(digest) = ready.pop() {
        settled += 1;
        for child in children.get(&digest).map_or(&[][..], Vec::as_slice) {
            indegree.entry(*child).and_modify(|count| {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    ready.push(*child);
                }
            });
        }
    }
    if settled == nodes.len() {
        Ok(())
    } else {
        Err(SourceGraphError::Cycle)
    }
}

/// Rejects duplicate or noncontiguous positions under one parent and relation.
fn validate_edge_positions(
    positions: &[(SourceNodeRef, SourceEdgeKind, u32)],
) -> Result<(), SourceGraphError> {
    if positions.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(SourceGraphError::DuplicateChildPosition);
    }
    let mut group = None;
    let mut expected = 0_u32;
    for (edge_source, relation, position) in positions.iter().copied() {
        if group != Some((edge_source, relation)) {
            group = Some((edge_source, relation));
            expected = 0;
        }
        if position != expected {
            return Err(SourceGraphError::ChildPositionGap);
        }
        expected = expected.saturating_add(1);
    }
    Ok(())
}

/// Deterministic fail-closed source-reference graph failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceGraphError {
    /// A relation name is outside the frozen source-edge vocabulary.
    EdgeKindGrammar,
    /// Node reference text is not one canonical algorithm-qualified digest.
    NodeReferenceText(DigestTextError),
    /// A configured resource budget was zero.
    NonPositiveBudget,
    /// Two child references share one relation and position under one parent.
    DuplicateChildPosition,
    /// Child positions under one parent and relation are not contiguous from zero.
    ChildPositionGap,
    /// A `contains` edge omitted its nonnegative child position.
    MissingChildPosition,
    /// A navigation relation carried one child position.
    PositionedNavigation,
    /// Two unequal nodes claim one complete node digest.
    NodeDigestCollision,
    /// A node, child, or root digest is outside the configured active member.
    NodeDigestMember,
    /// A declared `contains` edge is absent from its parent node preimage.
    UnbackedContainment,
    /// An edge endpoint is absent from the same graph.
    DanglingEdge,
    /// A declared current root is absent from the same graph.
    DanglingRoot,
    /// A traversal start node is absent from the same graph.
    UnknownNode,
    /// The source graph contains a directed cycle.
    Cycle,
    /// A referenced node is unreachable from every current root.
    StaleReference,
    /// Total nodes exceed the configured node budget.
    NodeBudget,
    /// Total edges exceed the configured edge budget.
    EdgeBudget,
    /// Visited nodes exceed the configured visited budget.
    VisitedBudget,
    /// Traversed depth exceeds the configured depth budget.
    DepthBudget,
}

impl SourceGraphError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EdgeKindGrammar => "SRG-EDGE-KIND",
            Self::NodeReferenceText(error) => error.code(),
            Self::NonPositiveBudget => "SRG-BUDGET-NONPOSITIVE",
            Self::DuplicateChildPosition => "SRG-CHILD-POSITION-DUPLICATE",
            Self::ChildPositionGap => "SRG-CHILD-POSITION-GAP",
            Self::MissingChildPosition => "SRG-CHILD-POSITION-MISSING",
            Self::PositionedNavigation => "SRG-CHILD-POSITION-NAVIGATION",
            Self::NodeDigestCollision => "SRG-NODE-COLLISION",
            Self::NodeDigestMember => "SRG-NODE-MEMBER",
            Self::UnbackedContainment => "SRG-EDGE-UNBACKED",
            Self::DanglingEdge => "SRG-EDGE-DANGLING",
            Self::DanglingRoot => "SRG-ROOT-DANGLING",
            Self::UnknownNode => "SRG-NODE-UNKNOWN",
            Self::Cycle => "SRG-SOURCE-CYCLE",
            Self::StaleReference => "SRG-SOURCE-STALE",
            Self::NodeBudget => "SRG-BUDGET-NODES",
            Self::EdgeBudget => "SRG-BUDGET-EDGES",
            Self::VisitedBudget => "SRG-BUDGET-VISITED",
            Self::DepthBudget => "SRG-BUDGET-DEPTH",
        }
    }
}

/// A parsed reference to one exact content-addressed binding record.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ImplementationBindingRef(SemanticDigest);

impl ImplementationBindingRef {
    /// Parses one algorithm-qualified record digest at the untrusted-input boundary.
    ///
    /// # Errors
    ///
    /// Rejects text outside the canonical algorithm-qualified digest form.
    pub fn parse(input: &str) -> Result<Self, DigestTextError> {
        SemanticDigest::parse_canonical(input).map(Self)
    }

    /// Wraps one record digest already computed by this core.
    #[must_use]
    pub const fn new(digest: SemanticDigest) -> Self {
        Self(digest)
    }

    /// Referenced record digest.
    #[must_use]
    pub const fn digest(self) -> SemanticDigest {
        self.0
    }

    /// Canonical algorithm-qualified reference text.
    #[must_use]
    pub fn canonical_text(self) -> String {
        self.0.canonical_text()
    }
}

/// One materialized correspondence from a specification node to its atom revision.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourceAuthority {
    node: SourceNodeRef,
    atom: AtomRevisionRef,
}

impl SourceAuthority {
    /// Binds one specification source node to the atom revision it materializes.
    #[must_use]
    pub const fn new(node: SourceNodeRef, atom: AtomRevisionRef) -> Self {
        Self { node, atom }
    }

    /// Exact specification node reference.
    #[must_use]
    pub const fn node(&self) -> SourceNodeRef {
        self.node
    }

    /// Exact governing atom revision.
    #[must_use]
    pub const fn atom(&self) -> &AtomRevisionRef {
        &self.atom
    }
}

/// One compact binding locator attached to one exact implementation source node.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BindingAttachment {
    subject: SourceNodeRef,
    record: ImplementationBindingRef,
}

impl BindingAttachment {
    /// Collects one attached subject node and the locator its source declares.
    #[must_use]
    pub const fn new(subject: SourceNodeRef, record: ImplementationBindingRef) -> Self {
        Self { subject, record }
    }

    /// Source node that carries the compact locator.
    #[must_use]
    pub const fn subject(&self) -> SourceNodeRef {
        self.subject
    }

    /// Declared record locator.
    #[must_use]
    pub const fn record(&self) -> ImplementationBindingRef {
        self.record
    }
}

/// Immutable content-addressed implementation binding that supplies no assurance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplementationBinding {
    meta: ObjectMeta,
    navigation: NavigationKind,
    subject: SourceNodeRef,
    requirement: SourceNodeRef,
    atom: AtomRevisionRef,
}

impl ImplementationBinding {
    /// Constructs one immutable binding record and computes its complete digest.
    ///
    /// The record digest binds the stable relation ID, navigation relation,
    /// exact implementation subject node, exact specification requirement node,
    /// and exact atom revision under `buoy/implementation-binding/v1`. The
    /// stable relation ID therefore preserves base-to-head impact identity
    /// while every field change produces one new immutable record.
    ///
    /// # Errors
    ///
    /// Rejects a subject, requirement, or atom revision outside one registered
    /// digest member, because one compact locator cannot resolve references
    /// that separate members address.
    pub fn new(
        relation: StableId,
        navigation: NavigationKind,
        subject: SourceNodeRef,
        requirement: SourceNodeRef,
        atom: AtomRevisionRef,
    ) -> Result<Self, ConstructionError> {
        Self::build(None, relation, navigation, subject, requirement, atom)
    }

    /// Constructs one immutable binding record under an explicit active member.
    ///
    /// A compact locator names no algorithm, so the record identity must be
    /// computed under the one member the repository configured rather than
    /// under the first member this build contains. Every reference the record
    /// binds must already address that same member.
    ///
    /// # Errors
    ///
    /// Returns every [`ImplementationBinding::new`] failure and rejects a
    /// subject, requirement, or atom revision outside the active member.
    pub fn new_with(
        active: ActiveAlgorithm,
        relation: StableId,
        navigation: NavigationKind,
        subject: SourceNodeRef,
        requirement: SourceNodeRef,
        atom: AtomRevisionRef,
    ) -> Result<Self, ConstructionError> {
        Self::build(
            Some(active),
            relation,
            navigation,
            subject,
            requirement,
            atom,
        )
    }

    fn build(
        active: Option<ActiveAlgorithm>,
        relation: StableId,
        navigation: NavigationKind,
        subject: SourceNodeRef,
        requirement: SourceNodeRef,
        atom: AtomRevisionRef,
    ) -> Result<Self, ConstructionError> {
        require_same_digest_algorithm(subject.digest(), requirement.digest())?;
        require_same_digest_algorithm(subject.digest(), atom.revision())?;
        if active.is_some_and(|active| active.algorithm() != subject.digest().algorithm()) {
            return Err(ConstructionError::DigestAlgorithmMismatch);
        }
        let relation_value = text_value(relation.as_str());
        // SourceAuthority binds this atom revision to the exact requirement
        // node. The semantic graph contains no separate Atom object requirement
        // for compact navigation records.
        let references = Vec::new();
        let fields = alloc::vec![
            (1, relation_value),
            (2, CanonicalValue::Enum(navigation as u16, None)),
            (3, CanonicalValue::Digest(subject.digest())),
            (4, CanonicalValue::Digest(requirement.digest())),
            (5, atom_revision_reference_value(&atom)),
        ];
        let meta = match active {
            Some(active) => ObjectMeta::for_fields_with(
                active,
                ObjectKind::ImplementationBinding,
                relation,
                "buoy/implementation-binding/v1",
                fields,
                references,
            ),
            None => ObjectMeta::for_fields(
                ObjectKind::ImplementationBinding,
                relation,
                "buoy/implementation-binding/v1",
                fields,
                references,
            ),
        };
        Ok(Self {
            meta,
            navigation,
            subject,
            requirement,
            atom,
        })
    }

    /// Shared metadata whose stable ID is the relation ID.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Closed navigation relation.
    #[must_use]
    pub const fn navigation(&self) -> NavigationKind {
        self.navigation
    }

    /// Exact implementation subject node reference.
    #[must_use]
    pub const fn subject(&self) -> SourceNodeRef {
        self.subject
    }

    /// Exact specification requirement node reference.
    #[must_use]
    pub const fn requirement(&self) -> SourceNodeRef {
        self.requirement
    }

    /// Exact governing atom revision.
    #[must_use]
    pub const fn atom(&self) -> &AtomRevisionRef {
        &self.atom
    }

    /// Locator for this exact record.
    #[must_use]
    pub const fn record(&self) -> ImplementationBindingRef {
        ImplementationBindingRef::new(self.meta.digest)
    }
}

/// One object-store member: the locator a store used and the record it holds.
///
/// The store, not this core, reads repository paths, so the locator arrives as
/// an independent claim that binding validation compares with the recomputed
/// record digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredBinding {
    locator: ImplementationBindingRef,
    record: ImplementationBinding,
}

impl StoredBinding {
    /// Pairs one claimed store locator with one materialized record.
    #[must_use]
    pub const fn new(locator: ImplementationBindingRef, record: ImplementationBinding) -> Self {
        Self { locator, record }
    }

    /// Locator the store claimed for this record.
    #[must_use]
    pub const fn locator(&self) -> ImplementationBindingRef {
        self.locator
    }

    /// Materialized immutable record.
    #[must_use]
    pub const fn record(&self) -> &ImplementationBinding {
        &self.record
    }
}

/// Navigational implementation link that supplies no assurance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplementationLink {
    meta: ObjectMeta,
    atoms: Vec<AtomRevisionRef>,
    parsed_item_digest: SemanticDigest,
    navigation: NavigationKind,
}

impl ImplementationLink {
    /// Constructs one complete navigational implementation link.
    ///
    /// # Errors
    ///
    /// Rejects an empty or duplicate atom-revision set or noncanonical text.
    pub fn new(
        id: StableId,
        atoms: Vec<AtomRevisionRef>,
        parsed_item_digest: SemanticDigest,
        navigation: NavigationKind,
    ) -> Result<Self, ConstructionError> {
        if atoms.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let atoms = canonicalize_set(atoms, atom_revision_reference_value)?;
        let id_value = text_value(id.as_str());
        let meta = ObjectMeta::for_fields(
            ObjectKind::ImplementationLink,
            id,
            "implementation-link",
            alloc::vec![
                (1, id_value),
                (
                    2,
                    CanonicalValue::Set(atoms.iter().map(atom_revision_reference_value).collect(),),
                ),
                (3, CanonicalValue::Digest(parsed_item_digest)),
                (4, CanonicalValue::Enum(navigation as u16, None)),
            ],
            Vec::new(),
        );
        Ok(Self {
            meta,
            atoms,
            parsed_item_digest,
            navigation,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Exact atom-revision set.
    #[must_use]
    pub fn atoms(&self) -> &[AtomRevisionRef] {
        &self.atoms
    }

    /// Parser-canonical attached item identity digest.
    #[must_use]
    pub const fn parsed_item_digest(&self) -> SemanticDigest {
        self.parsed_item_digest
    }

    /// Closed navigation relation.
    #[must_use]
    pub const fn navigation(&self) -> NavigationKind {
        self.navigation
    }
}

/// Repository-owned external navigation link that supplies no assurance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalTraceLink {
    meta: ObjectMeta,
    atoms: Vec<AtomRevisionRef>,
    relation: ExternalTraceRelation,
    target: ExternalTraceTarget,
}

impl ExternalTraceLink {
    /// Constructs one complete external trace link.
    ///
    /// # Errors
    ///
    /// Rejects an empty or duplicate atom-revision set.
    pub fn new(
        id: StableId,
        atoms: Vec<AtomRevisionRef>,
        relation: ExternalTraceRelation,
        target: ExternalTraceTarget,
    ) -> Result<Self, ConstructionError> {
        if atoms.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let atoms = canonicalize_set(atoms, atom_revision_reference_value)?;
        let id_value = text_value(id.as_str());
        let meta = ObjectMeta::for_fields(
            ObjectKind::ExternalTraceLink,
            id,
            "external-trace-link",
            alloc::vec![
                (1, id_value),
                (
                    2,
                    CanonicalValue::Set(atoms.iter().map(atom_revision_reference_value).collect()),
                ),
                (3, CanonicalValue::Enum(relation as u16, None)),
                (4, CanonicalValue::Text(target.as_str().to_string())),
            ],
            Vec::new(),
        );
        Ok(Self {
            meta,
            atoms,
            relation,
            target,
        })
    }

    /// Returns the shared object metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Returns the exact atom-revision set.
    #[must_use]
    pub fn atoms(&self) -> &[AtomRevisionRef] {
        &self.atoms
    }

    /// Returns the closed link relation.
    #[must_use]
    pub const fn relation(&self) -> ExternalTraceRelation {
        self.relation
    }

    /// Returns the normalized external target.
    #[must_use]
    pub const fn target(&self) -> &ExternalTraceTarget {
        &self.target
    }

    /// Returns atom IDs whose bound revisions are absent or stale.
    #[must_use]
    pub fn stale_atom_ids(&self, current: &BTreeMap<StableId, SemanticDigest>) -> Vec<StableId> {
        self.atoms
            .iter()
            .filter(|atom| current.get(atom.id()) != Some(&atom.revision()))
            .map(|atom| atom.id().clone())
            .collect()
    }
}

/// Pinned oracle selection and normalization declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Oracle {
    meta: ObjectMeta,
    kind: OracleKind,
    runner: ToolIdentity,
    selection: TypedPayload,
    normalization: TypedPayload,
    configuration: SemanticDigest,
    scope: TypedPayload,
    exclusions: Vec<String>,
}

impl Oracle {
    /// Constructs one complete oracle declaration and computes its object digest.
    ///
    /// # Errors
    ///
    /// Rejects duplicate exclusions or noncanonical retained text.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: StableId,
        kind: OracleKind,
        runner: ToolIdentity,
        selection: TypedPayload,
        normalization: TypedPayload,
        configuration: SemanticDigest,
        scope: TypedPayload,
        exclusions: Vec<String>,
    ) -> Result<Self, ConstructionError> {
        let exclusions = canonicalize_nonempty_text_set(exclusions)?;
        let id_value = text_value(id.as_str());
        let meta = ObjectMeta::for_fields(
            ObjectKind::Oracle,
            id,
            "oracle",
            alloc::vec![
                (1, id_value),
                (2, CanonicalValue::Enum(kind as u16, None)),
                (3, runner.canonical_value()),
                (4, selection.canonical_value()),
                (5, normalization.canonical_value()),
                (6, CanonicalValue::Digest(configuration)),
                (7, scope.canonical_value()),
                (
                    8,
                    CanonicalValue::Set(
                        exclusions
                            .iter()
                            .cloned()
                            .map(CanonicalValue::Text)
                            .collect(),
                    ),
                ),
            ],
            Vec::new(),
        );
        Ok(Self {
            meta,
            kind,
            runner,
            selection,
            normalization,
            configuration,
            scope,
            exclusions,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Closed oracle category.
    #[must_use]
    pub const fn oracle_kind(&self) -> OracleKind {
        self.kind
    }

    /// Exact runner tool identity.
    #[must_use]
    pub const fn runner(&self) -> &ToolIdentity {
        &self.runner
    }

    /// Pinned selection semantics.
    #[must_use]
    pub const fn selection(&self) -> &TypedPayload {
        &self.selection
    }

    /// Pinned verdict-normalization semantics.
    #[must_use]
    pub const fn normalization(&self) -> &TypedPayload {
        &self.normalization
    }

    /// Complete execution-configuration digest.
    #[must_use]
    pub const fn configuration(&self) -> SemanticDigest {
        self.configuration
    }

    /// Exact oracle claim scope.
    #[must_use]
    pub const fn scope(&self) -> &TypedPayload {
        &self.scope
    }

    /// Explicit exclusions in canonical set order.
    #[must_use]
    pub fn exclusions(&self) -> &[String] {
        &self.exclusions
    }
}

/// One normalized oracle/subject verdict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedVerdict {
    oracle: ObjectRef,
    selected_subjects: Vec<ObjectRef>,
    verdict: Verdict,
    semantic_result: Option<TypedPayload>,
    digest: SemanticDigest,
}

impl NormalizedVerdict {
    /// Constructs one normalized verdict with an explicit semantic-result option.
    ///
    /// # Errors
    ///
    /// Rejects duplicate selected subjects, noncanonical text, or a semantic
    /// result attached to `not-run`.
    pub fn new(
        oracle: ObjectRef,
        selected_subjects: Vec<ObjectRef>,
        verdict: Verdict,
        semantic_result: Option<TypedPayload>,
    ) -> Result<Self, ConstructionError> {
        if verdict == Verdict::NotRun && semantic_result.is_some() {
            return Err(ConstructionError::VerdictResultShape);
        }
        if oracle.kind() != ObjectKind::Oracle {
            return Err(ConstructionError::ReferenceKind);
        }
        let selected_subjects = canonicalize_set(selected_subjects, object_reference_value)?;
        let fields = normalized_verdict_fields(
            &oracle,
            &selected_subjects,
            verdict,
            semantic_result.as_ref(),
        );
        let digest = semantic_digest_trusted("normalized-verdict", fields);
        Ok(Self {
            oracle,
            selected_subjects,
            verdict,
            semantic_result,
            digest,
        })
    }

    /// Exact oracle reference.
    #[must_use]
    pub const fn oracle(&self) -> &ObjectRef {
        &self.oracle
    }

    /// Canonically ordered selected subject set.
    #[must_use]
    pub fn selected_subjects(&self) -> &[ObjectRef] {
        &self.selected_subjects
    }

    /// Closed five-state verdict.
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// Optional schema-bound semantic result.
    #[must_use]
    pub const fn semantic_result(&self) -> Option<&TypedPayload> {
        self.semantic_result.as_ref()
    }

    /// Complete normalized-verdict digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    pub(crate) fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(normalized_verdict_fields(
            &self.oracle,
            &self.selected_subjects,
            self.verdict,
            self.semantic_result.as_ref(),
        ))
    }
}

fn normalized_verdict_fields(
    oracle: &ObjectRef,
    selected_subjects: &[ObjectRef],
    verdict: Verdict,
    semantic_result: Option<&TypedPayload>,
) -> Vec<(u16, CanonicalValue)> {
    alloc::vec![
        (1, object_reference_value(oracle)),
        (
            2,
            CanonicalValue::Set(
                selected_subjects
                    .iter()
                    .map(object_reference_value)
                    .collect(),
            ),
        ),
        (3, CanonicalValue::Enum(verdict as u16, None)),
        (
            4,
            semantic_result.map_or(CanonicalValue::None, |result| {
                CanonicalValue::Some(Box::new(result.canonical_value()))
            }),
        ),
    ]
}

/// Explicit reviewed formal or evidence assumption.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assumption {
    meta: ObjectMeta,
    statement: String,
    scope: TypedPayload,
    exclusions: Vec<String>,
    approval: ApprovalBinding,
}

impl Assumption {
    /// Constructs one complete reviewed assumption.
    ///
    /// # Errors
    ///
    /// Rejects an empty statement, duplicate exclusions, or noncanonical text.
    pub fn new(
        id: StableId,
        statement: impl Into<String>,
        scope: TypedPayload,
        exclusions: Vec<String>,
        approval: ApprovalBinding,
    ) -> Result<Self, ConstructionError> {
        let statement = statement.into();
        require_nonempty_nfc(&statement)?;
        let exclusions = canonicalize_nonempty_text_set(exclusions)?;
        let id_value = text_value(id.as_str());
        let meta = ObjectMeta::for_fields(
            ObjectKind::Assumption,
            id,
            "assumption",
            alloc::vec![
                (1, id_value),
                (2, CanonicalValue::Text(statement.clone())),
                (3, scope.canonical_value()),
                (
                    4,
                    CanonicalValue::Set(
                        exclusions
                            .iter()
                            .cloned()
                            .map(CanonicalValue::Text)
                            .collect(),
                    ),
                ),
                (5, approval_binding_value(&approval)),
            ],
            Vec::new(),
        );
        Ok(Self {
            meta,
            statement,
            scope,
            exclusions,
            approval,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Normative assumption statement.
    #[must_use]
    pub fn statement(&self) -> &str {
        &self.statement
    }

    /// Exact assumption scope.
    #[must_use]
    pub const fn scope(&self) -> &TypedPayload {
        &self.scope
    }

    /// Explicit exclusions in canonical set order.
    #[must_use]
    pub fn exclusions(&self) -> &[String] {
        &self.exclusions
    }

    /// Repository-owned correspondence approval.
    #[must_use]
    pub const fn approval(&self) -> &ApprovalBinding {
        &self.approval
    }
}

/// Finite state/event/transition model declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteModel {
    meta: ObjectMeta,
    state_space: TypedPayload,
    event_alphabet: TypedPayload,
    transition_relation: TypedPayload,
    initial_states: Vec<TypedPayload>,
    bound: TypedPayload,
    checked_invariants: Vec<AtomRevisionRef>,
    assumptions: Vec<ObjectRef>,
    exclusions: Vec<String>,
}

impl FiniteModel {
    /// Constructs one complete finite model declaration.
    ///
    /// # Errors
    ///
    /// Rejects an empty initial-state set, duplicate sets, or noncanonical text.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: StableId,
        state_space: TypedPayload,
        event_alphabet: TypedPayload,
        transition_relation: TypedPayload,
        initial_states: Vec<TypedPayload>,
        bound: TypedPayload,
        checked_invariants: Vec<AtomRevisionRef>,
        assumptions: Vec<ObjectRef>,
        exclusions: Vec<String>,
    ) -> Result<Self, ConstructionError> {
        if initial_states.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let initial_states = canonicalize_set(initial_states, TypedPayload::canonical_value)?;
        let checked_invariants =
            canonicalize_set(checked_invariants, atom_revision_reference_value)?;
        let assumptions = canonicalize_set(assumptions, object_reference_value)?;
        if assumptions
            .iter()
            .any(|assumption| assumption.kind() != ObjectKind::Assumption)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        let exclusions = canonicalize_nonempty_text_set(exclusions)?;
        let id_value = text_value(id.as_str());
        let meta = ObjectMeta::for_fields(
            ObjectKind::FiniteModel,
            id,
            "finite-model",
            alloc::vec![
                (1, id_value),
                (2, state_space.canonical_value()),
                (3, event_alphabet.canonical_value()),
                (4, transition_relation.canonical_value()),
                (
                    5,
                    CanonicalValue::Set(
                        initial_states
                            .iter()
                            .map(TypedPayload::canonical_value)
                            .collect(),
                    ),
                ),
                (6, bound.canonical_value()),
                (
                    7,
                    CanonicalValue::Set(
                        checked_invariants
                            .iter()
                            .map(atom_revision_reference_value)
                            .collect(),
                    ),
                ),
                (
                    8,
                    CanonicalValue::Set(assumptions.iter().map(object_reference_value).collect(),),
                ),
                (
                    9,
                    CanonicalValue::Set(
                        exclusions
                            .iter()
                            .cloned()
                            .map(CanonicalValue::Text)
                            .collect(),
                    ),
                ),
            ],
            assumptions.clone(),
        );
        Ok(Self {
            meta,
            state_space,
            event_alphabet,
            transition_relation,
            initial_states,
            bound,
            checked_invariants,
            assumptions,
            exclusions,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Pinned state-space schema and bytes.
    #[must_use]
    pub const fn state_space(&self) -> &TypedPayload {
        &self.state_space
    }

    /// Pinned event alphabet.
    #[must_use]
    pub const fn event_alphabet(&self) -> &TypedPayload {
        &self.event_alphabet
    }

    /// Pinned transition relation.
    #[must_use]
    pub const fn transition_relation(&self) -> &TypedPayload {
        &self.transition_relation
    }

    /// Nonempty canonical initial-state set.
    #[must_use]
    pub fn initial_states(&self) -> &[TypedPayload] {
        &self.initial_states
    }

    /// Exact declared exploration bound.
    #[must_use]
    pub const fn bound(&self) -> &TypedPayload {
        &self.bound
    }

    /// Atom revisions whose invariants are checked.
    #[must_use]
    pub fn checked_invariants(&self) -> &[AtomRevisionRef] {
        &self.checked_invariants
    }

    /// Explicit assumption references.
    #[must_use]
    pub fn assumptions(&self) -> &[ObjectRef] {
        &self.assumptions
    }

    /// Explicit exclusions in canonical set order.
    #[must_use]
    pub fn exclusions(&self) -> &[String] {
        &self.exclusions
    }
}

/// Review-only exact base-to-head reaffirmation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reaffirmation {
    meta: ObjectMeta,
    affected_object_kind: ObjectKind,
    affected_object_id: StableId,
    base_object_digest: SemanticDigest,
    head_object_digest: SemanticDigest,
    review_approval: ApprovalBinding,
    review_statement: String,
}

impl Reaffirmation {
    /// Constructs one exact individually approved rereview record.
    ///
    /// # Errors
    ///
    /// Rejects empty retained text, equal base/head digests, or noncanonical
    /// semantic fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: StableId,
        affected_object_kind: ObjectKind,
        affected_object_id: StableId,
        base_object_digest: SemanticDigest,
        head_object_digest: SemanticDigest,
        review_approval: ApprovalBinding,
        review_statement: impl Into<String>,
    ) -> Result<Self, ConstructionError> {
        let review_statement = review_statement.into();
        require_nonempty_nfc(&review_statement)?;
        require_same_digest_algorithm(base_object_digest, head_object_digest)?;
        if base_object_digest == head_object_digest {
            return Err(ConstructionError::EqualDigests);
        }
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (2, CanonicalValue::Enum(affected_object_kind as u16, None),),
            (3, CanonicalValue::Digest(base_object_digest)),
            (4, CanonicalValue::Digest(head_object_digest)),
            (5, approval_binding_value(&review_approval)),
            (6, CanonicalValue::Text(review_statement.clone())),
            (7, text_value(affected_object_id.as_str())),
        ];
        let affected = ObjectRef::new(
            affected_object_kind,
            affected_object_id.clone(),
            head_object_digest,
        );
        let meta = ObjectMeta::for_fields(
            ObjectKind::Reaffirmation,
            id,
            "reaffirmation",
            fields,
            alloc::vec![affected],
        );
        Ok(Self {
            meta,
            affected_object_kind,
            affected_object_id,
            base_object_digest,
            head_object_digest,
            review_approval,
            review_statement,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Exact affected object kind.
    #[must_use]
    pub const fn affected_object_kind(&self) -> ObjectKind {
        self.affected_object_kind
    }

    /// Exact affected stable object ID.
    #[must_use]
    pub const fn affected_object_id(&self) -> &StableId {
        &self.affected_object_id
    }

    /// Exact base object digest.
    #[must_use]
    pub const fn base_object_digest(&self) -> SemanticDigest {
        self.base_object_digest
    }

    /// Exact different head object digest.
    #[must_use]
    pub const fn head_object_digest(&self) -> SemanticDigest {
        self.head_object_digest
    }

    /// Repository-owned review approval.
    #[must_use]
    pub const fn review_approval(&self) -> &ApprovalBinding {
        &self.review_approval
    }

    /// Nonempty review statement.
    #[must_use]
    pub fn review_statement(&self) -> &str {
        &self.review_statement
    }
}

/// Syntax-attached potential evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceCarrier {
    meta: ObjectMeta,
    role: EvidenceRole,
    atoms: Vec<AtomRevisionRef>,
    parsed_item_digest: SemanticDigest,
    oracle: ObjectRef,
    configuration: SemanticDigest,
    corpora: Vec<SemanticDigest>,
    scope: TypedPayload,
    assumptions: Vec<ObjectRef>,
    exclusions: Vec<String>,
}

impl EvidenceCarrier {
    /// Constructs one complete exact carrier registration.
    ///
    /// # Errors
    ///
    /// Rejects an empty/duplicate atom set, duplicate binding sets, or
    /// noncanonical retained text.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: StableId,
        role: EvidenceRole,
        atoms: Vec<AtomRevisionRef>,
        parsed_item_digest: SemanticDigest,
        oracle: ObjectRef,
        configuration: SemanticDigest,
        corpora: Vec<SemanticDigest>,
        scope: TypedPayload,
        assumptions: Vec<ObjectRef>,
        exclusions: Vec<String>,
    ) -> Result<Self, ConstructionError> {
        if atoms.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        if (
            oracle.kind(),
            assumptions
                .iter()
                .all(|assumption| assumption.kind() == ObjectKind::Assumption),
        ) != (ObjectKind::Oracle, true)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        if !role_scope_compatible(role, scope.schema()) {
            return Err(ConstructionError::RoleScopeShape);
        }
        let atoms = canonicalize_set(atoms, atom_revision_reference_value)?;
        let corpora = canonicalize_set(corpora, digest_value)?;
        let assumptions = canonicalize_set(assumptions, object_reference_value)?;
        let exclusions = canonicalize_nonempty_text_set(exclusions)?;
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (2, CanonicalValue::Enum(role as u16, None)),
            (
                3,
                CanonicalValue::Set(atoms.iter().map(atom_revision_reference_value).collect(),),
            ),
            (4, CanonicalValue::Digest(parsed_item_digest)),
            (5, object_reference_value(&oracle)),
            (6, CanonicalValue::Digest(configuration)),
            (
                7,
                CanonicalValue::Set(
                    corpora
                        .iter()
                        .copied()
                        .map(CanonicalValue::Digest)
                        .collect(),
                ),
            ),
            (8, scope.canonical_value()),
            (
                9,
                CanonicalValue::Set(assumptions.iter().map(object_reference_value).collect(),),
            ),
            (
                10,
                CanonicalValue::Set(
                    exclusions
                        .iter()
                        .cloned()
                        .map(CanonicalValue::Text)
                        .collect(),
                ),
            ),
        ];
        let mut references = BTreeSet::new();
        references.insert(oracle.clone());
        references.extend(assumptions.iter().cloned());
        let references =
            canonicalize_validated_set(references.into_iter().collect(), object_reference_value);
        let meta = ObjectMeta::for_fields(
            ObjectKind::EvidenceCarrier,
            id,
            "evidence-carrier",
            fields,
            references,
        );
        Ok(Self {
            meta,
            role,
            atoms,
            parsed_item_digest,
            oracle,
            configuration,
            corpora,
            scope,
            assumptions,
            exclusions,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Declared role.
    #[must_use]
    pub const fn role(&self) -> EvidenceRole {
        self.role
    }

    /// Exact atom revisions.
    #[must_use]
    pub fn atoms(&self) -> &[AtomRevisionRef] {
        &self.atoms
    }

    /// Parser-canonical attached item identity digest.
    #[must_use]
    pub const fn parsed_item_digest(&self) -> SemanticDigest {
        self.parsed_item_digest
    }

    /// Exact oracle reference.
    #[must_use]
    pub const fn oracle(&self) -> &ObjectRef {
        &self.oracle
    }

    /// Execution configuration digest.
    #[must_use]
    pub const fn configuration(&self) -> SemanticDigest {
        self.configuration
    }

    /// Exact corpus digests.
    #[must_use]
    pub fn corpora(&self) -> &[SemanticDigest] {
        &self.corpora
    }

    /// Exact claim scope.
    #[must_use]
    pub const fn scope(&self) -> &TypedPayload {
        &self.scope
    }

    /// Explicit assumption references.
    #[must_use]
    pub fn assumptions(&self) -> &[ObjectRef] {
        &self.assumptions
    }

    /// Explicit exclusions in canonical set order.
    #[must_use]
    pub fn exclusions(&self) -> &[String] {
        &self.exclusions
    }
}

/// Object or repository artifact derivation binding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ArtifactBinding {
    /// Exact semantic graph object.
    Object(ObjectRef),
    /// Exact selected repository artifact expectation.
    RepositoryArtifact(RepositoryArtifactBinding),
}

impl ArtifactBinding {
    /// Constructs a complete exact object-binding variant.
    #[must_use]
    pub const fn object(reference: ObjectRef) -> Self {
        Self::Object(reference)
    }

    /// Constructs a complete repository-artifact variant.
    ///
    /// # Errors
    ///
    /// Rejects an invalid domain, empty media type, or empty representation.
    pub fn repository(
        path: RepositoryPath,
        raw_domain: impl Into<String>,
        media_type: impl Into<String>,
        representation: impl Into<String>,
        digest: SemanticDigest,
    ) -> Result<Self, ConstructionError> {
        RepositoryArtifactBinding::new(path, raw_domain, media_type, representation, digest)
            .map(Self::RepositoryArtifact)
    }

    /// Closed binding variant.
    #[must_use]
    pub const fn kind(&self) -> ArtifactBindingKind {
        match self {
            Self::Object(_) => ArtifactBindingKind::Object,
            Self::RepositoryArtifact(_) => ArtifactBindingKind::RepositoryArtifact,
        }
    }

    /// Bound object reference when this is an object binding.
    #[must_use]
    pub const fn object_reference(&self) -> Option<&ObjectRef> {
        match self {
            Self::Object(reference) => Some(reference),
            Self::RepositoryArtifact(_) => None,
        }
    }

    /// Complete repository-artifact binding when selected.
    #[must_use]
    pub const fn repository_artifact(&self) -> Option<&RepositoryArtifactBinding> {
        match self {
            Self::Object(_) => None,
            Self::RepositoryArtifact(binding) => Some(binding),
        }
    }

    /// Exact bound semantic object or raw-artifact digest.
    #[must_use]
    pub const fn artifact_digest(&self) -> SemanticDigest {
        match self {
            Self::Object(reference) => reference.digest(),
            Self::RepositoryArtifact(binding) => binding.digest(),
        }
    }

    /// Computes the complete artifact-binding domain digest.
    ///
    /// # Errors
    ///
    /// Returns an encoding error for noncanonical retained text.
    pub fn binding_digest(&self) -> Result<SemanticDigest, EncodingError> {
        Ok(semantic_digest_trusted(
            "artifact-binding",
            self.canonical_fields(),
        ))
    }

    fn canonical_fields(&self) -> Vec<(u16, CanonicalValue)> {
        match self {
            Self::Object(reference) => alloc::vec![
                (
                    1,
                    CanonicalValue::Enum(ArtifactBindingKind::Object as u16, None)
                ),
                (
                    2,
                    CanonicalValue::Some(Box::new(object_reference_value(reference))),
                ),
                (3, CanonicalValue::None),
                (4, CanonicalValue::Digest(reference.digest())),
                (5, CanonicalValue::None),
                (6, CanonicalValue::None),
                (7, CanonicalValue::None),
            ],
            Self::RepositoryArtifact(binding) => alloc::vec![
                (
                    1,
                    CanonicalValue::Enum(ArtifactBindingKind::RepositoryArtifact as u16, None),
                ),
                (2, CanonicalValue::None),
                (
                    3,
                    CanonicalValue::Some(Box::new(CanonicalValue::Text(
                        binding.path.as_str().to_string(),
                    ))),
                ),
                (4, CanonicalValue::Digest(binding.digest)),
                (
                    5,
                    CanonicalValue::Some(Box::new(CanonicalValue::Text(
                        binding.raw_domain.clone(),
                    ))),
                ),
                (
                    6,
                    CanonicalValue::Some(Box::new(CanonicalValue::Text(
                        binding.media_type.clone(),
                    ))),
                ),
                (
                    7,
                    CanonicalValue::Some(Box::new(CanonicalValue::Text(
                        binding.representation.clone(),
                    ))),
                ),
            ],
        }
    }

    pub(crate) fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(self.canonical_fields())
    }
}

/// Complete repository-artifact binding expectation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RepositoryArtifactBinding {
    path: RepositoryPath,
    raw_domain: String,
    media_type: String,
    representation: String,
    digest: SemanticDigest,
}

fn parse_artifact_shape(
    raw_domain: String,
    media_type: String,
    representation: String,
) -> Result<(String, String, String), ConstructionError> {
    if raw_domain != "raw-artifact" || media_type.is_empty() || representation.is_empty() {
        return Err(ConstructionError::ArtifactBindingShape);
    }
    require_nfc(&media_type)?;
    require_nfc(&representation)?;
    Ok((raw_domain, media_type, representation))
}

impl RepositoryArtifactBinding {
    /// Constructs a complete frozen-v2 repository-artifact binding.
    ///
    /// # Errors
    ///
    /// Rejects a domain other than `raw-artifact` or empty media/representation identities.
    pub fn new(
        path: RepositoryPath,
        raw_domain: impl Into<String>,
        media_type: impl Into<String>,
        representation: impl Into<String>,
        digest: SemanticDigest,
    ) -> Result<Self, ConstructionError> {
        let (raw_domain, media_type, representation) =
            parse_artifact_shape(raw_domain.into(), media_type.into(), representation.into())?;
        Ok(Self {
            path,
            raw_domain,
            media_type,
            representation,
            digest,
        })
    }

    /// Canonical repository-relative path.
    #[must_use]
    pub const fn path(&self) -> &RepositoryPath {
        &self.path
    }

    /// Required semantic digest domain.
    #[must_use]
    pub fn raw_domain(&self) -> &str {
        &self.raw_domain
    }

    /// Required raw-artifact media type.
    #[must_use]
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Required representation identity.
    #[must_use]
    pub fn representation(&self) -> &str {
        &self.representation
    }

    /// Exact required raw-artifact digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }
}

/// Closed media type for repository artifacts selected from saved bytes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RawArtifactMediaType {
    /// Markdown source selected through `markdown/v1`.
    Markdown,
    /// Rust source selected through `rust/v1`.
    Rust,
    /// TOML source selected through `binding/v1`.
    Toml,
    /// Exact opaque bytes selected through `artifact/v1`.
    OpaqueBytes,
}

impl RawArtifactMediaType {
    /// Frozen media-type spelling used in canonical artifact rows.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "text/markdown",
            Self::Rust => "text/x-rust",
            Self::Toml => "application/toml",
            Self::OpaqueBytes => "application/octet-stream",
        }
    }
}

/// One explicit current selected repository artifact.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ArtifactEntry {
    path: RepositoryPath,
    raw_domain: String,
    media_type: String,
    representation: String,
    raw_digest: SemanticDigest,
}

impl ArtifactEntry {
    /// Constructs one saved-bytes artifact after closed adapter selection.
    #[must_use]
    pub fn saved_bytes(
        path: RepositoryPath,
        media_type: RawArtifactMediaType,
        raw_digest: SemanticDigest,
    ) -> Self {
        Self {
            path,
            raw_domain: String::from("raw-artifact"),
            media_type: String::from(media_type.as_str()),
            representation: String::from("saved-bytes/v1"),
            raw_digest,
        }
    }

    /// Constructs one exact current snapshot/artifact-index row.
    ///
    /// # Errors
    ///
    /// Rejects an invalid domain, empty media type, or empty representation.
    pub fn new(
        path: RepositoryPath,
        raw_domain: impl Into<String>,
        media_type: impl Into<String>,
        representation: impl Into<String>,
        raw_digest: SemanticDigest,
    ) -> Result<Self, ConstructionError> {
        let (raw_domain, media_type, representation) =
            parse_artifact_shape(raw_domain.into(), media_type.into(), representation.into())?;
        Ok(Self {
            path,
            raw_domain,
            media_type,
            representation,
            raw_digest,
        })
    }

    /// Canonical repository-relative path.
    #[must_use]
    pub const fn path(&self) -> &RepositoryPath {
        &self.path
    }

    /// Required semantic digest domain.
    #[must_use]
    pub fn raw_domain(&self) -> &str {
        &self.raw_domain
    }

    /// Declared raw-artifact media type.
    #[must_use]
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Declared representation identity.
    #[must_use]
    pub fn representation(&self) -> &str {
        &self.representation
    }

    /// Current raw-artifact digest.
    #[must_use]
    pub const fn raw_digest(&self) -> SemanticDigest {
        self.raw_digest
    }

    fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, CanonicalValue::Text(self.path.as_str().to_string())),
            (2, CanonicalValue::Text(self.raw_domain.clone())),
            (3, CanonicalValue::Text(self.media_type.clone())),
            (4, CanonicalValue::Text(self.representation.clone())),
            (5, CanonicalValue::Digest(self.raw_digest)),
        ])
    }
}

/// Canonically ordered explicit current repository artifact index.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ArtifactIndex {
    entries: Vec<ArtifactEntry>,
    by_digest: BTreeMap<SemanticDigest, Vec<usize>>,
}

impl ArtifactIndex {
    /// Validates and canonically orders an artifact index.
    ///
    /// # Errors
    ///
    /// Rejects duplicate paths and portable path collisions before retaining entries.
    pub fn new(mut entries: Vec<ArtifactEntry>) -> Result<Self, ArtifactIndexError> {
        entries.sort_unstable_by(|left, right| left.path.cmp(&right.path));
        let paths = entries.iter().map(|entry| entry.path.clone()).collect();
        crate::value::canonical_paths(paths).map_err(|error| match error {
            crate::value::PathSetError::CanonicalCollision => ArtifactIndexError::DuplicatePath,
            crate::value::PathSetError::PortableCollision => ArtifactIndexError::PortableCollision,
        })?;
        let mut by_digest = BTreeMap::<SemanticDigest, Vec<usize>>::new();
        for (index, entry) in entries.iter().enumerate() {
            by_digest.entry(entry.raw_digest).or_default().push(index);
        }
        Ok(Self { entries, by_digest })
    }

    /// Canonically ordered current entries.
    #[must_use]
    pub fn entries(&self) -> &[ArtifactEntry] {
        &self.entries
    }

    /// Computes the complete frozen-v2 artifact-index identity.
    ///
    /// # Errors
    ///
    /// Returns an encoding error only if retained entry values violate the
    /// canonical set contract.
    pub fn digest(&self) -> Result<SemanticDigest, EncodingError> {
        let values = self
            .entries
            .iter()
            .map(ArtifactEntry::canonical_value)
            .collect();
        // Construction proved unique paths, and every canonical entry includes its path.
        let values = canonicalize_validated_set(values, Clone::clone);
        Ok(semantic_digest_trusted(
            "artifact-index",
            alloc::vec![(1, CanonicalValue::Set(values))],
        ))
    }

    /// Returns canonical paths for all selected artifacts sharing one raw digest.
    #[must_use]
    pub fn paths_for_digest(&self, digest: SemanticDigest) -> Vec<&RepositoryPath> {
        self.by_digest
            .get(&digest)
            .into_iter()
            .flatten()
            .filter_map(|index| self.entries.get(*index))
            .map(ArtifactEntry::path)
            .collect()
    }

    /// Returns whether one repository binding exactly matches the current index.
    #[must_use]
    pub fn contains_binding(&self, binding: &RepositoryArtifactBinding) -> bool {
        self.entries
            .binary_search_by(|entry| entry.path.cmp(binding.path()))
            .ok()
            .and_then(|index| self.entries.get(index))
            .is_some_and(|entry| {
                entry.media_type == binding.media_type()
                    && entry.representation == binding.representation()
                    && entry.raw_digest == binding.digest()
            })
    }
}

/// Explicit artifact-index construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactIndexError {
    /// Two entries use the same canonical path.
    DuplicatePath,
    /// Two entries share the same full portable case-fold key.
    PortableCollision,
}

impl ArtifactIndexError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DuplicatePath => "SCF-ARTIFACT-DUPLICATE-PATH",
            Self::PortableCollision => "SCF-ARTIFACT-PORTABLE-COLLISION",
        }
    }
}

/// Intentional generated or mirrored relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Derivation {
    meta: ObjectMeta,
    sources: Vec<ArtifactBinding>,
    targets: Vec<ArtifactBinding>,
    tool: ToolIdentity,
    configuration: SemanticDigest,
    oracle: ObjectRef,
    relation: DerivationRelation,
    relation_scope: TypedPayload,
    exclusions: Vec<String>,
}

impl Derivation {
    /// Constructs one complete bound derivation.
    ///
    /// # Errors
    ///
    /// Rejects empty/duplicate source or target sets, overlapping bindings,
    /// duplicate exclusions, or noncanonical retained text.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: StableId,
        sources: Vec<ArtifactBinding>,
        targets: Vec<ArtifactBinding>,
        tool: ToolIdentity,
        configuration: SemanticDigest,
        oracle: ObjectRef,
        relation: DerivationRelation,
        relation_scope: TypedPayload,
        exclusions: Vec<String>,
    ) -> Result<Self, ConstructionError> {
        if (sources.is_empty(), targets.is_empty()) != (false, false) {
            return Err(ConstructionError::EmptySet);
        }
        if oracle.kind() != ObjectKind::Oracle {
            return Err(ConstructionError::ReferenceKind);
        }
        if relation == DerivationRelation::ByteEqual && (sources.len() != 1 || targets.len() != 1) {
            return Err(ConstructionError::LocalInvariant);
        }
        let sources = canonicalize_set(sources, ArtifactBinding::canonical_value)?;
        let targets = canonicalize_set(targets, ArtifactBinding::canonical_value)?;
        if sources
            .iter()
            .any(|source| targets.iter().any(|target| source == target))
        {
            return Err(ConstructionError::OverlappingSets);
        }
        let exclusions = canonicalize_nonempty_text_set(exclusions)?;
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (
                2,
                CanonicalValue::Set(
                    sources
                        .iter()
                        .map(ArtifactBinding::canonical_value)
                        .collect(),
                ),
            ),
            (
                3,
                CanonicalValue::Set(
                    targets
                        .iter()
                        .map(ArtifactBinding::canonical_value)
                        .collect(),
                ),
            ),
            (4, tool.canonical_value()),
            (5, CanonicalValue::Digest(configuration)),
            (6, object_reference_value(&oracle)),
            (7, CanonicalValue::Enum(relation as u16, None)),
            (8, relation_scope.canonical_value()),
            (
                9,
                CanonicalValue::Set(
                    exclusions
                        .iter()
                        .cloned()
                        .map(CanonicalValue::Text)
                        .collect(),
                ),
            ),
        ];
        let mut references = BTreeSet::new();
        references.insert(oracle.clone());
        references.extend(
            sources
                .iter()
                .chain(&targets)
                .filter_map(ArtifactBinding::object_reference)
                .cloned(),
        );
        let references =
            canonicalize_validated_set(references.into_iter().collect(), object_reference_value);
        let meta =
            ObjectMeta::for_fields(ObjectKind::Derivation, id, "derivation", fields, references);
        Ok(Self {
            meta,
            sources,
            targets,
            tool,
            configuration,
            oracle,
            relation,
            relation_scope,
            exclusions,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Source bindings in canonical set order.
    #[must_use]
    pub fn sources(&self) -> &[ArtifactBinding] {
        &self.sources
    }

    /// Target bindings in canonical set order.
    #[must_use]
    pub fn targets(&self) -> &[ArtifactBinding] {
        &self.targets
    }

    /// Generator or checker identity.
    #[must_use]
    pub const fn tool(&self) -> &ToolIdentity {
        &self.tool
    }

    /// Exact derivation-configuration digest.
    #[must_use]
    pub const fn configuration(&self) -> SemanticDigest {
        self.configuration
    }

    /// Consistency oracle.
    #[must_use]
    pub const fn oracle(&self) -> &ObjectRef {
        &self.oracle
    }

    /// Closed declared relation.
    #[must_use]
    pub const fn relation(&self) -> DerivationRelation {
        self.relation
    }

    /// Exact relation scope.
    #[must_use]
    pub const fn relation_scope(&self) -> &TypedPayload {
        &self.relation_scope
    }

    /// Explicit exclusions in canonical set order.
    #[must_use]
    pub fn exclusions(&self) -> &[String] {
        &self.exclusions
    }
}

/// Complete semantic revision of one finite coverage policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoveragePolicyRevision {
    digest: SemanticDigest,
    scope: TypedPayload,
    required_roles: Vec<EvidenceRole>,
    enforcement: EnforcementMode,
    ratchet_id: ScopeId,
    authority_selectors: Vec<TypedSelector>,
    derivation_requirements: Option<TypedPayload>,
}

impl CoveragePolicyRevision {
    /// Constructs one complete digest-bound policy revision.
    ///
    /// # Errors
    ///
    /// Rejects an empty ratchet ID, duplicate sets, or noncanonical fields.
    pub fn new(
        scope: TypedPayload,
        required_roles: Vec<EvidenceRole>,
        enforcement: EnforcementMode,
        ratchet_id: ScopeId,
        authority_selectors: Vec<TypedSelector>,
        derivation_requirements: Option<TypedPayload>,
    ) -> Result<Self, ConstructionError> {
        let required_roles = canonicalize_set(required_roles, |role| {
            CanonicalValue::Enum(*role as u16, None)
        })?;
        let authority_selectors = canonicalize_set(authority_selectors, typed_selector_value)?;
        let fields = alloc::vec![
            (1, typed_payload_value(&scope)),
            (
                2,
                CanonicalValue::Set(
                    required_roles
                        .iter()
                        .map(|role| CanonicalValue::Enum(*role as u16, None))
                        .collect(),
                ),
            ),
            (3, CanonicalValue::Enum(enforcement as u16, None)),
            (4, CanonicalValue::Text(ratchet_id.as_str().to_string())),
            (
                5,
                CanonicalValue::Set(
                    authority_selectors
                        .iter()
                        .map(typed_selector_value)
                        .collect(),
                ),
            ),
            (
                6,
                derivation_requirements
                    .as_ref()
                    .map_or(CanonicalValue::None, |requirements| {
                        CanonicalValue::Some(Box::new(typed_payload_value(requirements)))
                    },),
            ),
        ];
        let digest = semantic_digest_trusted("coverage-policy-revision", fields);
        Ok(Self {
            digest,
            scope,
            required_roles,
            enforcement,
            ratchet_id,
            authority_selectors,
            derivation_requirements,
        })
    }

    /// Exact policy revision digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Finite policy scope predicate.
    #[must_use]
    pub const fn scope(&self) -> &TypedPayload {
        &self.scope
    }

    /// Required evidence roles in canonical set order.
    #[must_use]
    pub fn required_roles(&self) -> &[EvidenceRole] {
        &self.required_roles
    }

    /// Advisory or blocking enforcement.
    #[must_use]
    pub const fn enforcement(&self) -> EnforcementMode {
        self.enforcement
    }

    /// Stable adoption-ratchet identity.
    #[must_use]
    pub fn ratchet_id(&self) -> &str {
        self.ratchet_id.as_str()
    }

    pub(crate) const fn ratchet_scope_id(&self) -> &ScopeId {
        &self.ratchet_id
    }

    /// Expected authority selectors in canonical set order.
    #[must_use]
    pub fn authority_selectors(&self) -> &[TypedSelector] {
        &self.authority_selectors
    }

    /// Optional pinned derivation requirements.
    #[must_use]
    pub const fn derivation_requirements(&self) -> Option<&TypedPayload> {
        self.derivation_requirements.as_ref()
    }
}

/// Versioned finite coverage-policy object and irreversible history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoveragePolicy {
    meta: ObjectMeta,
    revision: CoveragePolicyRevision,
    active_approval: ApprovalBinding,
    lifecycle: PolicyLifecycle,
    replacements: Vec<ObjectRef>,
    terminal_approval: Option<ApprovalBinding>,
    materialized_subjects: Vec<ObjectRef>,
}

impl CoveragePolicy {
    /// Constructs one complete policy object.
    ///
    /// `materialized_subjects` is temporary derived scope materialization used
    /// by the current pure algorithms; it is not authoritative policy input.
    ///
    /// # Errors
    ///
    /// Rejects invalid active/terminal lifecycle shapes, wrong replacement
    /// kinds, duplicate sets, or noncanonical fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: StableId,
        revision: CoveragePolicyRevision,
        active_approval: ApprovalBinding,
        lifecycle: PolicyLifecycle,
        replacements: Vec<ObjectRef>,
        terminal_approval: Option<ApprovalBinding>,
        materialized_subjects: Vec<ObjectRef>,
    ) -> Result<Self, ConstructionError> {
        let replacements = canonicalize_set(replacements, object_reference_value)?;
        let materialized_subjects =
            canonicalize_set(materialized_subjects, object_reference_value)?;
        if replacements
            .iter()
            .any(|reference| reference.kind() != ObjectKind::CoveragePolicy)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        if replacements.iter().any(|reference| *reference.id() == id) {
            return Err(ConstructionError::LocalInvariant);
        }
        let valid_shape = matches!(
            (
                lifecycle,
                replacements.is_empty(),
                terminal_approval.is_some()
            ),
            (PolicyLifecycle::Active, true, false)
                | (PolicyLifecycle::Superseded, false, true)
                | (PolicyLifecycle::Retired, true, true)
        );
        if !valid_shape {
            return Err(ConstructionError::LifecycleShape);
        }
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (2, CanonicalValue::Digest(revision.digest())),
            (3, approval_binding_value(&active_approval)),
            (4, CanonicalValue::Enum(lifecycle as u16, None)),
            (
                5,
                CanonicalValue::Set(replacements.iter().map(object_reference_value).collect(),),
            ),
            (6, approval_value(terminal_approval.as_ref())),
        ];
        let references = canonicalize_validated_set(
            replacements
                .iter()
                .chain(materialized_subjects.iter())
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            object_reference_value,
        );
        let meta = ObjectMeta::for_fields(
            ObjectKind::CoveragePolicy,
            id,
            "coverage-policy-object",
            fields,
            references,
        );
        Ok(Self {
            meta,
            revision,
            active_approval,
            lifecycle,
            replacements,
            terminal_approval,
            materialized_subjects,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Exact policy semantic revision digest.
    #[must_use]
    pub const fn revision(&self) -> SemanticDigest {
        self.revision.digest()
    }

    /// Complete policy revision value.
    #[must_use]
    pub const fn revision_value(&self) -> &CoveragePolicyRevision {
        &self.revision
    }

    /// Active repository-owned policy approval.
    #[must_use]
    pub const fn active_approval(&self) -> &ApprovalBinding {
        &self.active_approval
    }

    /// Irreversible policy lifecycle.
    #[must_use]
    pub const fn lifecycle(&self) -> PolicyLifecycle {
        self.lifecycle
    }

    /// Replacement policy references in canonical order.
    #[must_use]
    pub fn replacements(&self) -> &[ObjectRef] {
        &self.replacements
    }

    /// Terminal approval for superseded or retired policy history.
    #[must_use]
    pub const fn terminal_approval(&self) -> Option<&ApprovalBinding> {
        self.terminal_approval.as_ref()
    }

    /// Enforcement mode retained by the revision.
    #[must_use]
    pub const fn mode(&self) -> EnforcementMode {
        self.revision.enforcement()
    }

    /// Adoption-ratchet identity retained by the revision.
    #[must_use]
    pub fn ratchet(&self) -> &str {
        self.revision.ratchet_id()
    }

    /// Temporary derived finite scope materialization.
    #[must_use]
    pub fn subjects(&self) -> &[ObjectRef] {
        &self.materialized_subjects
    }

    /// Required roles retained by the revision.
    #[must_use]
    pub fn roles(&self) -> &[EvidenceRole] {
        self.revision.required_roles()
    }
}

/// Closed policy-obligation kind.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum ObligationKind {
    /// Ratified authority exists.
    Authority = 1,
    /// Finite surface is complete.
    SurfaceCompleteness = 2,
    /// Finite surface rows have authority.
    SurfaceAuthority = 3,
    /// One explicit evidence role is required.
    RequiredRole = 4,
    /// Exact semantic bindings are fresh.
    Freshness = 5,
    /// Required oracle was selected.
    OracleSelection = 6,
    /// Required execution verdict passed.
    Execution = 7,
    /// Declared derivation relation is current and consistent.
    DerivationConsistency = 8,
    /// Complete current proof binding is accepted.
    Proof = 9,
    /// Base/head impact has exact disposition closure.
    ChangeImpact = 10,
}

/// Exact policy obligation reference.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObligationRef {
    digest: SemanticDigest,
    policy_id: StableId,
    kind: ObligationKind,
    subject: ObjectRef,
    required_role: Option<EvidenceRole>,
}

impl ObligationRef {
    /// Constructs one exact closed policy obligation.
    ///
    /// # Errors
    ///
    /// Rejects a required-role payload on another kind, its absence on the
    /// required-role kind, or noncanonical fields.
    pub fn new(
        policy_id: StableId,
        kind: ObligationKind,
        subject: ObjectRef,
        required_role: Option<EvidenceRole>,
    ) -> Result<Self, ConstructionError> {
        if (kind == ObligationKind::RequiredRole) != required_role.is_some() {
            return Err(ConstructionError::ObligationRoleShape);
        }
        let fields = alloc::vec![
            (1, text_value(policy_id.as_str())),
            (2, CanonicalValue::Enum(kind as u16, None)),
            (3, object_reference_value(&subject)),
            (
                4,
                required_role.map_or(CanonicalValue::None, |role| {
                    CanonicalValue::Some(Box::new(CanonicalValue::Enum(role as u16, None)))
                }),
            ),
        ];
        let digest = semantic_digest_trusted("obligation-reference", fields);
        Ok(Self {
            digest,
            policy_id,
            kind,
            subject,
            required_role,
        })
    }

    pub(crate) fn for_policy(
        policy: &CoveragePolicy,
        kind: ObligationKind,
        subject: ObjectRef,
        required_role: Option<EvidenceRole>,
    ) -> Self {
        let policy_id = policy.meta().id().clone();
        let fields = alloc::vec![
            (1, text_value(policy_id.as_str())),
            (2, CanonicalValue::Enum(kind as u16, None)),
            (3, object_reference_value(&subject)),
            (
                4,
                required_role.map_or(CanonicalValue::None, |role| {
                    CanonicalValue::Some(Box::new(CanonicalValue::Enum(role as u16, None)))
                }),
            ),
        ];
        let digest = semantic_digest_trusted("obligation-reference", fields);
        Self {
            digest,
            policy_id,
            kind,
            subject,
            required_role,
        }
    }

    /// Exact obligation semantic digest.
    #[must_use]
    pub const fn digest(&self) -> SemanticDigest {
        self.digest
    }

    /// Stable policy ID.
    #[must_use]
    pub const fn policy_id(&self) -> &StableId {
        &self.policy_id
    }

    /// Closed obligation kind.
    #[must_use]
    pub const fn kind(&self) -> ObligationKind {
        self.kind
    }

    /// Exact obligation subject.
    #[must_use]
    pub const fn subject(&self) -> &ObjectRef {
        &self.subject
    }

    /// Required evidence role, only for [`ObligationKind::RequiredRole`].
    #[must_use]
    pub const fn required_role(&self) -> Option<EvidenceRole> {
        self.required_role
    }

    pub(crate) fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (1, text_value(self.policy_id.as_str())),
            (2, CanonicalValue::Enum(self.kind as u16, None)),
            (3, object_reference_value(&self.subject)),
            (
                4,
                self.required_role.map_or(CanonicalValue::None, |role| {
                    CanonicalValue::Some(Box::new(CanonicalValue::Enum(role as u16, None)))
                }),
            ),
        ])
    }
}

/// One exact derived policy-ratchet impact.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PolicyRatchetImpact {
    ratchet_id: ScopeId,
    base_policy: ObjectRef,
    head_policy: ObjectRef,
    weakened_obligations: Vec<ObligationRef>,
}

impl PolicyRatchetImpact {
    /// Constructs one exact base-to-head policy weakening observation.
    ///
    /// # Errors
    ///
    /// Rejects an invalid ratchet ID, non-policy references, an empty weakened
    /// obligation set, or duplicate obligations.
    pub fn new(
        ratchet_id: ScopeId,
        base_policy: ObjectRef,
        head_policy: ObjectRef,
        weakened_obligations: Vec<ObligationRef>,
    ) -> Result<Self, ConstructionError> {
        if (base_policy.kind(), head_policy.kind())
            != (ObjectKind::CoveragePolicy, ObjectKind::CoveragePolicy)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        let weakened_obligations =
            canonicalize_set(weakened_obligations, ObligationRef::canonical_value)?;
        if weakened_obligations.is_empty() {
            return Err(ConstructionError::LocalInvariant);
        }
        Ok(Self::from_validated(
            ratchet_id,
            base_policy,
            head_policy,
            weakened_obligations,
        ))
    }

    pub(crate) fn from_validated(
        ratchet_id: ScopeId,
        base_policy: ObjectRef,
        head_policy: ObjectRef,
        weakened_obligations: Vec<ObligationRef>,
    ) -> Self {
        Self {
            ratchet_id,
            base_policy,
            head_policy,
            weakened_obligations,
        }
    }

    /// Stable ratchet lineage ID.
    #[must_use]
    pub fn ratchet_id(&self) -> &str {
        self.ratchet_id.as_str()
    }

    /// Exact base policy binding.
    #[must_use]
    pub const fn base_policy(&self) -> &ObjectRef {
        &self.base_policy
    }

    /// Exact head policy binding.
    #[must_use]
    pub const fn head_policy(&self) -> &ObjectRef {
        &self.head_policy
    }

    /// Canonically ordered obligations weakened by the change.
    #[must_use]
    pub fn weakened_obligations(&self) -> &[ObligationRef] {
        &self.weakened_obligations
    }

    pub(crate) fn canonical_value(&self) -> CanonicalValue {
        CanonicalValue::Record(alloc::vec![
            (
                1,
                CanonicalValue::Text(self.ratchet_id.as_str().to_string())
            ),
            (2, object_reference_value(&self.base_policy)),
            (3, object_reference_value(&self.head_policy)),
            (
                4,
                CanonicalValue::Set(
                    self.weakened_obligations
                        .iter()
                        .map(ObligationRef::canonical_value)
                        .collect(),
                ),
            ),
        ])
    }
}

pub(crate) fn obligation_reference_value(obligation: &ObligationRef) -> CanonicalValue {
    obligation.canonical_value()
}

/// Deterministic semantic-value construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructionError {
    /// Canonical encoding rejected retained text, tags, or duplicate set values.
    Encoding(EncodingError),
    /// A schema-required set was empty.
    EmptySet,
    /// A positive generation was zero.
    NonPositiveGeneration,
    /// A source extent ended before it started.
    SourceSpan,
    /// Required-role option does not match the obligation kind.
    ObligationRoleShape,
    /// Artifact domain, media type, or representation shape is invalid.
    ArtifactBindingShape,
    /// Two schema-declared disjoint sets overlap.
    OverlappingSets,
    /// A `not-run` verdict incorrectly retained a semantic result.
    VerdictResultShape,
    /// A schema-required text field was empty.
    EmptyText,
    /// A stable object ID violated the frozen uppercase grammar.
    StableIdGrammar,
    /// An authority, owner, namespace, tool, or ratchet ID violated the frozen grammar.
    ScopeIdGrammar,
    /// A typed schema or adapter ID violated the frozen versioned grammar.
    SchemaIdGrammar,
    /// A source item kind violated the versioned lower-case grammar.
    SourceItemKindGrammar,
    /// An external trace target was unsafe or noncanonical.
    ExternalTraceTargetGrammar,
    /// A normative atom statement omitted an uppercase RFC 2119 term.
    NormativeStatement,
    /// A role and its declared scope schema are incompatible.
    RoleScopeShape,
    /// A local kind-specific option or set shape is invalid.
    LocalInvariant,
    /// An irreversible lifecycle retained the wrong terminal evidence shape.
    LifecycleShape,
    /// An object reference used a kind forbidden by its field.
    ReferenceKind,
    /// Two digests required to identify a change used different algorithms.
    DigestAlgorithmMismatch,
    /// Two digests required to identify a change were equal.
    EqualDigests,
    /// A change disposition retained the wrong variant-dependent fields.
    ChangeDispositionShape,
}

impl From<EncodingError> for ConstructionError {
    fn from(error: EncodingError) -> Self {
        Self::Encoding(error)
    }
}

impl From<IdentifierError> for ConstructionError {
    fn from(error: IdentifierError) -> Self {
        match error {
            IdentifierError::StableIdGrammar => Self::StableIdGrammar,
            IdentifierError::ScopeIdGrammar => Self::ScopeIdGrammar,
            IdentifierError::SchemaIdGrammar => Self::SchemaIdGrammar,
            IdentifierError::SourceItemKindGrammar => Self::SourceItemKindGrammar,
            IdentifierError::ExternalTraceTargetGrammar => Self::ExternalTraceTargetGrammar,
        }
    }
}

impl ConstructionError {
    /// Stable machine diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Encoding(error) => error.code(),
            Self::EmptySet | Self::EmptyText => "SCF-CONSTRUCT-FIELD-EMPTY",
            Self::NonPositiveGeneration => "SCF-CONSTRUCT-GENERATION",
            Self::SourceSpan => "SCF-SOURCE-MAP-SPAN",
            Self::ObligationRoleShape => "SCF-CONSTRUCT-OBLIGATION-ROLE",
            Self::ArtifactBindingShape => "SCF-CONSTRUCT-ARTIFACT-BINDING",
            Self::OverlappingSets => "SCF-CONSTRUCT-SET-OVERLAP",
            Self::VerdictResultShape => "SCF-CONSTRUCT-VERDICT-RESULT",
            Self::StableIdGrammar => "SCF-CONSTRUCT-STABLE-ID",
            Self::ScopeIdGrammar => "SCF-CONSTRUCT-SCOPE-ID",
            Self::SchemaIdGrammar => "SCF-CONSTRUCT-SCHEMA-ID",
            Self::SourceItemKindGrammar => "SPG-SOURCE-ITEM-KIND",
            Self::ExternalTraceTargetGrammar => "SPG-EXTERNAL-TRACE-TARGET",
            Self::NormativeStatement => "SCF-CONSTRUCT-NORMATIVE-STATEMENT",
            Self::RoleScopeShape => "SCF-CONSTRUCT-ROLE-SCOPE",
            Self::LocalInvariant => "SCF-CONSTRUCT-LOCAL-INVARIANT",
            Self::LifecycleShape => "SCF-LIFECYCLE-SHAPE",
            Self::ReferenceKind => "SCF-CONSTRUCT-REFERENCE-KIND",
            Self::DigestAlgorithmMismatch => "DIG-ALGORITHM-MISMATCH",
            Self::EqualDigests => "SCF-CONSTRUCT-DIGEST-EQUALITY",
            Self::ChangeDispositionShape => "SCF-CONSTRUCT-CHANGE-DISPOSITION",
        }
    }
}

const fn debt_lifecycle_shape_valid(
    lifecycle: DebtLifecycle,
    has_discharge_evidence: bool,
    has_replacements: bool,
    has_terminal_approval: bool,
) -> bool {
    matches!(
        (
            lifecycle,
            has_discharge_evidence,
            has_replacements,
            has_terminal_approval,
        ),
        (DebtLifecycle::Active, false, false, false)
            | (DebtLifecycle::Discharged, true, false, true)
            | (DebtLifecycle::Superseded, false, true, true)
    )
}

const fn waiver_lifecycle_shape_valid(
    lifecycle: WaiverLifecycle,
    has_discharge_evidence: bool,
    has_replacements: bool,
    has_terminal_approval: bool,
) -> bool {
    matches!(
        (
            lifecycle,
            has_discharge_evidence,
            has_replacements,
            has_terminal_approval,
        ),
        (WaiverLifecycle::Active, false, false, false)
            | (WaiverLifecycle::Discharged, true, false, true)
            | (WaiverLifecycle::Revoked, false, false, true)
            | (WaiverLifecycle::Superseded, false, true, true)
    )
}

/// Visible debt that never satisfies an obligation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebtRecord {
    meta: ObjectMeta,
    obligations: Vec<ObligationRef>,
    objects: Vec<ObjectRef>,
    rationale: String,
    owner: ScopeId,
    remediation: String,
    discharge_condition: TypedPayload,
    external_references: Vec<String>,
    lifecycle: DebtLifecycle,
    discharge_evidence: Vec<ObjectRef>,
    replacements: Vec<ObjectRef>,
    terminal_approval: Option<ApprovalBinding>,
}

impl DebtRecord {
    /// Constructs one complete irreversible debt history record.
    ///
    /// # Errors
    ///
    /// Rejects empty obligations/text, duplicates, wrong replacement kinds,
    /// noncanonical fields, or an invalid lifecycle evidence shape.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn new(
        id: StableId,
        obligations: Vec<ObligationRef>,
        objects: Vec<ObjectRef>,
        rationale: impl Into<String>,
        owner: ScopeId,
        remediation: impl Into<String>,
        discharge_condition: TypedPayload,
        external_references: Vec<String>,
        lifecycle: DebtLifecycle,
        discharge_evidence: Vec<ObjectRef>,
        replacements: Vec<ObjectRef>,
        terminal_approval: Option<ApprovalBinding>,
    ) -> Result<Self, ConstructionError> {
        let rationale = rationale.into();
        let remediation = remediation.into();
        require_nonempty_nfc(&rationale)?;
        require_nonempty_nfc(&remediation)?;
        if obligations.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let obligations = canonicalize_set(obligations, obligation_reference_value)?;
        let objects = canonicalize_set(objects, object_reference_value)?;
        let external_references = canonicalize_nonempty_text_set(external_references)?;
        let discharge_evidence = canonicalize_set(discharge_evidence, object_reference_value)?;
        let replacements = canonicalize_set(replacements, object_reference_value)?;
        if replacements
            .iter()
            .any(|reference| reference.kind() != ObjectKind::DebtRecord)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        if replacements.iter().any(|reference| *reference.id() == id) {
            return Err(ConstructionError::LocalInvariant);
        }
        if !debt_lifecycle_shape_valid(
            lifecycle,
            !discharge_evidence.is_empty(),
            !replacements.is_empty(),
            terminal_approval.is_some(),
        ) {
            return Err(ConstructionError::LifecycleShape);
        }
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (
                2,
                CanonicalValue::Set(obligations.iter().map(obligation_reference_value).collect(),),
            ),
            (
                3,
                CanonicalValue::Set(objects.iter().map(object_reference_value).collect()),
            ),
            (4, CanonicalValue::Text(rationale.clone())),
            (5, CanonicalValue::Text(owner.as_str().to_string())),
            (6, CanonicalValue::Text(remediation.clone())),
            (7, typed_payload_value(&discharge_condition)),
            (
                8,
                CanonicalValue::Set(
                    external_references
                        .iter()
                        .cloned()
                        .map(CanonicalValue::Text)
                        .collect(),
                ),
            ),
            (9, CanonicalValue::Enum(lifecycle as u16, None)),
            (
                10,
                CanonicalValue::Set(
                    discharge_evidence
                        .iter()
                        .map(object_reference_value)
                        .collect(),
                ),
            ),
            (
                11,
                CanonicalValue::Set(replacements.iter().map(object_reference_value).collect(),),
            ),
            (12, approval_value(terminal_approval.as_ref())),
        ];
        let references = canonicalize_validated_set(
            obligations
                .iter()
                .map(|obligation| obligation.subject().clone())
                .chain(objects.iter().cloned())
                .chain(discharge_evidence.iter().cloned())
                .chain(replacements.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            object_reference_value,
        );
        let meta = ObjectMeta::for_fields(
            ObjectKind::DebtRecord,
            id,
            "debt-record",
            fields,
            references,
        );
        Ok(Self {
            meta,
            obligations,
            objects,
            rationale,
            owner,
            remediation,
            discharge_condition,
            external_references,
            lifecycle,
            discharge_evidence,
            replacements,
            terminal_approval,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Bound obligations in canonical set order.
    #[must_use]
    pub fn obligations(&self) -> &[ObligationRef] {
        &self.obligations
    }

    /// Affected objects in canonical set order.
    #[must_use]
    pub fn objects(&self) -> &[ObjectRef] {
        &self.objects
    }

    /// Nonempty debt rationale.
    #[must_use]
    pub fn rationale(&self) -> &str {
        &self.rationale
    }

    /// Responsible owner ID.
    #[must_use]
    pub fn owner(&self) -> &str {
        self.owner.as_str()
    }

    /// Nonempty remediation statement.
    #[must_use]
    pub fn remediation(&self) -> &str {
        &self.remediation
    }

    /// Machine-readable discharge condition.
    #[must_use]
    pub const fn discharge_condition(&self) -> &TypedPayload {
        &self.discharge_condition
    }

    /// Informational external references in canonical order.
    #[must_use]
    pub fn external_references(&self) -> &[String] {
        &self.external_references
    }

    /// Debt lifecycle.
    #[must_use]
    pub const fn lifecycle(&self) -> DebtLifecycle {
        self.lifecycle
    }

    /// Terminal discharge evidence.
    #[must_use]
    pub fn discharge_evidence(&self) -> &[ObjectRef] {
        &self.discharge_evidence
    }

    /// Replacement debt records.
    #[must_use]
    pub fn replacements(&self) -> &[ObjectRef] {
        &self.replacements
    }

    /// Terminal approval when the debt is no longer active.
    #[must_use]
    pub const fn terminal_approval(&self) -> Option<&ApprovalBinding> {
        self.terminal_approval.as_ref()
    }
}

/// Process-only waiver that preserves unsatisfied facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Waiver {
    meta: ObjectMeta,
    obligations: Vec<ObligationRef>,
    objects: Vec<ObjectRef>,
    rationale: String,
    owner: ScopeId,
    discharge_condition: TypedPayload,
    active_approval: ApprovalBinding,
    external_references: Vec<String>,
    lifecycle: WaiverLifecycle,
    discharge_evidence: Vec<ObjectRef>,
    replacements: Vec<ObjectRef>,
    terminal_approval: Option<ApprovalBinding>,
}

impl Waiver {
    /// Constructs one complete irreversible process-waiver history record.
    ///
    /// # Errors
    ///
    /// Rejects empty obligations/text, duplicates, wrong replacement kinds,
    /// noncanonical fields, or an invalid lifecycle evidence shape.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn new(
        id: StableId,
        obligations: Vec<ObligationRef>,
        objects: Vec<ObjectRef>,
        rationale: impl Into<String>,
        owner: ScopeId,
        discharge_condition: TypedPayload,
        active_approval: ApprovalBinding,
        external_references: Vec<String>,
        lifecycle: WaiverLifecycle,
        discharge_evidence: Vec<ObjectRef>,
        replacements: Vec<ObjectRef>,
        terminal_approval: Option<ApprovalBinding>,
    ) -> Result<Self, ConstructionError> {
        let rationale = rationale.into();
        require_nonempty_nfc(&rationale)?;
        if obligations.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let obligations = canonicalize_set(obligations, obligation_reference_value)?;
        let objects = canonicalize_set(objects, object_reference_value)?;
        let external_references = canonicalize_nonempty_text_set(external_references)?;
        let discharge_evidence = canonicalize_set(discharge_evidence, object_reference_value)?;
        let replacements = canonicalize_set(replacements, object_reference_value)?;
        if replacements
            .iter()
            .any(|reference| reference.kind() != ObjectKind::Waiver)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        if replacements.iter().any(|reference| *reference.id() == id) {
            return Err(ConstructionError::LocalInvariant);
        }
        if !waiver_lifecycle_shape_valid(
            lifecycle,
            !discharge_evidence.is_empty(),
            !replacements.is_empty(),
            terminal_approval.is_some(),
        ) {
            return Err(ConstructionError::LifecycleShape);
        }
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (
                2,
                CanonicalValue::Set(obligations.iter().map(obligation_reference_value).collect(),),
            ),
            (
                3,
                CanonicalValue::Set(objects.iter().map(object_reference_value).collect()),
            ),
            (4, CanonicalValue::Text(rationale.clone())),
            (5, CanonicalValue::Text(owner.as_str().to_string())),
            (6, typed_payload_value(&discharge_condition)),
            (7, approval_binding_value(&active_approval)),
            (
                8,
                CanonicalValue::Set(
                    external_references
                        .iter()
                        .cloned()
                        .map(CanonicalValue::Text)
                        .collect(),
                ),
            ),
            (9, CanonicalValue::Enum(lifecycle as u16, None)),
            (
                10,
                CanonicalValue::Set(
                    discharge_evidence
                        .iter()
                        .map(object_reference_value)
                        .collect(),
                ),
            ),
            (
                11,
                CanonicalValue::Set(replacements.iter().map(object_reference_value).collect(),),
            ),
            (12, approval_value(terminal_approval.as_ref())),
        ];
        let references = canonicalize_validated_set(
            obligations
                .iter()
                .map(|obligation| obligation.subject().clone())
                .chain(objects.iter().cloned())
                .chain(discharge_evidence.iter().cloned())
                .chain(replacements.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            object_reference_value,
        );
        let meta = ObjectMeta::for_fields(ObjectKind::Waiver, id, "waiver", fields, references);
        Ok(Self {
            meta,
            obligations,
            objects,
            rationale,
            owner,
            discharge_condition,
            active_approval,
            external_references,
            lifecycle,
            discharge_evidence,
            replacements,
            terminal_approval,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Bound obligations in canonical set order.
    #[must_use]
    pub fn obligations(&self) -> &[ObligationRef] {
        &self.obligations
    }

    /// Affected objects in canonical set order.
    #[must_use]
    pub fn objects(&self) -> &[ObjectRef] {
        &self.objects
    }

    /// Nonempty waiver rationale.
    #[must_use]
    pub fn rationale(&self) -> &str {
        &self.rationale
    }

    /// Responsible owner ID.
    #[must_use]
    pub fn owner(&self) -> &str {
        self.owner.as_str()
    }

    /// Machine-readable discharge condition.
    #[must_use]
    pub const fn discharge_condition(&self) -> &TypedPayload {
        &self.discharge_condition
    }

    /// Active process approval.
    #[must_use]
    pub const fn active_approval(&self) -> &ApprovalBinding {
        &self.active_approval
    }

    /// Informational external references in canonical order.
    #[must_use]
    pub fn external_references(&self) -> &[String] {
        &self.external_references
    }

    /// Waiver lifecycle.
    #[must_use]
    pub const fn lifecycle(&self) -> WaiverLifecycle {
        self.lifecycle
    }

    /// Terminal discharge evidence.
    #[must_use]
    pub fn discharge_evidence(&self) -> &[ObjectRef] {
        &self.discharge_evidence
    }

    /// Replacement waiver records.
    #[must_use]
    pub fn replacements(&self) -> &[ObjectRef] {
        &self.replacements
    }

    /// Terminal approval when the waiver is no longer active.
    #[must_use]
    pub const fn terminal_approval(&self) -> Option<&ApprovalBinding> {
        self.terminal_approval.as_ref()
    }
}

/// Human-approved formal property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormalProperty {
    meta: ObjectMeta,
    atoms: Vec<AtomRevisionRef>,
    property: TypedPayload,
    implementation_abstraction: TypedPayload,
    assumptions: Vec<ObjectRef>,
    exclusions: Vec<String>,
    correspondence_approval: ApprovalBinding,
}

impl FormalProperty {
    /// Constructs one complete approved formal-property registration.
    ///
    /// # Errors
    ///
    /// Rejects an empty/duplicate atom set, duplicate assumptions/exclusions,
    /// or noncanonical retained text.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: StableId,
        atoms: Vec<AtomRevisionRef>,
        property: TypedPayload,
        implementation_abstraction: TypedPayload,
        assumptions: Vec<ObjectRef>,
        exclusions: Vec<String>,
        correspondence_approval: ApprovalBinding,
    ) -> Result<Self, ConstructionError> {
        if atoms.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        let atoms = canonicalize_set(atoms, atom_revision_reference_value)?;
        let assumptions = canonicalize_set(assumptions, object_reference_value)?;
        if assumptions
            .iter()
            .any(|assumption| assumption.kind() != ObjectKind::Assumption)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        let exclusions = canonicalize_nonempty_text_set(exclusions)?;
        let id_value = text_value(id.as_str());
        let meta = ObjectMeta::for_fields(
            ObjectKind::FormalProperty,
            id,
            "formal-property",
            alloc::vec![
                (1, id_value),
                (
                    2,
                    CanonicalValue::Set(atoms.iter().map(atom_revision_reference_value).collect(),),
                ),
                (3, property.canonical_value()),
                (4, implementation_abstraction.canonical_value()),
                (
                    5,
                    CanonicalValue::Set(assumptions.iter().map(object_reference_value).collect(),),
                ),
                (
                    6,
                    CanonicalValue::Set(
                        exclusions
                            .iter()
                            .cloned()
                            .map(CanonicalValue::Text)
                            .collect(),
                    ),
                ),
                (7, approval_binding_value(&correspondence_approval)),
            ],
            assumptions.clone(),
        );
        Ok(Self {
            meta,
            atoms,
            property,
            implementation_abstraction,
            assumptions,
            exclusions,
            correspondence_approval,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Exact atom bindings.
    #[must_use]
    pub fn atoms(&self) -> &[AtomRevisionRef] {
        &self.atoms
    }

    /// Formal property payload.
    #[must_use]
    pub const fn property(&self) -> &TypedPayload {
        &self.property
    }

    /// Exact implementation-abstraction payload.
    #[must_use]
    pub const fn implementation_abstraction(&self) -> &TypedPayload {
        &self.implementation_abstraction
    }

    /// Explicit assumption references.
    #[must_use]
    pub fn assumptions(&self) -> &[ObjectRef] {
        &self.assumptions
    }

    /// Explicit exclusions in canonical set order.
    #[must_use]
    pub fn exclusions(&self) -> &[String] {
        &self.exclusions
    }

    /// Repository-owned prose-to-property correspondence approval.
    #[must_use]
    pub const fn correspondence_approval(&self) -> &ApprovalBinding {
        &self.correspondence_approval
    }

    /// Whether the schema-required correspondence approval is bound.
    #[must_use]
    pub const fn approved(&self) -> bool {
        true
    }
}

/// Exact proof tool, artifact, assumption, and verdict bindings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofRecord {
    meta: ObjectMeta,
    atoms: Vec<AtomRevisionRef>,
    property: ObjectRef,
    implementation: ArtifactBinding,
    assumptions: Vec<ObjectRef>,
    translators: Vec<ToolIdentity>,
    verifier: ToolIdentity,
    verifier_configuration: SemanticDigest,
    external_models: Vec<ArtifactBinding>,
    generated: Vec<ArtifactBinding>,
    proof_sources: Vec<ArtifactBinding>,
    verdict: Verdict,
    scope: TypedPayload,
    exclusions: Vec<String>,
}

impl ProofRecord {
    /// Constructs one complete proof-record value and computes its object digest.
    ///
    /// # Errors
    ///
    /// Rejects empty atom/proof-source sets, duplicate sets, or noncanonical text.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn new(
        id: StableId,
        atoms: Vec<AtomRevisionRef>,
        property: ObjectRef,
        implementation: ArtifactBinding,
        assumptions: Vec<ObjectRef>,
        translators: Vec<ToolIdentity>,
        verifier: ToolIdentity,
        verifier_configuration: SemanticDigest,
        external_models: Vec<ArtifactBinding>,
        generated: Vec<ArtifactBinding>,
        proof_sources: Vec<ArtifactBinding>,
        verdict: Verdict,
        scope: TypedPayload,
        exclusions: Vec<String>,
    ) -> Result<Self, ConstructionError> {
        if atoms.is_empty() || proof_sources.is_empty() {
            return Err(ConstructionError::EmptySet);
        }
        if (
            property.kind(),
            assumptions
                .iter()
                .all(|assumption| assumption.kind() == ObjectKind::Assumption),
            implementation
                .object_reference()
                .is_none_or(|reference| reference.kind() == ObjectKind::ImplementationLink),
        ) != (ObjectKind::FormalProperty, true, true)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        let atoms = canonicalize_set(atoms, atom_revision_reference_value)?;
        let assumptions = canonicalize_set(assumptions, object_reference_value)?;
        let translators = canonicalize_set(translators, ToolIdentity::canonical_value)?;
        let external_models = canonicalize_set(external_models, ArtifactBinding::canonical_value)?;
        let generated = canonicalize_set(generated, ArtifactBinding::canonical_value)?;
        let proof_sources = canonicalize_set(proof_sources, ArtifactBinding::canonical_value)?;
        let exclusions = canonicalize_nonempty_text_set(exclusions)?;
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (
                2,
                CanonicalValue::Set(atoms.iter().map(atom_revision_reference_value).collect(),),
            ),
            (3, object_reference_value(&property)),
            (4, implementation.canonical_value()),
            (
                5,
                CanonicalValue::Set(assumptions.iter().map(object_reference_value).collect(),),
            ),
            (
                6,
                CanonicalValue::Set(
                    translators
                        .iter()
                        .map(ToolIdentity::canonical_value)
                        .collect(),
                ),
            ),
            (7, verifier.canonical_value()),
            (8, CanonicalValue::Digest(verifier_configuration)),
            (
                9,
                CanonicalValue::Set(
                    external_models
                        .iter()
                        .map(ArtifactBinding::canonical_value)
                        .collect(),
                ),
            ),
            (
                10,
                CanonicalValue::Set(
                    generated
                        .iter()
                        .map(ArtifactBinding::canonical_value)
                        .collect(),
                ),
            ),
            (
                11,
                CanonicalValue::Set(
                    proof_sources
                        .iter()
                        .map(ArtifactBinding::canonical_value)
                        .collect(),
                ),
            ),
            (12, CanonicalValue::Enum(verdict as u16, None)),
            (13, scope.canonical_value()),
            (
                14,
                CanonicalValue::Set(
                    exclusions
                        .iter()
                        .cloned()
                        .map(CanonicalValue::Text)
                        .collect(),
                ),
            ),
        ];
        let mut references = BTreeSet::new();
        references.insert(property.clone());
        references.extend(assumptions.iter().cloned());
        references.extend(
            core::iter::once(&implementation)
                .chain(&external_models)
                .chain(&generated)
                .chain(&proof_sources)
                .filter_map(ArtifactBinding::object_reference)
                .cloned(),
        );
        let references =
            canonicalize_validated_set(references.into_iter().collect(), object_reference_value);
        let meta = ObjectMeta::for_fields(
            ObjectKind::ProofRecord,
            id,
            "proof-record",
            fields,
            references,
        );
        Ok(Self {
            meta,
            atoms,
            property,
            implementation,
            assumptions,
            translators,
            verifier,
            verifier_configuration,
            external_models,
            generated,
            proof_sources,
            verdict,
            scope,
            exclusions,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Exact atom revisions.
    #[must_use]
    pub fn atoms(&self) -> &[AtomRevisionRef] {
        &self.atoms
    }

    /// Formal-property reference.
    #[must_use]
    pub const fn property(&self) -> &ObjectRef {
        &self.property
    }

    /// Implementation representation.
    #[must_use]
    pub const fn implementation(&self) -> &ArtifactBinding {
        &self.implementation
    }

    /// Assumption references.
    #[must_use]
    pub fn assumptions(&self) -> &[ObjectRef] {
        &self.assumptions
    }

    /// Translator identities.
    #[must_use]
    pub fn translators(&self) -> &[ToolIdentity] {
        &self.translators
    }

    /// Verifier identity.
    #[must_use]
    pub const fn verifier(&self) -> &ToolIdentity {
        &self.verifier
    }

    /// Exact verifier-configuration digest.
    #[must_use]
    pub const fn verifier_configuration(&self) -> SemanticDigest {
        self.verifier_configuration
    }

    /// Bound external models.
    #[must_use]
    pub fn external_models(&self) -> &[ArtifactBinding] {
        &self.external_models
    }

    /// Bound generated artifacts.
    #[must_use]
    pub fn generated(&self) -> &[ArtifactBinding] {
        &self.generated
    }

    /// Bound proof sources.
    #[must_use]
    pub fn proof_sources(&self) -> &[ArtifactBinding] {
        &self.proof_sources
    }

    /// Normalized verifier verdict.
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// Exact claimed proof scope.
    #[must_use]
    pub const fn scope(&self) -> &TypedPayload {
        &self.scope
    }

    /// Explicit exclusions in canonical set order.
    #[must_use]
    pub fn exclusions(&self) -> &[String] {
        &self.exclusions
    }
}

/// One exact neutral impact disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeDisposition {
    meta: ObjectMeta,
    impacted_kind: ObjectKind,
    impacted_id: StableId,
    base: Option<SemanticDigest>,
    head: Option<SemanticDigest>,
    disposition: ChangeDispositionKind,
    reaffirmation_digest: Option<SemanticDigest>,
    waiver: Option<ObjectRef>,
    replacements: Vec<ObjectRef>,
}

impl ChangeDisposition {
    /// Constructs one complete variant-shaped impact disposition.
    ///
    /// # Errors
    ///
    /// Rejects duplicate/wrong-kind replacements, noncanonical fields, or a
    /// disposition-specific option shape violation.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn new(
        id: StableId,
        impacted_kind: ObjectKind,
        impacted_id: StableId,
        base: Option<SemanticDigest>,
        head: Option<SemanticDigest>,
        disposition: ChangeDispositionKind,
        reaffirmation_digest: Option<SemanticDigest>,
        waiver: Option<ObjectRef>,
        replacements: Vec<ObjectRef>,
    ) -> Result<Self, ConstructionError> {
        let replacements = canonicalize_set(replacements, object_reference_value)?;
        if (
            replacements
                .iter()
                .all(|reference| reference.kind() == impacted_kind),
            waiver
                .as_ref()
                .is_none_or(|reference| reference.kind() == ObjectKind::Waiver),
        ) != (true, true)
        {
            return Err(ConstructionError::ReferenceKind);
        }
        if replacements
            .iter()
            .any(|reference| *reference.id() == impacted_id)
        {
            return Err(ConstructionError::LocalInvariant);
        }
        if let (Some(base), Some(head)) = (base, head) {
            require_same_digest_algorithm(base, head)?;
        }
        let different_pair = base.zip(head).is_some_and(|(base, head)| base != head);
        let valid_shape = matches!(
            (
                disposition,
                different_pair,
                reaffirmation_digest.is_some(),
                waiver.is_some(),
                replacements.is_empty(),
                base.is_some(),
                head.is_some(),
            ),
            (
                ChangeDispositionKind::Reaffirmed,
                true,
                true,
                false,
                true,
                true,
                true
            ) | (
                ChangeDispositionKind::Revised,
                true,
                false,
                false,
                _,
                true,
                true
            ) | (
                ChangeDispositionKind::Retired,
                _,
                false,
                false,
                _,
                true,
                false
            ) | (ChangeDispositionKind::Waived, _, false, true, true, true, _)
                | (
                    ChangeDispositionKind::Waived,
                    _,
                    false,
                    true,
                    true,
                    false,
                    true
                )
        );
        if !valid_shape {
            return Err(ConstructionError::ChangeDispositionShape);
        }
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (2, CanonicalValue::Enum(impacted_kind as u16, None)),
            (3, text_value(impacted_id.as_str())),
            (4, digest_option_value(base)),
            (5, digest_option_value(head)),
            (6, CanonicalValue::Enum(disposition as u16, None)),
            (7, digest_option_value(reaffirmation_digest)),
            (
                8,
                waiver.as_ref().map_or(CanonicalValue::None, |reference| {
                    CanonicalValue::Some(Box::new(object_reference_value(reference)))
                }),
            ),
            (
                9,
                CanonicalValue::Set(replacements.iter().map(object_reference_value).collect(),),
            ),
        ];
        let mut references = waiver
            .iter()
            .chain(&replacements)
            .cloned()
            .collect::<BTreeSet<_>>();
        for object_digest in base.into_iter().chain(head) {
            references.insert(ObjectRef::new(
                impacted_kind,
                impacted_id.clone(),
                object_digest,
            ));
        }
        let references =
            canonicalize_validated_set(references.into_iter().collect(), object_reference_value);
        let meta = ObjectMeta::for_fields(
            ObjectKind::ChangeDisposition,
            id,
            "change-disposition",
            fields,
            references,
        );
        Ok(Self {
            meta,
            impacted_kind,
            impacted_id,
            base,
            head,
            disposition,
            reaffirmation_digest,
            waiver,
            replacements,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Impacted kind.
    #[must_use]
    pub const fn impacted_kind(&self) -> ObjectKind {
        self.impacted_kind
    }

    /// Impacted stable ID.
    #[must_use]
    pub const fn impacted_id(&self) -> &StableId {
        &self.impacted_id
    }

    /// Expected base digest.
    #[must_use]
    pub const fn base(&self) -> Option<SemanticDigest> {
        self.base
    }

    /// Expected head digest.
    #[must_use]
    pub const fn head(&self) -> Option<SemanticDigest> {
        self.head
    }

    /// Disposition class.
    #[must_use]
    pub const fn disposition(&self) -> ChangeDispositionKind {
        self.disposition
    }

    /// Exact reaffirmation digest for a reaffirmed disposition.
    #[must_use]
    pub const fn reaffirmation_digest(&self) -> Option<SemanticDigest> {
        self.reaffirmation_digest
    }

    /// Current process waiver for a waived disposition.
    #[must_use]
    pub const fn waiver(&self) -> Option<&ObjectRef> {
        self.waiver.as_ref()
    }

    /// Replacement objects for revised or retired dispositions.
    #[must_use]
    pub fn replacements(&self) -> &[ObjectRef] {
        &self.replacements
    }

    fn canonical_value(&self) -> CanonicalValue {
        self.meta.canonical_value().clone()
    }
}

fn change_disposition_value(disposition: &ChangeDisposition) -> CanonicalValue {
    disposition.canonical_value()
}

/// Provider-neutral graph-pair change manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeManifest {
    meta: ObjectMeta,
    base_graph: SemanticDigest,
    head_graph: SemanticDigest,
    dispositions: Vec<ChangeDisposition>,
    approval: Option<ApprovalBinding>,
}

impl ChangeManifest {
    /// Constructs one complete provider-neutral change manifest.
    ///
    /// # Errors
    ///
    /// Rejects duplicate dispositions or noncanonical fields.
    pub fn new(
        id: StableId,
        base_graph: SemanticDigest,
        head_graph: SemanticDigest,
        dispositions: Vec<ChangeDisposition>,
        approval: Option<ApprovalBinding>,
    ) -> Result<Self, ConstructionError> {
        require_same_digest_algorithm(base_graph, head_graph)?;
        if base_graph == head_graph && !dispositions.is_empty() {
            return Err(ConstructionError::LocalInvariant);
        }
        let dispositions = canonicalize_set(dispositions, change_disposition_value)?;
        let mut impact_keys = BTreeSet::new();
        if dispositions.iter().any(|disposition| {
            !impact_keys.insert((
                disposition.impacted_kind(),
                disposition.impacted_id().clone(),
            ))
        }) {
            return Err(ConstructionError::LocalInvariant);
        }
        let fields = alloc::vec![
            (1, text_value(id.as_str())),
            (2, CanonicalValue::Digest(base_graph)),
            (3, CanonicalValue::Digest(head_graph)),
            (
                4,
                CanonicalValue::Set(dispositions.iter().map(change_disposition_value).collect(),),
            ),
            (5, approval_value(approval.as_ref())),
        ];
        let references = canonicalize_validated_set(
            dispositions
                .iter()
                .map(|disposition| {
                    ObjectRef::new(
                        ObjectKind::ChangeDisposition,
                        disposition.meta().id().clone(),
                        disposition.meta().digest(),
                    )
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            object_reference_value,
        );
        let meta = ObjectMeta::for_fields(
            ObjectKind::ChangeManifest,
            id,
            "change-manifest",
            fields,
            references,
        );
        Ok(Self {
            meta,
            base_graph,
            head_graph,
            dispositions,
            approval,
        })
    }

    /// Shared metadata.
    #[must_use]
    pub const fn meta(&self) -> &ObjectMeta {
        &self.meta
    }

    /// Base complete-graph digest.
    #[must_use]
    pub const fn base_graph(&self) -> SemanticDigest {
        self.base_graph
    }

    /// Different head complete-graph digest.
    #[must_use]
    pub const fn head_graph(&self) -> SemanticDigest {
        self.head_graph
    }

    /// Exact impact dispositions in canonical set order.
    #[must_use]
    pub fn dispositions(&self) -> &[ChangeDisposition] {
        &self.dispositions
    }

    /// Optional repository-owned change approval.
    #[must_use]
    pub const fn approval(&self) -> Option<&ApprovalBinding> {
        self.approval.as_ref()
    }
}

/// Complete schema-v2 semantic graph.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SpecGraph {
    /// Live atoms.
    pub atoms: Vec<Atom>,
    /// Permanent terminal atoms.
    pub tombstones: Vec<Tombstone>,
    /// Governed surfaces.
    pub decision_surfaces: Vec<DecisionSurface>,
    /// Evidence carriers.
    pub evidence_carriers: Vec<EvidenceCarrier>,
    /// Navigational links.
    pub implementation_links: Vec<ImplementationLink>,
    /// Immutable content-addressed implementation bindings.
    pub implementation_bindings: Vec<ImplementationBinding>,
    /// Intentional derivations.
    pub derivations: Vec<Derivation>,
    /// Coverage policies, including terminal history.
    pub coverage_policies: Vec<CoveragePolicy>,
    /// Debt history.
    pub debt_records: Vec<DebtRecord>,
    /// Waiver history.
    pub waivers: Vec<Waiver>,
    /// Oracle declarations.
    pub oracles: Vec<Oracle>,
    /// Finite models.
    pub finite_models: Vec<FiniteModel>,
    /// Formal properties.
    pub formal_properties: Vec<FormalProperty>,
    /// Assumptions.
    pub assumptions: Vec<Assumption>,
    /// Proof records.
    pub proof_records: Vec<ProofRecord>,
    /// Reaffirmation review records.
    pub reaffirmations: Vec<Reaffirmation>,
    /// Repository-owned external navigation links.
    pub external_trace_links: Vec<ExternalTraceLink>,
}

impl SpecGraph {
    /// Collects every common object metadata value in schema kind order.
    #[must_use]
    pub fn object_meta(&self) -> Vec<&ObjectMeta> {
        let mut values = Vec::new();
        values.extend(self.atoms.iter().map(Atom::meta));
        values.extend(self.tombstones.iter().map(Tombstone::meta));
        values.extend(self.decision_surfaces.iter().map(DecisionSurface::meta));
        values.extend(self.evidence_carriers.iter().map(EvidenceCarrier::meta));
        values.extend(
            self.implementation_links
                .iter()
                .map(ImplementationLink::meta),
        );
        values.extend(
            self.implementation_bindings
                .iter()
                .map(ImplementationBinding::meta),
        );
        values.extend(self.derivations.iter().map(Derivation::meta));
        values.extend(self.coverage_policies.iter().map(CoveragePolicy::meta));
        values.extend(self.debt_records.iter().map(DebtRecord::meta));
        values.extend(self.waivers.iter().map(Waiver::meta));
        values.extend(self.oracles.iter().map(Oracle::meta));
        values.extend(self.finite_models.iter().map(FiniteModel::meta));
        values.extend(self.formal_properties.iter().map(FormalProperty::meta));
        values.extend(self.assumptions.iter().map(Assumption::meta));
        values.extend(self.proof_records.iter().map(ProofRecord::meta));
        values.extend(self.reaffirmations.iter().map(Reaffirmation::meta));
        values.extend(
            self.external_trace_links
                .iter()
                .map(ExternalTraceLink::meta),
        );
        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// Two unequal nodes never share one digest under an intact hash, so this
    /// forged pair reaches the collision guard that no public constructor can.
    #[test]
    #[allow(clippy::unwrap_used)]
    fn digest_equal_unequal_nodes_fail_closed() {
        let item = |payload: &[u8]| {
            ParsedItemIdentity::new(
                SchemaId::new("model/v1").unwrap(),
                RepositoryPath::new("src/lib.rs").unwrap(),
                SourceItemKind::new("model/node/v1").unwrap(),
                payload.to_vec(),
                None,
            )
            .unwrap()
        };
        let honest = SourceNode::new(item(b"honest"), Vec::new()).unwrap();
        let forged = SourceNode {
            item: item(b"forged"),
            children: Vec::new(),
            digest: honest.digest(),
        };
        assert_ne!(honest, forged);
        assert_eq!(honest.digest(), forged.digest());

        let budgets = SourceBudgets::new(
            SourceNodeBudget::new(4).unwrap(),
            SourceEdgeBudget::new(4).unwrap(),
            SourceDepthBudget::new(4).unwrap(),
            SourceVisitedBudget::new(4).unwrap(),
        );
        for nodes in [
            vec![honest.clone(), forged.clone()],
            vec![forged, honest.clone()],
        ] {
            assert_eq!(
                SourceGraph::new(nodes, Vec::new(), Vec::new(), budgets),
                Err(SourceGraphError::NodeDigestCollision),
                "one digest never carries two unequal nodes"
            );
        }
        assert!(
            SourceGraph::new(
                vec![honest.clone(), honest],
                Vec::new(),
                Vec::new(),
                budgets,
            )
            .is_ok(),
            "one repeated equal node aggregates idempotently"
        );
    }
}
